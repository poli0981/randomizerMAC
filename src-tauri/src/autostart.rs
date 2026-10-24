//! Start with the operating system.
//!
//! * Windows: a per-user Task Scheduler task (`RandomMac-Autostart-<SID>`, at logon, highest
//!   privileges) so the elevated app starts without a UAC prompt. Unlike v1 the task never stops
//!   on battery or after 72 hours, and it is re-checked on every start (self-healing).
//! * Linux: an XDG autostart entry in `~/.config/autostart`.

use crate::error::{ApiError, ApiResult};

#[cfg(windows)]
pub use self::windows::*;

#[cfg(target_os = "linux")]
pub use self::linux::*;

#[cfg(not(any(windows, target_os = "linux")))]
pub fn set(_enabled: bool, _minimized: bool) -> ApiResult<()> {
    Err(ApiError::new("UNSUPPORTED", "autostart is not supported"))
}

#[cfg(not(any(windows, target_os = "linux")))]
pub fn is_enabled() -> bool {
    false
}

/// Quotes one argument for the `Exec` key of a desktop entry. The quoting rule escapes `"`, `` ` ``,
/// `$` and `\`; the string-value rule, which is applied before it when the file is read, then
/// doubles every backslash; `%` starts field codes and must be doubled. Paths with line breaks
/// cannot be represented.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn desktop_exec_arg(arg: &str) -> Option<String> {
    if arg.contains(['\n', '\r']) {
        return None;
    }
    let mut quoted = String::with_capacity(arg.len() + 2);
    quoted.push('"');
    for c in arg.chars() {
        if matches!(c, '"' | '`' | '$' | '\\') {
            quoted.push('\\');
        }
        quoted.push(c);
    }
    quoted.push('"');
    Some(quoted.replace('\\', "\\\\").replace('%', "%%"))
}

#[cfg(test)]
mod exec_tests {
    use super::desktop_exec_arg;

