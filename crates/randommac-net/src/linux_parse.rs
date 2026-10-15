//! Platform-independent parsing helpers for the Linux backend (sysfs, udev, nmcli, iproute2).
//! Kept separate from the syscall code so they are unit-tested on every OS with fixtures.

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;

use randommac_core::MacAddress;

use crate::error::NetError;

/// `ARPHRD_ETHER`: Ethernet and Wi-Fi interfaces.
pub const ARPHRD_ETHER: u16 = 1;
const IFF_UP: u32 = 0x1;

/// What `/sys/class/net/<name>` tells us about an interface.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SysfsIface {
    pub name: String,
    pub ifindex: u32,
    pub address: Option<MacAddress>,
    pub arphrd: u16,
    pub operstate: String,
    pub carrier: Option<bool>,
    pub admin_up: bool,
    /// 0 = permanent, 1 = random, 2 = stolen, 3 = set by userspace.
    pub addr_assign_type: Option<u8>,
    /// No backing device (`device` link absent): bridges, veth, dummy, tun/tap, …
    pub is_virtual: bool,
    pub is_wireless: bool,
    pub driver: Option<String>,
}

fn read_trim(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

fn uevent_value(text: &str, key: &str) -> Option<String> {
    text.lines().find_map(|l| {
        l.strip_prefix(key)
            .and_then(|rest| rest.strip_prefix('='))
            .map(|v| v.trim().to_string())
    })
}

/// Reads one interface below `class_net` (normally `/sys/class/net`).
pub fn read_interface(class_net: &Path, name: &str) -> io::Result<SysfsIface> {
    let dir = class_net.join(name);
    if !dir.exists() {
        return Err(io::Error::new(io::ErrorKind::NotFound, name.to_string()));
    }
    let device = dir.join("device");
    let uevent = read_trim(&dir.join("uevent")).unwrap_or_default();
    let driver = read_trim(&device.join("uevent"))
        .and_then(|t| uevent_value(&t, "DRIVER"))
        .or_else(|| {
            fs::read_link(device.join("driver"))
                .ok()
                .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        });
    let flags = read_trim(&dir.join("flags"))
        .and_then(|f| u32::from_str_radix(f.trim_start_matches("0x"), 16).ok())
        .unwrap_or(0);
    Ok(SysfsIface {
        name: name.to_string(),
        ifindex: read_trim(&dir.join("ifindex"))
            .and_then(|s| s.parse().ok())
            .unwrap_or(0),
        address: read_trim(&dir.join("address")).and_then(|s| s.parse().ok()),
        arphrd: read_trim(&dir.join("type"))
            .and_then(|s| s.parse().ok())
            .unwrap_or(0),
        operstate: read_trim(&dir.join("operstate")).unwrap_or_else(|| "unknown".into()),
        carrier: read_trim(&dir.join("carrier")).map(|c| c == "1"),
        admin_up: flags & IFF_UP != 0,
        addr_assign_type: read_trim(&dir.join("addr_assign_type")).and_then(|s| s.parse().ok()),
        is_virtual: !device.exists(),
        is_wireless: dir.join("wireless").exists()
            || dir.join("phy80211").exists()
            || uevent_value(&uevent, "DEVTYPE").as_deref() == Some("wlan"),
        driver,
    })
}

/// Reads every interface below `class_net`, sorted by name.
pub fn read_interfaces(class_net: &Path) -> io::Result<Vec<SysfsIface>> {
    let mut names: Vec<String> = fs::read_dir(class_net)?
        .filter_map(Result::ok)
        .filter_map(|e| e.file_name().to_str().map(str::to_string))
        .collect();
    names.sort();
    Ok(names
        .iter()
        .filter_map(|n| read_interface(class_net, n).ok())
        .collect())
}

/// Parses a udev database file (`/run/udev/data/n<ifindex>`) into its `E:` properties.
pub fn parse_udev_properties(text: &str) -> HashMap<String, String> {
    text.lines()
        .filter_map(|l| l.strip_prefix("E:"))
        .filter_map(|kv| kv.split_once('='))
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

/// Human readable model name from udev properties.
pub fn udev_description(props: &HashMap<String, String>) -> Option<String> {
    let model = props
        .get("ID_MODEL_FROM_DATABASE")
        .or_else(|| props.get("ID_MODEL"))?;
    match props.get("ID_VENDOR_FROM_DATABASE") {
        Some(vendor) if !model.starts_with(vendor.as_str()) => Some(format!("{vendor} {model}")),
        _ => Some(model.clone()),
    }
}

/// Splits one line of `nmcli -t` output on unescaped `:` and removes the escaping.
pub fn split_terse(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut cur = String::new();
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                if let Some(next) = chars.next() {
                    cur.push(next);
                }
            }
            ':' => fields.push(std::mem::take(&mut cur)),
            _ => cur.push(c),
        }
    }
    fields.push(cur);
    fields
}

