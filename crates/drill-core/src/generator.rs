use std::cell::Cell;
use std::collections::HashSet;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use crate::answer::AnswerNode;
use crate::effort::{
    calculate_graph_effort, linear_equation_graph, one_digit_addition_graph, OperationWeights,
};
use crate::error::GenerationError;
use crate::identity::{validate_seed, ProblemSetIdentity};
use crate::model::{
    AnswerInputInterface, AnswerSchema, EditorStructure, GenerateProblemRequest,
    GenerateWorksheetRequest, LayoutMetadata, Problem, ProblemPrompt, RationalCoefficient,
    Worksheet, MAX_ANSWER, MAX_OPERAND, MIN_ANSWER, MIN_OPERAND, SCHEMA_VERSION,
};
use crate::registry::{active_registration, registration, resolved_weights, ThemeRegistration};
use crate::rng::DeterministicRng;

pub const DEFAULT_TIMEOUT: Duration = Duration::from_millis(100);
pub const DEFAULT_MAX_ATTEMPTS: u64 = 10_000;
const CANDIDATE_POOL_MULTIPLIER: usize = 8;
const DIFFICULTY_BOOTSTRAP_DRAWS: usize = 5;
const EFFORT_TRIM_PER_SIDE: usize = 2;
const DIVERSITY_MULTIPLIER: usize = 2;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GenerationConfig {
    pub timeout: Duration,
    pub max_attempts: u64,
}

impl Default for GenerationConfig {
    fn default() -> Self {
        Self {
            timeout: DEFAULT_TIMEOUT,
            max_attempts: DEFAULT_MAX_ATTEMPTS,
        }
    }
}

impl GenerationConfig {
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn with_max_attempts(mut self, max_attempts: u64) -> Self {
        self.max_attempts = max_attempts;
        self
    }

    pub fn from_request(request: &GenerateWorksheetRequest) -> Self {
        Self {
            timeout: request
                .timeout_ms
                .map(Duration::from_millis)
                .unwrap_or(DEFAULT_TIMEOUT),
            max_attempts: request.max_attempts.unwrap_or(DEFAULT_MAX_ATTEMPTS),
        }
    }
}

/// Registry-backed generator interface. Revisioned implementations remain
/// addressable so a decoded problem-set ID can reproduce historic worksheets.
pub trait ProblemGenerator: Sync {
    fn registration(&self) -> &'static ThemeRegistration;
    fn draw_candidate(
        &self,
        rng: &mut DeterministicRng,
        ordinal: u32,
        weights: &OperationWeights,
    ) -> Option<Problem>;

    /// Optional canonical-answer support. Each candidate first samples one
    /// answer uniformly from this domain, then constructs an expression
    /// conditioned on that fixed answer. Difficulty compares the completed
    /// candidates globally, across different answers.
    fn answer_domain(&self) -> Option<&'static [AnswerNode]> {
        None
    }

    fn draw_candidate_for_answer(
        &self,
        rng: &mut DeterministicRng,
        ordinal: u32,
        weights: &OperationWeights,
        _answer: &AnswerNode,
    ) -> Option<Problem> {
        self.draw_candidate(rng, ordinal, weights)
    }
}

#[derive(Debug)]
pub struct OneDigitAdditionGenerator;

