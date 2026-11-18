//! DNS settings through the IP Helper API: `GetInterfaceDnsSettings` / `SetInterfaceDnsSettings`
//! (Windows 10 2004 and later; DNS over HTTPS with `DNS_INTERFACE_SETTINGS3`, Windows 11).
//!
//! The three functions are looked up at run time: linking them would keep RandomMac from starting
//! at all on older Windows 10 builds. Each address family is a separate call; whenever the system
//! supports DoH the DoH settings are written too (empty for plain DNS), because Windows otherwise
//! keeps old DoH settings per server address.

use std::mem::{transmute, zeroed};
use std::sync::OnceLock;

use randommac_core::dns::{
    DnsConfig, DnsOriginalRecord, DnsSnapshot, DohServerSnapshot, FamilyDns, WinFamilySnapshot,
};
use windows_sys::Win32::System::LibraryLoader as ll;

use super::*;
use crate::dns_types::*;
use crate::win_dns::{self, FamilyCall};

type GetFn = unsafe extern "system" fn(GUID, *mut ip::DNS_INTERFACE_SETTINGS) -> f::WIN32_ERROR;
type SetFn = unsafe extern "system" fn(GUID, *const ip::DNS_INTERFACE_SETTINGS) -> f::WIN32_ERROR;
type FreeFn = unsafe extern "system" fn(*mut ip::DNS_INTERFACE_SETTINGS);

struct DnsApi {
    get: GetFn,
    set: SetFn,
    free: FreeFn,
}

/// The DNS functions of iphlpapi.dll, if this Windows has them.
fn api() -> Option<&'static DnsApi> {
    static API: OnceLock<Option<DnsApi>> = OnceLock::new();
    API.get_or_init(|| {
        let name = wide("iphlpapi.dll");
        // SAFETY: plain module lookups with a NUL-terminated name; iphlpapi.dll is already loaded
        // (RandomMac links other functions of it), LoadLibraryExW only searches System32.
        unsafe {
            let mut module = ll::GetModuleHandleW(name.as_ptr());
            if module.is_null() {
                module =
                    ll::LoadLibraryExW(name.as_ptr(), null_mut(), ll::LOAD_LIBRARY_SEARCH_SYSTEM32);
            }
            if module.is_null() {
                return None;
            }
            let get = ll::GetProcAddress(module, c"GetInterfaceDnsSettings".as_ptr().cast())?;
            let set = ll::GetProcAddress(module, c"SetInterfaceDnsSettings".as_ptr().cast())?;
            let free = ll::GetProcAddress(module, c"FreeInterfaceDnsSettings".as_ptr().cast())?;
            // SAFETY: the exports have exactly these signatures (netioapi.h).
            Some(DnsApi {
                get: transmute::<unsafe extern "system" fn() -> isize, GetFn>(get),
                set: transmute::<unsafe extern "system" fn() -> isize, SetFn>(set),
                free: transmute::<unsafe extern "system" fn() -> isize, FreeFn>(free),
            })
        }
    })
    .as_ref()
}

fn guid_of(text: &str) -> Result<GUID, NetError> {
    win_dns::parse_guid(text)
        .map(GUID::from_u128)
        .ok_or_else(|| NetError::InvalidInput(format!("not an interface GUID: {text}")))
}

/// Reads a NUL-terminated UTF-16 string ("" for a null pointer).
///
/// # Safety
/// `p` is null or points to a NUL-terminated string.
unsafe fn read_pwstr(p: *const u16) -> String {
    if p.is_null() {
        return String::new();
    }
    let mut len = 0;
    // SAFETY: the caller promises a terminated string; the length is capped.
    while len < 32_768 && unsafe { *p.add(len) } != 0 {
        len += 1;
    }
    // SAFETY: `len` units were just read.
    String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(p, len) })
}

/// The DoH properties of a server-property array.
///
/// # Safety
/// `props` is null or points to `count` initialized properties.
unsafe fn read_props(count: u32, props: *const ip::DNS_SERVER_PROPERTY) -> Vec<DohServerSnapshot> {
    if props.is_null() || count > 64 {
        return Vec::new();
    }
    // SAFETY: the caller promises `count` elements.
    let props = unsafe { std::slice::from_raw_parts(props, count as usize) };
    props
        .iter()
        .filter(|p| p.Type == ip::DnsServerDohProperty)
        .filter_map(|p| {
            // SAFETY: for the DoH type the union holds a DNS_DOH_SERVER_SETTINGS pointer.
            let doh = unsafe { p.Property.DohSettings };
            if doh.is_null() {
                return None;
            }
            // SAFETY: non-null pointer from the API's own allocation.
            let (template, flags) = unsafe { (read_pwstr((*doh).Template), (*doh).Flags) };
            Some(DohServerSnapshot {
                server_index: p.ServerIndex,
                template: Some(template).filter(|t| !t.is_empty()),
                flags,
            })
        })
        .collect()
}

