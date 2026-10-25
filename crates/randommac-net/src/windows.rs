//! Windows backend.
//!
//! * Enumeration: SetupAPI over the Net device class (includes disabled adapters), the driver
//!   key opened with `SetupDiOpenDevRegKey(DIREG_DRV)` (exactly this device's
//!   `Class\{4d36e972-…}\NNNN` key — fixes v1's wrong-subkey matching), `GetIfTable2` for live
//!   state and the burned-in address, `GetAdaptersAddresses` for IPs.
//! * Change: write the `NetworkAddress` driver property, restart the device with
//!   `DIF_PROPERTYCHANGE`/`DICS_PROPCHANGE` (what Device Manager does), poll `GetIfEntry2` until
//!   the new address shows up and roll back if the driver ignores it.

use std::collections::HashMap;
use std::ffi::CStr;
use std::mem::size_of;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::ptr::{null, null_mut};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use randommac_core::{ChangeStatus, MacAddress, OriginalRecord};
use windows_sys::Win32::Devices::DeviceAndDriverInstallation as di;
use windows_sys::Win32::Foundation as f;
use windows_sys::Win32::NetworkManagement::IpHelper as ip;
use windows_sys::Win32::NetworkManagement::Ndis as ndis;
use windows_sys::Win32::Networking::WinSock as ws;
use windows_sys::Win32::Security as sec;
use windows_sys::Win32::System::Registry as reg;
use windows_sys::Win32::System::Threading as th;
use windows_sys::core::GUID;

use crate::NetworkBackend;
use crate::error::NetError;
use crate::types::*;
use crate::win_classify::*;

const RESTART_TIMEOUT: Duration = Duration::from_secs(90);
const NETWORK_KEY: &str =
    r"SYSTEM\CurrentControlSet\Control\Network\{4D36E972-E325-11CE-BFC1-08002BE10318}";

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn from_wide(buf: &[u16]) -> String {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end])
}

fn last_error(context: &str) -> NetError {
    // SAFETY: GetLastError has no preconditions.
    let code = unsafe { f::GetLastError() };
    os_error(context, code)
}

fn os_error(context: &str, code: u32) -> NetError {
    const ERROR_ACCESS_DENIED: u32 = 5;
    if code == ERROR_ACCESS_DENIED {
        NetError::PermissionDenied(context.to_string())
    } else {
        NetError::Os {
            context: context.to_string(),
            code,
        }
    }
}

/// Whether the current process token is elevated (UAC "Run as administrator").
pub fn is_elevated() -> bool {
    let mut token: f::HANDLE = null_mut();
    // SAFETY: GetCurrentProcess returns a pseudo handle; token receives a real handle we close.
    if unsafe { th::OpenProcessToken(th::GetCurrentProcess(), sec::TOKEN_QUERY, &mut token) } == 0 {
        return false;
    }
    let mut elevation = sec::TOKEN_ELEVATION::default();
    let mut returned = 0u32;
    // SAFETY: elevation is a properly sized TOKEN_ELEVATION buffer.
    let ok = unsafe {
        sec::GetTokenInformation(
            token,
            sec::TokenElevation,
            (&mut elevation as *mut sec::TOKEN_ELEVATION).cast(),
            size_of::<sec::TOKEN_ELEVATION>() as u32,
            &mut returned,
        )
    };
    // SAFETY: token is a valid handle owned here.
    unsafe { f::CloseHandle(token) };
    ok != 0 && elevation.TokenIsElevated != 0
}

// ---------------------------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------------------------

struct RegKey(reg::HKEY);

impl Drop for RegKey {
    fn drop(&mut self) {
        // SAFETY: the key was opened by us and is closed exactly once.
        unsafe { reg::RegCloseKey(self.0) };
    }
}

impl RegKey {
    fn open(root: reg::HKEY, path: &str, access: u32) -> Option<Self> {
        let mut key: reg::HKEY = null_mut();
        let p = wide(path);
        // SAFETY: p is NUL-terminated; key receives a handle on success.
        let rc = unsafe { reg::RegOpenKeyExW(root, p.as_ptr(), 0, access, &mut key) };
        (rc == f::ERROR_SUCCESS).then_some(Self(key))
    }

    fn query(&self, name: &str) -> Option<(u32, Vec<u8>)> {
        let n = wide(name);
        let mut kind = 0u32;
        let mut len = 0u32;
        // SAFETY: size query with a null data pointer.
        let rc = unsafe {
            reg::RegQueryValueExW(self.0, n.as_ptr(), null(), &mut kind, null_mut(), &mut len)
        };
        if rc != f::ERROR_SUCCESS || len > 64 * 1024 {
            return None;
        }
        let mut buf = vec![0u8; len as usize + 2];
        let mut len2 = len;
        // SAFETY: buf has at least len2 bytes.
        let rc = unsafe {
            reg::RegQueryValueExW(
                self.0,
                n.as_ptr(),
                null(),
                &mut kind,
                buf.as_mut_ptr(),
                &mut len2,
            )
        };
        if rc != f::ERROR_SUCCESS {
            return None;
        }
        buf.truncate(len2 as usize);
        Some((kind, buf))
    }

