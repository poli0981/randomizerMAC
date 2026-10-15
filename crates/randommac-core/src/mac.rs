//! MAC-48 address type with strict parsing and configurable formatting.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A 48-bit IEEE 802 MAC address.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct MacAddress([u8; 6]);

/// Separator used when formatting a MAC address for display.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Separator {
    /// `AA:BB:CC:DD:EE:FF`
    #[default]
    Colon,
    /// `AA-BB-CC-DD-EE-FF`
    Hyphen,
    /// `AABB.CCDD.EEFF` (Cisco style)
    Dot,
    /// `AABBCCDDEEFF`
    None,
}

/// Display format for MAC addresses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct MacFormat {
    pub separator: Separator,
    pub uppercase: bool,
}

impl Default for MacFormat {
    fn default() -> Self {
        Self {
            separator: Separator::Colon,
            uppercase: true,
        }
    }
}

/// Why a string could not be parsed as a MAC address.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ParseMacError {
    #[error("the MAC address is empty")]
    Empty,
    #[error("a MAC address has 12 hex digits; got {0} characters")]
    InvalidLength(usize),
    #[error("invalid character {ch:?} at position {pos}")]
    InvalidCharacter { ch: char, pos: usize },
    #[error("separators must be consistent (all ':' or all '-')")]
    MixedSeparators,
}

impl MacAddress {
    /// `00:00:00:00:00:00`
    pub const ZERO: Self = Self([0; 6]);
    /// `FF:FF:FF:FF:FF:FF`
    pub const BROADCAST: Self = Self([0xff; 6]);

    pub const fn new(octets: [u8; 6]) -> Self {
        Self(octets)
    }

    pub const fn octets(self) -> [u8; 6] {
        self.0
    }

    /// The first three octets (Organizationally Unique Identifier).
    pub const fn oui(self) -> [u8; 3] {
        [self.0[0], self.0[1], self.0[2]]
    }

    /// I/G bit set: group (multicast/broadcast) address.
    pub const fn is_multicast(self) -> bool {
        self.0[0] & 0x01 != 0
    }

    pub const fn is_unicast(self) -> bool {
        !self.is_multicast()
    }

    /// U/L bit set: locally administered (not assigned by a vendor).
    pub const fn is_locally_administered(self) -> bool {
        self.0[0] & 0x02 != 0
    }

    pub const fn is_universal(self) -> bool {
        !self.is_locally_administered()
    }

    pub fn is_zero(self) -> bool {
        self == Self::ZERO
    }

    pub fn is_broadcast(self) -> bool {
        self == Self::BROADCAST
    }

    /// Whether the address may be assigned to an interface (unicast and non-zero).
    pub fn is_assignable(self) -> bool {
        self.is_unicast() && !self.is_zero()
    }

    /// Windows wireless drivers only accept locally administered unicast addresses
    /// (the second hex digit is 2, 6, A or E).
    pub fn is_windows_wifi_compatible(self) -> bool {
        self.is_assignable() && self.is_locally_administered()
    }

    /// Returns the address with the U/L and I/G bits forced to "locally administered unicast".
    pub const fn to_local_unicast(self) -> Self {
        let mut o = self.0;
        o[0] = (o[0] & 0xfc) | 0x02;
        Self(o)
    }

    /// Formats the address using the given display format.
    pub fn format(self, format: MacFormat) -> String {
        let o = self.0;
        let hex = |b: u8| {
            if format.uppercase {
                format!("{b:02X}")
            } else {
                format!("{b:02x}")
            }
        };
        match format.separator {
            Separator::Colon => o.iter().map(|b| hex(*b)).collect::<Vec<_>>().join(":"),
            Separator::Hyphen => o.iter().map(|b| hex(*b)).collect::<Vec<_>>().join("-"),
            Separator::None => o.iter().map(|b| hex(*b)).collect(),
            Separator::Dot => o
                .chunks(2)
                .map(|pair| format!("{}{}", hex(pair[0]), hex(pair[1])))
                .collect::<Vec<_>>()
                .join("."),
        }
    }

