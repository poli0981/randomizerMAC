//! Command-line flags of the GUI executable (the scriptable interface is `randommac-cli`).

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Args {
    /// Started by the operating system at sign-in (autostart task / XDG autostart entry).
    pub autostart: bool,
    /// Start hidden in the tray.
    pub minimized: bool,
    /// Start, wait until the UI reports ready, then exit 0 (CI smoke test).
    pub smoke_test: bool,
    /// Never ask for elevation (Windows).
    pub no_elevate: bool,
    pub version: bool,
    /// Installer hook: remove RandomMac 1.x leftovers (elevated logon task).
    pub cleanup_legacy: bool,
    /// Uninstaller hook: remove autostart entries.
    pub cleanup_uninstall: bool,
    /// Wait for this process to exit before starting (used after a self-relaunch).
    pub after_pid: Option<u32>,
    /// Use the in-memory demo backend (debug builds only).
    pub mock: bool,
}

impl Args {
    pub fn parse() -> Self {
        Self::from_iter(std::env::args().skip(1))
    }

    pub fn from_iter<I: IntoIterator<Item = String>>(iter: I) -> Self {
        let mut args = Args::default();
        let mut it = iter.into_iter().peekable();
        while let Some(arg) = it.next() {
            match arg.as_str() {
                "--autostart" => args.autostart = true,
                "--minimized" => args.minimized = true,
                "--smoke-test" => args.smoke_test = true,
                "--no-elevate" => args.no_elevate = true,
                "--version" | "-V" => args.version = true,
                "--cleanup-legacy" => args.cleanup_legacy = true,
                "--cleanup-uninstall" => args.cleanup_uninstall = true,
                "--mock" => args.mock = cfg!(debug_assertions),
                "--after-pid" => args.after_pid = it.next().and_then(|v| v.parse().ok()),
                other => {
                    if let Some(v) = other.strip_prefix("--after-pid=") {
                        args.after_pid = v.parse().ok();
                    }
                }
            }
        }
        if cfg!(debug_assertions)
            && std::env::var("RANDOMMAC_BACKEND").is_ok_and(|v| v.eq_ignore_ascii_case("mock"))
        {
            args.mock = true;
        }
        args
    }

    /// Flags forwarded when the app relaunches itself (elevation, restart).
    pub fn forwarded(&self) -> Vec<String> {
        let mut out = Vec::new();
        if self.autostart {
            out.push("--autostart".to_string());
        }
        if self.minimized {
            out.push("--minimized".to_string());
        }
        if self.mock {
            out.push("--mock".to_string());
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(v: &[&str]) -> Args {
        Args::from_iter(v.iter().map(|s| s.to_string()))
    }

    #[test]
    fn parses_flags() {
        let a = parse(&[
            "--autostart",
            "--minimized",
            "--after-pid",
            "42",
            "--unknown",
        ]);
        assert!(a.autostart && a.minimized);
        assert_eq!(a.after_pid, Some(42));
        assert_eq!(parse(&["--after-pid=7"]).after_pid, Some(7));
        assert!(parse(&["--version"]).version);
        assert_eq!(a.forwarded(), vec!["--autostart", "--minimized"]);
        assert!(parse(&[]).forwarded().is_empty());
    }
}