    fn get_string(&self, name: &str) -> Option<String> {
        let (kind, buf) = self.query(name)?;
        if kind != reg::REG_SZ && kind != reg::REG_EXPAND_SZ {
            return None;
        }
        let units: Vec<u16> = buf
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| u16::from_le_bytes(*c))
            .collect();
        Some(from_wide(&units).trim().to_string())
    }

    fn get_dword(&self, name: &str) -> Option<u32> {
        let (kind, buf) = self.query(name)?;
        if kind == reg::REG_DWORD && buf.len() >= 4 {
            Some(u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]))
        } else if kind == reg::REG_SZ {
            let units: Vec<u16> = buf
                .as_chunks::<2>()
                .0
                .iter()
                .map(|c| u16::from_le_bytes(*c))
                .collect();
            from_wide(&units).trim().parse().ok()
        } else {
            None
        }
    }

    fn set_string(&self, name: &str, value: &str) -> Result<(), NetError> {
        let n = wide(name);
        let v = wide(value);
        // SAFETY: v is a NUL-terminated UTF-16 buffer of the size we pass.
        let rc = unsafe {
            reg::RegSetValueExW(
                self.0,
                n.as_ptr(),
                0,
                reg::REG_SZ,
                v.as_ptr().cast(),
                (v.len() * 2) as u32,
            )
        };
        if rc == f::ERROR_SUCCESS {
            Ok(())
        } else {
            Err(os_error("writing NetworkAddress", rc))
        }
    }

    fn delete_value(&self, name: &str) -> Result<(), NetError> {
        let n = wide(name);
        // SAFETY: n is NUL-terminated.
        let rc = unsafe { reg::RegDeleteValueW(self.0, n.as_ptr()) };
        if rc == f::ERROR_SUCCESS || rc == f::ERROR_FILE_NOT_FOUND {
            Ok(())
        } else {
            Err(os_error("deleting NetworkAddress", rc))
        }
    }

    fn has_subkey(&self, path: &str) -> bool {
        Self::open(self.0, path, reg::KEY_READ).is_some()
    }
}

fn connection_name(guid: &str) -> Option<String> {
    RegKey::open(
        reg::HKEY_LOCAL_MACHINE,
        &format!(r"{NETWORK_KEY}\{guid}\Connection"),
        reg::KEY_READ,
    )?
    .get_string("Name")
    .filter(|s| !s.is_empty())
}

// ---------------------------------------------------------------------------------------------
// SetupAPI
// ---------------------------------------------------------------------------------------------

struct DevInfoSet(di::HDEVINFO);

impl Drop for DevInfoSet {
    fn drop(&mut self) {
        // SAFETY: the set was created by us and is destroyed exactly once.
        unsafe { di::SetupDiDestroyDeviceInfoList(self.0) };
    }
}

fn new_devinfo_data() -> di::SP_DEVINFO_DATA {
    di::SP_DEVINFO_DATA {
        cbSize: size_of::<di::SP_DEVINFO_DATA>() as u32,
        ..Default::default()
    }
}

impl DevInfoSet {
    fn present_net_devices() -> Result<Self, NetError> {
        // SAFETY: valid class GUID pointer; no enumerator or window.
        let h = unsafe {
            di::SetupDiGetClassDevsW(
                &di::GUID_DEVCLASS_NET,
                null(),
                null_mut(),
                di::DIGCF_PRESENT,
            )
        };
        if h == -1 {
            return Err(last_error("SetupDiGetClassDevsW"));
        }
        Ok(Self(h))
    }

    /// Opens a set containing just the device with the given instance id.
    fn open_instance(instance_id: &str) -> Result<(Self, di::SP_DEVINFO_DATA), NetError> {
        // SAFETY: valid class GUID pointer; no window.
        let h = unsafe { di::SetupDiCreateDeviceInfoList(&di::GUID_DEVCLASS_NET, null_mut()) };
        if h == -1 {
            return Err(last_error("SetupDiCreateDeviceInfoList"));
        }
        let set = Self(h);
        let mut data = new_devinfo_data();
        let id = wide(instance_id);
        // SAFETY: id is NUL-terminated and data has the correct cbSize.
        if unsafe { di::SetupDiOpenDeviceInfoW(set.0, id.as_ptr(), null_mut(), 0, &mut data) } == 0
        {
            return Err(NetError::NotFound(instance_id.to_string()));
        }
        Ok((set, data))
    }

