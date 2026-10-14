//! Bounded history of MAC changes.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::mac::MacAddress;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HistoryAction {
    Apply,
    Restore,
}

/// What triggered the change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum ChangeSource {
    #[default]
    Ui,
    Tray,
    Startup,
    Schedule,
    Exit,
    Cli,
    ImportV1,
}

/// Final state of an operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum ChangeStatus {
    /// The new address is active and was verified.
    #[default]
    Applied,
    /// The change was accepted but could not be verified (e.g. the adapter stayed down).
    Unverified,
    /// The driver ignored the address; the previous configuration was restored.
    RolledBack,
    /// Windows asked for a reboot before the change takes effect.
    RebootRequired,
    Failed,
}

impl ChangeStatus {
    pub fn is_success(self) -> bool {
        matches!(
            self,
            ChangeStatus::Applied | ChangeStatus::Unverified | ChangeStatus::RebootRequired
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub adapter_id: String,
    pub adapter_name: String,
    pub action: HistoryAction,
    #[serde(default)]
    pub source: ChangeSource,
    #[serde(default)]
    pub status: ChangeStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_mac: Option<MacAddress>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested_mac: Option<MacAddress>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub new_mac: Option<MacAddress>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl HistoryEntry {
    pub fn success(&self) -> bool {
        self.status.is_success()
    }
}

/// Retention rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HistoryPolicy {
    pub max_entries: usize,
    /// 0 keeps entries forever.
    pub retention_days: u32,
}

impl Default for HistoryPolicy {
    fn default() -> Self {
        Self {
            max_entries: 500,
            retention_days: 90,
        }
    }
}

pub const HISTORY_SCHEMA_VERSION: u32 = 2;

/// History, newest entry first.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct History {
    pub schema_version: u32,
    pub entries: Vec<HistoryEntry>,
}

impl History {
    pub fn push(&mut self, entry: HistoryEntry, policy: HistoryPolicy, now: DateTime<Utc>) {
        self.entries.insert(0, entry);
        self.prune(policy, now);
    }

