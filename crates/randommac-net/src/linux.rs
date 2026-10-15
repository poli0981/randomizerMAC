//! Linux backend.
//!
//! * Enumeration: sysfs (`/sys/class/net`), udev database for model names, `getifaddrs` for IPs,
//!   `ETHTOOL_GPERMADDR` for the burned-in address (no root needed).
//! * NetworkManager method: `cloned-mac-address` on the active connection profile, re-activated
//!   with `nmcli` as the calling user (polkit asks the desktop for authentication if needed).
//! * iproute2 method: `ip -force -batch -` run directly as root or through `pkexec` (one
//!   authentication prompt). The GUI itself never runs as root.

use std::collections::HashMap;
use std::ffi::CStr;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

use randommac_core::settings::LinuxMethod;
use randommac_core::{ChangeStatus, MacAddress, OriginalRecord};

use crate::NetworkBackend;
use crate::error::NetError;
use crate::linux_parse::*;
use crate::process::{self, Output};
use crate::types::*;

const NMCLI: &[&str] = &["/usr/bin/nmcli", "/bin/nmcli", "/usr/sbin/nmcli"];
const IP: &[&str] = &["/usr/sbin/ip", "/sbin/ip", "/usr/bin/ip", "/bin/ip"];
const PKEXEC: &[&str] = &["/usr/bin/pkexec", "/bin/pkexec"];
const NMCLI_TIMEOUT: Duration = Duration::from_secs(20);
const NM_UP_TIMEOUT: Duration = Duration::from_secs(75);
/// Includes time for the user to answer the authentication dialog.
const PKEXEC_TIMEOUT: Duration = Duration::from_secs(180);

pub struct LinuxBackend {
    class_net: PathBuf,
    udev_data: PathBuf,
}

impl Default for LinuxBackend {
    fn default() -> Self {
        Self::new()
    }
}

struct NmDevice {
    state: u32,
    con_uuid: String,
}

impl LinuxBackend {
    pub fn new() -> Self {
        Self {
            class_net: PathBuf::from("/sys/class/net"),
            udev_data: PathBuf::from("/run/udev/data"),
        }
    }

    fn is_root() -> bool {
        // SAFETY: geteuid has no preconditions.
        unsafe { libc::geteuid() == 0 }
    }

    fn nmcli() -> Option<PathBuf> {
        process::find_program(NMCLI)
    }

    fn nm_run(args: &[&str], timeout: Duration) -> Result<Output, NetError> {
        let nmcli = Self::nmcli()
            .ok_or_else(|| NetError::NmUnavailable("nmcli is not installed".into()))?;
        process::run(&nmcli, args, None, timeout)
    }

    fn nm_running() -> bool {
        Self::nm_run(&["-t", "-f", "RUNNING", "general"], NMCLI_TIMEOUT)
            .is_ok_and(|o| o.success() && o.stdout.trim() == "running")
    }

    fn nm_device(ifname: &str) -> Option<NmDevice> {
        let out = Self::nm_run(
            &[
                "-g",
                "GENERAL.STATE,GENERAL.CON-UUID",
                "device",
                "show",
                ifname,
            ],
            NMCLI_TIMEOUT,
        )
        .ok()
        .filter(Output::success)?;
        let mut lines = out.stdout.lines();
        let state = parse_nm_state(lines.next()?)?;
        let con_uuid = unescape_value(lines.next().unwrap_or_default().trim());
        Some(NmDevice { state, con_uuid })
    }

    fn nm_connection(uuid: &str) -> Option<NmConnection> {
        let out = Self::nm_run(
            &[
                "-g",
                "connection.id,connection.type",
                "connection",
                "show",
                "uuid",
                uuid,
            ],
            NMCLI_TIMEOUT,
        )
        .ok()
        .filter(Output::success)?;
        let mut lines = out.stdout.lines();
        let name = unescape_value(lines.next()?.trim());
        let setting = unescape_value(lines.next()?.trim());
        if setting != "802-3-ethernet" && setting != "802-11-wireless" {
            return None;
        }
        let cloned = Self::nm_run(
            &[
                "-g",
                &format!("{setting}.cloned-mac-address"),
                "connection",
                "show",
                "uuid",
                uuid,
            ],
            NMCLI_TIMEOUT,
        )
        .ok()
        .filter(Output::success)
        .map(|o| unescape_value(o.stdout.trim()))
        .unwrap_or_default();
        Some(NmConnection {
            uuid: uuid.to_string(),
            name,
            setting,
            cloned_mac: cloned,
        })
    }