    fn device(&self, index: u32) -> Option<di::SP_DEVINFO_DATA> {
        let mut data = new_devinfo_data();
        // SAFETY: data has the correct cbSize.
        (unsafe { di::SetupDiEnumDeviceInfo(self.0, index, &mut data) } != 0).then_some(data)
    }

    fn instance_id(&self, data: &di::SP_DEVINFO_DATA) -> Option<String> {
        let mut buf = [0u16; 512];
        let mut required = 0u32;
        // SAFETY: buf length is passed correctly.
        let ok = unsafe {
            di::SetupDiGetDeviceInstanceIdW(
                self.0,
                data,
                buf.as_mut_ptr(),
                buf.len() as u32,
                &mut required,
            )
        };
        (ok != 0).then(|| from_wide(&buf))
    }

    fn driver_key(&self, data: &di::SP_DEVINFO_DATA, write: bool) -> Option<RegKey> {
        let access = reg::KEY_READ | if write { reg::KEY_SET_VALUE } else { 0 };
        // SAFETY: data belongs to this set.
        let key = unsafe {
            di::SetupDiOpenDevRegKey(self.0, data, di::DICS_FLAG_GLOBAL, 0, di::DIREG_DRV, access)
        };
        if key.is_null() || key == f::INVALID_HANDLE_VALUE {
            None
        } else {
            Some(RegKey(key))
        }
    }

    fn is_disabled(data: &di::SP_DEVINFO_DATA) -> bool {
        let mut status = 0u32;
        let mut problem = 0u32;
        // SAFETY: DevInst comes from SetupAPI.
        let cr = unsafe { di::CM_Get_DevNode_Status(&mut status, &mut problem, data.DevInst, 0) };
        cr == di::CR_SUCCESS && problem == di::CM_PROB_DISABLED
    }

    /// Calls the class installer with a property-change request. Returns whether Windows asks
    /// for a reboot before the change takes effect.
    fn property_change(
        &self,
        data: &di::SP_DEVINFO_DATA,
        state: u32,
        scope: u32,
    ) -> Result<bool, NetError> {
        let params = di::SP_PROPCHANGE_PARAMS {
            ClassInstallHeader: di::SP_CLASSINSTALL_HEADER {
                cbSize: size_of::<di::SP_CLASSINSTALL_HEADER>() as u32,
                InstallFunction: di::DIF_PROPERTYCHANGE,
            },
            StateChange: state,
            Scope: scope,
            HwProfile: 0,
        };
        // SAFETY: params outlives the call and the size matches the structure.
        if unsafe {
            di::SetupDiSetClassInstallParamsW(
                self.0,
                data,
                &params.ClassInstallHeader,
                size_of::<di::SP_PROPCHANGE_PARAMS>() as u32,
            )
        } == 0
        {
            return Err(last_error("SetupDiSetClassInstallParamsW"));
        }
        // SAFETY: data belongs to this set.
        if unsafe { di::SetupDiCallClassInstaller(di::DIF_PROPERTYCHANGE, self.0, data) } == 0 {
            return Err(last_error("restarting the adapter"));
        }
        let mut install = di::SP_DEVINSTALL_PARAMS_W {
            cbSize: size_of::<di::SP_DEVINSTALL_PARAMS_W>() as u32,
            ..Default::default()
        };
        // SAFETY: install has the correct cbSize.
        let ok = unsafe { di::SetupDiGetDeviceInstallParamsW(self.0, data, &mut install) };
        Ok(ok != 0 && install.Flags & (di::DI_NEEDREBOOT | di::DI_NEEDRESTART) != 0)
    }
}

/// Runs a device state change on a worker thread with a watchdog: the class installer is
/// synchronous and some drivers hang.
fn change_device_state(instance_id: &str, state: u32, scope: u32) -> Result<bool, NetError> {
    let (tx, rx) = mpsc::channel();
    let id = instance_id.to_string();
    thread::spawn(move || {
        let result = DevInfoSet::open_instance(&id)
            .and_then(|(set, data)| set.property_change(&data, state, scope));
        let _ = tx.send(result);
    });
    rx.recv_timeout(RESTART_TIMEOUT)
        .map_err(|_| NetError::Timeout("restarting the adapter".into()))?
}

fn restart_device(instance_id: &str) -> Result<bool, NetError> {
    change_device_state(
        instance_id,
        di::DICS_PROPCHANGE,
        di::DICS_FLAG_CONFIGSPECIFIC,
    )
}

