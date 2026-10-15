//! In-memory backend for UI development, screenshots and tests. Selected with
//! `RANDOMMAC_BACKEND=mock` in debug builds or `--mock` in the CLI; it never touches real adapters.

use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use randommac_core::{ChangeStatus, MacAddress, OriginalRecord};

use crate::NetworkBackend;
use crate::error::NetError;
use crate::types::*;

/// Failure injection for tests (`RANDOMMAC_MOCK_SCENARIO`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MockScenario {
    #[default]
    Normal,
    /// The "driver" ignores new addresses; the change is rolled back.
    DriverIgnores,
    /// Every step takes a while (to see progress in the UI).
    Slow,
}

impl MockScenario {
    pub fn from_name(name: &str) -> Self {
        match name.trim().to_ascii_lowercase().as_str() {
            "driver_ignores" | "driver-ignores" => Self::DriverIgnores,
            "slow" => Self::Slow,
            _ => Self::Normal,
        }
    }
}

pub struct MockBackend {
    adapters: Mutex<Vec<Adapter>>,
    scenario: MockScenario,
}

fn mac(s: &str) -> MacAddress {
    s.parse().unwrap_or_default()
}

fn sample_adapters() -> Vec<Adapter> {
    let base = Adapter {
        id: String::new(),
        name: String::new(),
        description: String::new(),
        interface_name: String::new(),
        kind: AdapterKind::Ethernet,
        is_virtual: false,
        state: AdapterState::Up,
        connected: true,
        current_mac: None,
        permanent_mac: None,
        override_value: None,
        supports_change: Support::Yes,
        methods: vec![ChangeMethod::Mock],
        nm_connection: None,
        driver: None,
        ipv4: Vec::new(),
        ipv6: Vec::new(),
    };
    vec![
        Adapter {
            id: "mock-ethernet".into(),
            name: "Ethernet".into(),
            description: "Realtek Gaming 2.5GbE Family Controller".into(),
            interface_name: "eth0".into(),
            current_mac: Some(mac("00:E0:4C:68:12:34")),
            permanent_mac: Some(mac("00:E0:4C:68:12:34")),
            driver: Some("r8169".into()),
            ipv4: vec!["192.168.1.23".into()],
            ipv6: vec!["fe80::2e0:4cff:fe68:1234".into()],
            ..base.clone()
        },
        Adapter {
            id: "mock-wifi".into(),
            name: "Wi-Fi".into(),
            description: "Intel(R) Wi-Fi 6E AX211 160MHz".into(),
            interface_name: "wlan0".into(),
            kind: AdapterKind::Wireless,
            current_mac: Some(mac("02:3C:91:5A:7E:10")),
            permanent_mac: Some(mac("00:1B:21:3A:4B:5C")),
            override_value: Some("023C915A7E10".into()),
            driver: Some("iwlwifi".into()),
            ipv4: vec!["10.0.0.42".into()],
            ..base.clone()
        },
        Adapter {
            id: "mock-usb".into(),
            name: "Ethernet 3".into(),
            description: "ASIX AX88179 USB 3.0 to Gigabit Ethernet Adapter".into(),
            interface_name: "enx0".into(),
            state: AdapterState::Disabled,
            connected: false,
            current_mac: Some(mac("00:0E:C6:11:22:33")),
            permanent_mac: Some(mac("00:0E:C6:11:22:33")),
            supports_change: Support::Unknown,
            ..base.clone()
        },
        Adapter {
            id: "mock-virtual".into(),
            name: "VirtualBox Host-Only Network".into(),
            description: "VirtualBox Host-Only Ethernet Adapter".into(),
            interface_name: "vboxnet0".into(),
            is_virtual: true,
            connected: false,
            state: AdapterState::Down,
            current_mac: Some(mac("08:00:27:00:00:0E")),
            permanent_mac: None,
            supports_change: Support::Unknown,
            ..base
        },
    ]
}

impl MockBackend {
    pub fn new(scenario: MockScenario) -> Self {
        Self {
            adapters: Mutex::new(sample_adapters()),
            scenario,
        }
    }

    pub fn from_env() -> Self {
        let scenario = std::env::var("RANDOMMAC_MOCK_SCENARIO")
            .map(|s| MockScenario::from_name(&s))
            .unwrap_or_default();
        Self::new(scenario)
    }

    fn pause(&self) {
        let ms = if self.scenario == MockScenario::Slow {
            900
        } else {
            120
        };
        thread::sleep(Duration::from_millis(ms));
    }

    fn update<F: FnOnce(&mut Adapter)>(&self, id: &str, f: F) -> Result<Adapter, NetError> {
        let mut list = self
            .adapters
            .lock()
            .map_err(|_| NetError::Io("mock poisoned".into()))?;
        let a = list
            .iter_mut()
            .find(|a| a.id == id)
            .ok_or_else(|| NetError::NotFound(id.to_string()))?;
        f(a);
        Ok(a.clone())
    }
}

