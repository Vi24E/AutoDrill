use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;

use crate::schema::is_supported_schema_version;
use crate::schema::SCHEMA_VERSION;
#[cfg(feature = "wire-types")]
use ts_rs::TS;

pub const MIN_DIFFICULTY: u8 = 1;
pub const MAX_DIFFICULTY: u8 = 4;
pub const DEFAULT_DIFFICULTY: Difficulty = Difficulty(2);
pub const MAX_SEED_LENGTH: usize = 16;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "wire-types", derive(TS))]
pub struct Difficulty(u8);

impl Difficulty {
    pub const fn value(self) -> u8 {
        self.0
    }
}

impl Default for Difficulty {
    fn default() -> Self {
        DEFAULT_DIFFICULTY
    }
}

impl TryFrom<u8> for Difficulty {
    type Error = IdentityError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if (MIN_DIFFICULTY..=MAX_DIFFICULTY).contains(&value) {
            Ok(Self(value))
        } else {
            Err(IdentityError::InvalidDifficulty { value })
        }
    }
}

impl Serialize for Difficulty {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u8(self.0)
    }
}

impl<'de> Deserialize<'de> for Difficulty {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::try_from(u8::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "wire-types", derive(TS))]
pub struct ProblemSetIdentity {
    schema_version: u16,
    numeric_theme_id: u32,
    generator_revision: u32,
    seed: String,
    difficulty: Difficulty,
}

impl ProblemSetIdentity {
    pub const fn schema_version(&self) -> u16 {
        self.schema_version
    }
    pub const fn numeric_theme_id(&self) -> u32 {
        self.numeric_theme_id
    }
    pub const fn generator_revision(&self) -> u32 {
        self.generator_revision
    }
    pub fn seed(&self) -> &str {
        &self.seed
    }
    pub const fn difficulty(&self) -> Difficulty {
        self.difficulty
    }

    pub fn new(
        numeric_theme_id: u32,
        generator_revision: u32,
        seed: impl Into<String>,
        difficulty: Difficulty,
    ) -> Result<Self, IdentityError> {
        Self::from_parts(
            SCHEMA_VERSION,
            numeric_theme_id,
            generator_revision,
            seed.into(),
            difficulty,
        )
    }

    fn from_parts(
        schema_version: u16,
        numeric_theme_id: u32,
        generator_revision: u32,
        seed: String,
        difficulty: Difficulty,
    ) -> Result<Self, IdentityError> {
        if !is_supported_schema_version(schema_version) {
            return Err(IdentityError::UnsupportedSchemaVersion {
                received: schema_version,
                expected: SCHEMA_VERSION,
            });
        }
        if numeric_theme_id == 0 {
            return Err(IdentityError::InvalidThemeId);
        }
        if generator_revision == 0 {
            return Err(IdentityError::InvalidGeneratorRevision);
        }
        validate_seed(&seed)?;
        Ok(Self {
            schema_version,
            numeric_theme_id,
            generator_revision,
            seed,
            difficulty,
        })
    }
}

impl<'de> Deserialize<'de> for ProblemSetIdentity {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Repr {
            schema_version: u16,
            numeric_theme_id: u32,
            generator_revision: u32,
            seed: String,
            difficulty: Difficulty,
        }
        let repr = Repr::deserialize(deserializer)?;
        Self::from_parts(
            repr.schema_version,
            repr.numeric_theme_id,
            repr.generator_revision,
            repr.seed,
            repr.difficulty,
        )
        .map_err(serde::de::Error::custom)
    }
}

impl fmt::Display for ProblemSetIdentity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}-{}-{}-{}-{}",
            self.schema_version,
            self.numeric_theme_id,
            self.generator_revision,
            self.seed,
            self.difficulty.value()
        )
    }
}

impl FromStr for ProblemSetIdentity {
    type Err = IdentityError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let mut parts = value.split('-');
        let schema_version = parse_part(parts.next(), "schema_version")?;
        let numeric_theme_id = parse_part(parts.next(), "numeric_theme_id")?;
        let generator_revision = parse_part(parts.next(), "generator_revision")?;
        let seed = parts
            .next()
            .ok_or(IdentityError::MalformedProblemSetId)?
            .to_owned();
        let difficulty_value: u8 = parse_part(parts.next(), "difficulty")?;
        let difficulty = Difficulty::try_from(difficulty_value)?;
        if parts.next().is_some() {
            return Err(IdentityError::MalformedProblemSetId);
        }
        Self::from_parts(
            schema_version,
            numeric_theme_id,
            generator_revision,
            seed,
            difficulty,
        )
    }
}

fn parse_part<T>(part: Option<&str>, name: &'static str) -> Result<T, IdentityError>
where
    T: FromStr,
{
    part.ok_or(IdentityError::MalformedProblemSetId)?
        .parse()
        .map_err(|_| IdentityError::InvalidNumericPart { name })
}

pub fn validate_seed(seed: &str) -> Result<(), IdentityError> {
    if seed.is_empty() || seed.len() > MAX_SEED_LENGTH {
        return Err(IdentityError::InvalidSeedLength {
            length: seed.len(),
            max: MAX_SEED_LENGTH,
        });
    }
    if !seed.bytes().all(is_seed_byte) {
        return Err(IdentityError::InvalidSeedCharacter);
    }
    Ok(())
}

fn is_seed_byte(value: u8) -> bool {
    matches!(value, b'1'..=b'9' | b'a'..=b'z' | b'A'..=b'Z') && !matches!(value, b'I' | b'l' | b'O')
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum IdentityError {
    #[error("problem-set ID is malformed")]
    MalformedProblemSetId,
    #[error("problem-set ID field {name} is not numeric")]
    InvalidNumericPart { name: &'static str },
    #[error("schema version {received} is unsupported; expected {expected}")]
    UnsupportedSchemaVersion { received: u16, expected: u16 },
    #[error("theme ID must be nonzero")]
    InvalidThemeId,
    #[error("generator revision must be nonzero")]
    InvalidGeneratorRevision,
    #[error("difficulty {value} is outside 1..=4")]
    InvalidDifficulty { value: u8 },
    #[error("seed length {length} is outside 1..={max}")]
    InvalidSeedLength { length: usize, max: usize },
    #[error("seed contains a character outside the versioned seed alphabet")]
    InvalidSeedCharacter,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_identity_construction_path_rejects_zero_theme_or_revision() {
        let difficulty = DEFAULT_DIFFICULTY;
        assert_eq!(
            ProblemSetIdentity::new(0, 1, "Ab3Z", difficulty),
            Err(IdentityError::InvalidThemeId)
        );
        assert_eq!(
            ProblemSetIdentity::new(1, 0, "Ab3Z", difficulty),
            Err(IdentityError::InvalidGeneratorRevision)
        );
        assert_eq!(
            format!("{SCHEMA_VERSION}-0-1-Ab3Z-{}", difficulty.value())
                .parse::<ProblemSetIdentity>(),
            Err(IdentityError::InvalidThemeId)
        );
        assert_eq!(
            format!("{SCHEMA_VERSION}-1-0-Ab3Z-{}", difficulty.value())
                .parse::<ProblemSetIdentity>(),
            Err(IdentityError::InvalidGeneratorRevision)
        );
        assert!(
            serde_json::from_value::<ProblemSetIdentity>(serde_json::json!({
                "schema_version": SCHEMA_VERSION,
                "numeric_theme_id": 0,
                "generator_revision": 1,
                "seed": "Ab3Z",
                "difficulty": difficulty.value(),
            }))
            .is_err()
        );
    }
}
