//! RandomMac 1.x migration: data import and removal of the v1 elevated logon task.

use chrono::Utc;
use randommac_core::{Settings, legacy_v1, paths};
use randommac_net::ops::DataStore;

use crate::state::V1ImportReport;

/// Imports settings, history and blacklist from `%LOCALAPPDATA%\RandomMac` (Windows only).
/// v1 files are only read, never modified.
pub fn import_v1(store: &DataStore, settings: &mut Settings) -> Option<V1ImportReport> {
    let dir = paths::legacy_v1_dir()?;
    if !dir.is_dir() {
        return None;
    }
    let now = Utc::now();
    let mut candidate = settings.clone();
    let data = legacy_v1::read_v1_dir(&dir, &mut candidate, now)?;
    if data.found_settings {
        *settings = candidate;
    }
    settings.legacy.imported_from_v1_at = Some(now);

    let history_count = data.history.len();
    if history_count > 0 {
        let (mut history, _) = store.load_history();
        history.entries.extend(data.history);
        history.prune(settings.history.policy(), now);
        if let Err(e) = store.save_history(&history) {
            tracing::warn!("could not save imported history: {e}");
        }
    }
    let (mut blacklist, _) = store.load_blacklist();
    let blacklist_count = blacklist.merge(data.blacklist, now);
    if blacklist_count > 0
        && let Err(e) = store.save_blacklist(&blacklist)
    {
        tracing::warn!("could not save imported blacklist: {e}");
    }
    tracing::info!(
        settings = data.found_settings,
        history = history_count,
        blacklist = blacklist_count,
        "imported RandomMac 1.x data"
    );
    Some(V1ImportReport {
        settings: data.found_settings,
        history: history_count,
        blacklist: blacklist_count,
        autostart: data.run_at_startup,
    })
}

/// Removes v1 leftovers that are a security risk. Returns whether anything was removed.
pub fn cleanup_legacy() -> bool {
    #[cfg(windows)]
    {
        crate::autostart::remove_legacy_v1()
    }
    #[cfg(not(windows))]
    {
        false
    }
}