/// One family's settings, and whether the DoH-capable version 3 of the API answered.
fn read_family(
    api: &DnsApi,
    guid: GUID,
    ipv6: bool,
) -> Result<(WinFamilySnapshot, bool), NetError> {
    let family_flag = if ipv6 {
        u64::from(ip::DNS_SETTING_IPV6)
    } else {
        0
    };
    // SAFETY: an all-zero settings structure is valid input with only Version and Flags set.
    let mut s3: ip::DNS_INTERFACE_SETTINGS3 = unsafe { zeroed() };
    s3.Version = ip::DNS_INTERFACE_SETTINGS_VERSION3;
    s3.Flags = family_flag;
    // SAFETY: `s3` is a version-3 structure, which the API recognizes by `Version`.
    let rc = unsafe { (api.get)(guid, (&raw mut s3).cast()) };
    if rc == f::ERROR_SUCCESS {
        // SAFETY: the API filled the structure; every pointer is null or its own allocation.
        let snap = unsafe {
            WinFamilySnapshot {
                name_server: read_pwstr(s3.NameServer),
                doh: read_props(s3.cServerProperties, s3.ServerProperties),
                profile_name_server: read_pwstr(s3.ProfileNameServer),
                profile_doh: read_props(s3.cProfileServerProperties, s3.ProfileServerProperties),
            }
        };
        // SAFETY: frees what GetInterfaceDnsSettings allocated, exactly once.
        unsafe { (api.free)((&raw mut s3).cast()) };
        return Ok((snap, true));
    }
    // Windows 10 before build 19645 has no version 3: read version 1 (no DoH).
    // SAFETY: as above, for a version-1 structure.
    let mut s1: ip::DNS_INTERFACE_SETTINGS = unsafe { zeroed() };
    s1.Version = ip::DNS_INTERFACE_SETTINGS_VERSION1;
    s1.Flags = family_flag;
    // SAFETY: `s1` is a version-1 structure.
    let rc = unsafe { (api.get)(guid, &raw mut s1) };
    if rc != f::ERROR_SUCCESS {
        return Err(os_error("GetInterfaceDnsSettings", rc));
    }
    // SAFETY: as above.
    let snap = unsafe {
        WinFamilySnapshot {
            name_server: read_pwstr(s1.NameServer),
            profile_name_server: read_pwstr(s1.ProfileNameServer),
            ..WinFamilySnapshot::default()
        }
    };
    // SAFETY: frees what GetInterfaceDnsSettings allocated, exactly once.
    unsafe { (api.free)(&raw mut s1) };
    Ok((snap, false))
}

fn read_both(
    api: &DnsApi,
    guid: GUID,
) -> Result<(WinFamilySnapshot, WinFamilySnapshot, bool), NetError> {
    let (v4, v3) = read_family(api, guid, false)?;
    let (v6, _) = read_family(api, guid, true)?;
    Ok((v4, v6, v3))
}

