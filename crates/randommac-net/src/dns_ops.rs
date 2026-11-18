//! DNS changes for every front end — the DNS counterpart of [`crate::ops`]: validation, the
//! snapshot "Restore original DNS" needs (`dns-state.json`), the change itself, and the history
//! entry. All of it runs under the cross-process operation lock.

use randommac_core::dns::{
    self, Catalog, CustomDnsPreset, DnsLabel, DnsRequest, DnsSummary, FamilyDns,
};
use randommac_core::random::new_id;
use randommac_core::{
    ChangeSource, ChangeStatus, DnsHistory, HistoryAction, HistoryEntry, HistoryKind, Settings,
    timefmt,
};
use serde::Serialize;

use crate::ops::{DataStore, OpError};
use crate::{
    Adapter, DnsCapabilities, DnsMethod, DnsOptions, DnsReason, DnsReport, DnsStatus, NetError,
    NetworkBackend, Stage, codes,
};

/// What a front end shows for one adapter.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsView {
    pub capabilities: DnsCapabilities,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<DnsStatus>,
    /// Why the status could not be read (`error.<CODE>`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_error_code: Option<String>,
    /// Settings from before RandomMac's first DNS change are recorded.
    pub restorable: bool,
    /// The provider variant, preset or local resolver the configured servers belong to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matched: Option<DnsLabel>,
}

/// The result of a DNS change that reached the backend.
#[derive(Debug, Clone)]
pub struct DnsOutcome {
    pub adapter: Adapter,
    pub report: Result<DnsReport, NetError>,
    /// `None` when nothing was done (nothing to restore).
    pub entry: Option<HistoryEntry>,
}

impl DnsOutcome {
    pub fn succeeded(&self) -> bool {
        self.report.as_ref().is_ok_and(|r| r.status.is_success())
    }
}

fn same<A: PartialEq + Copy>(family: &FamilyDns<A>, list: &[A]) -> bool {
    let servers = family.servers();
    servers.len() == list.len() && servers.iter().all(|s| list.contains(s))
}

/// Which catalog variant, preset or the local resolver has exactly the configured servers (IPv6
/// may be automatic when only IPv4 was set).
pub fn match_label(
    summary: &DnsSummary,
    catalog: &Catalog,
    presets: &[CustomDnsPreset],
) -> Option<DnsLabel> {
    if summary.ipv4.is_automatic() && summary.ipv6.is_automatic() {
        return Some(DnsLabel::Automatic);
    }
    let fits = |v4: &[std::net::Ipv4Addr], v6: &[std::net::Ipv6Addr]| {
        same(&summary.ipv4, v4) && (summary.ipv6.is_automatic() || same(&summary.ipv6, v6))
    };
    if fits(&[dns::LOCAL_RESOLVER_V4], &[dns::LOCAL_RESOLVER_V6]) {
        return Some(DnsLabel::LocalResolver);
    }
    for p in &catalog.providers {
        if let Some(v) = p.variants.iter().find(|v| fits(&v.ipv4, &v.ipv6)) {
            return Some(DnsLabel::Provider {
                provider: p.id.clone(),
                variant: v.id.clone(),
            });
        }
    }
    if let Some(preset) = presets.iter().find(|p| fits(&p.ipv4, &p.ipv6)) {
        return Some(DnsLabel::Preset {
            name: preset.name.clone(),
        });
    }
    Some(DnsLabel::Custom)
}

/// The DNS page's data for one adapter. Takes no lock: it only reads.
pub fn dns_view(
    backend: &dyn NetworkBackend,
    store: &DataStore,
    settings: &Settings,
    adapter_id: &str,
) -> Result<DnsView, OpError> {
    let adapter = backend.find_adapter(adapter_id)?;
    let capabilities = backend.dns_capabilities(&adapter);
    let (status, status_error_code) = if capabilities.supported {
        match backend.dns_status(&adapter) {
            Ok(s) => (Some(s), None),
            Err(e) => (None, Some(e.code().to_string())),
        }
    } else {
        (None, None)
    };
    let (state, _) = store.load_dns_state();
    let matched = status
        .as_ref()
        .and_then(|s| match_label(&s.configured, dns::catalog(), &settings.dns.presets));
    Ok(DnsView {
        capabilities,
        status,
        status_error_code,
        restorable: state.original(&adapter.id).is_some(),
        matched,
    })
}

