//! Runs MAC operations for the UI, the tray, the scheduler and startup automation: one at a
//! time, off the main thread, with progress events, history refresh and OS notifications.

use std::sync::Arc;
use std::sync::atomic::Ordering;

use randommac_core::{ChangeSource, ChangeStatus, GenerateRequest, MacAddress, OsRandom, generate};
use randommac_net::ops::{self, Outcome};
use randommac_net::{ChangeReport, Stage};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;

use crate::error::{ApiError, ApiResult};
use crate::i18n::{Key, t};
use crate::state::AppState;

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

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProgressEvent<'a> {
    adapter_id: &'a str,
    stage: Stage,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct BusyEvent<'a> {
    busy: bool,
    adapter_id: &'a str,
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
    if state.read_only {
        return Err(ApiError::read_only());
    }
    // While quitting, only the restore-on-exit changes may run.
    if state.exiting.load(Ordering::SeqCst) && source != ChangeSource::Exit {
        return Err(ApiError::busy());
    }
    let _guard = if wait {
        state.op_lock.lock().await
    } else {
        state.op_lock.try_lock().map_err(|_| ApiError::busy())?
    };
    let _ = app.emit(
        "app:busy",
        BusyEvent {
            busy: true,
            adapter_id: &adapter_id,
        },
    );

    let backend = Arc::clone(&state.backend);
    let store = state.store.clone();
    let settings = state.settings();
    let emitter = app.clone();
    let id = adapter_id.clone();
    let restore = matches!(action, Action::Restore);
    let result = tauri::async_runtime::spawn_blocking(move || {
        let mut progress = |stage: Stage| {
            let _ = emitter.emit(
                "op:progress",
                ProgressEvent {
                    adapter_id: &id,
                    stage,
                },
            );
        };
        match action {
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
        }
    })
    .await
    .map_err(|e| ApiError::new("INTERNAL", e.to_string()))?;

    let _ = app.emit(
        "app:busy",
        BusyEvent {
            busy: false,
            adapter_id: &adapter_id,
        },
    );
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
    Ok(OpResult {
        adapter_id,
        adapter_name: outcome.adapter.name.clone(),
        entry: outcome.entry.clone(),
        report: outcome.report.as_ref().ok().cloned(),
        error: outcome
            .report
            .as_ref()
            .err()
            .map(|e| ApiError::from(e.clone())),
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
