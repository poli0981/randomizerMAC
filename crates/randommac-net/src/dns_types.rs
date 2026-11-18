//! What the backends report about DNS: what an adapter supports, how it is configured, and how a
//! change went.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::time::Duration;

use randommac_core::ChangeStatus;
use randommac_core::dns::{DnsSummary, EncryptionSupport};
use serde::Serialize;

/// How a DNS change is made.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DnsMethod {
    /// `SetInterfaceDnsSettings` (Windows).
    WindowsIpHelper,
    /// The active NetworkManager profile (Linux).
    NetworkManager,
    Mock,
}

impl DnsMethod {
    /// The value stored in history entries.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::WindowsIpHelper => "windowsIpHelper",
            Self::NetworkManager => "networkManager",
            Self::Mock => "mock",
        }
    }
}

/// Why DNS (or one feature of it) is not available for an adapter. Front ends translate the code
/// (`dns.reason.<CODE>`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DnsReason {
    /// This platform has no DNS support in RandomMac.
    UnsupportedPlatform,
    /// Windows older than version 2004 has no API for per-adapter DNS settings.
    RequiresWindows2004,
    /// DNS over HTTPS needs Windows 11 (or Windows Server 2022).
    DohRequiresNewerWindows,
    /// Windows has no DNS-over-TLS client.
    DotUnsupportedWindows,
    /// NetworkManager has no DNS-over-HTTPS client.
    DohUnsupportedLinux,
    /// NetworkManager does not manage this device.
    NotNmManaged,
    NmUnavailable,
    /// The device has no active NetworkManager profile to hold the settings.
    NoActiveConnection,
    /// DNS over TLS works only when NetworkManager hands DNS to systemd-resolved.
    DotNeedsResolved,
    /// NetworkManager's DNS mode could not be read.
    BackendUnknown,
    AdapterDisabled,
    NotElevated,
    /// The connection profile turns this address family off, so it takes no DNS servers.
    FamilyDisabled,
}

impl DnsReason {
    /// The serialized code (`dns.reason.<CODE>` in the UI).
    pub fn code(self) -> &'static str {
        match self {
            Self::UnsupportedPlatform => "UNSUPPORTED_PLATFORM",
            Self::RequiresWindows2004 => "REQUIRES_WINDOWS2004",
            Self::DohRequiresNewerWindows => "DOH_REQUIRES_NEWER_WINDOWS",
            Self::DotUnsupportedWindows => "DOT_UNSUPPORTED_WINDOWS",
            Self::DohUnsupportedLinux => "DOH_UNSUPPORTED_LINUX",
            Self::NotNmManaged => "NOT_NM_MANAGED",
            Self::NmUnavailable => "NM_UNAVAILABLE",
            Self::NoActiveConnection => "NO_ACTIVE_CONNECTION",
            Self::DotNeedsResolved => "DOT_NEEDS_RESOLVED",
            Self::BackendUnknown => "BACKEND_UNKNOWN",
            Self::AdapterDisabled => "ADAPTER_DISABLED",
            Self::NotElevated => "NOT_ELEVATED",
            Self::FamilyDisabled => "FAMILY_DISABLED",
        }
    }
}

/// Whether a feature can be used, and why not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FeatureSupport {
    pub available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<DnsReason>,
}

impl FeatureSupport {
    pub const YES: Self = Self {
        available: true,
        reason: None,
    };

    pub fn no(reason: DnsReason) -> Self {
        Self {
            available: false,
            reason: Some(reason),
        }
    }
}

/// What can be done with an adapter's DNS settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsCapabilities {
    /// The settings can be read (and shown).
    pub supported: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<DnsReason>,
    /// They can also be changed now (rights, adapter enabled, …).
    pub can_change: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub change_reason: Option<DnsReason>,
    pub ipv4: FeatureSupport,
    pub ipv6: FeatureSupport,
    pub doh: FeatureSupport,
    pub dot: FeatureSupport,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<DnsMethod>,
}

impl DnsCapabilities {
    pub fn unsupported(reason: DnsReason) -> Self {
        Self {
            supported: false,
            reason: Some(reason),
            can_change: false,
            change_reason: Some(reason),
            ipv4: FeatureSupport::no(reason),
            ipv6: FeatureSupport::no(reason),
            doh: FeatureSupport::no(reason),
            dot: FeatureSupport::no(reason),
            method: None,
        }
    }

    /// The encryption a request may use.
    pub fn encryption(&self) -> EncryptionSupport {
        EncryptionSupport {
            doh: self.doh.available,
            dot: self.dot.available,
        }
    }
}

