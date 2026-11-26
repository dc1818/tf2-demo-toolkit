//! Completion and publication for automatic native image batches.
use super::*;

pub(super) const VERSION: u32 = 1;
const STAGING: &str = ".tf2tk-image-staging";
const MAX_CONSOLE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_RECEIPT_BYTES: u64 = 64 * 1024 * 1024;

pub(super) fn is_batch(session: &Path) -> bool {
    session
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            name.starts_with("tf2_demo_toolkit_batch_")
                || name.starts_with("tf2fragdemohelper_batch_")
        })
}

fn session_name(session: &Path) -> Result<&str> {
    session
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| is_batch(session) && safe_token(name))
        .context("invalid image recording session name")
}

fn safe_token(token: &str) -> bool {
    !token.is_empty()
        && token
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}

pub(super) fn log_name(session: &Path) -> Result<String> {
    Ok(format!("{}.log", session_name(session)?))
}

pub(super) fn capture_base(session_name: &str, order: usize) -> String {
    format!("tf2frag_{session_name}_{order:03}")
}

#[derive(Default)]
pub(super) struct CompletionLog {
    pub(super) started: HashSet<String>,
    pub(super) finished: HashSet<String>,
    pub(super) batch_finished: bool,
}

#[derive(PartialEq)]
enum Phase {
    Started,
    Ending,
    Stopped,
}

impl CompletionLog {
    pub(super) fn read(path: &Path) -> Result<Self> {
        let bytes = read_file_bounded(path, MAX_CONSOLE_BYTES)?;
        Ok(Self::parse(&String::from_utf8_lossy(&bytes)))
    }

    fn parse(text: &str) -> Self {
        let mut outcome = Self::default();
        let mut active: Option<(&str, Phase)> = None;
        let mut invalid = HashSet::new();
        for line in text
            .split_inclusive('\n')
            .filter(|line| line.ends_with('\n'))
        {
            let line = line.trim();
            // TF2 optionally prefixes a complete line with its console timestamp.
            let command = line
                .split_once(": ")
                .map_or(line, |(_, message)| message.trim());
            if matches!(
                command,
                "Stopped recording movie..." | "Finishing recording ... Stopped recording movie..."
            ) {
                if let Some((id, Phase::Ending)) = active {
                    active = Some((id, Phase::Stopped));
                }
                continue;
            }
            if !command.starts_with("TF2FRAG_") {
                continue;
            }
            // TF2 can combine successive echoes on one console line.
            for token in command.split_ascii_whitespace() {
                if token == "TF2FRAG_BATCH_FINISHED" {
                    outcome.batch_finished = true;
                } else if let Some(id) = token
                    .strip_prefix("TF2FRAG_IMAGE_START_")
                    .filter(|id| safe_token(id))
                {
                    if let Some((previous, _)) = active.take() {
                        invalid.insert(previous);
                    }
                    if !outcome.started.insert(id.to_owned()) {
                        invalid.insert(id);
                    }
                    active = Some((id, Phase::Started));
                } else if let Some(id) = token
                    .strip_prefix("TF2FRAG_IMAGE_END_")
                    .filter(|id| safe_token(id))
                {
                    if matches!(&active, Some((key, Phase::Started)) if *key == id) {
                        active = Some((id, Phase::Ending));
                    } else {
                        invalid.insert(id);
                    }
                } else if let Some(id) = token
                    .strip_prefix("TF2FRAG_IMAGE_FINALIZED_")
                    .filter(|id| safe_token(id))
                {
                    if matches!(&active, Some((key, Phase::Stopped)) if *key == id) {
                        active = None;
                        outcome.finished.insert(id.to_owned());
                    } else {
                        invalid.insert(id);
                    }
                }
            }
        }
        outcome.finished.retain(|id| !invalid.contains(id.as_str()));
        outcome
    }
}

pub(super) fn retained_log(session: &Path, game: &Path) -> CompletionLog {
    log_name(session)
        .ok()
        .and_then(|name| CompletionLog::read(&game.join(name)).ok())
        .or_else(|| CompletionLog::read(&session.join("tf2_console.log")).ok())
        .unwrap_or_default()
}

#[derive(Deserialize, Serialize)]
struct Receipt {
    version: u32,
    capture_base: String,
    artifacts: Vec<Artifact>,
}

#[derive(Deserialize, Serialize)]
struct Artifact {
    source_name: String,
    sha256: String,
}

