//! Runs MAC and DNS operations for the UI, the tray, the scheduler and startup automation: one at
//! a time, off the main thread, with progress events, history refresh and OS notifications.

use std::sync::Arc;
use std::sync::atomic::Ordering;

use randommac_core::dns::{DnsRequest, catalog};
use randommac_core::{ChangeSource, ChangeStatus, GenerateRequest, MacAddress, OsRandom, generate};
use randommac_net::dns_ops::{self, DnsOutcome};
use randommac_net::ops::{self, Outcome};
use randommac_net::{ChangeReport, DnsOptions, DnsReport, Stage, codes};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;

use crate::error::{ApiError, ApiResult};
use crate::i18n::{Key, t};
use crate::state::AppState;
use crate::tray_menu;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpResult {
    pub adapter_id: String,
    pub adapter_name: String,
    pub entry: randommac_core::HistoryEntry,
    pub report: Option<ChangeReport>,
    pub error: Option<ApiError>,
    pub success: bool,
}

/// The result of a DNS change for the UI (`entry` is absent when there was nothing to do).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsOpResult {
    pub adapter_id: String,
    pub adapter_name: String,
    pub entry: Option<randommac_core::HistoryEntry>,
    pub report: Option<DnsReport>,
    pub error: Option<ApiError>,
    pub success: bool,
}

/// The backend's report, or the error the UI gets instead, of a finished change.
fn split_report<R: Clone>(
    report: &Result<R, randommac_net::NetError>,
) -> (Option<R>, Option<ApiError>) {
    match report {
        Ok(r) => (Some(r.clone()), None),
        Err(e) => (None, Some(ApiError::from(e.clone()))),
    }
}

/// What an operation changes, sent with its events so that each page shows only its own steps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum OpKind {
    Mac,
    Dns,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProgressEvent<'a> {
    adapter_id: &'a str,
    stage: Stage,
    kind: OpKind,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct BusyEvent<'a> {
    busy: bool,
    adapter_id: &'a str,
    kind: OpKind,
}

/// Holds the operation lock for one change and announces it: `app:busy` when it starts and,
/// whatever happens, when it ends.
struct OpGuard<'a> {
    app: AppHandle,
    adapter_id: String,
    kind: OpKind,
    _lock: tokio::sync::MutexGuard<'a, ()>,
}

impl Drop for OpGuard<'_> {
    fn drop(&mut self) {
        let _ = self.app.emit(
            "app:busy",
            BusyEvent {
                busy: false,
                adapter_id: &self.adapter_id,
                kind: self.kind,
            },
        );
    }
}

/// The checks every change starts with (read-only mode, quitting), then the lock: waited for
/// (`wait`) or refused with BUSY.
async fn begin<'a>(
    app: &AppHandle,
    state: &'a AppState,
    adapter_id: &str,
    kind: OpKind,
    source: ChangeSource,
    wait: bool,
) -> ApiResult<OpGuard<'a>> {
    if state.read_only {
        return Err(ApiError::read_only());
    }
    // While quitting, only the restore-on-exit changes may run.
    if state.exiting.load(Ordering::SeqCst) && source != ChangeSource::Exit {
        return Err(ApiError::busy());
    }
    let lock = if wait {
        state.op_lock.lock().await
    } else {
        state.op_lock.try_lock().map_err(|_| ApiError::busy())?
    };
    let _ = app.emit(
        "app:busy",
        BusyEvent {
            busy: true,
            adapter_id,
            kind,
        },
    );
    Ok(OpGuard {
        app: app.clone(),
        adapter_id: adapter_id.to_string(),
        kind,
        _lock: lock,
    })
}

/// A progress callback that emits `op:progress`.
fn progress_emitter(app: AppHandle, adapter_id: String, kind: OpKind) -> impl FnMut(Stage) {
    move |stage: Stage| {
        let _ = app.emit(
            "op:progress",
            ProgressEvent {
                adapter_id: &adapter_id,
                stage,
                kind,
            },
        );
    }
}

enum Action {
    Apply {
        mac: Option<MacAddress>,
        ignore_blacklist: bool,
    },
    Restore,
}

fn window_visible(app: &AppHandle) -> bool {
    app.get_webview_window("main")
        .and_then(|w| w.is_visible().ok())
        .unwrap_or(false)
}