    #[test]
    fn desktop_exec_quoting_follows_the_spec() {
        assert_eq!(
            desktop_exec_arg("/usr/bin/randommac").as_deref(),
            Some("\"/usr/bin/randommac\"")
        );
        // Literal `$` is `\\$` and a literal backslash is four backslashes in the file.
        assert_eq!(
            desktop_exec_arg(r"/home/a$b/c\d.AppImage").as_deref(),
            Some(r#""/home/a\\$b/c\\\\d.AppImage""#)
        );
        assert_eq!(
            desktop_exec_arg("/opt/100% \"x\"").as_deref(),
            Some(r#""/opt/100%% \\"x\\"""#)
        );
        assert_eq!(desktop_exec_arg("/tmp/a\nb"), None);
    }
}

#[cfg(windows)]
mod windows {
    use std::path::Path;
    use std::time::Duration;

    use randommac_net::process;

    use super::*;
    use crate::platform::windows::{current_user_sid, is_elevated, system32};

    const TIMEOUT: Duration = Duration::from_secs(15);
    const TASK_PREFIX: &str = "RandomMac-Autostart-";

    fn task_name() -> ApiResult<String> {
        current_user_sid()
            .map(|sid| format!("{TASK_PREFIX}{sid}"))
            .ok_or_else(|| ApiError::new("INTERNAL", "cannot determine the user SID"))
    }

    fn schtasks(args: &[&str]) -> ApiResult<process::Output> {
        Ok(process::run(
            &system32("schtasks.exe"),
            args,
            None,
            TIMEOUT,
        )?)
    }

    fn xml_escape(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
    }

    fn task_xml(sid: &str, exe: &Path, minimized: bool) -> String {
        let args = if minimized {
            "--autostart --minimized"
        } else {
            "--autostart"
        };
        format!(
            r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.4" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <RegistrationInfo>
    <Author>RandomMac</Author>
    <Description>Starts RandomMac when you sign in.</Description>
  </RegistrationInfo>
  <Triggers>
    <LogonTrigger>
      <Enabled>true</Enabled>
      <UserId>{sid}</UserId>
      <Delay>PT10S</Delay>
    </LogonTrigger>
  </Triggers>
  <Principals>
    <Principal id="Author">
      <UserId>{sid}</UserId>
      <LogonType>InteractiveToken</LogonType>
      <RunLevel>HighestAvailable</RunLevel>
    </Principal>
  </Principals>
  <Settings>
    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>
    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>
    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>
    <AllowHardTerminate>true</AllowHardTerminate>
    <StartWhenAvailable>false</StartWhenAvailable>
    <RunOnlyIfNetworkAvailable>false</RunOnlyIfNetworkAvailable>
    <IdleSettings>
      <StopOnIdleEnd>false</StopOnIdleEnd>
      <RestartOnIdle>false</RestartOnIdle>
    </IdleSettings>
    <AllowStartOnDemand>true</AllowStartOnDemand>
    <Enabled>true</Enabled>
    <Hidden>false</Hidden>
    <RunOnlyIfIdle>false</RunOnlyIfIdle>
    <WakeToRun>false</WakeToRun>
    <ExecutionTimeLimit>PT0S</ExecutionTimeLimit>
    <Priority>5</Priority>
  </Settings>
  <Actions Context="Author">
    <Exec>
      <Command>"{exe}"</Command>
      <Arguments>{args}</Arguments>
    </Exec>
  </Actions>
</Task>
"#,
            sid = xml_escape(sid),
            exe = xml_escape(&exe.to_string_lossy()),
        )
    }

    /// UTF-16LE with a byte order mark, as `schtasks /Create /XML` expects for `encoding="UTF-16"`.
    fn utf16_with_bom(text: &str) -> Vec<u8> {
        let mut bytes = vec![0xff, 0xfe];
        for unit in text.encode_utf16() {
            bytes.extend_from_slice(&unit.to_le_bytes());
        }
        bytes
    }

    /// Command line of a task, from `schtasks /Query /XML` output.
    fn task_command(xml: &str) -> Option<String> {
        let start = xml.find("<Command>")? + "<Command>".len();
        let end = xml[start..].find("</Command>")? + start;
        Some(xml_unescape(xml[start..end].trim().trim_matches('"')))
    }

    /// Reverses [`xml_escape`] (and `&apos;`); `&amp;` last so `&amp;lt;` stays `&lt;`.
    fn xml_unescape(s: &str) -> String {
        s.replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&apos;", "'")
            .replace("&amp;", "&")
    }

    fn query(name: &str) -> Option<String> {
        let out = schtasks(&["/Query", "/TN", name, "/XML"]).ok()?;
        out.success().then_some(out.stdout)
    }

    /// Enables or disables autostart for the current user. Requires elevation.
    pub fn set(enabled: bool, minimized: bool) -> ApiResult<()> {
        if !is_elevated() {
            return Err(ApiError::read_only());
        }
        let name = task_name()?;
        if !enabled {
            if query(&name).is_some() {
                let out = schtasks(&["/Delete", "/TN", &name, "/F"])?;
                if !out.success() {
                    return Err(ApiError::new(
                        "AUTOSTART_FAILED",
                        out.stderr.trim().to_string(),
                    ));
                }
            }
            return Ok(());
        }
        let sid = current_user_sid().ok_or_else(|| ApiError::new("INTERNAL", "no user SID"))?;
        let exe = std::env::current_exe()?;
        // TMP points to an administrators-only folder while elevated (see platform::windows).
        let dir = std::env::temp_dir();
        let path = dir.join(format!(
            "randommac-task-{}.xml",
            randommac_core::random::new_id()
        ));
        std::fs::write(&path, utf16_with_bom(&task_xml(&sid, &exe, minimized)))?;
        let path_str = path.to_string_lossy().into_owned();
        let result = schtasks(&["/Create", "/TN", &name, "/XML", &path_str, "/F"]);
        let _ = std::fs::remove_file(&path);
        let out = result?;
        if out.success() {
            Ok(())
        } else {
            Err(ApiError::new(
                "AUTOSTART_FAILED",
                format!("{}{}", out.stderr.trim(), out.stdout.trim()),
            ))
        }
    }

    /// Whether our task exists and starts the current executable.
    pub fn is_enabled() -> bool {
        let Ok(name) = task_name() else { return false };
        let Some(xml) = query(&name) else {
            return false;
        };
        let Ok(exe) = std::env::current_exe() else {
            return false;
        };
        task_command(&xml).is_some_and(|c| Path::new(&c) == exe)
    }

    /// Re-creates the task if it is missing or points to another executable.
    pub fn heal(enabled: bool, minimized: bool) {
        if enabled && is_elevated() && !is_enabled() {
            if let Err(e) = set(true, minimized) {
                tracing::warn!("could not repair the autostart task: {e}");
            } else {
                tracing::info!("autostart task repaired");
            }
        }
    }

    /// Removes every RandomMac autostart task (uninstaller hook).
    pub fn remove_all() {
        let Ok(out) = schtasks(&["/Query", "/FO", "CSV", "/NH"]) else {
            return;
        };
        for line in out.stdout.lines() {
            let Some(name) = line
                .split(',')
                .next()
                .map(|s| s.trim_matches('"').trim_start_matches('\\'))
            else {
                continue;
            };
            if name.starts_with(TASK_PREFIX) {
                let _ = schtasks(&["/Delete", "/TN", name, "/F"]);
            }
        }
    }

    /// Deletes the elevated logon task created by RandomMac 1.x when it still points into the
    /// user-writable Velopack folder, plus the legacy HKCU Run value. Returns whether something
    /// was removed.
    pub fn remove_legacy_v1() -> bool {
        let mut removed = false;
        if let Some(xml) = query("RandomMac") {
            let points_to_v1 = task_command(&xml).is_some_and(|c| {
                let c = c.to_ascii_lowercase();
                c.contains(r"\appdata\local\randommac\") || c.ends_with("randommac.app.exe")
            });
            if points_to_v1
                && schtasks(&["/Delete", "/TN", "RandomMac", "/F"]).is_ok_and(|o| o.success())
            {
                tracing::info!("removed the RandomMac 1.x logon task");
                removed = true;
            }
        }
        let reg = system32("reg.exe");
        if let Ok(out) = process::run(
            &reg,
            &[
                "delete",
                r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
                "/v",
                "RandomMac",
                "/f",
            ],
            None,
            TIMEOUT,
        ) && out.success()
        {
            removed = true;
        }
        removed
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn xml_contains_settings() {
            let xml = task_xml(
                "S-1-5-21-1",
                Path::new(r"C:\Program Files\RandomMac\randommac.exe"),
                true,
            );
            assert!(xml.contains("<RunLevel>HighestAvailable</RunLevel>"));
            assert!(xml.contains("<DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>"));
            assert!(xml.contains("<ExecutionTimeLimit>PT0S</ExecutionTimeLimit>"));
            assert!(xml.contains("--autostart --minimized"));
            assert_eq!(
                task_command(&xml).as_deref(),
                Some(r"C:\Program Files\RandomMac\randommac.exe")
            );
            assert_eq!(xml_escape("a&b<c>"), "a&amp;b&lt;c&gt;");
        }

        #[test]
        fn xml_escapes_values_and_omits_minimized() {
            let xml = task_xml("S-1-5-21-2", Path::new(r"C:\R&D\randommac.exe"), false);
            assert!(xml.contains("<Arguments>--autostart</Arguments>"));
            assert!(xml.contains(r#"<Command>"C:\R&amp;D\randommac.exe"</Command>"#));
            assert!(xml.contains("<UserId>S-1-5-21-2</UserId>"));
            // is_enabled() compares this with current_exe(), so entities must be decoded.
            assert_eq!(task_command(&xml).as_deref(), Some(r"C:\R&D\randommac.exe"));
            assert_eq!(xml_escape(r#""q""#), "&quot;q&quot;");
            assert_eq!(xml_unescape("&amp;lt;&apos;&gt;"), "&lt;'>");
            let tricky = r#"a&b<c>"d"&lt;"#;
            assert_eq!(xml_unescape(&xml_escape(tricky)), tricky);
        }

        #[test]
        fn task_command_needs_both_tags() {
            assert_eq!(task_command("<Task/>"), None);
            assert_eq!(task_command("<Command>x.exe"), None);
            assert_eq!(
                task_command("<Command> \"C:\\a b\\x.exe\" </Command>").as_deref(),
                Some(r"C:\a b\x.exe")
            );
        }

        #[test]
        fn task_files_are_utf16_with_a_bom() {
            assert_eq!(utf16_with_bom("Aé"), [0xff, 0xfe, 0x41, 0x00, 0xe9, 0x00]);
            let xml = task_xml("S-1-5-21-3", Path::new(r"C:\x.exe"), true);
            let bytes = utf16_with_bom(&xml);
            let units: Vec<u16> = bytes[2..]
                .as_chunks::<2>()
                .0
                .iter()
                .map(|c| u16::from_le_bytes(*c))
                .collect();
            assert_eq!(String::from_utf16(&units).expect("utf-16"), xml);
        }

        #[test]
        fn read_only_queries() {
            assert!(task_name().is_ok_and(|n| n.starts_with(TASK_PREFIX)));
            // Only queries the scheduler; nothing is created or removed.
            let _ = is_enabled();
            heal(false, false);
        }
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use std::path::{Path, PathBuf};

    use super::*;

    fn entry_path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| {
            d.join("autostart")
                .join(format!("{}.desktop", randommac_core::APP_ID))
        })
    }

    fn exec_target() -> ApiResult<String> {
        // AppImages run from a temporary mount; autostart must point at the image itself.
        if let Some(appimage) = std::env::var_os("APPIMAGE") {
            return Ok(appimage.to_string_lossy().into_owned());
        }
        Ok(std::env::current_exe()?.to_string_lossy().into_owned())
    }

    pub fn set(enabled: bool, minimized: bool) -> ApiResult<()> {
        let path = entry_path().ok_or_else(|| ApiError::new("INTERNAL", "no config directory"))?;
        if enabled {
            write_entry(&path, &exec_target()?, minimized)
        } else {
            remove_entry(&path)
        }
    }

    fn remove_entry(path: &Path) -> ApiResult<()> {
        match std::fs::remove_file(path) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.into()),
            _ => Ok(()),
        }
    }

    fn write_entry(path: &Path, program: &str, minimized: bool) -> ApiResult<()> {
        let target = desktop_exec_arg(program).ok_or_else(|| {
            ApiError::new(
                "AUTOSTART_FAILED",
                "the program path cannot be used in an autostart entry",
            )
        })?;
        let exec = format!(
            "{target} --autostart{}",
            if minimized { " --minimized" } else { "" }
        );
        let content = format!(
            "[Desktop Entry]\nType=Application\nName=RandomMac\nComment=Randomize network adapter MAC addresses\nExec={exec}\nIcon=randommac\nTerminal=false\nX-GNOME-Autostart-enabled=true\n"
        );
        randommac_core::storage::write_atomic(path, content.as_bytes())?;
        Ok(())
    }

    pub fn is_enabled() -> bool {
        entry_path().is_some_and(|p| p.is_file())
    }

    pub fn heal(enabled: bool, minimized: bool) {
        if enabled && !is_enabled() {
            let _ = set(true, minimized);
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn writes_and_removes_the_desktop_entry() {
            let dir = tempfile::tempdir().expect("tempdir");
            let path = dir.path().join("autostart").join("randommac.desktop");
            write_entry(&path, "/opt/Random Mac/randommac", true).expect("write");
            let content = std::fs::read_to_string(&path).expect("read");
            assert!(content.starts_with("[Desktop Entry]\nType=Application\n"));
            assert!(
                content.contains("\nExec=\"/opt/Random Mac/randommac\" --autostart --minimized\n"),
                "{content}"
            );
            write_entry(&path, "/usr/bin/randommac", false).expect("rewrite");
            let content = std::fs::read_to_string(&path).expect("read");
            assert!(content.contains("\nExec=\"/usr/bin/randommac\" --autostart\n"));

            remove_entry(&path).expect("remove");
            assert!(!path.exists());
            // Removing a missing entry is not an error.
            remove_entry(&path).expect("remove again");
        }

        #[test]
        fn refuses_paths_a_desktop_entry_cannot_hold() {
            let dir = tempfile::tempdir().expect("tempdir");
            let path = dir.path().join("randommac.desktop");
            let err = write_entry(&path, "/tmp/a\nb", false).expect_err("newline");
            assert_eq!(err.code, "AUTOSTART_FAILED");
            assert!(!path.exists());
        }

        #[test]
        fn removing_fails_for_non_files() {
            let dir = tempfile::tempdir().expect("tempdir");
            assert!(remove_entry(dir.path()).is_err());
        }

        #[test]
        fn entry_lives_in_the_autostart_folder() {
            let path = entry_path().expect("config dir");
            assert!(path.ends_with(format!("autostart/{}.desktop", randommac_core::APP_ID)));
            assert!(exec_target().is_ok_and(|t| !t.is_empty()));
            // Read-only checks; nothing is created.
            let _ = is_enabled();
            heal(false, false);
        }
    }
}
