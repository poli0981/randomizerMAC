//! High-level MAC operations shared by the GUI and the CLI.
//!
//! Each operation takes the cross-process [`OperationLock`], re-reads the adapter from the OS,
//! validates the request against the blacklist, records the adapter's original configuration
//! *before* touching it, runs the backend and appends a history entry — so the GUI and the CLI
//! can never interleave changes or lose history (v1 bugs M1/M2/M7).

use std::io;
use std::time::Duration;

use chrono::Utc;
use randommac_core::blacklist::BlockReason;
use randommac_core::history::HistoryPolicy;
use randommac_core::lock::OperationLock;
use randommac_core::storage::{self, LoadStatus};
use randommac_core::{
    AppPaths, AppState, Blacklist, ChangeSource, ChangeStatus, History, HistoryAction,
    HistoryEntry, MacAddress, Settings, random::new_id,
};

use crate::{Adapter, ChangeOptions, ChangeReport, NetError, NetworkBackend, Stage};

const LOCK_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, thiserror::Error)]
pub enum OpError {
    #[error(transparent)]
    Net(#[from] NetError),
    #[error("{0} is not a usable unicast address")]
    NotAssignable(MacAddress),
    #[error("{mac} is blocked by the blacklist ({reason})")]
    Blacklisted { mac: MacAddress, reason: String },
    #[error("another MAC change is in progress")]
    Busy,
    #[error("storage error: {0}")]
    Storage(String),
}

impl OpError {
    pub fn code(&self) -> &'static str {
        match self {
            OpError::Net(e) => e.code(),
            OpError::NotAssignable(_) => "INVALID_MAC",
            OpError::Blacklisted { .. } => "BLACKLISTED",
            OpError::Busy => "BUSY",
            OpError::Storage(_) => "STORAGE_ERROR",
        }
    }
}

impl From<io::Error> for OpError {
    fn from(e: io::Error) -> Self {
        if e.kind() == io::ErrorKind::WouldBlock {
            OpError::Busy
        } else {
            OpError::Storage(e.to_string())
        }
    }
}

/// File-backed data shared by every RandomMac process.
#[derive(Debug, Clone)]
pub struct DataStore {
    pub paths: AppPaths,
}

impl DataStore {
    pub fn new(paths: AppPaths) -> Self {
        Self { paths }
    }

    pub fn load_settings(&self) -> (Settings, LoadStatus) {
        storage::load_with(&self.paths.settings_file(), |text| {
            Settings::from_json(text).map_err(|e| e.to_string())
        })
    }

    pub fn save_settings(&self, settings: &Settings) -> io::Result<()> {
        storage::save_json(&self.paths.settings_file(), settings)
    }

    pub fn load_history(&self) -> (History, LoadStatus) {
        storage::load_json(&self.paths.history_file())
    }

    pub fn save_history(&self, history: &History) -> io::Result<()> {
        storage::save_json(&self.paths.history_file(), history)
    }

    pub fn load_blacklist(&self) -> (Blacklist, LoadStatus) {
        let (mut bl, status): (Blacklist, LoadStatus) =
            storage::load_json(&self.paths.blacklist_file());
        bl.normalize();
        (bl, status)
    }

    pub fn save_blacklist(&self, blacklist: &Blacklist) -> io::Result<()> {
        storage::save_json(&self.paths.blacklist_file(), blacklist)
    }

    pub fn load_state(&self) -> (AppState, LoadStatus) {
        storage::load_json(&self.paths.state_file())
    }

    pub fn save_state(&self, state: &AppState) -> io::Result<()> {
        storage::save_json(&self.paths.state_file(), state)
    }

    /// Appends one entry under the retention policy.
    pub fn append_history(&self, entry: HistoryEntry, policy: HistoryPolicy) -> io::Result<()> {
        let (mut history, _) = self.load_history();
        history.push(entry, policy, Utc::now());
        self.save_history(&history)
    }

    pub fn lock(&self) -> Result<OperationLock, OpError> {
        Ok(OperationLock::acquire(
            &self.paths.lock_file(),
            LOCK_TIMEOUT,
        )?)
    }
}

/// Result of an apply/restore that reached the backend.
#[derive(Debug, Clone)]
pub struct Outcome {
    pub adapter: Adapter,
    pub report: Result<ChangeReport, NetError>,
    pub entry: HistoryEntry,
}

impl Outcome {
    pub fn succeeded(&self) -> bool {
        self.report.as_ref().is_ok_and(|r| r.status.is_success())
    }
}

