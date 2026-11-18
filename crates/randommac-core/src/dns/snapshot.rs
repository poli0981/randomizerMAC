//! What an adapter's DNS settings were before RandomMac first changed them (`dns-state.json`), so
//! "Restore original DNS" puts back exactly that, across restarts. The file is separate from
//! `state.json` (MAC originals), so a problem with one never costs the other.

use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize};

use super::model::DnsConfig;
use crate::timefmt::{self, OffsetDateTime};

pub const DNS_STATE_SCHEMA_VERSION: u32 = 1;

/// One address family's settings as Windows keeps them for an interface.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct WinFamilySnapshot {
    /// Static servers ("" = from the network).
    pub name_server: String,
    pub doh: Vec<DohServerSnapshot>,
    /// Servers set through the Settings app's "profile" fields (Windows 11).
    pub profile_name_server: String,
    pub profile_doh: Vec<DohServerSnapshot>,
}

/// The DNS-over-HTTPS settings of one server (`ServerIndex` into its server list).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DohServerSnapshot {
    pub server_index: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
    pub flags: u64,
}

/// A NetworkManager profile's DNS properties, exactly as nmcli prints them.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct NmDnsProfile {
    pub ipv4_dns: String,
    pub ipv4_ignore_auto_dns: String,
    pub ipv6_dns: String,
    pub ipv6_ignore_auto_dns: String,
    pub dns_over_tls: String,
}

