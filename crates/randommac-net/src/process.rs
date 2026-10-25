//! Running external programs safely: absolute paths only, no shell, a scrubbed environment on
//! Unix (AppImage leaks `LD_LIBRARY_PATH` & co. into children), output captured, and a timeout.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use crate::error::NetError;

/// `PATH` given to child processes on Unix.
pub const SAFE_PATH: &str = "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Output {
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

impl Output {
    pub fn success(&self) -> bool {
        self.code == Some(0)
    }
}

/// Returns the first candidate that exists as a file.
pub fn find_program(candidates: &[&str]) -> Option<PathBuf> {
    candidates.iter().map(PathBuf::from).find(|p| p.is_file())
}

/// Runs `program` (an absolute path) with `args`, optionally feeding `stdin`.
pub fn run(
    program: &Path,
    args: &[&str],
    stdin: Option<&str>,
    timeout: Duration,
) -> Result<Output, NetError> {
    let name = program.display().to_string();
    if !program.is_absolute() {
        return Err(NetError::InvalidInput(format!(
            "refusing to run {name}: not an absolute path"
        )));
    }
    let mut cmd = Command::new(program);
    cmd.args(args)
        .stdin(if stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(unix)]
    {
        cmd.env_clear().env("PATH", SAFE_PATH).env("LC_ALL", "C");
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let mut child = cmd.spawn().map_err(|e| NetError::CommandFailed {
        program: name.clone(),
        code: None,
        stderr: e.to_string(),
    })?;
    if let Some(input) = stdin
        && let Some(mut pipe) = child.stdin.take()
    {
        // Small scripts fit in the pipe buffer; dropping the handle closes stdin.
        pipe.write_all(input.as_bytes())?;
    }
    let mut stdout = child.stdout.take();
    let mut stderr = child.stderr.take();
    let out_reader = thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(p) = stdout.as_mut() {
            let _ = p.read_to_end(&mut buf);
        }
        decode(buf)
    });
    let err_reader = thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(p) = stderr.as_mut() {
            let _ = p.read_to_end(&mut buf);
        }
        decode(buf)
    });
    let deadline = Instant::now() + timeout;
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(NetError::Timeout(name));
        }
        thread::sleep(Duration::from_millis(25));
    };
    Ok(Output {
        code: status.code(),
        stdout: out_reader.join().unwrap_or_default(),
        stderr: err_reader.join().unwrap_or_default(),
    })
}

/// Decodes captured output. Text is UTF-8 when valid; otherwise, on Windows, it is in the OEM
/// code page that console programs such as `schtasks.exe` use for pipes (non-ASCII user names,
/// localized task names). Lossy UTF-8 is the last resort, so ASCII parts always survive.
fn decode(bytes: Vec<u8>) -> String {
    match String::from_utf8(bytes) {
        Ok(text) => text,
        Err(e) => {
            let bytes = e.into_bytes();
            #[cfg(windows)]
            if let Some(text) = decode_oem(&bytes) {
                return text;
            }
            String::from_utf8_lossy(&bytes).into_owned()
        }
    }
}

