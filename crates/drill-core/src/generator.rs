use std::cell::Cell;
use std::collections::HashSet;
use std::time::{Duration, Instant};

use crate::error::GenerationError;
use crate::model::{
    LayoutMetadata, OperationCounts, Problem, Worksheet, CURRICULUM_PATH, DEFAULT_COLUMNS,
    DEFAULT_PROBLEM_COUNT, DEFAULT_ROWS, GENERATOR_VERSION, MAX_OPERAND, SCHEMA_VERSION, SKILL_ID,
};
use crate::rng::DeterministicRng;

pub const DEFAULT_TIMEOUT: Duration = Duration::from_millis(100);
pub const DEFAULT_MAX_ATTEMPTS: u64 = 10_000;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GenerationConfig {
    pub problem_count: usize,
    pub timeout: Duration,
    pub max_attempts: u64,
}

impl Default for GenerationConfig {
    fn default() -> Self {
        Self {
            problem_count: DEFAULT_PROBLEM_COUNT,
            timeout: DEFAULT_TIMEOUT,
            max_attempts: DEFAULT_MAX_ATTEMPTS,
        }
    }
}

impl GenerationConfig {
    pub fn with_problem_count(mut self, problem_count: usize) -> Self {
        self.problem_count = problem_count;
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn with_max_attempts(mut self, max_attempts: u64) -> Self {
        self.max_attempts = max_attempts;
        self
    }
}

/// The clock boundary makes timeout behavior deterministic in native tests and
/// leaves the production implementation on a monotonic source.
pub trait MonotonicClock {
    fn now(&self) -> Duration;
}

#[derive(Debug)]
pub struct SystemClock {
    origin: Instant,
}

impl SystemClock {
    pub fn new() -> Self {
        Self {
            origin: Instant::now(),
        }
    }
}

impl Default for SystemClock {
    fn default() -> Self {
        Self::new()
    }
}

impl MonotonicClock for SystemClock {
    fn now(&self) -> Duration {
        self.origin.elapsed()
    }
}

/// Test helper that advances by a fixed amount on every clock read.  It is also
/// useful to callers that want to assert the timeout path without sleeping.
#[derive(Debug)]
pub struct StepClock {
    current: Cell<Duration>,
    step: Duration,
}

impl StepClock {
    pub fn new(start: Duration, step: Duration) -> Self {
        Self {
            current: Cell::new(start),
            step,
        }
    }
}

impl MonotonicClock for StepClock {
    fn now(&self) -> Duration {
        let current = self.current.get();
        self.current.set(current.saturating_add(self.step));
        current
    }
}

pub fn generate_problem(seed: &str) -> Result<Problem, GenerationError> {
    let mut rng = DeterministicRng::from_seed(seed);
    let (left, right) = rng.next_ordered_pair();
    Ok(problem(1, left, right))
}

pub fn generate_worksheet(seed: &str) -> Result<Worksheet, GenerationError> {
    let config = GenerationConfig::default();
    let clock = SystemClock::new();
    generate_worksheet_with_clock(seed, &config, &clock)
}

pub fn generate_worksheet_with_config(
    seed: &str,
    config: &GenerationConfig,
) -> Result<Worksheet, GenerationError> {
    let clock = SystemClock::new();
    generate_worksheet_with_clock(seed, config, &clock)
}

pub fn generate_worksheet_with_clock<C: MonotonicClock + ?Sized>(
    seed: &str,
    config: &GenerationConfig,
    clock: &C,
) -> Result<Worksheet, GenerationError> {
    if config.problem_count > 81 {
        return Err(GenerationError::InvalidProblemCount {
            requested: config.problem_count,
        });
    }

    let started = clock.now();
    let mut rng = DeterministicRng::from_seed(seed);
    let mut seen = HashSet::with_capacity(config.problem_count);
    let mut problems = Vec::with_capacity(config.problem_count);
    let mut attempts = 0_u64;

    while problems.len() < config.problem_count {
        if elapsed(started, clock.now()) >= config.timeout {
            return Err(GenerationError::timeout(config.timeout));
        }
        if attempts >= config.max_attempts {
            return Err(GenerationError::AttemptLimit {
                attempts,
                max_attempts: config.max_attempts,
            });
        }

        let (left, right) = rng.next_ordered_pair();
        attempts += 1;
        let key = (u16::from(left) * 10) + u16::from(right);
        if !seen.insert(key) {
            continue;
        }
        problems.push(problem((problems.len() + 1) as u32, left, right));

        // Check after a draw too, so an injected clock can deterministically
        // exercise timeout even when the final unique pair was just accepted.
        if elapsed(started, clock.now()) >= config.timeout {
            return Err(GenerationError::timeout(config.timeout));
        }
    }

    Ok(Worksheet {
        schema_version: SCHEMA_VERSION,
        skill_id: SKILL_ID.to_owned(),
        curriculum_path: CURRICULUM_PATH
            .iter()
            .map(|item| (*item).to_owned())
            .collect(),
        layout: LayoutMetadata {
            problem_count: config.problem_count,
            columns: DEFAULT_COLUMNS,
            rows: DEFAULT_ROWS,
        },
        generator_version: GENERATOR_VERSION.to_owned(),
        seed: seed.to_owned(),
        problems,
    })
}

fn elapsed(started: Duration, current: Duration) -> Duration {
    current.saturating_sub(started)
}

fn problem(id: u32, left: u8, right: u8) -> Problem {
    let answer = left + right;
    debug_assert!((1..=MAX_OPERAND).contains(&left));
    debug_assert!((1..=MAX_OPERAND).contains(&right));
    Problem {
        schema_version: SCHEMA_VERSION,
        id,
        left,
        right,
        answer,
        operation_counts: OperationCounts::one_digit_addition(answer >= 10),
    }
}