impl ProblemGenerator for OneDigitAdditionGenerator {
    fn registration(&self) -> &'static ThemeRegistration {
        &crate::registry::ONE_DIGIT_ADDITION_REGISTRATION
    }

    fn draw_candidate(
        &self,
        rng: &mut DeterministicRng,
        ordinal: u32,
        weights: &OperationWeights,
    ) -> Option<Problem> {
        let (left, right) = rng.next_ordered_pair();
        Some(addition_problem(ordinal, left, right, weights))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LinearEquationMode {
    IntegerSolution,
    RationalSolution,
}

#[derive(Debug)]
pub struct LinearEquationGenerator {
    registration: &'static ThemeRegistration,
    mode: LinearEquationMode,
}

impl ProblemGenerator for LinearEquationGenerator {
    fn registration(&self) -> &'static ThemeRegistration {
        self.registration
    }

    fn answer_domain(&self) -> Option<&'static [AnswerNode]> {
        Some(linear_answer_domain(self.mode))
    }

    fn draw_candidate(
        &self,
        rng: &mut DeterministicRng,
        ordinal: u32,
        weights: &OperationWeights,
    ) -> Option<Problem> {
        let domain = linear_answer_domain(self.mode);
        let answer = &domain[rng.next_bounded(domain.len() as u64) as usize];
        self.draw_candidate_for_answer(rng, ordinal, weights, answer)
    }

    fn draw_candidate_for_answer(
        &self,
        rng: &mut DeterministicRng,
        ordinal: u32,
        weights: &OperationWeights,
        answer: &AnswerNode,
    ) -> Option<Problem> {
        let solution = answer_node_rational(answer)?;
        let shape = rng.next_bounded(4);

        // For the rational theme, most candidates deliberately make the final
        // B/A reducible after transposition. This teaches actual reduction
        // without changing the uniformly chosen canonical answer.
        let prefer_reduction = self.mode == LinearEquationMode::RationalSolution
            && !solution.is_zero()
            && rng.next_bounded(4) != 0;
        let coefficients = if prefer_reduction {
            draw_reduction_conditioned_coefficients(rng, self.mode, shape, solution)
                .or_else(|| draw_conditioned_coefficients(rng, self.mode, shape, solution))
        } else {
            draw_conditioned_coefficients(rng, self.mode, shape, solution)
        }?;
        let (a, b, c, d) = coefficients;
        let left_negative_constant_as_subtraction = b.numerator < 0 && rng.next_bounded(2) == 0;
        let right_negative_constant_as_subtraction = d.numerator < 0 && rng.next_bounded(2) == 0;
        Some(linear_equation_problem(
            ordinal,
            self.registration.numeric_theme_id,
            self.mode,
            a,
            b,
            c,
            d,
            left_negative_constant_as_subtraction,
            right_negative_constant_as_subtraction,
            solution,
            weights,
        ))
    }
}

static ONE_DIGIT_ADDITION_GENERATOR: OneDigitAdditionGenerator = OneDigitAdditionGenerator;
static LINEAR_EQUATION_1_GENERATOR: LinearEquationGenerator = LinearEquationGenerator {
    registration: &crate::registry::LINEAR_EQUATION_1_REGISTRATION,
    mode: LinearEquationMode::IntegerSolution,
};
static LINEAR_EQUATION_2_GENERATOR: LinearEquationGenerator = LinearEquationGenerator {
    registration: &crate::registry::LINEAR_EQUATION_2_REGISTRATION,
    mode: LinearEquationMode::RationalSolution,
};

static REGISTERED_GENERATORS: [&dyn ProblemGenerator; 3] = [
    &ONE_DIGIT_ADDITION_GENERATOR,
    &LINEAR_EQUATION_1_GENERATOR,
    &LINEAR_EQUATION_2_GENERATOR,
];

pub fn registered_generator(
    numeric_theme_id: u32,
    generator_revision: u32,
) -> Option<&'static dyn ProblemGenerator> {
    registration(numeric_theme_id, generator_revision)?;
    REGISTERED_GENERATORS.iter().copied().find(|generator| {
        let registration = generator.registration();
        registration.numeric_theme_id == numeric_theme_id
            && registration.generator_revision == generator_revision
    })
}

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
    generate_problem_request(&GenerateProblemRequest {
        seed: seed.to_owned(),
        ..GenerateProblemRequest::default()
    })
}

pub fn generate_problem_request(
    request: &GenerateProblemRequest,
) -> Result<Problem, GenerationError> {
    if request.schema_version != SCHEMA_VERSION {
        return Err(GenerationError::UnsupportedSchemaVersion {
            received: request.schema_version,
            expected: SCHEMA_VERSION,
        });
    }
    validate_seed(&request.seed)?;
    let registration =
        active_registration(request.numeric_theme_id).ok_or(GenerationError::UnknownTheme {
            numeric_theme_id: request.numeric_theme_id,
        })?;
    let generator = registered_generator(
        registration.numeric_theme_id,
        registration.generator_revision,
    )
    .ok_or(GenerationError::UnknownGeneratorRevision {
        numeric_theme_id: registration.numeric_theme_id,
        generator_revision: registration.generator_revision,
    })?;
    let mut rng = DeterministicRng::from_seed(&request.seed);
    let weights = resolved_weights(registration);
    let fixed_answer = generator
        .answer_domain()
        .map(|domain| domain[rng.next_bounded(domain.len() as u64) as usize].clone());
    for ordinal in 1..=DEFAULT_MAX_ATTEMPTS {
        let problem = match fixed_answer.as_ref() {
            Some(answer) => {
                generator.draw_candidate_for_answer(&mut rng, ordinal as u32, &weights, answer)
            }
            None => generator.draw_candidate(&mut rng, ordinal as u32, &weights),
        };
        if let Some(problem) = problem {
            return Ok(problem);
        }
    }
    Err(GenerationError::AttemptLimit {
        attempts: DEFAULT_MAX_ATTEMPTS,
        max_attempts: DEFAULT_MAX_ATTEMPTS,
    })
}

