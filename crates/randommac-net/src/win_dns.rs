//! Windows DNS settings as plain data: the calls a change is made of, and the checks on what
//! Windows reports back. Compiled on every platform so it is tested everywhere; `windows.rs`
//! turns the calls into `SetInterfaceDnsSettings`.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use randommac_core::dns::{
    DnsConfig, DnsEncryption, DnsSummary, DohServerSnapshot, EncryptionMode, FamilyDns,
    WinFamilySnapshot,
};

/// `DNS_DOH_SERVER_SETTINGS` flags.
pub const DOH_ENABLE_AUTO: u64 = 0x1;
pub const DOH_ENABLE: u64 = 0x2;
pub const DOH_FALLBACK_TO_UDP: u64 = 0x4;

/// One `SetInterfaceDnsSettings` call: the settings of one address family.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FamilyCall {
    pub ipv6: bool,
    /// Comma-separated servers; empty = servers from the network.
    pub name_server: String,
    /// DNS-over-HTTPS settings per server, sent when `set_doh`.
    pub doh: Vec<DohServerSnapshot>,
    /// Also write the DoH settings (`DNS_SETTING_DOH`). Done whenever Windows supports DoH, so that
    /// a change clears DoH settings from before: Windows keeps them per server address otherwise.
    pub set_doh: bool,
}

fn joined<A: ToString>(servers: &[A]) -> String {
    servers
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

/// The two calls that apply `config`.
pub fn plan_apply(config: &DnsConfig, doh_supported: bool) -> [FamilyCall; 2] {
    let doh = |count: usize| -> Vec<DohServerSnapshot> {
        match &config.encryption {
            DnsEncryption::Doh {
                template,
                allow_fallback,
            } if doh_supported => (0..count)
                .map(|i| DohServerSnapshot {
                    server_index: u32::try_from(i).unwrap_or(u32::MAX),
                    template: Some(template.clone()),
                    flags: DOH_ENABLE
                        | if *allow_fallback {
                            DOH_FALLBACK_TO_UDP
                        } else {
                            0
                        },
                })
                .collect(),
            _ => Vec::new(),
        }
    };
    let v4 = config.ipv4.servers();
    let v6 = config.ipv6.servers();
    [
        FamilyCall {
            ipv6: false,
            name_server: joined(v4),
            doh: doh(v4.len()),
            set_doh: doh_supported,
        },
        FamilyCall {
            ipv6: true,
            name_server: joined(v6),
            doh: doh(v6.len()),
            set_doh: doh_supported,
        },
    ]
}

/// The two calls that put recorded settings back (the "profile" fields, which only the Settings
/// app writes, are left alone).
pub fn plan_restore(
    ipv4: &WinFamilySnapshot,
    ipv6: &WinFamilySnapshot,
    doh_supported: bool,
) -> [FamilyCall; 2] {
    let call = |s: &WinFamilySnapshot, ipv6| FamilyCall {
        ipv6,
        name_server: s.name_server.clone(),
        doh: if doh_supported {
            s.doh.clone()
        } else {
            Vec::new()
        },
        set_doh: doh_supported,
    };
    [call(ipv4, false), call(ipv6, true)]
}

/// The addresses of a `NameServer` value (comma or space separated); anything else is skipped.
pub fn parse_name_servers(list: &str) -> Vec<IpAddr> {
    list.split([',', ' ', ';'])
        .filter_map(|s| s.trim().parse().ok())
        .collect()
}

fn sorted_doh(list: &[DohServerSnapshot]) -> Vec<(u32, Option<&str>, u64)> {
    let mut v: Vec<_> = list
        .iter()
        .map(|d| {
            (
                d.server_index,
                d.template.as_deref().map(str::trim),
                d.flags,
            )
        })
        .collect();
    v.sort_unstable();
    v
}

/// True when the settings Windows reports are the ones `call` wrote.
pub fn matches(call: &FamilyCall, read: &WinFamilySnapshot) -> bool {
    parse_name_servers(&call.name_server) == parse_name_servers(&read.name_server)
        && (!call.set_doh || sorted_doh(&call.doh) == sorted_doh(&read.doh))
}

fn family<A: Copy>(list: &str, pick: fn(IpAddr) -> Option<A>) -> FamilyDns<A> {
    let servers: Vec<A> = parse_name_servers(list)
        .into_iter()
        .filter_map(pick)
        .collect();
    FamilyDns::from_servers(&servers)
}

fn v4(ip: IpAddr) -> Option<Ipv4Addr> {
    match ip {
        IpAddr::V4(a) => Some(a),
        IpAddr::V6(_) => None,
    }
}

fn v6(ip: IpAddr) -> Option<Ipv6Addr> {
    match ip {
        IpAddr::V6(a) => Some(a),
        IpAddr::V4(_) => None,
    }
}

fn doh_enabled(d: &DohServerSnapshot) -> bool {
    d.flags & (DOH_ENABLE | DOH_ENABLE_AUTO) != 0
}

/// The configuration as a summary (servers configured on the adapter, not the profile fields).
pub fn summary(ipv4: &WinFamilySnapshot, ipv6: &WinFamilySnapshot) -> DnsSummary {
    let encrypted = ipv4.doh.iter().chain(&ipv6.doh).any(doh_enabled);
    DnsSummary {
        ipv4: family(&ipv4.name_server, v4),
        ipv6: family(&ipv6.name_server, v6),
        encryption: if encrypted {
            EncryptionMode::Doh
        } else {
            EncryptionMode::None
        },
    }
}

/// The DoH template and whether fallback to plain DNS is allowed, when DoH is on.
pub fn doh_details(
    ipv4: &WinFamilySnapshot,
    ipv6: &WinFamilySnapshot,
) -> (Option<String>, Option<bool>) {
    let first = ipv4.doh.iter().chain(&ipv6.doh).find(|d| doh_enabled(d));
    match first {
        Some(d) => (d.template.clone(), Some(d.flags & DOH_FALLBACK_TO_UDP != 0)),
        None => (None, None),
    }
}

/// True when the adapter uses the first server of `config` (any server, for "automatic"). Windows
/// may list an adapter's IPv6 servers before its IPv4 ones, so the position does not matter.
pub fn in_use(config: &DnsConfig, effective: &[IpAddr]) -> bool {
    config
        .servers()
        .first()
        .is_none_or(|wanted| effective.contains(wanted))
}

/// Windows reports these site-local addresses as the DNS servers of an IPv6 interface that has
/// none.
pub fn is_placeholder_dns(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V6(a) => {
            let s = a.segments();
            s[..4] == [0xfec0, 0, 0, 0xffff] && s[4..7] == [0, 0, 0] && (1..=3).contains(&s[7])
        }
        IpAddr::V4(_) => false,
    }
}