/// Removes `nmcli -g` escaping from a single value.
pub fn unescape_value(value: &str) -> String {
    split_terse(value).join(":")
}

/// `"100 (connected)"` → `100`.
pub fn parse_nm_state(value: &str) -> Option<u32> {
    value
        .trim()
        .split(|c: char| !c.is_ascii_digit())
        .next()
        .and_then(|n| n.parse().ok())
}

/// NetworkManager device states (subset).
pub const NM_DEVICE_STATE_UNMANAGED: u32 = 10;

/// Interface names accepted by `ip(8)`: 1–15 characters, no whitespace, `/` or leading `-`.
pub fn valid_ifname(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 15
        && !name.starts_with('-')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '@' | ':' | '+' | '-'))
}

/// Builds the `ip -batch` script that changes the address (down → address → up).
pub fn ip_batch_script(ifname: &str, mac: MacAddress, bring_up: bool) -> Result<String, NetError> {
    if !valid_ifname(ifname) {
        return Err(NetError::InvalidInput(format!(
            "invalid interface name {ifname:?}"
        )));
    }
    let mut script = format!(
        "link set dev {ifname} down\nlink set dev {ifname} address {}\n",
        mac.to_colon_lower()
    );
    if bring_up {
        script.push_str(&format!("link set dev {ifname} up\n"));
    }
    Ok(script)
}

/// Maps `pkexec` exit codes to errors (126 = dialog dismissed, 127 = not authorized / no agent).
pub fn pkexec_error(code: Option<i32>, stderr: &str) -> Option<NetError> {
    match code {
        Some(126) => Some(NetError::AuthCancelled),
        Some(127) => Some(NetError::AuthUnavailable(stderr.trim().to_string())),
        _ => None,
    }
}