impl NetworkBackend for MockBackend {
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            platform: "mock",
            elevated: true,
            can_change: true,
            network_manager: false,
            iproute2: false,
            pkexec: false,
        }
    }

    fn list_adapters(&self) -> Result<Vec<Adapter>, NetError> {
        self.adapters
            .lock()
            .map(|l| l.clone())
            .map_err(|_| NetError::Io("mock poisoned".into()))
    }

    fn apply(
        &self,
        adapter: &Adapter,
        mac: MacAddress,
        _opts: &ChangeOptions,
        progress: &mut dyn FnMut(Stage),
    ) -> Result<ChangeReport, NetError> {
        if adapter.state == AdapterState::Disabled {
            return Err(NetError::Unsupported("the adapter is disabled".into()));
        }
        let previous = adapter.current_mac;
        for stage in [
            Stage::Preparing,
            Stage::Writing,
            Stage::Restarting,
            Stage::Verifying,
        ] {
            progress(stage);
            self.pause();
        }
        if self.scenario == MockScenario::DriverIgnores {
            progress(Stage::RollingBack);
            self.pause();
            return Ok(ChangeReport {
                method: ChangeMethod::Mock,
                status: ChangeStatus::RolledBack,
                previous_mac: previous,
                requested_mac: Some(mac),
                observed_mac: previous,
                detail_code: Some("DRIVER_REJECTED".into()),
                nm_connection_uuid: None,
                nm_previous_value: None,
                temporary: false,
            });
        }
        self.update(&adapter.id, |a| {
            a.current_mac = Some(mac);
            a.override_value = Some(mac.to_compact_upper());
        })?;
        Ok(ChangeReport {
            method: ChangeMethod::Mock,
            status: ChangeStatus::Applied,
            previous_mac: previous,
            requested_mac: Some(mac),
            observed_mac: Some(mac),
            detail_code: None,
            nm_connection_uuid: None,
            nm_previous_value: None,
            temporary: false,
        })
    }

    fn restore(
        &self,
        adapter: &Adapter,
        original: Option<&OriginalRecord>,
        _opts: &ChangeOptions,
        progress: &mut dyn FnMut(Stage),
    ) -> Result<ChangeReport, NetError> {
        let target = adapter
            .permanent_mac
            .or_else(|| original.and_then(|o| o.mac))
            .ok_or(NetError::NoOriginal)?;
        for stage in [
            Stage::Preparing,
            Stage::Writing,
            Stage::Restarting,
            Stage::Verifying,
        ] {
            progress(stage);
            self.pause();
        }
        let previous = adapter.current_mac;
        self.update(&adapter.id, |a| {
            a.current_mac = Some(target);
            a.override_value = None;
        })?;
        Ok(ChangeReport {
            method: ChangeMethod::Mock,
            status: ChangeStatus::Applied,
            previous_mac: previous,
            requested_mac: Some(target),
            observed_mac: Some(target),
            detail_code: None,
            nm_connection_uuid: None,
            nm_previous_value: None,
            temporary: false,
        })
    }

    fn set_enabled(&self, adapter: &Adapter, enabled: bool) -> Result<(), NetError> {
        self.pause();
        self.update(&adapter.id, |a| {
            a.state = if enabled {
                AdapterState::Up
            } else {
                AdapterState::Disabled
            };
            a.connected = enabled && !a.is_virtual;
        })
        .map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_and_restore() {
        let backend = MockBackend::new(MockScenario::Normal);
        let eth = backend.find_adapter("mock-ethernet").expect("adapter");
        let new: MacAddress = "02:12:34:56:78:9A".parse().expect("mac");
        let mut stages = Vec::new();
        let report = backend
            .apply(&eth, new, &ChangeOptions::default(), &mut |s| {
                stages.push(s)
            })
            .expect("apply");
        assert_eq!(report.status, ChangeStatus::Applied);
        assert_eq!(stages.first(), Some(&Stage::Preparing));
        let after = backend.find_adapter("mock-ethernet").expect("adapter");
        assert_eq!(after.current_mac, Some(new));
        assert!(after.differs_from_permanent());

        backend
            .restore(&after, None, &ChangeOptions::default(), &mut |_| {})
            .expect("restore");
        let restored = backend.find_adapter("mock-ethernet").expect("adapter");
        assert!(!restored.differs_from_permanent());
    }

    #[test]
    fn driver_ignores_scenario_rolls_back() {
        let backend = MockBackend::new(MockScenario::DriverIgnores);
        let eth = backend.find_adapter("mock-ethernet").expect("adapter");
        let report = backend
            .apply(
                &eth,
                "02:00:00:00:00:01".parse().expect("mac"),
                &ChangeOptions::default(),
                &mut |_| {},
            )
            .expect("apply");
        assert_eq!(report.status, ChangeStatus::RolledBack);
        assert_eq!(
            backend
                .find_adapter("mock-ethernet")
                .expect("a")
                .current_mac,
            eth.current_mac
        );
        assert_eq!(MockScenario::from_name("SLOW"), MockScenario::Slow);
    }

    #[test]
    fn disabled_adapters() {
        let backend = MockBackend::new(MockScenario::Normal);
        let usb = backend.find_adapter("mock-usb").expect("adapter");
        assert!(
            backend
                .apply(
                    &usb,
                    "02:00:00:00:00:02".parse().expect("mac"),
                    &ChangeOptions::default(),
                    &mut |_| {}
                )
                .is_err()
        );
        backend.set_enabled(&usb, true).expect("enable");
        assert_eq!(
            backend.find_adapter("mock-usb").expect("a").state,
            AdapterState::Up
        );
    }
}