/// A GUID in registry form (`{D2091925-8A19-41E1-9A2E-2A746835F240}`) as a 128-bit number, the
/// way `windows_sys::core::GUID::from_u128` takes it.
pub fn parse_guid(text: &str) -> Option<u128> {
    let inner = text.trim().strip_prefix('{')?.strip_suffix('}')?;
    let parts: Vec<&str> = inner.split('-').collect();
    let lens = [8, 4, 4, 4, 12];
    if parts.len() != 5
        || parts
            .iter()
            .zip(lens)
            .any(|(p, l)| p.len() != l || !p.bytes().all(|b| b.is_ascii_hexdigit()))
    {
        return None;
    }
    u128::from_str_radix(&parts.concat(), 16).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(v4: &[&str], v6: &[&str], encryption: DnsEncryption) -> DnsConfig {
        DnsConfig {
            ipv4: FamilyDns::from_servers(
                &v4.iter()
                    .map(|s| s.parse().expect("v4"))
                    .collect::<Vec<Ipv4Addr>>(),
            ),
            ipv6: FamilyDns::from_servers(
                &v6.iter()
                    .map(|s| s.parse().expect("v6"))
                    .collect::<Vec<Ipv6Addr>>(),
            ),
            encryption,
        }
    }

    const QUAD9: &str = "https://dns.quad9.net/dns-query";

    fn doh(fallback: bool) -> DnsEncryption {
        DnsEncryption::Doh {
            template: QUAD9.into(),
            allow_fallback: fallback,
        }
    }

    #[test]
    fn plans_for_plain_doh_and_automatic() {
        let plain = config(
            &["9.9.9.9", "149.112.112.112"],
            &["2620:fe::fe"],
            DnsEncryption::None,
        );
        let [a, b] = plan_apply(&plain, true);
        assert_eq!(a.name_server, "9.9.9.9,149.112.112.112");
        assert!(!a.ipv6 && b.ipv6);
        assert_eq!(b.name_server, "2620:fe::fe");
        // DoH supported: plain DNS still writes (empty) DoH settings to clear old ones.
        assert!(a.set_doh && a.doh.is_empty());

        let [a, b] = plan_apply(
            &config(
                &["9.9.9.9", "149.112.112.112"],
                &["2620:fe::fe"],
                doh(false),
            ),
            true,
        );
        assert_eq!(a.doh.len(), 2);
        assert_eq!(
            a.doh[1],
            DohServerSnapshot {
                server_index: 1,
                template: Some(QUAD9.into()),
                flags: DOH_ENABLE
            }
        );
        assert_eq!(b.doh.len(), 1);
        let [a, _] = plan_apply(&config(&["9.9.9.9"], &[], doh(true)), true);
        assert_eq!(a.doh[0].flags, DOH_ENABLE | DOH_FALLBACK_TO_UDP);

        // No DoH API (Windows 10): no DoH settings at all.
        let [a, _] = plan_apply(&config(&["9.9.9.9"], &[], doh(false)), false);
        assert!(!a.set_doh && a.doh.is_empty());

        let [a, b] = plan_apply(&DnsConfig::automatic(), true);
        assert_eq!((a.name_server.as_str(), b.name_server.as_str()), ("", ""));
        assert!(a.set_doh);
    }

    fn snap(name_server: &str, doh: Vec<DohServerSnapshot>) -> WinFamilySnapshot {
        WinFamilySnapshot {
            name_server: name_server.into(),
            doh,
            ..Default::default()
        }
    }

    #[test]
    fn restore_plans_and_matching() {
        let ipv4 = snap(
            "192.0.2.1",
            vec![DohServerSnapshot {
                server_index: 0,
                template: None,
                flags: DOH_ENABLE_AUTO,
            }],
        );
        let ipv6 = snap("", vec![]);
        let [a, b] = plan_restore(&ipv4, &ipv6, true);
        assert_eq!(a.name_server, "192.0.2.1");
        assert_eq!(a.doh, ipv4.doh);
        assert!(b.ipv6 && b.name_server.is_empty());
        assert!(plan_restore(&ipv4, &ipv6, false)[0].doh.is_empty());

        assert!(matches(&a, &ipv4));
        // Order and separators of the server list do not matter for the DoH comparison's input.
        let read = snap("192.0.2.1 ", ipv4.doh.clone());
        assert!(matches(&a, &read));
        assert!(!matches(&a, &snap("192.0.2.2", ipv4.doh.clone())));
        assert!(!matches(&a, &snap("192.0.2.1", vec![])));
        // Without set_doh the DoH settings are not compared.
        let no_doh = FamilyCall {
            set_doh: false,
            ..a.clone()
        };
        assert!(matches(&no_doh, &snap("192.0.2.1", vec![])));
    }

    #[test]
    fn summaries_and_details() {
        let ipv4 = snap(
            "9.9.9.9,149.112.112.112",
            vec![DohServerSnapshot {
                server_index: 0,
                template: Some(QUAD9.into()),
                flags: DOH_ENABLE | DOH_FALLBACK_TO_UDP,
            }],
        );
        let ipv6 = snap("", vec![]);
        let s = summary(&ipv4, &ipv6);
        assert_eq!(
            s.to_string(),
            "IPv4 9.9.9.9, 149.112.112.112; IPv6 automatic; DoH"
        );
        assert_eq!(doh_details(&ipv4, &ipv6), (Some(QUAD9.into()), Some(true)));
        let plain = snap("9.9.9.9", vec![]);
        assert_eq!(summary(&plain, &ipv6).encryption, EncryptionMode::None);
        assert_eq!(doh_details(&plain, &ipv6), (None, None));
        // A v6 address in the v4 list (or garbage) is ignored.
        assert_eq!(
            summary(&snap("2620:fe::fe,x", vec![]), &ipv6).ipv4,
            FamilyDns::Automatic
        );
    }

    #[test]
    fn in_use_ignores_the_order_windows_lists_servers_in() {
        let ip = |s: &str| s.parse::<IpAddr>().expect("ip");
        let quad9 = config(
            &["9.9.9.9", "149.112.112.112"],
            &["2620:fe::fe"],
            DnsEncryption::None,
        );
        // Seen on Windows 11 24H2: the IPv6 servers come first.
        let effective = [ip("2620:fe::fe"), ip("9.9.9.9"), ip("149.112.112.112")];
        assert!(in_use(&quad9, &effective));
        assert!(!in_use(&quad9, &[ip("192.168.1.1"), ip("2620:fe::fe")]));
        assert!(!in_use(&quad9, &[]));
        let v6_only = config(&[], &["2620:fe::9"], DnsEncryption::None);
        assert!(in_use(&v6_only, &[ip("192.168.1.1"), ip("2620:fe::9")]));
        // Automatic: whatever the network hands out.
        let automatic = config(&[], &[], DnsEncryption::None);
        assert!(in_use(&automatic, &[]));
        assert!(in_use(&automatic, &[ip("192.168.1.1")]));
    }

    #[test]
    fn placeholders_and_guids() {
        for p in ["fec0:0:0:ffff::1", "fec0:0:0:ffff::2", "fec0:0:0:ffff::3"] {
            assert!(is_placeholder_dns(&p.parse().expect("ip")), "{p}");
        }
        for real in ["fec0:0:0:ffff::4", "2620:fe::fe", "9.9.9.9"] {
            assert!(!is_placeholder_dns(&real.parse().expect("ip")), "{real}");
        }
        assert_eq!(
            parse_guid("{D2091925-8A19-41E1-9A2E-2A746835F240}"),
            Some(0xD2091925_8A19_41E1_9A2E_2A746835F240)
        );
        for bad in [
            "D2091925-8A19-41E1-9A2E-2A746835F240",
            "{D2091925-8A19-41E1-9A2E}",
            "{G2091925-8A19-41E1-9A2E-2A746835F240}",
            "{}",
        ] {
            assert_eq!(parse_guid(bad), None, "{bad}");
        }
        assert_eq!(
            parse_name_servers("9.9.9.9, 1.1.1.1;bad"),
            vec![
                "9.9.9.9".parse::<IpAddr>().expect("ip"),
                "1.1.1.1".parse().expect("ip")
            ]
        );
    }
}
