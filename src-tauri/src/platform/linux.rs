//! Linux-specific process setup.

use std::process::Command;

/// Must run first in `main`, while the process is still single-threaded.
/// Returns an error message when the GUI must not start.
pub fn prepare() -> Result<(), String> {
    // SAFETY: geteuid has no preconditions.
    if unsafe { libc::geteuid() } == 0 && std::env::var_os("RANDOMMAC_ALLOW_ROOT").is_none() {
        return Err(
            "RandomMac's window must not run as root. Start it as your normal user (it asks for \
             authentication when needed) or use `sudo randommac-cli`."
                .into(),
        );
    }
    // WebKitGTK's DMABUF renderer shows a blank window with the proprietary NVIDIA driver.
    if std::path::Path::new("/sys/module/nvidia").exists()
        && std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none()
    {
        // SAFETY: called before any other thread exists.
        unsafe { std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1") };
    }
    Ok(())
}

/// Variables injected by the AppImage runtime that break host programs.
const APPIMAGE_VARS: &[&str] = &[
    "LD_LIBRARY_PATH",
    "LD_PRELOAD",
    "APPDIR",
    "APPIMAGE",
    "ARGV0",
    "OWD",
    "GIO_MODULE_DIR",
    "GIO_EXTRA_MODULES",
    "GDK_PIXBUF_MODULE_FILE",
    "GDK_PIXBUF_MODULEDIR",
    "GTK_PATH",
    "GTK_DATA_PREFIX",
    "GTK_EXE_PREFIX",
    "GTK_IM_MODULE_FILE",
    "GST_PLUGIN_SYSTEM_PATH",
    "GST_PLUGIN_SYSTEM_PATH_1_0",
    "GST_PLUGIN_PATH",
    "PYTHONHOME",
    "PYTHONPATH",
    "PERLLIB",
    "QT_PLUGIN_PATH",
];

pub fn open_with_xdg(target: &str) -> std::io::Result<()> {
    let xdg_open = ["/usr/bin/xdg-open", "/bin/xdg-open"]
        .into_iter()
        .find(|p| std::path::Path::new(p).is_file())
        .unwrap_or("xdg-open");
    let mut cmd = Command::new(xdg_open);
    for var in APPIMAGE_VARS {
        cmd.env_remove(var);
    }
    cmd.arg(target).spawn().map(|_| ())
}

/// Whether a StatusNotifierItem host (tray area) is available on the session bus. GNOME without
/// the AppIndicator extension (e.g. Fedora Workstation) has none, so tray icons are invisible.
pub fn tray_host_available() -> bool {
    let Ok(conn) = zbus::blocking::Connection::session() else {
        return false;
    };
    let reply = conn.call_method(
        Some("org.kde.StatusNotifierWatcher"),
        "/StatusNotifierWatcher",
        Some("org.freedesktop.DBus.Properties"),
        "Get",
        &(
            "org.kde.StatusNotifierWatcher",
            "IsStatusNotifierHostRegistered",
        ),
    );
    match reply {
        Ok(msg) => msg
            .body()
            .deserialize::<zbus::zvariant::OwnedValue>()
            .ok()
            .and_then(|v| bool::try_from(v).ok())
            .unwrap_or(true),
        Err(_) => false,
    }
}