#[cfg(windows)]
fn decode_oem(bytes: &[u8]) -> Option<String> {
    use windows_sys::Win32::Globalization::{CP_OEMCP, MultiByteToWideChar};

    let len = i32::try_from(bytes.len()).ok()?;
    if len == 0 {
        return Some(String::new());
    }
    // SAFETY: the input pointer and length describe `bytes`; a null output buffer only asks for
    // the required size.
    let needed =
        unsafe { MultiByteToWideChar(CP_OEMCP, 0, bytes.as_ptr(), len, std::ptr::null_mut(), 0) };
    let capacity = usize::try_from(needed).ok().filter(|&n| n > 0)?;
    let mut wide = vec![0u16; capacity];
    // SAFETY: `wide` has room for exactly `needed` UTF-16 units.
    let written =
        unsafe { MultiByteToWideChar(CP_OEMCP, 0, bytes.as_ptr(), len, wide.as_mut_ptr(), needed) };
    wide.truncate(usize::try_from(written).ok().filter(|&n| n > 0)?);
    Some(String::from_utf16_lossy(&wide))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_non_utf8_output_without_losing_ascii() {
        assert_eq!(decode(b"plain".to_vec()), "plain");
        let vietnamese = "Tiếng Việt";
        assert_eq!(decode(vietnamese.as_bytes().to_vec()), vietnamese);
        // A 1.x task path for a user named "José" as written by schtasks in a single-byte code
        // page: invalid UTF-8, but the parts RandomMac looks for must survive.
        let mut raw = br"C:\Users\Jos".to_vec();
        raw.push(0xE9);
        raw.extend_from_slice(br"\AppData\Local\RandomMac\current\RandomMac.App.exe");
        let text = decode(raw);
        assert!(text.starts_with(r"C:\Users\Jos"), "{text}");
        assert!(text.contains(r"\AppData\Local\RandomMac\"), "{text}");
        assert!(text.ends_with("RandomMac.App.exe"), "{text}");
    }

    #[test]
    fn rejects_relative_programs() {
        let err = run(Path::new("ip"), &[], None, Duration::from_secs(1)).expect_err("relative");
        assert_eq!(err.code(), "INVALID_INPUT");
    }

    #[test]
    fn reports_programs_that_cannot_start() {
        let dir = tempfile::tempdir().expect("tempdir");
        let missing = dir.path().join("missing-program");
        let err = run(&missing, &[], Some("input"), Duration::from_secs(1)).expect_err("missing");
        assert_eq!(err.code(), "COMMAND_FAILED");
        assert!(err.to_string().contains("missing-program"), "{err}");
    }

    #[test]
    fn finds_the_first_existing_program() {
        let dir = tempfile::tempdir().expect("tempdir");
        let present = dir.path().join("present");
        std::fs::write(&present, b"").expect("write");
        let absent = dir.path().join("absent");
        let (absent, present) = (
            absent.to_str().expect("utf-8"),
            present.to_str().expect("utf-8"),
        );
        assert_eq!(
            find_program(&[absent, present]),
            Some(PathBuf::from(present))
        );
        assert_eq!(find_program(&[absent]), None);
        // Directories do not count as programs.
        assert_eq!(find_program(&[dir.path().to_str().expect("utf-8")]), None);
        assert!(
            Output {
                code: Some(0),
                stdout: String::new(),
                stderr: String::new(),
            }
            .success()
        );
    }

    #[cfg(unix)]
    #[test]
    fn captures_output_and_stdin() {
        let sh = find_program(&["/bin/sh", "/usr/bin/sh"]).expect("sh");
        let out = run(
            &sh,
            &["-c", "cat; echo err >&2; echo \"$PATH\"; exit 3"],
            Some("hello\n"),
            Duration::from_secs(5),
        )
        .expect("run");
        assert_eq!(out.code, Some(3));
        assert!(out.stdout.starts_with("hello\n"));
        assert!(out.stdout.contains(SAFE_PATH));
        assert_eq!(out.stderr.trim(), "err");
    }

    #[cfg(unix)]
    #[test]
    fn times_out() {
        let sh = find_program(&["/bin/sh", "/usr/bin/sh"]).expect("sh");
        let err =
            run(&sh, &["-c", "sleep 5"], None, Duration::from_millis(200)).expect_err("timeout");
        assert_eq!(err.code(), "TIMEOUT");
    }

    #[cfg(windows)]
    fn system32(program: &str) -> PathBuf {
        let root = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into());
        PathBuf::from(root).join("System32").join(program)
    }

    #[cfg(windows)]
    #[test]
    fn runs_system_programs() {
        let cmd = system32("cmd.exe");
        let out = run(&cmd, &["/C", "echo hi"], None, Duration::from_secs(10)).expect("run");
        assert!(out.success());
        assert_eq!(out.stdout.trim(), "hi");
    }

    #[cfg(windows)]
    #[test]
    fn feeds_stdin_and_captures_errors() {
        let out = run(
            &system32("sort.exe"),
            &[],
            Some("pear\r\napple\r\n"),
            Duration::from_secs(10),
        )
        .expect("run");
        assert!(out.success());
        assert_eq!(out.stdout.lines().collect::<Vec<_>>(), ["apple", "pear"]);

        let cmd = system32("cmd.exe");
        let out = run(
            &cmd,
            &["/C", "echo oops 1>&2 & exit 3"],
            None,
            Duration::from_secs(10),
        )
        .expect("run");
        assert_eq!(out.code, Some(3));
        assert!(!out.success());
        assert_eq!(out.stderr.trim(), "oops");
    }

    #[cfg(windows)]
    #[test]
    fn times_out() {
        let err = run(
            &system32("ping.exe"),
            &["-n", "30", "127.0.0.1"],
            None,
            Duration::from_millis(300),
        )
        .expect_err("timeout");
        assert_eq!(err.code(), "TIMEOUT");
    }

    #[cfg(windows)]
    #[test]
    fn decodes_empty_oem_output() {
        assert_eq!(decode_oem(&[]).as_deref(), Some(""));
        assert_eq!(decode_oem(b"abc").as_deref(), Some("abc"));
    }
}
