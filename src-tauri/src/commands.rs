//! Tauri commands called by the UI. All of them are listed in build.rs and granted in
//! capabilities/main.json; arguments are validated here, never trusted.

use std::io::Write as _;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use chrono::Utc;
use randommac_core::blacklist::{BlacklistEntry, MacPattern};
use randommac_core::{
    ChangeSource, GenerateRequest, GenerationMode, HistoryEntry, MacAddress, OsRandom, Settings,
    generate, history, oui,
};
use randommac_net::{Adapter, Capabilities, ops};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;

use crate::error::{ApiError, ApiResult};
use crate::i18n::{Key, t};
use crate::logging::LogEntry;
use crate::ops_runner::{self, OpResult};
use crate::state::{AppState, StartupReport};
use crate::updater::{self, UpdateInfo};
use crate::{autostart, links, platform, scheduler, tray};

async fn blocking<T, F>(f: F) -> ApiResult<T>
where
    T: Send + 'static,
    F: FnOnce() -> ApiResult<T> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| ApiError::new("INTERNAL", e.to_string()))?
}

// ---------------------------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    version: String,
    os: &'static str,
    arch: &'static str,
    os_version: String,
    elevated: bool,
    read_only: bool,
    mock: bool,
    package_kind: Option<String>,
    tray_available: bool,
    capabilities: Capabilities,
    data_dir: String,
    log_dir: String,
    oui_source_date: &'static str,
    autostart_enabled: bool,
    startup: StartupReport,
    launched_minimized: bool,
}

fn os_version() -> String {
    #[cfg(target_os = "linux")]
    {
        std::fs::read_to_string("/etc/os-release")
            .ok()
            .and_then(|t| {
                t.lines()
                    .find_map(|l| l.strip_prefix("PRETTY_NAME="))
                    .map(|v| v.trim_matches('"').to_string())
            })
            .unwrap_or_else(|| "Linux".into())
    }
    #[cfg(not(target_os = "linux"))]
    {
        std::env::consts::OS.to_string()
    }
}

#[tauri::command]
pub async fn app_info(app: AppHandle, state: State<'_, AppState>) -> ApiResult<AppInfo> {
    let backend = Arc::clone(&state.backend);
    let capabilities = blocking(move || Ok(backend.capabilities())).await?;
    let autostart_enabled = blocking(|| Ok(autostart::is_enabled())).await?;
    let settings = state.settings();
    Ok(AppInfo {
        version: app.package_info().version.to_string(),
        os: std::env::consts::OS,
        arch: std::env::consts::ARCH,
        os_version: os_version(),
        elevated: state.elevated,
        read_only: state.read_only,
        mock: state.mock,
        package_kind: tauri::utils::platform::bundle_type()
            .map(|b| format!("{b:?}").to_lowercase()),
        tray_available: state.tray(),
        capabilities,
        data_dir: state.store.paths.data_dir.display().to_string(),
        log_dir: state.store.paths.log_dir.display().to_string(),
        oui_source_date: oui::SOURCE_DATE,
        autostart_enabled,
        startup: state.startup.lock().map(|s| s.clone()).unwrap_or_default(),
        launched_minimized: state.args.minimized
            || (state.args.autostart && settings.start_minimized),
    })
}

/// Called by the UI once it has rendered; shows the window unless the app starts minimized.
#[tauri::command]
pub fn ui_ready(app: AppHandle, state: State<'_, AppState>) {
    if state.ui_ready.swap(true, Ordering::SeqCst) {
        return;
    }
    crate::on_ui_ready(&app);
}