pub fn notify(app: &AppHandle, body: String, force: bool) {
    let state = app.state::<AppState>();
    if !state.settings().show_notifications {
        return;
    }
    if !force && window_visible(app) {
        return;
    }
    if let Err(e) = app
        .notification()
        .builder()
        .title("RandomMac")
        .body(body)
        .show()
    {
        tracing::debug!("notification failed: {e}");
    }
}

/// A tray or automation action that could not run, in the UI language with its error code (the
/// backend's own message is English).
pub fn notify_error(app: &AppHandle, error: &ApiError) {
    let lang = app.state::<AppState>().lang();
    let body = if error.code == "BUSY" {
        t(lang, Key::NotifyBusy, &[])
    } else {
        t(lang, Key::NotifyActionFailed, &[&error.code])
    };
    notify(app, body, true);
}

fn notify_outcome(app: &AppHandle, outcome: &Outcome, restore: bool, source: ChangeSource) {
    let lang = app.state::<AppState>().lang();
    let name = outcome.adapter.name.as_str();
    let body = match &outcome.report {
        Ok(r) if r.status == ChangeStatus::RolledBack => t(lang, Key::NotifyRolledBack, &[name]),
        Ok(r) if r.status.is_success() && restore => t(lang, Key::NotifyRestored, &[name]),
        Ok(r) if r.status.is_success() => {
            let mac = r
                .observed_mac
                .or(r.requested_mac)
                .map(|m| m.to_string())
                .unwrap_or_default();
            t(lang, Key::NotifyChanged, &[name, &mac])
        }
        Ok(r) => t(lang, Key::NotifyFailed, &[name, &format!("{:?}", r.status)]),
        Err(e) => t(lang, Key::NotifyFailed, &[name, e.code()]),
    };
    notify(app, body, source != ChangeSource::Ui);
}

/// Runs an operation in its own task, so cancelling the caller (the scheduler being restarted,
/// a closed IPC request) can never stop it half-way: the busy flag, history refresh,
/// restore-on-exit tracking and the operation lock always see the end of the change.
async fn run(
    app: &AppHandle,
    adapter_id: String,
    action: Action,
    source: ChangeSource,
    wait: bool,
) -> ApiResult<OpResult> {
    let app = app.clone();
    tauri::async_runtime::spawn(
        async move { run_inner(&app, adapter_id, action, source, wait).await },
    )
    .await
    .map_err(|e| ApiError::new("INTERNAL", e.to_string()))?
}

async fn run_inner(
    app: &AppHandle,
    adapter_id: String,
    action: Action,
    source: ChangeSource,
    wait: bool,
) -> ApiResult<OpResult> {
    let state = app.state::<AppState>();
    let guard = begin(app, &state, &adapter_id, OpKind::Mac, source, wait).await?;

    let backend = Arc::clone(&state.backend);
    let store = state.store.clone();
    let settings = state.settings();
    let id = adapter_id.clone();
    let mut progress = progress_emitter(app.clone(), adapter_id.clone(), OpKind::Mac);
    let restore = matches!(action, Action::Restore);
    let result = tauri::async_runtime::spawn_blocking(move || match action {
        Action::Restore => ops::restore_mac(
            backend.as_ref(),
            &store,
            &settings,
            &id,
            source,
            &mut progress,
        ),
        Action::Apply {
            mac,
            ignore_blacklist,
        } => {
            let mac = match mac {
                Some(m) => m,
                None => {
                    let adapter = backend.find_adapter(&id)?;
                    let (blacklist, _) = store.load_blacklist();
                    let avoid: Vec<MacAddress> = adapter.current_mac.into_iter().collect();
                    generate(
                        &mut OsRandom,
                        &GenerateRequest {
                            mode: settings.generation_mode.clone(),
                            base: adapter.permanent_mac.or(adapter.current_mac),
                            avoid: &avoid,
                            blacklist: Some(&blacklist),
                            adapter_id: Some(&adapter.id),
                        },
                    )
                    .map_err(|e| {
                        ops::OpError::Net(randommac_net::NetError::InvalidInput(e.to_string()))
                    })?
                }
            };
            ops::apply_mac(
                backend.as_ref(),
                &store,
                &settings,
                &id,
                mac,
                source,
                ignore_blacklist,
                &mut progress,
            )
        }
    })
    .await
    .map_err(|e| ApiError::new("INTERNAL", e.to_string()))?;

    drop(guard);
    let _ = app.emit("history:changed", ());
    let _ = app.emit("adapters:changed", ());

    let outcome = result.map_err(ApiError::from)?;
    let success = outcome.succeeded();
    if success {
        if restore {
            state.unmark_changed(&adapter_id);
        } else {
            state.mark_changed(&adapter_id);
        }
    }
    notify_outcome(app, &outcome, restore, source);
    crate::tray::refresh(app);
    let (report, error) = split_report(&outcome.report);
    Ok(OpResult {
        adapter_id,
        adapter_name: outcome.adapter.name.clone(),
        entry: outcome.entry.clone(),
        report,
        error,
        success,
    })
}

