//! Windows-specific process setup.
//!
//! RandomMac runs elevated (writing the `NetworkAddress` driver property requires it), so the
//! process is hardened before anything else starts:
//! * `WEBVIEW2_*` environment overrides are removed — they can be set by any process of the user
//!   (HKCU\Environment) and would otherwise let it inject a browser executable or debugging
//!   flags into the elevated WebView2;
//! * DLLs are only loaded from System32 and the (admin-protected) install folder;
//! * `TMP`/`TEMP` point to a fresh folder only administrators can write, so files the updater
//!   downloads cannot be swapped by unelevated code before they are executed.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicIsize, Ordering};

use windows_sys::Win32::Foundation as f;
use windows_sys::Win32::Security as sec;
use windows_sys::Win32::Security::Authorization as authz;
use windows_sys::Win32::Storage::FileSystem as fs;
use windows_sys::Win32::System::LibraryLoader as ll;
use windows_sys::Win32::System::Threading as th;
use windows_sys::Win32::UI::Shell as shell;
use windows_sys::Win32::UI::WindowsAndMessaging as wm;

pub use randommac_net::windows::is_elevated;

const ACTIVATION_EVENT: &str = r"Local\io.github.poli0981.randommac.activate";

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn system_root() -> PathBuf {
    std::env::var_os("SystemRoot")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .unwrap_or_else(|| PathBuf::from(r"C:\Windows"))
}

pub fn system32(exe: &str) -> PathBuf {
    system_root().join("System32").join(exe)
}

/// Must run first in `main`, while the process is still single-threaded.
pub fn harden() {
    for var in [
        "WEBVIEW2_BROWSER_EXECUTABLE_FOLDER",
        "WEBVIEW2_USER_DATA_FOLDER",
        "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS",
        "WEBVIEW2_RELEASE_CHANNEL_PREFERENCE",
        "WEBVIEW2_WAIT_FOR_SCRIPT_DEBUGGER",
        "WEBVIEW2_PIPE_FOR_SCRIPT_DEBUGGER",
    ] {
        // SAFETY: called at the very start of main before any other thread exists.
        unsafe { std::env::remove_var(var) };
    }
    // SAFETY: plain flag call.
    unsafe {
        ll::SetDefaultDllDirectories(
            ll::LOAD_LIBRARY_SEARCH_SYSTEM32 | ll::LOAD_LIBRARY_SEARCH_APPLICATION_DIR,
        )
    };
}

/// Security descriptor from SDDL, freed on drop.
struct SecurityDescriptor(sec::PSECURITY_DESCRIPTOR);

impl SecurityDescriptor {
    fn from_sddl(sddl: &str) -> Option<Self> {
        let w = wide(sddl);
        let mut sd: sec::PSECURITY_DESCRIPTOR = null_mut();
        // SAFETY: w is NUL-terminated; sd receives a LocalAlloc'd descriptor.
        let ok = unsafe {
            authz::ConvertStringSecurityDescriptorToSecurityDescriptorW(
                w.as_ptr(),
                authz::SDDL_REVISION_1,
                &mut sd,
                null_mut(),
            )
        };
        (ok != 0 && !sd.is_null()).then_some(Self(sd))
    }

    fn attributes(&self) -> sec::SECURITY_ATTRIBUTES {
        sec::SECURITY_ATTRIBUTES {
            nLength: size_of::<sec::SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: self.0,
            bInheritHandle: 0,
        }
    }
}

impl Drop for SecurityDescriptor {
    fn drop(&mut self) {
        // SAFETY: the descriptor was allocated by ConvertStringSecurityDescriptorToSecurityDescriptorW.
        unsafe { f::LocalFree(self.0) };
    }
}

/// Points `TMP`/`TEMP` at `%SystemRoot%\Temp\RandomMac-<random>` (SYSTEM + Administrators only).
/// Call only when elevated and before other threads start.
pub fn secure_temp_dir() -> Option<PathBuf> {
    let sd = SecurityDescriptor::from_sddl("D:P(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)")?;
    let name = format!("RandomMac-{}", randommac_core::random::new_id());
    let dir = system_root().join("Temp").join(name);
    let w = wide(&dir.to_string_lossy());
    let sa = sd.attributes();
    // SAFETY: w is NUL-terminated and sa references a live descriptor.
    if unsafe { fs::CreateDirectoryW(w.as_ptr(), &sa) } == 0 {
        return None;
    }
    // SAFETY: still single-threaded (see caller).
    unsafe {
        std::env::set_var("TMP", &dir);
        std::env::set_var("TEMP", &dir);
    }
    Some(dir)
}

