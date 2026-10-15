//! Platform-independent helpers for the Windows backend (unit-tested on every OS).

use randommac_core::MacAddress;

use crate::types::AdapterKind;

/// `Characteristics` flags of a network-class driver key (netcfgx.h).
pub const NCF_VIRTUAL: u32 = 0x1;
pub const NCF_PHYSICAL: u32 = 0x4;
pub const NCF_HIDDEN: u32 = 0x8;

/// `*IfType` / `MIB_IF_ROW2.Type`
pub const IF_TYPE_ETHERNET_CSMACD: u32 = 6;
pub const IF_TYPE_IEEE80211: u32 = 71;

/// `*PhysicalMediaType` (NDIS_PHYSICAL_MEDIUM)
const MEDIUM_WIRELESS_LAN: u32 = 1;
const MEDIUM_NATIVE_802_11: u32 = 9;
const MEDIUM_802_3: u32 = 14;

/// Hidden adapters (WAN miniports, kernel debug adapters, …) are never shown.
pub fn is_hidden(characteristics: u32) -> bool {
    characteristics & NCF_HIDDEN != 0
}

/// Virtual or software adapters (Hyper-V, VPN, VirtualBox, Bluetooth PAN, …).
pub fn is_virtual(characteristics: u32, instance_id: &str) -> bool {
    let id = instance_id.to_ascii_uppercase();
    characteristics & NCF_VIRTUAL != 0
        || characteristics & NCF_PHYSICAL == 0
        || ["ROOT\\", "SWD\\", "BTH\\", "BTHENUM\\", "{"]
            .iter()
            .any(|p| id.starts_with(p))
}

/// Ethernet or Wi-Fi adapters are the only ones whose MAC address can be changed.
pub fn is_relevant(if_type: Option<u32>, medium: Option<u32>) -> bool {
    match if_type {
        Some(IF_TYPE_ETHERNET_CSMACD) | Some(IF_TYPE_IEEE80211) => true,
        Some(_) => false,
        None => matches!(
            medium,
            Some(MEDIUM_WIRELESS_LAN) | Some(MEDIUM_NATIVE_802_11) | Some(MEDIUM_802_3)
        ),
    }
}

pub fn kind_from(if_type: Option<u32>, medium: Option<u32>) -> AdapterKind {
    if if_type == Some(IF_TYPE_IEEE80211)
        || matches!(
            medium,
            Some(MEDIUM_WIRELESS_LAN) | Some(MEDIUM_NATIVE_802_11)
        )
    {
        AdapterKind::Wireless
    } else if if_type == Some(IF_TYPE_ETHERNET_CSMACD) || medium == Some(MEDIUM_802_3) {
        AdapterKind::Ethernet
    } else {
        AdapterKind::Other
    }
}

/// Parses `{XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX}` into (data1, data2, data3, data4).
pub fn parse_guid(s: &str) -> Option<(u32, u16, u16, [u8; 8])> {
    let s = s.trim().trim_start_matches('{').trim_end_matches('}');
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 5
        || [8, 4, 4, 4, 12]
            .iter()
            .zip(&parts)
            .any(|(len, p)| p.len() != *len || !p.chars().all(|c| c.is_ascii_hexdigit()))
    {
        return None;
    }
    let d1 = u32::from_str_radix(parts[0], 16).ok()?;
    let d2 = u16::from_str_radix(parts[1], 16).ok()?;
    let d3 = u16::from_str_radix(parts[2], 16).ok()?;
    let tail = format!("{}{}", parts[3], parts[4]);
    let mut d4 = [0u8; 8];
    for (i, b) in d4.iter_mut().enumerate() {
        *b = u8::from_str_radix(&tail[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some((d1, d2, d3, d4))
}

/// Canonical `{UPPERCASE}` form used as a map key.
pub fn normalize_guid(s: &str) -> String {
    let inner = s.trim().trim_start_matches('{').trim_end_matches('}');
    format!("{{{}}}", inner.to_ascii_uppercase())
}

/// Parses a `NetworkAddress` registry value ("02AB…", sometimes with separators).
pub fn parse_registry_mac(value: &str) -> Option<MacAddress> {
    value.trim().parse().ok()
}

/// A 6-byte hardware address from an IP Helper row (`None` for other lengths or all zeros).
pub fn mac_from_row(len: u32, bytes: &[u8]) -> Option<MacAddress> {
    if len != 6 || bytes.len() < 6 {
        return None;
    }
    MacAddress::try_from(&bytes[..6])
        .ok()
        .filter(|m| !m.is_zero())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classification_matches_real_machine() {
        // Values read from the maintainer's machine (see docs/ARCHITECTURE.md).
        let wifi_chars = 0x84; // NCF_PHYSICAL | NCF_HAS_UI
        assert!(!is_hidden(wifi_chars));
        assert!(!is_virtual(
            wifi_chars,
            r"PCI\VEN_10EC&DEV_C852&SUBSYS_C85210EC&REV_01\00E04CFFFEC8520100"
        ));
        assert_eq!(kind_from(Some(71), Some(9)), AdapterKind::Wireless);
        assert!(is_relevant(Some(71), Some(9)));

        assert!(!is_virtual(
            0x4,
            r"PCI\VEN_10EC&DEV_8125&SUBSYS_E0001458&REV_05\01000000684CE00000"
        ));
        assert_eq!(kind_from(Some(6), Some(14)), AdapterKind::Ethernet);

        assert!(is_virtual(0x1, r"ROOT\NET\0001")); // VirtualBox host-only
        assert!(is_virtual(0x4, r"BTH\MS_BTHPAN\6&1"));
        assert!(is_virtual(0x0, r"PCI\SOMETHING"));
        assert!(is_hidden(0x8 | 0x1));
        assert!(!is_relevant(Some(53), None)); // propVirtual (VPN)
        assert!(!is_relevant(None, None));
        assert_eq!(kind_from(None, None), AdapterKind::Other);
    }

    #[test]
    fn guids() {
        let (d1, d2, d3, d4) = parse_guid("{59AB0DC4-AC80-4C28-8DF3-D2CF25B40A3F}").expect("guid");
        assert_eq!((d1, d2, d3), (0x59AB0DC4, 0xAC80, 0x4C28));
        assert_eq!(d4, [0x8D, 0xF3, 0xD2, 0xCF, 0x25, 0xB4, 0x0A, 0x3F]);
        assert!(parse_guid("59ab0dc4-ac80-4c28-8df3-d2cf25b40a3f").is_some());
        assert!(parse_guid("{59AB0DC4-AC80-4C28-8DF3}").is_none());
        assert!(parse_guid("{ZZAB0DC4-AC80-4C28-8DF3-D2CF25B40A3F}").is_none());
        assert_eq!(
            normalize_guid("{59ab0dc4-ac80-4c28-8df3-d2cf25b40a3f}"),
            "{59AB0DC4-AC80-4C28-8DF3-D2CF25B40A3F}"
        );
    }

    #[test]
    fn registry_and_row_addresses() {
        assert_eq!(
            parse_registry_mac("02AB0CD00EFF")
                .map(|m| m.to_string())
                .as_deref(),
            Some("02:AB:0C:D0:0E:FF")
        );
        assert!(parse_registry_mac("").is_none());
        let mut row = [0u8; 32];
        row[..6].copy_from_slice(&[0x30, 0x56, 0x0f, 0x09, 0x8f, 0xe4]);
        assert!(mac_from_row(6, &row).is_some());
        assert!(mac_from_row(0, &row).is_none());
        assert!(mac_from_row(6, &[0u8; 32]).is_none());
    }
}
