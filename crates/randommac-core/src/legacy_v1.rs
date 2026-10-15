//! One-time import of RandomMac 1.x data (`%LOCALAPPDATA%\RandomMac\*.json`).
//!
//! v1 wrote PascalCase JSON with .NET timestamps; every field is read defensively because v1
//! could persist `null`s and partially written files.

use std::path::Path;

use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use serde_json::Value;

use crate::blacklist::{Blacklist, MacPattern};
use crate::history::{ChangeSource, ChangeStatus, HistoryAction, HistoryEntry};
use crate::mac::MacAddress;
use crate::random::new_id;
use crate::settings::{Language, Settings, Theme};
use crate::storage::read_text;

/// Adapter id used for v1 history entries (v1 only stored a volatile WMI index).
pub const LEGACY_ADAPTER_ID: &str = "legacy-v1";

/// What was found in a v1 data folder.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct V1Data {
    pub found_settings: bool,
    /// v1 "Run at startup" was on (caller re-creates autostart if it can).
    pub run_at_startup: bool,
    pub history: Vec<HistoryEntry>,
    pub blacklist: Blacklist,
}

/// Parses .NET `DateTime` JSON: `…Z`, `…+07:00` or unspecified kind (treated as UTC, like v1).
pub fn parse_v1_timestamp(s: &str) -> Option<DateTime<Utc>> {
    let s = s.trim();
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Some(dt.with_timezone(&Utc));
    }
    NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f")
        .ok()
        .map(|naive| Utc.from_utc_datetime(&naive))
}

fn get<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    v.get(key).filter(|x| !x.is_null())
}

fn mac_field(v: &Value, key: &str) -> Option<MacAddress> {
    get(v, key)?.as_str()?.parse().ok()
}

/// Applies v1 settings onto `settings`; returns whether "Run at startup" was enabled.
pub fn apply_v1_settings(v1: &Value, settings: &mut Settings) -> bool {
    if let Some(lang) = get(v1, "Language").and_then(Value::as_str) {
        settings.language = match lang.trim().to_ascii_lowercase().as_str() {
            "en" => Language::En,
            "vi" => Language::Vi,
            _ => settings.language,
        };
    }
    if let Some(theme) = get(v1, "ThemeMode").and_then(Value::as_str) {
        settings.theme = match theme.trim().to_ascii_lowercase().as_str() {
            "dark" => Theme::Dark,
            "light" => Theme::Light,
            _ => settings.theme,
        };
    }
    let flag = |key: &str| get(v1, key).and_then(Value::as_bool);
    if let Some(b) = flag("MinimizeToTray") {
        settings.close_to_tray = b;
    }
    if let Some(b) = flag("StartMinimized") {
        settings.start_minimized = b;
    }
    if let Some(b) = flag("ShowNotifications") {
        settings.show_notifications = b;
    }
    if let Some(b) = flag("AutoChangeOnStartup") {
        settings.automation.change_on_startup = b;
    }
    if let Some(ids) = get(v1, "AutoChangeAdapterIds").and_then(Value::as_array) {
        // v1 stored PnP device instance ids, which are also v2's adapter ids on Windows.
        settings.automation.startup_adapter_ids = ids
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect();
    }
    let run_at_startup = flag("RunAtStartup").unwrap_or(false);
    settings.start_with_system = run_at_startup;
    settings.sanitize();
    run_at_startup
}

/// Converts the v1 history array.
pub fn parse_v1_history(v1: &Value) -> Vec<HistoryEntry> {
    let Some(items) = v1.as_array() else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|item| {
            let timestamp = get(item, "Timestamp")
                .and_then(Value::as_str)
                .and_then(parse_v1_timestamp)?;
            let success = get(item, "Success")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let new_mac = mac_field(item, "NewMac");
            Some(HistoryEntry {
                id: new_id(),
                timestamp,
                adapter_id: LEGACY_ADAPTER_ID.to_string(),
                adapter_name: get(item, "AdapterName")
                    .and_then(Value::as_str)
                    .unwrap_or("?")
                    .chars()
                    .take(256)
                    .collect(),
                action: HistoryAction::Apply,
                source: ChangeSource::ImportV1,
                status: if success {
                    ChangeStatus::Applied
                } else {
                    ChangeStatus::Failed
                },
                previous_mac: mac_field(item, "PreviousMac"),
                requested_mac: new_mac,
                new_mac: success.then_some(new_mac).flatten(),
                method: None,
                error_code: None,
                error: get(item, "ErrorMessage")
                    .and_then(Value::as_str)
                    .map(|s| s.chars().take(1000).collect()),
            })
        })
        .collect()
}