    fn nm_check(out: Output, what: &str) -> Result<(), NetError> {
        if out.success() {
            return Ok(());
        }
        let msg = format!("{}{}", out.stderr.trim(), out.stdout.trim());
        let lower = msg.to_ascii_lowercase();
        if lower.contains("not authorized") || lower.contains("insufficient privileges") {
            return Err(NetError::PermissionDenied(msg));
        }
        Err(NetError::CommandFailed {
            program: format!("nmcli {what}"),
            code: out.code,
            stderr: msg,
        })
    }

    fn nm_set_cloned(conn: &NmConnection, value: &str) -> Result<(), NetError> {
        let prop = format!("{}.cloned-mac-address", conn.setting);
        let out = Self::nm_run(
            &["connection", "modify", "uuid", &conn.uuid, &prop, value],
            NMCLI_TIMEOUT,
        )?;
        Self::nm_check(out, "connection modify")
    }

    fn nm_up(conn: &NmConnection) -> Result<(), NetError> {
        let out = Self::nm_run(
            &["--wait", "60", "connection", "up", "uuid", &conn.uuid],
            NM_UP_TIMEOUT,
        )?;
        Self::nm_check(out, "connection up")
    }

    fn current_address(&self, ifname: &str) -> Option<MacAddress> {
        read_interface(&self.class_net, ifname).ok()?.address
    }

    /// Polls sysfs until the interface reports `expected` or the timeout elapses.
    fn wait_for(
        &self,
        ifname: &str,
        expected: MacAddress,
        timeout: Duration,
    ) -> Option<MacAddress> {
        let deadline = Instant::now() + timeout;
        loop {
            let current = self.current_address(ifname);
            if current == Some(expected) || Instant::now() >= deadline {
                return current;
            }
            thread::sleep(Duration::from_millis(250));
        }
    }

    fn run_ip_batch(script: &str) -> Result<(), NetError> {
        let ip = process::find_program(IP)
            .ok_or_else(|| NetError::Unsupported("iproute2 (`ip`) is not installed".into()))?;
        let ip_str = ip.to_string_lossy().into_owned();
        let out = if Self::is_root() {
            process::run(&ip, &["-force", "-batch", "-"], Some(script), NMCLI_TIMEOUT)?
        } else {
            let pkexec = process::find_program(PKEXEC).ok_or_else(|| {
                NetError::AuthUnavailable(
                    "pkexec (polkit) is not installed; run the CLI with sudo".into(),
                )
            })?;
            let out = process::run(
                &pkexec,
                &[&ip_str, "-force", "-batch", "-"],
                Some(script),
                PKEXEC_TIMEOUT,
            )?;
            if let Some(e) = pkexec_error(out.code, &out.stderr) {
                return Err(e);
            }
            out
        };
        if out.success() {
            Ok(())
        } else {
            Err(NetError::CommandFailed {
                program: "ip".into(),
                code: out.code,
                stderr: out.stderr.trim().to_string(),
            })
        }
    }

    fn choose_method(
        &self,
        adapter: &Adapter,
        requested: LinuxMethod,
    ) -> Result<ChangeMethod, NetError> {
        let nm = adapter.methods.contains(&ChangeMethod::NetworkManager);
        match requested {
            LinuxMethod::NetworkManager if nm => Ok(ChangeMethod::NetworkManager),
            LinuxMethod::NetworkManager => {
                if Self::nmcli().is_none() || !Self::nm_running() {
                    Err(NetError::NmUnavailable(
                        "NetworkManager is not running".into(),
                    ))
                } else {
                    Err(NetError::NmNoActiveConnection)
                }
            }
            LinuxMethod::Iproute2 => Ok(ChangeMethod::Iproute2),
            LinuxMethod::Auto if nm => Ok(ChangeMethod::NetworkManager),
            LinuxMethod::Auto => Ok(ChangeMethod::Iproute2),
        }
    }

    fn report(
        method: ChangeMethod,
        status: ChangeStatus,
        previous: Option<MacAddress>,
        requested: Option<MacAddress>,
        observed: Option<MacAddress>,
    ) -> ChangeReport {
        ChangeReport {
            method,
            status,
            previous_mac: previous,
            requested_mac: requested,
            observed_mac: observed,
            detail_code: None,
            nm_connection_uuid: None,
            nm_previous_value: None,
            temporary: method == ChangeMethod::Iproute2,
        }
    }