// ---------------------------------------------------------------------------------------------
// IP Helper
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct IfRow {
    alias: String,
    current: Option<MacAddress>,
    permanent: Option<MacAddress>,
    up: bool,
    media_connected: bool,
    hardware: bool,
    filter: bool,
}

fn row_from(r: &ip::MIB_IF_ROW2) -> IfRow {
    let flags = r.InterfaceAndOperStatusFlags._bitfield;
    IfRow {
        alias: from_wide(&r.Alias),
        current: mac_from_row(r.PhysicalAddressLength, &r.PhysicalAddress),
        permanent: mac_from_row(r.PhysicalAddressLength, &r.PermanentPhysicalAddress),
        up: r.OperStatus == ndis::IfOperStatusUp,
        media_connected: r.MediaConnectState == ndis::MediaConnectStateConnected,
        hardware: flags & 0x1 != 0,
        filter: flags & 0x2 != 0,
    }
}

fn guid_to_string(g: &GUID) -> String {
    format!(
        "{{{:08X}-{:04X}-{:04X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}}}",
        g.data1,
        g.data2,
        g.data3,
        g.data4[0],
        g.data4[1],
        g.data4[2],
        g.data4[3],
        g.data4[4],
        g.data4[5],
        g.data4[6],
        g.data4[7]
    )
}

/// Live interface rows keyed by `{GUID}` (filter-module rows are skipped).
fn interface_rows() -> HashMap<String, IfRow> {
    let mut map = HashMap::new();
    let mut table: *mut ip::MIB_IF_TABLE2 = null_mut();
    // SAFETY: GetIfTable2 allocates the table, which we free with FreeMibTable.
    if unsafe { ip::GetIfTable2(&mut table) } != f::ERROR_SUCCESS || table.is_null() {
        return map;
    }
    // SAFETY: table points to NumEntries consecutive rows.
    let rows = unsafe {
        std::slice::from_raw_parts((*table).Table.as_ptr(), (*table).NumEntries as usize)
    };
    for r in rows {
        let row = row_from(r);
        if row.filter {
            continue;
        }
        let key = guid_to_string(&r.InterfaceGuid);
        let replace = match map.get(&key) {
            None => true,
            Some(existing) => !existing.hardware && row.hardware,
        };
        if replace {
            map.insert(key, row);
        }
    }
    // SAFETY: table was allocated by GetIfTable2.
    unsafe { ip::FreeMibTable(table.cast()) };
    map
}

/// Current row of a single interface, looked up by its LUID (the IfIndex can change when the
/// device restarts).
fn interface_row(guid: &str) -> Option<IfRow> {
    let (d1, d2, d3, d4) = parse_guid(guid)?;
    let g = GUID {
        data1: d1,
        data2: d2,
        data3: d3,
        data4: d4,
    };
    // SAFETY: zeroed MIB_IF_ROW2/NET_LUID are valid "empty" values for these APIs.
    let mut luid: ndis::NET_LUID_LH = unsafe { std::mem::zeroed() };
    // SAFETY: g and luid are valid for the call.
    if unsafe { ip::ConvertInterfaceGuidToLuid(&g, &mut luid) } != f::ERROR_SUCCESS {
        return None;
    }
    // SAFETY: see above.
    let mut row: ip::MIB_IF_ROW2 = unsafe { std::mem::zeroed() };
    row.InterfaceLuid = luid;
    // SAFETY: row has its LUID set as required by GetIfEntry2.
    (unsafe { ip::GetIfEntry2(&mut row) } == f::ERROR_SUCCESS).then(|| row_from(&row))
}

fn wait_for_address(guid: &str, expected: MacAddress, timeout: Duration) -> Option<MacAddress> {
    let deadline = Instant::now() + timeout;
    let mut last = None;
    loop {
        if let Some(row) = interface_row(guid) {
            last = row.current;
            if row.current == Some(expected) {
                return last;
            }
        }
        if Instant::now() >= deadline {
            return last;
        }
        thread::sleep(Duration::from_millis(500));
    }
}

