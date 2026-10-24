//! RandomMac 1.x migration: data import and removal of the v1 elevated logon task.

use std::path::Path;

use chrono::{DateTime, Utc};
use randommac_core::{Settings, legacy_v1, paths};
use randommac_net::ops::DataStore;

use crate::state::V1ImportReport;

/// Imports settings, history and blacklist from `%LOCALAPPDATA%\RandomMac` (Windows only).
/// v1 files are only read, never modified.
pub fn import_v1(store: &DataStore, settings: &mut Settings) -> Option<V1ImportReport> {
    import_v1_from(&paths::legacy_v1_dir()?, store, settings, Utc::now())
}

fn import_v1_from(
    dir: &Path,
    store: &DataStore,
    settings: &mut Settings,
    now: DateTime<Utc>,
) -> Option<V1ImportReport> {
    if !dir.is_dir() {
        return None;
    }
    let mut candidate = settings.clone();
    let data = legacy_v1::read_v1_dir(dir, &mut candidate, now)?;
    if data.found_settings {
        *settings = candidate;
    }
    settings.legacy.imported_from_v1_at = Some(now);

    // Importing again (Settings → Data) must not duplicate entries imported earlier: v1 entries
    // get fresh ids, so compare their content instead.
    let (mut history, _) = store.load_history();
    let key = |e: &randommac_core::HistoryEntry| {
        (e.timestamp, e.adapter_id.clone(), e.previous_mac, e.new_mac)
    };
    let known: std::collections::HashSet<_> = history.entries.iter().map(key).collect();
    let new_entries: Vec<_> = data
        .history
        .into_iter()
        .filter(|e| !known.contains(&key(e)))
        .collect();
    let mut history_count = 0;
    if !new_entries.is_empty() {
        let imported: std::collections::HashSet<_> = new_entries.iter().map(key).collect();
        history.entries.extend(new_entries);
        history.prune(settings.history.policy(), now);
        // Entries older than the retention period are dropped right away; report what is kept.
        history_count = history
            .entries
            .iter()
            .filter(|e| imported.contains(&key(e)))
            .count();
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

#[cfg(test)]
mod tests {
    use randommac_core::AppPaths;
    use randommac_core::settings::Language;

    use super::*;

    fn v1_dir() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        let write =
            |name: &str, json: &str| std::fs::write(dir.path().join(name), json).expect("w");
        write("settings.json", r#"{"Language":"vi","RunAtStartup":true}"#);
        // One recent change and one older than the default 90-day retention.
        let entry = |days_ago: i64, new_mac: &str| {
            let at = (Utc::now() - chrono::Duration::days(days_ago)).to_rfc3339();
            format!(
                r#"{{"AdapterName":"Wi-Fi","AdapterDeviceId":7,"PreviousMac":"A4-C3-F0-11-22-33","NewMac":"{new_mac}","Success":true,"Timestamp":"{at}"}}"#
            )
        };
        write(
            "history.json",
            &format!(
                "[{},{}]",
                entry(1, "02-11-22-33-44-55"),
                entry(200, "06-11-22-33-44-55")
            ),
        );
        write("blacklist.json", r#"{"global":["02-00-00-00-00-01"]}"#);
        dir
    }

    #[test]
    fn imports_v1_data_once() {
        let v1 = v1_dir();
        let data = tempfile::tempdir().expect("tempdir");
        let store = DataStore::new(AppPaths::under(data.path()));
        let mut settings = Settings::default();
        let now = Utc::now();

        let report = import_v1_from(v1.path(), &store, &mut settings, now).expect("report");
        assert!(report.settings && report.autostart);
        assert_eq!((report.history, report.blacklist), (1, 1));
        assert_eq!(settings.language, Language::Vi);
        assert_eq!(settings.legacy.imported_from_v1_at, Some(now));
        assert_eq!(store.load_history().0.entries.len(), 1);
        assert_eq!(store.load_blacklist().0.entries.len(), 1);

        // Importing again (Settings → Data) adds nothing twice.
        let again = import_v1_from(v1.path(), &store, &mut settings, Utc::now()).expect("again");
        assert_eq!((again.history, again.blacklist), (0, 0));
        assert_eq!(store.load_history().0.entries.len(), 1);
        assert_eq!(store.load_blacklist().0.entries.len(), 1);
    }

    #[test]
    fn keeps_settings_when_v1_had_none() {
        let v1 = v1_dir();
        std::fs::remove_file(v1.path().join("settings.json")).expect("rm");
        let data = tempfile::tempdir().expect("tempdir");
        let store = DataStore::new(AppPaths::under(data.path()));
        let mut settings = Settings::default();
        let report = import_v1_from(v1.path(), &store, &mut settings, Utc::now()).expect("report");
        assert!(!report.settings && !report.autostart);
        assert_eq!(settings.language, Settings::default().language);
        assert!(settings.legacy.imported_from_v1_at.is_some());
    }

    #[test]
    fn nothing_to_import() {
        let data = tempfile::tempdir().expect("tempdir");
        let store = DataStore::new(AppPaths::under(data.path()));
        let mut settings = Settings::default();
        let missing = data.path().join("RandomMac");
        assert!(import_v1_from(&missing, &store, &mut settings, Utc::now()).is_none());
        // An empty v1 folder has nothing either.
        std::fs::create_dir(&missing).expect("mkdir");
        assert!(import_v1_from(&missing, &store, &mut settings, Utc::now()).is_none());
        assert_eq!(settings.legacy.imported_from_v1_at, None);
    }
}
