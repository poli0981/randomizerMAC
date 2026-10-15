//! Randomness sources. Production code uses the operating system CSPRNG; tests inject a
//! deterministic source.

/// Error returned when the operating system random number generator is unavailable.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("the operating system random number generator failed: {0}")]
pub struct RandomError(pub String);

/// A source of random bytes.
pub trait RandomSource {
    fn fill(&mut self, buf: &mut [u8]) -> Result<(), RandomError>;

    /// A uniformly distributed integer in `0..bound` (rejection sampling, no modulo bias).
    fn below(&mut self, bound: u32) -> Result<u32, RandomError> {
        assert!(bound > 0, "bound must be positive");
        let zone = u32::MAX - (u32::MAX % bound);
        loop {
            let mut b = [0u8; 4];
            self.fill(&mut b)?;
            let v = u32::from_le_bytes(b);
            if v < zone {
                return Ok(v % bound);
            }
        }
    }
}

/// The operating system CSPRNG (`getrandom`).
#[derive(Debug, Default, Clone, Copy)]
pub struct OsRandom;

impl RandomSource for OsRandom {
    fn fill(&mut self, buf: &mut [u8]) -> Result<(), RandomError> {
        getrandom::fill(buf).map_err(|e| RandomError(e.to_string()))
    }
}

/// Random lowercase hex identifier (16 chars) for history/blacklist entries.
pub fn new_id() -> String {
    let mut b = [0u8; 8];
    if OsRandom.fill(&mut b).is_err() {
        // Extremely unlikely; fall back to a time-derived id rather than failing a user action.
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default();
        b = (nanos as u64).to_le_bytes();
    }
    b.iter().map(|x| format!("{x:02x}")).collect()
}

#[cfg(test)]
pub(crate) mod test_support {
    use super::*;

    /// Deterministic xorshift source for reproducible tests.
    pub struct SeqRandom(pub u64);

    impl RandomSource for SeqRandom {
        fn fill(&mut self, buf: &mut [u8]) -> Result<(), RandomError> {
            for b in buf.iter_mut() {
                self.0 ^= self.0 << 13;
                self.0 ^= self.0 >> 7;
                self.0 ^= self.0 << 17;
                *b = (self.0 >> 24) as u8;
            }
            Ok(())
        }
    }

    /// Replays a fixed byte sequence (cycling).
    pub struct ScriptedRandom {
        pub bytes: Vec<u8>,
        pub pos: usize,
    }

    impl RandomSource for ScriptedRandom {
        fn fill(&mut self, buf: &mut [u8]) -> Result<(), RandomError> {
            for b in buf.iter_mut() {
                *b = self.bytes[self.pos % self.bytes.len()];
                self.pos += 1;
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::SeqRandom;
    use super::*;

    #[test]
    fn below_is_in_range() {
        let mut rng = SeqRandom(0x1234_5678_9abc_def0);
        for bound in [1u32, 2, 3, 7, 100, 1533] {
            for _ in 0..500 {
                assert!(rng.below(bound).expect("rng") < bound);
            }
        }
    }

    #[test]
    fn os_random_and_ids() {
        let mut a = [0u8; 32];
        OsRandom.fill(&mut a).expect("OS RNG available in tests");
        assert_ne!(a, [0u8; 32]);
        let id = new_id();
        assert_eq!(id.len(), 16);
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(new_id(), id);
    }
}
