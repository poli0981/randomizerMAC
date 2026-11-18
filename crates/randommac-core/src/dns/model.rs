//! DNS configurations, what a front end can ask for, and the validation every front end shares.

use std::fmt;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use serde::{Deserialize, Serialize};

use super::catalog::Catalog;
use super::presets::{self, CustomDnsPreset};

/// Servers per address family: a primary and a secondary, as Windows Settings and NetworkManager's
/// editors offer.
pub const MAX_SERVERS_PER_FAMILY: usize = 2;

/// The servers of the "Local resolver" target: a DNS client such as dnscrypt-proxy listening on
/// this computer, which does the encryption itself.
pub const LOCAL_RESOLVER_V4: Ipv4Addr = Ipv4Addr::LOCALHOST;
pub const LOCAL_RESOLVER_V6: Ipv6Addr = Ipv6Addr::LOCALHOST;

const MAX_TEMPLATE_LEN: usize = 512;
const MAX_HOST_LEN: usize = 253;

/// One address family's DNS servers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "camelCase")]
pub enum FamilyDns<A> {
    /// Servers handed out by the network (DHCP, router advertisements).
    Automatic,
    /// Fixed servers, in order of preference.
    Static { servers: Vec<A> },
}

impl<A: Copy> FamilyDns<A> {
    /// `Automatic` for an empty list, else these servers (validated by [`validate_config`]).
    pub fn from_servers(list: &[A]) -> Self {
        if list.is_empty() {
            Self::Automatic
        } else {
            Self::Static {
                servers: list.to_vec(),
            }
        }
    }

    pub fn servers(&self) -> &[A] {
        match self {
            Self::Automatic => &[],
            Self::Static { servers } => servers,
        }
    }

    pub fn is_automatic(&self) -> bool {
        matches!(self, Self::Automatic)
    }
}

/// Encryption of DNS traffic to the static servers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "camelCase")]
pub enum DnsEncryption {
    None,
    /// DNS over HTTPS (Windows 11): every server uses this URI template.
    #[serde(rename_all = "camelCase")]
    Doh {
        template: String,
        /// Fall back to unencrypted DNS when the encrypted query fails.
        allow_fallback: bool,
    },
    /// DNS over TLS (Linux with systemd-resolved): the name in the servers' certificates.
    #[serde(rename_all = "camelCase")]
    Dot {
        server_name: Option<String>,
        /// Opportunistic: fall back to unencrypted DNS when TLS fails.
        allow_fallback: bool,
    },
}

impl DnsEncryption {
    pub fn mode(&self) -> EncryptionMode {
        match self {
            Self::None => EncryptionMode::None,
            Self::Doh { .. } => EncryptionMode::Doh,
            Self::Dot { .. } => EncryptionMode::Dot,
        }
    }
}

/// The DNS configuration of one adapter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsConfig {
    pub ipv4: FamilyDns<Ipv4Addr>,
    pub ipv6: FamilyDns<Ipv6Addr>,
    pub encryption: DnsEncryption,
}

impl DnsConfig {
    /// Everything from the network, nothing encrypted by RandomMac.
    pub fn automatic() -> Self {
        Self {
            ipv4: FamilyDns::Automatic,
            ipv6: FamilyDns::Automatic,
            encryption: DnsEncryption::None,
        }
    }

    pub fn is_automatic(&self) -> bool {
        self.ipv4.is_automatic() && self.ipv6.is_automatic()
    }

    /// The static servers, IPv4 first.
    pub fn servers(&self) -> Vec<IpAddr> {
        self.ipv4
            .servers()
            .iter()
            .map(|a| IpAddr::V4(*a))
            .chain(self.ipv6.servers().iter().map(|a| IpAddr::V6(*a)))
            .collect()
    }

    pub fn summary(&self) -> DnsSummary {
        DnsSummary {
            ipv4: self.ipv4.clone(),
            ipv6: self.ipv6.clone(),
            encryption: self.encryption.mode(),
        }
    }
}