fn write_family(api: &DnsApi, guid: GUID, call: &FamilyCall, v3: bool) -> Result<(), NetError> {
    // Every buffer the structure points into lives until the call returns and is not touched
    // after its address is taken.
    let name = wide(&call.name_server);
    let templates: Vec<Option<Vec<u16>>> = call
        .doh
        .iter()
        .map(|d| d.template.as_deref().map(wide))
        .collect();
    let mut doh: Vec<ip::DNS_DOH_SERVER_SETTINGS> = call
        .doh
        .iter()
        .zip(&templates)
        .map(|(d, t)| ip::DNS_DOH_SERVER_SETTINGS {
            Template: t.as_ref().map_or(null_mut(), |t| t.as_ptr().cast_mut()),
            Flags: d.flags,
        })
        .collect();
    let mut props: Vec<ip::DNS_SERVER_PROPERTY> = call
        .doh
        .iter()
        .zip(doh.iter_mut())
        .map(|(d, settings)| ip::DNS_SERVER_PROPERTY {
            Version: ip::DNS_SERVER_PROPERTY_VERSION1,
            ServerIndex: d.server_index,
            Type: ip::DnsServerDohProperty,
            Property: ip::DNS_SERVER_PROPERTY_TYPES {
                DohSettings: settings,
            },
        })
        .collect();
    let mut flags = u64::from(ip::DNS_SETTING_NAMESERVER);
    if call.ipv6 {
        flags |= u64::from(ip::DNS_SETTING_IPV6);
    }
    let rc = if v3 && call.set_doh {
        // SAFETY: all-zero is valid; only the fields named by Flags are set.
        let mut s: ip::DNS_INTERFACE_SETTINGS3 = unsafe { zeroed() };
        s.Version = ip::DNS_INTERFACE_SETTINGS_VERSION3;
        s.Flags = flags | u64::from(ip::DNS_SETTING_DOH);
        s.NameServer = name.as_ptr().cast_mut();
        if !props.is_empty() {
            s.cServerProperties = u32::try_from(props.len()).unwrap_or(u32::MAX);
            s.ServerProperties = props.as_mut_ptr();
        }
        // SAFETY: `s` and everything it points to is valid for the duration of the call.
        unsafe { (api.set)(guid, (&raw const s).cast()) }
    } else {
        // SAFETY: as above, for a version-1 structure.
        let mut s: ip::DNS_INTERFACE_SETTINGS = unsafe { zeroed() };
        s.Version = ip::DNS_INTERFACE_SETTINGS_VERSION1;
        s.Flags = flags;
        s.NameServer = name.as_ptr().cast_mut();
        // SAFETY: `s` and the name buffer are valid for the duration of the call.
        unsafe { (api.set)(guid, &raw const s) }
    };
    if rc == f::ERROR_SUCCESS {
        Ok(())
    } else {
        let family = if call.ipv6 { "IPv6" } else { "IPv4" };
        Err(os_error(&format!("SetInterfaceDnsSettings ({family})"), rc))
    }
}

/// True when the IPv6 stack is bound to the interface.
fn ipv6_bound(guid: GUID) -> bool {
    let mut luid: f::LUID = unsafe { zeroed() };
    // SAFETY: both pointers refer to live locals.
    if unsafe { ip::ConvertInterfaceGuidToLuid(&guid, (&raw mut luid).cast()) } != f::ERROR_SUCCESS
    {
        return false;
    }
    // SAFETY: all-zero is a valid row; Family and InterfaceLuid select the entry.
    let mut row: ip::MIB_IPINTERFACE_ROW = unsafe { zeroed() };
    row.Family = ws::AF_INET6;
    // SAFETY: the NET_LUID_LH union is the same 64-bit value as LUID.
    row.InterfaceLuid = unsafe { transmute::<f::LUID, ndis::NET_LUID_LH>(luid) };
    // SAFETY: `row` is a live, initialized row.
    unsafe { ip::GetIpInterfaceEntry(&raw mut row) == f::ERROR_SUCCESS }
}

fn flush_cache() -> bool {
    crate::process::run(
        &system32("ipconfig.exe"),
        &["/flushdns"],
        None,
        Duration::from_secs(15),
    )
    .map(|o| o.success())
    .unwrap_or(false)
}

fn require_change(adapter: &Adapter) -> Result<&'static DnsApi, NetError> {
    WindowsBackend::require_elevation()?;
    if adapter.state == AdapterState::Disabled {
        return Err(NetError::Unsupported("the adapter is disabled".into()));
    }
    api().ok_or_else(|| {
        NetError::Unsupported("DNS settings need Windows 10 version 2004 or later".into())
    })
}

/// Waits until the adapter uses the first requested server (or the timeout passes).
fn wait_until_in_use(guid: &str, config: &DnsConfig, timeout: Duration) -> (Vec<IpAddr>, bool) {
    let wanted = config.servers().first().copied();
    let deadline = Instant::now() + timeout;
    loop {
        let servers = dns_servers(guid);
        let ok = wanted.is_none_or(|w| servers.first() == Some(&w));
        if ok || Instant::now() >= deadline {
            return (servers, ok);
        }
        thread::sleep(Duration::from_millis(500));
    }
}

fn write_all(api: &DnsApi, guid: GUID, calls: &[FamilyCall], v3: bool) -> Result<(), NetError> {
    calls
        .iter()
        .try_for_each(|c| write_family(api, guid, c, v3))
}

fn finish(report: &mut DnsReport, opts: &DnsOptions) {
    if opts.flush_cache {
        report.cache_flushed = flush_cache();
        if !report.cache_flushed {
            report.warnings.push(codes::DNS_CACHE_NOT_FLUSHED.into());
        }
    }
}

