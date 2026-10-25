//! Logging: an in-memory ring buffer for the Log page plus a size-capped rotating file.
//!
//! `tauri-plugin-log` is not used because its rotation deletes files inside a folder the user
//! can write to while the app may run elevated; here only `randommac.<n>.log` names are touched.

use std::collections::VecDeque;
use std::fmt::Write as _;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use serde::Serialize;
use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::{Context, Layer, SubscriberExt};
use tracing_subscriber::util::SubscriberInitExt;

const CAPACITY: usize = 2000;
const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;
const KEEP_FILES: usize = 5;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub seq: u64,
    pub timestamp: String,
    pub level: String,
    pub target: String,
    pub message: String,
}

#[derive(Default)]
pub struct LogBuffer {
    entries: Mutex<VecDeque<LogEntry>>,
    seq: AtomicU64,
}

impl LogBuffer {
    pub fn push(&self, level: &str, target: &str, message: String) -> LogEntry {
        let entry = LogEntry {
            seq: self.seq.fetch_add(1, Ordering::Relaxed) + 1,
            timestamp: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            level: level.to_string(),
            target: target.to_string(),
            message,
        };
        if let Ok(mut q) = self.entries.lock() {
            if q.len() >= CAPACITY {
                q.pop_front();
            }
            q.push_back(entry.clone());
        }
        entry
    }

    /// Entries with a sequence number greater than `after` (all when `None`).
    pub fn since(&self, after: Option<u64>) -> Vec<LogEntry> {
        self.entries
            .lock()
            .map(|q| {
                q.iter()
                    .filter(|e| after.is_none_or(|s| e.seq > s))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn clear(&self) {
        if let Ok(mut q) = self.entries.lock() {
            q.clear();
        }
    }
}

/// Appends to `randommac.log`, rotating to `randommac.1.log` … `randommac.5.log`.
pub struct RotatingFile {
    dir: PathBuf,
    state: Mutex<Option<(File, u64)>>,
}

impl RotatingFile {
    pub fn new(dir: &Path) -> Self {
        Self {
            dir: dir.to_path_buf(),
            state: Mutex::new(None),
        }
    }

    fn path(&self, index: usize) -> PathBuf {
        if index == 0 {
            self.dir.join("randommac.log")
        } else {
            self.dir.join(format!("randommac.{index}.log"))
        }
    }

    fn open(&self) -> Option<(File, u64)> {
        randommac_core::storage::ensure_dir(&self.dir).ok()?;
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.path(0))
            .ok()?;
        let len = file.metadata().map(|m| m.len()).unwrap_or(0);
        Some((file, len))
    }

    fn rotate(&self) {
        let _ = fs::remove_file(self.path(KEEP_FILES));
        for i in (0..KEEP_FILES).rev() {
            let from = self.path(i);
            if from.exists() {
                let _ = fs::rename(&from, self.path(i + 1));
            }
        }
    }

    pub fn write_line(&self, line: &str) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        if state.is_none() {
            *state = self.open();
        }
        if state
            .as_ref()
            .is_some_and(|(_, len)| *len >= MAX_FILE_BYTES)
        {
            *state = None;
            self.rotate();
            *state = self.open();
        }
        if let Some((file, len)) = state.as_mut()
            && file.write_all(line.as_bytes()).is_ok()
        {
            *len += line.len() as u64;
        }
    }
}

#[derive(Default)]
struct MessageVisitor {
    message: String,
    fields: String,
}

impl Visit for MessageVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message.push_str(value);
        } else {
            let _ = write!(self.fields, " {}={}", field.name(), value);
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            let _ = write!(self.message, "{value:?}");
        } else {
            let _ = write!(self.fields, " {}={:?}", field.name(), value);
        }
    }
}

struct AppLayer {
    buffer: Arc<LogBuffer>,
    file: Option<Arc<RotatingFile>>,
}

impl<S: Subscriber> Layer<S> for AppLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let meta = event.metadata();
        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);
        let message = format!("{}{}", visitor.message, visitor.fields);
        let level = match *meta.level() {
            Level::ERROR => "error",
            Level::WARN => "warn",
            Level::INFO => "info",
            Level::DEBUG => "debug",
            Level::TRACE => "trace",
        };
        let entry = self.buffer.push(level, meta.target(), message);
        if let Some(file) = &self.file {
            file.write_line(&format!(
                "{} {:<5} {}: {}\n",
                entry.timestamp,
                level.to_uppercase(),
                entry.target,
                entry.message
            ));
        }
    }
}