/// IPv4/IPv6 addresses keyed by `{GUID}`.
fn ip_addresses() -> HashMap<String, (Vec<String>, Vec<String>)> {
    let mut map: HashMap<String, (Vec<String>, Vec<String>)> = HashMap::new();
    let flags =
        ip::GAA_FLAG_SKIP_ANYCAST | ip::GAA_FLAG_SKIP_MULTICAST | ip::GAA_FLAG_SKIP_DNS_SERVER;
    let mut size: u32 = 16 * 1024;
    let mut buf: Vec<u64> = Vec::new();
    for _ in 0..4 {
        buf = vec![0u64; (size as usize).div_ceil(8)];
        // SAFETY: buf is 8-byte aligned and `size` bytes long.
        let rc = unsafe {
            ip::GetAdaptersAddresses(
                u32::from(ws::AF_UNSPEC),
                flags,
                null(),
                buf.as_mut_ptr().cast(),
                &mut size,
            )
        };
        if rc == f::ERROR_SUCCESS {
            break;
        }
        if rc != f::ERROR_BUFFER_OVERFLOW {
            return map;
        }
    }
    let mut cur = buf.as_ptr() as *const ip::IP_ADAPTER_ADDRESSES_LH;
    while !cur.is_null() {
        // SAFETY: cur walks the linked list written into buf.
        let a = unsafe { &*cur };
        if !a.AdapterName.is_null() {
            // SAFETY: AdapterName is a NUL-terminated ANSI string inside buf.
            let name = unsafe { CStr::from_ptr(a.AdapterName.cast()) }
                .to_string_lossy()
                .to_ascii_uppercase();
            let entry = map.entry(normalize_guid(&name)).or_default();
            let mut ucast = a.FirstUnicastAddress;
            while !ucast.is_null() {
                // SAFETY: unicast list nodes live inside buf.
                let u = unsafe { &*ucast };
                let sa = u.Address.lpSockaddr;
                if !sa.is_null() {
                    // SAFETY: sa points to a SOCKADDR whose family selects the concrete type.
                    let family = unsafe { (*sa).sa_family };
                    if family == ws::AF_INET {
                        // SAFETY: AF_INET means SOCKADDR_IN.
                        let sin = unsafe { &*(sa as *const ws::SOCKADDR_IN) };
                        // SAFETY: reading the u32 view of the address union.
                        let raw = unsafe { sin.sin_addr.S_un.S_addr };
                        entry.0.push(Ipv4Addr::from(u32::from_be(raw)).to_string());
                    } else if family == ws::AF_INET6 {
                        // SAFETY: AF_INET6 means SOCKADDR_IN6.
                        let sin6 = unsafe { &*(sa as *const ws::SOCKADDR_IN6) };
                        // SAFETY: reading the byte view of the address union.
                        let bytes = unsafe { sin6.sin6_addr.u.Byte };
                        entry.1.push(Ipv6Addr::from(bytes).to_string());
                    }
                }
                ucast = u.Next;
            }
        }
        cur = a.Next;
    }
    map
}

// ---------------------------------------------------------------------------------------------
// Backend
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Default)]
pub struct WindowsBackend;

impl WindowsBackend {
    pub fn new() -> Self {
        Self
    }

    fn require_elevation() -> Result<(), NetError> {
        if is_elevated() {
            Ok(())
        } else {
            Err(NetError::NotElevated)
        }
    }

    /// Opens the driver key of the adapter for writing and returns it with its interface GUID.
    fn open_for_change(instance_id: &str) -> Result<(RegKey, String), NetError> {
        let (set, data) = DevInfoSet::open_instance(instance_id)?;
        let key = set
            .driver_key(&data, true)
            .ok_or_else(|| NetError::PermissionDenied("opening the driver key".into()))?;
        let guid = key
            .get_string("NetCfgInstanceId")
            .map(|g| normalize_guid(&g))
            .ok_or_else(|| NetError::NotFound(instance_id.to_string()))?;
        Ok((key, guid))
    }

    fn write_override(instance_id: &str, value: Option<&str>) -> Result<(), NetError> {
        let (key, _) = Self::open_for_change(instance_id)?;
        match value {
            Some(v) if !v.is_empty() => key.set_string("NetworkAddress", v),
            _ => key.delete_value("NetworkAddress"),
        }
    }

    fn report(
        status: ChangeStatus,
        previous: Option<MacAddress>,
        requested: Option<MacAddress>,
        observed: Option<MacAddress>,
        detail: Option<&str>,
    ) -> ChangeReport {
        ChangeReport {
            method: ChangeMethod::WindowsRegistry,
            status,
            previous_mac: previous,
            requested_mac: requested,
            observed_mac: observed,
            detail_code: detail.map(str::to_string),
            nm_connection_uuid: None,
            nm_previous_value: None,
            temporary: false,
        }
    }
}

impl NetworkBackend for WindowsBackend {
    fn capabilities(&self) -> Capabilities {
        let elevated = is_elevated();
        Capabilities {
            platform: "windows",
            elevated,
            can_change: elevated,
            network_manager: false,
            iproute2: false,
            pkexec: false,
        }
    }

