//! Cross-process lock that serializes MAC changes between the GUI and the CLI.

use std::fs::{File, OpenOptions, TryLockError};
use std::io;
use std::path::Path;
use std::time::{Duration, Instant};

/// Held for the duration of a MAC change; released on drop.
#[derive(Debug)]
pub struct OperationLock {
    file: File,
}

impl OperationLock {
    /// Waits up to `timeout` for the lock.
    pub fn acquire(path: &Path, timeout: Duration) -> io::Result<Self> {
        if let Some(dir) = path.parent() {
            crate::storage::ensure_dir(dir)?;
        }
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(path)?;
        let deadline = Instant::now() + timeout;
        loop {
            match file.try_lock() {
                Ok(()) => return Ok(Self { file }),
                Err(TryLockError::WouldBlock) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(100));
                }
                Err(TryLockError::WouldBlock) => {
                    return Err(io::Error::new(
                        io::ErrorKind::WouldBlock,
                        "another RandomMac process is changing a MAC address",
                    ));
                }
                Err(TryLockError::Error(e)) => return Err(e),
            }
        }
    }
}

impl Drop for OperationLock {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exclusive_until_dropped() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("op.lock");
        let first = OperationLock::acquire(&path, Duration::from_millis(10)).expect("first");
        let second = OperationLock::acquire(&path, Duration::from_millis(200));
        assert_eq!(second.expect_err("held").kind(), io::ErrorKind::WouldBlock);
        drop(first);
        OperationLock::acquire(&path, Duration::from_millis(10)).expect("released");
    }
}