pub fn generate_worksheet(seed: &str) -> Result<Worksheet, GenerationError> {
    let request = GenerateWorksheetRequest {
        seed: seed.to_owned(),
        ..GenerateWorksheetRequest::default()
    };
    generate_worksheet_request(&request)
}

pub fn generate_worksheet_request(
    request: &GenerateWorksheetRequest,
) -> Result<Worksheet, GenerationError> {
    let clock = SystemClock::new();
    generate_worksheet_request_with_clock(request, &clock)
}

pub fn generate_worksheet_request_with_clock<C: MonotonicClock + ?Sized>(
    request: &GenerateWorksheetRequest,
    clock: &C,
) -> Result<Worksheet, GenerationError> {
    if request.schema_version != SCHEMA_VERSION {
        return Err(GenerationError::UnsupportedSchemaVersion {
            received: request.schema_version,
            expected: SCHEMA_VERSION,
        });
    }
    validate_seed(&request.seed)?;
    let registration =
        active_registration(request.numeric_theme_id).ok_or(GenerationError::UnknownTheme {
            numeric_theme_id: request.numeric_theme_id,
        })?;
    let identity = ProblemSetIdentity::new(
        registration.numeric_theme_id,
        registration.generator_revision,
        request.seed.clone(),
        request.difficulty,
    )?;
    generate_identity_with_clock(&identity, &GenerationConfig::from_request(request), clock)
}

pub fn regenerate_problem_set(problem_set_id: &str) -> Result<Worksheet, GenerationError> {
    let identity: ProblemSetIdentity = problem_set_id.parse()?;
    let clock = SystemClock::new();
    generate_identity_with_clock(&identity, &GenerationConfig::default(), &clock)
}

pub fn generate_identity_with_clock<C: MonotonicClock + ?Sized>(
    identity: &ProblemSetIdentity,
    config: &GenerationConfig,
    clock: &C,
) -> Result<Worksheet, GenerationError> {
    if identity.schema_version != SCHEMA_VERSION {
        return Err(GenerationError::UnsupportedSchemaVersion {
            received: identity.schema_version,
            expected: SCHEMA_VERSION,
        });
    }
    let registration = registration(identity.numeric_theme_id, identity.generator_revision).ok_or(
        GenerationError::UnknownGeneratorRevision {
            numeric_theme_id: identity.numeric_theme_id,
            generator_revision: identity.generator_revision,
        },
    )?;
    let generator = registered_generator(identity.numeric_theme_id, identity.generator_revision)
        .ok_or(GenerationError::UnknownGeneratorRevision {
            numeric_theme_id: identity.numeric_theme_id,
            generator_revision: identity.generator_revision,
        })?;
    generate_with_generator(identity, registration, generator, config, clock)
}