/// Installs the global subscriber. Safe to call once; later calls are ignored.
pub fn init(log_dir: Option<&Path>) -> Arc<LogBuffer> {
    let buffer = Arc::new(LogBuffer::default());
    let default_filter = if cfg!(debug_assertions) {
        "debug,tao=info,wry=info,tauri=info,hyper=info,reqwest=info,zbus=info"
    } else {
        "info,tao=warn,wry=warn,hyper=warn,reqwest=warn,zbus=warn"
    };
    let filter =
        EnvFilter::try_from_env("RANDOMMAC_LOG").unwrap_or_else(|_| EnvFilter::new(default_filter));
    let layer = AppLayer {
        buffer: buffer.clone(),
        file: log_dir.map(|d| Arc::new(RotatingFile::new(d))),
    };
    let stderr = cfg!(debug_assertions)
        .then(|| tracing_subscriber::fmt::layer().with_writer(std::io::stderr));
    let _ = tracing_subscriber::registry()
        .with(filter)
        .with(layer)
        .with(stderr)
        .try_init();
    std::panic::set_hook(Box::new(|info| {
        tracing::error!(target: "randommac::panic", "{info}");
        eprintln!("{info}");
    }));
    buffer
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buffer_is_bounded_and_ordered() {
        let b = LogBuffer::default();
        for i in 0..(CAPACITY + 10) {
            b.push("info", "t", format!("m{i}"));
        }
        let all = b.since(None);
        assert_eq!(all.len(), CAPACITY);
        assert_eq!(all[0].message, "m10");
        let last_seq = all.last().map(|e| e.seq).expect("entries");
        assert!(b.since(Some(last_seq)).is_empty());
        assert_eq!(b.since(Some(last_seq - 2)).len(), 2);
        b.clear();
        assert!(b.since(None).is_empty());
    }

    #[test]
    fn layer_writes_events_to_the_buffer_and_the_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let buffer = Arc::new(LogBuffer::default());
        let layer = AppLayer {
            buffer: buffer.clone(),
            file: Some(Arc::new(RotatingFile::new(dir.path()))),
        };
        tracing::subscriber::with_default(tracing_subscriber::registry().with(layer), || {
            tracing::info!(target: "randommac::test", adapter = "Wi-Fi", "changed");
            tracing::warn!(code = 5, "slow {}", "driver");
            tracing::error!(message = "plain");
            tracing::debug!("details");
            tracing::trace!("noise");
        });
        let entries = buffer.since(None);
        let levels: Vec<&str> = entries.iter().map(|e| e.level.as_str()).collect();
        assert_eq!(levels, ["info", "warn", "error", "debug", "trace"]);
        assert_eq!(entries[0].target, "randommac::test");
        assert_eq!(entries[0].message, "changed adapter=Wi-Fi");
        assert_eq!(entries[1].message, "slow driver code=5");
        assert_eq!(entries[2].message, "plain");
        assert!(entries.windows(2).all(|w| w[0].seq < w[1].seq));

        let file = fs::read_to_string(dir.path().join("randommac.log")).expect("log file");
        assert_eq!(file.lines().count(), 5, "{file}");
        assert!(
            file.contains("INFO  randommac::test: changed adapter=Wi-Fi"),
            "{file}"
        );
    }

    #[test]
    fn layer_without_a_file_only_buffers() {
        let buffer = Arc::new(LogBuffer::default());
        let layer = AppLayer {
            buffer: buffer.clone(),
            file: None,
        };
        tracing::subscriber::with_default(tracing_subscriber::registry().with(layer), || {
            tracing::info!(text = "quoted", "hello");
        });
        assert_eq!(buffer.since(None)[0].message, "hello text=quoted");
    }

    #[test]
    fn unwritable_log_folder_is_ignored() {
        let dir = tempfile::tempdir().expect("tempdir");
        let file = dir.path().join("not-a-folder");
        fs::write(&file, b"").expect("write");
        let f = RotatingFile::new(&file);
        f.write_line("lost\n");
        assert_eq!(fs::read(&file).expect("read"), b"");
    }

    #[test]
    fn init_returns_a_buffer_that_receives_events() {
        let dir = tempfile::tempdir().expect("tempdir");
        let buffer = init(Some(dir.path()));
        // Restore the default panic hook for the other tests.
        let _ = std::panic::take_hook();
        tracing::warn!(target: "randommac::init_test", "hello from init");
        assert!(
            buffer
                .since(None)
                .iter()
                .any(|e| e.target == "randommac::init_test" && e.message == "hello from init")
        );
        // A second call keeps the first subscriber.
        let second = init(None);
        let _ = std::panic::take_hook();
        tracing::warn!(target: "randommac::init_test", "again");
        assert!(second.since(None).is_empty());
    }

    #[test]
    fn file_rotates() {
        let dir = tempfile::tempdir().expect("tempdir");
        let f = RotatingFile::new(dir.path());
        let line = "x".repeat(1024) + "\n";
        for _ in 0..(3 * 1024 * 3) {
            f.write_line(&line);
        }
        assert!(dir.path().join("randommac.log").exists());
        assert!(dir.path().join("randommac.1.log").exists());
        assert!(!dir.path().join("randommac.6.log").exists());
    }
}
