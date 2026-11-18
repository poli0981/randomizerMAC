//! NetworkManager DNS settings as plain data: the nmcli arguments of a change and the parsing of
//! what nmcli prints (formats taken from NetworkManager 1.46 on Ubuntu 24.04). Compiled on every
//! platform so it is tested everywhere; `linux.rs` runs the commands.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use randommac_core::dns::{
    DnsConfig, DnsEncryption, DnsSummary, EncryptionMode, FamilyDns, NmDnsProfile,
};

use crate::dns_types::codes;
use crate::linux_parse::unescape_value;

/// The profile properties RandomMac reads, in the order `nmcli -g` prints them.
pub const PROFILE_FIELDS: &str = "ipv4.dns,ipv4.ignore-auto-dns,ipv4.method,ipv6.dns,ipv6.ignore-auto-dns,ipv6.method,connection.dns-over-tls";
/// The servers a device uses (`nmcli -g IP4.DNS,IP6.DNS device show <ifname>`).
pub const EFFECTIVE_FIELDS: &str = "IP4.DNS,IP6.DNS";

/// A profile's DNS properties, unescaped.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NmProfileDns {
    pub ipv4_dns: String,
    pub ipv4_ignore_auto_dns: String,
    pub ipv4_method: String,
    pub ipv6_dns: String,
    pub ipv6_ignore_auto_dns: String,
    pub ipv6_method: String,
    pub dns_over_tls: String,
}

/// Parses `nmcli -g PROFILE_FIELDS connection show uuid <UUID>`: seven lines, one per property.
pub fn parse_profile(stdout: &str) -> Option<NmProfileDns> {
    let lines: Vec<String> = stdout
        .lines()
        .map(|l| unescape_value(l.trim_end_matches('\r')))
        .collect();
    let [
        ipv4_dns,
        ipv4_ignore,
        ipv4_method,
        ipv6_dns,
        ipv6_ignore,
        ipv6_method,
        dot,
    ] = lines.as_slice()
    else {
        return None;
    };
    Some(NmProfileDns {
        ipv4_dns: ipv4_dns.clone(),
        ipv4_ignore_auto_dns: ipv4_ignore.clone(),
        ipv4_method: ipv4_method.clone(),
        ipv6_dns: ipv6_dns.clone(),
        ipv6_ignore_auto_dns: ipv6_ignore.clone(),
        ipv6_method: ipv6_method.clone(),
        dns_over_tls: dot.clone(),
    })
}

/// The servers of an `ipv4.dns` / `ipv6.dns` value, with the TLS server name of `IP#name`.
pub fn parse_server_list(list: &str) -> Vec<(IpAddr, Option<String>)> {
    list.split([',', ' '])
        .filter_map(|item| {
            let item = item.trim();
            let (ip, name) = match item.split_once('#') {
                Some((ip, name)) => (ip, Some(name.to_string()).filter(|n| !n.is_empty())),
                None => (item, None),
            };
            ip.parse().ok().map(|ip| (ip, name))
        })
        .collect()
}

/// The servers a device uses: `IP4.DNS` and `IP6.DNS`, each a line of `a | b` values.
pub fn parse_effective(stdout: &str) -> (Vec<Ipv4Addr>, Vec<Ipv6Addr>) {
    let mut lines = stdout
        .lines()
        .map(|l| unescape_value(l.trim_end_matches('\r')));
    let list = |line: Option<String>| -> Vec<IpAddr> {
        line.unwrap_or_default()
            .split(['|', ','])
            .filter_map(|s| s.trim().parse().ok())
            .collect()
    };
    let v4 = list(lines.next()).into_iter().filter_map(|ip| match ip {
        IpAddr::V4(a) => Some(a),
        IpAddr::V6(_) => None,
    });
    let v6 = list(lines.next()).into_iter().filter_map(|ip| match ip {
        IpAddr::V6(a) => Some(a),
        IpAddr::V4(_) => None,
    });
    (v4.collect(), v6.collect())
}

/// NetworkManager refuses DNS servers on a family whose method is one of these.
pub fn family_allows_dns(method: &str, ipv6: bool) -> bool {
    let refused: &[&str] = if ipv6 {
        &["disabled", "ignore", "link-local"]
    } else {
        &["disabled", "link-local"]
    };
    !refused.contains(&method.trim())
}

/// `connection.dns-over-tls`: nmcli prints the number and accepts the name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NmDot {
    Default,
    No,
    Opportunistic,
    Yes,
}

pub fn parse_dot(value: &str) -> NmDot {
    match value.trim() {
        "0" | "no" => NmDot::No,
        "1" | "opportunistic" => NmDot::Opportunistic,
        "2" | "yes" => NmDot::Yes,
        _ => NmDot::Default,
    }
}