#[tauri::command]
pub async fn relaunch_elevated(app: AppHandle) -> ApiResult<()> {
    #[cfg(windows)]
    {
        use platform::windows::ElevationError;

        let state = app.state::<AppState>();
        let mut args = state.args.forwarded();
        args.push(format!("--after-pid={}", std::process::id()));
        platform::windows::release_instance();
        // The UAC prompt can stay open for a long time: keep it off the main thread.
        let result = tauri::async_runtime::spawn_blocking(move || {
            platform::windows::relaunch_elevated(&args)
        })
        .await
        .unwrap_or(Err(ElevationError::Failed(0)));
        if result.is_ok() {
            state.exiting.store(true, Ordering::SeqCst);
            app.exit(0);
            return Ok(());
        }
        // Whatever went wrong, this instance keeps running: take the single-instance role back.
        let _ = platform::windows::register_instance({
            let app = app.clone();
            move || tray::show_main_window(&app)
        });
        match result {
            Err(ElevationError::Cancelled) => {
                Err(ApiError::new("AUTH_CANCELLED", "elevation was cancelled"))
            }
            Err(ElevationError::Failed(code)) => Err(ApiError::new(
                "OS_ERROR",
                format!("could not restart as administrator (error {code})"),
            )),
            Ok(()) => Ok(()),
        }
    }
    #[cfg(not(windows))]
    {
        let _ = app;
        Err(ApiError::new("UNSUPPORTED", "only needed on Windows"))
    }
}

#[tauri::command]
pub fn quit_app(app: AppHandle) {
    crate::request_quit(&app);
}

#[tauri::command]
pub fn open_url(url: String) -> ApiResult<()> {
    let url = links::allowed(&url).ok_or_else(|| ApiError::invalid("this link is not allowed"))?;
    platform::open_external(&url)?;
    Ok(())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Location {
    Data,
    Logs,
    Config,
}

#[tauri::command]
pub fn open_location(state: State<'_, AppState>, kind: Location) -> ApiResult<()> {
    let p = &state.store.paths;
    let dir = match kind {
        Location::Data => &p.data_dir,
        Location::Logs => &p.log_dir,
        Location::Config => &p.config_dir,
    };
    randommac_core::storage::ensure_dir(dir)?;
    platform::open_path(dir)?;
    Ok(())
}

#[tauri::command]
pub async fn diagnostics(app: AppHandle, state: State<'_, AppState>) -> ApiResult<String> {
    let info = app_info(app.clone(), state.clone()).await?;
    let backend = Arc::clone(&state.backend);
    let adapters = blocking(move || Ok(backend.list_adapters().unwrap_or_default())).await?;
    let settings = state.settings();
    let mut out = format!(
        "RandomMac {} ({} {}, {})\npackage: {}\nelevated: {} · read-only: {} · tray: {}\ncapabilities: {}\nlinux method: {:?}\nadapters:\n",
        info.version,
        info.os,
        info.arch,
        info.os_version,
        info.package_kind.unwrap_or_else(|| "dev".into()),
        info.elevated,
        info.read_only,
        info.tray_available,
        serde_json::to_string(&info.capabilities)?,
        settings.linux.method,
    );
    for a in adapters {
        // Hardware addresses are redacted to the vendor prefix.
        let prefix = a
            .permanent_mac
            .or(a.current_mac)
            .map(|m| {
                let o = m.oui();
                format!("{:02X}:{:02X}:{:02X}:xx:xx:xx", o[0], o[1], o[2])
            })
            .unwrap_or_else(|| "-".into());
        out.push_str(&format!(
            "  - {} [{:?}{}] {:?} · {} · methods {:?} · support {:?} · override {}\n",
            a.description,
            a.kind,
            if a.is_virtual { ", virtual" } else { "" },
            a.state,
            prefix,
            a.methods,
            a.supports_change,
            a.override_value.is_some(),
        ));
    }
    Ok(out)
}

// ---------------------------------------------------------------------------------------------
// Adapters & MAC
// ---------------------------------------------------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdapterView {
    #[serde(flatten)]
    adapter: Adapter,
    vendor: Option<&'static str>,
    hardware_vendor: Option<&'static str>,
    original_mac: Option<MacAddress>,
    /// The current address is not the original one.
    changed: bool,
    /// RandomMac has something to undo for this adapter.
    restorable: bool,
}