fn describe_block(reason: BlockReason) -> String {
    match reason {
        BlockReason::Reserved => "reserved address".into(),
        BlockReason::Entry { pattern, .. } => format!("rule {pattern}"),
    }
}

fn history_entry(
    adapter: &Adapter,
    action: HistoryAction,
    source: ChangeSource,
    requested: Option<MacAddress>,
    report: &Result<ChangeReport, NetError>,
) -> HistoryEntry {
    let (status, previous, new_mac, method, error_code, error) = match report {
        Ok(r) => (
            r.status,
            r.previous_mac,
            r.status
                .is_success()
                .then_some(r.observed_mac.or(r.requested_mac))
                .flatten(),
            Some(r.method.as_str().to_string()),
            r.detail_code.clone(),
            None,
        ),
        Err(e) => (
            ChangeStatus::Failed,
            adapter.current_mac,
            None,
            None,
            Some(e.code().to_string()),
            Some(e.to_string()),
        ),
    };
    HistoryEntry {
        id: new_id(),
        timestamp: Utc::now(),
        adapter_id: adapter.id.clone(),
        adapter_name: adapter.name.clone(),
        action,
        source,
        status,
        previous_mac: previous,
        requested_mac: requested.or_else(|| report.as_ref().ok().and_then(|r| r.requested_mac)),
        new_mac,
        method,
        error_code,
        error,
    }
}

/// Validates `mac` for `adapter` (assignable and not blacklisted).
pub fn validate(
    store: &DataStore,
    adapter_id: &str,
    mac: MacAddress,
    ignore_blacklist: bool,
) -> Result<(), OpError> {
    if !mac.is_assignable() {
        return Err(OpError::NotAssignable(mac));
    }
    // Reserved addresses stay blocked even when the user's rules are ignored.
    if randommac_core::blacklist::RESERVED.contains(&mac) {
        return Err(OpError::Blacklisted {
            mac,
            reason: describe_block(BlockReason::Reserved),
        });
    }
    if !ignore_blacklist {
        let (bl, _) = store.load_blacklist();
        if let Some(reason) = bl.check(mac, Some(adapter_id)) {
            return Err(OpError::Blacklisted {
                mac,
                reason: describe_block(reason),
            });
        }
    }
    Ok(())
}

/// Applies `mac` to the adapter. Validation errors and lock contention are returned as `Err`;
/// backend failures are recorded in history and returned inside the [`Outcome`].
#[allow(clippy::too_many_arguments)]
pub fn apply_mac(
    backend: &dyn NetworkBackend,
    store: &DataStore,
    settings: &Settings,
    adapter_id: &str,
    mac: MacAddress,
    source: ChangeSource,
    ignore_blacklist: bool,
    progress: &mut dyn FnMut(Stage),
) -> Result<Outcome, OpError> {
    let _lock = store.lock()?;
    let adapter = backend.find_adapter(adapter_id)?;
    validate(store, &adapter.id, mac, ignore_blacklist)?;

    // Remember what the adapter looked like before our first change (crash-safe: saved first).
    let (mut state, _) = store.load_state();
    let remembered = state.remember(
        &adapter.id,
        adapter.current_mac,
        adapter
            .nm_connection
            .as_ref()
            .map(|c| (c.uuid.as_str(), c.cloned_mac.as_str())),
        Utc::now(),
    );
    if !remembered.is_empty() {
        store.save_state(&state)?;
    }

    let opts = ChangeOptions {
        linux_method: settings.linux.method,
        ..ChangeOptions::default()
    };
    let report = backend.apply(&adapter, mac, &opts, progress);

    // Forget the new record only when we are sure the adapter is exactly as before; a failed
    // rollback or an error after the configuration was written must stay restorable.
    let nothing_changed = match &report {
        Ok(r) => r.status == ChangeStatus::RolledBack,
        Err(NetError::RollbackFailed(_)) => false,
        Err(_) => backend
            .find_adapter(&adapter.id)
            .is_ok_and(|now| same_configuration(&adapter, &now)),
    };
    if !remembered.is_empty() && nothing_changed {
        state.forget(&adapter.id, &remembered);
        store.save_state(&state)?;
    }

    let entry = history_entry(&adapter, HistoryAction::Apply, source, Some(mac), &report);
    store.append_history(entry.clone(), settings.history.policy())?;
    Ok(Outcome {
        adapter,
        report,
        entry,
    })
}

