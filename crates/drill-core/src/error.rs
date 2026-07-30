use std::time::Duration;

use thiserror::Error;

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum GenerationError {
    #[error("worksheet generation exceeded the timeout of {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },
    #[error("worksheet generation exceeded the attempt limit of {max_attempts}")]
    AttemptLimit { attempts: u64, max_attempts: u64 },
    #[error("requested problem count {requested} exceeds the 81 unique ordered pairs")]
    InvalidProblemCount { requested: usize },
}

impl GenerationError {
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Timeout { .. } => "generation_timeout",
            Self::AttemptLimit { .. } => "generation_attempt_limit",
            Self::InvalidProblemCount { .. } => "invalid_problem_count",
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
    #[error("the integer draft is outside the supported range")]
    IntegerOverflow,
    #[error("editing a negative integer draft is not supported")]
    NegativeDraft,
}