    /// `AABBCCDDEEFF` — the representation Windows expects in the `NetworkAddress` registry value.
    pub fn to_compact_upper(self) -> String {
        self.format(MacFormat {
            separator: Separator::None,
            uppercase: true,
        })
    }

    /// `aa:bb:cc:dd:ee:ff` — the representation used by iproute2 and NetworkManager.
    pub fn to_colon_lower(self) -> String {
        self.format(MacFormat {
            separator: Separator::Colon,
            uppercase: false,
        })
    }
}

fn hex_value(ch: char) -> Option<u8> {
    ch.to_digit(16).map(|d| d as u8)
}

fn parse_hex_run(chars: &[(usize, char)]) -> Result<Vec<u8>, ParseMacError> {
    debug_assert!(chars.len().is_multiple_of(2));
    chars
        .chunks(2)
        .map(|pair| {
            let (p0, c0) = pair[0];
            let (p1, c1) = pair[1];
            let hi = hex_value(c0).ok_or(ParseMacError::InvalidCharacter { ch: c0, pos: p0 })?;
            let lo = hex_value(c1).ok_or(ParseMacError::InvalidCharacter { ch: c1, pos: p1 })?;
            Ok(hi << 4 | lo)
        })
        .collect()
}

fn to_array(bytes: Vec<u8>) -> MacAddress {
    let mut out = [0u8; 6];
    out.copy_from_slice(&bytes);
    MacAddress(out)
}

impl FromStr for MacAddress {
    type Err = ParseMacError;

    /// Accepts exactly these shapes (case-insensitive, surrounding whitespace ignored):
    /// `AA:BB:CC:DD:EE:FF`, `AA-BB-CC-DD-EE-FF`, `AABB.CCDD.EEFF`, `AABBCCDDEEFF`.
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err(ParseMacError::Empty);
        }
        let chars: Vec<(usize, char)> = trimmed.chars().enumerate().collect();
        match chars.len() {
            12 => parse_hex_run(&chars).map(to_array),
            14 => {
                let mut digits = Vec::with_capacity(12);
                for &(pos, ch) in &chars {
                    if pos == 4 || pos == 9 {
                        if ch != '.' {
                            return Err(ParseMacError::InvalidCharacter { ch, pos });
                        }
                    } else {
                        digits.push((pos, ch));
                    }
                }
                parse_hex_run(&digits).map(to_array)
            }
            17 => {
                let (_, sep) = chars[2];
                if sep != ':' && sep != '-' {
                    return Err(ParseMacError::InvalidCharacter { ch: sep, pos: 2 });
                }
                let mut digits = Vec::with_capacity(12);
                for &(pos, ch) in &chars {
                    if pos % 3 == 2 {
                        if ch == sep {
                            continue;
                        }
                        if ch == ':' || ch == '-' {
                            return Err(ParseMacError::MixedSeparators);
                        }
                        return Err(ParseMacError::InvalidCharacter { ch, pos });
                    }
                    digits.push((pos, ch));
                }
                parse_hex_run(&digits).map(to_array)
            }
            n => Err(ParseMacError::InvalidLength(n)),
        }
    }
}

impl fmt::Display for MacAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.format(MacFormat::default()))
    }
}

impl fmt::Debug for MacAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MacAddress({self})")
    }
}

impl From<[u8; 6]> for MacAddress {
    fn from(octets: [u8; 6]) -> Self {
        Self(octets)
    }
}

impl TryFrom<&[u8]> for MacAddress {
    type Error = ParseMacError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        if bytes.len() != 6 {
            return Err(ParseMacError::InvalidLength(bytes.len() * 2));
        }
        let mut out = [0u8; 6];
        out.copy_from_slice(bytes);
        Ok(Self(out))
    }
}