    fn apply_nm(
        &self,
        adapter: &Adapter,
        mac: MacAddress,
        opts: &ChangeOptions,
        progress: &mut dyn FnMut(Stage),
    ) -> Result<ChangeReport, NetError> {
        let conn = adapter
            .nm_connection
            .clone()
            .ok_or(NetError::NmNoActiveConnection)?;
        let previous_value = conn.cloned_mac.clone();
        let previous = adapter.current_mac;
        progress(Stage::Writing);
        Self::nm_set_cloned(&conn, &mac.to_colon_lower())?;
        progress(Stage::Restarting);
        if let Err(e) = Self::nm_up(&conn) {
            progress(Stage::RollingBack);
            let _ = Self::nm_set_cloned(&conn, &previous_value);
            let _ = Self::nm_up(&conn);
            return Err(e);
        }
        progress(Stage::Verifying);
        let observed = self.wait_for(&adapter.interface_name, mac, opts.verify_timeout);
        let mut report = Self::report(
            ChangeMethod::NetworkManager,
            ChangeStatus::Applied,
            previous,
            Some(mac),
            observed,
        );
        report.nm_connection_uuid = Some(conn.uuid.clone());
        report.nm_previous_value = Some(previous_value.clone());
        if observed != Some(mac) {
            if observed == previous {
                progress(Stage::RollingBack);
                Self::nm_set_cloned(&conn, &previous_value)
                    .and_then(|()| Self::nm_up(&conn))
                    .map_err(|e| NetError::RollbackFailed(e.to_string()))?;
                report.status = ChangeStatus::RolledBack;
                report.detail_code = Some("DRIVER_REJECTED".into());
            } else {
                report.status = ChangeStatus::Unverified;
                report.detail_code = Some("VERIFY_TIMEOUT".into());
            }
        }
        Ok(report)
    }

    fn apply_ip(
        &self,
        adapter: &Adapter,
        mac: MacAddress,
        opts: &ChangeOptions,
        progress: &mut dyn FnMut(Stage),
    ) -> Result<ChangeReport, NetError> {
        let iface = read_interface(&self.class_net, &adapter.interface_name)
            .map_err(|_| NetError::NotFound(adapter.interface_name.clone()))?;
        let script = ip_batch_script(&iface.name, mac, iface.admin_up)?;
        progress(Stage::Writing);
        Self::run_ip_batch(&script)?;
        progress(Stage::Verifying);
        let observed = self.wait_for(
            &iface.name,
            mac,
            opts.verify_timeout.min(Duration::from_secs(5)),
        );
        let status = if observed == Some(mac) {
            ChangeStatus::Applied
        } else {
            ChangeStatus::Unverified
        };
        Ok(Self::report(
            ChangeMethod::Iproute2,
            status,
            adapter.current_mac,
            Some(mac),
            observed,
        ))
    }

    fn ip_addresses() -> HashMap<String, (Vec<String>, Vec<String>)> {
        let mut map: HashMap<String, (Vec<String>, Vec<String>)> = HashMap::new();
        let mut ifap: *mut libc::ifaddrs = std::ptr::null_mut();
        // SAFETY: getifaddrs allocates a list we free with freeifaddrs below.
        if unsafe { libc::getifaddrs(&mut ifap) } != 0 {
            return map;
        }
        let mut cur = ifap;
        while !cur.is_null() {
            // SAFETY: cur is a valid node of the list returned by getifaddrs.
            let ifa = unsafe { &*cur };
            if !ifa.ifa_addr.is_null() && !ifa.ifa_name.is_null() {
                // SAFETY: ifa_name is a NUL-terminated string owned by the list.
                let name = unsafe { CStr::from_ptr(ifa.ifa_name) }
                    .to_string_lossy()
                    .into_owned();
                // SAFETY: ifa_addr is non-null; sa_family tells us the concrete type.
                let family = i32::from(unsafe { (*ifa.ifa_addr).sa_family });
                let entry = map.entry(name).or_default();
                if family == libc::AF_INET {
                    // SAFETY: family is AF_INET, so the address is a sockaddr_in.
                    let sin = unsafe { &*(ifa.ifa_addr as *const libc::sockaddr_in) };
                    entry
                        .0
                        .push(Ipv4Addr::from(u32::from_be(sin.sin_addr.s_addr)).to_string());
                } else if family == libc::AF_INET6 {
                    // SAFETY: family is AF_INET6, so the address is a sockaddr_in6.
                    let sin6 = unsafe { &*(ifa.ifa_addr as *const libc::sockaddr_in6) };
                    entry
                        .1
                        .push(Ipv6Addr::from(sin6.sin6_addr.s6_addr).to_string());
                }
            }
            cur = ifa.ifa_next;
        }
        // SAFETY: ifap came from getifaddrs and is freed exactly once.
        unsafe { libc::freeifaddrs(ifap) };
        map
    }