/// Values of `cloned-mac-address` that make NetworkManager keep or reset the hardware address.
pub fn nm_value_keeps_current(value: &str) -> bool {
    matches!(value.trim(), "" | "preserve")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(root: &Path, rel: &str, content: &str) {
        let p = root.join(rel);
        fs::create_dir_all(p.parent().expect("parent")).expect("mkdir");
        fs::write(p, content).expect("write");
    }

    #[test]
    fn reads_sysfs_fixture() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let net = tmp.path();
        // Physical Wi-Fi
        write(net, "wlp2s0/address", "a2:0e:a6:3e:d0:ac\n");
        write(net, "wlp2s0/type", "1\n");
        write(net, "wlp2s0/ifindex", "3\n");
        write(net, "wlp2s0/operstate", "up\n");
        write(net, "wlp2s0/carrier", "1\n");
        write(net, "wlp2s0/flags", "0x1003\n");
        write(net, "wlp2s0/addr_assign_type", "3\n");
        write(
            net,
            "wlp2s0/uevent",
            "DEVTYPE=wlan\nINTERFACE=wlp2s0\nIFINDEX=3\n",
        );
        write(
            net,
            "wlp2s0/device/uevent",
            "DRIVER=iwlwifi\nPCI_ID=8086:A0F0\n",
        );
        // Virtual bridge, admin down
        write(net, "docker0/address", "02:42:ac:11:00:01\n");
        write(net, "docker0/type", "1\n");
        write(net, "docker0/operstate", "down\n");
        write(net, "docker0/flags", "0x1002\n");
        // Loopback
        write(net, "lo/address", "00:00:00:00:00:00\n");
        write(net, "lo/type", "772\n");

        let all = read_interfaces(net).expect("read");
        assert_eq!(
            all.iter().map(|i| i.name.as_str()).collect::<Vec<_>>(),
            ["docker0", "lo", "wlp2s0"]
        );
        let wlan = all.iter().find(|i| i.name == "wlp2s0").expect("wlan");
        assert!(wlan.is_wireless && !wlan.is_virtual && wlan.admin_up);
        assert_eq!(wlan.driver.as_deref(), Some("iwlwifi"));
        assert_eq!(wlan.carrier, Some(true));
        assert_eq!(wlan.addr_assign_type, Some(3));
        assert_eq!(
            wlan.address.map(|m| m.to_string()).as_deref(),
            Some("A2:0E:A6:3E:D0:AC")
        );
        let docker = all.iter().find(|i| i.name == "docker0").expect("docker");
        assert!(docker.is_virtual && !docker.admin_up && !docker.is_wireless);
        assert_eq!(docker.carrier, None);
        assert_eq!(all.iter().find(|i| i.name == "lo").expect("lo").arphrd, 772);
    }

    #[test]
    fn udev_properties() {
        let props = parse_udev_properties(
            "I:123\nE:ID_VENDOR_FROM_DATABASE=Intel Corporation\nE:ID_MODEL_FROM_DATABASE=Wi-Fi 6 AX201\nG:systemd\n",
        );
        assert_eq!(
            udev_description(&props).as_deref(),
            Some("Intel Corporation Wi-Fi 6 AX201")
        );
        let same = parse_udev_properties(
            "E:ID_VENDOR_FROM_DATABASE=Realtek\nE:ID_MODEL_FROM_DATABASE=Realtek RTL8125\n",
        );
        assert_eq!(udev_description(&same).as_deref(), Some("Realtek RTL8125"));
        assert!(udev_description(&HashMap::new()).is_none());
    }

    #[test]
    fn nmcli_terse() {
        assert_eq!(
            split_terse(r"wlp2s0:wifi:100 (connected):Caf\:e Wi\\Fi"),
            vec!["wlp2s0", "wifi", "100 (connected)", r"Caf:e Wi\Fi"]
        );
        assert_eq!(
            unescape_value(r"02\:11\:22\:33\:44\:55"),
            "02:11:22:33:44:55"
        );
        assert_eq!(unescape_value(""), "");
        assert_eq!(parse_nm_state("100 (connected)"), Some(100));
        assert_eq!(
            parse_nm_state("10 (unmanaged)"),
            Some(NM_DEVICE_STATE_UNMANAGED)
        );
        assert_eq!(parse_nm_state("garbage"), None);
        assert!(nm_value_keeps_current("") && nm_value_keeps_current("preserve"));
        assert!(!nm_value_keeps_current("random"));
    }

    #[test]
    fn ifname_validation_blocks_injection() {
        for ok in [
            "eth0",
            "wlp2s0",
            "enx00e04c680001",
            "br-1a2b",
            "veth@if3",
            "dummy0",
        ] {
            assert!(valid_ifname(ok), "{ok}");
        }
        for bad in [
            "",
            "-x",
            "eth0 down",
            "dummy0\nlink set eth0 down",
            "a/b",
            "sixteen-chars-xx",
            "eth0;reboot",
        ] {
            assert!(!valid_ifname(bad), "{bad:?}");
        }
    }

    #[test]
    fn batch_script() {
        let mac: MacAddress = "02:AB:CD:EF:01:23".parse().expect("mac");
        assert_eq!(
            ip_batch_script("dummy0", mac, true).expect("script"),
            "link set dev dummy0 down\nlink set dev dummy0 address 02:ab:cd:ef:01:23\nlink set dev dummy0 up\n"
        );
        assert!(
            !ip_batch_script("dummy0", mac, false)
                .expect("script")
                .contains(" up")
        );
        assert!(ip_batch_script("x\ny", mac, true).is_err());
        assert!(matches!(
            pkexec_error(Some(126), ""),
            Some(NetError::AuthCancelled)
        ));
        assert!(matches!(
            pkexec_error(Some(127), "no agent"),
            Some(NetError::AuthUnavailable(_))
        ));
        assert!(pkexec_error(Some(2), "").is_none());
    }
}
