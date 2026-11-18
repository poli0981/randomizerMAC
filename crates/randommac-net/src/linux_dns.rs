//! DNS settings through NetworkManager: `ipv4.dns`, `ipv6.dns`, their `ignore-auto-dns` and
//! `connection.dns-over-tls` of the device's active profile, applied with `nmcli device reapply`
//! (no reconnect). DNS over TLS needs NetworkManager to hand DNS to systemd-resolved; the resolver
//! cache is left alone (flushing it would ask for authorization).

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use randommac_core::ChangeStatus;
use randommac_core::dns::{DnsConfig, DnsOriginalRecord, DnsSnapshot, NmDnsProfile};

use super::*;
use crate::dns_types::*;
use crate::nm_dns::{self, NmProfileDns};

const BUSCTL: &[&str] = &["/usr/bin/busctl", "/bin/busctl"];
const POLL: Duration = Duration::from_millis(250);

fn connection(adapter: &Adapter) -> Result<&NmConnection, NetError> {
    adapter
        .nm_connection
        .as_ref()
        .ok_or(NetError::NmNoActiveConnection)
}

fn read_profile(uuid: &str) -> Result<NmProfileDns, NetError> {
    let out = LinuxBackend::nm_run(
        &[
            "-g",
            nm_dns::PROFILE_FIELDS,
            "connection",
            "show",
            "uuid",
            uuid,
        ],
        NMCLI_TIMEOUT,
    )?;
    let stdout = out.stdout.clone();
    let code = out.code;
    LinuxBackend::nm_check(out, "connection show")?;
    nm_dns::parse_profile(&stdout).ok_or_else(|| NetError::CommandFailed {
        program: "nmcli connection show".into(),
        code,
        stderr: "unexpected output".into(),
    })
}

fn modify(uuid: &str, args: &[String]) -> Result<(), NetError> {
    let mut argv = vec!["connection", "modify", "uuid", uuid];
    argv.extend(args.iter().map(String::as_str));
    LinuxBackend::nm_check(
        LinuxBackend::nm_run(&argv, NMCLI_TIMEOUT)?,
        "connection modify",
    )
}

/// Makes the device use the profile's new values: `device reapply`, or reconnecting when that
/// fails.
fn activate(ifname: &str, conn: &NmConnection) -> Result<(), NetError> {
    let reapplied = LinuxBackend::nm_run(&["device", "reapply", ifname], NMCLI_TIMEOUT)
        .and_then(|out| LinuxBackend::nm_check(out, "device reapply"));
    match reapplied {
        Ok(()) => Ok(()),
        Err(_) => LinuxBackend::nm_up(conn),
    }
}

fn effective(ifname: &str) -> (Vec<Ipv4Addr>, Vec<Ipv6Addr>) {
    LinuxBackend::nm_run(
        &["-g", nm_dns::EFFECTIVE_FIELDS, "device", "show", ifname],
        NMCLI_TIMEOUT,
    )
    .ok()
    .filter(Output::success)
    .map(|out| nm_dns::parse_effective(&out.stdout))
    .unwrap_or_default()
}

fn in_use(config: &DnsConfig, v4: &[Ipv4Addr], v6: &[Ipv6Addr]) -> bool {
    match config.servers().first() {
        None => true,
        Some(IpAddr::V4(a)) => v4.contains(a),
        Some(IpAddr::V6(a)) => v6.contains(a),
    }
}

/// Waits until the device uses the first requested server (or the timeout passes).
fn wait_until_in_use(ifname: &str, config: &DnsConfig, timeout: Duration) -> (Vec<IpAddr>, bool) {
    let deadline = Instant::now() + timeout;
    loop {
        let (v4, v6) = effective(ifname);
        let ok = in_use(config, &v4, &v6);
        if ok || Instant::now() >= deadline {
            let all = v4
                .into_iter()
                .map(IpAddr::V4)
                .chain(v6.into_iter().map(IpAddr::V6))
                .collect();
            return (all, ok);
        }
        thread::sleep(POLL);
    }
}

/// NetworkManager's DNS mode (`systemd-resolved`, `default`, `dnsmasq`, …).
fn dns_mode() -> Option<String> {
    let busctl = process::find_program(BUSCTL)?;
    let out = process::run(
        &busctl,
        &[
            "--system",
            "--no-pager",
            "get-property",
            "org.freedesktop.NetworkManager",
            "/org/freedesktop/NetworkManager/DnsManager",
            "org.freedesktop.NetworkManager.DnsManager",
            "Mode",
        ],
        None,
        Duration::from_secs(10),
    )
    .ok()?;
    if !out.success() {
        return None;
    }
    nm_dns::parse_busctl_string(&out.stdout)
}

fn family(allowed: bool) -> FeatureSupport {
    if allowed {
        FeatureSupport::YES
    } else {
        FeatureSupport::no(DnsReason::FamilyDisabled)
    }
}