    fn description(&self, iface: &SysfsIface) -> String {
        let from_udev = std::fs::read_to_string(self.udev_data.join(format!("n{}", iface.ifindex)))
            .ok()
            .map(|t| parse_udev_properties(&t))
            .and_then(|p| udev_description(&p));
        from_udev
            .or_else(|| {
                iface
                    .driver
                    .as_ref()
                    .map(|d| format!("{d} network interface"))
            })
            .unwrap_or_else(|| {
                if iface.is_virtual {
                    "Virtual network interface".into()
                } else {
                    "Network interface".into()
                }
            })
    }
}

/// Burned-in address via `SIOCETHTOOL`/`ETHTOOL_GPERMADDR` (allowed for unprivileged users).
/// Returns `None` for virtual devices, which report all zeros.
pub fn permanent_address(ifname: &str) -> Option<MacAddress> {
    const SIOCETHTOOL: libc::c_ulong = 0x8946;
    const ETHTOOL_GPERMADDR: u32 = 0x20;
    const MAX_ADDR_LEN: usize = 32;

    #[repr(C)]
    struct PermAddr {
        cmd: u32,
        size: u32,
        data: [u8; MAX_ADDR_LEN],
    }
    #[repr(C)]
    struct IfReq {
        name: [libc::c_char; libc::IFNAMSIZ],
        data: *mut libc::c_void,
        _pad: [u8; 16],
    }

    if !valid_ifname(ifname) {
        return None;
    }
    let mut perm = PermAddr {
        cmd: ETHTOOL_GPERMADDR,
        size: MAX_ADDR_LEN as u32,
        data: [0; MAX_ADDR_LEN],
    };
    let mut req = IfReq {
        name: [0; libc::IFNAMSIZ],
        data: (&mut perm as *mut PermAddr).cast(),
        _pad: [0; 16],
    };
    for (dst, src) in req.name.iter_mut().zip(ifname.bytes()) {
        *dst = src as libc::c_char;
    }
    // SAFETY: plain socket creation; the descriptor is closed below.
    let fd = unsafe { libc::socket(libc::AF_INET, libc::SOCK_DGRAM | libc::SOCK_CLOEXEC, 0) };
    if fd < 0 {
        return None;
    }
    // SAFETY: req points to a properly initialized ifreq whose data field points to `perm`,
    // both of which outlive the call.
    let rc = unsafe { libc::ioctl(fd, SIOCETHTOOL as _, &mut req as *mut IfReq) };
    // SAFETY: fd is a valid descriptor owned by this function.
    unsafe { libc::close(fd) };
    if rc < 0 || perm.size != 6 {
        return None;
    }
    MacAddress::try_from(&perm.data[..6])
        .ok()
        .filter(|m| !m.is_zero())
}

impl NetworkBackend for LinuxBackend {
    fn capabilities(&self) -> Capabilities {
        let root = Self::is_root();
        let ip = process::find_program(IP).is_some();
        let pkexec = process::find_program(PKEXEC).is_some();
        let nm = Self::nmcli().is_some() && Self::nm_running();
        Capabilities {
            platform: "linux",
            elevated: root,
            can_change: nm || (ip && (root || pkexec)),
            network_manager: nm,
            iproute2: ip,
            pkexec,
        }
    }

