use std::time::Duration;

use thiserror::Error;

use crate::identity::IdentityError;

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum GenerationError {
    #[error("worksheet generation exceeded the timeout of {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },
    #[error("worksheet generation exceeded the attempt limit of {max_attempts}")]
    AttemptLimit { attempts: u64, max_attempts: u64 },
    #[error("schema version {received} is unsupported; expected {expected}")]
    UnsupportedSchemaVersion { received: u16, expected: u16 },
    #[error("theme {numeric_theme_id} has no registered generator")]
    UnknownTheme { numeric_theme_id: u32 },
    #[error("theme {numeric_theme_id} generator revision {generator_revision} is unavailable")]
    UnknownGeneratorRevision {
        numeric_theme_id: u32,
        generator_revision: u32,
    },
    #[error(transparent)]
    InvalidIdentity(#[from] IdentityError),
}

impl GenerationError {
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Timeout { .. } => "generation_timeout",
            Self::AttemptLimit { .. } => "generation_attempt_limit",
            Self::UnsupportedSchemaVersion { .. } => "unsupported_schema_version",
            Self::UnknownTheme { .. } => "unknown_theme",
            Self::UnknownGeneratorRevision { .. } => "unknown_generator_revision",
            Self::InvalidIdentity(IdentityError::UnsupportedSchemaVersion { .. }) => {
                "unsupported_schema_version"
            }
            Self::InvalidIdentity(_) => "invalid_problem_set_identity",
        }
    }

    pub fn timeout(timeout: Duration) -> Self {
        Self::Timeout {
            timeout_ms: timeout.as_millis().try_into().unwrap_or(u64::MAX),
        }
    }
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum EditorError {
    #[error("digit must be between 0 and 9")]
    InvalidDigit,
    #[error("answer AST size cannot exceed {max_size}")]
    AnswerSizeLimit { max_size: usize },
    #[error("the integer draft is outside the supported range")]
    IntegerOverflow,
    #[error("editing a negative integer draft is not supported")]
    NegativeDraft,
    #[error("the editor only supports integer draft nodes")]
    UnsupportedDraftNode,
    #[error("the editor path does not select an editable slot")]
    InvalidPath,
    #[error("the decimal draft is invalid")]
    InvalidDecimalDraft,
    #[error("the editor structure {structure:?} is not allowed by the input interface")]
    StructureNotAllowed {
        structure: crate::model::EditorStructure,
    },
    #[error("the answer violates the input interface capability")]
    InputInterfaceViolation,
}