pub fn remove_temp_dir(dir: &Path) {
    let _ = std::fs::remove_dir_all(dir);
}

#[derive(Debug)]
pub enum ElevationError {
    Cancelled,
    Failed(u32),
}

fn quote_arg(arg: &str) -> String {
    if !arg.is_empty() && !arg.contains([' ', '\t', '"']) {
        return arg.to_string();
    }
    format!("\"{}\"", arg.replace('"', "\\\""))
}

/// Starts this executable again through UAC (`runas`).
pub fn relaunch_elevated(args: &[String]) -> Result<(), ElevationError> {
    let exe = std::env::current_exe().map_err(|_| ElevationError::Failed(0))?;
    let file = wide(&exe.to_string_lossy());
    let params = wide(
        &args
            .iter()
            .map(|a| quote_arg(a))
            .collect::<Vec<_>>()
            .join(" "),
    );
    let verb = wide("runas");
    let mut info = shell::SHELLEXECUTEINFOW {
        cbSize: size_of::<shell::SHELLEXECUTEINFOW>() as u32,
        fMask: shell::SEE_MASK_NOASYNC,
        lpVerb: verb.as_ptr(),
        lpFile: file.as_ptr(),
        lpParameters: params.as_ptr(),
        nShow: wm::SW_SHOWNORMAL,
        ..Default::default()
    };
    // SAFETY: all strings outlive the call; info has the correct cbSize.
    if unsafe { shell::ShellExecuteExW(&mut info) } != 0 {
        return Ok(());
    }
    // SAFETY: plain call.
    match unsafe { f::GetLastError() } {
        f::ERROR_CANCELLED => Err(ElevationError::Cancelled),
        code => Err(ElevationError::Failed(code)),
    }
}

/// Waits (bounded) for a process to exit — used after a self-relaunch so the new instance
/// does not race the old one for the single-instance event.
pub fn wait_for_process(pid: u32, timeout_ms: u32) {
    // SAFETY: OpenProcess with SYNCHRONIZE only; the handle is closed below.
    let h = unsafe { th::OpenProcess(th::PROCESS_SYNCHRONIZE, 0, pid) };
    if h.is_null() {
        return;
    }
    // SAFETY: h is a valid process handle.
    unsafe {
        th::WaitForSingleObject(h, timeout_ms);
        f::CloseHandle(h);
    }
}

static ACTIVATION_HANDLE: AtomicIsize = AtomicIsize::new(0);

/// If another instance runs in this session, asks it to show its window and returns `true`.
/// Works across integrity levels: the event created by an elevated instance grants
/// "modify state" to authenticated users and carries a medium mandatory label, so a normal
/// Start-menu launch can wake it without a UAC prompt.
pub fn signal_existing_instance() -> bool {
    let name = wide(ACTIVATION_EVENT);
    // SAFETY: name is NUL-terminated.
    let h = unsafe { th::OpenEventW(th::EVENT_MODIFY_STATE, 0, name.as_ptr()) };
    if h.is_null() {
        // SAFETY: plain call.
        return unsafe { f::GetLastError() } == f::ERROR_ACCESS_DENIED;
    }
    // SAFETY: h is a valid event handle.
    unsafe {
        th::SetEvent(h);
        f::CloseHandle(h);
    }
    true
}