pub(super) fn capabilities(adapter: &Adapter) -> DnsCapabilities {
    let Some(api) = api() else {
        return DnsCapabilities::unsupported(DnsReason::RequiresWindows2004);
    };
    let Ok(guid) = WindowsBackend::interface_guid(&adapter.id).and_then(|g| guid_of(&g)) else {
        return DnsCapabilities::unsupported(DnsReason::UnsupportedPlatform);
    };
    let doh = match read_family(api, guid, false) {
        Ok((_, true)) => FeatureSupport::YES,
        _ => FeatureSupport::no(DnsReason::DohRequiresNewerWindows),
    };
    let change_reason = if adapter.state == AdapterState::Disabled {
        Some(DnsReason::AdapterDisabled)
    } else if !is_elevated() {
        Some(DnsReason::NotElevated)
    } else {
        None
    };
    DnsCapabilities {
        supported: true,
        reason: None,
        can_change: change_reason.is_none(),
        change_reason,
        ipv4: FeatureSupport::YES,
        ipv6: if ipv6_bound(guid) {
            FeatureSupport::YES
        } else {
            FeatureSupport::no(DnsReason::UnsupportedPlatform)
        },
        doh,
        dot: FeatureSupport::no(DnsReason::DotUnsupportedWindows),
        method: Some(DnsMethod::WindowsIpHelper),
    }
}

pub(super) fn status(adapter: &Adapter) -> Result<DnsStatus, NetError> {
    let api = api().ok_or_else(|| {
        NetError::Unsupported("DNS settings need Windows 10 version 2004 or later".into())
    })?;
    let guid_text = WindowsBackend::interface_guid(&adapter.id)?;
    let (v4, v6, _) = read_both(api, guid_of(&guid_text)?)?;
    let (doh_template, fallback) = win_dns::doh_details(&v4, &v6);
    let effective = dns_servers(&guid_text);
    Ok(DnsStatus {
        configured: win_dns::summary(&v4, &v6),
        doh_template,
        dot_server_name: None,
        fallback,
        effective_ipv4: effective
            .iter()
            .filter_map(|ip| match ip {
                IpAddr::V4(a) => Some(*a),
                IpAddr::V6(_) => None,
            })
            .collect(),
        effective_ipv6: effective
            .iter()
            .filter_map(|ip| match ip {
                IpAddr::V6(a) => Some(*a),
                IpAddr::V4(_) => None,
            })
            .collect(),
        profile_override: !v4.profile_name_server.is_empty() || !v6.profile_name_server.is_empty(),
        nm_connection_uuid: None,
    })
}

pub(super) fn snapshot(adapter: &Adapter) -> Result<DnsSnapshot, NetError> {
    let api = api().ok_or_else(|| {
        NetError::Unsupported("DNS settings need Windows 10 version 2004 or later".into())
    })?;
    let guid = guid_of(&WindowsBackend::interface_guid(&adapter.id)?)?;
    let (ipv4, ipv6, _) = read_both(api, guid)?;
    Ok(DnsSnapshot::Windows { ipv4, ipv6 })
}

pub(super) fn apply(
    adapter: &Adapter,
    config: &DnsConfig,
    previous: &DnsSnapshot,
    opts: &DnsOptions,
    progress: &mut dyn FnMut(Stage),
) -> Result<DnsReport, NetError> {
    let api = require_change(adapter)?;
    progress(Stage::Preparing);
    let guid_text = WindowsBackend::interface_guid(&adapter.id)?;
    let guid = guid_of(&guid_text)?;
    let (cur4, cur6, v3) = read_both(api, guid)?;
    let (prev4, prev6) = match previous {
        DnsSnapshot::Windows { ipv4, ipv6 } => (ipv4.clone(), ipv6.clone()),
        _ => (cur4, cur6),
    };
    let mut warnings = Vec::new();
    let v6_bound = ipv6_bound(guid);
    let mut calls = win_dns::plan_apply(config, v3).to_vec();
    if !v6_bound {
        calls.retain(|c| !c.ipv6);
        if !matches!(config.ipv6, FamilyDns::Automatic) {
            warnings.push(codes::DNS_IPV6_SKIPPED.to_string());
        }
    }
    let rollback: Vec<FamilyCall> = win_dns::plan_restore(&prev4, &prev6, v3)
        .into_iter()
        .filter(|c| v6_bound || !c.ipv6)
        .collect();

    progress(Stage::Writing);
    if let Err(e) = write_all(api, guid, &calls, v3) {
        progress(Stage::RollingBack);
        return match write_all(api, guid, &rollback, v3) {
            Ok(()) => Err(e),
            Err(again) => Err(NetError::RollbackFailed(format!("{e}; restoring: {again}"))),
        };
    }

    progress(Stage::Verifying);
    let (read4, read6, _) = read_both(api, guid)?;
    let written = calls
        .iter()
        .all(|c| win_dns::matches(c, if c.ipv6 { &read6 } else { &read4 }));
    let mut report = DnsReport {
        method: DnsMethod::WindowsIpHelper,
        status: ChangeStatus::Applied,
        previous: Some(win_dns::summary(&prev4, &prev6)),
        requested: Some(config.summary()),
        observed: Some(win_dns::summary(&read4, &read6)),
        effective: Vec::new(),
        detail_code: None,
        warnings,
        cache_flushed: false,
        nm_connection_uuid: None,
    };
    if !written {
        progress(Stage::RollingBack);
        write_all(api, guid, &rollback, v3).map_err(|e| {
            NetError::RollbackFailed(format!(
                "Windows did not keep the DNS settings; restoring: {e}"
            ))
        })?;
        report.status = ChangeStatus::RolledBack;
        report.detail_code = Some(codes::DNS_NOT_APPLIED.into());
        report.observed = report.previous.clone();
        report.effective = dns_servers(&guid_text);
        return Ok(report);
    }
    let timeout = if adapter.connected && !config.is_automatic() {
        opts.verify_timeout
    } else {
        Duration::ZERO
    };
    let (effective, in_use) = wait_until_in_use(&guid_text, config, timeout);
    report.effective = effective;
    if !read4.profile_name_server.is_empty() || !read6.profile_name_server.is_empty() {
        report.warnings.push(codes::DNS_PROFILE_OVERRIDE.into());
    }
    if !in_use && adapter.connected {
        report.status = ChangeStatus::Unverified;
        report.detail_code = Some(codes::DNS_NOT_ACTIVE.into());
    }
    finish(&mut report, opts);
    Ok(report)
}