fn recorded_summary(p: &NmDnsProfile) -> randommac_core::dns::DnsSummary {
    nm_dns::summary_of(&p.ipv4_dns, &p.ipv6_dns, &p.dns_over_tls)
}

pub(super) fn capabilities(adapter: &Adapter) -> DnsCapabilities {
    if LinuxBackend::nmcli().is_none() || !LinuxBackend::nm_running() {
        return DnsCapabilities::unsupported(DnsReason::NmUnavailable);
    }
    let Some(conn) = &adapter.nm_connection else {
        let managed = valid_ifname(&adapter.interface_name)
            && LinuxBackend::nm_device(&adapter.interface_name)
                .is_some_and(|d| d.state > NM_DEVICE_STATE_UNMANAGED);
        return DnsCapabilities::unsupported(if managed {
            DnsReason::NoActiveConnection
        } else {
            DnsReason::NotNmManaged
        });
    };
    let Ok(profile) = read_profile(&conn.uuid) else {
        return DnsCapabilities::unsupported(DnsReason::NoActiveConnection);
    };
    let dot = match dns_mode().as_deref() {
        Some("systemd-resolved") => FeatureSupport::YES,
        Some(_) => FeatureSupport::no(DnsReason::DotNeedsResolved),
        None => FeatureSupport::no(DnsReason::BackendUnknown),
    };
    DnsCapabilities {
        supported: true,
        reason: None,
        can_change: true,
        change_reason: None,
        ipv4: family(nm_dns::family_allows_dns(&profile.ipv4_method, false)),
        ipv6: family(nm_dns::family_allows_dns(&profile.ipv6_method, true)),
        doh: FeatureSupport::no(DnsReason::DohUnsupportedLinux),
        dot,
        method: Some(DnsMethod::NetworkManager),
    }
}

pub(super) fn status(adapter: &Adapter) -> Result<DnsStatus, NetError> {
    let conn = connection(adapter)?;
    let profile = read_profile(&conn.uuid)?;
    let (effective_ipv4, effective_ipv6) = effective(&adapter.interface_name);
    let fallback = match nm_dns::parse_dot(&profile.dns_over_tls) {
        nm_dns::NmDot::Opportunistic => Some(true),
        nm_dns::NmDot::Yes => Some(false),
        _ => None,
    };
    Ok(DnsStatus {
        configured: profile.summary(),
        doh_template: None,
        dot_server_name: profile.dot_server_name(),
        fallback,
        effective_ipv4,
        effective_ipv6,
        profile_override: false,
        nm_connection_uuid: Some(conn.uuid.clone()),
    })
}

pub(super) fn snapshot(adapter: &Adapter) -> Result<DnsSnapshot, NetError> {
    let conn = connection(adapter)?;
    let profile = read_profile(&conn.uuid)?;
    Ok(DnsSnapshot::NetworkManager {
        uuid: conn.uuid.clone(),
        profile: profile.snapshot(),
    })
}

pub(super) fn apply(
    adapter: &Adapter,
    config: &DnsConfig,
    previous: &DnsSnapshot,
    opts: &DnsOptions,
    progress: &mut dyn FnMut(Stage),
) -> Result<DnsReport, NetError> {
    let conn = connection(adapter)?;
    let ifname = adapter.interface_name.as_str();
    if !valid_ifname(ifname) {
        return Err(NetError::InvalidInput(format!(
            "invalid interface name: {ifname:?}"
        )));
    }
    progress(Stage::Preparing);
    let current = read_profile(&conn.uuid)?;
    let (args, warnings) = nm_dns::modify_args(config, &current);
    if !config.is_automatic() && !args.iter().any(|a| a == "ipv4.dns" || a == "ipv6.dns") {
        return Err(NetError::Unsupported(
            "this connection profile takes no DNS servers".into(),
        ));
    }
    let before = match previous {
        DnsSnapshot::NetworkManager { uuid, profile } if *uuid == conn.uuid => profile.clone(),
        _ => current.snapshot(),
    };
    let undo = nm_dns::restore_args(&before, &current);
    let rollback = |progress: &mut dyn FnMut(Stage)| -> Result<(), NetError> {
        progress(Stage::RollingBack);
        modify(&conn.uuid, &undo).and_then(|()| activate(ifname, conn))
    };

    progress(Stage::Writing);
    modify(&conn.uuid, &args)?;
    progress(Stage::Restarting);
    if let Err(e) = activate(ifname, conn) {
        return match rollback(progress) {
            Ok(()) => Err(e),
            Err(again) => Err(NetError::RollbackFailed(format!("{e}; restoring: {again}"))),
        };
    }

    progress(Stage::Verifying);
    let read = read_profile(&conn.uuid)?;
    let mut report = DnsReport {
        method: DnsMethod::NetworkManager,
        status: ChangeStatus::Applied,
        previous: Some(recorded_summary(&before)),
        requested: Some(config.summary()),
        observed: Some(read.summary()),
        effective: Vec::new(),
        detail_code: None,
        warnings: warnings.into_iter().map(String::from).collect(),
        cache_flushed: false,
        nm_connection_uuid: Some(conn.uuid.clone()),
    };
    if !nm_dns::profile_matches(&args, &read) {
        rollback(progress).map_err(|e| {
            NetError::RollbackFailed(format!(
                "NetworkManager did not keep the DNS settings; restoring: {e}"
            ))
        })?;
        report.status = ChangeStatus::RolledBack;
        report.detail_code = Some(codes::DNS_NOT_APPLIED.into());
        report.observed = report.previous.clone();
        report.effective = wait_until_in_use(ifname, config, Duration::ZERO).0;
        return Ok(report);
    }
    let timeout = if adapter.connected {
        opts.verify_timeout
    } else {
        Duration::ZERO
    };
    let (mut effective, mut ok) = wait_until_in_use(ifname, config, timeout);
    if !ok && adapter.connected {
        // `reapply` can keep servers from DHCPv6 after ignore-auto-dns changes: reconnect once.
        if LinuxBackend::nm_up(conn).is_ok() {
            (effective, ok) = wait_until_in_use(ifname, config, opts.verify_timeout);
        }
    }
    report.effective = effective;
    if !ok && adapter.connected {
        report.status = ChangeStatus::Unverified;
        report.detail_code = Some(codes::DNS_NOT_ACTIVE.into());
    }
    Ok(report)
}