impl NmDot {
    pub fn as_arg(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::No => "no",
            Self::Opportunistic => "opportunistic",
            Self::Yes => "yes",
        }
    }
}

impl NmProfileDns {
    /// The values a restore writes back.
    pub fn snapshot(&self) -> NmDnsProfile {
        NmDnsProfile {
            ipv4_dns: self.ipv4_dns.clone(),
            ipv4_ignore_auto_dns: self.ipv4_ignore_auto_dns.clone(),
            ipv6_dns: self.ipv6_dns.clone(),
            ipv6_ignore_auto_dns: self.ipv6_ignore_auto_dns.clone(),
            dns_over_tls: self.dns_over_tls.clone(),
        }
    }

    pub fn summary(&self) -> DnsSummary {
        summary_of(&self.ipv4_dns, &self.ipv6_dns, &self.dns_over_tls)
    }

    /// The TLS server name of the first server, if any.
    pub fn dot_server_name(&self) -> Option<String> {
        parse_server_list(&self.ipv4_dns)
            .into_iter()
            .chain(parse_server_list(&self.ipv6_dns))
            .find_map(|(_, name)| name)
    }
}

/// A summary of recorded or read values.
pub fn summary_of(ipv4_dns: &str, ipv6_dns: &str, dot: &str) -> DnsSummary {
    let v4: Vec<Ipv4Addr> = parse_server_list(ipv4_dns)
        .into_iter()
        .filter_map(|(ip, _)| match ip {
            IpAddr::V4(a) => Some(a),
            IpAddr::V6(_) => None,
        })
        .collect();
    let v6: Vec<Ipv6Addr> = parse_server_list(ipv6_dns)
        .into_iter()
        .filter_map(|(ip, _)| match ip {
            IpAddr::V6(a) => Some(a),
            IpAddr::V4(_) => None,
        })
        .collect();
    let encrypted = matches!(parse_dot(dot), NmDot::Yes | NmDot::Opportunistic);
    DnsSummary {
        ipv4: FamilyDns::from_servers(&v4),
        ipv6: FamilyDns::from_servers(&v6),
        encryption: if encrypted && !(v4.is_empty() && v6.is_empty()) {
            EncryptionMode::Dot
        } else {
            EncryptionMode::None
        },
    }
}

