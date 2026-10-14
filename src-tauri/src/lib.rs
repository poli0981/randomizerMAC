//! RandomMac desktop app (Tauri 2).
//!
//! Startup order matters: process hardening and the single-instance check happen before UAC
//! elevation is requested, and elevation happens before any thread, WebView or plugin starts.

mod args;
mod autostart;
mod commands;
mod error;
mod i18n;
mod legacy;
mod links;
mod logging;
mod ops_runner;
mod platform;
mod scheduler;
mod state;
mod tray;
mod updater;

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use randommac_core::storage::LoadStatus;
use randommac_core::{AppPaths, ChangeSource, settings::Theme};
use randommac_net::NetworkBackend;
use randommac_net::mock::MockBackend;
use randommac_net::ops::DataStore;
use tauri::{AppHandle, Manager, RunEvent, WebviewUrl, WebviewWindowBuilder, WindowEvent};
use tauri_plugin_window_state::StateFlags;

use crate::args::Args;
use crate::state::{AppState, StartupReport};

/// Entry point used by `main.rs`.
pub fn main_entry() -> ExitCode {
    #[cfg(windows)]
    platform::windows::harden();
    #[cfg(target_os = "linux")]
    if let Err(msg) = platform::linux::prepare() {
        eprintln!("{msg}");
        return ExitCode::from(1);
    }

    let args = Args::parse();
    if args.version {
        println!("RandomMac {}", env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }
    #[cfg(windows)]
    {
        if let Some(pid) = args.after_pid {
            platform::windows::wait_for_process(pid, 15_000);
        }
        if args.cleanup_legacy {
            legacy::cleanup_legacy();
            return ExitCode::SUCCESS;
        }
        if args.cleanup_uninstall {
            autostart::remove_all();
            return ExitCode::SUCCESS;
        }
        if !args.smoke_test && platform::windows::signal_existing_instance() {
            return ExitCode::SUCCESS;
        }
    }

    #[cfg(windows)]
    let (elevated, read_only, temp_dir) = {
        let elevated = platform::windows::is_elevated();
        let want_elevation = (!cfg!(debug_assertions)
            || std::env::var_os("RANDOMMAC_ELEVATE").is_some())
            && !args.no_elevate
            && !args.autostart
            && !args.smoke_test
            && !args.mock;
        if !elevated && want_elevation {
            match platform::windows::relaunch_elevated(&args.forwarded()) {
                Ok(()) => return ExitCode::SUCCESS,
                Err(platform::windows::ElevationError::Cancelled) => {}
                Err(platform::windows::ElevationError::Failed(code)) => {
                    eprintln!("could not restart as administrator (error {code})");
                }
            }
        }
        let temp_dir = if elevated {
            platform::windows::secure_temp_dir()
        } else {
            None
        };
        (elevated, !elevated && !args.mock, temp_dir)
    };
    #[cfg(not(windows))]
    let (elevated, read_only, temp_dir): (bool, bool, Option<PathBuf>) = (false, false, None);
    run_app(args, elevated, read_only, temp_dir)
}

fn run_app(args: Args, elevated: bool, read_only: bool, temp_dir: Option<PathBuf>) -> ExitCode {
    let paths = match AppPaths::resolve() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("cannot determine the data folder: {e}");
            return ExitCode::from(1);
        }
    };
    let logs = logging::init(Some(&paths.log_dir));
    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        os = std::env::consts::OS,
        elevated,
        read_only,
        mock = args.mock,
        "RandomMac starting"
    );

    let store = DataStore::new(paths);
    let mut startup = StartupReport::default();
    let (mut settings, status) = store.load_settings();
    if let LoadStatus::Recovered { reason, .. } = &status {
        startup
            .recovered_files
            .push(format!("settings.json: {reason}"));
    }
    let first_run = matches!(status, LoadStatus::Missing);
    if first_run && !args.mock {
        startup.v1_import = legacy::import_v1(&store, &mut settings);
    }
    if elevated {
        startup.legacy_task_removed = legacy::cleanup_legacy();
        if startup.legacy_task_removed {
            settings.legacy.legacy_task_removed_at = Some(chrono::Utc::now());
        }
    }
    if (first_run || startup.v1_import.is_some() || startup.legacy_task_removed)
        && let Err(e) = store.save_settings(&settings)
    {
        tracing::warn!("could not save settings: {e}");
    }
    if let Some(report) = &startup.v1_import
        && report.autostart
        && !read_only
        && let Err(e) = autostart::set(true, settings.start_minimized)
    {
        tracing::warn!("could not migrate v1 autostart: {e}");
    }
    if !read_only && !args.mock {
        autostart::heal(settings.start_with_system, settings.start_minimized);
    }

    let backend: Arc<dyn NetworkBackend> = if args.mock {
        Arc::new(MockBackend::from_env())
    } else {
        Arc::from(randommac_net::system_backend())
    };
    let state = AppState {
        backend,
        store,
        settings: Mutex::new(settings),
        op_lock: tokio::sync::Mutex::new(()),
        session_changed: Mutex::default(),
        elevated,
        read_only,
        mock: args.mock,
        args,
        tray_available: AtomicBool::new(false),
        logs,
        startup: Mutex::new(startup),
        updater: Mutex::default(),
        exiting: AtomicBool::new(false),
        ui_ready: AtomicBool::new(false),
        close_hint_shown: AtomicBool::new(false),
        scheduler: Mutex::new(None),
        temp_dir,
    };

    #[allow(unused_mut)]
    let mut builder = tauri::Builder::default();
    #[cfg(target_os = "linux")]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            tray::show_main_window(app);
        }));
    }
    let app = builder
        .plugin(
            tauri_plugin_window_state::Builder::new()
                .with_state_flags(StateFlags::all() & !StateFlags::VISIBLE)
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::ui_ready,
            commands::relaunch_elevated,
            commands::quit_app,
            commands::open_url,
            commands::open_location,
            commands::diagnostics,
            commands::list_adapters,
            commands::set_adapter_enabled,
            commands::generate_mac,
            commands::inspect_mac,
            commands::list_vendors,
            commands::apply_mac,
            commands::restore_mac,
            commands::restore_all,
            commands::history_list,
            commands::history_clear,
            commands::history_export,
            commands::blacklist_get,
            commands::blacklist_add,
            commands::blacklist_remove,
            commands::blacklist_import,
            commands::blacklist_export,
            commands::settings_get,
            commands::settings_update,
            commands::settings_reset,
            commands::settings_export,
            commands::settings_import,
            commands::bundle_export,
            commands::v1_import,
            commands::logs_get,
            commands::logs_clear,
            commands::logs_export,
            commands::update_check,
            commands::update_install,
        ])
        .setup(|app| {
            setup(app.handle())?;
            Ok(())
        })
        .on_window_event(on_window_event)
        .build(tauri::generate_context!());
    let app = match app {
        Ok(app) => app,
        Err(e) => {
            tracing::error!("failed to start: {e}");
            eprintln!("failed to start RandomMac: {e}");
            return ExitCode::from(1);
        }
    };
    app.run(|handle, event| match event {
        RunEvent::ExitRequested { api, code, .. } => {
            let state = handle.state::<AppState>();
            if code.is_none() && !state.exiting.load(Ordering::SeqCst) {
                // Last window closed while the app should keep running in the tray.
                api.prevent_exit();
            }
        }
        RunEvent::Exit => {
            let state = handle.state::<AppState>();
            #[cfg(windows)]
            if let Some(dir) = &state.temp_dir {
                platform::windows::remove_temp_dir(dir);
            }
            #[cfg(not(windows))]
            let _ = &state.temp_dir;
            tracing::info!("RandomMac exiting");
        }
        _ => {}
    });
    ExitCode::SUCCESS
}

