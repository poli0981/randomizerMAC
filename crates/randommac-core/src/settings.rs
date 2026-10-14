//! Persistent user settings (`settings.json`).
//!
//! Loading is deliberately forgiving: unknown fields are ignored, missing fields take their
//! defaults and invalid values are replaced by defaults during [`Settings::sanitize`], so a
//! hand-edited or imported file can never stop the app from starting (v1 bug M6).

use chrono::{DateTime, Utc};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize};

use crate::generator::GenerationMode;
use crate::history::HistoryPolicy;
use crate::mac::MacFormat;

pub const SETTINGS_SCHEMA_VERSION: u32 = 2;

/// Deserializes a value, falling back to its default when the JSON value has the wrong shape.
fn lenient<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: DeserializeOwned + Default,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(serde_json::from_value(value).unwrap_or_default())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Language {
    #[default]
    System,
    En,
    Vi,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

/// How MAC changes are performed on Linux.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LinuxMethod {
    /// NetworkManager for managed devices with an active connection, iproute2 otherwise.
    #[default]
    Auto,
    NetworkManager,
    Iproute2,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AutomationSettings {
    /// Randomize the selected adapters when the app is started with the operating system.
    #[serde(deserialize_with = "lenient")]
    pub change_on_startup: bool,
    #[serde(deserialize_with = "lenient")]
    pub startup_adapter_ids: Vec<String>,
    /// Periodically randomize the selected adapters while the app is running.
    #[serde(deserialize_with = "lenient")]
    pub interval_enabled: bool,
    #[serde(deserialize_with = "lenient_interval")]
    pub interval_minutes: u32,
    #[serde(deserialize_with = "lenient")]
    pub interval_adapter_ids: Vec<String>,
    /// Restore every adapter changed in this session when the app exits.
    #[serde(deserialize_with = "lenient")]
    pub restore_on_exit: bool,
}

fn lenient_interval<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u32, D::Error> {
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(serde_json::from_value(value).unwrap_or(AutomationSettings::default().interval_minutes))
}

impl Default for AutomationSettings {
    fn default() -> Self {
        Self {
            change_on_startup: false,
            startup_adapter_ids: Vec::new(),
            interval_enabled: false,
            interval_minutes: 60,
            interval_adapter_ids: Vec::new(),
            restore_on_exit: false,
        }
    }
}

pub const MIN_INTERVAL_MINUTES: u32 = 5;
pub const MAX_INTERVAL_MINUTES: u32 = 7 * 24 * 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HistorySettings {
    #[serde(deserialize_with = "lenient_max_entries")]
    pub max_entries: u32,
    /// 0 = keep forever.
    #[serde(deserialize_with = "lenient_retention")]
    pub retention_days: u32,
}

fn lenient_max_entries<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u32, D::Error> {
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(serde_json::from_value(value).unwrap_or(HistorySettings::default().max_entries))
}

fn lenient_retention<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u32, D::Error> {
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(serde_json::from_value(value).unwrap_or(HistorySettings::default().retention_days))
}

impl Default for HistorySettings {
    fn default() -> Self {
        let p = HistoryPolicy::default();
        Self {
            max_entries: p.max_entries as u32,
            retention_days: p.retention_days,
        }
    }
}

impl HistorySettings {
    pub fn policy(self) -> HistoryPolicy {
        HistoryPolicy {
            max_entries: self.max_entries as usize,
            retention_days: self.retention_days,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LinuxSettings {
    #[serde(deserialize_with = "lenient")]
    pub method: LinuxMethod,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LegacyInfo {
    /// When settings/history/blacklist were imported from RandomMac 1.x.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub imported_from_v1_at: Option<DateTime<Utc>>,
    /// The v1 elevated logon task was removed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub legacy_task_removed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub schema_version: u32,
    #[serde(deserialize_with = "lenient")]
    pub language: Language,
    #[serde(deserialize_with = "lenient")]
    pub theme: Theme,
    pub start_with_system: bool,
    pub start_minimized: bool,
    pub close_to_tray: bool,
    pub show_notifications: bool,
    pub confirm_before_apply: bool,
    pub auto_check_updates: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_update_check: Option<DateTime<Utc>>,
    #[serde(deserialize_with = "lenient")]
    pub generation_mode: GenerationMode,
    #[serde(deserialize_with = "lenient")]
    pub mac_format: MacFormat,
    pub show_virtual_adapters: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_adapter_id: Option<String>,
    #[serde(deserialize_with = "lenient")]
    pub automation: AutomationSettings,
    #[serde(deserialize_with = "lenient")]
    pub linux: LinuxSettings,
    #[serde(deserialize_with = "lenient")]
    pub history: HistorySettings,
    #[serde(deserialize_with = "lenient")]
    pub legacy: LegacyInfo,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            schema_version: SETTINGS_SCHEMA_VERSION,
            language: Language::System,
            theme: Theme::System,
            start_with_system: false,
            start_minimized: false,
            close_to_tray: true,
            show_notifications: true,
            confirm_before_apply: false,
            auto_check_updates: true,
            last_update_check: None,
            generation_mode: GenerationMode::Random,
            mac_format: MacFormat::default(),
            show_virtual_adapters: false,
            selected_adapter_id: None,
            automation: AutomationSettings::default(),
            linux: LinuxSettings::default(),
            history: HistorySettings::default(),
            legacy: LegacyInfo::default(),
        }
    }
}

fn clean_ids(ids: &mut Vec<String>) {
    let mut seen = std::collections::HashSet::new();
    ids.retain(|id| {
        let id = id.trim();
        !id.is_empty() && id.len() <= 512 && seen.insert(id.to_string())
    });
    ids.truncate(64);
}

impl Settings {
    /// Clamps and normalizes every field so the struct is always safe to use.
    pub fn sanitize(&mut self) {
        self.schema_version = SETTINGS_SCHEMA_VERSION;
        let a = &mut self.automation;
        clean_ids(&mut a.startup_adapter_ids);
        clean_ids(&mut a.interval_adapter_ids);
        a.interval_minutes = a
            .interval_minutes
            .clamp(MIN_INTERVAL_MINUTES, MAX_INTERVAL_MINUTES);
        self.history.max_entries = self.history.max_entries.clamp(10, 10_000);
        self.history.retention_days = self.history.retention_days.min(3650);
        if let GenerationMode::Vendor { vendor_id } = &self.generation_mode
            && crate::oui::vendor_by_id(vendor_id).is_none()
        {
            self.generation_mode = GenerationMode::Random;
        }
        if self
            .selected_adapter_id
            .as_deref()
            .is_some_and(|s| s.trim().is_empty() || s.len() > 512)
        {
            self.selected_adapter_id = None;
        }
    }

    /// Parses settings JSON and sanitizes the result.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        let mut s: Settings = serde_json::from_str(json)?;
        s.sanitize();
        Ok(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_round_trip() {
        let s = Settings::default();
        let json = serde_json::to_string_pretty(&s).expect("ser");
        assert!(json.contains("\"closeToTray\": true"));
        assert!(json.contains("\"generationMode\""));
        assert_eq!(Settings::from_json(&json).expect("de"), s);
        assert_eq!(Settings::from_json("{}").expect("empty object"), s);
    }

    #[test]
    fn tolerates_bad_values() {
        let json = r#"{
            "language": "klingon",
            "theme": 42,
            "closeToTray": false,
            "generationMode": {"kind": "vendor", "vendorId": "acme"},
            "automation": {"startupAdapterIds": null, "intervalMinutes": 1},
            "history": {"maxEntries": 999999, "retentionDays": 99999},
            "linux": {"method": "carrier-pigeon"},
            "unknownField": true
        }"#;
        let s = Settings::from_json(json).expect("lenient parse");
        assert_eq!(s.language, Language::System);
        assert_eq!(s.theme, Theme::System);
        assert!(!s.close_to_tray);
        assert_eq!(s.generation_mode, GenerationMode::Random);
        assert!(s.automation.startup_adapter_ids.is_empty());
        assert_eq!(s.automation.interval_minutes, MIN_INTERVAL_MINUTES);
        assert_eq!(s.history.max_entries, 10_000);
        assert_eq!(s.history.retention_days, 3650);
        assert_eq!(s.linux.method, LinuxMethod::Auto);
    }

    #[test]
    fn sanitize_dedupes_ids() {
        let mut s = Settings::default();
        s.automation.startup_adapter_ids = vec![
            "a".into(),
            " ".into(),
            "a".into(),
            "b".into(),
            String::new(),
        ];
        s.selected_adapter_id = Some("  ".into());
        s.sanitize();
        assert_eq!(s.automation.startup_adapter_ids, vec!["a", "b"]);
        assert_eq!(s.selected_adapter_id, None);
    }
}