/// How DNS traffic is protected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EncryptionMode {
    #[default]
    None,
    Doh,
    Dot,
}

/// A configuration without the template or server name, as kept in the history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsSummary {
    pub ipv4: FamilyDns<Ipv4Addr>,
    pub ipv6: FamilyDns<Ipv6Addr>,
    #[serde(default)]
    pub encryption: EncryptionMode,
}

impl fmt::Display for DnsSummary {
    /// `IPv4 9.9.9.9, 149.112.112.112; IPv6 automatic; DoH` — for the CLI and CSV exports.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn family<A: fmt::Display>(
            f: &mut fmt::Formatter<'_>,
            name: &str,
            dns: &FamilyDns<A>,
        ) -> fmt::Result {
            match dns {
                FamilyDns::Automatic => write!(f, "{name} automatic"),
                FamilyDns::Static { servers } => {
                    let list: Vec<String> = servers.iter().map(ToString::to_string).collect();
                    write!(f, "{name} {}", list.join(", "))
                }
            }
        }
        family(f, "IPv4", &self.ipv4)?;
        f.write_str("; ")?;
        family(f, "IPv6", &self.ipv6)?;
        match self.encryption {
            EncryptionMode::None => Ok(()),
            EncryptionMode::Doh => f.write_str("; DoH"),
            EncryptionMode::Dot => f.write_str("; DoT"),
        }
    }
}

/// The endpoints of a provider variant, a preset or the custom form.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DnsEndpoints {
    pub ipv4: Vec<Ipv4Addr>,
    pub ipv6: Vec<Ipv6Addr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doh: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dot: Option<String>,
}

/// What the user picked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum DnsTarget {
    /// Servers from the network.
    Automatic,
    /// 127.0.0.1 and ::1.
    LocalResolver,
    /// A provider from the catalog (`variant` = its default when absent).
    #[serde(rename_all = "camelCase")]
    Provider {
        provider: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        variant: Option<String>,
    },
    /// A custom preset from the settings, by id (the CLI also accepts its name).
    Preset { id: String },
    /// Servers typed in by the user.
    Custom(DnsEndpoints),
}

/// Which encryption to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EncryptionChoice {
    None,
    Doh,
    Dot,
    /// DoH where the system supports it, otherwise DoT, otherwise none.
    #[default]
    Preferred,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsRequest {
    pub target: DnsTarget,
    #[serde(default)]
    pub encryption: EncryptionChoice,
    #[serde(default)]
    pub allow_fallback: bool,
}

/// The encryption the system can apply to an adapter.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EncryptionSupport {
    pub doh: bool,
    pub dot: bool,
}

/// What a configuration is called, for the history and the UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum DnsLabel {
    Automatic,
    LocalResolver,
    /// Catalog ids; front ends show the provider's name.
    Provider {
        provider: String,
        variant: String,
    },
    Preset {
        name: String,
    },
    Custom,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedDns {
    pub config: DnsConfig,
    pub label: DnsLabel,
}

/// Why a DNS request or configuration is not usable.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DnsInputError {
    #[error("no DNS server was given")]
    NoServers,
    #[error("at most {MAX_SERVERS_PER_FAMILY} servers per address family")]
    TooManyServers,
    #[error("{0} is listed twice")]
    DuplicateServer(IpAddr),
    #[error("{0} cannot be a DNS server")]
    UnusableServer(IpAddr),
    #[error("the DNS-over-HTTPS template must be an https:// URL with a host name")]
    BadDohTemplate,
    #[error("the DNS-over-HTTPS template names an IP address that is not one of the servers")]
    DohTemplateIpMismatch,
    #[error("the DNS-over-TLS server name is not a valid host name")]
    BadDotHost,
    #[error("encryption needs at least one fixed DNS server")]
    EncryptionNeedsStaticServers,
    #[error("this adapter does not support {0}")]
    EncryptionUnavailable(&'static str),
    #[error("these servers publish no {0} endpoint")]
    NoEndpointForEncryption(&'static str),
    #[error("unknown DNS provider '{0}'")]
    UnknownProvider(String),
    #[error("unknown variant '{1}' of DNS provider '{0}'")]
    UnknownVariant(String, String),
    #[error("unknown DNS preset '{0}'")]
    UnknownPreset(String),
}

impl DnsInputError {
    /// Stable code for the UI (`error.<CODE>`).
    pub fn code(&self) -> &'static str {
        match self {
            Self::UnknownPreset(_) => "UNKNOWN_DNS_PRESET",
            _ => "INVALID_DNS",
        }
    }
}

