//! App-owned storage names and restartable migration from the former name.
use anyhow::{bail, Context, Result};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub const APP_FOLDER: &str = "TF2 Demo Toolkit";
const LEGACY_FOLDER: &str = "TF2FragDemoHelper";
const MIGRATION: &str = "name_migration.json";

pub fn data_root() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join(APP_FOLDER)
}

pub fn config_root() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join(APP_FOLDER)
}

pub fn cache_root() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join(APP_FOLDER)
}

/// Run before settings, recovery, or background workers open any app files.
pub fn migrate() -> Result<()> {
    let mut roots = vec![data_root(), config_root(), cache_root()];
    roots.sort();
    // Windows cache and local data commonly share a directory.
    roots.dedup();
    // Do not move paths while an older Toolkit/TF2 session may still use them.
    for new in &roots {
        crate::recording::ensure_profile_inactive(
            &new.with_file_name(LEGACY_FOLDER)
                .join("active_recording_profile.json"),
        )?;
        if new.join(MIGRATION).is_file() {
            crate::recording::ensure_profile_inactive(&new.join("active_recording_profile.json"))?;
        }
    }
    for new in &roots {
        migrate_root(&new.with_file_name(LEGACY_FOLDER), &new)?;
    }
    if roots.iter().any(|root| root.join(MIGRATION).is_file()) {
        // Settings in Roaming/AppData may reference sessions in Local/AppData.
        // Keep the journal until every root has had every path mapping applied.
        for root in &roots {
            if root.is_dir() {
                for new in &roots {
                    rebase_documents(root, &new.with_file_name(LEGACY_FOLDER), new)?;
                }
            }
        }
        for root in &roots {
            let marker = root.join(MIGRATION);
            if marker.is_file() {
                fs::remove_file(marker)?;
            }
        }
    }
    Ok(())
}

fn migrate_root(old: &Path, new: &Path) -> Result<()> {
    if old.is_dir() {
        if new.is_dir() && fs::read_dir(new)?.next().is_none() {
            fs::remove_dir(new)?;
        }
        if new.exists() {
            bail!("Both {} and {} contain app data. No files were overwritten; resolve the duplicate folders before continuing.", old.display(), new.display());
        }
        fs::write(old.join(MIGRATION), serde_json::to_vec(old)?)?;
        fs::rename(old, new)
            .with_context(|| format!("could not migrate {} to {}", old.display(), new.display()))?;
    }
    let marker = new.join(MIGRATION);
    if marker.is_file() {
        let source: PathBuf = serde_json::from_slice(&fs::read(&marker)?)?;
        rename_owned_entries(new)?;
        rebase_documents(new, &source, new)?;
    }
    Ok(())
}

fn branded_name(name: &str) -> String {
    name.strip_prefix("tf2fragdemohelper")
        .map(|suffix| format!("tf2_demo_toolkit{suffix}"))
        .unwrap_or_else(|| name.to_owned())
}

fn rename_owned_entries(root: &Path) -> Result<()> {
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            continue;
        }
        let path = entry.path();
        if kind.is_dir() {
            rename_owned_entries(&path)?;
        }
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        let branded = branded_name(name);
        if branded != name {
            let destination = path.with_file_name(branded);
            if destination.exists() {
                bail!(
                    "Migration conflict: {} already exists",
                    destination.display()
                );
            }
            fs::rename(path, destination)?;
        }
    }
    Ok(())
}

fn rebase_value(value: &mut serde_json::Value, old: &Path, new: &Path) {
    match value {
        serde_json::Value::String(text) => {
            let path = Path::new(text);
            if let Ok(relative) = path.strip_prefix(old).or_else(|_| path.strip_prefix(new)) {
                let mut destination = new.to_owned();
                for component in relative.components() {
                    destination.push(branded_name(&component.as_os_str().to_string_lossy()));
                }
                *text = destination.to_string_lossy().into_owned();
            }
        }
        serde_json::Value::Array(values) => {
            for v in values {
                rebase_value(v, old, new);
            }
        }
        serde_json::Value::Object(values) => {
            for v in values.values_mut() {
                rebase_value(v, old, new);
            }
        }
        _ => {}
    }
}