#[tauri::command]
pub async fn list_adapters(state: State<'_, AppState>) -> ApiResult<Vec<AdapterView>> {
    let backend = Arc::clone(&state.backend);
    let store = state.store.clone();
    let session = state.changed_adapters();
    blocking(move || {
        let adapters = backend.list_adapters()?;
        let (app_state, _) = store.load_state();
        Ok(adapters
            .into_iter()
            .map(|a| {
                let original = ops::original_mac(&a, &app_state);
                let changed = matches!((a.current_mac, original), (Some(c), Some(o)) if c != o);
                let restorable = a.override_value.is_some()
                    || app_state.original(&a.id).is_some()
                    || session.contains(&a.id);
                AdapterView {
                    vendor: a
                        .current_mac
                        .and_then(|m| oui::lookup(m.oui()))
                        .map(|v| v.name),
                    hardware_vendor: original.and_then(|m| oui::lookup(m.oui())).map(|v| v.name),
                    original_mac: original,
                    changed,
                    restorable,
                    adapter: a,
                }
            })
            .collect())
    })
    .await
}

#[tauri::command]
pub async fn set_adapter_enabled(
    app: AppHandle,
    state: State<'_, AppState>,
    adapter_id: String,
    enabled: bool,
) -> ApiResult<()> {
    if state.read_only {
        return Err(ApiError::read_only());
    }
    let _guard = state.op_lock.try_lock().map_err(|_| ApiError::busy())?;
    let backend = Arc::clone(&state.backend);
    blocking(move || {
        let adapter = backend.find_adapter(&adapter_id)?;
        backend.set_enabled(&adapter, enabled)?;
        Ok(())
    })
    .await?;
    let _ = app.emit("adapters:changed", ());
    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MacInfo {
    valid: bool,
    error: Option<String>,
    mac: Option<MacAddress>,
    vendor: Option<&'static str>,
    warnings: Vec<&'static str>,
}

fn inspect(state: &AppState, mac: MacAddress, adapter: Option<&Adapter>) -> MacInfo {
    let mut warnings = Vec::new();
    if mac.is_multicast() {
        warnings.push("multicast");
    }
    if mac.is_zero() {
        warnings.push("zero");
    }
    if let Some(a) = adapter {
        if a.current_mac == Some(mac) {
            warnings.push("sameAsCurrent");
        }
        if cfg!(windows)
            && a.kind == randommac_net::AdapterKind::Wireless
            && !mac.is_locally_administered()
        {
            warnings.push("wifiNeedsLocal");
        }
    }
    let (blacklist, _) = state.store.load_blacklist();
    if blacklist.is_blocked(mac, adapter.map(|a| a.id.as_str())) {
        warnings.push("blacklisted");
    }
    MacInfo {
        valid: mac.is_assignable(),
        error: None,
        mac: Some(mac),
        vendor: oui::lookup(mac.oui()).map(|v| v.name),
        warnings,
    }
}

#[tauri::command]
pub async fn inspect_mac(
    state: State<'_, AppState>,
    input: String,
    adapter_id: Option<String>,
) -> ApiResult<MacInfo> {
    let mac: MacAddress = match input.parse() {
        Ok(m) => m,
        Err(e) => {
            return Ok(MacInfo {
                valid: false,
                error: Some(e.to_string()),
                mac: None,
                vendor: None,
                warnings: vec![],
            });
        }
    };
    let adapter = match adapter_id {
        Some(id) => {
            let backend = Arc::clone(&state.backend);
            blocking(move || Ok(backend.find_adapter(&id).ok())).await?
        }
        None => None,
    };
    Ok(inspect(&state, mac, adapter.as_ref()))
}

#[tauri::command]
pub async fn generate_mac(
    state: State<'_, AppState>,
    adapter_id: Option<String>,
    mode: Option<GenerationMode>,
) -> ApiResult<MacInfo> {
    let mode = mode.unwrap_or_else(|| state.settings().generation_mode);
    let adapter = match adapter_id {
        Some(id) => {
            let backend = Arc::clone(&state.backend);
            blocking(move || Ok(backend.find_adapter(&id).ok())).await?
        }
        None => None,
    };
    let (blacklist, _) = state.store.load_blacklist();
    let avoid: Vec<MacAddress> = adapter
        .as_ref()
        .and_then(|a| a.current_mac)
        .into_iter()
        .collect();
    let mac = generate(
        &mut OsRandom,
        &GenerateRequest {
            mode,
            base: adapter
                .as_ref()
                .and_then(|a| a.permanent_mac.or(a.current_mac)),
            avoid: &avoid,
            blacklist: Some(&blacklist),
            adapter_id: adapter.as_ref().map(|a| a.id.as_str()),
        },
    )
    .map_err(|e| ApiError::new("GENERATE_FAILED", e.to_string()))?;
    Ok(inspect(&state, mac, adapter.as_ref()))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VendorView {
    id: &'static str,
    name: &'static str,
    imitable: bool,
    oui_count: u32,
}

#[tauri::command]
pub fn list_vendors() -> Vec<VendorView> {
    let mut v: Vec<VendorView> = oui::vendors()
        .iter()
        .map(|v| VendorView {
            id: v.id,
            name: v.name,
            imitable: v.imitable,
            oui_count: v.oui_count,
        })
        .collect();
    v.sort_by_key(|x| x.name.to_lowercase());
    v
}

#[tauri::command]
pub async fn apply_mac(
    app: AppHandle,
    adapter_id: String,
    mac: String,
    ignore_blacklist: Option<bool>,
) -> ApiResult<OpResult> {
    let mac: MacAddress = mac
        .parse()
        .map_err(|e: randommac_core::ParseMacError| ApiError::new("INVALID_MAC", e.to_string()))?;
    let state = app.state::<AppState>();
    {
        if let Ok(mut s) = state.settings.lock()
            && s.selected_adapter_id.as_deref() != Some(adapter_id.as_str())
        {
            s.selected_adapter_id = Some(adapter_id.clone());
            let _ = state.store.save_settings(&s);
        }
    }
    ops_runner::apply(
        &app,
        adapter_id,
        Some(mac),
        ChangeSource::Ui,
        ignore_blacklist.unwrap_or(false),
        false,
    )
    .await
}

#[tauri::command]
pub async fn restore_mac(app: AppHandle, adapter_id: String) -> ApiResult<OpResult> {
    ops_runner::restore(&app, adapter_id, ChangeSource::Ui, false).await
}

#[tauri::command]
pub async fn restore_all(app: AppHandle) -> ApiResult<Vec<OpResult>> {
    ops_runner::restore_all(&app, ChangeSource::Ui, false).await
}

// ---------------------------------------------------------------------------------------------
// History
// ---------------------------------------------------------------------------------------------

#[tauri::command]
pub async fn history_list(state: State<'_, AppState>) -> ApiResult<Vec<HistoryEntry>> {
    let store = state.store.clone();
    blocking(move || Ok(store.load_history().0.entries)).await
}

#[tauri::command]
pub async fn history_clear(app: AppHandle, state: State<'_, AppState>) -> ApiResult<()> {
    let store = state.store.clone();
    blocking(move || {
        let _lock = store.lock()?;
        let (mut h, _) = store.load_history();
        h.clear();
        store.save_history(&h)?;
        Ok(())
    })
    .await?;
    let _ = app.emit("history:changed", ());
    Ok(())
}

fn save_dialog(app: &AppHandle, title: &str, name: &str, ext: &str) -> Option<PathBuf> {
    app.dialog()
        .file()
        .set_title(title)
        .set_file_name(name)
        .add_filter(ext.to_uppercase(), &[ext])
        .blocking_save_file()
        .and_then(|p| p.into_path().ok())
}

fn open_dialog(app: &AppHandle, title: &str, ext: &str) -> Option<PathBuf> {
    app.dialog()
        .file()
        .set_title(title)
        .add_filter(ext.to_uppercase(), &[ext])
        .blocking_pick_file()
        .and_then(|p| p.into_path().ok())
}

fn write_file(path: &PathBuf, bytes: &[u8]) -> ApiResult<()> {
    let mut f = std::fs::File::create(path)?;
    f.write_all(bytes)?;
    f.sync_all()?;
    Ok(())
}

fn stamp() -> String {
    Utc::now().format("%Y%m%d-%H%M%S").to_string()
}

#[tauri::command]
pub async fn history_export(
    app: AppHandle,
    state: State<'_, AppState>,
    format: String,
) -> ApiResult<Option<String>> {
    let store = state.store.clone();
    let title = t(state.lang(), Key::DialogExport, &[]);
    blocking(move || {
        let csv = format == "csv";
        let ext = if csv { "csv" } else { "json" };
        let Some(path) = save_dialog(
            &app,
            &title,
            &format!("randommac-history-{}.{ext}", stamp()),
            ext,
        ) else {
            return Ok(None);
        };
        let (h, _) = store.load_history();
        let bytes = if csv {
            history::to_csv(&h.entries).into_bytes()
        } else {
            serde_json::to_vec_pretty(&h.entries)?
        };
        write_file(&path, &bytes)?;
        Ok(Some(path.display().to_string()))
    })
    .await
}

// ---------------------------------------------------------------------------------------------
// Blacklist
// ---------------------------------------------------------------------------------------------

#[tauri::command]
pub async fn blacklist_get(state: State<'_, AppState>) -> ApiResult<Vec<BlacklistEntry>> {
    let store = state.store.clone();
    blocking(move || Ok(store.load_blacklist().0.entries)).await
}

#[tauri::command]
pub async fn blacklist_add(
    app: AppHandle,
    state: State<'_, AppState>,
    pattern: String,
    adapter_id: Option<String>,
    note: Option<String>,
) -> ApiResult<BlacklistEntry> {
    let pattern: MacPattern =
        pattern
            .parse()
            .map_err(|e: randommac_core::blacklist::PatternError| {
                ApiError::new("INVALID_PATTERN", e.to_string())
            })?;
    let store = state.store.clone();
    let entry = blocking(move || {
        let (mut bl, _) = store.load_blacklist();
        let entry = bl
            .add(pattern, adapter_id, note, Utc::now())
            .map_err(|e| ApiError::new("BLACKLIST_REJECTED", e.to_string()))?;
        store.save_blacklist(&bl)?;
        Ok(entry)
    })
    .await?;
    let _ = app.emit("blacklist:changed", ());
    Ok(entry)
}

#[tauri::command]
pub async fn blacklist_remove(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> ApiResult<bool> {
    let store = state.store.clone();
    let removed = blocking(move || {
        let (mut bl, _) = store.load_blacklist();
        let removed = bl.remove(&id);
        if removed {
            store.save_blacklist(&bl)?;
        }
        Ok(removed)
    })
    .await?;
    let _ = app.emit("blacklist:changed", ());
    Ok(removed)
}

#[tauri::command]
pub async fn blacklist_export(
    app: AppHandle,
    state: State<'_, AppState>,
) -> ApiResult<Option<String>> {
    let store = state.store.clone();
    let title = t(state.lang(), Key::DialogExport, &[]);
    blocking(move || {
        let Some(path) = save_dialog(&app, &title, "randommac-blacklist.json", "json") else {
            return Ok(None);
        };
        let (bl, _) = store.load_blacklist();
        write_file(&path, &serde_json::to_vec_pretty(&bl)?)?;
        Ok(Some(path.display().to_string()))
    })
    .await
}

#[tauri::command]
pub async fn blacklist_import(
    app: AppHandle,
    state: State<'_, AppState>,
) -> ApiResult<Option<usize>> {
    let store = state.store.clone();
    let title = t(state.lang(), Key::DialogImport, &[]);
    let emitter = app.clone();
    let added = blocking(move || {
        let Some(path) = open_dialog(&app, &title, "json") else {
            return Ok(None);
        };
        let text = randommac_core::storage::read_text(&path)?.unwrap_or_default();
        let value: serde_json::Value = serde_json::from_str(&text)?;
        // Accept RandomMac 2 files and RandomMac 1.x files.
        let incoming = match serde_json::from_value::<randommac_core::Blacklist>(value.clone()) {
            Ok(bl) if !bl.entries.is_empty() => bl,
            _ => randommac_core::legacy_v1::parse_v1_blacklist(&value, Utc::now()),
        };
        let (mut bl, _) = store.load_blacklist();
        let added = bl.merge(incoming, Utc::now());
        store.save_blacklist(&bl)?;
        Ok(Some(added))
    })
    .await?;
    let _ = emitter.emit("blacklist:changed", ());
    Ok(added)
}

// ---------------------------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------------------------

#[tauri::command]
pub fn settings_get(state: State<'_, AppState>) -> Settings {
    state.settings()
}

/// Validates, saves and applies side effects (autostart, tray language, scheduler).
fn apply_settings(app: &AppHandle, mut next: Settings) -> ApiResult<Settings> {
    let state = app.state::<AppState>();
    next.sanitize();
    let previous = state.settings();
    // Fields the UI does not own are kept.
    next.last_update_check = previous.last_update_check;
    next.legacy = previous.legacy.clone();
    if next.start_with_system != previous.start_with_system
        || (next.start_with_system && next.start_minimized != previous.start_minimized)
    {
        autostart::set(next.start_with_system, next.start_minimized)?;
    }
    state.store.save_settings(&next)?;
    if let Ok(mut s) = state.settings.lock() {
        *s = next.clone();
    }
    if next.language != previous.language
        || next.selected_adapter_id != previous.selected_adapter_id
    {
        tray::refresh(app);
    }
    if next.automation != previous.automation {
        scheduler::restart(app);
    }
    let _ = app.emit("settings:changed", &next);
    Ok(next)
}

#[tauri::command]
pub async fn settings_update(app: AppHandle, settings: Settings) -> ApiResult<Settings> {
    blocking(move || apply_settings(&app, settings)).await
}

#[tauri::command]
pub async fn settings_reset(app: AppHandle) -> ApiResult<Settings> {
    blocking(move || apply_settings(&app, Settings::default())).await
}

#[tauri::command]
pub async fn settings_export(
    app: AppHandle,
    state: State<'_, AppState>,
) -> ApiResult<Option<String>> {
    let settings = state.settings();
    let title = t(state.lang(), Key::DialogExport, &[]);
    blocking(move || {
        let Some(path) = save_dialog(&app, &title, "randommac-settings.json", "json") else {
            return Ok(None);
        };
        write_file(&path, &serde_json::to_vec_pretty(&settings)?)?;
        Ok(Some(path.display().to_string()))
    })
    .await
}

#[tauri::command]
pub async fn settings_import(
    app: AppHandle,
    state: State<'_, AppState>,
) -> ApiResult<Option<Settings>> {
    let title = t(state.lang(), Key::DialogImport, &[]);
    let current = state.settings();
    blocking(move || {
        let Some(path) = open_dialog(&app, &title, "json") else {
            return Ok(None);
        };
        let text = randommac_core::storage::read_text(&path)?.unwrap_or_default();
        let mut imported =
            Settings::from_json(&text).map_err(|e| ApiError::new("PARSE_ERROR", e.to_string()))?;
        // Importing never silently changes autostart (it needs elevation and is machine specific).
        imported.start_with_system = current.start_with_system;
        apply_settings(&app, imported).map(Some)
    })
    .await
}

/// Exports settings, blacklist and history in one zip (like v1's "Export All").
#[tauri::command]
pub async fn bundle_export(
    app: AppHandle,
    state: State<'_, AppState>,
) -> ApiResult<Option<String>> {
    let store = state.store.clone();
    let settings = state.settings();
    let title = t(state.lang(), Key::DialogExport, &[]);
    blocking(move || {
        let Some(path) = save_dialog(
            &app,
            &title,
            &format!("randommac-bundle-{}.zip", stamp()),
            "zip",
        ) else {
            return Ok(None);
        };
        write_bundle(&path, &settings, &store)?;
        Ok(Some(path.display().to_string()))
    })
    .await
}

fn write_bundle(path: &PathBuf, settings: &Settings, store: &ops::DataStore) -> ApiResult<()> {
    let file = std::fs::File::create(path)?;
    let mut zip = zip::ZipWriter::new(file);
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    let mut add = |name: &str, bytes: Vec<u8>| -> ApiResult<()> {
        zip.start_file(name, opts)
            .map_err(|e| ApiError::new("IO_ERROR", e.to_string()))?;
        zip.write_all(&bytes)?;
        Ok(())
    };
    add("settings.json", serde_json::to_vec_pretty(settings)?)?;
    add(
        "blacklist.json",
        serde_json::to_vec_pretty(&store.load_blacklist().0)?,
    )?;
    add(
        "history.json",
        serde_json::to_vec_pretty(&store.load_history().0)?,
    )?;
    zip.finish()
        .map_err(|e| ApiError::new("IO_ERROR", e.to_string()))?;
    Ok(())
}

#[tauri::command]
pub async fn v1_import(
    app: AppHandle,
    state: State<'_, AppState>,
) -> ApiResult<Option<crate::state::V1ImportReport>> {
    let store = state.store.clone();
    let current = state.settings();
    let mut settings = current.clone();
    let app2 = app.clone();
    let report = blocking(move || {
        let report = crate::legacy::import_v1(&store, &mut settings);
        if report.is_some() {
            // Like settings_import: never change autostart as a side effect (it needs
            // elevation and would fail in read-only mode after the data was already imported).
            settings.start_with_system = current.start_with_system;
            settings.start_minimized = current.start_minimized;
            let imported_at = settings.legacy.imported_from_v1_at;
            apply_settings(&app2, settings)?;
            // apply_settings keeps the stored `legacy` block; record this import explicitly.
            let state = app2.state::<AppState>();
            let updated = state.settings.lock().ok().map(|mut s| {
                s.legacy.imported_from_v1_at = imported_at;
                s.clone()
            });
            if let Some(updated) = updated {
                state.store.save_settings(&updated)?;
            }
        }
        Ok(report)
    })
    .await?;
    let _ = app.emit("history:changed", ());
    let _ = app.emit("blacklist:changed", ());
    Ok(report)
}

// ---------------------------------------------------------------------------------------------
// Logs
// ---------------------------------------------------------------------------------------------

#[tauri::command]
pub fn logs_get(state: State<'_, AppState>, after: Option<u64>) -> Vec<LogEntry> {
    state.logs.since(after)
}

#[tauri::command]
pub fn logs_clear(state: State<'_, AppState>) {
    state.logs.clear();
}

#[tauri::command]
pub async fn logs_export(app: AppHandle, state: State<'_, AppState>) -> ApiResult<Option<String>> {
    let entries = state.logs.since(None);
    let title = t(state.lang(), Key::DialogExport, &[]);
    blocking(move || {
        let Some(path) = save_dialog(
            &app,
            &title,
            &format!("randommac-log-{}.txt", stamp()),
            "txt",
        ) else {
            return Ok(None);
        };
        let text: String = entries
            .iter()
            .map(|e| {
                format!(
                    "{} {:<5} {}: {}\n",
                    e.timestamp,
                    e.level.to_uppercase(),
                    e.target,
                    e.message
                )
            })
            .collect();
        write_file(&path, text.as_bytes())?;
        Ok(Some(path.display().to_string()))
    })
    .await
}

// ---------------------------------------------------------------------------------------------
// Updates
// ---------------------------------------------------------------------------------------------

#[tauri::command]
pub async fn update_check(app: AppHandle) -> ApiResult<UpdateInfo> {
    updater::check(&app).await
}

#[tauri::command]
pub async fn update_install(app: AppHandle) -> ApiResult<()> {
    updater::install(&app).await
}
