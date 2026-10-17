//! Stage TF2 demos without replaying the recorder's console commands.
//!
//! Console commands can reload graphics presets or execute binds while seeking.
//! Keep their packet lengths/offsets but replace the command strings with NULs;
//! network packets, user commands, ticks, and the original file are untouched.

use std::{
    fs::{self, File, OpenOptions},
    io::{self, BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

const HEADER_LENGTH: usize = 1072;
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Default, PartialEq, Eq)]
pub struct PlaybackStats {
    pub console_commands_neutralized: u64,
}

struct TemporaryFile(PathBuf);

impl Drop for TemporaryFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

/// Always create an independent copy. Older staged destinations may be hard
/// links to the original, so never open an existing destination for writing.
pub fn stage_playback_demo(source: &Path, destination: &Path) -> io::Result<PlaybackStats> {
    if source.canonicalize()?
        == destination
            .canonicalize()
            .unwrap_or_else(|_| destination.to_owned())
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "the staged demo must differ from the original",
        ));
    }
    let parent = destination
        .parent()
        .ok_or_else(|| invalid("staged demo has no parent directory"))?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let temporary = parent.join(format!(
        ".tf2_demo_toolkit_stage_{}_{}_{}.tmp",
        std::process::id(),
        nonce,
        TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    let mut input = BufReader::new(File::open(source)?);
    let output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    let guard = TemporaryFile(temporary);
    let mut output = BufWriter::new(output);
    let stats = copy_for_playback(&mut input, &mut output)?;
    output.flush()?;
    drop(output);
    drop(input);

    // Windows rename cannot replace an existing file. Unlink its directory
    // entry only, after validating the complete new copy; this also breaks an
    // old hard link without touching the original demo's contents.
    match fs::remove_file(destination) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    fs::rename(&guard.0, destination)?;
    Ok(stats)
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn transfer(
    input: &mut impl Read,
    output: &mut impl Write,
    mut length: u64,
    neutralize: bool,
) -> io::Result<()> {
    let mut buffer = [0u8; 16 * 1024];
    while length != 0 {
        let count = length.min(buffer.len() as u64) as usize;
        input.read_exact(&mut buffer[..count])?;
        if neutralize {
            buffer[..count].fill(0);
        }
        output.write_all(&buffer[..count])?;
        length -= count as u64;
    }
    Ok(())
}

fn copy_for_playback(input: &mut impl Read, output: &mut impl Write) -> io::Result<PlaybackStats> {
    let mut header = [0u8; HEADER_LENGTH];
    input.read_exact(&mut header)?;
    if &header[..8] != b"HL2DEMO\0" || u32::from_le_bytes(header[8..12].try_into().unwrap()) != 3 {
        return Err(invalid("playback staging requires a TF2 protocol 3 demo"));
    }
    output.write_all(&header)?;
    let mut stats = PlaybackStats::default();
    loop {
        let mut kind = [0u8; 1];
        input.read_exact(&mut kind)?;
        output.write_all(&kind)?;
        if kind[0] == 7 {
            // TF2's stop packet stores a 24-bit tick (see parser/packet/stop.rs).
            transfer(input, output, 3, false)?;
            io::copy(input, output)?;
            return Ok(stats);
        }
        if !matches!(kind[0], 1..=6 | 8) {
            return Err(invalid(
                "unknown TF2 demo packet type during playback staging",
            ));
        }
        transfer(input, output, 4, false)?; // tick, including reset/negative ticks
        match kind[0] {
            1 | 2 => transfer(input, output, 84, false)?, // view info and sequence numbers
            3 => continue,                                // sync tick has no payload
            5 => transfer(input, output, 4, false)?,      // user command sequence number
            _ => {}
        }
        let mut length = [0u8; 4];
        input.read_exact(&mut length)?;
        output.write_all(&length)?;
        let length = i32::from_le_bytes(length);
        if length < 0 {
            return Err(invalid("negative TF2 demo payload length"));
        }
        transfer(input, output, length as u64, kind[0] == 4)?;
        if kind[0] == 4 {
            stats.console_commands_neutralized += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (Vec<u8>, Vec<u8>) {
        let mut original = vec![0u8; HEADER_LENGTH];
        original[..8].copy_from_slice(b"HL2DEMO\0");
        original[8..12].copy_from_slice(&3u32.to_le_bytes());
        let mut command_ranges = Vec::new();
        for (index, kind) in [1, 6, 3, 8, 4, 2, 5, 4].into_iter().enumerate() {
            original.push(kind);
            // Deliberately include a tick reset and the signon sentinel.
            original
                .extend_from_slice(&[-1i32, 0, 0, 0, 30469, 35020, 0, 53160][index].to_le_bytes());
            match kind {
                1 | 2 => original.extend([42; 84]),
                3 => continue,
                5 => original.extend(789u32.to_le_bytes()),
                _ => {}
            }
            let payload = if kind == 4 {
                b"exec autoexec; mat_savechanges\0".as_slice()
            } else {
                &[5, 4, 0, 2, 7, 8]
            };
            original.extend((payload.len() as u32).to_le_bytes());
            if kind == 4 {
                command_ranges.push(original.len()..original.len() + payload.len());
            }
            original.extend(payload);
        }
        original.extend([7, 0x86, 0xd0, 0]);
        original.extend(b"trailer");
        let mut expected = original.clone();
        for range in command_ranges {
            expected[range].fill(0);
        }
        (original, expected)
    }

    fn test_directory() -> TemporaryFile {
        let path = std::env::temp_dir().join(format!(
            "tf2tk_demo_test_{}_{}",
            std::process::id(),
            TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        TemporaryFile(path)
    }

    #[test]
    fn only_console_payloads_change_and_all_offsets_survive() {
        let (original, expected) = fixture();
        let mut output = Vec::new();
        let stats = copy_for_playback(&mut original.as_slice(), &mut output).unwrap();
        assert_eq!(stats.console_commands_neutralized, 2);
        assert_eq!(output, expected);
        // Idempotent even if a previously staged demo is selected as input.
        let mut again = Vec::new();
        copy_for_playback(&mut output.as_slice(), &mut again).unwrap();
        assert_eq!(again, expected);
    }

    #[test]
    fn replacing_an_old_hard_link_does_not_modify_the_source() {
        let dir = test_directory();
        let source = dir.0.join("original.dem");
        let destination = dir.0.join("staged.dem");
        let (original, expected) = fixture();
        fs::write(&source, &original).unwrap();
        fs::hard_link(&source, &destination).unwrap();
        stage_playback_demo(&source, &destination).unwrap();
        assert_eq!(fs::read(&source).unwrap(), original);
        assert_eq!(fs::read(&destination).unwrap(), expected);
        assert!(stage_playback_demo(&source, &source).is_err());
        assert_eq!(fs::read(&source).unwrap(), original);
        fs::remove_dir_all(&dir.0).unwrap();
    }

    #[test]
    fn invalid_input_preserves_existing_destination_and_cleans_temporary_files() {
        let dir = test_directory();
        let source = dir.0.join("invalid.dem");
        let destination = dir.0.join("staged.dem");
        let (valid, _) = fixture();
        let mut wrong_protocol = valid.clone();
        wrong_protocol[8] = 4;
        let mut unknown_packet = valid.clone();
        unknown_packet[HEADER_LENGTH] = 9;
        let mut negative_length = valid.clone();
        negative_length[HEADER_LENGTH + 89..HEADER_LENGTH + 93].fill(0xff);
        for invalid in [
            valid[..100].to_vec(),
            valid[..HEADER_LENGTH + 95].to_vec(),
            valid[..valid.len() - 9].to_vec(),
            wrong_protocol,
            unknown_packet,
            negative_length,
        ] {
            fs::write(&source, &invalid).unwrap();
            fs::write(&destination, b"existing staged file").unwrap();
            assert!(stage_playback_demo(&source, &destination).is_err());
            assert_eq!(fs::read(&source).unwrap(), invalid);
            assert_eq!(fs::read(&destination).unwrap(), b"existing staged file");
            assert_eq!(fs::read_dir(&dir.0).unwrap().count(), 2);
        }
        fs::remove_dir_all(&dir.0).unwrap();
    }
}