/// The error for an adapter whose DNS settings cannot be changed now.
fn cannot_change(caps: &DnsCapabilities) -> NetError {
    match caps.change_reason.or(caps.reason) {
        Some(DnsReason::NotElevated) => NetError::NotElevated,
        Some(DnsReason::NmUnavailable) => {
            NetError::NmUnavailable("NetworkManager is not running".into())
        }
        Some(DnsReason::NoActiveConnection) => NetError::NmNoActiveConnection,
        Some(reason) => NetError::Unsupported(format!(
            "DNS settings cannot be changed on this adapter ({})",
            reason.code()
        )),
        None => NetError::Unsupported("DNS settings cannot be changed on this adapter".into()),
    }
}

fn dns_entry(
    adapter: &Adapter,
    action: HistoryAction,
    source: ChangeSource,
    requested: Option<DnsSummary>,
    label: Option<DnsLabel>,
    report: &Result<DnsReport, NetError>,
) -> HistoryEntry {
    let (status, method, error_code, error, dns) = match report {
        Ok(r) => (
            r.status,
            Some(r.method.as_str().to_string()),
            r.detail_code.clone(),
            None,
            DnsHistory {
                previous: r.previous.clone(),
                requested: requested.or_else(|| r.requested.clone()),
                observed: r.observed.clone(),
                label,
                warnings: r.warnings.clone(),
            },
        ),
        Err(e) => (
            ChangeStatus::Failed,
            None,
            Some(e.code().to_string()),
            Some(e.to_string()),
            DnsHistory {
                requested,
                label,
                ..DnsHistory::default()
            },
        ),
    };
    HistoryEntry {
        id: new_id(),
        timestamp: timefmt::now(),
        adapter_id: adapter.id.clone(),
        adapter_name: adapter.name.clone(),
        action,
        source,
        status,
        previous_mac: None,
        requested_mac: None,
        new_mac: None,
        method,
        error_code,
        error,
        kind: HistoryKind::Dns,
        dns: Some(dns),
    }
}

/// Applies a DNS request to one adapter.
///
/// The settings from before RandomMac's first DNS change are recorded first; when the change does
/// not happen they are forgotten again, and when the adapter ends up back on them (say
/// "Automatic" on an adapter that was automatic) there is nothing left to restore.
#[allow(clippy::too_many_arguments)]
pub fn apply_dns(
    backend: &dyn NetworkBackend,
    store: &DataStore,
    settings: &Settings,
    adapter_id: &str,
    request: &DnsRequest,
    source: ChangeSource,
    opts: &DnsOptions,
    progress: &mut dyn FnMut(Stage),
) -> Result<DnsOutcome, OpError> {
    let _lock = store.lock()?;
    let adapter = backend.find_adapter(adapter_id)?;
    let caps = backend.dns_capabilities(&adapter);
    if !caps.supported || !caps.can_change {
        return Err(cannot_change(&caps).into());
    }
    let resolved = dns::resolve_request(
        request,
        dns::catalog(),
        &settings.dns.presets,
        caps.encryption(),
    )?;
    let snapshot = backend.dns_snapshot(&adapter)?;
    let (mut state, _) = store.load_dns_state();
    let remembered = state.remember(&adapter.id, &snapshot, timefmt::now());
    store.save_dns_state(&state)?;

    let report = backend.apply_dns(&adapter, &resolved.config, &snapshot, opts, progress);
    let unchanged = match &report {
        Ok(r) => r.status == ChangeStatus::RolledBack,
        Err(NetError::RollbackFailed(_)) => false,
        Err(_) => backend
            .dns_snapshot(&adapter)
            .is_ok_and(|now| now == snapshot),
    };
    if unchanged {
        state.forget(&adapter.id, &remembered);
    } else if report.as_ref().is_ok_and(|r| r.status.is_success())
        && let Ok(now) = backend.dns_snapshot(&adapter)
    {
        state.clear_if_original(&adapter.id, &now);
    }
    store.save_dns_state(&state)?;

    let entry = dns_entry(
        &adapter,
        HistoryAction::Apply,
        source,
        Some(resolved.config.summary()),
        Some(resolved.label),
        &report,
    );
    store.append_history(entry.clone(), settings.history.policy())?;
    Ok(DnsOutcome {
        adapter,
        report,
        entry: Some(entry),
    })
}

