//! Crash-safe JSON persistence.
//!
//! * Writes go to a temporary file in the same directory, are flushed to disk and then renamed
//!   over the target, so a crash never leaves a truncated file (v1 bugs M4/M5).
//! * A file that cannot be parsed is moved aside as `<name>.corrupt-<timestamp>` (at most three
//!   are kept) and defaults are used; saving keeps working afterwards.
//! * On Windows the app runs elevated while its data lives in the user profile, so a data
//!   directory that is a symbolic link or junction is refused.

use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::Serialize;
use serde::de::DeserializeOwned;

/// Files larger than this are treated as corrupt rather than loaded.
pub const MAX_FILE_BYTES: u64 = 8 * 1024 * 1024;
const KEEP_CORRUPT: usize = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadStatus {
    Loaded,
    Missing,
    /// The file was unreadable; it was moved to `backup` (if possible) and defaults were used.
    Recovered {
        backup: Option<PathBuf>,
        reason: String,
    },
}

/// Creates `dir` (and parents) and makes sure it is a real directory.
pub fn ensure_dir(dir: &Path) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    let meta = fs::symlink_metadata(dir)?;
    #[cfg(windows)]
    if meta.file_type().is_symlink() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!(
                "refusing to use {} because it is a symbolic link or junction",
                dir.display()
            ),
        ));
    }
    if !meta.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{} is not a directory", dir.display()),
        ));
    }
    Ok(())
}

/// Atomically replaces `path` with `bytes`.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let dir = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path has no parent"))?;
    ensure_dir(dir)?;
    let mut tmp = tempfile::Builder::new()
        .prefix(".tmp-")
        .suffix(".part")
        .tempfile_in(dir)?;
    tmp.write_all(bytes)?;
    tmp.as_file().sync_all()?;
    tmp.persist(path).map_err(|e| e.error)?;
    #[cfg(unix)]
    if let Ok(d) = fs::File::open(dir) {
        // Make the rename itself durable.
        let _ = d.sync_all();
    }
    Ok(())
}

/// Serializes `value` as pretty JSON and writes it atomically.
pub fn save_json<T: Serialize>(path: &Path, value: &T) -> io::Result<()> {
    let mut json = serde_json::to_vec_pretty(value).map_err(io::Error::other)?;
    json.push(b'\n');
    write_atomic(path, &json)
}

/// Reads a UTF-8 text file with a size cap. `Ok(None)` when it does not exist.
pub fn read_text(path: &Path) -> io::Result<Option<String>> {
    let file = match fs::File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e),
    };
    let len = file.metadata()?.len();
    if len > MAX_FILE_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("file is too large ({len} bytes)"),
        ));
    }
    let mut text = String::new();
    file.take(MAX_FILE_BYTES + 1).read_to_string(&mut text)?;
    Ok(Some(text.trim_start_matches('\u{feff}').to_string()))
}

/// Loads `path` with a custom parser, recovering from unreadable files.
pub fn load_with<T, F>(path: &Path, parse: F) -> (T, LoadStatus)
where
    T: Default,
    F: FnOnce(&str) -> Result<T, String>,
{
    let outcome = match read_text(path) {
        Ok(None) => return (T::default(), LoadStatus::Missing),
        Ok(Some(text)) if text.trim().is_empty() => return (T::default(), LoadStatus::Missing),
        Ok(Some(text)) => parse(&text),
        Err(e) => Err(e.to_string()),
    };
    match outcome {
        Ok(value) => (value, LoadStatus::Loaded),
        Err(reason) => {
            let backup = quarantine(path).ok();
            tracing::warn!(path = %path.display(), %reason, ?backup, "unreadable data file; using defaults");
            (T::default(), LoadStatus::Recovered { backup, reason })
        }
    }
}

/// Loads a JSON file into `T`, recovering from unreadable files.
pub fn load_json<T: DeserializeOwned + Default>(path: &Path) -> (T, LoadStatus) {
    load_with(path, |text| {
        serde_json::from_str(text).map_err(|e| e.to_string())
    })
}