fn image_extension(settings: &AppSettings) -> &str {
    if settings.recording_format.starts_with("TGA") {
        "tga"
    } else {
        "jpg"
    }
}

fn artifact_number(name: &str, prefix: &str, extension: &str) -> Option<u32> {
    let (number, file_extension) = name.strip_prefix(prefix)?.rsplit_once('.')?;
    if !file_extension.eq_ignore_ascii_case(extension)
        || number.is_empty()
        || !number.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    number.parse().ok()
}

impl Receipt {
    fn validate(&self, clip: &PreparedClip, extension: &str) -> Result<()> {
        anyhow::ensure!(
            self.version == VERSION && self.capture_base == clip.capture_base,
            "image completion receipt belongs to another capture"
        );
        let mut frames = BTreeSet::new();
        let mut audio = false;
        for artifact in &self.artifacts {
            let name = &artifact.source_name;
            anyhow::ensure!(
                artifact.sha256.len() == 64
                    && artifact.sha256.bytes().all(|byte| byte.is_ascii_hexdigit()),
                "invalid image completion fingerprint"
            );
            if name.eq_ignore_ascii_case(&format!("{}.wav", clip.capture_base)) {
                anyhow::ensure!(!audio, "repeated audio artifact");
                audio = true;
            } else if let Some(number) = artifact_number(name, &clip.capture_base, extension) {
                anyhow::ensure!(frames.insert(number), "repeated image frame number");
            } else {
                bail!("unexpected image completion artifact");
            }
        }
        anyhow::ensure!(
            !frames.is_empty()
                && frames
                    .into_iter()
                    .enumerate()
                    .all(|(index, frame)| index == frame as usize),
            "image sequence has missing frames"
        );
        Ok(())
    }
}

fn validate_image(path: &Path, extension: &str) -> Result<()> {
    let mut file = File::open(path)?;
    let length = file.metadata()?.len();
    if extension == "tga" {
        let mut header = [0; 18];
        file.read_exact(&mut header)?;
        let width = u64::from(u16::from_le_bytes([header[12], header[13]]));
        let height = u64::from(u16::from_le_bytes([header[14], header[15]]));
        // Source's raw movie output is uncompressed true-colour TGA.
        anyhow::ensure!(
            header[1] == 0
                && header[2] == 2
                && matches!(header[16], 24 | 32)
                && width > 0
                && height > 0
                && length >= 18 + u64::from(header[0]) + width * height * u64::from(header[16] / 8),
            "truncated or unsupported native TGA frame"
        );
    } else {
        let mut header = [0; 3];
        let mut tail = [0; 2];
        file.read_exact(&mut header)?;
        anyhow::ensure!(
            length > 5 && header == [0xff, 0xd8, 0xff],
            "invalid native JPG header"
        );
        file.seek(SeekFrom::End(-2))?;
        file.read_exact(&mut tail)?;
        anyhow::ensure!(tail == [0xff, 0xd9], "truncated native JPG frame");
    }
    Ok(())
}

fn validate_wav(path: &Path) -> Result<()> {
    let mut file = File::open(path)?;
    let length = file.metadata()?.len();
    let mut header = [0; 12];
    file.read_exact(&mut header)?;
    let end = 8 + u64::from(u32::from_le_bytes(header[4..8].try_into()?));
    anyhow::ensure!(
        &header[..4] == b"RIFF" && &header[8..] == b"WAVE" && end > 12 && end <= length,
        "truncated or incomplete native WAV header"
    );
    let mut offset = 12;
    let mut has_format = false;
    let mut has_audio = false;
    while offset + 8 <= end {
        let mut chunk = [0; 8];
        file.read_exact(&mut chunk)?;
        let bytes = u64::from(u32::from_le_bytes(chunk[4..].try_into()?));
        offset += 8 + bytes + bytes % 2;
        anyhow::ensure!(offset <= end, "native WAV chunk exceeds file size");
        has_format |= &chunk[..4] == b"fmt " && bytes >= 16;
        has_audio |= &chunk[..4] == b"data" && bytes > 0;
        file.seek(SeekFrom::Start(offset))?;
    }
    anyhow::ensure!(
        offset == end && has_format && has_audio,
        "native WAV has no complete audio"
    );
    Ok(())
}