fn check_v4(ip: Ipv4Addr) -> Result<(), DnsInputError> {
    let bad = ip.is_unspecified() || ip.is_broadcast() || ip.is_multicast() || ip.octets()[0] == 0;
    if bad {
        Err(DnsInputError::UnusableServer(IpAddr::V4(ip)))
    } else {
        Ok(())
    }
}

fn check_v6(ip: Ipv6Addr) -> Result<(), DnsInputError> {
    // Link-local servers need a zone (interface) id, which the system settings cannot hold;
    // IPv4-mapped addresses belong in the IPv4 list.
    let link_local = (ip.segments()[0] & 0xffc0) == 0xfe80;
    let bad =
        ip.is_unspecified() || ip.is_multicast() || link_local || ip.to_ipv4_mapped().is_some();
    if bad {
        Err(DnsInputError::UnusableServer(IpAddr::V6(ip)))
    } else {
        Ok(())
    }
}

fn check_list<A: Copy + PartialEq + Into<IpAddr>>(
    list: &[A],
    check: fn(A) -> Result<(), DnsInputError>,
) -> Result<(), DnsInputError> {
    if list.len() > MAX_SERVERS_PER_FAMILY {
        return Err(DnsInputError::TooManyServers);
    }
    for (i, ip) in list.iter().enumerate() {
        check(*ip)?;
        if list[..i].contains(ip) {
            return Err(DnsInputError::DuplicateServer((*ip).into()));
        }
    }
    Ok(())
}

/// True for a DNS host name: 1–253 characters, labels of 1–63 letters, digits and hyphens that do
/// not start or end with a hyphen, no trailing dot, and not an IP address.
pub fn is_host_name(host: &str) -> bool {
    !host.is_empty()
        && host.len() <= MAX_HOST_LEN
        && host.parse::<IpAddr>().is_err()
        && host.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        })
}

pub fn validate_dot_host(host: &str) -> Result<(), DnsInputError> {
    if is_host_name(host) {
        Ok(())
    } else {
        Err(DnsInputError::BadDotHost)
    }
}

/// Checks a DoH URI template: `https://host[:port]/path`, where `host` is a host name or an IP
/// address that every server shares (Windows refuses a template whose IP differs from the server).
pub fn validate_doh_template(template: &str, servers: &[IpAddr]) -> Result<(), DnsInputError> {
    let bad = || DnsInputError::BadDohTemplate;
    if template.len() > MAX_TEMPLATE_LEN
        || !template.is_ascii()
        || template
            .bytes()
            .any(|b| b.is_ascii_control() || b == b' ' || b"\"<>\\^`|".contains(&b))
    {
        return Err(bad());
    }
    let rest = template.strip_prefix("https://").ok_or_else(bad)?;
    let (authority, path) = rest.split_at(rest.find('/').ok_or_else(bad)?);
    if path.len() < 2 || authority.contains('@') {
        return Err(bad());
    }
    let (host, port) = if let Some(v6) = authority.strip_prefix('[') {
        let (host, after) = v6.split_once(']').ok_or_else(bad)?;
        (host, after.strip_prefix(':'))
    } else {
        match authority.split_once(':') {
            Some((host, port)) => (host, Some(port)),
            None => (authority, None),
        }
    };
    if let Some(port) = port
        && !port.parse::<u16>().is_ok_and(|p| p > 0)
    {
        return Err(bad());
    }
    if let Ok(ip) = host.parse::<IpAddr>() {
        if servers.is_empty() || servers.iter().any(|s| *s != ip) {
            return Err(DnsInputError::DohTemplateIpMismatch);
        }
        return Ok(());
    }
    if is_host_name(host) {
        Ok(())
    } else {
        Err(bad())
    }
}

