//! Shared application state managed by Tauri.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use randommac_core::Settings;
use randommac_net::NetworkBackend;
use randommac_net::ops::DataStore;
use serde::Serialize;

use crate::args::Args;
use crate::i18n::Lang;
use crate::logging::LogBuffer;
use crate::updater::UpdaterState;

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V1ImportReport {
    pub settings: bool,
    pub history: usize,
    pub blacklist: usize,
    pub autostart: bool,
}

/// Things that happened during startup and are shown once in the UI.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartupReport {
    pub v1_import: Option<V1ImportReport>,
    pub legacy_task_removed: bool,
    /// Data files that were unreadable and replaced by defaults (originals kept as `.corrupt-*`).
    pub recovered_files: Vec<String>,
}

pub struct AppState {
    pub backend: Arc<dyn NetworkBackend>,
    pub store: DataStore,
    pub settings: Mutex<Settings>,
    /// One MAC change at a time inside this process (the file lock covers other processes).
    pub op_lock: tokio::sync::Mutex<()>,
    /// Adapters changed during this session (for "restore on exit").
    pub session_changed: Mutex<BTreeSet<String>>,
    pub elevated: bool,
    /// Windows without administrator rights: everything except changes works.
    pub read_only: bool,
    pub mock: bool,
    pub args: Args,
    pub tray_available: AtomicBool,
    pub logs: Arc<LogBuffer>,
    pub startup: Mutex<StartupReport>,
    pub updater: Mutex<UpdaterState>,
    pub exiting: AtomicBool,
    pub ui_ready: AtomicBool,
    pub close_hint_shown: AtomicBool,
    pub scheduler: Mutex<Option<tauri::async_runtime::JoinHandle<()>>>,
    pub temp_dir: Option<PathBuf>,
}

impl AppState {
    pub fn settings(&self) -> Settings {
        self.settings.lock().map(|s| s.clone()).unwrap_or_default()
    }

    pub fn lang(&self) -> Lang {
        Lang::resolve(self.settings().language)
    }

    pub fn tray(&self) -> bool {
        self.tray_available.load(Ordering::Relaxed)
    }

    pub fn mark_changed(&self, adapter_id: &str) {
        if let Ok(mut s) = self.session_changed.lock() {
            s.insert(adapter_id.to_string());
        }
    }

    pub fn unmark_changed(&self, adapter_id: &str) {
        if let Ok(mut s) = self.session_changed.lock() {
            s.remove(adapter_id);
        }
    }

    pub fn changed_adapters(&self) -> Vec<String> {
        self.session_changed
            .lock()
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default()
    }
}
