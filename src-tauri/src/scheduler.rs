//! Periodic randomization while the app is running.

use std::time::Duration;

use randommac_core::ChangeSource;
use tauri::{AppHandle, Manager};

use crate::ops_runner;
use crate::state::AppState;

/// (Re)starts the periodic task according to the current settings.
pub fn restart(app: &AppHandle) {
    let state = app.state::<AppState>();
    if let Ok(mut slot) = state.scheduler.lock()
        && let Some(handle) = slot.take()
    {
        handle.abort();
    }
    let settings = state.settings();
    let auto = &settings.automation;
    if !auto.interval_enabled || auto.interval_adapter_ids.is_empty() || state.read_only {
        return;
    }
    let period = Duration::from_secs(u64::from(auto.interval_minutes) * 60);
    let app2 = app.clone();
    let handle = tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(period).await;
            let ids = app2
                .state::<AppState>()
                .settings()
                .automation
                .interval_adapter_ids;
            for id in ids {
                if let Err(e) =
                    ops_runner::apply(&app2, id.clone(), None, ChangeSource::Schedule, false, true)
                        .await
                {
                    tracing::warn!(adapter = %id, "scheduled change failed: {e}");
                }
            }
        }
    });
    if let Ok(mut slot) = state.scheduler.lock() {
        *slot = Some(handle);
    }
    tracing::info!(
        minutes = auto.interval_minutes,
        "periodic randomization enabled"
    );
}