/// Validates a complete configuration (the last check before it is applied).
pub fn validate_config(config: &DnsConfig) -> Result<(), DnsInputError> {
    check_list(config.ipv4.servers(), check_v4)?;
    check_list(config.ipv6.servers(), check_v6)?;
    let servers = config.servers();
    match &config.encryption {
        DnsEncryption::None => Ok(()),
        _ if servers.is_empty() => Err(DnsInputError::EncryptionNeedsStaticServers),
        DnsEncryption::Doh { template, .. } => validate_doh_template(template, &servers),
        DnsEncryption::Dot { server_name, .. } => match server_name {
            Some(name) => validate_dot_host(name),
            None => Ok(()),
        },
    }
}

/// Turns a request into the configuration to apply: looks up the provider or preset, keeps two
/// servers per family, and chooses the encryption the adapter supports.
pub fn resolve_request(
    request: &DnsRequest,
    catalog: &Catalog,
    custom: &[CustomDnsPreset],
    support: EncryptionSupport,
) -> Result<ResolvedDns, DnsInputError> {
    let (endpoints, label) = match &request.target {
        DnsTarget::Automatic => {
            return Ok(ResolvedDns {
                config: DnsConfig::automatic(),
                label: DnsLabel::Automatic,
            });
        }
        DnsTarget::LocalResolver => {
            let config = DnsConfig {
                ipv4: FamilyDns::Static {
                    servers: vec![LOCAL_RESOLVER_V4],
                },
                ipv6: FamilyDns::Static {
                    servers: vec![LOCAL_RESOLVER_V6],
                },
                encryption: DnsEncryption::None,
            };
            return Ok(ResolvedDns {
                config,
                label: DnsLabel::LocalResolver,
            });
        }
        DnsTarget::Provider { provider, variant } => {
            let p = catalog
                .provider(provider)
                .ok_or_else(|| DnsInputError::UnknownProvider(provider.clone()))?;
            let v = p.variant(variant.as_deref()).ok_or_else(|| {
                DnsInputError::UnknownVariant(provider.clone(), variant.clone().unwrap_or_default())
            })?;
            let label = DnsLabel::Provider {
                provider: p.id.clone(),
                variant: v.id.clone(),
            };
            (v.endpoints(), label)
        }
        DnsTarget::Preset { id } => {
            let preset = presets::find(custom, id)
                .ok_or_else(|| DnsInputError::UnknownPreset(id.clone()))?;
            (
                preset.endpoints(),
                DnsLabel::Preset {
                    name: preset.name.clone(),
                },
            )
        }
        DnsTarget::Custom(endpoints) => (endpoints.clone(), DnsLabel::Custom),
    };

    if endpoints.ipv4.len() > MAX_SERVERS_PER_FAMILY
        || endpoints.ipv6.len() > MAX_SERVERS_PER_FAMILY
    {
        return Err(DnsInputError::TooManyServers);
    }
    let ipv4 = FamilyDns::from_servers(&endpoints.ipv4);
    let ipv6 = FamilyDns::from_servers(&endpoints.ipv6);
    if ipv4.is_automatic() && ipv6.is_automatic() {
        return Err(DnsInputError::NoServers);
    }
    let fallback = request.allow_fallback;
    let doh = |template: &String| DnsEncryption::Doh {
        template: template.clone(),
        allow_fallback: fallback,
    };
    let dot = |name: &String| DnsEncryption::Dot {
        server_name: Some(name.clone()),
        allow_fallback: fallback,
    };
    let encryption = match request.encryption {
        EncryptionChoice::None => DnsEncryption::None,
        EncryptionChoice::Doh => {
            if !support.doh {
                return Err(DnsInputError::EncryptionUnavailable("DNS over HTTPS"));
            }
            doh(endpoints
                .doh
                .as_ref()
                .ok_or(DnsInputError::NoEndpointForEncryption("DNS-over-HTTPS"))?)
        }
        EncryptionChoice::Dot => {
            if !support.dot {
                return Err(DnsInputError::EncryptionUnavailable("DNS over TLS"));
            }
            dot(endpoints
                .dot
                .as_ref()
                .ok_or(DnsInputError::NoEndpointForEncryption("DNS-over-TLS"))?)
        }
        EncryptionChoice::Preferred => match (&endpoints.doh, &endpoints.dot) {
            (Some(t), _) if support.doh => doh(t),
            (_, Some(n)) if support.dot => dot(n),
            _ => DnsEncryption::None,
        },
    };
    let config = DnsConfig {
        ipv4,
        ipv6,
        encryption,
    };
    validate_config(&config)?;
    Ok(ResolvedDns { config, label })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dns::catalog;

    fn v4(s: &str) -> Ipv4Addr {
        s.parse().expect("v4")
    }

    fn v6(s: &str) -> Ipv6Addr {
        s.parse().expect("v6")
    }

    fn quad9() -> DnsEndpoints {
        DnsEndpoints {
            ipv4: vec![v4("9.9.9.9"), v4("149.112.112.112")],
            ipv6: vec![v6("2620:fe::fe"), v6("2620:fe::9")],
            doh: Some("https://dns.quad9.net/dns-query".into()),
            dot: Some("dns.quad9.net".into()),
        }
    }

    fn request(target: DnsTarget, encryption: EncryptionChoice) -> DnsRequest {
        DnsRequest {
            target,
            encryption,
            allow_fallback: false,
        }
    }

    const BOTH: EncryptionSupport = EncryptionSupport {
        doh: true,
        dot: true,
    };
    const NONE: EncryptionSupport = EncryptionSupport {
        doh: false,
        dot: false,
    };

    fn resolve(req: &DnsRequest, support: EncryptionSupport) -> Result<ResolvedDns, DnsInputError> {
        resolve_request(req, catalog::catalog(), &[], support)
    }

    #[test]
    fn automatic_and_local_resolver() {
        let r = resolve(&request(DnsTarget::Automatic, EncryptionChoice::Doh), NONE).expect("auto");
        assert!(r.config.is_automatic());
        assert_eq!(r.label, DnsLabel::Automatic);
        let r = resolve(
            &request(DnsTarget::LocalResolver, EncryptionChoice::Preferred),
            BOTH,
        )
        .expect("local");
        assert_eq!(
            r.config.servers(),
            vec![IpAddr::V4(LOCAL_RESOLVER_V4), IpAddr::V6(LOCAL_RESOLVER_V6)]
        );
        assert_eq!(r.config.encryption, DnsEncryption::None);
        assert_eq!(r.label, DnsLabel::LocalResolver);
    }

    #[test]
    fn custom_endpoints_and_encryption_choice() {
        let custom = DnsTarget::Custom(quad9());
        let plain = resolve(&request(custom.clone(), EncryptionChoice::None), BOTH).expect("plain");
        assert_eq!(plain.config.encryption, DnsEncryption::None);
        assert_eq!(plain.config.servers().len(), 4);
        assert_eq!(plain.label, DnsLabel::Custom);

        let preferred =
            resolve(&request(custom.clone(), EncryptionChoice::Preferred), BOTH).expect("pref");
        assert!(matches!(
            preferred.config.encryption,
            DnsEncryption::Doh { .. }
        ));
        let dot_only = EncryptionSupport {
            doh: false,
            dot: true,
        };
        let preferred = resolve(
            &request(custom.clone(), EncryptionChoice::Preferred),
            dot_only,
        )
        .expect("pref");
        assert_eq!(
            preferred.config.encryption,
            DnsEncryption::Dot {
                server_name: Some("dns.quad9.net".into()),
                allow_fallback: false
            }
        );
        let preferred =
            resolve(&request(custom.clone(), EncryptionChoice::Preferred), NONE).expect("pref");
        assert_eq!(preferred.config.encryption, DnsEncryption::None);

        let mut req = request(custom.clone(), EncryptionChoice::Doh);
        req.allow_fallback = true;
        let doh = resolve(&req, BOTH).expect("doh");
        assert_eq!(
            doh.config.encryption,
            DnsEncryption::Doh {
                template: "https://dns.quad9.net/dns-query".into(),
                allow_fallback: true
            }
        );
        assert_eq!(
            resolve(&request(custom.clone(), EncryptionChoice::Doh), NONE),
            Err(DnsInputError::EncryptionUnavailable("DNS over HTTPS"))
        );
        assert_eq!(
            resolve(&request(custom, EncryptionChoice::Dot), NONE),
            Err(DnsInputError::EncryptionUnavailable("DNS over TLS"))
        );
        let bare = DnsTarget::Custom(DnsEndpoints {
            ipv4: vec![v4("192.0.2.53")],
            ..Default::default()
        });
        assert_eq!(
            resolve(&request(bare.clone(), EncryptionChoice::Doh), BOTH),
            Err(DnsInputError::NoEndpointForEncryption("DNS-over-HTTPS"))
        );
        assert_eq!(
            resolve(&request(bare, EncryptionChoice::Dot), BOTH),
            Err(DnsInputError::NoEndpointForEncryption("DNS-over-TLS"))
        );
    }

    #[test]
    fn missing_families_are_automatic() {
        let only_v4 = DnsTarget::Custom(DnsEndpoints {
            ipv4: vec![v4("192.0.2.53")],
            ..Default::default()
        });
        let r = resolve(&request(only_v4, EncryptionChoice::None), NONE).expect("v4 only");
        assert!(r.config.ipv6.is_automatic());
        assert_eq!(r.config.ipv4.servers(), &[v4("192.0.2.53")]);
        assert_eq!(
            resolve(
                &request(
                    DnsTarget::Custom(DnsEndpoints::default()),
                    EncryptionChoice::None
                ),
                NONE
            ),
            Err(DnsInputError::NoServers)
        );
        let three = DnsTarget::Custom(DnsEndpoints {
            ipv4: vec![v4("192.0.2.1"), v4("192.0.2.2"), v4("192.0.2.3")],
            ..Default::default()
        });
        assert_eq!(
            resolve(&request(three, EncryptionChoice::None), NONE),
            Err(DnsInputError::TooManyServers)
        );
    }

    #[test]
    fn providers_and_presets() {
        let quad9 = DnsTarget::Provider {
            provider: "quad9".into(),
            variant: None,
        };
        let r = resolve(&request(quad9, EncryptionChoice::None), NONE).expect("quad9");
        assert_eq!(
            r.label,
            DnsLabel::Provider {
                provider: "quad9".into(),
                variant: "recommended".into()
            }
        );
        assert_eq!(r.config.ipv4.servers()[0], v4("9.9.9.9"));

        let unknown = DnsTarget::Provider {
            provider: "nope".into(),
            variant: None,
        };
        assert_eq!(
            resolve(&request(unknown, EncryptionChoice::None), NONE),
            Err(DnsInputError::UnknownProvider("nope".into()))
        );
        let bad_variant = DnsTarget::Provider {
            provider: "quad9".into(),
            variant: Some("x".into()),
        };
        assert_eq!(
            resolve(&request(bad_variant, EncryptionChoice::None), NONE),
            Err(DnsInputError::UnknownVariant("quad9".into(), "x".into()))
        );

        let preset = CustomDnsPreset {
            id: "0123456789abcdef".into(),
            name: "Home".into(),
            ipv4: vec![v4("192.0.2.53")],
            ipv6: vec![],
            doh_template: None,
            dot_host: None,
        };
        let by_name = DnsRequest {
            target: DnsTarget::Preset { id: "home".into() },
            encryption: EncryptionChoice::None,
            allow_fallback: false,
        };
        let r = resolve_request(
            &by_name,
            catalog::catalog(),
            std::slice::from_ref(&preset),
            NONE,
        )
        .expect("preset");
        assert_eq!(
            r.label,
            DnsLabel::Preset {
                name: "Home".into()
            }
        );
        let missing = DnsRequest {
            target: DnsTarget::Preset { id: "x".into() },
            ..by_name
        };
        let err =
            resolve_request(&missing, catalog::catalog(), &[preset], NONE).expect_err("missing");
        assert_eq!(err, DnsInputError::UnknownPreset("x".into()));
        assert_eq!(err.code(), "UNKNOWN_DNS_PRESET");
        assert_eq!(DnsInputError::NoServers.code(), "INVALID_DNS");
    }

    #[test]
    fn unusable_and_duplicate_servers() {
        for bad in ["0.0.0.0", "255.255.255.255", "224.0.0.1", "0.1.2.3"] {
            let t = DnsTarget::Custom(DnsEndpoints {
                ipv4: vec![v4(bad)],
                ..Default::default()
            });
            assert!(
                matches!(
                    resolve(&request(t, EncryptionChoice::None), NONE),
                    Err(DnsInputError::UnusableServer(_))
                ),
                "{bad}"
            );
        }
        for bad in ["::", "ff02::1", "fe80::1", "::ffff:192.0.2.1"] {
            let t = DnsTarget::Custom(DnsEndpoints {
                ipv6: vec![v6(bad)],
                ..Default::default()
            });
            assert!(
                matches!(
                    resolve(&request(t, EncryptionChoice::None), NONE),
                    Err(DnsInputError::UnusableServer(_))
                ),
                "{bad}"
            );
        }
        let dup = DnsTarget::Custom(DnsEndpoints {
            ipv4: vec![v4("192.0.2.1"), v4("192.0.2.1")],
            ..Default::default()
        });
        assert_eq!(
            resolve(&request(dup, EncryptionChoice::None), NONE),
            Err(DnsInputError::DuplicateServer(
                "192.0.2.1".parse().expect("ip")
            ))
        );
        // Loopback (a local resolver) and private addresses (a router) are fine.
        let ok = DnsTarget::Custom(DnsEndpoints {
            ipv4: vec![v4("127.0.0.1"), v4("192.168.1.1")],
            ipv6: vec![v6("::1"), v6("fd00::53")],
            ..Default::default()
        });
        assert!(resolve(&request(ok, EncryptionChoice::None), NONE).is_ok());
    }

    #[test]
    fn doh_templates() {
        let servers: Vec<IpAddr> = vec!["9.9.9.9".parse().expect("ip")];
        for ok in [
            "https://dns.quad9.net/dns-query",
            "https://dns.google/dns-query{?dns}",
            "https://doh.cleanbrowsing.org/doh/family-filter/",
            "https://example.com:8443/q",
            "https://9.9.9.9/dns-query",
        ] {
            assert_eq!(validate_doh_template(ok, &servers), Ok(()), "{ok}");
        }
        for bad in [
            "http://dns.quad9.net/dns-query",
            "https://dns.quad9.net",
            "https://dns.quad9.net/",
            "https://user@dns.quad9.net/q",
            "https://dns quad9.net/q",
            "https://-bad.net/q",
            "https://example.com:0/q",
            "https://example.com:99999/q",
            "https://[2620:fe::fe/q",
            "https://exämple.com/q",
            "",
        ] {
            assert_eq!(
                validate_doh_template(bad, &servers),
                Err(DnsInputError::BadDohTemplate),
                "{bad:?}"
            );
        }
        assert_eq!(
            validate_doh_template("https://1.1.1.1/dns-query", &servers),
            Err(DnsInputError::DohTemplateIpMismatch)
        );
        assert_eq!(
            validate_doh_template(
                "https://[2620:fe::fe]/dns-query",
                &["2620:fe::fe".parse().expect("ip")]
            ),
            Ok(())
        );
        assert_eq!(
            validate_doh_template(&format!("https://a.b/{}", "x".repeat(600)), &servers),
            Err(DnsInputError::BadDohTemplate)
        );
    }

    #[test]
    fn dot_host_names() {
        for ok in [
            "dns.quad9.net",
            "one.one.one.one",
            "dns.google",
            "localhost",
            "a-b.c1",
        ] {
            assert!(is_host_name(ok), "{ok}");
        }
        for bad in [
            "",
            "dns..google",
            "-x.net",
            "x-.net",
            "dns.google.",
            "9.9.9.9",
            "dns_google",
            &"a".repeat(64),
        ] {
            assert!(!is_host_name(bad), "{bad:?}");
        }
        assert_eq!(validate_dot_host("dns.quad9.net"), Ok(()));
        assert_eq!(validate_dot_host("9.9.9.9"), Err(DnsInputError::BadDotHost));
    }

    #[test]
    fn config_validation() {
        let mut c = DnsConfig::automatic();
        assert_eq!(validate_config(&c), Ok(()));
        c.encryption = DnsEncryption::Dot {
            server_name: None,
            allow_fallback: true,
        };
        assert_eq!(
            validate_config(&c),
            Err(DnsInputError::EncryptionNeedsStaticServers)
        );
        c.ipv4 = FamilyDns::Static {
            servers: vec![v4("9.9.9.9")],
        };
        assert_eq!(validate_config(&c), Ok(()));
        c.encryption = DnsEncryption::Dot {
            server_name: Some("bad name".into()),
            allow_fallback: false,
        };
        assert_eq!(validate_config(&c), Err(DnsInputError::BadDotHost));
    }

    #[test]
    fn summary_text_and_serde() {
        let r = resolve(
            &request(DnsTarget::Custom(quad9()), EncryptionChoice::Doh),
            BOTH,
        )
        .expect("doh");
        let summary = r.config.summary();
        assert_eq!(
            summary.to_string(),
            "IPv4 9.9.9.9, 149.112.112.112; IPv6 2620:fe::fe, 2620:fe::9; DoH"
        );
        assert_eq!(
            DnsConfig::automatic().summary().to_string(),
            "IPv4 automatic; IPv6 automatic"
        );
        let json = serde_json::to_value(&summary).expect("ser");
        assert_eq!(json["ipv4"]["mode"], "static");
        assert_eq!(json["ipv4"]["servers"][0], "9.9.9.9");
        assert_eq!(json["encryption"], "doh");
        let back: DnsSummary = serde_json::from_value(json).expect("de");
        assert_eq!(back, summary);

        let req: DnsRequest =
            serde_json::from_str(r#"{"target":{"kind":"provider","provider":"quad9"}}"#)
                .expect("req");
        assert_eq!(req.encryption, EncryptionChoice::Preferred);
        assert!(!req.allow_fallback);
        let custom: DnsTarget = serde_json::from_str(
            r#"{"kind":"custom","ipv4":["192.0.2.53"],"doh":"https://x.example/q"}"#,
        )
        .expect("custom");
        assert_eq!(
            custom,
            DnsTarget::Custom(DnsEndpoints {
                ipv4: vec![v4("192.0.2.53")],
                doh: Some("https://x.example/q".into()),
                ..Default::default()
            })
        );
    }
}