    /// Sorts newest-first and applies the retention policy.
    pub fn prune(&mut self, policy: HistoryPolicy, now: DateTime<Utc>) {
        self.entries.sort_by_key(|e| std::cmp::Reverse(e.timestamp));
        if policy.retention_days > 0 {
            let cutoff = now - Duration::days(i64::from(policy.retention_days));
            self.entries.retain(|e| e.timestamp >= cutoff);
        }
        self.entries.truncate(policy.max_entries.max(1));
        self.schema_version = HISTORY_SCHEMA_VERSION;
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Case-insensitive search over adapter name and addresses (any separator).
    pub fn search<'a>(
        &'a self,
        query: &'a str,
        adapter_id: Option<&'a str>,
    ) -> impl Iterator<Item = &'a HistoryEntry> + 'a {
        let needle = normalize_query(query);
        self.entries.iter().filter(move |e| {
            if adapter_id.is_some_and(|id| id != e.adapter_id) {
                return false;
            }
            if needle.is_empty() {
                return true;
            }
            let mut haystack = e.adapter_name.to_lowercase();
            for m in [e.previous_mac, e.requested_mac, e.new_mac]
                .into_iter()
                .flatten()
            {
                haystack.push(' ');
                haystack.push_str(&m.to_compact_upper().to_lowercase());
            }
            haystack.contains(&needle)
                || e.adapter_name
                    .to_lowercase()
                    .contains(&query.trim().to_lowercase())
        })
    }
}

fn normalize_query(q: &str) -> String {
    let q = q.trim().to_lowercase();
    // Treat "aa:bb", "aa-bb" and "aabb" the same when the query looks like (part of) a MAC.
    if !q.is_empty()
        && q.chars()
            .all(|c| c.is_ascii_hexdigit() || c == ':' || c == '-' || c == '.')
    {
        q.chars().filter(|c| c.is_ascii_hexdigit()).collect()
    } else {
        q
    }
}

fn csv_cell(value: &str) -> String {
    // Neutralize spreadsheet formula injection, then quote.
    let guarded = if value.starts_with(['=', '+', '-', '@', '\t', '\r']) {
        format!("'{value}")
    } else {
        value.to_string()
    };
    format!("\"{}\"", guarded.replace('"', "\"\""))
}

/// Renders entries as RFC 4180 CSV (with a header row).
pub fn to_csv(entries: &[HistoryEntry]) -> String {
    let mut out = String::from(
        "timestamp,adapter,adapterId,action,source,status,previousMac,requestedMac,newMac,method,errorCode,error\r\n",
    );
    for e in entries {
        let mac = |m: Option<MacAddress>| m.map(|m| m.to_string()).unwrap_or_default();
        let enum_str = |v: serde_json::Value| v.as_str().unwrap_or_default().to_string();
        let row = [
            e.timestamp.to_rfc3339(),
            e.adapter_name.clone(),
            e.adapter_id.clone(),
            enum_str(serde_json::to_value(e.action).unwrap_or_default()),
            enum_str(serde_json::to_value(e.source).unwrap_or_default()),
            enum_str(serde_json::to_value(e.status).unwrap_or_default()),
            mac(e.previous_mac),
            mac(e.requested_mac),
            mac(e.new_mac),
            e.method.clone().unwrap_or_default(),
            e.error_code.clone().unwrap_or_default(),
            e.error.clone().unwrap_or_default(),
        ];
        out.push_str(
            &row.iter()
                .map(|c| csv_cell(c))
                .collect::<Vec<_>>()
                .join(","),
        );
        out.push_str("\r\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str, ts: DateTime<Utc>, name: &str) -> HistoryEntry {
        HistoryEntry {
            id: id.into(),
            timestamp: ts,
            adapter_id: format!("id-{name}"),
            adapter_name: name.into(),
            action: HistoryAction::Apply,
            source: ChangeSource::Ui,
            status: ChangeStatus::Applied,
            previous_mac: Some("30:56:0F:09:8F:E4".parse().expect("mac")),
            requested_mac: Some("02:AA:BB:CC:DD:EE".parse().expect("mac")),
            new_mac: Some("02:AA:BB:CC:DD:EE".parse().expect("mac")),
            method: Some("windowsRegistry".into()),
            error_code: None,
            error: None,
        }
    }

    #[test]
    fn push_orders_and_prunes() {
        let now = Utc::now();
        let mut h = History::default();
        let policy = HistoryPolicy {
            max_entries: 3,
            retention_days: 30,
        };
        h.push(entry("old", now - Duration::days(40), "Wi-Fi"), policy, now);
        assert!(
            h.entries.is_empty(),
            "entries older than retention are dropped"
        );
        for i in 0..5 {
            h.push(
                entry(
                    &format!("e{i}"),
                    now - Duration::minutes(10 - i),
                    "Ethernet",
                ),
                policy,
                now,
            );
        }
        assert_eq!(h.entries.len(), 3);
        assert_eq!(h.entries[0].id, "e4");
        assert!(
            h.entries
                .windows(2)
                .all(|w| w[0].timestamp >= w[1].timestamp)
        );

        let forever = HistoryPolicy {
            max_entries: 10,
            retention_days: 0,
        };
        h.push(
            entry("ancient", now - Duration::days(4000), "x"),
            forever,
            now,
        );
        assert!(h.entries.iter().any(|e| e.id == "ancient"));
    }

    #[test]
    fn search_matches_names_and_any_mac_format() {
        let now = Utc::now();
        let mut h = History::default();
        h.push(entry("a", now, "Wi-Fi"), HistoryPolicy::default(), now);
        h.push(entry("b", now, "Ethernet"), HistoryPolicy::default(), now);
        assert_eq!(h.search("wi-fi", None).count(), 1);
        assert_eq!(h.search("02-aa-bb", None).count(), 2);
        assert_eq!(h.search("02aabb", None).count(), 2);
        assert_eq!(h.search("30:56:0f", Some("id-Ethernet")).count(), 1);
        assert_eq!(h.search("", None).count(), 2);
        assert_eq!(h.search("zzz", None).count(), 0);
    }

    #[test]
    fn csv_is_escaped() {
        let now = Utc::now();
        let mut e = entry("a", now, "=HYPERLINK(\"x\")");
        e.error = Some("said \"no\"".into());
        let csv = to_csv(&[e]);
        let mut lines = csv.lines();
        assert!(
            lines
                .next()
                .expect("header")
                .starts_with("timestamp,adapter")
        );
        let row = lines.next().expect("row");
        assert!(row.contains("\"'=HYPERLINK(\"\"x\"\")\""));
        assert!(row.contains("\"said \"\"no\"\"\""));
        assert!(row.contains("\"apply\"") && row.contains("\"applied\""));
    }

    #[test]
    fn status_success() {
        assert!(ChangeStatus::Applied.is_success());
        assert!(ChangeStatus::RebootRequired.is_success());
        assert!(!ChangeStatus::RolledBack.is_success());
        assert!(!ChangeStatus::Failed.is_success());
    }
}