    fn list_adapters(&self) -> Result<Vec<Adapter>, NetError> {
        let ifaces = read_interfaces(&self.class_net)?;
        let ips = Self::ip_addresses();
        let nm = Self::nmcli().is_some() && Self::nm_running();
        let ip_ok = process::find_program(IP).is_some()
            && (Self::is_root() || process::find_program(PKEXEC).is_some());
        let mut out = Vec::new();
        for iface in ifaces {
            if iface.arphrd != ARPHRD_ETHER || iface.name == "lo" || !valid_ifname(&iface.name) {
                continue;
            }
            let permanent = permanent_address(&iface.name).or_else(|| {
                (iface.addr_assign_type == Some(0))
                    .then_some(iface.address)
                    .flatten()
            });
            let nm_device = if nm {
                Self::nm_device(&iface.name)
            } else {
                None
            };
            let nm_connection = nm_device
                .as_ref()
                .filter(|d| d.state > NM_DEVICE_STATE_UNMANAGED && !d.con_uuid.is_empty())
                .and_then(|d| Self::nm_connection(&d.con_uuid));
            let mut methods = Vec::new();
            if nm_connection.is_some() {
                methods.push(ChangeMethod::NetworkManager);
            }
            if ip_ok {
                methods.push(ChangeMethod::Iproute2);
            }
            // Only an explicit address counts as an override; "random"/"stable"/"preserve"/
            // "permanent" are NetworkManager policies that "Restore" must not erase.
            let override_value = nm_connection
                .as_ref()
                .map(|c| c.cloned_mac.clone())
                .filter(|v| v.parse::<MacAddress>().is_ok());
            let (ipv4, ipv6) = ips.get(&iface.name).cloned().unwrap_or_default();
            out.push(Adapter {
                id: iface.name.clone(),
                name: iface.name.clone(),
                description: self.description(&iface),
                interface_name: iface.name.clone(),
                kind: if iface.is_wireless {
                    AdapterKind::Wireless
                } else {
                    AdapterKind::Ethernet
                },
                is_virtual: iface.is_virtual,
                state: if iface.operstate == "up" {
                    AdapterState::Up
                } else {
                    AdapterState::Down
                },
                connected: iface.operstate == "up" && iface.carrier != Some(false),
                current_mac: iface.address,
                permanent_mac: permanent,
                override_value,
                supports_change: if methods.is_empty() {
                    Support::No
                } else {
                    Support::Yes
                },
                methods,
                nm_connection,
                driver: iface.driver.clone(),
                ipv4,
                ipv6,
            });
        }
        Ok(out)
    }

    fn apply(
        &self,
        adapter: &Adapter,
        mac: MacAddress,
        opts: &ChangeOptions,
        progress: &mut dyn FnMut(Stage),
    ) -> Result<ChangeReport, NetError> {
        progress(Stage::Preparing);
        if !valid_ifname(&adapter.interface_name) {
            return Err(NetError::InvalidInput(adapter.interface_name.clone()));
        }
        match self.choose_method(adapter, opts.linux_method)? {
            ChangeMethod::NetworkManager => self.apply_nm(adapter, mac, opts, progress),
            _ => self.apply_ip(adapter, mac, opts, progress),
        }
    }

