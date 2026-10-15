//! Curated vendor table derived from the IEEE MA-L registry (see `scripts/gen-oui.mjs`).
//!
//! Only well-known vendors are included: enough to label most consumer and server NICs and to
//! offer an "imitate vendor" generation mode, while keeping the binary small.

mod vendors;

/// A vendor known to the OUI table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Vendor {
    /// Stable identifier used in settings (`"apple"`, `"tp-link"`, …).
    pub id: &'static str,
    /// Human readable name.
    pub name: &'static str,
    /// Offered in the "imitate vendor" generation mode.
    pub imitable: bool,
    /// Number of MA-L blocks assigned to this vendor in the table.
    pub oui_count: u32,
}

const RECORD: usize = 5;
static INDEX: &[u8] = include_bytes!("index.bin");

/// Date of the IEEE registry snapshot the table was generated from.
pub const SOURCE_DATE: &str = vendors::SOURCE_DATE;

fn record(i: usize) -> ([u8; 3], u16) {
    let r = &INDEX[i * RECORD..(i + 1) * RECORD];
    ([r[0], r[1], r[2]], u16::from_le_bytes([r[3], r[4]]))
}

fn record_count() -> usize {
    INDEX.len() / RECORD
}

/// All vendors in the table.
pub fn vendors() -> &'static [Vendor] {
    vendors::VENDORS
}

/// Looks a vendor up by its stable id.
pub fn vendor_by_id(id: &str) -> Option<&'static Vendor> {
    vendors::VENDORS.iter().find(|v| v.id == id)
}

/// Returns the vendor that owns `oui`, if it is one of the curated vendors.
pub fn lookup(oui: [u8; 3]) -> Option<&'static Vendor> {
    let (mut lo, mut hi) = (0usize, record_count());
    while lo < hi {
        let mid = (lo + hi) / 2;
        let (key, idx) = record(mid);
        match key.cmp(&oui) {
            std::cmp::Ordering::Less => lo = mid + 1,
            std::cmp::Ordering::Greater => hi = mid,
            std::cmp::Ordering::Equal => return vendors::VENDORS.get(usize::from(idx)),
        }
    }
    None
}

/// All OUIs assigned to the vendor with the given id (empty if the id is unknown).
pub fn ouis_of(vendor_id: &str) -> Vec<[u8; 3]> {
    let Some(pos) = vendors::VENDORS.iter().position(|v| v.id == vendor_id) else {
        return Vec::new();
    };
    (0..record_count())
        .map(record)
        .filter(|(_, idx)| usize::from(*idx) == pos)
        .map(|(oui, _)| oui)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_is_well_formed() {
        assert_eq!(INDEX.len() % RECORD, 0);
        assert!(record_count() > 1000);
        let mut prev: Option<[u8; 3]> = None;
        for i in 0..record_count() {
            let (oui, idx) = record(i);
            assert!(usize::from(idx) < vendors().len());
            assert_eq!(oui[0] & 0x03, 0, "only universal unicast OUIs are indexed");
            if let Some(p) = prev {
                assert!(p < oui, "index must be strictly sorted");
            }
            prev = Some(oui);
        }
    }

    #[test]
    fn known_assignments() {
        // Long-standing, stable assignments.
        assert_eq!(lookup([0x00, 0x03, 0x93]).map(|v| v.id), Some("apple"));
        assert_eq!(lookup([0x00, 0x15, 0x5d]).map(|v| v.id), Some("microsoft"));
        assert_eq!(lookup([0x08, 0x00, 0x27]).map(|v| v.id), Some("virtualbox"));
        assert_eq!(lookup([0x00, 0x50, 0x56]).map(|v| v.id), Some("vmware"));
        assert_eq!(lookup([0x00, 0xe0, 0x4c]).map(|v| v.id), Some("realtek"));
        // QEMU/KVM's 52:54:00 is locally administered and therefore never a vendor (v1 bug L3).
        assert_eq!(lookup([0x52, 0x54, 0x00]), None);
        assert_eq!(lookup([0x02, 0x00, 0x00]), None);
    }

    #[test]
    fn vendor_queries() {
        let apple = vendor_by_id("apple").expect("apple is curated");
        assert!(apple.imitable);
        let ouis = ouis_of("apple");
        assert_eq!(ouis.len() as u32, apple.oui_count);
        assert!(
            ouis.iter()
                .all(|o| lookup(*o).map(|v| v.id) == Some("apple"))
        );
        assert!(ouis_of("no-such-vendor").is_empty());
        assert!(vendors().iter().filter(|v| v.imitable).count() >= 10);
        assert!(!SOURCE_DATE.is_empty());
    }
}