/// Applies `mac` (or a generated address when `None`).
pub async fn apply(
    app: &AppHandle,
    adapter_id: String,
    mac: Option<MacAddress>,
    source: ChangeSource,
    ignore_blacklist: bool,
    wait: bool,
) -> ApiResult<OpResult> {
    run(
        app,
        adapter_id,
        Action::Apply {
            mac,
            ignore_blacklist,
        },
        source,
        wait,
    )
    .await
}

pub async fn restore(
    app: &AppHandle,
    adapter_id: String,
    source: ChangeSource,
    wait: bool,
) -> ApiResult<OpResult> {
    run(app, adapter_id, Action::Restore, source, wait).await
}

/// Restores every adapter RandomMac has something to undo for (`only_session`: only adapters
/// changed since the app started — used by "restore on exit").
pub async fn restore_all(
    app: &AppHandle,
    source: ChangeSource,
    only_session: bool,
) -> ApiResult<Vec<OpResult>> {
    let state = app.state::<AppState>();
    if state.read_only {
        return Err(ApiError::read_only());
    }
    let backend = Arc::clone(&state.backend);
    let store = state.store.clone();
    let session = state.changed_adapters();
    let targets = tauri::async_runtime::spawn_blocking(move || -> ApiResult<Vec<String>> {
        let adapters = backend.list_adapters()?;
        let (app_state, _) = store.load_state();
        Ok(adapters
            .into_iter()
            .filter(|a| {
                session.contains(&a.id)
                    || (!only_session
                        && (a.override_value.is_some() || app_state.original(&a.id).is_some()))
            })
            .map(|a| a.id)
            .collect())
    })
    .await
    .map_err(|e| ApiError::new("INTERNAL", e.to_string()))??;
    let mut results = Vec::new();
    for id in targets {
        match restore(app, id.clone(), source, true).await {
            Ok(r) => results.push(r),
            Err(e) => tracing::warn!(adapter = %id, "restore failed: {e}"),
        }
    }
    Ok(results)
}

// ---------------------------------------------------------------------------------------------
// DNS
// ---------------------------------------------------------------------------------------------

enum DnsAction {
    Apply(DnsRequest),
    Restore,
}

fn notify_dns_outcome(app: &AppHandle, outcome: &DnsOutcome, restore: bool, source: ChangeSource) {
    let lang = app.state::<AppState>().lang();
    let name = outcome.adapter.name.as_str();
    let body = match &outcome.report {
        Ok(r) if r.detail_code.as_deref() == Some(codes::NOTHING_TO_RESTORE) => {
            t(lang, Key::NotifyDnsNothingToRestore, &[name])
        }
        Ok(r) if r.status == ChangeStatus::RolledBack => t(lang, Key::NotifyDnsRolledBack, &[name]),
        Ok(r) if r.status.is_success() && restore => t(lang, Key::NotifyDnsRestored, &[name]),
        Ok(r) if r.status.is_success() => {
            let label = outcome
                .entry
                .as_ref()
                .and_then(|e| e.dns.as_ref())
                .and_then(|d| d.label.as_ref());
            let what = tray_menu::label_text(lang, label, catalog());
            t(lang, Key::NotifyDnsChanged, &[name, &what])
        }
        Ok(r) => t(
            lang,
            Key::NotifyDnsFailed,
            &[name, &format!("{:?}", r.status)],
        ),
        Err(e) => t(lang, Key::NotifyDnsFailed, &[name, e.code()]),
    };
    notify(app, body, source != ChangeSource::Ui);
}