    fn list_adapters(&self) -> Result<Vec<Adapter>, NetError> {
        let set = DevInfoSet::present_net_devices()?;
        let rows = interface_rows();
        let ips = ip_addresses();
        let mut out = Vec::new();
        let mut index = 0;
        while let Some(data) = set.device(index) {
            index += 1;
            let Some(id) = set.instance_id(&data) else {
                continue;
            };
            let Some(key) = set.driver_key(&data, false) else {
                continue;
            };
            let characteristics = key.get_dword("Characteristics").unwrap_or(0);
            if is_hidden(characteristics) {
                continue;
            }
            let Some(guid) = key
                .get_string("NetCfgInstanceId")
                .map(|g| normalize_guid(&g))
            else {
                continue;
            };
            let if_type = key.get_dword("*IfType");
            let medium = key.get_dword("*PhysicalMediaType");
            if !is_relevant(if_type, medium) {
                continue;
            }
            let description = key.get_string("DriverDesc").unwrap_or_default();
            let override_value = key.get_string("NetworkAddress").filter(|v| !v.is_empty());
            let advertised = key.has_subkey(r"Ndi\params\NetworkAddress");
            let disabled = DevInfoSet::is_disabled(&data);
            let row = rows.get(&guid);
            let name = connection_name(&guid)
                .or_else(|| row.map(|r| r.alias.clone()).filter(|a| !a.is_empty()))
                .unwrap_or_else(|| description.clone());
            let (ipv4, ipv6) = ips.get(&guid).cloned().unwrap_or_default();
            let is_virtual = is_virtual(characteristics, &id);
            let current = row.and_then(|r| r.current);
            out.push(Adapter {
                id,
                name: name.clone(),
                description,
                interface_name: name,
                kind: kind_from(if_type, medium),
                is_virtual,
                state: if disabled {
                    AdapterState::Disabled
                } else {
                    match row {
                        Some(r) if r.up => AdapterState::Up,
                        Some(_) => AdapterState::Down,
                        None => AdapterState::Unknown,
                    }
                },
                connected: row.is_some_and(|r| r.up && r.media_connected),
                current_mac: current,
                permanent_mac: row.and_then(|r| r.permanent),
                override_value,
                supports_change: if advertised {
                    Support::Yes
                } else {
                    Support::Unknown
                },
                methods: vec![ChangeMethod::WindowsRegistry],
                nm_connection: None,
                driver: None,
                ipv4,
                ipv6,
            });
        }
        out.sort_by(|a, b| {
            (
                a.is_virtual,
                a.kind != AdapterKind::Ethernet && a.kind != AdapterKind::Wireless,
                a.name.to_lowercase(),
            )
                .cmp(&(
                    b.is_virtual,
                    b.kind != AdapterKind::Ethernet && b.kind != AdapterKind::Wireless,
                    b.name.to_lowercase(),
                ))
        });
        Ok(out)
    }

    fn apply(
        &self,
        adapter: &Adapter,
        mac: MacAddress,
        opts: &ChangeOptions,
        progress: &mut dyn FnMut(Stage),
    ) -> Result<ChangeReport, NetError> {
        Self::require_elevation()?;
        if adapter.state == AdapterState::Disabled {
            return Err(NetError::Unsupported(
                "the adapter is disabled; enable it first".into(),
            ));
        }
        progress(Stage::Preparing);
        let (key, guid) = Self::open_for_change(&adapter.id)?;
        let previous_value = key.get_string("NetworkAddress").filter(|v| !v.is_empty());
        let previous_mac = interface_row(&guid)
            .and_then(|r| r.current)
            .or(adapter.current_mac);
        progress(Stage::Writing);
        key.set_string("NetworkAddress", &mac.to_compact_upper())?;
        drop(key);

        progress(Stage::Restarting);
        let reboot = match restart_device(&adapter.id) {
            Ok(r) => r,
            Err(e) => {
                let _ = Self::write_override(&adapter.id, previous_value.as_deref());
                return Err(e);
            }
        };
        if reboot {
            // Nothing is observed yet: the new address takes effect after the reboot.
            return Ok(Self::report(
                ChangeStatus::RebootRequired,
                previous_mac,
                Some(mac),
                None,
                Some("REBOOT_REQUIRED"),
            ));
        }

        progress(Stage::Verifying);
        let observed = wait_for_address(&guid, mac, opts.verify_timeout);
        if observed == Some(mac) {
            return Ok(Self::report(
                ChangeStatus::Applied,
                previous_mac,
                Some(mac),
                observed,
                None,
            ));
        }
        if observed.is_none() {
            // The adapter did not report back (e.g. still initializing); keep the change.
            return Ok(Self::report(
                ChangeStatus::Unverified,
                previous_mac,
                Some(mac),
                None,
                Some("VERIFY_TIMEOUT"),
            ));
        }
        // The driver (or Windows' own Wi-Fi randomization) ignored the address: roll back.
        progress(Stage::RollingBack);
        Self::write_override(&adapter.id, previous_value.as_deref())
            .and_then(|()| restart_device(&adapter.id).map(|_| ()))
            .map_err(|e| NetError::RollbackFailed(e.to_string()))?;
        Ok(Self::report(
            ChangeStatus::RolledBack,
            previous_mac,
            Some(mac),
            observed,
            Some("DRIVER_REJECTED"),
        ))
    }