/// Converts the v1 blacklist (nested `{global, adapters}` or the older flat array).
pub fn parse_v1_blacklist(v1: &Value, now: DateTime<Utc>) -> Blacklist {
    let note = Some("Imported from RandomMac 1.x".to_string());
    let mut out = Blacklist::default();
    let mut add = |raw: &Value, scope: Option<String>| {
        if let Some(pattern) = raw.as_str().and_then(|s| s.parse::<MacPattern>().ok()) {
            // Reserved addresses are built-in rules in v2; add() rejects them.
            let _ = out.add(pattern, scope, note.clone(), now);
        }
    };
    match v1 {
        Value::Array(items) => items.iter().for_each(|m| add(m, None)),
        Value::Object(_) => {
            if let Some(global) = get(v1, "global").and_then(Value::as_array) {
                global.iter().for_each(|m| add(m, None));
            }
            if let Some(adapters) = get(v1, "adapters").and_then(Value::as_object) {
                for (pnp_id, macs) in adapters {
                    if let Some(macs) = macs.as_array() {
                        macs.iter().for_each(|m| add(m, Some(pnp_id.clone())));
                    }
                }
            }
        }
        _ => {}
    }
    out.normalize();
    out
}

fn read_json(path: &Path) -> Option<Value> {
    let text = read_text(path).ok()??;
    serde_json::from_str(&text).ok()
}