fn record_receipt(
    clip: &PreparedClip,
    game: &Path,
    extension: &str,
    path: &Path,
) -> Result<Receipt> {
    let mut artifacts = Vec::new();
    let wave = format!("{}.wav", clip.capture_base);
    for entry in fs::read_dir(game)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        if name.eq_ignore_ascii_case(&wave) {
            validate_wav(&entry.path())?;
        } else if artifact_number(&name, &clip.capture_base, extension).is_some() {
            validate_image(&entry.path(), extension)?;
        } else {
            continue;
        }
        artifacts.push(Artifact {
            source_name: name,
            sha256: file_fingerprint(&entry.path())?,
        });
    }
    artifacts.sort_by(|left, right| left.source_name.cmp(&right.source_name));
    let receipt = Receipt {
        version: VERSION,
        capture_base: clip.capture_base.clone(),
        artifacts,
    };
    receipt.validate(clip, extension)?;
    let temporary = path.with_extension("tmp");
    let mut file = File::create(&temporary)?;
    serde_json::to_writer(&mut file, &receipt)?;
    file.sync_all()?;
    drop(file);
    fs::rename(temporary, path)?;
    Ok(receipt)
}

fn destination(root: &Path, clip: &PreparedClip, artifact: &Artifact) -> PathBuf {
    let suffix = &artifact.source_name[clip.capture_base.len()..];
    if suffix.eq_ignore_ascii_case(".wav") {
        root.join("Audio")
            .join(format!("{}.wav", clip.recording_identifier))
    } else {
        root.join("Frames").join(format!("frame{suffix}"))
    }
}

/// Complete a confirmed image batch without exposing a partial output to discovery.
/// Returns whether this invocation published a new directory.
pub(super) fn publish(
    clip: &PreparedClip,
    game: &Path,
    session: &Path,
    settings: &AppSettings,
    finished: bool,
) -> Result<bool> {
    let session_id = session_name(session)?;
    anyhow::ensure!(
        clip.capture_base == capture_base(session_id, clip.order),
        "invalid image capture identity"
    );
    let output = &clip.final_output_path;
    let category = output
        .parent()
        .context("image sequence has no output folder")?;
    anyhow::ensure!(
        category
            .file_name()
            .is_some_and(|name| name == "Image Sequences")
            && safe_token(&clip.recording_identifier)
            && output
                .file_name()
                .is_some_and(|name| name == clip.recording_identifier.as_str())
            && clip.frames_path.as_deref() == Some(output.join("Frames").as_path())
            && clip.audio_path.as_deref() == Some(output.join("Audio").as_path()),
        "invalid image sequence destination"
    );
    let receipt_path = session.join(format!("{}.image-complete.json", clip.capture_base));
    let extension = image_extension(settings);
    let receipt: Receipt = if receipt_path.is_file() {
        serde_json::from_slice(&read_file_bounded(&receipt_path, MAX_RECEIPT_BYTES)?)?
    } else {
        anyhow::ensure!(
            finished,
            "no session-specific image completion evidence; source media retained for salvage"
        );
        record_receipt(clip, game, extension, &receipt_path)?
    };
    receipt.validate(clip, extension)?;

    if output.exists() {
        // A crash after the directory rename but before indexing is recoverable.
        verify_directory(output, clip, &receipt)?;
        return Ok(false);
    }
    let staging_root = category
        .parent()
        .context("image sequence has no recording root")?
        .join(STAGING);
    let staging = staging_root.join(&clip.capture_base);
    let copies = staging_root.join(format!("{}.copies", clip.capture_base));
    fs::create_dir_all(staging.join("Frames"))?;
    fs::create_dir_all(staging.join("Audio"))?;
    for artifact in &receipt.artifacts {
        let staged = destination(&staging, clip, artifact);
        if staged.exists() {
            verify_artifact(&staged, artifact)?;
            continue;
        }
        let source = game.join(&artifact.source_name);
        verify_artifact(&source, artifact)?;
        if fs::rename(&source, &staged).is_err() {
            copy_artifact(&source, &staged, &copies, artifact)?;
        }
    }
    verify_directory(&staging, clip, &receipt)?;
    fs::create_dir_all(category)?;
    fs::rename(staging, output)?;
    let _ = fs::remove_dir(copies);
    let _ = fs::remove_dir(staging_root);
    Ok(true)
}

fn copy_artifact(source: &Path, staged: &Path, copies: &Path, artifact: &Artifact) -> Result<()> {
    fs::create_dir_all(copies)?;
    // A retry overwrites its own incomplete copy outside the publishable directory.
    let partial = copies.join(&artifact.source_name);
    fs::copy(source, &partial)?;
    OpenOptions::new().write(true).open(&partial)?.sync_all()?;
    verify_artifact(&partial, artifact)?;
    fs::rename(partial, staged)?;
    fs::remove_file(source)?;
    Ok(())
}

