//! A tiny deterministic RNG kept in the domain crate so output is stable across
//! native and WASM builds without relying on platform-specific randomness.

/// SplitMix64 is fast, reproducible, and has a well-defined integer algorithm.
#[derive(Clone, Debug)]
pub(crate) struct DeterministicRng {
    state: u64,
}

impl DeterministicRng {
    pub(crate) fn from_seed(seed: &str) -> Self {
        Self {
            state: seed_to_u64(seed),
        }
    }

    pub(crate) fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Draw an integer uniformly in `[0, upper)`, rejecting the partial range
    /// that would otherwise introduce modulo bias.
    pub(crate) fn next_bounded(&mut self, upper: u64) -> u64 {
        assert!(upper > 0, "upper bound must be positive");
        let zone = u64::MAX - (u64::MAX % upper);
        loop {
            let value = self.next_u64();
            if value < zone {
                return value % upper;
            }
        }
    }

    pub(crate) fn next_ordered_pair(&mut self) -> (u8, u8) {
        let pair = self.next_bounded(81);
        ((pair / 9 + 1) as u8, (pair % 9 + 1) as u8)
    }
}

/// Decimal and hexadecimal seeds remain intuitive, while arbitrary strings are
/// still accepted using a stable FNV-1a hash.  The original string is retained
/// in the worksheet DTO so JS never has to round a numeric seed.
pub(crate) fn seed_to_u64(seed: &str) -> u64 {
    let trimmed = seed.trim();
    if let Some(hex) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        if let Ok(value) = u64::from_str_radix(hex, 16) {
            return value;
        }
    }
    if let Ok(value) = trimmed.parse::<u64>() {
        return value;
    }

    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in seed.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x1000_0000_01b3);
    }
    hash
}