/// An adapter's current DNS settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsStatus {
    /// What is configured on the adapter (Automatic = from the network).
    pub configured: DnsSummary,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doh_template: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dot_server_name: Option<String>,
    /// Encrypted DNS may fall back to unencrypted queries.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback: Option<bool>,
    /// The servers actually in use, configured or handed out by the network.
    pub effective_ipv4: Vec<Ipv4Addr>,
    pub effective_ipv6: Vec<Ipv6Addr>,
    /// Windows 11: servers set through the Settings app's own fields take precedence.
    pub profile_override: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nm_connection_uuid: Option<String>,
}

/// How a DNS change is carried out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DnsOptions {
    /// How long to wait for the new servers to be in use.
    pub verify_timeout: Duration,
    /// Flush the resolver cache afterwards (Windows).
    pub flush_cache: bool,
}

impl Default for DnsOptions {
    fn default() -> Self {
        Self {
            verify_timeout: Duration::from_secs(10),
            flush_cache: true,
        }
    }
}

/// The result of a DNS change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsReport {
    pub method: DnsMethod,
    pub status: ChangeStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous: Option<DnsSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requested: Option<DnsSummary>,
    /// What the system reports afterwards.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observed: Option<DnsSummary>,
    pub effective: Vec<IpAddr>,
    /// `DNS_NOT_APPLIED`, `DNS_NOT_ACTIVE`, `NOTHING_TO_RESTORE`, …
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail_code: Option<String>,
    /// `DNS_IPV6_SKIPPED`, `DNS_PROFILE_OVERRIDE`, `DNS_CACHE_NOT_FLUSHED`, …
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
    pub cache_flushed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nm_connection_uuid: Option<String>,
}

/// Detail and warning codes.
pub mod codes {
    /// The settings read back differ from the ones written; the previous ones were restored.
    pub const DNS_NOT_APPLIED: &str = "DNS_NOT_APPLIED";
    /// Written and read back, but the adapter does not use the new servers yet.
    pub const DNS_NOT_ACTIVE: &str = "DNS_NOT_ACTIVE";
    pub const NOTHING_TO_RESTORE: &str = "NOTHING_TO_RESTORE";
    pub const DNS_IPV4_SKIPPED: &str = "DNS_IPV4_SKIPPED";
    pub const DNS_IPV6_SKIPPED: &str = "DNS_IPV6_SKIPPED";
    pub const DNS_PROFILE_OVERRIDE: &str = "DNS_PROFILE_OVERRIDE";
    pub const DNS_CACHE_NOT_FLUSHED: &str = "DNS_CACHE_NOT_FLUSHED";
    pub const DNS_FALLBACK_ALLOWED: &str = "DNS_FALLBACK_ALLOWED";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_and_serialization() {
        assert_eq!(DnsMethod::WindowsIpHelper.as_str(), "windowsIpHelper");
        assert_eq!(DnsMethod::NetworkManager.as_str(), "networkManager");
        assert_eq!(DnsMethod::Mock.as_str(), "mock");
        let caps = DnsCapabilities::unsupported(DnsReason::RequiresWindows2004);
        let json = serde_json::to_value(&caps).expect("ser");
        assert_eq!(json["reason"], "REQUIRES_WINDOWS2004");
        assert_eq!(json["doh"]["available"], false);
        assert_eq!(
            caps.encryption(),
            EncryptionSupport {
                doh: false,
                dot: false
            }
        );
        assert_eq!(
            serde_json::to_value(FeatureSupport::YES).expect("ser"),
            serde_json::json!({"available": true})
        );
        assert_eq!(
            DnsOptions::default().verify_timeout,
            Duration::from_secs(10)
        );
        for reason in [
            DnsReason::UnsupportedPlatform,
            DnsReason::RequiresWindows2004,
            DnsReason::DohRequiresNewerWindows,
            DnsReason::DotUnsupportedWindows,
            DnsReason::DohUnsupportedLinux,
            DnsReason::NotNmManaged,
            DnsReason::NmUnavailable,
            DnsReason::NoActiveConnection,
            DnsReason::DotNeedsResolved,
            DnsReason::BackendUnknown,
            DnsReason::AdapterDisabled,
            DnsReason::NotElevated,
            DnsReason::FamilyDisabled,
        ] {
            assert_eq!(
                serde_json::to_value(reason).expect("ser"),
                reason.code(),
                "{reason:?}"
            );
        }
    }
}
