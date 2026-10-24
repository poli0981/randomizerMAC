//! Locations of RandomMac's files. Shared by the GUI and the CLI so both see the same data.

use std::io;
use std::path::{Path, PathBuf};

/// Application identifier (also the Tauri bundle identifier).
pub const APP_ID: &str = "io.github.poli0981.randommac";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppPaths {
    /// settings.json, blacklist.json
    pub config_dir: PathBuf,
    /// history.json, state.json, operation.lock
    pub data_dir: PathBuf,
    pub log_dir: PathBuf,
}

impl AppPaths {
    /// Platform default locations:
    /// * Windows: `%LOCALAPPDATA%\io.github.poli0981.randommac\`
    /// * Linux: `$XDG_CONFIG_HOME/io.github.poli0981.randommac/` and
    ///   `$XDG_DATA_HOME/io.github.poli0981.randommac/`
    pub fn resolve() -> io::Result<Self> {
        let missing = || io::Error::new(io::ErrorKind::NotFound, "no home directory");
        if cfg!(windows) {
            let base = dirs::data_local_dir().ok_or_else(missing)?.join(APP_ID);
            Ok(Self::under(&base))
        } else {
            let config_dir = dirs::config_dir().ok_or_else(missing)?.join(APP_ID);
            let data_dir = dirs::data_dir().ok_or_else(missing)?.join(APP_ID);
            let log_dir = data_dir.join("logs");
            Ok(Self {
                config_dir,
                data_dir,
                log_dir,
            })
        }
    }

    /// Everything under one directory (portable use and tests).
    pub fn under(root: &Path) -> Self {
        Self {
            config_dir: root.to_path_buf(),
            data_dir: root.to_path_buf(),
            log_dir: root.join("logs"),
        }
    }

    pub fn settings_file(&self) -> PathBuf {
        self.config_dir.join("settings.json")
    }

    pub fn blacklist_file(&self) -> PathBuf {
        self.config_dir.join("blacklist.json")
    }

    pub fn history_file(&self) -> PathBuf {
        self.data_dir.join("history.json")
    }

    pub fn state_file(&self) -> PathBuf {
        self.data_dir.join("state.json")
    }

    pub fn lock_file(&self) -> PathBuf {
        self.data_dir.join("operation.lock")
    }
}

/// Data folder of RandomMac 1.x (`%LOCALAPPDATA%\RandomMac`, Windows only).
pub fn legacy_v1_dir() -> Option<PathBuf> {
    if cfg!(windows) {
        dirs::data_local_dir().map(|d| d.join("RandomMac"))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout() {
        let p = AppPaths::under(Path::new("root"));
        assert_eq!(p.settings_file(), Path::new("root").join("settings.json"));
        assert_eq!(p.history_file(), Path::new("root").join("history.json"));
        assert_eq!(p.log_dir, Path::new("root").join("logs"));
        let resolved = AppPaths::resolve().expect("home dir available in tests");
        assert!(resolved.config_dir.ends_with(APP_ID));
        assert!(resolved.data_dir.ends_with(APP_ID));
        assert!(resolved.log_dir.starts_with(&resolved.data_dir));
        assert_eq!(p.blacklist_file(), Path::new("root").join("blacklist.json"));
        assert_eq!(p.state_file(), Path::new("root").join("state.json"));
        assert_eq!(p.lock_file(), Path::new("root").join("operation.lock"));
    }

    #[test]
    fn legacy_folder_only_on_windows() {
        let dir = legacy_v1_dir();
        assert_eq!(dir.is_some(), cfg!(windows));
        assert!(dir.is_none_or(|d| d.ends_with("RandomMac")));
    }
}