fn verify_artifact(path: &Path, artifact: &Artifact) -> Result<()> {
    anyhow::ensure!(
        fs::symlink_metadata(path)?.is_file() && file_fingerprint(path)? == artifact.sha256,
        "image artifact changed: {}",
        path.display()
    );
    Ok(())
}

fn verify_directory(root: &Path, clip: &PreparedClip, receipt: &Receipt) -> Result<()> {
    let expected: HashSet<_> = receipt
        .artifacts
        .iter()
        .map(|artifact| destination(root, clip, artifact))
        .collect();
    for entry in WalkDir::new(root).max_depth(2) {
        let entry = entry?;
        if entry.file_type().is_dir() {
            anyhow::ensure!(
                entry.path() == root
                    || entry.path() == root.join("Frames")
                    || entry.path() == root.join("Audio"),
                "unexpected directory in image sequence"
            );
        } else {
            anyhow::ensure!(
                expected.contains(entry.path()),
                "unexpected file in image sequence"
            );
        }
    }
    for artifact in &receipt.artifacts {
        verify_artifact(&destination(root, clip, artifact), artifact)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_ordered_session_clip_markers_can_prove_completion() {
        let text = concat!(
            "TF2FRAG_IMAGE_START_session_a_001\n",
            "TF2FRAG_IMAGE_END_session_a_001\n",
            "Stopped recording movie...\n",
            "TF2FRAG_IMAGE_FINALIZED_session_a_001 TF2FRAG_BATCH_FINISHED\n",
            "TF2FRAG_IMAGE_START_session_a_002\n",
            "TF2FRAG_IMAGE_END_session_b_002\n",
            "Stopped recording movie...\n",
            "TF2FRAG_IMAGE_FINALIZED_session_a_002\n",
            "TF2FRAG_IMAGE_START_session_a_003\n",
            "TF2FRAG_IMAGE_END_session_a_003\n",
            "Stopped recording movie...\n",
            "TF2FRAG_IMAGE_FINALIZED_session_a_003"
        );
        let evidence = CompletionLog::parse(text);
        assert!(evidence.batch_finished);
        assert_eq!(evidence.finished, HashSet::from(["session_a_001".into()]));
        assert!(CompletionLog::parse(concat!(
            "TF2FRAG_IMAGE_START_duplicate\n",
            "TF2FRAG_IMAGE_START_duplicate\n",
            "TF2FRAG_IMAGE_END_duplicate\n",
            "Stopped recording movie...\n",
            "TF2FRAG_IMAGE_FINALIZED_duplicate\n",
        ))
        .finished
        .is_empty());
    }

    #[test]
    fn partial_transfers_stay_out_of_discovery_and_retry_from_the_receipt() {
        let root = std::env::temp_dir().join(format!(
            "tf2-image-retry-{}-{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap()
        ));
        let session = root.join("tf2_demo_toolkit_batch_retry");
        let game = root.join("tf");
        let media = root.join("recordings");
        let output = media.join("Image Sequences/retry-clip");
        fs::create_dir_all(&session).unwrap();
        fs::create_dir_all(&game).unwrap();
        let clip = prepared_clip_from_manifest(&StoredRecordingClip {
            order: 1,
            native_capture_base: capture_base("tf2_demo_toolkit_batch_retry", 1),
            recording_identifier: "retry-clip".into(),
            frames_path: Some(output.join("Frames")),
            audio_path: Some(output.join("Audio")),
            expected_output_path: output.clone(),
            ..StoredRecordingClip::default()
        });
        let settings = AppSettings {
            recording_format: "JPG Image Sequence".into(),
            ..Default::default()
        };
        let frames = [
            [0xff, 0xd8, 0xff, 1, 0xff, 0xd9],
            [0xff, 0xd8, 0xff, 2, 0xff, 0xd9],
        ];
        for (index, frame) in frames.iter().enumerate() {
            fs::write(
                game.join(format!("{}{index:04}.jpg", clip.capture_base)),
                frame,
            )
            .unwrap();
        }
        assert!(publish(&clip, &game, &session, &settings, false).is_err());
        assert!(final_recording_outputs(&media).is_empty());

        let stage = media.join(STAGING).join(&clip.capture_base);
        let blocker = stage.join("Frames/frame0001.jpg");
        fs::create_dir_all(&blocker).unwrap();
        assert!(publish(&clip, &game, &session, &settings, true).is_err());
        assert!(stage.join("Frames/frame0000.jpg").is_file());
        assert!(final_recording_outputs(&media).is_empty());
        assert!(game
            .join(format!("{}0001.jpg", clip.capture_base))
            .is_file());

        fs::remove_dir(&blocker).unwrap();
        assert!(publish(&clip, &game, &session, &settings, false).unwrap());
        assert_eq!(final_recording_outputs(&media), [output.clone()]);
        for (index, frame) in frames.iter().enumerate() {
            assert_eq!(
                fs::read(output.join(format!("Frames/frame{index:04}.jpg"))).unwrap(),
                *frame
            );
        }
        assert!(!publish(&clip, &game, &session, &settings, false).unwrap());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn retry_replaces_abandoned_copy_without_putting_temporary_bytes_in_the_sequence() {
        let root = std::env::temp_dir().join(format!(
            "tf2-image-copy-{}-{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap()
        ));
        let copies = root.join("copies");
        let staging = root.join("staging");
        fs::create_dir_all(&copies).unwrap();
        fs::create_dir_all(&staging).unwrap();
        let source = root.join("source.jpg");
        fs::write(&source, b"complete source").unwrap();
        let artifact = Artifact {
            source_name: "source.jpg".into(),
            sha256: file_fingerprint(&source).unwrap(),
        };
        fs::write(copies.join(&artifact.source_name), b"crashed copy").unwrap();
        let staged = staging.join("frame0000.jpg");
        copy_artifact(&source, &staged, &copies, &artifact).unwrap();
        assert!(!source.exists());
        assert_eq!(fs::read(&staged).unwrap(), b"complete source");
        assert_eq!(fs::read_dir(&copies).unwrap().count(), 0);
        assert_eq!(fs::read_dir(&staging).unwrap().count(), 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn incomplete_and_legacy_recovery_preserve_source_and_the_old_recording() {
        let root = std::env::temp_dir().join(format!(
            "tf2-image-incomplete-{}-{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap()
        ));
        let session = root.join("tf2_demo_toolkit_batch_replacement");
        let game = root.join("tf");
        fs::create_dir_all(&session).unwrap();
        fs::create_dir_all(&game).unwrap();
        let id = capture_base("tf2_demo_toolkit_batch_replacement", 1);
        let partial = game.join(format!("{id}0000.tga"));
        fs::write(&partial, b"interrupted source").unwrap();
        let old = root.join("previous.mp4");
        fs::write(&old, b"old published recording").unwrap();
        let index_path = root.join("index.ndjson");
        let mut index = RecordingIndex::load_from_path(&index_path, 100_000, 100);
        index
            .store_entry(RecordingEntry {
                recording_key: "fixture-key".into(),
                output_path: old.clone(),
                ..Default::default()
            })
            .unwrap();
        let before = fs::read(&index_path).unwrap();
        let output = root.join("Image Sequences/replacement");
        let settings = AppSettings {
            recording_format: "TGA Image Sequence".into(),
            ..Default::default()
        };
        fs::write(
            session.join("tf2_console.log"),
            format!("TF2FRAG_IMAGE_START_{id}\n"),
        )
        .unwrap();
        for version in [Some(VERSION), None] {
            write_json_atomic(&session.join("recording_manifest.json"), &json!({
                "image_completion_version": version, "output_format": "TGA Image Sequence", "game_directory": game,
                "clips": [{
                    "order": 1, "recording_key": "fixture-key", "candidate_id": "replacement",
                    "recording_identifier": "replacement", "native_capture_base": id,
                    "expected_output_path": output, "frames_path": output.join("Frames"),
                    "audio_path": output.join("Audio"), "replace_existing": true
                }]
            })).unwrap();
            let mut report = RecordingRecoveryReport::default();
            recover_one_recording_session_with_index(&session, &settings, &mut report, &index_path);
            assert_eq!(report.retained_sessions, 1);
            assert_eq!(report.recovered_clips + report.indexed_clips, 0);
            assert!(!output.exists());
            assert_eq!(fs::read(&partial).unwrap(), b"interrupted source");
            assert_eq!(fs::read(&old).unwrap(), b"old published recording");
            assert_eq!(fs::read(&index_path).unwrap(), before);
        }
        fs::remove_dir_all(root).unwrap();
    }
}
