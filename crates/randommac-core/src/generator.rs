//! Random MAC address generation.

use serde::{Deserialize, Serialize};

use crate::blacklist::Blacklist;
use crate::mac::MacAddress;
use crate::oui;
use crate::random::{RandomError, RandomSource};

/// How new addresses are generated.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum GenerationMode {
    /// 46 random bits; locally administered unicast (accepted by every driver, incl. Windows Wi-Fi).
    #[default]
    Random,
    /// Keep the adapter's own vendor prefix (from its hardware address) and randomize the rest.
    KeepVendor,
    /// Use a random OUI of the given vendor so the device looks like that vendor's hardware.
    Vendor {
        #[serde(rename = "vendorId")]
        vendor_id: String,
    },
}

impl GenerationMode {
    /// Whether generated addresses are universally administered (vendor-like).
    /// Windows Wi-Fi drivers usually reject such addresses.
    pub fn produces_universal(&self) -> bool {
        !matches!(self, GenerationMode::Random)
    }
}

/// Input for [`generate`].
#[derive(Debug, Clone, Default)]
pub struct GenerateRequest<'a> {
    pub mode: GenerationMode,
    /// Hardware (preferred) or current address of the adapter; required for `KeepVendor`.
    pub base: Option<MacAddress>,
    /// Addresses that must not be produced (e.g. the current address).
    pub avoid: &'a [MacAddress],
    pub blacklist: Option<&'a Blacklist>,
    pub adapter_id: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GenerateError {
    #[error(transparent)]
    Random(#[from] RandomError),
    #[error("the adapter has no known hardware address to take the vendor prefix from")]
    NoBaseAddress,
    #[error("unknown vendor `{0}`")]
    UnknownVendor(String),
    #[error("every candidate address was blocked by the blacklist")]
    Exhausted,
}

const MAX_ATTEMPTS: usize = 64;

fn candidate(
    rng: &mut impl RandomSource,
    mode: &GenerationMode,
    base: Option<MacAddress>,
    vendor_ouis: &[[u8; 3]],
) -> Result<MacAddress, GenerateError> {
    let mut b = [0u8; 6];
    rng.fill(&mut b)?;
    match mode {
        GenerationMode::Random => Ok(MacAddress::new(b).to_local_unicast()),
        GenerationMode::KeepVendor => {
            let base = base.ok_or(GenerateError::NoBaseAddress)?;
            let o = base.oui();
            // Keep the prefix but never produce a group address.
            Ok(MacAddress::new([o[0] & 0xfe, o[1], o[2], b[3], b[4], b[5]]))
        }
        GenerationMode::Vendor { .. } => {
            let pick = rng.below(vendor_ouis.len() as u32)? as usize;
            let o = vendor_ouis[pick];
            Ok(MacAddress::new([o[0], o[1], o[2], b[3], b[4], b[5]]))
        }
    }
}

/// Generates an assignable address that is not blocked and not in `avoid`.
pub fn generate(
    rng: &mut impl RandomSource,
    req: &GenerateRequest<'_>,
) -> Result<MacAddress, GenerateError> {
    let vendor_ouis = match &req.mode {
        GenerationMode::Vendor { vendor_id } => {
            let ouis = oui::ouis_of(vendor_id);
            if ouis.is_empty() {
                return Err(GenerateError::UnknownVendor(vendor_id.clone()));
            }
            ouis
        }
        _ => Vec::new(),
    };
    for _ in 0..MAX_ATTEMPTS {
        let mac = candidate(rng, &req.mode, req.base, &vendor_ouis)?;
        if !mac.is_assignable() || req.avoid.contains(&mac) {
            continue;
        }
        if req
            .blacklist
            .is_some_and(|bl| bl.is_blocked(mac, req.adapter_id))
        {
            continue;
        }
        if req.blacklist.is_none() && crate::blacklist::RESERVED.contains(&mac) {
            continue;
        }
        return Ok(mac);
    }
    Err(GenerateError::Exhausted)
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;
    use crate::random::test_support::{ScriptedRandom, SeqRandom};

    #[test]
    fn random_mode_is_always_local_unicast() {
        let mut rng = SeqRandom(0xdead_beef_cafe_f00d);
        let req = GenerateRequest::default();
        let mut seen = std::collections::HashSet::new();
        for _ in 0..10_000 {
            let mac = generate(&mut rng, &req).expect("generate");
            assert!(mac.is_windows_wifi_compatible(), "{mac}");
            seen.insert(mac);
        }
        assert!(
            seen.len() > 9_990,
            "addresses should practically never repeat"
        );
    }

    #[test]
    fn keep_vendor_keeps_prefix() {
        let mut rng = SeqRandom(42);
        let base: MacAddress = "30:56:0F:09:8F:E4".parse().expect("mac");
        let req = GenerateRequest {
            mode: GenerationMode::KeepVendor,
            base: Some(base),
            avoid: &[base],
            ..Default::default()
        };
        for _ in 0..1_000 {
            let mac = generate(&mut rng, &req).expect("generate");
            assert_eq!(mac.oui(), base.oui());
            assert_ne!(mac, base);
            assert!(mac.is_assignable());
        }
        let no_base = GenerateRequest {
            mode: GenerationMode::KeepVendor,
            ..Default::default()
        };
        assert_eq!(
            generate(&mut rng, &no_base),
            Err(GenerateError::NoBaseAddress)
        );
    }

    #[test]
    fn vendor_mode_uses_vendor_ouis() {
        let mut rng = SeqRandom(7);
        let req = GenerateRequest {
            mode: GenerationMode::Vendor {
                vendor_id: "apple".into(),
            },
            ..Default::default()
        };
        for _ in 0..500 {
            let mac = generate(&mut rng, &req).expect("generate");
            assert_eq!(oui::lookup(mac.oui()).map(|v| v.id), Some("apple"));
            assert!(mac.is_universal() && mac.is_unicast());
        }
        let unknown = GenerateRequest {
            mode: GenerationMode::Vendor {
                vendor_id: "acme".into(),
            },
            ..Default::default()
        };
        assert_eq!(
            generate(&mut rng, &unknown),
            Err(GenerateError::UnknownVendor("acme".into()))
        );
    }

    #[test]
    fn respects_blacklist_and_reports_exhaustion() {
        let now = Utc::now();
        let mut bl = Blacklist::default();
        // Block the whole prefix the scripted RNG always produces.
        bl.add("02:01:01".parse().expect("p"), None, None, now)
            .expect("add");
        let mut rng = ScriptedRandom {
            bytes: vec![0x01],
            pos: 0,
        };
        let req = GenerateRequest {
            blacklist: Some(&bl),
            ..Default::default()
        };
        assert_eq!(generate(&mut rng, &req), Err(GenerateError::Exhausted));

        let mut rng = SeqRandom(99);
        for _ in 0..2_000 {
            let mac = generate(&mut rng, &req).expect("generate");
            assert!(!bl.is_blocked(mac, None));
        }
    }

    #[test]
    fn serde_shape() {
        let json = serde_json::to_string(&GenerationMode::Vendor {
            vendor_id: "intel".into(),
        })
        .expect("ser");
        assert_eq!(json, r#"{"kind":"vendor","vendorId":"intel"}"#);
        let random: GenerationMode = serde_json::from_str(r#"{"kind":"random"}"#).expect("de");
        assert_eq!(random, GenerationMode::Random);
        assert!(GenerationMode::KeepVendor.produces_universal());
    }
}
