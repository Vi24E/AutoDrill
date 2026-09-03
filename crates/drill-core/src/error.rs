use std::time::Duration;

use thiserror::Error;

use crate::identity::IdentityError;
use crate::model::{ProblemInvariantError, WorksheetInvariantError};
use crate::registry::RegistryError;

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum SamplingError {
    #[error("answer-conditioned sampling requires a non-empty answer domain")]
    EmptyAnswerDomain,
    #[error("layered sampling requires at least one layer")]
    EmptyLayers,
    #[error("layer minimum quota {minimum_total} exceeds worksheet size {problem_count}")]
    LayerMinimumExceedsWorksheet {
        minimum_total: usize,
        problem_count: usize,
    },
    #[error("sampling layer index {index} is outside 0..{layer_count}")]
    LayerOutOfRange { index: usize, layer_count: usize },
    #[error("candidate pool has only {available} distinct problems but {required} are required")]
    InsufficientDistinctCandidates { required: usize, available: usize },
    #[error("exhaustive finite sampling requires worksheet size {worksheet_size} to equal domain size {domain_size}")]
    ExhaustiveFiniteDomainSizeMismatch {
        worksheet_size: usize,
        domain_size: usize,
    },
    #[error("constructive layered bootstrap multiplier must be nonzero")]
    ZeroBootstrapMultiplier,
    #[error("answer-conditioned generator returned a problem for a different canonical answer")]
    AnswerConditionMismatch,
    #[error("constructive layered generator returned layer {actual} when layer {requested} was requested")]
    RequestedLayerMismatch { requested: usize, actual: usize },
}

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
    #[error(transparent)]
    InvalidRegistry(#[from] RegistryError),
    #[error(transparent)]
    InvalidSampling(#[from] SamplingError),
    #[error("generated problem violates its domain contract: {reason}")]
    InvalidGeneratedProblem { reason: &'static str },
    #[error("generated worksheet violates its domain contract: {reason}")]
    InvalidGeneratedWorksheet { reason: &'static str },
}

impl From<ProblemInvariantError> for GenerationError {
    fn from(error: ProblemInvariantError) -> Self {
        Self::InvalidGeneratedProblem {
            reason: error.description(),
        }
    }
}

impl From<WorksheetInvariantError> for GenerationError {
    fn from(error: WorksheetInvariantError) -> Self {
        Self::InvalidGeneratedWorksheet {
            reason: error.description(),
        }
    }
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
            Self::InvalidRegistry(_) => "invalid_registry",
            Self::InvalidSampling(_) => "invalid_sampling_strategy",
            Self::InvalidGeneratedProblem { .. } => "invalid_generated_problem",
            Self::InvalidGeneratedWorksheet { .. } => "invalid_generated_worksheet",
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
    #[error("answer AST size cannot exceed {max_size}")]
    AnswerSizeLimit { max_size: usize },
    #[error("the input structure {structure:?} is not allowed by the input interface")]
    StructureNotAllowed {
        structure: crate::model::EditorStructure,
    },
    #[error("the answer violates the input interface capability")]
    InputInterfaceViolation,
}