/// Undoes RandomMac's changes on the adapter.
pub fn restore_mac(
    backend: &dyn NetworkBackend,
    store: &DataStore,
    settings: &Settings,
    adapter_id: &str,
    source: ChangeSource,
    progress: &mut dyn FnMut(Stage),
) -> Result<Outcome, OpError> {
    let _lock = store.lock()?;
    let adapter = backend.find_adapter(adapter_id)?;
    let (mut state, _) = store.load_state();
    let original = state.original(&adapter.id).cloned();
    let opts = ChangeOptions {
        linux_method: settings.linux.method,
        ..ChangeOptions::default()
    };
    let report = backend.restore(&adapter, original.as_ref(), &opts, progress);
    if report.as_ref().is_ok_and(|r| r.status.is_success()) && original.is_some() {
        state.clear_original(&adapter.id);
        store.save_state(&state)?;
    }
    let entry = history_entry(&adapter, HistoryAction::Restore, source, None, &report);
    store.append_history(entry.clone(), settings.history.policy())?;
    Ok(Outcome {
        adapter,
        report,
        entry,
    })
}

/// Whether two snapshots of an adapter have the same address and the same override (Windows
/// `NetworkAddress`, NetworkManager `cloned-mac-address`).
fn same_configuration(before: &Adapter, after: &Adapter) -> bool {
    let profile = |a: &Adapter| {
        a.nm_connection
            .as_ref()
            .map(|c| (c.uuid.clone(), c.cloned_mac.clone()))
    };
    before.current_mac == after.current_mac
        && before.override_value == after.override_value
        && profile(before) == profile(after)
}