pub(super) fn restore(
    adapter: &Adapter,
    original: &DnsOriginalRecord,
    opts: &DnsOptions,
    progress: &mut dyn FnMut(Stage),
) -> Result<DnsReport, NetError> {
    if original.nm_profiles.is_empty() {
        return Err(NetError::NoOriginal);
    }
    let ifname = adapter.interface_name.as_str();
    if !valid_ifname(ifname) {
        return Err(NetError::InvalidInput(format!(
            "invalid interface name: {ifname:?}"
        )));
    }
    progress(Stage::Preparing);
    let active = adapter.nm_connection.as_ref();
    let mut report = DnsReport {
        method: DnsMethod::NetworkManager,
        status: ChangeStatus::Applied,
        previous: None,
        requested: None,
        observed: None,
        effective: Vec::new(),
        detail_code: None,
        warnings: Vec::new(),
        cache_flushed: false,
        nm_connection_uuid: active.map(|c| c.uuid.clone()),
    };
    for (uuid, recorded) in &original.nm_profiles {
        // A profile deleted since then has nothing left to restore.
        let Ok(current) = read_profile(uuid) else {
            continue;
        };
        progress(Stage::Writing);
        modify(uuid, &nm_dns::restore_args(recorded, &current))?;
        if let Some(conn) = active.filter(|c| c.uuid == *uuid) {
            report.previous = Some(current.summary());
            report.requested = Some(recorded_summary(recorded));
            progress(Stage::Restarting);
            activate(ifname, conn)?;
        }
    }
    progress(Stage::Verifying);
    if let Some(conn) = active {
        let read = read_profile(&conn.uuid)?;
        report.observed = Some(read.summary());
        let timeout = if adapter.connected {
            opts.verify_timeout
        } else {
            Duration::ZERO
        };
        let target = report.requested.clone().unwrap_or_else(|| read.summary());
        let config = DnsConfig {
            ipv4: target.ipv4,
            ipv6: target.ipv6,
            encryption: randommac_core::dns::DnsEncryption::None,
        };
        report.effective = wait_until_in_use(ifname, &config, timeout).0;
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_use_checks_the_first_requested_server() {
        let mut config = DnsConfig::automatic();
        assert!(in_use(&config, &[], &[]));
        config.ipv4 = randommac_core::dns::FamilyDns::Static {
            servers: vec![Ipv4Addr::new(9, 9, 9, 9)],
        };
        assert!(in_use(&config, &[Ipv4Addr::new(9, 9, 9, 9)], &[]));
        assert!(!in_use(&config, &[Ipv4Addr::new(1, 1, 1, 1)], &[]));
        let v6_only = DnsConfig {
            ipv4: randommac_core::dns::FamilyDns::Automatic,
            ipv6: randommac_core::dns::FamilyDns::Static {
                servers: vec!["2620:fe::fe".parse().expect("ip")],
            },
            encryption: randommac_core::dns::DnsEncryption::None,
        };
        assert!(in_use(&v6_only, &[], &["2620:fe::fe".parse().expect("ip")]));
        assert_eq!(family(false), FeatureSupport::no(DnsReason::FamilyDisabled));
    }
}