fn is_app_url(url: &url::Url) -> bool {
    match url.scheme() {
        "tauri" => true,
        "http" | "https" => {
            matches!(url.host_str(), Some("tauri.localhost"))
                || (cfg!(debug_assertions) && matches!(url.host_str(), Some("localhost")))
        }
        _ => false,
    }
}

fn window_theme(theme: Theme) -> Option<tauri::Theme> {
    match theme {
        Theme::Light => Some(tauri::Theme::Light),
        Theme::Dark => Some(tauri::Theme::Dark),
        Theme::System => None,
    }
}

fn setup(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let state = app.state::<AppState>();
    #[cfg(windows)]
    if !state.args.smoke_test {
        let handle = app.clone();
        if !platform::windows::register_instance(move || tray::show_main_window(&handle)) {
            std::process::exit(0);
        }
    }

    let settings = state.settings();
    #[allow(unused_mut)]
    let mut window = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
        .title("RandomMac")
        .inner_size(1120.0, 740.0)
        .min_inner_size(880.0, 600.0)
        .visible(false)
        .center()
        .theme(window_theme(settings.theme))
        .disable_drag_drop_handler()
        .on_navigation(|url| {
            let allowed = is_app_url(url);
            if !allowed {
                tracing::warn!(%url, "blocked navigation");
            }
            allowed
        });
    #[cfg(windows)]
    {
        // WebView2 cannot share one profile folder between elevated and normal processes.
        let folder = if state.elevated {
            "EBWebView"
        } else {
            "EBWebView-user"
        };
        window = window.data_directory(state.store.paths.data_dir.join(folder));
    }
    window.build()?;

    #[cfg(target_os = "linux")]
    let tray_host = platform::linux::tray_host_available();
    #[cfg(not(target_os = "linux"))]
    let tray_host = true;
    // Mark the tray as available before creating it so the first menu refresh runs.
    state.tray_available.store(tray_host, Ordering::Relaxed);
    let tray_ok = tray_host && tray::create(app);
    state.tray_available.store(tray_ok, Ordering::Relaxed);
    if !tray_ok {
        tracing::info!("no system tray available; closing the window quits the app");
    }

    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(4)).await;
        let state = handle.state::<AppState>();
        if !state.ui_ready.swap(true, Ordering::SeqCst) {
            tracing::warn!("UI did not report ready in time; showing the window anyway");
            on_ui_ready(&handle);
        }
    });
    if state.args.smoke_test {
        tauri::async_runtime::spawn(async {
            tokio::time::sleep(Duration::from_secs(45)).await;
            eprintln!("smoke test failed: the UI did not become ready");
            std::process::exit(1);
        });
    }

    let handle = app.clone();
    tauri::async_runtime::spawn(async move { ops_runner::startup_automation(&handle).await });
    scheduler::restart(app);
    updater::spawn_startup_check(app);
    Ok(())
}

