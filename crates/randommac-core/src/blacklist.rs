//! User-managed list of MAC addresses / prefixes that must never be generated or applied.

use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::mac::MacAddress;
use crate::random::new_id;

/// Addresses that are always refused, regardless of the user's list (kept from v1).
pub const RESERVED: [MacAddress; 3] = [
    MacAddress::ZERO,
    MacAddress::BROADCAST,
    MacAddress::new([0xaa; 6]),
];

/// Upper bound on stored entries; larger imports are truncated.
pub const MAX_ENTRIES: usize = 10_000;

/// A full MAC address or a 1–5 byte prefix (an OUI is a 3-byte prefix).
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct MacPattern {
    bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PatternError {
    #[error("enter a MAC address or a prefix such as AA:BB:CC")]
    Empty,
    #[error("invalid MAC address or prefix: {0}")]
    Invalid(String),
}

impl MacPattern {
    pub fn exact(mac: MacAddress) -> Self {
        Self {
            bytes: mac.octets().to_vec(),
        }
    }

    pub fn is_exact(&self) -> bool {
        self.bytes.len() == 6
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn matches(&self, mac: MacAddress) -> bool {
        mac.octets().starts_with(&self.bytes)
    }
}

impl FromStr for MacPattern {
    type Err = PatternError;

    /// Accepts a full MAC in any format `MacAddress` accepts, or a prefix such as
    /// `AA:BB:CC`, `AA-BB-CC`, `AA:BB:CC:*` or `AABBCC`.
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let s = input.trim();
        if s.is_empty() {
            return Err(PatternError::Empty);
        }
        if let Ok(mac) = s.parse::<MacAddress>() {
            return Ok(Self::exact(mac));
        }
        let invalid = || PatternError::Invalid(s.to_string());
        let body = s
            .strip_suffix(":*")
            .or_else(|| s.strip_suffix("-*"))
            .or_else(|| s.strip_suffix('*'))
            .unwrap_or(s);
        let groups: Vec<&str> = if body.contains(':') {
            body.split(':').collect()
        } else if body.contains('-') {
            body.split('-').collect()
        } else {
            if !body.len().is_multiple_of(2) || !body.is_ascii() {
                return Err(invalid());
            }
            (0..body.len())
                .step_by(2)
                .map(|i| &body[i..i + 2])
                .collect()
        };
        if groups.is_empty() || groups.len() > 5 {
            return Err(invalid());
        }
        let bytes = groups
            .iter()
            .map(|g| {
                if g.len() == 2 && g.chars().all(|c| c.is_ascii_hexdigit()) {
                    u8::from_str_radix(g, 16).map_err(|_| invalid())
                } else {
                    Err(invalid())
                }
            })
            .collect::<Result<Vec<u8>, _>>()?;
        Ok(Self { bytes })
    }
}

impl fmt::Display for MacPattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let hex: Vec<String> = self.bytes.iter().map(|b| format!("{b:02X}")).collect();
        if self.is_exact() {
            f.write_str(&hex.join(":"))
        } else {
            write!(f, "{}:*", hex.join(":"))
        }
    }
}

impl fmt::Debug for MacPattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MacPattern({self})")
    }
}

impl Serialize for MacPattern {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for MacPattern {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

/// One blacklist rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlacklistEntry {
    pub id: String,
    pub pattern: MacPattern,
    /// `None` = applies to every adapter.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adapter_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Why an address is blocked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockReason {
    Reserved,
    Entry { id: String, pattern: String },
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BlacklistError {
    #[error(transparent)]
    Pattern(#[from] PatternError),
    #[error("this address is always blocked")]
    Reserved,
    #[error("an identical rule already exists")]
    Duplicate,
    #[error("the blacklist is full ({MAX_ENTRIES} entries)")]
    Full,
}

/// The user's blacklist.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Blacklist {
    pub schema_version: u32,
    pub entries: Vec<BlacklistEntry>,
}

pub const BLACKLIST_SCHEMA_VERSION: u32 = 2;

impl Blacklist {
    /// Returns why `mac` is blocked for `adapter_id`, if it is.
    pub fn check(&self, mac: MacAddress, adapter_id: Option<&str>) -> Option<BlockReason> {
        if RESERVED.contains(&mac) {
            return Some(BlockReason::Reserved);
        }
        self.entries
            .iter()
            .find(|e| {
                let scoped = match (&e.adapter_id, adapter_id) {
                    (None, _) => true,
                    (Some(scope), Some(id)) => scope == id,
                    (Some(_), None) => false,
                };
                scoped && e.pattern.matches(mac)
            })
            .map(|e| BlockReason::Entry {
                id: e.id.clone(),
                pattern: e.pattern.to_string(),
            })
    }

    pub fn is_blocked(&self, mac: MacAddress, adapter_id: Option<&str>) -> bool {
        self.check(mac, adapter_id).is_some()
    }

    /// Adds a rule; rejects reserved addresses and exact duplicates.
    pub fn add(
        &mut self,
        pattern: MacPattern,
        adapter_id: Option<String>,
        note: Option<String>,
        now: DateTime<Utc>,
    ) -> Result<BlacklistEntry, BlacklistError> {
        if pattern.is_exact()
            && let Ok(mac) = MacAddress::try_from(pattern.bytes())
            && RESERVED.contains(&mac)
        {
            return Err(BlacklistError::Reserved);
        }
        let adapter_id = adapter_id.filter(|s| !s.trim().is_empty());
        if self
            .entries
            .iter()
            .any(|e| e.pattern == pattern && e.adapter_id == adapter_id)
        {
            return Err(BlacklistError::Duplicate);
        }
        if self.entries.len() >= MAX_ENTRIES {
            return Err(BlacklistError::Full);
        }
        let entry = BlacklistEntry {
            id: new_id(),
            pattern,
            adapter_id,
            note: note
                .map(|n| n.trim().chars().take(200).collect::<String>())
                .filter(|n| !n.is_empty()),
            created_at: now,
        };
        self.entries.push(entry.clone());
        Ok(entry)
    }