fn quarantine(path: &Path) -> io::Result<PathBuf> {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "bad file name"))?
        .to_string();
    let stamp = Utc::now().format("%Y%m%d%H%M%S%3f");
    let target = path.with_file_name(format!("{name}.corrupt-{stamp}"));
    fs::rename(path, &target)?;
    prune_corrupt(path, &name);
    Ok(target)
}

/// Keeps only the newest `KEEP_CORRUPT` quarantined copies of `name`. Only files that match the
/// exact naming pattern are ever deleted.
fn prune_corrupt(path: &Path, name: &str) {
    let Some(dir) = path.parent() else { return };
    let prefix = format!("{name}.corrupt-");
    let Ok(read) = fs::read_dir(dir) else { return };
    let mut found: Vec<(String, PathBuf)> = read
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
        .filter_map(|e| {
            let file_name = e.file_name().to_str()?.to_string();
            let suffix = file_name.strip_prefix(&prefix)?;
            (suffix.len() == 17 && suffix.chars().all(|c| c.is_ascii_digit()))
                .then(|| (file_name.clone(), e.path()))
        })
        .collect();
    found.sort();
    while found.len() > KEEP_CORRUPT {
        let (_, p) = found.remove(0);
        let _ = fs::remove_file(p);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
    struct Doc {
        value: u32,
    }

    #[test]
    fn save_and_load_round_trip() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("nested").join("doc.json");
        let (doc, status) = load_json::<Doc>(&path);
        assert_eq!((doc, status), (Doc::default(), LoadStatus::Missing));

        save_json(&path, &Doc { value: 7 }).expect("save");
        save_json(&path, &Doc { value: 8 }).expect("overwrite");
        assert_eq!(
            load_json::<Doc>(&path),
            (Doc { value: 8 }, LoadStatus::Loaded)
        );
        let leftovers: Vec<_> = fs::read_dir(path.parent().expect("parent"))
            .expect("read dir")
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().starts_with(".tmp-"))
            .collect();
        assert!(
            leftovers.is_empty(),
            "temporary files must not be left behind"
        );
    }

    #[test]
    fn corrupt_files_are_quarantined_and_bounded() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("doc.json");
        for i in 0..5 {
            fs::write(&path, format!("{{ not json {i}")).expect("write");
            let (doc, status) = load_json::<Doc>(&path);
            assert_eq!(doc, Doc::default());
            assert!(matches!(
                status,
                LoadStatus::Recovered {
                    backup: Some(_),
                    ..
                }
            ));
            assert!(!path.exists());
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let corrupt = fs::read_dir(dir.path())
            .expect("read dir")
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().contains(".corrupt-"))
            .count();
        assert_eq!(corrupt, KEEP_CORRUPT);
        // Saving still works after recovery.
        save_json(&path, &Doc { value: 1 }).expect("save after recovery");
        assert_eq!(load_json::<Doc>(&path).0, Doc { value: 1 });
    }

    #[test]
    fn empty_and_bom_files() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("doc.json");
        fs::write(&path, "   ").expect("write");
        assert_eq!(load_json::<Doc>(&path).1, LoadStatus::Missing);
        fs::write(&path, "\u{feff}{\"value\": 3}").expect("write");
        assert_eq!(
            load_json::<Doc>(&path),
            (Doc { value: 3 }, LoadStatus::Loaded)
        );
    }

    #[test]
    fn oversized_files_are_rejected() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("big.json");
        let f = fs::File::create(&path).expect("create");
        f.set_len(MAX_FILE_BYTES + 1).expect("set_len");
        drop(f);
        assert!(read_text(&path).is_err());
    }

    #[cfg(windows)]
    #[test]
    fn junction_data_dir_is_refused() {
        let dir = tempfile::tempdir().expect("tempdir");
        let target = dir.path().join("target");
        fs::create_dir(&target).expect("mkdir");
        let link = dir.path().join("link");
        let status = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(&link)
            .arg(&target)
            .output()
            .expect("mklink");
        if status.status.success() {
            let err = ensure_dir(&link).expect_err("junction must be refused");
            assert_eq!(err.kind(), io::ErrorKind::PermissionDenied);
        }
    }
}