/// The UI rendered (or the fallback timer fired): show the window unless starting minimized.
pub(crate) fn on_ui_ready(app: &AppHandle) {
    let state = app.state::<AppState>();
    if state.args.smoke_test {
        println!("smoke test passed");
        state.exiting.store(true, Ordering::SeqCst);
        app.exit(0);
        return;
    }
    let settings = state.settings();
    let minimized = state.args.minimized || (state.args.autostart && settings.start_minimized);
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    if minimized && state.tray() {
        return;
    }
    let _ = window.show();
    if minimized {
        let _ = window.minimize();
    } else {
        let _ = window.set_focus();
    }
}

fn on_window_event(window: &tauri::Window, event: &WindowEvent) {
    if window.label() != "main" {
        return;
    }
    if let WindowEvent::CloseRequested { api, .. } = event {
        let app = window.app_handle();
        let state = app.state::<AppState>();
        if state.exiting.load(Ordering::SeqCst) {
            return;
        }
        api.prevent_close();
        if state.settings().close_to_tray && state.tray() {
            let _ = window.hide();
            if !state.close_hint_shown.swap(true, Ordering::SeqCst) {
                let lang = state.lang();
                ops_runner::notify(app, i18n::t(lang, i18n::Key::NotifyStillRunning, &[]), true);
            }
        } else {
            request_quit(app);
        }
    }
}

/// Quits the app, first restoring adapters changed in this session if the user asked for it.
pub(crate) fn request_quit(app: &AppHandle) {
    let state = app.state::<AppState>();
    if state.exiting.swap(true, Ordering::SeqCst) {
        return;
    }
    let restore = state.settings().automation.restore_on_exit
        && !state.read_only
        && !state.changed_adapters().is_empty();
    if !restore {
        app.exit(0);
        return;
    }
    let app = app.clone();
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.hide();
    }
    tauri::async_runtime::spawn(async move {
        let result = tokio::time::timeout(
            Duration::from_secs(120),
            ops_runner::restore_all(&app, ChangeSource::Exit, true),
        )
        .await;
        if !matches!(result, Ok(Ok(_))) {
            tracing::warn!("restore on exit did not complete");
        }
        app.exit(0);
    });
}