fn generate_with_generator<C: MonotonicClock + ?Sized>(
    identity: &ProblemSetIdentity,
    registration: &'static ThemeRegistration,
    generator: &dyn ProblemGenerator,
    config: &GenerationConfig,
    clock: &C,
) -> Result<Worksheet, GenerationError> {
    let started = clock.now();
    let mut attempts = 0_u64;
    let mut rng = DeterministicRng::from_seed(&identity.seed);
    let weights = resolved_weights(registration);
    let n = registration.problem_count;
    let pool_size = CANDIDATE_POOL_MULTIPLIER * n;

    let mut pool = loop {
        let mut candidate_pool = Vec::with_capacity(pool_size);
        let mut distinct = HashSet::with_capacity(pool_size);
        while candidate_pool.len() < pool_size {
            // When a generator exposes an answer domain, sample the answer
            // exactly once for this candidate. If construction fails, retry
            // the expression while keeping that answer fixed. This preserves
            // the requested uniform source distribution without requiring or
            // forbidding duplicate answers in the final worksheet.
            let fixed_answer = generator
                .answer_domain()
                .map(|domain| domain[rng.next_bounded(domain.len() as u64) as usize].clone());
            loop {
                consume_attempt(started, clock, config, &mut attempts)?;
                let ordinal = u32::try_from(attempts).unwrap_or(u32::MAX);
                let problem = match fixed_answer.as_ref() {
                    Some(answer) => {
                        generator.draw_candidate_for_answer(&mut rng, ordinal, &weights, answer)
                    }
                    None => generator.draw_candidate(&mut rng, ordinal, &weights),
                };
                let Some(problem) = problem else {
                    continue;
                };
                if fixed_answer
                    .as_ref()
                    .is_some_and(|answer| problem.canonical_answer != *answer)
                {
                    continue;
                }
                distinct.insert(problem_key(&problem));
                candidate_pool.push(problem);
                break;
            }
        }
        check_timeout(started, clock, config)?;
        if distinct.len() >= DIVERSITY_MULTIPLIER * n {
            break candidate_pool;
        }
        // The full pool is discarded. The next loop consumes fresh attempts
        // and fresh deterministic RNG draws.
    };

    pool.sort_by(|left, right| {
        left.effort
            .total_cmp(&right.effort)
            .then_with(|| problem_key(left).cmp(&problem_key(right)))
            .then_with(|| left.id.cmp(&right.id))
    });

    let bootstrap_count = n + EFFORT_TRIM_PER_SIDE * 2;
    let mut selected = Vec::with_capacity(bootstrap_count);
    let mut selected_expressions = HashSet::with_capacity(bootstrap_count);
    while selected.len() < bootstrap_count {
        consume_attempt(started, clock, config, &mut attempts)?;
        let mut draws = [0_usize; DIFFICULTY_BOOTSTRAP_DRAWS];
        for draw in &mut draws {
            *draw = rng.next_bounded(pool.len() as u64) as usize + 1;
        }
        draws.sort_unstable();
        let selected_index = draws[usize::from(identity.difficulty.value() - 1)] - 1;
        let key = problem_key(&pool[selected_index]);
        if !selected_expressions.insert(key) {
            continue;
        }
        selected.push(pool.remove(selected_index));
    }

    // Trim the two easiest and two hardest bootstrap selections. This keeps
    // the rank bias from difficulty while suppressing accidental effort
    // outliers before the final worksheet is shuffled.
    selected.sort_by(|left, right| {
        left.effort
            .total_cmp(&right.effort)
            .then_with(|| problem_key(left).cmp(&problem_key(right)))
            .then_with(|| left.id.cmp(&right.id))
    });
    selected = selected
        .into_iter()
        .skip(EFFORT_TRIM_PER_SIDE)
        .take(n)
        .collect();

    // Fisher-Yates using the same deterministic RNG stream.
    for upper in (1..selected.len()).rev() {
        let swap_with = rng.next_bounded((upper + 1) as u64) as usize;
        selected.swap(upper, swap_with);
    }
    for (index, problem) in selected.iter_mut().enumerate() {
        problem.id = (index + 1) as u32;
    }
    check_timeout(started, clock, config)?;

    Ok(Worksheet {
        schema_version: SCHEMA_VERSION,
        problem_set_id: identity.to_string(),
        identity: identity.clone(),
        skill_id: registration.skill_id.to_owned(),
        curriculum_path: registration
            .curriculum_path
            .iter()
            .map(|segment| (*segment).to_owned())
            .collect(),
        layout: LayoutMetadata {
            problem_count: registration.problem_count,
            columns: registration.columns,
            rows: registration.rows,
        },
        problems: selected,
    })
}

fn consume_attempt<C: MonotonicClock + ?Sized>(
    started: Duration,
    clock: &C,
    config: &GenerationConfig,
    attempts: &mut u64,
) -> Result<(), GenerationError> {
    check_timeout(started, clock, config)?;
    if *attempts >= config.max_attempts {
        return Err(GenerationError::AttemptLimit {
            attempts: *attempts,
            max_attempts: config.max_attempts,
        });
    }
    *attempts += 1;
    Ok(())
}

fn check_timeout<C: MonotonicClock + ?Sized>(
    started: Duration,
    clock: &C,
    config: &GenerationConfig,
) -> Result<(), GenerationError> {
    if clock.now().saturating_sub(started) >= config.timeout {
        Err(GenerationError::timeout(config.timeout))
    } else {
        Ok(())
    }
}

fn problem_key(problem: &Problem) -> ProblemPrompt {
    match &problem.prompt {
        ProblemPrompt::Addition { left, right } => ProblemPrompt::Addition {
            left: *left,
            right: *right,
        },
        ProblemPrompt::LinearEquation { a, b, c, d, .. } => ProblemPrompt::LinearEquation {
            a: *a,
            b: *b,
            c: *c,
            d: *d,
            left_negative_constant_as_subtraction: false,
            right_negative_constant_as_subtraction: false,
        },
    }
}

fn linear_answer_domain(mode: LinearEquationMode) -> &'static [AnswerNode] {
    static INTEGER: OnceLock<Vec<AnswerNode>> = OnceLock::new();
    static RATIONAL: OnceLock<Vec<AnswerNode>> = OnceLock::new();
    match mode {
        LinearEquationMode::IntegerSolution => {
            INTEGER.get_or_init(|| (-15_i64..=15).map(AnswerNode::Integer).collect())
        }
        LinearEquationMode::RationalSolution => RATIONAL.get_or_init(|| {
            linear_solution_domain(mode)
                .iter()
                .copied()
                .map(rational_answer)
                .collect()
        }),
    }
}

