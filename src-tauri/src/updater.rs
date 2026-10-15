//! Update checks and installation (signed `latest.json` on GitHub Releases).
//!
//! NSIS/MSI/AppImage updates use the plugin's installer. For .deb/.rpm the package is installed
//! with `pkexec apt-get install` / `pkexec dnf install` so new dependencies are resolved and a
//! cancelled authentication never falls back to password prompts.

use std::time::Duration;

use chrono::Utc;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::{Update, UpdaterExt};

use crate::error::{ApiError, ApiResult};
use crate::i18n::{Key, t};
use crate::state::AppState;

#[derive(Default)]
pub struct UpdaterState {
    pub pending: Option<Update>,
    pub installing: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub current_version: String,
    pub available: bool,
    pub version: Option<String>,
    pub date: Option<String>,
    pub notes: Option<String>,
    pub checked_at: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Progress {
    downloaded: u64,
    total: Option<u64>,
}

pub async fn check(app: &AppHandle) -> ApiResult<UpdateInfo> {
    let state = app.state::<AppState>();
    let update = app.updater()?.check().await?;
    let now = Utc::now();
    if let Ok(mut s) = state.settings.lock() {
        s.last_update_check = Some(now);
        let _ = state.store.save_settings(&s);
    }
    let info = UpdateInfo {
        current_version: app.package_info().version.to_string(),
        available: update.is_some(),
        version: update.as_ref().map(|u| u.version.clone()),
        date: update.as_ref().and_then(|u| u.date.map(|d| d.to_string())),
        notes: update.as_ref().and_then(|u| u.body.clone()),
        checked_at: now.to_rfc3339(),
    };
    if let Ok(mut s) = state.updater.lock() {
        s.pending = update;
    }
    Ok(info)
}

#[cfg(target_os = "linux")]
fn install_linux_package(bytes: &[u8], kind: &str) -> ApiResult<()> {
    use std::os::unix::fs::PermissionsExt;

    use randommac_net::process;

    let base = std::env::var_os("XDG_RUNTIME_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let dir = base.join(format!(
        "randommac-update-{}",
        randommac_core::random::new_id()
    ));
    std::fs::create_dir(&dir)?;
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))?;
    let file = dir.join(format!("randommac-update.{kind}"));
    std::fs::write(&file, bytes)?;
    let file_str = file.to_string_lossy().into_owned();
    let (manager, args): (&[&str], Vec<&str>) = if kind == "deb" {
        (
            &["/usr/bin/apt-get", "/bin/apt-get"],
            vec!["install", "-y", &file_str],
        )
    } else {
        (
            &["/usr/bin/dnf", "/bin/dnf", "/usr/bin/dnf5"],
            vec!["install", "-y", &file_str],
        )
    };
    let manager = process::find_program(manager)
        .ok_or_else(|| ApiError::new("UPDATE_FAILED", "no package manager found"))?;
    let pkexec = process::find_program(&["/usr/bin/pkexec", "/bin/pkexec"])
        .ok_or_else(|| ApiError::new("AUTH_UNAVAILABLE", "pkexec is not installed"))?;
    let manager_str = manager.to_string_lossy().into_owned();
    let mut full: Vec<&str> = vec![&manager_str];
    full.extend(args);
    let out = process::run(&pkexec, &full, None, Duration::from_secs(900));
    let _ = std::fs::remove_dir_all(&dir);
    let out = out?;
    if let Some(e) = randommac_net::linux_parse::pkexec_error(out.code, &out.stderr) {
        return Err(e.into());
    }
    if !out.success() {
        return Err(ApiError::new(
            "UPDATE_FAILED",
            out.stderr.trim().to_string(),
        ));
    }
    Ok(())
}

/// Downloads and installs the pending update, then restarts the app.
pub async fn install(app: &AppHandle) -> ApiResult<()> {
    let state = app.state::<AppState>();
    let update = {
        let mut s = state
            .updater
            .lock()
            .map_err(|_| ApiError::new("INTERNAL", "updater state poisoned"))?;
        if s.installing {
            return Err(ApiError::busy());
        }
        s.installing = true;
        s.pending.clone()
    };
    let result = async {
        let update = update.ok_or_else(|| ApiError::new("NO_UPDATE", "no update is pending"))?;
        let emitter = app.clone();
        let mut downloaded = 0u64;
        let bytes = update
            .download(
                move |chunk, total| {
                    downloaded += chunk as u64;
                    let _ = emitter.emit("update:progress", Progress { downloaded, total });
                },
                || {},
            )
            .await?;
        let _ = app.emit("update:installing", ());
        let kind = tauri::utils::platform::bundle_type();
        #[cfg(target_os = "linux")]
        {
            use tauri::utils::config::BundleType;
            match kind {
                Some(BundleType::Deb) => {
                    let b = bytes.clone();
                    tauri::async_runtime::spawn_blocking(move || install_linux_package(&b, "deb"))
                        .await
                        .map_err(|e| ApiError::new("INTERNAL", e.to_string()))??;
                }
                Some(BundleType::Rpm) => {
                    let b = bytes.clone();
                    tauri::async_runtime::spawn_blocking(move || install_linux_package(&b, "rpm"))
                        .await
                        .map_err(|e| ApiError::new("INTERNAL", e.to_string()))??;
                }
                _ => update.install(&bytes)?,
            }
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = kind;
            update.install(&bytes)?;
        }
        Ok::<(), ApiError>(())
    }
    .await;
    if let Ok(mut s) = state.updater.lock() {
        s.installing = false;
    }
    result?;
    app.restart();
}

/// Background check at startup (at most once a day, if enabled).
pub fn spawn_startup_check(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let state = app.state::<AppState>();
        let settings = state.settings();
        if !settings.auto_check_updates || state.mock {
            return;
        }
        let due = settings
            .last_update_check
            .is_none_or(|t| Utc::now() - t > chrono::Duration::hours(24));
        if !due {
            return;
        }
        tokio::time::sleep(Duration::from_secs(10)).await;
        match check(&app).await {
            Ok(info) if info.available => {
                let version = info.version.clone().unwrap_or_default();
                let _ = app.emit("update:available", info);
                let lang = state.lang();
                crate::ops_runner::notify(&app, t(lang, Key::NotifyUpdate, &[&version]), true);
            }
            Ok(_) => tracing::info!("RandomMac is up to date"),
            Err(e) => tracing::info!("update check failed: {e}"),
        }
    });
}
