//! Internal state (`state.json`): what each adapter looked like before RandomMac first changed it,
//! so "Restore" can undo exactly our changes even across restarts (v1 bug H5).

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::mac::MacAddress;

pub const STATE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OriginalRecord {
    /// Address the adapter had before our first change.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mac: Option<MacAddress>,
    /// NetworkManager profiles (Linux) modified through this adapter, by connection UUID, with
    /// the `cloned-mac-address` value each had before our first change ("" = unset). Profiles are
    /// tracked separately because the adapter can roam between connections.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub nm_profiles: BTreeMap<String, String>,
    pub recorded_at: DateTime<Utc>,
}

/// What [`AppState::remember`] added, so a change that did not happen can be forgotten again.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[must_use]
pub struct Remembered {
    /// A new record was created for the adapter.
    pub created: bool,
    /// The NetworkManager profile added to an existing record.
    pub nm_profile: Option<String>,
}

impl Remembered {
    pub fn is_empty(&self) -> bool {
        !self.created && self.nm_profile.is_none()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppState {
    pub schema_version: u32,
    pub originals: BTreeMap<String, OriginalRecord>,
}

impl AppState {
    /// Records the configuration `adapter_id` has before a change: its current address the first
    /// time, and the previous `cloned-mac-address` of the active NetworkManager profile
    /// (`nm_profile` = (uuid, value)) the first time that profile is involved. Existing entries are
    /// never overwritten, so the values from before RandomMac's first change win.
    pub fn remember(
        &mut self,
        adapter_id: &str,
        current: Option<MacAddress>,
        nm_profile: Option<(&str, &str)>,
        now: DateTime<Utc>,
    ) -> Remembered {
        self.schema_version = STATE_SCHEMA_VERSION;
        let mut remembered = Remembered::default();
        let record = self
            .originals
            .entry(adapter_id.to_string())
            .or_insert_with(|| {
                remembered.created = true;
                OriginalRecord {
                    mac: current,
                    nm_profiles: BTreeMap::new(),
                    recorded_at: now,
                }
            });
        if let Some((uuid, value)) = nm_profile
            && !record.nm_profiles.contains_key(uuid)
        {
            record
                .nm_profiles
                .insert(uuid.to_string(), value.to_string());
            if !remembered.created {
                remembered.nm_profile = Some(uuid.to_string());
            }
        }
        remembered
    }

    /// Undoes exactly what [`AppState::remember`] added (used when the change did not happen).
    pub fn forget(&mut self, adapter_id: &str, remembered: &Remembered) {
        if remembered.created {
            self.originals.remove(adapter_id);
        } else if let Some(uuid) = &remembered.nm_profile
            && let Some(record) = self.originals.get_mut(adapter_id)
        {
            record.nm_profiles.remove(uuid);
        }
    }

    pub fn original(&self, adapter_id: &str) -> Option<&OriginalRecord> {
        self.originals.get(adapter_id)
    }

    pub fn clear_original(&mut self, adapter_id: &str) -> Option<OriginalRecord> {
        self.originals.remove(adapter_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mac(s: &str) -> Option<MacAddress> {
        Some(s.parse().expect("mac"))
    }

    #[test]
    fn first_values_win() {
        let now = Utc::now();
        let mut s = AppState::default();
        let r = s.remember("eth0", mac("30:56:0F:09:8F:E4"), None, now);
        assert_eq!(
            r,
            Remembered {
                created: true,
                nm_profile: None
            }
        );
        let r = s.remember("eth0", mac("02:00:00:00:00:01"), None, now);
        assert!(r.is_empty());
        assert_eq!(
            s.original("eth0").and_then(|o| o.mac),
            mac("30:56:0F:09:8F:E4")
        );
        let json = serde_json::to_string(&s).expect("ser");
        assert!(!json.contains("nmProfiles"), "{json}");
        assert_eq!(serde_json::from_str::<AppState>(&json).expect("de"), s);
        assert!(s.clear_original("eth0").is_some());
        assert!(s.original("eth0").is_none());
    }

    #[test]
    fn profiles_are_tracked_per_connection() {
        let now = Utc::now();
        let mut s = AppState::default();
        let home = s.remember("wlan0", mac("50:EE:32:8C:BF:42"), Some(("home", "")), now);
        assert!(home.created && home.nm_profile.is_none());
        // Roaming to another profile adds it; the first profile keeps its original value.
        let office = s.remember(
            "wlan0",
            mac("02:11:22:33:44:55"),
            Some(("office", "stable")),
            now,
        );
        assert_eq!(office.nm_profile.as_deref(), Some("office"));
        let again = s.remember("wlan0", None, Some(("home", "02:11:22:33:44:55")), now);
        assert!(again.is_empty());
        let rec = s.original("wlan0").expect("record");
        assert_eq!(rec.nm_profiles.get("home").map(String::as_str), Some(""));
        assert_eq!(
            rec.nm_profiles.get("office").map(String::as_str),
            Some("stable")
        );
        assert_eq!(rec.mac, mac("50:EE:32:8C:BF:42"));

        // A failed change on "office" forgets only that profile.
        s.forget("wlan0", &office);
        let rec = s.original("wlan0").expect("record");
        assert!(rec.nm_profiles.contains_key("home") && !rec.nm_profiles.contains_key("office"));
        // ...and a failed first change forgets the whole record.
        s.forget("wlan0", &home);
        assert!(s.original("wlan0").is_none());
    }
}