fn linear_solution_domain(mode: LinearEquationMode) -> &'static [RationalCoefficient] {
    static INTEGER: OnceLock<Vec<RationalCoefficient>> = OnceLock::new();
    static RATIONAL: OnceLock<Vec<RationalCoefficient>> = OnceLock::new();
    match mode {
        LinearEquationMode::IntegerSolution => INTEGER.get_or_init(|| {
            (-15_i64..=15)
                .map(|value| RationalCoefficient::new(value, 1).expect("integer solution"))
                .collect()
        }),
        LinearEquationMode::RationalSolution => RATIONAL.get_or_init(|| {
            let mut values = (-15_i64..=15)
                .map(|value| RationalCoefficient::new(value, 1).expect("integer solution"))
                .collect::<Vec<_>>();
            for denominator in 2_i64..=12 {
                let max_numerator = if denominator == 2 { 20_i64 } else { 15_i64 };
                for numerator_abs in 1_i64..=max_numerator {
                    let value = RationalCoefficient::new(numerator_abs, denominator)
                        .expect("positive rational solution");
                    // The requested limits apply to the final reduced answer.
                    if value.denominator != denominator {
                        continue;
                    }
                    values.push(value);
                    values.push(
                        RationalCoefficient::new(-numerator_abs, denominator)
                            .expect("negative rational solution"),
                    );
                }
            }
            values.sort_unstable();
            values.dedup();
            values
        }),
    }
}

fn answer_node_rational(answer: &AnswerNode) -> Option<RationalCoefficient> {
    match answer {
        AnswerNode::Integer(value) => RationalCoefficient::new(*value, 1),
        AnswerNode::Fraction {
            numerator,
            denominator,
        } => {
            let numerator = numerator.as_integer()?;
            let denominator = denominator.as_integer()?;
            RationalCoefficient::new(numerator, denominator)
        }
        _ => None,
    }
}

fn coefficient_domain(
    mode: LinearEquationMode,
    allow_zero: bool,
) -> &'static [RationalCoefficient] {
    match (mode, allow_zero) {
        (LinearEquationMode::IntegerSolution, false) => linear_integer_domain(),
        (LinearEquationMode::IntegerSolution, true) => linear_integer_domain_with_zero(),
        (LinearEquationMode::RationalSolution, false) => linear_rational_domain(),
        (LinearEquationMode::RationalSolution, true) => linear_rational_domain_with_zero(),
    }
}

fn coefficient_allowed(
    value: RationalCoefficient,
    mode: LinearEquationMode,
    allow_zero: bool,
) -> bool {
    coefficient_domain(mode, allow_zero).contains(&value)
}

/// Pick (minuend, subtrahend) uniformly from admissible pairs satisfying
/// `minuend - subtrahend = difference`.
fn draw_difference_pair(
    rng: &mut DeterministicRng,
    mode: LinearEquationMode,
    difference: RationalCoefficient,
    minuend_allow_zero: bool,
    subtrahend_allow_zero: bool,
) -> Option<(RationalCoefficient, RationalCoefficient)> {
    let minuends = coefficient_domain(mode, minuend_allow_zero);
    let subtrahends = coefficient_domain(mode, subtrahend_allow_zero);
    let valid = minuends
        .iter()
        .copied()
        .filter_map(|minuend| {
            let subtrahend = minuend.subtract(difference)?;
            subtrahends
                .contains(&subtrahend)
                .then_some((minuend, subtrahend))
        })
        .collect::<Vec<_>>();
    (!valid.is_empty()).then(|| valid[rng.next_bounded(valid.len() as u64) as usize])
}

