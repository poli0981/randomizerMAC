//! Operating-system backends for RandomMac.
//!
//! [`NetworkBackend`] is implemented for Windows (registry + SetupAPI for MAC addresses, IP Helper
//! for DNS), Linux (NetworkManager / iproute2) and an in-memory mock. Higher layers (GUI, CLI) handle validation against the
//! blacklist, history and the cross-process lock; backends only talk to the operating system.

pub mod dns_ops;
pub mod dns_probe;
pub mod dns_types;
pub mod error;
pub mod linux_parse;
pub mod mock;
pub mod nm_dns;
pub mod ops;
pub mod process;
pub mod types;
pub mod win_classify;
pub mod win_dns;

/// The DNS methods of [`NetworkBackend`] for a backend whose `dns` module provides
/// `capabilities`, `status`, `snapshot`, `apply` and `restore` (Windows and Linux): written once
/// for both `impl` blocks, after their MAC methods.
#[cfg(any(windows, target_os = "linux"))]
macro_rules! dns_methods {
    () => {
        fn dns_capabilities(&self, adapter: &Adapter) -> DnsCapabilities {
            dns::capabilities(adapter)
        }

        fn dns_status(&self, adapter: &Adapter) -> Result<DnsStatus, NetError> {
            dns::status(adapter)
        }

        fn dns_snapshot(&self, adapter: &Adapter) -> Result<DnsSnapshot, NetError> {
            dns::snapshot(adapter)
        }

        fn apply_dns(
            &self,
            adapter: &Adapter,
            config: &DnsConfig,
            previous: &DnsSnapshot,
            opts: &DnsOptions,
            progress: &mut dyn FnMut(Stage),
        ) -> Result<DnsReport, NetError> {
            dns::apply(adapter, config, previous, opts, progress)
        }

        fn restore_dns(
            &self,
            adapter: &Adapter,
            original: &DnsOriginalRecord,
            opts: &DnsOptions,
            progress: &mut dyn FnMut(Stage),
        ) -> Result<DnsReport, NetError> {
            dns::restore(adapter, original, opts, progress)
        }
    };
}

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(windows)]
pub mod windows;

pub use dns_types::*;
pub use error::NetError;
pub use types::*;

use randommac_core::dns::{DnsConfig, DnsOriginalRecord, DnsSnapshot};
use randommac_core::{MacAddress, OriginalRecord};

fn dns_unsupported() -> NetError {
    NetError::Unsupported("DNS settings are not supported on this platform".into())
}

/// Operations every platform backend provides.
pub trait NetworkBackend: Send + Sync {
    fn capabilities(&self) -> Capabilities;

    /// All Ethernet/Wi-Fi adapters, physical first.
    fn list_adapters(&self) -> Result<Vec<Adapter>, NetError>;

    /// Fresh information about one adapter.
    fn find_adapter(&self, id: &str) -> Result<Adapter, NetError> {
        self.list_adapters()?
            .into_iter()
            .find(|a| a.id == id)
            .ok_or_else(|| NetError::NotFound(id.to_string()))
    }

    /// Sets `mac` on the adapter, verifies it and rolls back if the driver ignores it.
    fn apply(
        &self,
        adapter: &Adapter,
        mac: MacAddress,
        opts: &ChangeOptions,
        progress: &mut dyn FnMut(Stage),
    ) -> Result<ChangeReport, NetError>;

    /// Undoes RandomMac's changes (registry override / NetworkManager profile / recorded original).
    fn restore(
        &self,
        adapter: &Adapter,
        original: Option<&OriginalRecord>,
        opts: &ChangeOptions,
        progress: &mut dyn FnMut(Stage),
    ) -> Result<ChangeReport, NetError>;

    /// Enables or disables the adapter (Windows only).
    fn set_enabled(&self, adapter: &Adapter, enabled: bool) -> Result<(), NetError> {
        let _ = (adapter, enabled);
        Err(NetError::Unsupported(
            "enabling or disabling adapters is not supported on this platform".into(),
        ))
    }

    /// What can be done with the adapter's DNS settings.
    fn dns_capabilities(&self, adapter: &Adapter) -> DnsCapabilities {
        let _ = adapter;
        DnsCapabilities::unsupported(DnsReason::UnsupportedPlatform)
    }

    /// The adapter's DNS settings and the servers it uses.
    fn dns_status(&self, adapter: &Adapter) -> Result<DnsStatus, NetError> {
        let _ = adapter;
        Err(dns_unsupported())
    }

    /// The adapter's DNS settings in the backend's own terms (what "Restore original DNS" puts
    /// back).
    fn dns_snapshot(&self, adapter: &Adapter) -> Result<DnsSnapshot, NetError> {
        let _ = adapter;
        Err(dns_unsupported())
    }

    /// Applies `config`, verifies it, and goes back to `previous` when the system did not take it.
    fn apply_dns(
        &self,
        adapter: &Adapter,
        config: &DnsConfig,
        previous: &DnsSnapshot,
        opts: &DnsOptions,
        progress: &mut dyn FnMut(Stage),
    ) -> Result<DnsReport, NetError> {
        let _ = (adapter, config, previous, opts, progress);
        Err(dns_unsupported())
    }

    /// Puts back the settings recorded before RandomMac's first DNS change.
    fn restore_dns(
        &self,
        adapter: &Adapter,
        original: &DnsOriginalRecord,
        opts: &DnsOptions,
        progress: &mut dyn FnMut(Stage),
    ) -> Result<DnsReport, NetError> {
        let _ = (adapter, original, opts, progress);
        Err(dns_unsupported())
    }
}

/// Backend for platforms without MAC changing support.
#[derive(Debug, Default)]
pub struct UnsupportedBackend;

