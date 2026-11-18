//! The few bytes of the DNS wire format (RFC 1035) the latency test needs.

use std::time::Duration;

use serde::{Deserialize, Serialize};

pub const DNS_PORT: u16 = 53;
const HEADER_LEN: usize = 12;

/// A query for the root zone's NS records with recursion desired. Every resolver has that answer
/// cached, so the measured time is the round trip to the resolver itself.
pub fn build_query(id: u16) -> Vec<u8> {
    let mut q = Vec::with_capacity(HEADER_LEN + 5);
    q.extend_from_slice(&id.to_be_bytes());
    q.extend_from_slice(&[0x01, 0x00]); // flags: RD
    q.extend_from_slice(&[0, 1, 0, 0, 0, 0, 0, 0]); // QDCOUNT 1, no other records
    q.push(0); // QNAME: the root
    q.extend_from_slice(&[0, 2, 0, 1]); // QTYPE NS, QCLASS IN
    q
}

/// The parts of a reply the test looks at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResponseInfo {
    /// 0 = NOERROR, 2 = SERVFAIL, 5 = REFUSED, …
    pub rcode: u8,
    pub truncated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum WireError {
    #[error("the packet is shorter than a DNS header")]
    TooShort,
    #[error("the reply belongs to another query")]
    WrongId,
    #[error("the packet is not a reply to a standard query")]
    NotAResponse,
}

/// Checks that `packet` is the reply to the query with `expected_id`.
pub fn parse_response(expected_id: u16, packet: &[u8]) -> Result<ResponseInfo, WireError> {
    if packet.len() < HEADER_LEN {
        return Err(WireError::TooShort);
    }
    if u16::from_be_bytes([packet[0], packet[1]]) != expected_id {
        return Err(WireError::WrongId);
    }
    let flags = packet[2];
    let is_response = flags & 0x80 != 0;
    let opcode = (flags >> 3) & 0x0f;
    if !is_response || opcode != 0 {
        return Err(WireError::NotAResponse);
    }
    Ok(ResponseInfo {
        rcode: packet[3] & 0x0f,
        truncated: flags & 0x02 != 0,
    })
}

/// Round-trip times of the answered attempts, in microseconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LatencyStats {
    pub median_us: u32,
    pub min_us: u32,
    pub max_us: u32,
}

impl LatencyStats {
    pub fn from_samples(samples: &[Duration]) -> Option<Self> {
        let mut us: Vec<u64> = samples
            .iter()
            .map(|d| u64::try_from(d.as_micros()).unwrap_or(u64::MAX))
            .collect();
        us.sort_unstable();
        let (first, last) = (*us.first()?, *us.last()?);
        let mid = us.len() / 2;
        let median = if us.len().is_multiple_of(2) {
            (us[mid - 1] + us[mid]) / 2
        } else {
            us[mid]
        };
        let clamp = |v: u64| u32::try_from(v).unwrap_or(u32::MAX);
        Some(Self {
            median_us: clamp(median),
            min_us: clamp(first),
            max_us: clamp(last),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_bytes() {
        assert_eq!(
            build_query(0xabcd),
            [
                0xab, 0xcd, 0x01, 0x00, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 1
            ]
        );
    }

    fn reply(id: u16, flags: u8, rcode_byte: u8) -> Vec<u8> {
        let mut p = id.to_be_bytes().to_vec();
        p.extend_from_slice(&[flags, rcode_byte, 0, 1, 0, 1, 0, 0, 0, 0]);
        p
    }

    #[test]
    fn replies() {
        assert_eq!(
            parse_response(7, &reply(7, 0x81, 0x80)),
            Ok(ResponseInfo {
                rcode: 0,
                truncated: false
            })
        );
        assert_eq!(
            parse_response(7, &reply(7, 0x83, 0x85)),
            Ok(ResponseInfo {
                rcode: 5,
                truncated: true
            })
        );
        assert_eq!(
            parse_response(7, &reply(8, 0x81, 0x80)),
            Err(WireError::WrongId)
        );
        assert_eq!(
            parse_response(7, &reply(7, 0x01, 0x80)),
            Err(WireError::NotAResponse)
        );
        // An answer to an inverse query (opcode 1) is not ours.
        assert_eq!(
            parse_response(7, &reply(7, 0x89, 0x80)),
            Err(WireError::NotAResponse)
        );
        assert_eq!(parse_response(7, &[0, 7, 0x81]), Err(WireError::TooShort));
        // Our own query is not a reply.
        assert_eq!(
            parse_response(7, &build_query(7)),
            Err(WireError::NotAResponse)
        );
    }

    #[test]
    fn stats() {
        assert_eq!(LatencyStats::from_samples(&[]), None);
        let ms = Duration::from_millis;
        assert_eq!(
            LatencyStats::from_samples(&[ms(30), ms(10), ms(20)]),
            Some(LatencyStats {
                median_us: 20_000,
                min_us: 10_000,
                max_us: 30_000
            })
        );
        assert_eq!(
            LatencyStats::from_samples(&[ms(40), ms(10), ms(20), ms(30)]).map(|s| s.median_us),
            Some(25_000)
        );
        assert_eq!(
            LatencyStats::from_samples(&[Duration::from_secs(u64::MAX / 2)]).map(|s| s.max_us),
            Some(u32::MAX)
        );
    }
}
