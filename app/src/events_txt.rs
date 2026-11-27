//! Read-only TF2 Demo Support bookmark import for the selected demo paths.
use serde::Serialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::{self, Read},
    path::{self, PathBuf},
};

const MAX_LOG_BYTES: u64 = 8 * 1024 * 1024;
const MAX_DIAGNOSTICS: usize = 32;

#[derive(Default)]
pub(crate) struct DemoImport {
    pub(crate) bookmarks: Vec<(i64, String)>,
    pub(crate) sources: Vec<SourceReport>,
}

#[derive(Clone, Default, Serialize)]
pub(crate) struct SourceReport {
    source: String,
    matched: usize,
    ignored_killstreaks: usize,
    malformed: usize,
    unmatched: usize,
    ambiguous: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    read_error: Option<String>,
    rejected_samples: Vec<(usize, &'static str)>,
}

fn report_rejection(report: &mut SourceReport, line: usize, reason: &'static str) {
    if report.rejected_samples.len() < MAX_DIAGNOSTICS {
        report.rejected_samples.push((line, reason));
    }
}

fn portable_path(path: &str) -> String {
    let path = path.replace('\\', "/").to_lowercase();
    let unc;
    let path = if let Some(rest) = path.strip_prefix("//?/unc/") {
        unc = format!("//{rest}");
        &unc
    } else {
        path.strip_prefix("//?/").unwrap_or(&path)
    };
    let root = if path.starts_with("//") {
        "//"
    } else if path.starts_with('/') {
        "/"
    } else {
        ""
    };
    let root_parts = if root == "//" {
        2
    } else if path.as_bytes().get(1) == Some(&b':') {
        1
    } else {
        0
    };
    let mut parts = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." if parts.len() > root_parts && parts.last() != Some(&"..") => {
                parts.pop();
            }
            _ => parts.push(part),
        }
    }
    format!("{root}{}", parts.join("/"))
}

fn demo_key(path: &str) -> String {
    let normalized = portable_path(path);
    normalized
        .strip_suffix(".dem")
        .unwrap_or(&normalized)
        .to_owned()
}

fn reference_key(directory: &str, reference: &str) -> String {
    let reference = reference.replace('\\', "/");
    if reference.starts_with('/') || reference.as_bytes().get(1) == Some(&b':') {
        demo_key(&reference)
    } else {
        demo_key(&format!("{directory}/{reference}"))
    }
}

enum Row<'a> {
    Separator,
    Killstreak,
    Bookmark {
        demo: &'a str,
        tick: i64,
        comment: &'a str,
    },
}

fn parse_line(line: &str) -> Option<Row<'_>> {
    let line = line.trim();
    if line.len() > 16 * 1024 {
        return None;
    }
    if line.is_empty() || line == ">" {
        return Some(Row::Separator);
    }
    let (_, event) = line.strip_prefix('[')?.split_once("] ")?;
    let (description, location) = event.rsplit_once(" (\"")?;
    let (demo, tick) = location.strip_suffix(')')?.rsplit_once("\" at ")?;
    if demo.trim().is_empty() || demo.contains('"') || demo.chars().any(char::is_control) {
        return None;
    }
    let tick = i64::from(tick.parse::<u32>().ok()?);
    if tick == 0 {
        return None;
    }
    let mut words = description.splitn(2, char::is_whitespace);
    match words.next()?.to_ascii_lowercase().as_str() {
        "bookmark" => {
            let comment = words.next().unwrap_or_default().trim();
            (!comment.chars().any(char::is_control)).then_some(Row::Bookmark {
                demo,
                tick,
                comment,
            })
        }
        "killstreak" => Some(Row::Killstreak),
        _ => None,
    }
}

fn decode(bytes: &[u8]) -> Option<String> {
    if bytes.starts_with(&[0xff, 0xfe]) || bytes.starts_with(&[0xfe, 0xff]) {
        let little_endian = bytes[0] == 0xff;
        let mut units = bytes[2..].chunks_exact(2);
        let decoded = char::decode_utf16(units.by_ref().map(|pair| {
            let pair = [pair[0], pair[1]];
            if little_endian {
                u16::from_le_bytes(pair)
            } else {
                u16::from_be_bytes(pair)
            }
        }))
        .collect::<Result<String, _>>()
        .ok()?;
        return units.remainder().is_empty().then_some(decoded);
    }
    let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes);
    std::str::from_utf8(bytes).ok().map(str::to_owned)
}

fn read_log(file: File) -> io::Result<String> {
    let mut data = Vec::new();
    file.take(MAX_LOG_BYTES + 1).read_to_end(&mut data)?;
    if data.len() as u64 > MAX_LOG_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "bookmark log exceeds 8 MiB",
        ));
    }
    decode(&data).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "unsupported bookmark log encoding",
        )
    })
}

