fn main() {
    // Every command the UI may call. Listing them makes Tauri generate one permission per
    // command, and only the ones granted in capabilities/main.json are reachable from the webview.
    let attributes =
        tauri_build::Attributes::new().app_manifest(tauri_build::AppManifest::new().commands(&[
            "app_info",
            "ui_ready",
            "relaunch_elevated",
            "quit_app",
            "open_url",
            "open_location",
            "diagnostics",
            "list_adapters",
            "set_adapter_enabled",
            "generate_mac",
            "inspect_mac",
            "list_vendors",
            "apply_mac",
            "restore_mac",
            "restore_all",
            "history_list",
            "history_clear",
            "history_export",
            "blacklist_get",
            "blacklist_add",
            "blacklist_remove",
            "blacklist_import",
            "blacklist_export",
            "settings_get",
            "settings_update",
            "settings_reset",
            "settings_export",
            "settings_import",
            "bundle_export",
            "v1_import",
            "logs_get",
            "logs_clear",
            "logs_export",
            "update_check",
            "update_install",
        ]));
    tauri_build::try_build(attributes).expect("failed to run tauri-build");
}
