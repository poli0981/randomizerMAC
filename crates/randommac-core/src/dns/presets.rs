//! Custom DNS presets: named server sets the user saves in the settings.

use std::collections::HashSet;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use serde::{Deserialize, Deserializer, Serialize};

use super::model::{
    DnsConfig, DnsEncryption, DnsEndpoints, DnsInputError, FamilyDns, MAX_SERVERS_PER_FAMILY,
    validate_config, validate_doh_template, validate_dot_host,
};
use crate::random::new_id;

pub const MAX_PRESETS: usize = 20;
pub const MAX_PRESET_NAME: usize = 40;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomDnsPreset {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub ipv4: Vec<Ipv4Addr>,
    #[serde(default)]
    pub ipv6: Vec<Ipv6Addr>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doh_template: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dot_host: Option<String>,
}

impl CustomDnsPreset {
    pub fn endpoints(&self) -> DnsEndpoints {
        DnsEndpoints {
            ipv4: self.ipv4.clone(),
            ipv6: self.ipv6.clone(),
            doh: self.doh_template.clone(),
            dot: self.dot_host.clone(),
        }
    }

    /// The servers, the DoH template and the DoT name must be usable as they are.
    pub fn validate(&self) -> Result<(), DnsInputError> {
        if self.ipv4.is_empty() && self.ipv6.is_empty() {
            return Err(DnsInputError::NoServers);
        }
        let config = DnsConfig {
            ipv4: FamilyDns::from_servers(&self.ipv4),
            ipv6: FamilyDns::from_servers(&self.ipv6),
            encryption: DnsEncryption::None,
        };
        validate_config(&config)?;
        let servers: Vec<IpAddr> = config.servers();
        if let Some(template) = &self.doh_template {
            validate_doh_template(template, &servers)?;
        }
        if let Some(host) = &self.dot_host {
            validate_dot_host(host)?;
        }
        Ok(())
    }
}

/// Reads the presets one by one: an entry that does not parse is dropped instead of discarding
/// the whole list (settings stay loadable after a bad hand edit or import).
pub fn lenient_presets<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<CustomDnsPreset>, D::Error> {
    let value = serde_json::Value::deserialize(deserializer)?;
    let serde_json::Value::Array(items) = value else {
        return Ok(Vec::new());
    };
    Ok(items
        .into_iter()
        .filter_map(|v| serde_json::from_value(v).ok())
        .collect())
}

fn is_id(id: &str) -> bool {
    id.len() == 16 && id.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Keeps at most [`MAX_PRESETS`] usable presets: trims names, drops presets that are invalid or
/// whose name repeats (ignoring case), and gives every preset a unique 16-hex-digit id.
pub fn sanitize(presets: &mut Vec<CustomDnsPreset>) {
    let mut names = HashSet::new();
    let mut ids = HashSet::new();
    presets.retain_mut(|p| {
        p.name = p.name.trim().chars().take(MAX_PRESET_NAME).collect();
        p.doh_template = p
            .doh_template
            .take()
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty());
        p.dot_host = p
            .dot_host
            .take()
            .map(|h| h.trim().to_string())
            .filter(|h| !h.is_empty());
        let usable = !p.name.is_empty()
            && p.ipv4.len() <= MAX_SERVERS_PER_FAMILY
            && p.ipv6.len() <= MAX_SERVERS_PER_FAMILY
            && p.validate().is_ok();
        if !usable || !names.insert(p.name.to_lowercase()) {
            return false;
        }
        if !is_id(&p.id) || ids.contains(&p.id) {
            p.id = new_id();
        }
        ids.insert(p.id.clone());
        true
    });
    presets.truncate(MAX_PRESETS);
}