fn draw_conditioned_coefficients(
    rng: &mut DeterministicRng,
    mode: LinearEquationMode,
    shape: u64,
    solution: RationalCoefficient,
) -> Option<(
    RationalCoefficient,
    RationalCoefficient,
    RationalCoefficient,
    RationalCoefficient,
)> {
    let zero = RationalCoefficient::zero();
    match shape {
        // ax + b = 0, so b = -ax.
        0 => {
            let a = draw_nonzero_linear_coefficient(rng, mode);
            let ax = a.multiply(solution)?;
            let b = zero.subtract(ax)?;
            coefficient_allowed(b, mode, true).then_some((a, b, zero, zero))
        }
        // ax + b = d, so d - b = ax.
        1 => {
            let a = draw_nonzero_linear_coefficient(rng, mode);
            let difference = a.multiply(solution)?;
            let (d, b) = draw_difference_pair(rng, mode, difference, false, true)?;
            Some((a, b, zero, d))
        }
        // ax + b = cx, so b = -(a-c)x. b=0 is intentionally rejected.
        2 => {
            let a = draw_nonzero_linear_coefficient(rng, mode);
            let c = draw_nonzero_linear_coefficient(rng, mode);
            let a_minus_c = a.subtract(c)?;
            if a_minus_c.is_zero() {
                return None;
            }
            let b = zero.subtract(a_minus_c.multiply(solution)?)?;
            if b.is_zero() || !coefficient_allowed(b, mode, false) {
                return None;
            }
            Some((a, b, c, zero))
        }
        // ax + b = cx + d, so d - b = (a-c)x.
        3 => {
            let a = draw_nonzero_linear_coefficient(rng, mode);
            let c = draw_nonzero_linear_coefficient(rng, mode);
            let a_minus_c = a.subtract(c)?;
            if a_minus_c.is_zero() {
                return None;
            }
            let difference = a_minus_c.multiply(solution)?;
            let (d, b) = draw_difference_pair(rng, mode, difference, false, true)?;
            Some((a, b, c, d))
        }
        _ => None,
    }
}

fn draw_reduction_conditioned_coefficients(
    rng: &mut DeterministicRng,
    mode: LinearEquationMode,
    shape: u64,
    solution: RationalCoefficient,
) -> Option<(
    RationalCoefficient,
    RationalCoefficient,
    RationalCoefficient,
    RationalCoefficient,
)> {
    if solution.is_zero() {
        return None;
    }
    let zero = RationalCoefficient::zero();
    // Try a small common factor first. A = kq and B = kp make x = B/A
    // intentionally reducible by k after transposition.
    let first_k = 2_i64 + rng.next_bounded(3) as i64;
    for offset in 0_i64..3 {
        let k = 2 + ((first_k - 2 + offset) % 3);
        let sign = if rng.next_bounded(2) == 0 {
            1_i64
        } else {
            -1_i64
        };
        let a_total =
            RationalCoefficient::new(sign.checked_mul(k)?.checked_mul(solution.denominator)?, 1)?;
        let b_total =
            RationalCoefficient::new(sign.checked_mul(k)?.checked_mul(solution.numerator)?, 1)?;
        let candidate = match shape {
            0 => {
                let a = a_total;
                let b = zero.subtract(b_total)?;
                (coefficient_allowed(a, mode, false) && coefficient_allowed(b, mode, true))
                    .then_some((a, b, zero, zero))
            }
            1 => {
                if !coefficient_allowed(a_total, mode, false) {
                    None
                } else {
                    draw_difference_pair(rng, mode, b_total, false, true)
                        .map(|(d, b)| (a_total, b, zero, d))
                }
            }
            2 => {
                let b = zero.subtract(b_total)?;
                if !coefficient_allowed(b, mode, false) {
                    None
                } else {
                    draw_difference_pair(rng, mode, a_total, false, false)
                        .map(|(a, c)| (a, b, c, zero))
                }
            }
            3 => {
                let x_pair = draw_difference_pair(rng, mode, a_total, false, false);
                let constant_pair = draw_difference_pair(rng, mode, b_total, false, true);
                match (x_pair, constant_pair) {
                    (Some((a, c)), Some((d, b))) => Some((a, b, c, d)),
                    _ => None,
                }
            }
            _ => None,
        };
        if candidate.is_some() {
            return candidate;
        }
    }
    None
}

fn draw_nonzero_linear_coefficient(
    rng: &mut DeterministicRng,
    mode: LinearEquationMode,
) -> RationalCoefficient {
    let domain = match mode {
        LinearEquationMode::IntegerSolution => linear_integer_domain(),
        LinearEquationMode::RationalSolution => linear_rational_domain(),
    };
    domain[rng.next_bounded(domain.len() as u64) as usize]
}

fn linear_integer_domain_with_zero() -> &'static [RationalCoefficient] {
    static VALUES: OnceLock<Vec<RationalCoefficient>> = OnceLock::new();
    VALUES.get_or_init(|| {
        (-15_i64..=15)
            .map(|value| RationalCoefficient::new(value, 1).expect("integer coefficient"))
            .collect()
    })
}

fn linear_rational_domain_with_zero() -> &'static [RationalCoefficient] {
    static VALUES: OnceLock<Vec<RationalCoefficient>> = OnceLock::new();
    VALUES.get_or_init(|| {
        let mut values = linear_rational_domain().to_vec();
        values.push(RationalCoefficient::zero());
        values.sort_unstable_by_key(|value| (value.denominator, value.numerator));
        values.dedup();
        values
    })
}