    pub fn remove(&mut self, id: &str) -> bool {
        let before = self.entries.len();
        self.entries.retain(|e| e.id != id);
        self.entries.len() != before
    }

    /// Drops duplicates, reserved exact entries and excess entries; refreshes the schema version.
    pub fn normalize(&mut self) {
        let mut seen = std::collections::HashSet::new();
        self.entries.retain(|e| {
            let reserved = e.pattern.is_exact()
                && MacAddress::try_from(e.pattern.bytes()).is_ok_and(|m| RESERVED.contains(&m));
            !reserved && !e.id.is_empty() && seen.insert((e.pattern.clone(), e.adapter_id.clone()))
        });
        self.entries.truncate(MAX_ENTRIES);
        self.schema_version = BLACKLIST_SCHEMA_VERSION;
    }

    /// Merges entries from another list (used by import); returns how many were added.
    pub fn merge(&mut self, other: Blacklist, now: DateTime<Utc>) -> usize {
        let mut added = 0;
        for e in other.entries {
            if self.add(e.pattern, e.adapter_id, e.note, now).is_ok() {
                added += 1;
            }
        }
        added
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mac(s: &str) -> MacAddress {
        s.parse().expect("valid mac")
    }

    #[test]
    fn pattern_parsing() {
        let p: MacPattern = "aa:bb:cc".parse().expect("oui prefix");
        assert_eq!(p.to_string(), "AA:BB:CC:*");
        assert_eq!("AA-BB-CC-*".parse::<MacPattern>(), Ok(p.clone()));
        assert_eq!("aabbcc".parse::<MacPattern>(), Ok(p.clone()));
        assert_eq!("AA:BB:CC:*".parse::<MacPattern>(), Ok(p.clone()));
        let exact: MacPattern = "02-11-22-33-44-55".parse().expect("exact");
        assert!(exact.is_exact());
        assert_eq!(exact.to_string(), "02:11:22:33:44:55");
        for bad in [
            "",
            "zz",
            "AA:BB:C",
            "AA:BB:CC:DD:EE:FF:00",
            "AAB",
            "AA:BB-CC",
            "ÀÀ",
        ] {
            assert!(bad.parse::<MacPattern>().is_err(), "{bad:?}");
        }
    }

    #[test]
    fn matching_and_scope() {
        let now = Utc::now();
        let mut bl = Blacklist::default();
        let oui = bl
            .add(
                "02:11:22".parse().expect("p"),
                None,
                Some(" lab ".into()),
                now,
            )
            .expect("add");
        assert_eq!(oui.note.as_deref(), Some("lab"));
        bl.add(
            "06:00:00:00:00:01".parse().expect("p"),
            Some("eth0".into()),
            None,
            now,
        )
        .expect("add scoped");

        assert!(bl.is_blocked(mac("02:11:22:99:88:77"), None));
        assert!(!bl.is_blocked(mac("02:11:23:99:88:77"), None));
        assert!(bl.is_blocked(mac("06:00:00:00:00:01"), Some("eth0")));
        assert!(!bl.is_blocked(mac("06:00:00:00:00:01"), Some("wlan0")));
        assert!(!bl.is_blocked(mac("06:00:00:00:00:01"), None));
        assert_eq!(
            bl.check(MacAddress::BROADCAST, None),
            Some(BlockReason::Reserved)
        );
        assert!(bl.is_blocked(mac("AA:AA:AA:AA:AA:AA"), Some("eth0")));
    }

    #[test]
    fn add_rejects_reserved_and_duplicates() {
        let now = Utc::now();
        let mut bl = Blacklist::default();
        assert_eq!(
            bl.add("00:00:00:00:00:00".parse().expect("p"), None, None, now),
            Err(BlacklistError::Reserved)
        );
        bl.add("02:00:00".parse().expect("p"), None, None, now)
            .expect("first");
        assert_eq!(
            bl.add("02-00-00".parse().expect("p"), None, None, now),
            Err(BlacklistError::Duplicate)
        );
        let id = bl.entries[0].id.clone();
        assert!(bl.remove(&id));
        assert!(!bl.remove(&id));
    }

    #[test]
    fn normalize_and_serde() {
        let now = Utc::now();
        let mut bl = Blacklist::default();
        bl.add("02:00:00".parse().expect("p"), None, None, now)
            .expect("add");
        let mut dup = bl.entries[0].clone();
        dup.id = "other".into();
        bl.entries.push(dup);
        bl.entries.push(BlacklistEntry {
            id: "r".into(),
            pattern: MacPattern::exact(MacAddress::BROADCAST),
            adapter_id: None,
            note: None,
            created_at: now,
        });
        bl.normalize();
        assert_eq!(bl.entries.len(), 1);
        assert_eq!(bl.schema_version, BLACKLIST_SCHEMA_VERSION);

        let json = serde_json::to_string(&bl).expect("ser");
        assert!(json.contains("\"02:00:00:*\""));
        let back: Blacklist = serde_json::from_str(&json).expect("de");
        assert_eq!(back, bl);
    }
}