pub(crate) fn collect(selected: &[PathBuf]) -> Vec<DemoImport> {
    let mut imports: Vec<_> = selected.iter().map(|_| DemoImport::default()).collect();
    let mut demos = BTreeMap::<String, Vec<usize>>::new();
    let mut directories = BTreeMap::<PathBuf, Vec<usize>>::new();
    for (index, path) in selected.iter().enumerate() {
        let Ok(absolute) = path::absolute(path) else {
            continue;
        };
        let Some(directory) = absolute.parent() else {
            continue;
        };
        let Some(path) = absolute.to_str() else {
            continue;
        };
        demos.entry(demo_key(path)).or_default().push(index);
        directories
            .entry(directory.to_owned())
            .or_default()
            .push(index);
    }
    for (directory, local) in directories {
        let key = portable_path(&directory.to_string_lossy());
        for name in ["_events.txt", "events.txt"] {
            let source = directory.join(name);
            let log = match File::open(&source) {
                Ok(file) => read_log(file),
                Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
                Err(error) => Err(error),
            };
            let mut report = SourceReport {
                source: source.to_string_lossy().into_owned(),
                ..SourceReport::default()
            };
            let mut readers: BTreeSet<_> = local.iter().copied().collect();
            match log {
                Ok(log) => {
                    for (index, line) in log.lines().enumerate() {
                        match parse_line(line) {
                            Some(Row::Separator) => {}
                            Some(Row::Killstreak) => report.ignored_killstreaks += 1,
                            Some(Row::Bookmark {
                                demo,
                                tick,
                                comment,
                            }) => match demos.get(&reference_key(&key, demo)).map(Vec::as_slice) {
                                Some([target]) => {
                                    imports[*target].bookmarks.push((tick, comment.into()));
                                    readers.insert(*target);
                                    report.matched += 1;
                                }
                                Some(_) => {
                                    report.ambiguous += 1;
                                    report_rejection(&mut report, index + 1, "ambiguous_demo");
                                }
                                None => {
                                    report.unmatched += 1;
                                    report_rejection(&mut report, index + 1, "unmatched_demo");
                                }
                            },
                            None => {
                                report.malformed += 1;
                                report_rejection(&mut report, index + 1, "malformed_record");
                            }
                        }
                    }
                }
                Err(error) => report.read_error = Some(error.to_string()),
            }
            for target in readers {
                imports[target].sources.push(report.clone());
            }
        }
    }
    imports
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, time::SystemTime};

    #[test]
    fn resolves_selected_paths_before_any_bookmark_can_be_assigned() {
        let nonce = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("a16-paths-{}-{nonce}", std::process::id()));
        let a = root.join("first");
        let b = root.join("second");
        fs::create_dir_all(&a).unwrap();
        fs::create_dir_all(&b).unwrap();
        let paths = vec![a.join("Round One.dem"), b.join("Round One.dem")];
        let log = format!(
            r#">
[2018/08/26 22:43] Bookmark First ("ROUND ONE" at 200)
[2018/08/26 22:44] Bookmark Second ("..\second\Round One.demo" at 300)
[2018/08/26 22:45] Bookmark Second ("{}" at 400)
[2018/08/26 22:45] Bookmark stale ("E:\moved\Round One.dem" at 500)
[2018/08/26 22:46] Killstreak 4 ("Round One" at 520)
[2018/08/26 22:46] Bookmark invalid ("Round One" at -5)
malformed
"#,
            paths[1].display()
        );
        let utf16 = [0xff, 0xfe]
            .into_iter()
            .chain(log.encode_utf16().flat_map(u16::to_le_bytes))
            .collect::<Vec<_>>();
        fs::write(a.join("_events.txt"), &utf16).unwrap();
        fs::write(b.join("events.txt"), [0xff]).unwrap();
        let imports = collect(&paths);
        assert_eq!(imports[0].bookmarks, [(200, "First".into())]);
        assert_eq!(imports[1].bookmarks, [(400, "Second".into())]);
        let report = &imports[0].sources[0];
        assert_eq!(
            (
                report.matched,
                report.ignored_killstreaks,
                report.malformed,
                report.unmatched
            ),
            (2, 1, 2, 2)
        );
        assert!(imports[1]
            .sources
            .iter()
            .any(|source| source.read_error.is_some()));
        let ambiguous = collect(&[paths[0].clone(), paths[0].clone()]);
        assert!(ambiguous
            .iter()
            .all(|import| import.bookmarks.is_empty() && import.sources[0].ambiguous == 1));
        assert_eq!(fs::read(a.join("_events.txt")).unwrap(), utf16);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn understands_portable_explicit_and_relative_demo_references() {
        assert_eq!(
            reference_key("c:/demos/one", r"..\two\Round One.DEM"),
            "c:/demos/two/round one"
        );
        assert_eq!(
            reference_key("c:/demos/one", r"\\?\UNC\Server\Match Share\One.dem"),
            "//server/match share/one"
        );
        assert_ne!(
            reference_key("c:/demos/one", r"e:\old\Round One.dem"),
            "c:/demos/one/round one"
        );
        assert!(decode(&[0xff, 0xfe, 0x00]).is_none());
    }
}