fn linear_integer_domain() -> &'static [RationalCoefficient] {
    static VALUES: OnceLock<Vec<RationalCoefficient>> = OnceLock::new();
    VALUES.get_or_init(|| {
        (-15_i64..=15)
            .filter(|value| *value != 0)
            .map(|value| RationalCoefficient::new(value, 1).expect("nonzero integer coefficient"))
            .collect()
    })
}

fn linear_rational_domain() -> &'static [RationalCoefficient] {
    static VALUES: OnceLock<Vec<RationalCoefficient>> = OnceLock::new();
    VALUES.get_or_init(|| {
        let mut values = linear_integer_domain().to_vec();
        for denominator in 2_i64..=9 {
            for numerator_abs in 1_i64..=(10 - denominator) {
                let Some(positive) = RationalCoefficient::new(numerator_abs, denominator) else {
                    continue;
                };
                if positive.denominator == 1 {
                    continue;
                }
                values.push(positive);
                values.push(
                    RationalCoefficient::new(-numerator_abs, denominator)
                        .expect("negative fraction"),
                );
            }
        }
        values.sort_unstable();
        values.dedup();
        values
    })
}

fn all_structures_input_interface() -> AnswerInputInterface {
    AnswerInputInterface::StructuredMath {
        allowed_structures: vec![
            EditorStructure::Fraction,
            EditorStructure::MixedFraction,
            EditorStructure::Decimal,
            EditorStructure::Root,
            EditorStructure::Negative,
            EditorStructure::PlusMinus,
            EditorStructure::Tuple,
        ],
    }
}

