//! Exact JSON encodings for mathematical integers crossing JavaScript/WASM.
//!
//! Rust keeps integer arithmetic numeric. JSON represents `i64` payloads as
//! canonical decimal strings so JavaScript never rounds them.

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

fn gcd_u128(mut left: u128, mut right: u128) -> u128 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left
}

/// Canonical exact rational used by normalization and prompt semantics.
///
/// This is arithmetic infrastructure, not an effort-model primitive. Keeping
/// it here gives exact normalization and semantic validation one overflow and
/// canonicalization policy without coupling either module to the other.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ExactRational {
    numerator: i128,
    denominator: i128,
}

impl ExactRational {
    pub(crate) fn new(numerator: i128, denominator: i128) -> Option<Self> {
        if denominator == 0 {
            return None;
        }
        let (numerator, denominator) = if denominator < 0 {
            (numerator.checked_neg()?, denominator.checked_neg()?)
        } else {
            (numerator, denominator)
        };
        if numerator == 0 {
            return Some(Self::zero());
        }
        let divisor = gcd_u128(numerator.unsigned_abs(), denominator as u128) as i128;
        Some(Self {
            numerator: numerator / divisor,
            denominator: denominator / divisor,
        })
    }

    pub(crate) const fn zero() -> Self {
        Self {
            numerator: 0,
            denominator: 1,
        }
    }

    pub(crate) const fn one() -> Self {
        Self {
            numerator: 1,
            denominator: 1,
        }
    }

    pub(crate) fn from_integer(value: i64) -> Self {
        Self {
            numerator: i128::from(value),
            denominator: 1,
        }
    }

    pub(crate) const fn numerator(self) -> i128 {
        self.numerator
    }

    pub(crate) const fn denominator(self) -> i128 {
        self.denominator
    }

    pub(crate) fn add(self, other: Self) -> Option<Self> {
        let left = self.numerator.checked_mul(other.denominator)?;
        let right = other.numerator.checked_mul(self.denominator)?;
        Self::new(
            left.checked_add(right)?,
            self.denominator.checked_mul(other.denominator)?,
        )
    }

    pub(crate) fn subtract(self, other: Self) -> Option<Self> {
        self.add(other.negate()?)
    }

    pub(crate) fn multiply(self, other: Self) -> Option<Self> {
        Self::new(
            self.numerator.checked_mul(other.numerator)?,
            self.denominator.checked_mul(other.denominator)?,
        )
    }

    pub(crate) fn divide(self, other: Self) -> Option<Self> {
        if other.is_zero() {
            return None;
        }
        Self::new(
            self.numerator.checked_mul(other.denominator)?,
            self.denominator.checked_mul(other.numerator)?,
        )
    }

    pub(crate) fn negate(self) -> Option<Self> {
        Self::new(self.numerator.checked_neg()?, self.denominator)
    }

    pub(crate) const fn is_zero(self) -> bool {
        self.numerator == 0
    }

    pub(crate) const fn is_integer(self) -> bool {
        self.denominator == 1
    }

    pub(crate) fn sign(self) -> std::cmp::Ordering {
        self.numerator.cmp(&0)
    }

    pub(crate) fn square_root(self) -> Option<Self> {
        if self.numerator < 0 {
            return None;
        }
        let numerator = exact_square_root_u128(self.numerator as u128)?;
        let denominator = exact_square_root_u128(self.denominator as u128)?;
        Self::new(
            i128::try_from(numerator).ok()?,
            i128::try_from(denominator).ok()?,
        )
    }
}

/// Decompose a nonnegative integer radicand as `outside² * inside`, where
/// `inside` is square-free. This is exact arithmetic semantics shared by
/// equation construction and effort calculation; the effort layer separately
/// models the human search used to discover square factors.
pub(crate) fn square_free_sqrt_decomposition(mut value: u64) -> Option<(u64, u64)> {
    let mut outside = 1_u64;
    let mut factor = 2_u64;
    while factor.saturating_mul(factor) <= value {
        let square = factor.checked_mul(factor)?;
        while value.is_multiple_of(square) {
            value /= square;
            outside = outside.checked_mul(factor)?;
        }
        factor = factor.checked_add(1)?;
    }
    Some((outside, value))
}

pub(crate) fn exact_square_root_u128(value: u128) -> Option<u128> {
    if value < 2 {
        return Some(value);
    }
    let mut x = value;
    let mut next = value / 2 + 1;
    while next < x {
        x = next;
        next = (x + value / x) / 2;
    }
    x.checked_mul(x)
        .filter(|square| *square == value)
        .map(|_| x)
}