/// The address RandomMac considers "original" for an adapter: the hardware address when the OS
/// reports it, otherwise the address recorded before our first change.
pub fn original_mac(adapter: &Adapter, state: &AppState) -> Option<MacAddress> {
    adapter
        .permanent_mac
        .or_else(|| state.original(&adapter.id).and_then(|o| o.mac))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::{MockBackend, MockScenario};

    fn store() -> (tempfile::TempDir, DataStore) {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = DataStore::new(AppPaths::under(dir.path()));
        (dir, store)
    }

    #[test]
    fn apply_records_original_and_history() {
        let (_dir, store) = store();
        let backend = MockBackend::new(MockScenario::Normal);
        let settings = Settings::default();
        let mac: MacAddress = "02:11:22:33:44:55".parse().expect("mac");
        let out = apply_mac(
            &backend,
            &store,
            &settings,
            "mock-ethernet",
            mac,
            ChangeSource::Cli,
            false,
            &mut |_| {},
        )
        .expect("apply");
        assert!(out.succeeded());
        let (state, _) = store.load_state();
        assert_eq!(
            state
                .original("mock-ethernet")
                .and_then(|o| o.mac)
                .map(|m| m.to_string())
                .as_deref(),
            Some("00:E0:4C:68:12:34")
        );
        let (history, _) = store.load_history();
        assert_eq!(history.entries.len(), 1);
        assert_eq!(history.entries[0].new_mac, Some(mac));
        assert_eq!(history.entries[0].source, ChangeSource::Cli);

        // A second change keeps the first original.
        let mac2: MacAddress = "02:11:22:33:44:56".parse().expect("mac");
        apply_mac(
            &backend,
            &store,
            &settings,
            "mock-ethernet",
            mac2,
            ChangeSource::Ui,
            false,
            &mut |_| {},
        )
        .expect("apply 2");
        assert_eq!(
            store
                .load_state()
                .0
                .original("mock-ethernet")
                .and_then(|o| o.mac),
            "00:E0:4C:68:12:34".parse().ok()
        );

        let restored = restore_mac(
            &backend,
            &store,
            &settings,
            "mock-ethernet",
            ChangeSource::Ui,
            &mut |_| {},
        )
        .expect("restore");
        assert!(restored.succeeded());
        assert!(store.load_state().0.original("mock-ethernet").is_none());
        assert_eq!(store.load_history().0.entries.len(), 3);
    }

    #[test]
    fn validation_and_blacklist() {
        let (_dir, store) = store();
        let backend = MockBackend::new(MockScenario::Normal);
        let settings = Settings::default();
        let multicast: MacAddress = "01:00:5E:00:00:01".parse().expect("mac");
        assert!(matches!(
            apply_mac(
                &backend,
                &store,
                &settings,
                "mock-ethernet",
                multicast,
                ChangeSource::Ui,
                false,
                &mut |_| {}
            ),
            Err(OpError::NotAssignable(_))
        ));
        let mut bl = Blacklist::default();
        bl.add("02:99:99".parse().expect("p"), None, None, Utc::now())
            .expect("add");
        store.save_blacklist(&bl).expect("save");
        let blocked: MacAddress = "02:99:99:00:00:01".parse().expect("mac");
        let err = apply_mac(
            &backend,
            &store,
            &settings,
            "mock-ethernet",
            blocked,
            ChangeSource::Ui,
            false,
            &mut |_| {},
        )
        .expect_err("blocked");
        assert_eq!(err.code(), "BLACKLISTED");
        apply_mac(
            &backend,
            &store,
            &settings,
            "mock-ethernet",
            blocked,
            ChangeSource::Ui,
            true,
            &mut |_| {},
        )
        .expect("override with ignore_blacklist");
        let reserved = apply_mac(
            &backend,
            &store,
            &settings,
            "mock-ethernet",
            "AA:AA:AA:AA:AA:AA".parse().expect("mac"),
            ChangeSource::Cli,
            true,
            &mut |_| {},
        )
        .expect_err("reserved addresses ignore the override");
        assert_eq!(reserved.code(), "BLACKLISTED");
        assert!(matches!(
            apply_mac(
                &backend,
                &store,
                &settings,
                "nope",
                blocked,
                ChangeSource::Ui,
                true,
                &mut |_| {}
            ),
            Err(OpError::Net(NetError::NotFound(_)))
        ));
    }

    #[test]
    fn rolled_back_changes_do_not_keep_original() {
        let (_dir, store) = store();
        let backend = MockBackend::new(MockScenario::DriverIgnores);
        let settings = Settings::default();
        let out = apply_mac(
            &backend,
            &store,
            &settings,
            "mock-ethernet",
            "02:00:00:00:00:07".parse().expect("mac"),
            ChangeSource::Ui,
            false,
            &mut |_| {},
        )
        .expect("apply");
        assert!(!out.succeeded());
        assert_eq!(out.entry.status, ChangeStatus::RolledBack);
        assert!(store.load_state().0.original("mock-ethernet").is_none());
    }

    #[test]
    fn busy_when_locked() {
        let (_dir, store) = store();
        let _held = store.lock().expect("lock");
        let other = DataStore::new(store.paths.clone());
        let err = OperationLock::acquire(&other.paths.lock_file(), Duration::from_millis(50))
            .expect_err("busy");
        assert_eq!(OpError::from(err).code(), "BUSY");
    }

    /// Fails every change, optionally after the mock adapter was already modified.
    struct FailingBackend {
        inner: MockBackend,
        change_first: bool,
        error: fn() -> NetError,
    }

    impl NetworkBackend for FailingBackend {
        fn capabilities(&self) -> crate::Capabilities {
            self.inner.capabilities()
        }
        fn list_adapters(&self) -> Result<Vec<Adapter>, NetError> {
            self.inner.list_adapters()
        }
        fn apply(
            &self,
            adapter: &Adapter,
            mac: MacAddress,
            opts: &ChangeOptions,
            progress: &mut dyn FnMut(Stage),
        ) -> Result<ChangeReport, NetError> {
            if self.change_first {
                self.inner.apply(adapter, mac, opts, progress)?;
            }
            Err((self.error)())
        }
        fn restore(
            &self,
            adapter: &Adapter,
            original: Option<&randommac_core::OriginalRecord>,
            opts: &ChangeOptions,
            progress: &mut dyn FnMut(Stage),
        ) -> Result<ChangeReport, NetError> {
            self.inner.restore(adapter, original, opts, progress)
        }
    }

    fn failed_apply_keeps_record(change_first: bool, error: fn() -> NetError) -> bool {
        let (_dir, store) = store();
        let backend = FailingBackend {
            inner: MockBackend::new(MockScenario::Normal),
            change_first,
            error,
        };
        let out = apply_mac(
            &backend,
            &store,
            &Settings::default(),
            "mock-ethernet",
            "02:00:00:00:00:09".parse().expect("mac"),
            ChangeSource::Ui,
            false,
            &mut |_| {},
        )
        .expect("outcome");
        assert!(out.report.is_err());
        store.load_state().0.original("mock-ethernet").is_some()
    }

    #[test]
    fn failed_changes_stay_restorable_unless_nothing_changed() {
        let denied = || NetError::PermissionDenied("no".into());
        let failed = || NetError::CommandFailed {
            program: "test".into(),
            code: Some(1),
            stderr: "boom".into(),
        };
        let rollback = || NetError::RollbackFailed("could not undo".into());
        assert!(!failed_apply_keeps_record(false, denied));
        assert!(failed_apply_keeps_record(true, failed));
        assert!(failed_apply_keeps_record(false, rollback));
    }
}