/// Reads everything importable from a v1 data folder. Returns `None` if nothing was found.
/// Never modifies or deletes the v1 files.
pub fn read_v1_dir(dir: &Path, settings: &mut Settings, now: DateTime<Utc>) -> Option<V1Data> {
    let settings_json = read_json(&dir.join("settings.json"));
    let history_json = read_json(&dir.join("history.json"));
    let blacklist_json = read_json(&dir.join("blacklist.json"));
    if settings_json.is_none() && history_json.is_none() && blacklist_json.is_none() {
        return None;
    }
    let mut data = V1Data::default();
    if let Some(s) = settings_json {
        data.found_settings = true;
        data.run_at_startup = apply_v1_settings(&s, settings);
    }
    if let Some(h) = history_json {
        data.history = parse_v1_history(&h);
    }
    if let Some(b) = blacklist_json {
        data.blacklist = parse_v1_blacklist(&b, now);
    }
    Some(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamps() {
        let z = parse_v1_timestamp("2026-05-11T16:14:05.1234567Z").expect("utc");
        let off = parse_v1_timestamp("2026-05-11T23:14:05.1234567+07:00").expect("offset");
        assert_eq!(z, off);
        let naive = parse_v1_timestamp("2026-05-11T16:14:05.1234567").expect("unspecified");
        assert_eq!(naive, z);
        assert!(parse_v1_timestamp("yesterday").is_none());
    }

    #[test]
    fn settings_mapping_is_defensive() {
        let v1: Value = serde_json::from_str(
            r#"{
                "Language": "vi", "RunAtStartup": true, "MinimizeToTray": false,
                "StartMinimized": true, "ShowNotifications": null, "ThemeMode": "Light",
                "AutoChangeOnStartup": true,
                "AutoChangeAdapterIds": ["PCI\\VEN_10EC&DEV_8125\\01", null, 5, "PCI\\VEN_10EC&DEV_8125\\01"],
                "LastUpdateCheckedAt": "2026-05-11T23:14:05.1234567+07:00"
            }"#,
        )
        .expect("json");
        let mut s = Settings::default();
        assert!(apply_v1_settings(&v1, &mut s));
        assert_eq!(s.language, Language::Vi);
        assert_eq!(s.theme, Theme::Light);
        assert!(!s.close_to_tray && s.start_minimized && s.show_notifications);
        assert!(s.start_with_system && s.automation.change_on_startup);
        assert_eq!(
            s.automation.startup_adapter_ids,
            vec!["PCI\\VEN_10EC&DEV_8125\\01".to_string()]
        );

        // v1 bug M6: null ids must not break anything.
        let mut s2 = Settings::default();
        let v1_null: Value =
            serde_json::from_str(r#"{"AutoChangeAdapterIds": null, "Language": null}"#)
                .expect("json");
        assert!(!apply_v1_settings(&v1_null, &mut s2));
        assert!(s2.automation.startup_adapter_ids.is_empty());
    }

    #[test]
    fn history_mapping() {
        let v1: Value = serde_json::from_str(
            r#"[
                {"AdapterName":"Wi-Fi","AdapterDeviceId":7,"PreviousMac":"A4-C3-F0-11-22-33","NewMac":"02-11-22-33-44-55","Success":true,"ErrorMessage":null,"Timestamp":"2026-05-11T16:14:05.1234567Z"},
                {"AdapterName":"Ethernet","AdapterDeviceId":1,"PreviousMac":"30-56-0F-09-8F-E4","NewMac":"06-11-22-33-44-55","Success":false,"ErrorMessage":"driver","Timestamp":"2026-05-10T10:00:00"},
                {"AdapterName":"broken"}
            ]"#,
        )
        .expect("json");
        let h = parse_v1_history(&v1);
        assert_eq!(h.len(), 2);
        assert_eq!(h[0].status, ChangeStatus::Applied);
        assert_eq!(
            h[0].new_mac.map(|m| m.to_string()).as_deref(),
            Some("02:11:22:33:44:55")
        );
        assert_eq!(h[1].status, ChangeStatus::Failed);
        assert_eq!(h[1].new_mac, None);
        assert_eq!(h[1].error.as_deref(), Some("driver"));
        assert!(h.iter().all(|e| e.source == ChangeSource::ImportV1));
    }

    #[test]
    fn blacklist_formats() {
        let now = Utc::now();
        let nested: Value = serde_json::from_str(
            r#"{"global":["00-00-00-00-00-00","FF-FF-FF-FF-FF-FF","AA-AA-AA-AA-AA-AA","02-00-00-00-00-01"],
                "adapters":{"PCI\\VEN_1":["06-00-00-00-00-02","garbage"]}}"#,
        )
        .expect("json");
        let bl = parse_v1_blacklist(&nested, now);
        assert_eq!(bl.entries.len(), 2);
        assert!(bl.is_blocked("02:00:00:00:00:01".parse().expect("mac"), None));
        assert!(bl.is_blocked(
            "06:00:00:00:00:02".parse().expect("mac"),
            Some("PCI\\VEN_1")
        ));
        assert!(!bl.is_blocked("06:00:00:00:00:02".parse().expect("mac"), Some("other")));

        let flat: Value = serde_json::from_str(r#"["02-00-00-00-00-09"]"#).expect("json");
        assert_eq!(parse_v1_blacklist(&flat, now).entries.len(), 1);
        assert!(parse_v1_blacklist(&Value::Null, now).entries.is_empty());
    }

    #[test]
    fn reads_directory() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut s = Settings::default();
        assert!(read_v1_dir(dir.path(), &mut s, Utc::now()).is_none());
        std::fs::write(dir.path().join("settings.json"), r#"{"RunAtStartup":true}"#).expect("w");
        std::fs::write(dir.path().join("history.json"), "{ truncated").expect("w");
        let data = read_v1_dir(dir.path(), &mut s, Utc::now()).expect("found");
        assert!(data.found_settings && data.run_at_startup);
        assert!(data.history.is_empty());
        assert!(
            dir.path().join("history.json").exists(),
            "v1 files are never touched"
        );
    }
}
