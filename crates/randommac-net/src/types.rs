//! Data types shared by all backends.

use std::time::Duration;

use randommac_core::{ChangeStatus, MacAddress, settings::LinuxMethod};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AdapterKind {
    Ethernet,
    Wireless,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AdapterState {
    Up,
    Down,
    /// Disabled in the operating system (Windows Device Manager / `ip link set down`).
    Disabled,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ChangeMethod {
    /// `NetworkAddress` driver property in the registry + device restart (Windows).
    WindowsRegistry,
    /// `cloned-mac-address` of the active NetworkManager connection profile (Linux, persistent).
    NetworkManager,
    /// `ip link set address` (Linux, until reboot).
    Iproute2,
    /// In-memory backend used for development and tests.
    Mock,
}

impl ChangeMethod {
    pub fn as_str(self) -> &'static str {
        match self {
            ChangeMethod::WindowsRegistry => "windowsRegistry",
            ChangeMethod::NetworkManager => "networkManager",
            ChangeMethod::Iproute2 => "iproute2",
            ChangeMethod::Mock => "mock",
        }
    }
}

/// Whether the driver is known to accept a custom MAC address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Support {
    Yes,
    Unknown,
    No,
}

/// NetworkManager connection currently active on a device.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NmConnection {
    pub uuid: String,
    pub name: String,
    /// `802-3-ethernet` or `802-11-wireless`.
    pub setting: String,
    /// Current `cloned-mac-address` value of the profile ("" = unset).
    pub cloned_mac: String,
}

/// A network adapter as seen by the operating system.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Adapter {
    /// Stable identifier: device instance id (Windows) or interface name (Linux).
    pub id: String,
    /// Display name ("Wi-Fi", "Ethernet 2", "wlp2s0").
    pub name: String,
    /// Hardware description ("Realtek 8852CE WiFi 6E PCI-E NIC").
    pub description: String,
    /// OS interface name (Linux ifname or Windows connection name).
    pub interface_name: String,
    pub kind: AdapterKind,
    pub is_virtual: bool,
    pub state: AdapterState,
    pub connected: bool,
    pub current_mac: Option<MacAddress>,
    /// Burned-in address reported by the hardware, if the OS exposes it.
    pub permanent_mac: Option<MacAddress>,
    /// Active override (registry `NetworkAddress` value, NM `cloned-mac-address`), if any.
    pub override_value: Option<String>,
    pub supports_change: Support,
    /// Methods that can change this adapter's address, preferred first.
    pub methods: Vec<ChangeMethod>,
    pub nm_connection: Option<NmConnection>,
    pub driver: Option<String>,
    pub ipv4: Vec<String>,
    pub ipv6: Vec<String>,
}

impl Adapter {
    /// The address differs from the hardware address (changed by us, the OS or another tool).
    pub fn differs_from_permanent(&self) -> bool {
        matches!((self.current_mac, self.permanent_mac), (Some(c), Some(p)) if c != p)
    }
}

/// Step of a MAC change, reported for progress display.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Stage {
    Preparing,
    Writing,
    Restarting,
    Verifying,
    RollingBack,
}

/// Options for apply/restore.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeOptions {
    pub linux_method: LinuxMethod,
    /// How long to wait for the adapter to come back with the new address.
    pub verify_timeout: Duration,
}

impl Default for ChangeOptions {
    fn default() -> Self {
        Self {
            linux_method: LinuxMethod::Auto,
            verify_timeout: Duration::from_secs(20),
        }
    }
}

/// Result of a completed apply/restore.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeReport {
    pub method: ChangeMethod,
    pub status: ChangeStatus,
    pub previous_mac: Option<MacAddress>,
    pub requested_mac: Option<MacAddress>,
    /// Address observed after the change.
    pub observed_mac: Option<MacAddress>,
    /// Machine-readable explanation for non-`Applied` outcomes (e.g. `DRIVER_REJECTED`).
    pub detail_code: Option<String>,
    /// NetworkManager connection that was modified and its previous `cloned-mac-address`.
    pub nm_connection_uuid: Option<String>,
    pub nm_previous_value: Option<String>,
    /// The change is lost on reboot (iproute2).
    pub temporary: bool,
}

/// What the current process can do.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Capabilities {
    pub platform: &'static str,
    /// Running with the privileges needed to change addresses directly (Windows admin / root).
    pub elevated: bool,
    /// MAC changes are possible at all (directly or through an authentication prompt).
    pub can_change: bool,
    pub network_manager: bool,
    pub iproute2: bool,
    /// A polkit authentication front-end (`pkexec`) is installed.
    pub pkexec: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn method_names_match_serde() {
        for method in [
            ChangeMethod::WindowsRegistry,
            ChangeMethod::NetworkManager,
            ChangeMethod::Iproute2,
            ChangeMethod::Mock,
        ] {
            let json = serde_json::to_value(method).expect("serialize");
            assert_eq!(json, serde_json::Value::String(method.as_str().into()));
        }
    }

    #[test]
    fn default_options_wait_twenty_seconds() {
        let opts = ChangeOptions::default();
        assert_eq!(opts.linux_method, LinuxMethod::Auto);
        assert_eq!(opts.verify_timeout, Duration::from_secs(20));
    }
}
