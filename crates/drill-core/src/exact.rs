//! Exact JSON encodings for mathematical integers crossing JavaScript/WASM.
//!
//! Rust keeps integer arithmetic numeric. JSON represents `i64`/`u64`
//! payloads as canonical decimal strings so JavaScript never rounds them.

pub(crate) mod i64_decimal_string {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(value: &i64, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&value.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<i64, D::Error>
    where
        D: Deserializer<'de>,
    {
        let encoded = String::deserialize(deserializer)?;
        let value = encoded.parse::<i64>().map_err(serde::de::Error::custom)?;
        if value.to_string() != encoded {
            return Err(serde::de::Error::custom(
                "expected a canonical base-10 i64 string",
            ));
        }
        Ok(value)
    }
}

pub(crate) mod u64_decimal_string {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(value: &u64, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&value.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<u64, D::Error>
    where
        D: Deserializer<'de>,
    {
        let encoded = String::deserialize(deserializer)?;
        let value = encoded.parse::<u64>().map_err(serde::de::Error::custom)?;
        if value.to_string() != encoded {
            return Err(serde::de::Error::custom(
                "expected a canonical base-10 u64 string",
            ));
        }
        Ok(value)
    }
}

/// Euclidean GCD for exact arithmetic normalization. This is deliberately
/// separate from effort-model GCD *search* operations, which model human work.
pub(crate) fn gcd_u64(mut left: u64, mut right: u64) -> u64 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left
}

pub(crate) fn gcd_u128(mut left: u128, mut right: u128) -> u128 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left
}