/// An adapter's DNS settings, in the backend's own terms.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum DnsSnapshot {
    Windows {
        ipv4: WinFamilySnapshot,
        ipv6: WinFamilySnapshot,
    },
    NetworkManager {
        uuid: String,
        profile: NmDnsProfile,
    },
    Mock {
        config: DnsConfig,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsOriginalRecord {
    /// Windows and the mock backend: the interface's settings before the first change.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interface: Option<DnsSnapshot>,
    /// Linux: the NetworkManager profiles changed through this adapter, by connection UUID (the
    /// adapter can roam between profiles), each with its values before the first change.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub nm_profiles: BTreeMap<String, NmDnsProfile>,
    #[serde(with = "timefmt::rfc3339")]
    pub recorded_at: OffsetDateTime,
}

/// What [`DnsState::remember`] added, so a change that did not happen can be forgotten again.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[must_use]
pub struct DnsRemembered {
    pub created: bool,
    pub interface: bool,
    pub nm_profile: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DnsState {
    pub schema_version: u32,
    #[serde(deserialize_with = "lenient_records")]
    pub originals: BTreeMap<String, DnsOriginalRecord>,
}

/// Reads the records one by one: a record that does not parse is dropped, the others are kept.
fn lenient_records<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<BTreeMap<String, DnsOriginalRecord>, D::Error> {
    let value = serde_json::Value::deserialize(deserializer)?;
    let serde_json::Value::Object(map) = value else {
        return Ok(BTreeMap::new());
    };
    Ok(map
        .into_iter()
        .filter_map(|(id, v)| serde_json::from_value(v).ok().map(|r| (id, r)))
        .collect())
}

impl DnsState {
    /// Records the settings `adapter_id` has before a change. Existing values are never replaced,
    /// so what was there before RandomMac's first change wins.
    pub fn remember(
        &mut self,
        adapter_id: &str,
        snapshot: &DnsSnapshot,
        now: OffsetDateTime,
    ) -> DnsRemembered {
        self.schema_version = DNS_STATE_SCHEMA_VERSION;
        let mut remembered = DnsRemembered::default();
        let record = self
            .originals
            .entry(adapter_id.to_string())
            .or_insert_with(|| {
                remembered.created = true;
                DnsOriginalRecord {
                    interface: None,
                    nm_profiles: BTreeMap::new(),
                    recorded_at: now,
                }
            });
        match snapshot {
            DnsSnapshot::NetworkManager { uuid, profile } => {
                if !record.nm_profiles.contains_key(uuid) {
                    record.nm_profiles.insert(uuid.clone(), profile.clone());
                    remembered.nm_profile = Some(uuid.clone());
                }
            }
            other => {
                if record.interface.is_none() {
                    record.interface = Some(other.clone());
                    remembered.interface = true;
                }
            }
        }
        remembered
    }

    /// Undoes exactly what [`DnsState::remember`] added (the change did not happen).
    pub fn forget(&mut self, adapter_id: &str, remembered: &DnsRemembered) {
        if remembered.created {
            self.originals.remove(adapter_id);
            return;
        }
        if let Some(record) = self.originals.get_mut(adapter_id) {
            if remembered.interface {
                record.interface = None;
            }
            if let Some(uuid) = &remembered.nm_profile {
                record.nm_profiles.remove(uuid);
            }
        }
    }

    pub fn original(&self, adapter_id: &str) -> Option<&DnsOriginalRecord> {
        self.originals.get(adapter_id)
    }

    pub fn clear(&mut self, adapter_id: &str) -> Option<DnsOriginalRecord> {
        self.originals.remove(adapter_id)
    }

    /// After a change: when the adapter is back to exactly its original settings (say
    /// "Automatic" on an adapter that was automatic), drops what no longer needs restoring.
    /// Returns true when the whole record is gone.
    pub fn clear_if_original(&mut self, adapter_id: &str, current: &DnsSnapshot) -> bool {
        let Some(record) = self.originals.get_mut(adapter_id) else {
            return false;
        };
        match current {
            DnsSnapshot::NetworkManager { uuid, profile } => {
                if record.nm_profiles.get(uuid) == Some(profile) {
                    record.nm_profiles.remove(uuid);
                }
            }
            other => {
                if record.interface.as_ref() == Some(other) {
                    record.interface = None;
                }
            }
        }
        if record.interface.is_none() && record.nm_profiles.is_empty() {
            self.originals.remove(adapter_id);
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;

    use super::*;
    use crate::dns::model::{DnsEncryption, FamilyDns};

    fn mock(server: Option<[u8; 4]>) -> DnsSnapshot {
        let mut config = DnsConfig::automatic();
        if let Some(octets) = server {
            config.ipv4 = FamilyDns::Static {
                servers: vec![Ipv4Addr::from(octets)],
            };
            config.encryption = DnsEncryption::None;
        }
        DnsSnapshot::Mock { config }
    }

    fn nm(uuid: &str, dns: &str) -> DnsSnapshot {
        DnsSnapshot::NetworkManager {
            uuid: uuid.into(),
            profile: NmDnsProfile {
                ipv4_dns: dns.into(),
                ipv4_ignore_auto_dns: "no".into(),
                dns_over_tls: "-1".into(),
                ..Default::default()
            },
        }
    }

    #[test]
    fn first_snapshot_wins_and_forget_undoes() {
        let now = timefmt::now();
        let mut s = DnsState::default();
        let r = s.remember("a", &mock(None), now);
        assert_eq!(
            r,
            DnsRemembered {
                created: true,
                interface: true,
                nm_profile: None
            }
        );
        let again = s.remember("a", &mock(Some([9, 9, 9, 9])), now);
        assert_eq!(again, DnsRemembered::default());
        assert_eq!(s.original("a").expect("record").interface, Some(mock(None)));
        assert_eq!(s.schema_version, DNS_STATE_SCHEMA_VERSION);

        s.forget("a", &again);
        assert!(s.original("a").is_some());
        s.forget("a", &r);
        assert!(s.original("a").is_none());
    }

    #[test]
    fn networkmanager_profiles_are_kept_per_uuid() {
        let now = timefmt::now();
        let mut s = DnsState::default();
        let first = s.remember("eth0", &nm("u1", ""), now);
        assert!(first.created);
        assert_eq!(first.nm_profile.as_deref(), Some("u1"));
        let second = s.remember("eth0", &nm("u2", "192.0.2.1"), now);
        assert_eq!(
            second,
            DnsRemembered {
                created: false,
                interface: false,
                nm_profile: Some("u2".into())
            }
        );
        assert_eq!(
            s.remember("eth0", &nm("u1", "9.9.9.9"), now),
            DnsRemembered::default()
        );
        let record = s.original("eth0").expect("record");
        assert_eq!(record.nm_profiles["u1"].ipv4_dns, "");
        assert_eq!(record.nm_profiles["u2"].ipv4_dns, "192.0.2.1");

        s.forget("eth0", &second);
        assert!(
            !s.original("eth0")
                .expect("record")
                .nm_profiles
                .contains_key("u2")
        );
        assert!(s.clear("eth0").is_some());
        assert!(s.clear("eth0").is_none());
    }

    #[test]
    fn back_to_the_original_clears_the_record() {
        let now = timefmt::now();
        let mut s = DnsState::default();
        let _ = s.remember("a", &mock(None), now);
        assert!(!s.clear_if_original("a", &mock(Some([9, 9, 9, 9]))));
        assert!(s.original("a").is_some());
        assert!(s.clear_if_original("a", &mock(None)));
        assert!(s.original("a").is_none());
        assert!(!s.clear_if_original("missing", &mock(None)));

        let _ = s.remember("eth0", &nm("u1", ""), now);
        let _ = s.remember("eth0", &nm("u2", ""), now);
        assert!(!s.clear_if_original("eth0", &nm("u1", "")));
        assert!(s.clear_if_original("eth0", &nm("u2", "")));
    }

    #[test]
    fn serde_round_trip_and_lenient_loading() {
        let now = timefmt::now();
        let mut s = DnsState::default();
        let _ = s.remember(
            "win",
            &DnsSnapshot::Windows {
                ipv4: WinFamilySnapshot {
                    name_server: "9.9.9.9".into(),
                    doh: vec![DohServerSnapshot {
                        server_index: 0,
                        template: Some("https://dns.quad9.net/dns-query".into()),
                        flags: 2,
                    }],
                    ..Default::default()
                },
                ipv6: WinFamilySnapshot::default(),
            },
            now,
        );
        let _ = s.remember("eth0", &nm("u1", ""), now);
        let json = serde_json::to_string(&s).expect("ser");
        assert!(json.contains(r#""kind":"windows""#));
        assert!(json.contains(r#""nmProfiles""#));
        let back: DnsState = serde_json::from_str(&json).expect("de");
        assert_eq!(back, s);

        let damaged = r#"{"schemaVersion":1,"originals":{
            "good":{"interface":{"kind":"mock","config":{"ipv4":{"mode":"automatic"},"ipv6":{"mode":"automatic"},"encryption":{"mode":"none"}}},"recordedAt":"2026-01-01T00:00:00Z"},
            "bad":{"recordedAt":"never"}
        }}"#;
        let loaded: DnsState = serde_json::from_str(damaged).expect("lenient");
        assert!(loaded.original("good").is_some());
        assert!(loaded.original("bad").is_none());
        let not_a_map: DnsState = serde_json::from_str(r#"{"originals":[1,2]}"#).expect("lenient");
        assert!(not_a_map.originals.is_empty());
    }
}