/// Creates the activation event and calls `on_activate` whenever another launch signals it.
/// Returns `false` if another instance won the race (it has been signalled instead).
pub fn register_instance(on_activate: impl Fn() + Send + 'static) -> bool {
    let sd = SecurityDescriptor::from_sddl(
        "D:(A;;GA;;;SY)(A;;GA;;;BA)(A;;0x00100002;;;AU)S:(ML;;NW;;;ME)",
    );
    let sa = sd.as_ref().map(SecurityDescriptor::attributes);
    let name = wide(ACTIVATION_EVENT);
    // SAFETY: name is NUL-terminated; sa (if any) references a live descriptor.
    let h = unsafe {
        th::CreateEventW(
            sa.as_ref()
                .map_or(null(), |s| s as *const sec::SECURITY_ATTRIBUTES),
            0,
            0,
            name.as_ptr(),
        )
    };
    drop(sd);
    if h.is_null() {
        return true; // cannot coordinate; run anyway
    }
    // SAFETY: plain call.
    if unsafe { f::GetLastError() } == f::ERROR_ALREADY_EXISTS {
        // SAFETY: h is valid.
        unsafe {
            th::SetEvent(h);
            f::CloseHandle(h);
        }
        return false;
    }
    ACTIVATION_HANDLE.store(h as isize, Ordering::SeqCst);
    let raw = h as isize;
    std::thread::spawn(move || {
        loop {
            // SAFETY: the handle stays open for the life of the process.
            let r = unsafe { th::WaitForSingleObject(raw as f::HANDLE, th::INFINITE) };
            if r != f::WAIT_OBJECT_0 {
                break;
            }
            on_activate();
        }
    });
    true
}

/// Releases the activation event early (before relaunching elevated).
pub fn release_instance() {
    let h = ACTIVATION_HANDLE.swap(0, Ordering::SeqCst);
    if h != 0 {
        // SAFETY: h came from CreateEventW and is closed once.
        unsafe { f::CloseHandle(h as f::HANDLE) };
    }
}

/// Opens a URL or folder through Explorer. When this process is elevated, Explorer hands the
/// request to the already running (unelevated) shell, so browsers never start elevated.
pub fn open_with_shell(target: &str) -> std::io::Result<()> {
    Command::new(system_root().join("explorer.exe"))
        .arg(target)
        .spawn()
        .map(|_| ())
}

/// String SID of the current user (for per-user task names).
pub fn current_user_sid() -> Option<String> {
    let mut token: f::HANDLE = null_mut();
    // SAFETY: token receives a handle we close below.
    if unsafe { th::OpenProcessToken(th::GetCurrentProcess(), sec::TOKEN_QUERY, &mut token) } == 0 {
        return None;
    }
    let mut buf = vec![0u8; 256];
    let mut len = 0u32;
    // SAFETY: buf is large enough for TOKEN_USER + SID (checked via len).
    let ok = unsafe {
        sec::GetTokenInformation(
            token,
            sec::TokenUser,
            buf.as_mut_ptr().cast(),
            buf.len() as u32,
            &mut len,
        )
    };
    let mut result = None;
    if ok != 0 {
        // SAFETY: buf now holds a TOKEN_USER.
        let user = unsafe { &*(buf.as_ptr() as *const sec::TOKEN_USER) };
        let mut s: windows_sys::core::PWSTR = null_mut();
        // SAFETY: the SID pointer is inside buf; s is LocalAlloc'd by the API.
        if unsafe { authz::ConvertSidToStringSidW(user.User.Sid, &mut s) } != 0 && !s.is_null() {
            let mut n = 0;
            // SAFETY: s is NUL-terminated.
            while unsafe { *s.add(n) } != 0 {
                n += 1;
            }
            // SAFETY: s has n valid UTF-16 units.
            result = Some(String::from_utf16_lossy(unsafe {
                std::slice::from_raw_parts(s, n)
            }));
            // SAFETY: allocated by ConvertSidToStringSidW.
            unsafe { f::LocalFree(s.cast()) };
        }
    }
    // SAFETY: token is valid.
    unsafe { f::CloseHandle(token) };
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quoting() {
        assert_eq!(quote_arg("--autostart"), "--autostart");
        assert_eq!(quote_arg("a b"), "\"a b\"");
        assert_eq!(quote_arg(""), "\"\"");
    }

    #[test]
    fn sid_and_paths() {
        let sid = current_user_sid().expect("sid");
        assert!(sid.starts_with("S-1-"));
        assert!(system32("schtasks.exe").is_absolute());
        assert!(SecurityDescriptor::from_sddl("D:P(A;OICI;FA;;;SY)").is_some());
    }
}