fn server_list<A: ToString>(servers: &[A], sni: Option<&str>) -> String {
    servers
        .iter()
        .map(|s| match sni {
            Some(name) => format!("{}#{name}", s.to_string()),
            None => s.to_string(),
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn family_args<A: ToString>(
    prefix: &str,
    dns: &FamilyDns<A>,
    sni: Option<&str>,
    args: &mut Vec<String>,
) {
    let (list, ignore) = match dns {
        FamilyDns::Automatic => (String::new(), "no"),
        FamilyDns::Static { servers } => (server_list(servers, sni), "yes"),
    };
    args.extend([
        format!("{prefix}.dns"),
        list,
        format!("{prefix}.ignore-auto-dns"),
        ignore.to_string(),
    ]);
}

/// The values a change writes: the `connection modify uuid <UUID>` arguments that follow, and the
/// warnings for families the profile cannot take DNS servers on.
pub fn modify_args(config: &DnsConfig, current: &NmProfileDns) -> (Vec<String>, Vec<&'static str>) {
    let (sni, dot) = match &config.encryption {
        DnsEncryption::Dot {
            server_name,
            allow_fallback,
        } => (
            server_name.as_deref(),
            if *allow_fallback {
                NmDot::Opportunistic
            } else {
                NmDot::Yes
            },
        ),
        _ => (None, NmDot::Default),
    };
    let mut args = Vec::new();
    let mut warnings = Vec::new();
    if family_allows_dns(&current.ipv4_method, false) {
        family_args("ipv4", &config.ipv4, sni, &mut args);
    } else if !config.ipv4.is_automatic() {
        warnings.push(codes::DNS_IPV4_SKIPPED);
    }
    if family_allows_dns(&current.ipv6_method, true) {
        family_args("ipv6", &config.ipv6, sni, &mut args);
    } else if !config.ipv6.is_automatic() {
        warnings.push(codes::DNS_IPV6_SKIPPED);
    }
    args.extend([
        "connection.dns-over-tls".to_string(),
        dot.as_arg().to_string(),
    ]);
    (args, warnings)
}

/// The arguments that put recorded values back (families the profile now refuses are left out).
pub fn restore_args(recorded: &NmDnsProfile, current: &NmProfileDns) -> Vec<String> {
    let mut args = Vec::new();
    if family_allows_dns(&current.ipv4_method, false) {
        args.extend([
            "ipv4.dns".to_string(),
            recorded.ipv4_dns.clone(),
            "ipv4.ignore-auto-dns".to_string(),
            bool_arg(&recorded.ipv4_ignore_auto_dns),
        ]);
    }
    if family_allows_dns(&current.ipv6_method, true) {
        args.extend([
            "ipv6.dns".to_string(),
            recorded.ipv6_dns.clone(),
            "ipv6.ignore-auto-dns".to_string(),
            bool_arg(&recorded.ipv6_ignore_auto_dns),
        ]);
    }
    args.extend([
        "connection.dns-over-tls".to_string(),
        parse_dot(&recorded.dns_over_tls).as_arg().to_string(),
    ]);
    args
}

fn bool_arg(value: &str) -> String {
    if value.trim() == "yes" { "yes" } else { "no" }.to_string()
}

/// True when a re-read profile holds the values `written` (from [`modify_args`]).
pub fn profile_matches(written: &[String], read: &NmProfileDns) -> bool {
    let value = |key: &str| {
        written
            .chunks(2)
            .find(|pair| pair.first().is_some_and(|k| k == key))
            .and_then(|pair| pair.get(1).cloned())
    };
    let servers_ok = |key: &str, actual: &str| {
        value(key).is_none_or(|expected| parse_server_list(&expected) == parse_server_list(actual))
    };
    let dot_expected = value("connection.dns-over-tls").map(|v| parse_dot(&v));
    servers_ok("ipv4.dns", &read.ipv4_dns)
        && servers_ok("ipv6.dns", &read.ipv6_dns)
        && dot_expected.is_none_or(|d| d == parse_dot(&read.dns_over_tls))
}

/// The value of `busctl get-property` for a string (`s "systemd-resolved"`).
pub fn parse_busctl_string(stdout: &str) -> Option<String> {
    let rest = stdout.trim().strip_prefix("s ")?;
    Some(rest.trim().trim_matches('"').to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `nmcli -g PROFILE_FIELDS connection show` after setting Quad9 with DoT (NM 1.46).
    const QUAD9_DOT: &str = "9.9.9.9#dns.quad9.net,149.112.112.112#dns.quad9.net\nyes\nmanual\n2620\\:fe\\:\\:fe#dns.quad9.net,2620\\:fe\\:\\:9#dns.quad9.net\nyes\nmanual\n2\n";
    const AUTOMATIC: &str = "\nno\nauto\n\nno\nauto\n-1\n";

    fn quad9(encryption: DnsEncryption) -> DnsConfig {
        DnsConfig {
            ipv4: FamilyDns::Static {
                servers: vec![
                    "9.9.9.9".parse().expect("ip"),
                    "149.112.112.112".parse().expect("ip"),
                ],
            },
            ipv6: FamilyDns::Static {
                servers: vec!["2620:fe::fe".parse().expect("ip")],
            },
            encryption,
        }
    }

    #[test]
    fn profiles() {
        let p = parse_profile(QUAD9_DOT).expect("profile");
        assert_eq!(
            p.ipv6_dns,
            "2620:fe::fe#dns.quad9.net,2620:fe::9#dns.quad9.net"
        );
        assert_eq!(p.dns_over_tls, "2");
        assert_eq!(
            p.summary().to_string(),
            "IPv4 9.9.9.9, 149.112.112.112; IPv6 2620:fe::fe, 2620:fe::9; DoT"
        );
        assert_eq!(p.dot_server_name().as_deref(), Some("dns.quad9.net"));
        let snap = p.snapshot();
        assert_eq!(snap.ipv4_ignore_auto_dns, "yes");

        let auto = parse_profile(AUTOMATIC).expect("auto");
        assert_eq!(auto.summary(), DnsConfig::automatic().summary());
        assert_eq!(auto.dot_server_name(), None);
        assert!(parse_profile("a\nb\n").is_none());
        assert!(parse_profile("").is_none());
    }

    #[test]
    fn effective_servers() {
        let (v4, v6) =
            parse_effective("9.9.9.9 | 149.112.112.112\n2620\\:fe\\:\\:fe | 2620\\:fe\\:\\:9\n");
        assert_eq!(v4.len(), 2);
        assert_eq!(
            v6,
            vec![
                "2620:fe::fe".parse::<Ipv6Addr>().expect("ip"),
                "2620:fe::9".parse().expect("ip")
            ]
        );
        assert_eq!(parse_effective("\n\n"), (vec![], vec![]));
        assert_eq!(parse_effective(""), (vec![], vec![]));
    }

    #[test]
    fn family_rules_and_dot_values() {
        assert!(family_allows_dns("manual", false) && family_allows_dns("auto", true));
        assert!(!family_allows_dns("disabled", false) && !family_allows_dns("link-local", false));
        assert!(family_allows_dns("ignore", false));
        assert!(!family_allows_dns("ignore", true) && !family_allows_dns("disabled", true));
        for (value, dot) in [
            ("-1", NmDot::Default),
            ("default", NmDot::Default),
            ("0", NmDot::No),
            ("no", NmDot::No),
            ("1", NmDot::Opportunistic),
            ("2", NmDot::Yes),
            ("yes", NmDot::Yes),
            ("", NmDot::Default),
        ] {
            assert_eq!(parse_dot(value), dot, "{value}");
        }
        assert_eq!(NmDot::Opportunistic.as_arg(), "opportunistic");
    }

    #[test]
    fn modify_and_restore_arguments() {
        let manual = parse_profile(AUTOMATIC)
            .map(|p| NmProfileDns {
                ipv4_method: "manual".into(),
                ..p
            })
            .expect("p");
        let (args, warnings) = modify_args(
            &quad9(DnsEncryption::Dot {
                server_name: Some("dns.quad9.net".into()),
                allow_fallback: false,
            }),
            &manual,
        );
        assert_eq!(
            args,
            [
                "ipv4.dns",
                "9.9.9.9#dns.quad9.net,149.112.112.112#dns.quad9.net",
                "ipv4.ignore-auto-dns",
                "yes",
                "ipv6.dns",
                "2620:fe::fe#dns.quad9.net",
                "ipv6.ignore-auto-dns",
                "yes",
                "connection.dns-over-tls",
                "yes",
            ]
        );
        assert!(warnings.is_empty());

        let (args, _) = modify_args(
            &quad9(DnsEncryption::Dot {
                server_name: None,
                allow_fallback: true,
            }),
            &manual,
        );
        assert_eq!(args[1], "9.9.9.9,149.112.112.112");
        assert_eq!(args.last().map(String::as_str), Some("opportunistic"));

        let (args, _) = modify_args(&DnsConfig::automatic(), &manual);
        assert_eq!(
            args,
            [
                "ipv4.dns",
                "",
                "ipv4.ignore-auto-dns",
                "no",
                "ipv6.dns",
                "",
                "ipv6.ignore-auto-dns",
                "no",
                "connection.dns-over-tls",
                "default"
            ]
        );

        let no_v6 = NmProfileDns {
            ipv6_method: "ignore".into(),
            ..manual.clone()
        };
        let (args, warnings) = modify_args(&quad9(DnsEncryption::None), &no_v6);
        assert!(!args.iter().any(|a| a.starts_with("ipv6")));
        assert_eq!(warnings, [codes::DNS_IPV6_SKIPPED]);
        let no_v4 = NmProfileDns {
            ipv4_method: "disabled".into(),
            ..manual.clone()
        };
        assert_eq!(
            modify_args(&quad9(DnsEncryption::None), &no_v4).1,
            [codes::DNS_IPV4_SKIPPED]
        );

        let recorded = parse_profile(QUAD9_DOT).expect("p").snapshot();
        let back = restore_args(&recorded, &manual);
        assert_eq!(
            back[1],
            "9.9.9.9#dns.quad9.net,149.112.112.112#dns.quad9.net"
        );
        assert_eq!(back.last().map(String::as_str), Some("yes"));
        assert!(
            !restore_args(&recorded, &no_v6)
                .iter()
                .any(|a| a.starts_with("ipv6"))
        );
        assert!(
            !restore_args(&recorded, &no_v4)
                .iter()
                .any(|a| a.starts_with("ipv4"))
        );
        assert_eq!(bool_arg("maybe"), "no");
    }

    #[test]
    fn read_back_comparison() {
        let config = quad9(DnsEncryption::Dot {
            server_name: Some("dns.quad9.net".into()),
            allow_fallback: false,
        });
        let manual = parse_profile(AUTOMATIC).expect("p");
        let (args, _) = modify_args(&config, &manual);
        let read = NmProfileDns {
            ipv4_dns: "9.9.9.9#dns.quad9.net,149.112.112.112#dns.quad9.net".into(),
            ipv6_dns: "2620:fe::fe#dns.quad9.net".into(),
            dns_over_tls: "2".into(),
            ..manual.clone()
        };
        assert!(profile_matches(&args, &read));
        assert!(!profile_matches(
            &args,
            &NmProfileDns {
                dns_over_tls: "-1".into(),
                ..read.clone()
            }
        ));
        assert!(!profile_matches(
            &args,
            &NmProfileDns {
                ipv4_dns: "9.9.9.9".into(),
                ..read
            }
        ));
    }

    #[test]
    fn busctl_strings() {
        assert_eq!(
            parse_busctl_string("s \"systemd-resolved\"\n").as_deref(),
            Some("systemd-resolved")
        );
        assert_eq!(parse_busctl_string("b true"), None);
    }
}