/// Puts back the DNS settings recorded before RandomMac's first DNS change on the adapter.
pub fn restore_dns(
    backend: &dyn NetworkBackend,
    store: &DataStore,
    settings: &Settings,
    adapter_id: &str,
    source: ChangeSource,
    opts: &DnsOptions,
    progress: &mut dyn FnMut(Stage),
) -> Result<DnsOutcome, OpError> {
    let _lock = store.lock()?;
    let adapter = backend.find_adapter(adapter_id)?;
    let (mut state, _) = store.load_dns_state();
    let Some(original) = state.original(&adapter.id).cloned() else {
        let caps = backend.dns_capabilities(&adapter);
        let current = backend.dns_status(&adapter).ok().map(|s| s.configured);
        let report = DnsReport {
            method: caps.method.unwrap_or(DnsMethod::Mock),
            status: ChangeStatus::Applied,
            previous: current.clone(),
            requested: None,
            observed: current,
            effective: Vec::new(),
            detail_code: Some(codes::NOTHING_TO_RESTORE.into()),
            warnings: Vec::new(),
            cache_flushed: false,
            nm_connection_uuid: None,
        };
        return Ok(DnsOutcome {
            adapter,
            report: Ok(report),
            entry: None,
        });
    };
    let caps = backend.dns_capabilities(&adapter);
    if !caps.supported || !caps.can_change {
        return Err(cannot_change(&caps).into());
    }
    let report = backend.restore_dns(&adapter, &original, opts, progress);
    if report.as_ref().is_ok_and(|r| r.status.is_success()) {
        state.clear(&adapter.id);
        store.save_dns_state(&state)?;
    }
    let entry = dns_entry(
        &adapter,
        HistoryAction::Restore,
        source,
        None,
        None,
        &report,
    );
    store.append_history(entry.clone(), settings.history.policy())?;
    Ok(DnsOutcome {
        adapter,
        report,
        entry: Some(entry),
    })
}

/// The adapters "every connected adapter" means: physical, connected, and able to change DNS.
pub fn dns_targets(backend: &dyn NetworkBackend) -> Result<Vec<Adapter>, NetError> {
    Ok(backend
        .list_adapters()?
        .into_iter()
        .filter(|a| a.connected && !a.is_virtual)
        .filter(|a| {
            let caps = backend.dns_capabilities(a);
            caps.supported && caps.can_change
        })
        .collect())
}

