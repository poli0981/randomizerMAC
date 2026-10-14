//! Tray icon and menu (localized, rebuilt when the language or the selected adapter changes).

use std::sync::atomic::Ordering;

use randommac_core::ChangeSource;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, Wry};

use crate::i18n::{Key, t};
use crate::ops_runner;
use crate::state::AppState;

pub const TRAY_ID: &str = "main";

/// The adapter tray actions apply to: the one selected in the UI, else the first connected
/// physical adapter.
fn target_adapter(app: &AppHandle) -> Option<(String, String)> {
    let state = app.state::<AppState>();
    let adapters = state.backend.list_adapters().ok()?;
    let selected = state.settings().selected_adapter_id;
    adapters
        .iter()
        .find(|a| Some(&a.id) == selected.as_ref())
        .or_else(|| adapters.iter().find(|a| !a.is_virtual && a.connected))
        .or_else(|| adapters.iter().find(|a| !a.is_virtual))
        .map(|a| (a.id.clone(), a.name.clone()))
}

fn build_menu(app: &AppHandle, target: Option<&(String, String)>) -> tauri::Result<Menu<Wry>> {
    let state = app.state::<AppState>();
    let lang = state.lang();
    let can_change = !state.read_only && target.is_some();
    let name = target
        .map(|(_, n)| n.clone())
        .unwrap_or_else(|| t(lang, Key::TrayNoAdapter, &[]));
    Menu::with_items(
        app,
        &[
            &MenuItem::with_id(app, "show", t(lang, Key::TrayShow, &[]), true, None::<&str>)?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(
                app,
                "randomize",
                t(lang, Key::TrayRandomize, &[&name]),
                can_change,
                None::<&str>,
            )?,
            &MenuItem::with_id(
                app,
                "restore",
                t(lang, Key::TrayRestore, &[&name]),
                can_change,
                None::<&str>,
            )?,
            &MenuItem::with_id(
                app,
                "restore_all",
                t(lang, Key::TrayRestoreAll, &[]),
                !state.read_only,
                None::<&str>,
            )?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(
                app,
                "updates",
                t(lang, Key::TrayCheckUpdates, &[]),
                true,
                None::<&str>,
            )?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, "quit", t(lang, Key::TrayQuit, &[]), true, None::<&str>)?,
        ],
    )
}

pub fn show_main_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

fn on_menu(app: &AppHandle, id: &str) {
    let app = app.clone();
    match id {
        "show" => show_main_window(&app),
        "randomize" | "restore" => {
            let restore = id == "restore";
            tauri::async_runtime::spawn(async move {
                let target = {
                    let app = app.clone();
                    tauri::async_runtime::spawn_blocking(move || target_adapter(&app))
                        .await
                        .ok()
                        .flatten()
                };
                let Some((adapter_id, _)) = target else {
                    return;
                };
                let result = if restore {
                    ops_runner::restore(&app, adapter_id, ChangeSource::Tray, false).await
                } else {
                    ops_runner::apply(&app, adapter_id, None, ChangeSource::Tray, false, false)
                        .await
                };
                if let Err(e) = result {
                    ops_runner::notify(&app, e.message.clone(), true);
                }
            });
        }
        "restore_all" => {
            tauri::async_runtime::spawn(async move {
                if let Err(e) = ops_runner::restore_all(&app, ChangeSource::Tray, false).await {
                    ops_runner::notify(&app, e.message.clone(), true);
                }
            });
        }
        "updates" => {
            show_main_window(&app);
            let _ = app.emit("app:navigate", "update");
        }
        "quit" => crate::request_quit(&app),
        _ => {}
    }
}

/// Creates the tray icon. Returns `false` if the platform has no tray.
pub fn create(app: &AppHandle) -> bool {
    let menu = match build_menu(app, None) {
        Ok(m) => m,
        Err(e) => {
            tracing::warn!("tray menu failed: {e}");
            return false;
        }
    };
    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("RandomMac")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| on_menu(app, event.id().as_ref()))
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    match builder.build(app) {
        Ok(_) => {
            refresh(app);
            true
        }
        Err(e) => {
            tracing::warn!("tray icon unavailable: {e}");
            false
        }
    }
}

/// Rebuilds the menu in the background (adapter enumeration can take a moment).
pub fn refresh(app: &AppHandle) {
    let state = app.state::<AppState>();
    if !state.tray_available.load(Ordering::Relaxed) {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let target = target_adapter(&app);
        match build_menu(&app, target.as_ref()) {
            Ok(menu) => {
                if let Some(tray) = app.tray_by_id(TRAY_ID) {
                    let _ = tray.set_menu(Some(menu));
                }
            }
            Err(e) => tracing::debug!("tray refresh failed: {e}"),
        }
    });
}