/// The preset with this id, or else with this name (ignoring case; the CLI accepts names).
pub fn find<'a>(presets: &'a [CustomDnsPreset], key: &str) -> Option<&'a CustomDnsPreset> {
    let key = key.trim();
    let lower = key.to_lowercase();
    presets
        .iter()
        .find(|p| p.id == key)
        .or_else(|| presets.iter().find(|p| p.name.to_lowercase() == lower))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn preset(id: &str, name: &str, ipv4: &[&str]) -> CustomDnsPreset {
        CustomDnsPreset {
            id: id.into(),
            name: name.into(),
            ipv4: ipv4.iter().map(|s| s.parse().expect("v4")).collect(),
            ipv6: vec![],
            doh_template: None,
            dot_host: None,
        }
    }

    #[test]
    fn sanitize_keeps_usable_presets() {
        let mut list = vec![
            preset("0123456789abcdef", "  Home  ", &["192.168.1.1"]),
            preset("0123456789abcdef", "Office", &["10.0.0.53"]), // duplicate id → new id
            preset("bad", "HOME", &["192.0.2.1"]),                // duplicate name
            preset("1123456789abcdef", "", &["192.0.2.1"]),       // no name
            preset("2123456789abcdef", "Nothing", &[]),           // no server
            preset("3123456789abcdef", "Broadcast", &["255.255.255.255"]),
            preset(
                "4123456789abcdef",
                "Three",
                &["192.0.2.1", "192.0.2.2", "192.0.2.3"],
            ),
        ];
        let mut with_doh = preset("x", "Encrypted", &["9.9.9.9"]);
        with_doh.doh_template = Some(" https://dns.quad9.net/dns-query ".into());
        with_doh.dot_host = Some("  ".into());
        list.push(with_doh);
        let mut bad_doh = preset("y", "Bad DoH", &["9.9.9.9"]);
        bad_doh.doh_template = Some("http://insecure.example/q".into());
        list.push(bad_doh);

        sanitize(&mut list);
        let names: Vec<&str> = list.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["Home", "Office", "Encrypted"]);
        assert_eq!(list[0].id, "0123456789abcdef");
        assert_ne!(list[1].id, list[0].id);
        assert!(is_id(&list[1].id) && is_id(&list[2].id));
        assert_eq!(
            list[2].doh_template.as_deref(),
            Some("https://dns.quad9.net/dns-query")
        );
        assert_eq!(list[2].dot_host, None);
    }

    #[test]
    fn sanitize_limits_count_and_name_length() {
        let mut list: Vec<CustomDnsPreset> = (0..30)
            .map(|i| {
                preset(
                    &format!("{i:016x}"),
                    &format!("Preset {i} {}", "x".repeat(60)),
                    &["192.0.2.1"],
                )
            })
            .collect();
        sanitize(&mut list);
        assert_eq!(list.len(), MAX_PRESETS);
        assert!(
            list.iter()
                .all(|p| p.name.chars().count() == MAX_PRESET_NAME)
        );
    }

    #[test]
    fn find_by_id_or_name() {
        let list = vec![preset("0123456789abcdef", "Home", &["192.168.1.1"])];
        assert_eq!(find(&list, "0123456789abcdef").expect("id").name, "Home");
        assert_eq!(find(&list, " home ").expect("name").name, "Home");
        assert!(find(&list, "office").is_none());
    }

    #[test]
    fn lenient_parsing_drops_bad_entries_only() {
        #[derive(Deserialize)]
        struct Wrapper {
            #[serde(deserialize_with = "lenient_presets")]
            presets: Vec<CustomDnsPreset>,
        }
        let json = r#"{"presets": [
            {"id": "0123456789abcdef", "name": "Home", "ipv4": ["192.168.1.1"]},
            {"id": "x", "name": "Bad", "ipv4": ["not an address"]},
            42,
            {"id": "1123456789abcdef", "name": "V6", "ipv6": ["2001:db8::53"], "dotHost": "dns.example"}
        ]}"#;
        let w: Wrapper = serde_json::from_str(json).expect("lenient");
        let names: Vec<&str> = w.presets.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["Home", "V6"]);
        let w: Wrapper =
            serde_json::from_str(r#"{"presets": {"not": "a list"}}"#).expect("lenient");
        assert!(w.presets.is_empty());
    }

    #[test]
    fn validation() {
        assert_eq!(
            preset("a", "x", &[]).validate(),
            Err(DnsInputError::NoServers)
        );
        let mut p = preset("a", "x", &["9.9.9.9"]);
        p.dot_host = Some("not a host".into());
        assert_eq!(p.validate(), Err(DnsInputError::BadDotHost));
        p.dot_host = None;
        p.doh_template = Some("https://1.1.1.1/dns-query".into());
        assert_eq!(p.validate(), Err(DnsInputError::DohTemplateIpMismatch));
    }
}