fn rebase_documents(directory: &Path, old: &Path, new: &Path) -> Result<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            continue;
        }
        let path = entry.path();
        if kind.is_dir() {
            rebase_documents(&path, old, new)?;
            continue;
        }
        if path.file_name().is_some_and(|name| name == MIGRATION) {
            continue;
        }
        let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        if !matches!(extension, "json" | "ndjson") {
            continue;
        }
        let original = fs::read_to_string(&path)?;
        let mut changed = false;
        let mut rewrite = |line: &str| -> String {
            // Keep damaged/incomplete recovery records byte-for-byte.
            let Ok(mut value) = serde_json::from_str::<serde_json::Value>(line) else {
                return line.to_owned();
            };
            let before = value.clone();
            rebase_value(&mut value, old, new);
            if value == before {
                return line.to_owned();
            }
            changed = true;
            serde_json::to_string(&value).expect("JSON value serialization")
        };
        let output = if extension == "ndjson" {
            original
                .split_inclusive('\n')
                .map(|line| {
                    let mut result = rewrite(line.trim_end_matches('\n'));
                    if line.ends_with('\n') {
                        result.push('\n');
                    }
                    result
                })
                .collect::<String>()
        } else {
            rewrite(&original)
        };
        if changed {
            replace_file(&path, output.as_bytes())?;
        }
    }
    Ok(())
}

fn replace_file(path: &Path, contents: &[u8]) -> Result<()> {
    let temporary = path.with_extension("migration-tmp");
    fs::write(&temporary, contents)?;
    #[cfg(not(windows))]
    fs::rename(&temporary, path)?;
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        #[link(name = "kernel32")]
        extern "system" {
            fn MoveFileExW(from: *const u16, to: *const u16, flags: u32) -> i32;
        }
        let from: Vec<u16> = temporary.as_os_str().encode_wide().chain(Some(0)).collect();
        let to: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        if unsafe { MoveFileExW(from.as_ptr(), to.as_ptr(), 1 | 8) } == 0 {
            return Err(std::io::Error::last_os_error().into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn migration_preserves_settings_and_rebases_recovery_paths_and_is_repeatable() {
        let root = std::env::temp_dir().join(format!("toolkit-migration-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let old = root.join(LEGACY_FOLDER);
        let new = root.join(APP_FOLDER);
        let session = old.join("Recording Sessions/tf2fragdemohelper_batch_test");
        fs::create_dir_all(&session).unwrap();
        fs::write(old.join("settings.json"), b"{\"fps\":240}").unwrap();
        fs::write(session.join("session.json"), serde_json::to_vec(&serde_json::json!({"working_path":session.join("frames"),"source_demo":"D:/demos/original.dem"})).unwrap()).unwrap();
        fs::write(
            old.join("recorded_clip_index.ndjson"),
            format!(
                "{}\n",
                serde_json::json!({"output":session.join("clip.mp4")})
            ),
        )
        .unwrap();
        migrate_root(&old, &new).unwrap();
        migrate_root(&old, &new).unwrap();
        assert!(!old.exists());
        assert_eq!(
            fs::read(new.join("settings.json")).unwrap(),
            b"{\"fps\":240}"
        );
        let session = new.join("Recording Sessions/tf2_demo_toolkit_batch_test");
        let data: serde_json::Value =
            serde_json::from_slice(&fs::read(session.join("session.json")).unwrap()).unwrap();
        assert_eq!(
            data["working_path"],
            session.join("frames").to_string_lossy().as_ref()
        );
        assert_eq!(data["source_demo"], "D:/demos/original.dem");
        assert!(fs::read_to_string(new.join("recorded_clip_index.ndjson"))
            .unwrap()
            .contains("tf2_demo_toolkit_batch_test"));
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn conflicting_roots_are_never_overwritten() {
        let root =
            std::env::temp_dir().join(format!("toolkit-migration-conflict-{}", std::process::id()));
        let old = root.join(LEGACY_FOLDER);
        let new = root.join(APP_FOLDER);
        fs::create_dir_all(&old).unwrap();
        fs::create_dir_all(&new).unwrap();
        fs::write(old.join("settings.json"), b"old").unwrap();
        fs::write(new.join("settings.json"), b"new").unwrap();
        assert!(migrate_root(&old, &new).is_err());
        assert_eq!(fs::read(old.join("settings.json")).unwrap(), b"old");
        assert_eq!(fs::read(new.join("settings.json")).unwrap(), b"new");
        fs::remove_dir_all(root).unwrap();
    }
}