    fn restore(
        &self,
        adapter: &Adapter,
        _original: Option<&OriginalRecord>,
        opts: &ChangeOptions,
        progress: &mut dyn FnMut(Stage),
    ) -> Result<ChangeReport, NetError> {
        Self::require_elevation()?;
        progress(Stage::Preparing);
        let (key, guid) = Self::open_for_change(&adapter.id)?;
        let override_value = key.get_string("NetworkAddress").filter(|v| !v.is_empty());
        let row = interface_row(&guid);
        let previous = row.as_ref().and_then(|r| r.current).or(adapter.current_mac);
        let permanent = row
            .as_ref()
            .and_then(|r| r.permanent)
            .or(adapter.permanent_mac);
        if override_value.is_none() {
            drop(key);
            // Nothing of ours to undo; report whether the address is the hardware one.
            let matches = previous.is_some() && previous == permanent;
            return Ok(Self::report(
                if matches {
                    ChangeStatus::Applied
                } else {
                    ChangeStatus::Unverified
                },
                previous,
                permanent,
                previous,
                Some(if matches {
                    "NOTHING_TO_RESTORE"
                } else {
                    "CHANGED_BY_OS"
                }),
            ));
        }
        progress(Stage::Writing);
        key.delete_value("NetworkAddress")?;
        drop(key);
        progress(Stage::Restarting);
        if restart_device(&adapter.id)? {
            return Ok(Self::report(
                ChangeStatus::RebootRequired,
                previous,
                permanent,
                None,
                Some("REBOOT_REQUIRED"),
            ));
        }
        progress(Stage::Verifying);
        let observed = match permanent {
            Some(p) => wait_for_address(&guid, p, opts.verify_timeout),
            None => {
                thread::sleep(Duration::from_secs(2));
                interface_row(&guid).and_then(|r| r.current)
            }
        };
        let status = match permanent {
            Some(p) if observed == Some(p) => ChangeStatus::Applied,
            _ => ChangeStatus::Unverified,
        };
        Ok(Self::report(status, previous, permanent, observed, None))
    }