impl NetworkBackend for UnsupportedBackend {
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            platform: std::env::consts::OS,
            elevated: false,
            can_change: false,
            network_manager: false,
            iproute2: false,
            pkexec: false,
        }
    }

    fn list_adapters(&self) -> Result<Vec<Adapter>, NetError> {
        Err(NetError::Unsupported(format!(
            "{} is not supported",
            std::env::consts::OS
        )))
    }

    fn apply(
        &self,
        _: &Adapter,
        _: MacAddress,
        _: &ChangeOptions,
        _: &mut dyn FnMut(Stage),
    ) -> Result<ChangeReport, NetError> {
        Err(NetError::Unsupported(std::env::consts::OS.into()))
    }

    fn restore(
        &self,
        _: &Adapter,
        _: Option<&OriginalRecord>,
        _: &ChangeOptions,
        _: &mut dyn FnMut(Stage),
    ) -> Result<ChangeReport, NetError> {
        Err(NetError::Unsupported(std::env::consts::OS.into()))
    }
}

/// The backend for the current operating system.
pub fn system_backend() -> Box<dyn NetworkBackend> {
    #[cfg(windows)]
    {
        Box::new(windows::WindowsBackend::new())
    }
    #[cfg(target_os = "linux")]
    {
        Box::new(linux::LinuxBackend::new())
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        Box::new(UnsupportedBackend)
    }
}

/// Finds an adapter by id, interface name, display name (case-insensitive) or current/permanent
/// MAC address. Ambiguous names return `None`.
pub fn resolve_adapter<'a>(adapters: &'a [Adapter], query: &str) -> Option<&'a Adapter> {
    let q = query.trim();
    if let Some(a) = adapters.iter().find(|a| a.id == q) {
        return Some(a);
    }
    let by_name: Vec<&Adapter> = adapters
        .iter()
        .filter(|a| a.interface_name.eq_ignore_ascii_case(q) || a.name.eq_ignore_ascii_case(q))
        .collect();
    if by_name.len() == 1 {
        return Some(by_name[0]);
    }
    if by_name.len() > 1 {
        return None;
    }
    let mac: MacAddress = q.parse().ok()?;
    let by_mac: Vec<&Adapter> = adapters
        .iter()
        .filter(|a| a.current_mac == Some(mac) || a.permanent_mac == Some(mac))
        .collect();
    (by_mac.len() == 1).then(|| by_mac[0])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::{MockBackend, MockScenario};

    #[test]
    fn resolves_by_id_name_and_mac() {
        let backend = MockBackend::new(MockScenario::Normal);
        let list = backend.list_adapters().expect("list");
        assert_eq!(
            resolve_adapter(&list, "mock-wifi").map(|a| a.id.as_str()),
            Some("mock-wifi")
        );
        assert_eq!(
            resolve_adapter(&list, "WLAN0").map(|a| a.id.as_str()),
            Some("mock-wifi")
        );
        assert_eq!(
            resolve_adapter(&list, "wi-fi").map(|a| a.id.as_str()),
            Some("mock-wifi")
        );
        assert_eq!(
            resolve_adapter(&list, "00-1b-21-3a-4b-5c").map(|a| a.id.as_str()),
            Some("mock-wifi")
        );
        assert!(resolve_adapter(&list, "nope").is_none());
    }

    #[test]
    fn ambiguous_names_and_addresses_resolve_to_nothing() {
        let backend = MockBackend::new(MockScenario::Normal);
        let mut list = backend.list_adapters().expect("list");
        let mut twin = list[0].clone();
        twin.id = "twin".into();
        let mac = twin.current_mac.expect("current mac").to_string();
        let name = twin.name.clone();
        list.push(twin);
        assert!(resolve_adapter(&list, &name).is_none());
        assert!(resolve_adapter(&list, &mac).is_none());
        assert_eq!(
            resolve_adapter(&list, " twin ").map(|a| a.id.as_str()),
            Some("twin")
        );
    }

    #[test]
    fn unsupported_backend_reports_errors() {
        let b = UnsupportedBackend;
        assert!(!b.capabilities().can_change);
        assert_eq!(
            b.list_adapters().expect_err("unsupported").code(),
            "UNSUPPORTED"
        );
        assert_eq!(
            b.find_adapter("x").expect_err("unsupported").code(),
            "UNSUPPORTED"
        );

        let adapter = MockBackend::new(MockScenario::Normal)
            .list_adapters()
            .expect("list")
            .remove(0);
        let mac: MacAddress = "02:11:22:33:44:55".parse().expect("mac");
        let opts = ChangeOptions::default();
        let mut stages = Vec::new();
        let mut progress = |s: Stage| stages.push(s);
        let errors = [
            b.apply(&adapter, mac, &opts, &mut progress)
                .expect_err("apply"),
            b.restore(&adapter, None, &opts, &mut progress)
                .expect_err("restore"),
            b.set_enabled(&adapter, false).expect_err("disable"),
        ];
        assert!(errors.iter().all(|e| e.code() == "UNSUPPORTED"));
        assert!(stages.is_empty());
    }

    #[test]
    fn find_adapter_reports_unknown_ids() {
        let backend = MockBackend::new(MockScenario::Normal);
        assert_eq!(
            backend.find_adapter("mock-wifi").expect("found").id,
            "mock-wifi"
        );
        assert_eq!(
            backend.find_adapter("missing").expect_err("missing"),
            NetError::NotFound("missing".into())
        );
    }

    #[test]
    fn system_backend_capabilities() {
        let caps = system_backend().capabilities();
        assert!(!caps.platform.is_empty());
    }
}
