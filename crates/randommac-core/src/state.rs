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
    /// NetworkManager connection whose profile we modified (Linux).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nm_connection_uuid: Option<String>,
    /// Previous value of the profile's `cloned-mac-address` ("" = unset).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nm_cloned_mac: Option<String>,
    pub recorded_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppState {
    pub schema_version: u32,
    pub originals: BTreeMap<String, OriginalRecord>,
}

impl AppState {
    /// Stores the original configuration of `adapter_id` unless one is already recorded.
    /// Returns `true` when a new record was stored.
    pub fn record_original(&mut self, adapter_id: &str, record: OriginalRecord) -> bool {
        self.schema_version = STATE_SCHEMA_VERSION;
        if self.originals.contains_key(adapter_id) {
            return false;
        }
        self.originals.insert(adapter_id.to_string(), record);
        true
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

    #[test]
    fn first_record_wins() {
        let now = Utc::now();
        let mut s = AppState::default();
        let first = OriginalRecord {
            mac: Some("30:56:0F:09:8F:E4".parse().expect("mac")),
            nm_connection_uuid: None,
            nm_cloned_mac: None,
            recorded_at: now,
        };
        assert!(s.record_original("eth0", first.clone()));
        let second = OriginalRecord {
            mac: Some("02:00:00:00:00:01".parse().expect("mac")),
            ..first.clone()
        };
        assert!(!s.record_original("eth0", second));
        assert_eq!(s.original("eth0"), Some(&first));
        let json = serde_json::to_string(&s).expect("ser");
        assert_eq!(serde_json::from_str::<AppState>(&json).expect("de"), s);
        assert_eq!(s.clear_original("eth0"), Some(first));
        assert!(s.original("eth0").is_none());
    }
}