/// Like [`run`] for a DNS change: the same lock and events (`kind: "dns"`), no "restore on
/// exit" tracking (that is for MAC addresses), and `dns:changed` afterwards.
async fn run_dns(
    app: &AppHandle,
    adapter_id: String,
    action: DnsAction,
    source: ChangeSource,
    wait: bool,
) -> ApiResult<DnsOpResult> {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let state = app.state::<AppState>();
        let guard = begin(&app, &state, &adapter_id, OpKind::Dns, source, wait).await?;
        let backend = Arc::clone(&state.backend);
        let store = state.store.clone();
        let settings = state.settings();
        let id = adapter_id.clone();
        let mut progress = progress_emitter(app.clone(), adapter_id.clone(), OpKind::Dns);
        let restore = matches!(action, DnsAction::Restore);
        let result = tauri::async_runtime::spawn_blocking(move || {
            let opts = DnsOptions::default();
            match action {
                DnsAction::Apply(request) => dns_ops::apply_dns(
                    backend.as_ref(),
                    &store,
                    &settings,
                    &id,
                    &request,
                    source,
                    &opts,
                    &mut progress,
                ),
                DnsAction::Restore => dns_ops::restore_dns(
                    backend.as_ref(),
                    &store,
                    &settings,
                    &id,
                    source,
                    &opts,
                    &mut progress,
                ),
            }
        })
        .await
        .map_err(|e| ApiError::new("INTERNAL", e.to_string()))?;
        drop(guard);
        let _ = app.emit("history:changed", ());
        let _ = app.emit("dns:changed", &adapter_id);

        let outcome = result.map_err(ApiError::from)?;
        let success = outcome.report.as_ref().is_ok_and(|r| r.status.is_success());
        notify_dns_outcome(&app, &outcome, restore, source);
        let (report, error) = split_report(&outcome.report);
        Ok(DnsOpResult {
            adapter_id,
            adapter_name: outcome.adapter.name.clone(),
            entry: outcome.entry.clone(),
            report,
            error,
            success,
        })
    })
    .await
    .map_err(|e| ApiError::new("INTERNAL", e.to_string()))?
}

pub async fn dns_apply(
    app: &AppHandle,
    adapter_id: String,
    request: DnsRequest,
    source: ChangeSource,
    wait: bool,
) -> ApiResult<DnsOpResult> {
    run_dns(app, adapter_id, DnsAction::Apply(request), source, wait).await
}

pub async fn dns_restore(
    app: &AppHandle,
    adapter_id: String,
    source: ChangeSource,
    wait: bool,
) -> ApiResult<DnsOpResult> {
    run_dns(app, adapter_id, DnsAction::Restore, source, wait).await
}

/// Applies a request to every connected physical adapter that can change DNS, one after the
/// other; an adapter that fails does not stop the others.
pub async fn dns_apply_all(
    app: &AppHandle,
    request: DnsRequest,
    source: ChangeSource,
) -> ApiResult<Vec<DnsOpResult>> {
    let state = app.state::<AppState>();
    if state.read_only {
        return Err(ApiError::read_only());
    }
    let backend = Arc::clone(&state.backend);
    let targets =
        tauri::async_runtime::spawn_blocking(move || dns_ops::dns_targets(backend.as_ref()))
            .await
            .map_err(|e| ApiError::new("INTERNAL", e.to_string()))??;
    if targets.is_empty() {
        return Err(ApiError::new(
            "ADAPTER_NOT_FOUND",
            "no connected adapter can change its DNS servers",
        ));
    }
    let mut results = Vec::new();
    for adapter in targets {
        match dns_apply(app, adapter.id.clone(), request.clone(), source, true).await {
            Ok(r) => results.push(r),
            Err(e) => results.push(DnsOpResult {
                adapter_id: adapter.id,
                adapter_name: adapter.name,
                entry: None,
                report: None,
                error: Some(e),
                success: false,
            }),
        }
    }
    Ok(results)
}

/// Randomizes the configured adapters after an autostart launch.
pub async fn startup_automation(app: &AppHandle) {
    let state = app.state::<AppState>();
    let settings = state.settings();
    if !state.args.autostart || !settings.automation.change_on_startup || state.read_only {
        return;
    }
    // Give the network stack a moment after sign-in.
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    let mut ok = 0usize;
    let mut failed = 0usize;
    for id in settings.automation.startup_adapter_ids.clone() {
        match apply(app, id.clone(), None, ChangeSource::Startup, false, true).await {
            Ok(r) if r.success => ok += 1,
            Ok(_) => failed += 1,
            Err(e) => {
                failed += 1;
                tracing::warn!(adapter = %id, "startup change failed: {e}");
            }
        }
    }
    if ok + failed > 0 {
        let lang = state.lang();
        notify(
            app,
            t(
                lang,
                Key::NotifyStartupSummary,
                &[&ok.to_string(), &failed.to_string()],
            ),
            true,
        );
    }
}
