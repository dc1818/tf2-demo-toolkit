//! Stable, recognizable export locations; loading remains content-based for legacy exports.
use std::{fs, path::{Path, PathBuf}};

pub const GROUP_FOLDER: &str = "Demo Toolkit Exports";
pub const RUN_PREFIX: &str = "DemoToolkit_Parse_";

pub fn destination(path: &Path, grouped: bool) -> PathBuf {
    if path.as_os_str().is_empty() { return PathBuf::new(); }
    if grouped && !is_group_folder(path) { path.join(GROUP_FOLDER) } else { path.to_path_buf() }
}

pub fn is_group_folder(path: &Path) -> bool {
    path.file_name().is_some_and(|name| name.eq_ignore_ascii_case(GROUP_FOLDER))
}

pub fn default_location() -> PathBuf {
    dirs::document_dir().or_else(dirs::home_dir).unwrap_or_else(|| PathBuf::from(".")).join(GROUP_FOLDER)
}

pub fn candidate_file(folder: &Path) -> Option<PathBuf> {
    let file = folder.join("frag_candidates.ndjson");
    file.is_file().then_some(file)
}

pub fn existing_picker_folder(path: &Path) -> Option<PathBuf> {
    path.ancestors().find(|path| !path.as_os_str().is_empty() && path.is_dir()).map(Path::to_path_buf)
}

/// Reserve a new folder atomically. Never merge two runs, even in the same second.
pub fn create_run_folder(parent: &Path, timestamp: &str) -> std::io::Result<PathBuf> {
    fs::create_dir_all(parent)?;
    for suffix in 0..10_000 {
        let name = if suffix == 0 { format!("{RUN_PREFIX}{timestamp}") } else { format!("{RUN_PREFIX}{timestamp}_{suffix:02}") };
        let path = parent.join(name);
        match fs::create_dir(&path) {
            Ok(()) => return Ok(path),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    Err(std::io::Error::new(std::io::ErrorKind::AlreadyExists, "Too many exports with this timestamp"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn grouping_is_optional_and_never_nests_itself() {
        let parent = Path::new("exports");
        let grouped = parent.join(GROUP_FOLDER);
        assert_eq!(destination(parent, true), grouped);
        assert_eq!(destination(&grouped, true), grouped);
        assert_eq!(destination(parent, false), parent);
        assert_eq!(destination(Path::new(""), true), PathBuf::new());
    }
    #[test]
    fn preferences_round_trip_and_old_settings_receive_grouping_default() {
        let mut settings = crate::models::AppSettings::default();
        settings.output_directory = PathBuf::from("custom-output");
        settings.group_parse_exports = false;
        let restored: crate::models::AppSettings = serde_json::from_str(&serde_json::to_string(&settings).unwrap()).unwrap();
        assert_eq!(restored.output_directory, settings.output_directory);
        assert!(!restored.group_parse_exports);
        let legacy: crate::models::AppSettings = serde_json::from_str(r#"{"output_directory":"old-output"}"#).unwrap();
        assert!(legacy.group_parse_exports);
        assert_eq!(destination(&legacy.output_directory, legacy.group_parse_exports), Path::new("old-output").join(GROUP_FOLDER));
    }
    #[test]
    fn distinct_runs_and_legacy_exports_are_recognized_by_contents() {
        let parent = std::env::temp_dir().join(format!("toolkit-exports-{}", std::process::id()));
        fs::create_dir_all(&parent).unwrap();
        let first = create_run_folder(&parent, "20261015_143221").unwrap();
        let second = create_run_folder(&parent, "20261015_143221").unwrap();
        assert_ne!(first, second);
        assert!(first.file_name().unwrap().to_string_lossy().starts_with(RUN_PREFIX));
        let legacy = parent.join("old_demo_export_123");
        fs::create_dir(&legacy).unwrap();
        for dir in [&first, &legacy] {
            assert!(candidate_file(dir).is_none());
            fs::write(dir.join("frag_candidates.ndjson"), "").unwrap();
            assert!(candidate_file(dir).is_some());
        }
        assert!(candidate_file(&parent).is_none());
        assert_eq!(existing_picker_folder(&parent.join("not-created")), Some(parent.clone()));
        fs::remove_dir_all(parent).unwrap();
    }
}