    fn restore(
        &self,
        adapter: &Adapter,
        original: Option<&OriginalRecord>,
        opts: &ChangeOptions,
        progress: &mut dyn FnMut(Stage),
    ) -> Result<ChangeReport, NetError> {
        progress(Stage::Preparing);
        let previous = adapter.current_mac;
        let profiles = original.map(|o| &o.nm_profiles);
        let active = adapter.nm_connection.clone();

        // Profiles we changed earlier that are not active now (the adapter roamed): put their
        // value back without activating them. Stop at the first failure so the record is kept.
        let inactive: Vec<(&String, &String)> = profiles
            .into_iter()
            .flatten()
            .filter(|(uuid, _)| active.as_ref().is_none_or(|c| &c.uuid != *uuid))
            .collect();
        if !inactive.is_empty() {
            if Self::nmcli().is_none() || !Self::nm_running() {
                return Err(NetError::NmUnavailable(
                    "NetworkManager is needed to undo changes to saved connections".into(),
                ));
            }
            for (uuid, value) in inactive {
                // A profile that no longer exists has nothing left to undo.
                if let Some(conn) = Self::nm_connection(uuid)
                    && conn.cloned_mac != *value
                {
                    progress(Stage::Writing);
                    Self::nm_set_cloned(&conn, value)?;
                }
            }
        }

        // The active profile: undo our change, or remove an explicit address someone set on it.
        if let Some(conn) = active {
            let recorded = profiles.and_then(|p| p.get(&conn.uuid)).cloned();
            let explicit = conn.cloned_mac.parse::<MacAddress>().is_ok();
            if recorded.is_some() || explicit {
                let original_value = recorded.unwrap_or_default();
                progress(Stage::Writing);
                let expected = if nm_value_keeps_current(&original_value) {
                    // "" / "preserve" would keep our address: force the hardware address first,
                    // then put the profile back exactly as it was without re-activating.
                    Self::nm_set_cloned(&conn, "permanent")?;
                    progress(Stage::Restarting);
                    Self::nm_up(&conn)?;
                    Self::nm_set_cloned(&conn, &original_value)?;
                    adapter.permanent_mac
                } else {
                    Self::nm_set_cloned(&conn, &original_value)?;
                    progress(Stage::Restarting);
                    Self::nm_up(&conn)?;
                    match original_value.as_str() {
                        "permanent" => adapter.permanent_mac,
                        // "random"/"stable": NetworkManager picks the address itself.
                        v => v.parse().ok(),
                    }
                };
                progress(Stage::Verifying);
                let (status, observed) = match expected {
                    Some(e) => {
                        let observed =
                            self.wait_for(&adapter.interface_name, e, opts.verify_timeout);
                        let status = if observed == Some(e) {
                            ChangeStatus::Applied
                        } else {
                            ChangeStatus::Unverified
                        };
                        (status, observed)
                    }
                    None => (
                        ChangeStatus::Applied,
                        self.current_address(&adapter.interface_name),
                    ),
                };
                let mut report = Self::report(
                    ChangeMethod::NetworkManager,
                    status,
                    previous,
                    expected,
                    observed,
                );
                report.nm_connection_uuid = Some(conn.uuid);
                return Ok(report);
            }
            // NetworkManager's own privacy setting chose this address: leave it alone.
            if original.and_then(|o| o.mac).is_none()
                && matches!(conn.cloned_mac.as_str(), "random" | "stable")
            {
                let at_hardware = previous.is_some() && previous == adapter.permanent_mac;
                let mut report = Self::report(
                    ChangeMethod::NetworkManager,
                    if at_hardware {
                        ChangeStatus::Applied
                    } else {
                        ChangeStatus::Unverified
                    },
                    previous,
                    adapter.permanent_mac,
                    previous,
                );
                report.detail_code = Some(
                    if at_hardware {
                        "NOTHING_TO_RESTORE"
                    } else {
                        "CHANGED_BY_OS"
                    }
                    .into(),
                );
                return Ok(report);
            }
        }

        let target = original
            .and_then(|o| o.mac)
            .or(adapter.permanent_mac)
            .ok_or(NetError::NoOriginal)?;
        if adapter.current_mac == Some(target) {
            return Ok(Self::report(
                ChangeMethod::Iproute2,
                ChangeStatus::Applied,
                previous,
                Some(target),
                Some(target),
            ));
        }
        self.apply_ip(adapter, target, opts, progress)
    }
}

/// Root of the sysfs tree used by tests that point the backend at fixtures.
#[cfg(test)]
impl LinuxBackend {
    pub fn with_roots(class_net: &std::path::Path, udev_data: &std::path::Path) -> Self {
        Self {
            class_net: class_net.to_path_buf(),
            udev_data: udev_data.to_path_buf(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loopback_has_no_permanent_address() {
        assert_eq!(permanent_address("lo"), None);
        assert_eq!(permanent_address("bad name"), None);
    }

    #[test]
    fn enumerates_fixture_without_nm() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let net = tmp.path().join("net");
        std::fs::create_dir_all(net.join("eth9/device")).expect("mkdir");
        std::fs::write(net.join("eth9/address"), "52:54:00:12:34:56\n").expect("w");
        std::fs::write(net.join("eth9/type"), "1\n").expect("w");
        std::fs::write(net.join("eth9/operstate"), "up\n").expect("w");
        std::fs::write(net.join("eth9/flags"), "0x1003\n").expect("w");
        std::fs::write(net.join("eth9/addr_assign_type"), "0\n").expect("w");
        let backend = LinuxBackend::with_roots(&net, tmp.path());
        let ifaces = read_interfaces(&net).expect("read");
        assert_eq!(ifaces.len(), 1);
        assert_eq!(backend.description(&ifaces[0]), "Network interface");
    }
}