pub(super) fn restore(
    adapter: &Adapter,
    original: &DnsOriginalRecord,
    opts: &DnsOptions,
    progress: &mut dyn FnMut(Stage),
) -> Result<DnsReport, NetError> {
    let Some(DnsSnapshot::Windows { ipv4, ipv6 }) = &original.interface else {
        return Err(NetError::NoOriginal);
    };
    let api = require_change(adapter)?;
    progress(Stage::Preparing);
    let guid_text = WindowsBackend::interface_guid(&adapter.id)?;
    let guid = guid_of(&guid_text)?;
    let (cur4, cur6, v3) = read_both(api, guid)?;
    let v6_bound = ipv6_bound(guid);
    let calls: Vec<FamilyCall> = win_dns::plan_restore(ipv4, ipv6, v3)
        .into_iter()
        .filter(|c| v6_bound || !c.ipv6)
        .collect();
    progress(Stage::Writing);
    write_all(api, guid, &calls, v3)?;
    progress(Stage::Verifying);
    let (read4, read6, _) = read_both(api, guid)?;
    let written = calls
        .iter()
        .all(|c| win_dns::matches(c, if c.ipv6 { &read6 } else { &read4 }));
    let mut report = DnsReport {
        method: DnsMethod::WindowsIpHelper,
        status: if written {
            ChangeStatus::Applied
        } else {
            ChangeStatus::Unverified
        },
        previous: Some(win_dns::summary(&cur4, &cur6)),
        requested: Some(win_dns::summary(ipv4, ipv6)),
        observed: Some(win_dns::summary(&read4, &read6)),
        effective: dns_servers(&guid_text),
        detail_code: (!written).then(|| codes::DNS_NOT_APPLIED.to_string()),
        warnings: Vec::new(),
        cache_flushed: false,
        nm_connection_uuid: None,
    };
    finish(&mut report, opts);
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_api_is_found_and_unknown_guids_fail() {
        // CI runners and supported Windows versions have the API.
        let api = api().expect("SetInterfaceDnsSettings is available");
        let unknown = GUID::from_u128(0x0000_0000_0000_0000_0000_0000_0000_f00d);
        assert!(read_family(api, unknown, false).is_err());
        assert!(!ipv6_bound(unknown));
        assert!(guid_of("not a guid").is_err());
        assert!(dns_servers("{00000000-0000-0000-0000-00000000F00D}").is_empty());
    }

    #[test]
    fn reading_pointers_safely() {
        // SAFETY: null pointers are explicitly allowed.
        unsafe {
            assert_eq!(read_pwstr(std::ptr::null()), "");
            assert!(read_props(3, std::ptr::null()).is_empty());
        }
        let text = wide("9.9.9.9");
        // SAFETY: `text` is NUL-terminated.
        assert_eq!(unsafe { read_pwstr(text.as_ptr()) }, "9.9.9.9");
    }
}
