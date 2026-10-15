//! Operating-system specific glue for the GUI process.

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(windows)]
pub mod windows;

use std::path::Path;

/// Opens a URL or folder with the desktop's default handler, as the logged-in user.
pub fn open_external(target: &str) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        windows::open_with_shell(target)
    }
    #[cfg(target_os = "linux")]
    {
        linux::open_with_xdg(target)
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        std::process::Command::new("open")
            .arg(target)
            .spawn()
            .map(|_| ())
    }
}

pub fn open_path(path: &Path) -> std::io::Result<()> {
    open_external(&path.to_string_lossy())
}
