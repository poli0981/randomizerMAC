//! The public DNS providers RandomMac offers (`catalog.json`).
//!
//! Every address and endpoint was checked against the provider's own documentation (the
//! `sources` of each entry, on `verifiedOn`); DNSCrypt stamps come from the signed list of the
//! DNSCrypt project. RandomMac applies plain DNS, DoH and DoT natively and only shows the DNSCrypt
//! stamps, for use with a DNSCrypt client such as dnscrypt-proxy.

use std::net::{Ipv4Addr, Ipv6Addr};
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use super::model::DnsEndpoints;

/// What a variant filters, as a generic label front ends translate (no provider-specific text).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum VariantLabel {
    /// The provider's standard service.
    Standard,
    /// No content filtering.
    NoFiltering,
    /// Blocks malware and phishing.
    Malware,
    /// Blocks malware and adult content.
    Family,
    /// Blocks adult content.
    Adult,
    /// Blocks ads and trackers (and malware).
    AdBlocking,
    /// Child protection.
    Child,
    /// Child protection and ad blocking.
    ChildNoAds,
    /// Blocks malware and sends EDNS Client Subnet.
    Ecs,
    /// Blocks social networks.
    Social,
    /// Resolves names that are blocked by censorship elsewhere.
    Uncensored,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DnscryptStamp {
    /// Resolver name in the DNSCrypt list.
    pub name: String,
    /// `address:port` the stamp points to.
    pub address: String,
    /// `sdns://…`
    pub stamp: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Variant {
    pub id: String,
    #[serde(default)]
    pub default: bool,
    pub label: VariantLabel,
    pub ipv4: Vec<Ipv4Addr>,
    #[serde(default)]
    pub ipv6: Vec<Ipv6Addr>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doh: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dot: Option<String>,
    #[serde(default)]
    pub dnscrypt: Vec<DnscryptStamp>,
}

impl Variant {
    pub fn endpoints(&self) -> DnsEndpoints {
        DnsEndpoints {
            ipv4: self.ipv4.clone(),
            ipv6: self.ipv6.clone(),
            doh: self.doh.clone(),
            dot: self.dot.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Provider {
    pub id: String,
    pub name: String,
    pub homepage: String,
    pub sources: Vec<String>,
    pub verified_on: String,
    pub variants: Vec<Variant>,
}

impl Provider {
    /// The variant with this id, or the default variant when `id` is `None`.
    pub fn variant(&self, id: Option<&str>) -> Option<&Variant> {
        match id {
            Some(id) => self.variants.iter().find(|v| v.id.eq_ignore_ascii_case(id)),
            None => self
                .variants
                .iter()
                .find(|v| v.default)
                .or(self.variants.first()),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Catalog {
    pub providers: Vec<Provider>,
}

impl Catalog {
    pub fn provider(&self, id: &str) -> Option<&Provider> {
        self.providers
            .iter()
            .find(|p| p.id.eq_ignore_ascii_case(id))
    }

    /// Parses catalog JSON (`catalog.json`, or a test fixture).
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

/// The built-in catalog. Should the embedded file ever fail to parse, the catalog is empty (and
/// the error is logged) rather than taking the app down; a unit test makes that impossible.
pub fn catalog() -> &'static Catalog {
    static CATALOG: OnceLock<Catalog> = OnceLock::new();
    CATALOG.get_or_init(|| {
        Catalog::from_json(include_str!("catalog.json")).unwrap_or_else(|e| {
            tracing::error!("the built-in DNS catalog does not parse: {e}");
            Catalog::default()
        })
    })
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::net::IpAddr;

    use super::*;
    use crate::dns::model::{is_host_name, validate_doh_template};

    /// Decodes the base64url payload of an `sdns://` stamp.
    fn stamp_bytes(stamp: &str) -> Vec<u8> {
        let payload = stamp.strip_prefix("sdns://").expect("sdns:// prefix");
        let mut bits: u32 = 0;
        let mut count = 0;
        let mut out = Vec::new();
        for c in payload.bytes() {
            let v = match c {
                b'A'..=b'Z' => c - b'A',
                b'a'..=b'z' => c - b'a' + 26,
                b'0'..=b'9' => c - b'0' + 52,
                b'-' => 62,
                b'_' => 63,
                _ => panic!("not base64url: {stamp}"),
            };
            bits = (bits << 6) | u32::from(v);
            count += 6;
            if count >= 8 {
                count -= 8;
                out.push((bits >> count) as u8);
            }
        }
        out
    }

    #[test]
    fn the_embedded_catalog_parses_and_is_consistent() {
        let parsed = Catalog::from_json(include_str!("catalog.json")).expect("catalog.json parses");
        assert_eq!(catalog(), &parsed);
        assert!(parsed.providers.len() >= 8);
        let mut ids = HashSet::new();
        for p in &parsed.providers {
            assert!(ids.insert(p.id.as_str()), "duplicate provider {}", p.id);
            assert!(!p.name.is_empty());
            assert!(p.homepage.starts_with("https://"), "{}", p.id);
            assert!(
                !p.sources.is_empty() && p.sources.iter().all(|s| s.starts_with("https://")),
                "{}",
                p.id
            );
            assert_eq!(p.verified_on.len(), 10, "{}", p.id);
            assert_eq!(
                p.variants.iter().filter(|v| v.default).count(),
                1,
                "{}: one default",
                p.id
            );
            let mut variant_ids = HashSet::new();
            for v in &p.variants {
                let at = format!("{}/{}", p.id, v.id);
                assert!(variant_ids.insert(v.id.as_str()), "duplicate variant {at}");
                assert!((1..=2).contains(&v.ipv4.len()), "{at}: 1–2 IPv4 servers");
                assert!(v.ipv6.len() <= 2, "{at}");
                let servers: Vec<IpAddr> = v.ipv4.iter().map(|a| IpAddr::V4(*a)).collect();
                if let Some(doh) = &v.doh {
                    assert_eq!(validate_doh_template(doh, &servers), Ok(()), "{at}: {doh}");
                }
                if let Some(dot) = &v.dot {
                    assert!(is_host_name(dot), "{at}: {dot}");
                }
                for s in &v.dnscrypt {
                    let bytes = stamp_bytes(&s.stamp);
                    assert_eq!(bytes[0], 0x01, "{at}: {} is a DNSCrypt stamp", s.name);
                    let len = usize::from(bytes[9]);
                    let address = std::str::from_utf8(&bytes[10..10 + len]).expect("utf-8 address");
                    assert_eq!(address, s.address, "{at}: {}", s.name);
                }
            }
        }
        // Mullvad's public DNS was discontinued on 2026-11-02.
        assert!(parsed.provider("mullvad").is_none());
    }

    #[test]
    fn lookups() {
        let c = catalog();
        let quad9 = c.provider("Quad9").expect("quad9");
        assert_eq!(quad9.variant(None).expect("default").id, "recommended");
        assert_eq!(
            quad9.variant(Some("ECS")).expect("ecs").label,
            VariantLabel::Ecs
        );
        assert!(quad9.variant(Some("nope")).is_none());
        let e = quad9.variant(None).expect("default").endpoints();
        assert_eq!(e.ipv4[0], Ipv4Addr::new(9, 9, 9, 9));
        assert_eq!(e.dot.as_deref(), Some("dns.quad9.net"));
        assert!(c.provider("nope").is_none());

        let no_default = Provider {
            id: "x".into(),
            name: "X".into(),
            homepage: "https://x.example".into(),
            sources: vec![],
            verified_on: String::new(),
            variants: vec![Variant {
                id: "only".into(),
                default: false,
                label: VariantLabel::Standard,
                ipv4: vec![Ipv4Addr::new(192, 0, 2, 1)],
                ipv6: vec![],
                doh: None,
                dot: None,
                dnscrypt: vec![],
            }],
        };
        assert_eq!(no_default.variant(None).expect("first").id, "only");
        assert!(Catalog::from_json("{").is_err());
    }
}
