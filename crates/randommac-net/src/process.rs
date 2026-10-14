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
        let mut s = String::new();
        if let Some(p) = stdout.as_mut() {
            let _ = p.read_to_string(&mut s);
        }
        s
    });
    let err_reader = thread::spawn(move || {
        let mut s = String::new();
        if let Some(p) = stderr.as_mut() {
            let _ = p.read_to_string(&mut s);
        }
        s
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_relative_programs() {
        let err = run(Path::new("ip"), &[], None, Duration::from_secs(1)).expect_err("relative");
        assert_eq!(err.code(), "INVALID_INPUT");
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
    #[test]
    fn runs_system_programs() {
        let root = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into());
        let cmd = PathBuf::from(root).join("System32").join("cmd.exe");
        let out = run(&cmd, &["/C", "echo hi"], None, Duration::from_secs(10)).expect("run");
        assert!(out.success());
        assert_eq!(out.stdout.trim(), "hi");
    }
}