impl Serialize for MacAddress {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for MacAddress {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Deserialize)]
    struct Vectors {
        valid: Vec<ValidVector>,
        invalid: Vec<String>,
    }

    #[derive(Deserialize)]
    struct ValidVector {
        input: String,
        canonical: String,
    }

    /// Shared with the frontend unit tests (`src/lib/mac.test.ts`).
    const VECTORS: &str = include_str!("../tests/vectors/mac_parse.json");

    #[test]
    fn shared_vectors() {
        let vectors: Vectors = serde_json::from_str(VECTORS).expect("vector file is valid JSON");
        assert!(!vectors.valid.is_empty() && !vectors.invalid.is_empty());
        for v in vectors.valid {
            let mac: MacAddress = v
                .input
                .parse()
                .unwrap_or_else(|e| panic!("{:?} should parse: {e}", v.input));
            assert_eq!(mac.to_string(), v.canonical, "input {:?}", v.input);
        }
        for input in vectors.invalid {
            assert!(
                input.parse::<MacAddress>().is_err(),
                "{input:?} should be rejected"
            );
        }
    }

    #[test]
    fn reports_error_kinds() {
        assert_eq!("".parse::<MacAddress>(), Err(ParseMacError::Empty));
        assert_eq!(
            "AA:BB:CC".parse::<MacAddress>(),
            Err(ParseMacError::InvalidLength(8))
        );
        assert_eq!(
            "AA:BB-CC:DD:EE:FF".parse::<MacAddress>(),
            Err(ParseMacError::MixedSeparators)
        );
        assert_eq!(
            "GG:BB:CC:DD:EE:FF".parse::<MacAddress>(),
            Err(ParseMacError::InvalidCharacter { ch: 'G', pos: 0 })
        );
    }

    #[test]
    fn formats() {
        let mac = MacAddress::new([0x02, 0xab, 0x0c, 0xd0, 0x0e, 0xff]);
        assert_eq!(mac.to_string(), "02:AB:0C:D0:0E:FF");
        assert_eq!(mac.to_compact_upper(), "02AB0CD00EFF");
        assert_eq!(mac.to_colon_lower(), "02:ab:0c:d0:0e:ff");
        let dotted = mac.format(MacFormat {
            separator: Separator::Dot,
            uppercase: false,
        });
        assert_eq!(dotted, "02ab.0cd0.0eff");
        assert_eq!(dotted.parse::<MacAddress>(), Ok(mac));
        let hyphen = mac.format(MacFormat {
            separator: Separator::Hyphen,
            uppercase: true,
        });
        assert_eq!(hyphen, "02-AB-0C-D0-0E-FF");
    }

    #[test]
    fn bit_semantics() {
        let laa = MacAddress::new([0x02, 0, 0, 0, 0, 1]);
        assert!(laa.is_locally_administered() && laa.is_unicast() && laa.is_assignable());
        assert!(laa.is_windows_wifi_compatible());

        let uaa = MacAddress::new([0x00, 0x1b, 0x21, 1, 2, 3]);
        assert!(uaa.is_universal() && !uaa.is_windows_wifi_compatible());

        let multicast = MacAddress::new([0x01, 0x00, 0x5e, 0, 0, 1]);
        assert!(multicast.is_multicast() && !multicast.is_assignable());

        assert!(!MacAddress::ZERO.is_assignable());
        assert!(MacAddress::BROADCAST.is_broadcast() && !MacAddress::BROADCAST.is_assignable());

        for first in 0..=255u8 {
            let mac = MacAddress::new([first, 1, 2, 3, 4, 5]).to_local_unicast();
            assert!(mac.is_windows_wifi_compatible(), "{mac}");
            assert!(matches!(mac.octets()[0] & 0x0f, 0x2 | 0x6 | 0xa | 0xe));
        }
    }

    #[test]
    fn serde_round_trip() {
        let mac = MacAddress::new([0x02, 0x11, 0x22, 0x33, 0x44, 0x55]);
        let json = serde_json::to_string(&mac).expect("serialize");
        assert_eq!(json, "\"02:11:22:33:44:55\"");
        let back: MacAddress = serde_json::from_str("\"02-11-22-33-44-55\"").expect("v1 format");
        assert_eq!(back, mac);
        assert!(serde_json::from_str::<MacAddress>("\"nope\"").is_err());
    }
}