/// Applies a request to every adapter [`dns_targets`] returns, one after the other; each has its
/// own result. `progress` gets the adapter id with each stage.
#[allow(clippy::type_complexity)]
pub fn apply_dns_all(
    backend: &dyn NetworkBackend,
    store: &DataStore,
    settings: &Settings,
    request: &DnsRequest,
    source: ChangeSource,
    opts: &DnsOptions,
    progress: &mut dyn FnMut(&str, Stage),
) -> Result<Vec<(Adapter, Result<DnsOutcome, OpError>)>, OpError> {
    let targets = dns_targets(backend)?;
    Ok(targets
        .into_iter()
        .map(|adapter| {
            let id = adapter.id.clone();
            let result = apply_dns(
                backend,
                store,
                settings,
                &id,
                request,
                source,
                opts,
                &mut |s| progress(&id, s),
            );
            (adapter, result)
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;

    use randommac_core::AppPaths;
    use randommac_core::dns::{DnsEndpoints, DnsTarget, EncryptionChoice};

    use super::*;
    use crate::mock::{MockBackend, MockScenario};

    fn store() -> (tempfile::TempDir, DataStore) {
        let dir = tempfile::tempdir().expect("tmp");
        let store = DataStore::new(AppPaths::under(dir.path()));
        (dir, store)
    }

    fn quad9() -> DnsRequest {
        DnsRequest {
            target: DnsTarget::Provider {
                provider: "quad9".into(),
                variant: None,
            },
            encryption: EncryptionChoice::Doh,
            allow_fallback: false,
        }
    }

    fn opts() -> DnsOptions {
        DnsOptions {
            verify_timeout: std::time::Duration::ZERO,
            flush_cache: false,
        }
    }

    #[test]
    fn apply_records_history_and_snapshot_then_restore_clears_it() {
        let (_dir, store) = store();
        let backend = MockBackend::new(MockScenario::Normal);
        let settings = Settings::default();
        let mut stages = Vec::new();
        let outcome = apply_dns(
            &backend,
            &store,
            &settings,
            "mock-ethernet",
            &quad9(),
            ChangeSource::Cli,
            &opts(),
            &mut |s| stages.push(s),
        )
        .expect("apply");
        assert!(outcome.succeeded());
        assert_eq!(stages.first(), Some(&Stage::Preparing));
        let entry = outcome.entry.expect("entry");
        assert_eq!(entry.kind, HistoryKind::Dns);
        assert_eq!(entry.method.as_deref(), Some("mock"));
        let dns = entry.dns.expect("dns part");
        assert_eq!(
            dns.label,
            Some(DnsLabel::Provider {
                provider: "quad9".into(),
                variant: "recommended".into()
            })
        );
        assert_eq!(
            dns.requested.expect("requested").encryption,
            dns::EncryptionMode::Doh
        );

        let view = dns_view(&backend, &store, &settings, "mock-ethernet").expect("view");
        assert!(view.restorable);
        assert_eq!(
            view.matched,
            Some(DnsLabel::Provider {
                provider: "quad9".into(),
                variant: "recommended".into()
            })
        );
        assert_eq!(
            view.status.expect("status").doh_template.as_deref(),
            Some("https://dns.quad9.net/dns-query")
        );

        let restored = restore_dns(
            &backend,
            &store,
            &settings,
            "mock-ethernet",
            ChangeSource::Cli,
            &opts(),
            &mut |_| {},
        )
        .expect("restore");
        assert!(restored.succeeded());
        assert_eq!(
            restored.entry.expect("entry").action,
            HistoryAction::Restore
        );
        let view = dns_view(&backend, &store, &settings, "mock-ethernet").expect("view");
        assert!(!view.restorable);
        assert_eq!(view.matched, Some(DnsLabel::Automatic));
        let (history, _) = store.load_history();
        assert_eq!(history.entries.len(), 2);

        // Nothing recorded: a no-op without a history entry.
        let again = restore_dns(
            &backend,
            &store,
            &settings,
            "mock-ethernet",
            ChangeSource::Cli,
            &opts(),
            &mut |_| {},
        )
        .expect("noop");
        assert!(again.entry.is_none());
        assert_eq!(
            again.report.expect("report").detail_code.as_deref(),
            Some(codes::NOTHING_TO_RESTORE)
        );
    }

    #[test]
    fn automatic_on_an_automatic_adapter_leaves_nothing_to_restore() {
        let (_dir, store) = store();
        let backend = MockBackend::new(MockScenario::Normal);
        let settings = Settings::default();
        let auto = DnsRequest {
            target: DnsTarget::Automatic,
            encryption: EncryptionChoice::None,
            allow_fallback: false,
        };
        let outcome = apply_dns(
            &backend,
            &store,
            &settings,
            "mock-wifi",
            &auto,
            ChangeSource::Ui,
            &opts(),
            &mut |_| {},
        )
        .expect("apply");
        assert!(outcome.succeeded());
        assert!(store.load_dns_state().0.original("mock-wifi").is_none());
    }

    #[test]
    fn rolled_back_changes_forget_the_snapshot() {
        let (_dir, store) = store();
        let backend = MockBackend::new(MockScenario::DriverIgnores);
        let settings = Settings::default();
        let outcome = apply_dns(
            &backend,
            &store,
            &settings,
            "mock-ethernet",
            &quad9(),
            ChangeSource::Ui,
            &opts(),
            &mut |_| {},
        )
        .expect("apply");
        assert!(!outcome.succeeded());
        let report = outcome.report.expect("report");
        assert_eq!(report.status, ChangeStatus::RolledBack);
        assert_eq!(report.detail_code.as_deref(), Some(codes::DNS_NOT_APPLIED));
        assert!(store.load_dns_state().0.original("mock-ethernet").is_none());
        assert_eq!(
            outcome.entry.expect("entry").status,
            ChangeStatus::RolledBack
        );
    }

    #[test]
    fn refused_requests_and_adapters() {
        let (_dir, store) = store();
        let backend = MockBackend::new(MockScenario::Normal);
        let settings = Settings::default();
        let bad = DnsRequest {
            target: DnsTarget::Custom(DnsEndpoints {
                ipv4: vec![Ipv4Addr::BROADCAST],
                ..Default::default()
            }),
            encryption: EncryptionChoice::None,
            allow_fallback: false,
        };
        let err = apply_dns(
            &backend,
            &store,
            &settings,
            "mock-ethernet",
            &bad,
            ChangeSource::Cli,
            &opts(),
            &mut |_| {},
        )
        .expect_err("invalid");
        assert_eq!(err.code(), "INVALID_DNS");
        // The disabled adapter (mock-usb) cannot change.
        let err = apply_dns(
            &backend,
            &store,
            &settings,
            "mock-usb",
            &quad9(),
            ChangeSource::Cli,
            &opts(),
            &mut |_| {},
        )
        .expect_err("disabled");
        assert_eq!(err.code(), "UNSUPPORTED");
        let err = apply_dns(
            &backend,
            &store,
            &settings,
            "nope",
            &quad9(),
            ChangeSource::Cli,
            &opts(),
            &mut |_| {},
        )
        .expect_err("missing");
        assert_eq!(err.code(), "ADAPTER_NOT_FOUND");
        // Nothing was recorded or logged.
        assert!(store.load_history().0.entries.is_empty());
        assert!(store.load_dns_state().0.originals.is_empty());
    }

    #[test]
    fn all_connected_physical_adapters() {
        let (_dir, store) = store();
        let backend = MockBackend::new(MockScenario::Normal);
        let settings = Settings::default();
        let targets: Vec<String> = dns_targets(&backend)
            .expect("targets")
            .into_iter()
            .map(|a| a.id)
            .collect();
        assert_eq!(targets, ["mock-ethernet", "mock-wifi"]);
        let mut seen = Vec::new();
        let results = apply_dns_all(
            &backend,
            &store,
            &settings,
            &quad9(),
            ChangeSource::Ui,
            &opts(),
            &mut |id, s| {
                seen.push((id.to_string(), s));
            },
        )
        .expect("all");
        assert_eq!(results.len(), 2);
        assert!(
            results
                .iter()
                .all(|(_, r)| r.as_ref().is_ok_and(DnsOutcome::succeeded))
        );
        assert!(seen.iter().any(|(id, _)| id == "mock-wifi"));
    }

    #[test]
    fn matching_labels() {
        let c = dns::catalog();
        let summary = |v4: &[&str], v6: &[&str]| DnsSummary {
            ipv4: FamilyDns::from_servers(
                &v4.iter()
                    .map(|s| s.parse().expect("v4"))
                    .collect::<Vec<_>>(),
            ),
            ipv6: FamilyDns::from_servers(
                &v6.iter()
                    .map(|s| s.parse().expect("v6"))
                    .collect::<Vec<_>>(),
            ),
            encryption: dns::EncryptionMode::None,
        };
        assert_eq!(
            match_label(&summary(&[], &[]), c, &[]),
            Some(DnsLabel::Automatic)
        );
        assert_eq!(
            match_label(&summary(&["127.0.0.1"], &[]), c, &[]),
            Some(DnsLabel::LocalResolver)
        );
        assert_eq!(
            match_label(&summary(&["1.0.0.1", "1.1.1.1"], &[]), c, &[]),
            Some(DnsLabel::Provider {
                provider: "cloudflare".into(),
                variant: "standard".into()
            })
        );
        let preset = CustomDnsPreset {
            id: "0123456789abcdef".into(),
            name: "Home".into(),
            ipv4: vec!["192.168.1.1".parse().expect("v4")],
            ipv6: vec![],
            doh_template: None,
            dot_host: None,
        };
        assert_eq!(
            match_label(&summary(&["192.168.1.1"], &[]), c, &[preset]),
            Some(DnsLabel::Preset {
                name: "Home".into()
            })
        );
        assert_eq!(
            match_label(&summary(&["192.0.2.53"], &[]), c, &[]),
            Some(DnsLabel::Custom)
        );
        assert_eq!(
            cannot_change(&DnsCapabilities::unsupported(DnsReason::NotElevated)).code(),
            "NOT_ELEVATED"
        );
        assert_eq!(
            cannot_change(&DnsCapabilities::unsupported(DnsReason::NmUnavailable)).code(),
            "NM_UNAVAILABLE"
        );
        assert_eq!(
            cannot_change(&DnsCapabilities::unsupported(DnsReason::NoActiveConnection)).code(),
            "NM_NO_ACTIVE_CONNECTION"
        );
    }
}