    fn set_enabled(&self, adapter: &Adapter, enabled: bool) -> Result<(), NetError> {
        Self::require_elevation()?;
        if enabled {
            change_device_state(&adapter.id, di::DICS_ENABLE, di::DICS_FLAG_GLOBAL)?;
            let _ = change_device_state(&adapter.id, di::DICS_ENABLE, di::DICS_FLAG_CONFIGSPECIFIC);
        } else {
            change_device_state(&adapter.id, di::DICS_DISABLE, di::DICS_FLAG_GLOBAL)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enumerates_without_admin() {
        let backend = WindowsBackend::new();
        let adapters = backend
            .list_adapters()
            .expect("enumeration works unelevated");
        for a in &adapters {
            assert!(!a.id.is_empty() && !a.name.is_empty());
            assert!(matches!(
                a.kind,
                AdapterKind::Ethernet | AdapterKind::Wireless | AdapterKind::Other
            ));
        }
        let _ = backend.capabilities();
    }

    #[test]
    fn guid_formatting_round_trips() {
        let s = "{59AB0DC4-AC80-4C28-8DF3-D2CF25B40A3F}";
        let (d1, d2, d3, d4) = parse_guid(s).expect("guid");
        let g = GUID {
            data1: d1,
            data2: d2,
            data3: d3,
            data4: d4,
        };
        assert_eq!(guid_to_string(&g), s);
    }

    /// A device instance id that does not exist: every change stops before touching anything.
    fn fake_adapter(state: AdapterState) -> Adapter {
        Adapter {
            id: r"PCI\RANDOMMAC_TEST_DOES_NOT_EXIST\0".into(),
            name: "x".into(),
            description: String::new(),
            interface_name: "x".into(),
            kind: AdapterKind::Ethernet,
            is_virtual: false,
            state,
            connected: false,
            current_mac: None,
            permanent_mac: None,
            override_value: None,
            supports_change: Support::Unknown,
            methods: vec![],
            nm_connection: None,
            driver: None,
            ipv4: vec![],
            ipv6: vec![],
        }
    }

    #[test]
    fn changes_check_elevation_and_the_device() {
        let backend = WindowsBackend::new();
        let mac: MacAddress = "02:00:00:00:00:01".parse().expect("mac");
        let opts = ChangeOptions::default();
        let mut stages = Vec::new();
        let mut progress = |s: Stage| stages.push(s);
        let up = fake_adapter(AdapterState::Up);
        let disabled = fake_adapter(AdapterState::Disabled);
        let results = [
            backend.apply(&up, mac, &opts, &mut progress).map(|_| ()),
            backend
                .apply(&disabled, mac, &opts, &mut progress)
                .map(|_| ()),
            backend.restore(&up, None, &opts, &mut progress).map(|_| ()),
            backend.set_enabled(&up, true),
            backend.set_enabled(&up, false),
        ];
        if is_elevated() {
            let not_found = NetError::NotFound(up.id.clone());
            assert_eq!(results[0], Err(not_found.clone()));
            assert_eq!(
                results[1].as_ref().map_err(NetError::code),
                Err("UNSUPPORTED")
            );
            assert_eq!(results[2], Err(not_found.clone()));
            assert_eq!(results[3], Err(not_found.clone()));
            assert_eq!(results[4], Err(not_found));
            assert_eq!(stages, [Stage::Preparing, Stage::Preparing]);
        } else {
            assert!(results.iter().all(|r| r == &Err(NetError::NotElevated)));
            assert!(stages.is_empty());
        }
        assert_eq!(backend.capabilities().can_change, is_elevated());
    }

    #[test]
    fn unknown_devices_and_interfaces() {
        let id = r"PCI\RANDOMMAC_TEST_DOES_NOT_EXIST\0";
        assert_eq!(
            DevInfoSet::open_instance(id).map(|_| ()),
            Err(NetError::NotFound(id.into()))
        );
        assert_eq!(restart_device(id), Err(NetError::NotFound(id.into())));
        let guid = "{00000000-0000-0000-0000-00000000F00D}";
        assert!(interface_row(guid).is_none());
        let mac: MacAddress = "02:00:00:00:00:01".parse().expect("mac");
        assert_eq!(wait_for_address(guid, mac, Duration::from_millis(1)), None);
        assert!(parse_guid("not a guid").is_none());
    }

    #[test]
    fn os_errors_and_strings() {
        assert_eq!(os_error("x", 5), NetError::PermissionDenied("x".into()));
        assert_eq!(
            os_error("x", 87),
            NetError::Os {
                context: "x".into(),
                code: 87
            }
        );
        assert_eq!(from_wide(&wide("abc")), "abc");
        assert_eq!(from_wide(&[0x61, 0x62]), "ab");
        let report = WindowsBackend::report(ChangeStatus::RolledBack, None, None, None, Some("D"));
        assert_eq!(report.method, ChangeMethod::WindowsRegistry);
        assert_eq!(report.detail_code.as_deref(), Some("D"));
        assert!(!report.temporary);
    }

    #[test]
    fn reads_registry_values() {
        let key = RegKey::open(
            reg::HKEY_LOCAL_MACHINE,
            r"SOFTWARE\Microsoft\Windows NT\CurrentVersion",
            reg::KEY_READ,
        )
        .expect("CurrentVersion key");
        // REG_SZ, also readable as a number.
        let build = key.get_string("CurrentBuildNumber").expect("build number");
        assert_eq!(key.get_dword("CurrentBuildNumber"), build.parse().ok());
        // REG_DWORD.
        assert!(
            key.get_dword("CurrentMajorVersionNumber")
                .is_some_and(|v| v >= 10)
        );
        assert_eq!(key.get_string("CurrentMajorVersionNumber"), None);
        assert_eq!(key.get_string("RandomMacTestMissingValue"), None);
        assert_eq!(key.get_dword("RandomMacTestMissingValue"), None);
        // Binary values are neither.
        assert_eq!(key.get_dword("DigitalProductId"), None);
        assert!(
            RegKey::open(
                reg::HKEY_LOCAL_MACHINE,
                r"SOFTWARE\RandomMacTestMissing",
                reg::KEY_READ
            )
            .is_none()
        );
    }

    #[test]
    fn changes_require_elevation() {
        if is_elevated() {
            return;
        }
        let backend = WindowsBackend::new();
        let fake = Adapter {
            id: "PCI\\FAKE".into(),
            name: "x".into(),
            description: String::new(),
            interface_name: "x".into(),
            kind: AdapterKind::Ethernet,
            is_virtual: false,
            state: AdapterState::Up,
            connected: false,
            current_mac: None,
            permanent_mac: None,
            override_value: None,
            supports_change: Support::Unknown,
            methods: vec![],
            nm_connection: None,
            driver: None,
            ipv4: vec![],
            ipv6: vec![],
        };
        let err = backend
            .apply(
                &fake,
                "02:00:00:00:00:01".parse().expect("mac"),
                &ChangeOptions::default(),
                &mut |_| {},
            )
            .expect_err("not elevated");
        assert_eq!(err, NetError::NotElevated);
    }
}