fn rational_answer(value: RationalCoefficient) -> AnswerNode {
    if value.denominator == 1 {
        AnswerNode::Integer(value.numerator)
    } else {
        AnswerNode::Fraction {
            numerator: Box::new(AnswerNode::Integer(value.numerator)),
            denominator: Box::new(AnswerNode::Integer(value.denominator)),
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn linear_equation_problem(
    id: u32,
    numeric_theme_id: u32,
    mode: LinearEquationMode,
    a: RationalCoefficient,
    b: RationalCoefficient,
    c: RationalCoefficient,
    d: RationalCoefficient,
    left_negative_constant_as_subtraction: bool,
    right_negative_constant_as_subtraction: bool,
    solution: RationalCoefficient,
    weights: &OperationWeights,
) -> Problem {
    let canonical_answer = rational_answer(solution);
    let solution_graph = linear_equation_graph(a, b, c, d, &canonical_answer);
    let result = calculate_graph_effort(&solution_graph, weights);
    let answer_schema = match mode {
        LinearEquationMode::IntegerSolution => AnswerSchema::Integer { min: -15, max: 15 },
        LinearEquationMode::RationalSolution => AnswerSchema::Rational {
            max_abs_numerator: 20,
            max_denominator: 12,
            require_reduced_fraction_form: true,
        },
    };
    Problem {
        schema_version: SCHEMA_VERSION,
        id,
        numeric_theme_id,
        prompt: ProblemPrompt::LinearEquation {
            a,
            b,
            c,
            d,
            left_negative_constant_as_subtraction,
            right_negative_constant_as_subtraction,
        },
        input_interface: all_structures_input_interface(),
        answer_schema,
        canonical_answer,
        solution_graph,
        operation_vector: result.operation_vector,
        effort: result.value,
    }
}

fn addition_problem(id: u32, left: u8, right: u8, weights: &OperationWeights) -> Problem {
    debug_assert!((MIN_OPERAND..=MAX_OPERAND).contains(&left));
    debug_assert!((MIN_OPERAND..=MAX_OPERAND).contains(&right));
    let answer = left + right;
    let solution_graph = one_digit_addition_graph(left, right);
    let result = calculate_graph_effort(&solution_graph, weights);
    Problem {
        schema_version: SCHEMA_VERSION,
        id,
        numeric_theme_id: crate::model::THEME_ID_ONE_DIGIT_ADDITION,
        prompt: ProblemPrompt::Addition { left, right },
        input_interface: AnswerInputInterface::SimpleNumeric {
            allow_decimal: false,
            allow_negative: false,
        },
        answer_schema: AnswerSchema::Integer {
            min: i64::from(MIN_ANSWER),
            max: i64::from(MAX_ANSWER),
        },
        canonical_answer: AnswerNode::Integer(i64::from(answer)),
        solution_graph,
        operation_vector: result.operation_vector,
        effort: result.value,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    struct ConstantGenerator;

    impl ProblemGenerator for ConstantGenerator {
        fn registration(&self) -> &'static ThemeRegistration {
            &crate::registry::ONE_DIGIT_ADDITION_REGISTRATION
        }

        fn draw_candidate(
            &self,
            _rng: &mut DeterministicRng,
            ordinal: u32,
            weights: &OperationWeights,
        ) -> Option<Problem> {
            Some(addition_problem(ordinal, 1, 1, weights))
        }
    }

    #[test]
    fn insufficient_diversity_regenerates_until_attempt_limit() {
        let identity = ProblemSetIdentity::new(
            crate::model::THEME_ID_ONE_DIGIT_ADDITION,
            crate::model::GENERATOR_REVISION_ONE_DIGIT_ADDITION,
            "Ab3Z",
            crate::identity::DEFAULT_DIFFICULTY,
        )
        .unwrap();
        let config = GenerationConfig::default().with_max_attempts(100);
        let clock = StepClock::new(Duration::ZERO, Duration::ZERO);
        let error = generate_with_generator(
            &identity,
            &crate::registry::ONE_DIGIT_ADDITION_REGISTRATION,
            &ConstantGenerator,
            &config,
            &clock,
        )
        .unwrap_err();
        assert_eq!(
            error,
            GenerationError::AttemptLimit {
                attempts: 100,
                max_attempts: 100
            }
        );
    }

    #[test]
    fn linear_answer_support_matches_requested_domain() {
        let integer = linear_solution_domain(LinearEquationMode::IntegerSolution);
        assert_eq!(integer.len(), 31);
        assert_eq!(integer.first().unwrap().numerator, -15);
        assert_eq!(integer.last().unwrap().numerator, 15);
        assert!(integer.iter().all(|value| value.denominator == 1));

        let rational = linear_solution_domain(LinearEquationMode::RationalSolution);
        let mut unique = rational.to_vec();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), rational.len());
        assert_eq!(rational.iter().filter(|value| value.is_zero()).count(), 1);
        for value in rational {
            match value.denominator {
                1 => assert!(value.numerator.abs() <= 15),
                2 => assert!(value.numerator.unsigned_abs() <= 20),
                3..=12 => assert!(value.numerator.unsigned_abs() <= 15),
                other => panic!("unexpected reduced denominator {other}"),
            }
        }
    }

    #[test]
    fn every_linear_answer_support_value_can_generate_an_equation() {
        for generator in [&LINEAR_EQUATION_1_GENERATOR, &LINEAR_EQUATION_2_GENERATOR] {
            let mut rng = DeterministicRng::from_seed("AllAns7");
            let weights = resolved_weights(generator.registration);
            for answer in linear_answer_domain(generator.mode) {
                let generated = (1_u32..=2_000).find_map(|ordinal| {
                    generator.draw_candidate_for_answer(&mut rng, ordinal, &weights, answer)
                });
                let problem = generated.unwrap_or_else(|| {
                    panic!("could not generate an equation for answer {answer:?}")
                });
                assert_eq!(&problem.canonical_answer, answer);
            }
        }
    }

    #[test]
    fn linear_constant_support_contains_zero_exactly_once() {
        let integer = linear_integer_domain_with_zero();
        assert_eq!(integer.len(), 31);
        assert_eq!(integer.iter().filter(|value| value.is_zero()).count(), 1);
        assert_eq!(integer.first().unwrap().numerator, -15);
        assert_eq!(integer.last().unwrap().numerator, 15);

        let rational = linear_rational_domain_with_zero();
        assert_eq!(rational.len(), linear_rational_domain().len() + 1);
        assert_eq!(rational.iter().filter(|value| value.is_zero()).count(), 1);
    }

    proptest! {
        #[test]
        fn diversity_regeneration_is_bounded_for_every_attempt_budget(extra in 0_u64..100) {
            let identity = ProblemSetIdentity::new(
                crate::model::THEME_ID_ONE_DIGIT_ADDITION,
                crate::model::GENERATOR_REVISION_ONE_DIGIT_ADDITION,
                "Dvrs",
                crate::identity::DEFAULT_DIFFICULTY,
            ).unwrap();
            let max_attempts = 100 + extra;
            let config = GenerationConfig::default().with_max_attempts(max_attempts);
            let clock = StepClock::new(Duration::ZERO, Duration::ZERO);
            let error = generate_with_generator(
                &identity,
                &crate::registry::ONE_DIGIT_ADDITION_REGISTRATION,
                &ConstantGenerator,
                &config,
                &clock,
            ).unwrap_err();
            prop_assert_eq!(error, GenerationError::AttemptLimit {
                attempts: max_attempts,
                max_attempts,
            });
        }
    }
}
