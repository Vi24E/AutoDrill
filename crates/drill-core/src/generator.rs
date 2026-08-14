use std::cell::Cell;
use std::collections::HashSet;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use crate::answer::{AnswerBinaryOperator, AnswerNode};
use crate::effort::{
    arithmetic_expression_graph, calculate_graph_effort, linear_equation_graph,
    one_digit_addition_graph, one_digit_subtraction_graph, quadratic_factoring_graph,
    quadratic_formula_graph, quadratic_square_graph, simultaneous_equation_graph,
    two_digit_addition_graph, OperationWeights,
};
use crate::error::GenerationError;
use crate::identity::{validate_seed, ProblemSetIdentity};
use crate::model::{
    AnswerInputInterface, AnswerSchema, ArithmeticExpression, ArithmeticOperator, EditorStructure,
    GenerateProblemRequest, GenerateWorksheetRequest, LayoutMetadata, LiarStatement, Problem,
    ProblemPrompt, QuadraticEquationForm, RationalCoefficient, Worksheet, MAX_ANSWER, MAX_OPERAND,
    MIN_ANSWER, MIN_OPERAND, SCHEMA_VERSION,
};
use crate::registry::{active_registration, registration, resolved_weights, ThemeRegistration};
use crate::rng::DeterministicRng;
use crate::themes::{division_table, liar_puzzle as liar_puzzle_theme, multiplication_table};

pub const DEFAULT_TIMEOUT: Duration = Duration::from_millis(500);
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

    /// Finite expression domains can ask the common sampler to build a small
    /// unique pool instead of an 8n pool with duplicate expressions.
    fn finite_distinct_candidate_count(&self) -> Option<usize> {
        None
    }

    fn draw_finite_candidate(
        &self,
        _index: usize,
        _ordinal: u32,
        _weights: &OperationWeights,
    ) -> Option<Problem> {
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
enum ArithmeticThemeMode {
    OneDigitSubtraction,
    TwoDigitAddition,
    MultiplicationTable,
    SignedArithmetic1,
    SignedArithmetic2,
    FractionAddition,
    FractionSubtraction,
    FractionMultiplication,
    FractionDivision,
    Division1,
    DecimalAddSubtract,
    DecimalMultiplyDivide,
}

#[derive(Debug)]
pub struct ArithmeticThemeGenerator {
    registration: &'static ThemeRegistration,
    mode: ArithmeticThemeMode,
}

impl ProblemGenerator for ArithmeticThemeGenerator {
    fn registration(&self) -> &'static ThemeRegistration {
        self.registration
    }

    fn finite_distinct_candidate_count(&self) -> Option<usize> {
        match self.mode {
            ArithmeticThemeMode::FractionAddition
            | ArithmeticThemeMode::FractionSubtraction
            | ArithmeticThemeMode::FractionMultiplication
            | ArithmeticThemeMode::FractionDivision => {
                Some(fraction_arithmetic_domain(self.mode).len())
            }
            _ => None,
        }
    }

    fn draw_finite_candidate(
        &self,
        index: usize,
        ordinal: u32,
        weights: &OperationWeights,
    ) -> Option<Problem> {
        let &(left, right, result) = fraction_arithmetic_domain(self.mode).get(index)?;
        Some(fraction_theme_problem(
            self.registration.numeric_theme_id,
            self.mode,
            ordinal,
            weights,
            left,
            right,
            result,
        ))
    }

    fn draw_candidate(
        &self,
        rng: &mut DeterministicRng,
        ordinal: u32,
        weights: &OperationWeights,
    ) -> Option<Problem> {
        arithmetic_theme_problem(
            self.registration.numeric_theme_id,
            self.mode,
            rng,
            ordinal,
            weights,
        )
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

#[derive(Debug)]
pub struct SimultaneousEquationGenerator {
    registration: &'static ThemeRegistration,
}

impl ProblemGenerator for SimultaneousEquationGenerator {
    fn registration(&self) -> &'static ThemeRegistration {
        self.registration
    }

    fn draw_candidate(
        &self,
        rng: &mut DeterministicRng,
        ordinal: u32,
        weights: &OperationWeights,
    ) -> Option<Problem> {
        simultaneous_equation_problem(self.registration.numeric_theme_id, rng, ordinal, weights)
    }
}

#[derive(Debug)]
pub struct LiarPuzzleGenerator {
    registration: &'static ThemeRegistration,
}

impl ProblemGenerator for LiarPuzzleGenerator {
    fn registration(&self) -> &'static ThemeRegistration {
        self.registration
    }

    fn draw_candidate(
        &self,
        rng: &mut DeterministicRng,
        ordinal: u32,
        weights: &OperationWeights,
    ) -> Option<Problem> {
        liar_puzzle_problem(self.registration.numeric_theme_id, rng, ordinal, weights)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum QuadraticEquationMode {
    SquareReduction,
    Factoring,
    Formula,
}

#[derive(Debug)]
pub struct QuadraticEquationGenerator {
    registration: &'static ThemeRegistration,
    mode: QuadraticEquationMode,
}

impl ProblemGenerator for QuadraticEquationGenerator {
    fn registration(&self) -> &'static ThemeRegistration {
        self.registration
    }

    fn answer_domain(&self) -> Option<&'static [AnswerNode]> {
        (self.mode == QuadraticEquationMode::SquareReduction).then(quadratic_one_answer_domain)
    }

    fn draw_candidate(
        &self,
        rng: &mut DeterministicRng,
        ordinal: u32,
        weights: &OperationWeights,
    ) -> Option<Problem> {
        if self.mode == QuadraticEquationMode::SquareReduction {
            let domain = quadratic_one_answer_domain();
            let answer = &domain[rng.next_bounded(domain.len() as u64) as usize];
            return self.draw_candidate_for_answer(rng, ordinal, weights, answer);
        }
        quadratic_equation_problem(
            self.registration.numeric_theme_id,
            self.mode,
            rng,
            ordinal,
            weights,
            None,
        )
    }

    fn draw_candidate_for_answer(
        &self,
        rng: &mut DeterministicRng,
        ordinal: u32,
        weights: &OperationWeights,
        answer: &AnswerNode,
    ) -> Option<Problem> {
        quadratic_equation_problem(
            self.registration.numeric_theme_id,
            self.mode,
            rng,
            ordinal,
            weights,
            Some(answer),
        )
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

static ONE_DIGIT_SUBTRACTION_GENERATOR: ArithmeticThemeGenerator = ArithmeticThemeGenerator {
    registration: &crate::registry::ONE_DIGIT_SUBTRACTION_REGISTRATION,
    mode: ArithmeticThemeMode::OneDigitSubtraction,
};
static TWO_DIGIT_ADDITION_GENERATOR: ArithmeticThemeGenerator = ArithmeticThemeGenerator {
    registration: &crate::registry::TWO_DIGIT_ADDITION_REGISTRATION,
    mode: ArithmeticThemeMode::TwoDigitAddition,
};
static MULTIPLICATION_TABLE_GENERATOR: ArithmeticThemeGenerator = ArithmeticThemeGenerator {
    registration: &crate::registry::MULTIPLICATION_TABLE_REGISTRATION,
    mode: ArithmeticThemeMode::MultiplicationTable,
};
static SIGNED_ARITHMETIC_1_GENERATOR: ArithmeticThemeGenerator = ArithmeticThemeGenerator {
    registration: &crate::registry::SIGNED_ARITHMETIC_1_REGISTRATION,
    mode: ArithmeticThemeMode::SignedArithmetic1,
};
static SIGNED_ARITHMETIC_2_GENERATOR: ArithmeticThemeGenerator = ArithmeticThemeGenerator {
    registration: &crate::registry::SIGNED_ARITHMETIC_2_REGISTRATION,
    mode: ArithmeticThemeMode::SignedArithmetic2,
};
static FRACTION_ADDITION_GENERATOR: ArithmeticThemeGenerator = ArithmeticThemeGenerator {
    registration: &crate::registry::FRACTION_ADDITION_REGISTRATION,
    mode: ArithmeticThemeMode::FractionAddition,
};
static FRACTION_SUBTRACTION_GENERATOR: ArithmeticThemeGenerator = ArithmeticThemeGenerator {
    registration: &crate::registry::FRACTION_SUBTRACTION_REGISTRATION,
    mode: ArithmeticThemeMode::FractionSubtraction,
};
static FRACTION_MULTIPLICATION_GENERATOR: ArithmeticThemeGenerator = ArithmeticThemeGenerator {
    registration: &crate::registry::FRACTION_MULTIPLICATION_REGISTRATION,
    mode: ArithmeticThemeMode::FractionMultiplication,
};
static FRACTION_DIVISION_GENERATOR: ArithmeticThemeGenerator = ArithmeticThemeGenerator {
    registration: &crate::registry::FRACTION_DIVISION_REGISTRATION,
    mode: ArithmeticThemeMode::FractionDivision,
};
static DIVISION_1_GENERATOR: ArithmeticThemeGenerator = ArithmeticThemeGenerator {
    registration: &crate::registry::DIVISION_1_REGISTRATION,
    mode: ArithmeticThemeMode::Division1,
};
static DECIMAL_ADD_SUBTRACT_GENERATOR: ArithmeticThemeGenerator = ArithmeticThemeGenerator {
    registration: &crate::registry::DECIMAL_ADD_SUBTRACT_REGISTRATION,
    mode: ArithmeticThemeMode::DecimalAddSubtract,
};
static DECIMAL_MULTIPLY_DIVIDE_GENERATOR: ArithmeticThemeGenerator = ArithmeticThemeGenerator {
    registration: &crate::registry::DECIMAL_MULTIPLY_DIVIDE_REGISTRATION,
    mode: ArithmeticThemeMode::DecimalMultiplyDivide,
};
static SIMULTANEOUS_EQUATION_1_GENERATOR: SimultaneousEquationGenerator =
    SimultaneousEquationGenerator {
        registration: &crate::registry::SIMULTANEOUS_EQUATION_1_REGISTRATION,
    };
static LIAR_PUZZLE_GENERATOR: LiarPuzzleGenerator = LiarPuzzleGenerator {
    registration: &crate::registry::LIAR_PUZZLE_REGISTRATION,
};
static QUADRATIC_EQUATION_1_GENERATOR: QuadraticEquationGenerator = QuadraticEquationGenerator {
    registration: &crate::registry::QUADRATIC_EQUATION_1_REGISTRATION,
    mode: QuadraticEquationMode::SquareReduction,
};
static QUADRATIC_EQUATION_2_GENERATOR: QuadraticEquationGenerator = QuadraticEquationGenerator {
    registration: &crate::registry::QUADRATIC_EQUATION_2_REGISTRATION,
    mode: QuadraticEquationMode::Factoring,
};
static QUADRATIC_EQUATION_3_GENERATOR: QuadraticEquationGenerator = QuadraticEquationGenerator {
    registration: &crate::registry::QUADRATIC_EQUATION_3_REGISTRATION,
    mode: QuadraticEquationMode::Formula,
};

static REGISTERED_GENERATORS: [&dyn ProblemGenerator; 20] = [
    &ONE_DIGIT_ADDITION_GENERATOR,
    &LINEAR_EQUATION_1_GENERATOR,
    &LINEAR_EQUATION_2_GENERATOR,
    &ONE_DIGIT_SUBTRACTION_GENERATOR,
    &TWO_DIGIT_ADDITION_GENERATOR,
    &MULTIPLICATION_TABLE_GENERATOR,
    &SIGNED_ARITHMETIC_1_GENERATOR,
    &SIGNED_ARITHMETIC_2_GENERATOR,
    &FRACTION_ADDITION_GENERATOR,
    &FRACTION_SUBTRACTION_GENERATOR,
    &FRACTION_MULTIPLICATION_GENERATOR,
    &FRACTION_DIVISION_GENERATOR,
    &DIVISION_1_GENERATOR,
    &DECIMAL_ADD_SUBTRACT_GENERATOR,
    &DECIMAL_MULTIPLY_DIVIDE_GENERATOR,
    &QUADRATIC_EQUATION_1_GENERATOR,
    &QUADRATIC_EQUATION_2_GENERATOR,
    &QUADRATIC_EQUATION_3_GENERATOR,
    &SIMULTANEOUS_EQUATION_1_GENERATOR,
    &LIAR_PUZZLE_GENERATOR,
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
            if problem_allowed_by_curriculum(registration, &problem) {
                return Ok(problem);
            }
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
    let finite_distinct_count = generator.finite_distinct_candidate_count();
    let unique_finite_pool = finite_distinct_count.is_some();
    let pool_size = CANDIDATE_POOL_MULTIPLIER * n;
    let required_diversity = DIVERSITY_MULTIPLIER * n;

    let mut pool = if let Some(finite_count) = finite_distinct_count {
        if finite_count as u64 > config.max_attempts {
            return Err(GenerationError::AttemptLimit {
                attempts: config.max_attempts,
                max_attempts: config.max_attempts,
            });
        }
        attempts = attempts.saturating_add(finite_count as u64);
        let mut candidate_pool = Vec::with_capacity(finite_count);
        for index in 0..finite_count {
            let ordinal = u32::try_from(index + 1).unwrap_or(u32::MAX);
            let Some(problem) = generator.draw_finite_candidate(index, ordinal, &weights) else {
                continue;
            };
            if problem_allowed_by_curriculum(registration, &problem) {
                candidate_pool.push(problem);
            }
        }
        let mut distinct = HashSet::with_capacity(candidate_pool.len());
        candidate_pool.retain(|problem| distinct.insert(problem_key(problem)));
        check_timeout(started, clock, config)?;
        candidate_pool
    } else {
        loop {
            let mut candidate_pool = Vec::with_capacity(pool_size);
            let mut distinct = HashSet::with_capacity(pool_size);
            let balance_liar_population =
                registration.numeric_theme_id == crate::model::THEME_ID_LIAR_PUZZLE;
            let liar_population_quota = pool_size / 2;
            let mut liar_three_person_count = 0_usize;
            let mut liar_four_person_count = 0_usize;
            debug_assert!(!balance_liar_population || pool_size.is_multiple_of(2));
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
                    if !problem_allowed_by_curriculum(registration, &problem) {
                        continue;
                    }
                    if fixed_answer
                        .as_ref()
                        .is_some_and(|answer| problem.canonical_answer != *answer)
                    {
                        continue;
                    }
                    if balance_liar_population {
                        match &problem.prompt {
                            ProblemPrompt::LiarPuzzle {
                                people_count: 3, ..
                            } if liar_three_person_count < liar_population_quota => {}
                            ProblemPrompt::LiarPuzzle {
                                people_count: 4, ..
                            } if liar_four_person_count < liar_population_quota => {}
                            _ => continue,
                        }
                    }
                    let key = problem_key(&problem);
                    if unique_finite_pool {
                        if !distinct.insert(key) {
                            continue;
                        }
                    } else {
                        distinct.insert(key);
                    }
                    if balance_liar_population {
                        match &problem.prompt {
                            ProblemPrompt::LiarPuzzle {
                                people_count: 3, ..
                            } => {
                                liar_three_person_count += 1;
                            }
                            ProblemPrompt::LiarPuzzle {
                                people_count: 4, ..
                            } => {
                                liar_four_person_count += 1;
                            }
                            _ => unreachable!("liar-puzzle balance was validated above"),
                        }
                    }
                    candidate_pool.push(problem);
                    break;
                }
            }
            check_timeout(started, clock, config)?;
            if balance_liar_population {
                debug_assert_eq!(liar_three_person_count, liar_population_quota);
                debug_assert_eq!(liar_four_person_count, liar_population_quota);
            }
            if distinct.len() >= required_diversity {
                break candidate_pool;
            }
            // The full pool is discarded. The next loop consumes fresh attempts
            // and fresh deterministic RNG draws.
        }
    };

    let mut selected = if identity.difficulty.value() == 4 {
        // Random difficulty is deliberately separate from effort-ranked
        // selection. Draw candidate slots uniformly from the original pool,
        // preserving the generator's source distribution. Only an exact prompt
        // already selected for this worksheet is rejected; there is no effort
        // sort, rank statistic, trimming, or pre-deduplication that could alter
        // the source probabilities.
        debug_assert!(pool.len() >= n);
        let mut selected = Vec::with_capacity(n);
        let mut selected_expressions = HashSet::with_capacity(n);
        while selected.len() < n {
            consume_attempt(started, clock, config, &mut attempts)?;
            let selected_index = rng.next_bounded(pool.len() as u64) as usize;
            let candidate = pool.swap_remove(selected_index);
            if selected_expressions.insert(problem_key(&candidate)) {
                selected.push(candidate);
            }
        }
        selected
    } else {
        pool.sort_by(|left, right| {
            left.effort
                .total_cmp(&right.effort)
                .then_with(|| problem_key(left).cmp(&problem_key(right)))
                .then_with(|| left.id.cmp(&right.id))
        });

        let bootstrap_count = n + EFFORT_TRIM_PER_SIDE * 2;
        debug_assert!(pool.len() >= bootstrap_count);
        let mut selected = Vec::with_capacity(bootstrap_count);
        let mut selected_expressions = HashSet::with_capacity(bootstrap_count);
        let order_statistic_index = match identity.difficulty.value() {
            1 => 0, // old "very easy"
            2 => 2, // old "normal"
            3 => 4, // old "very hard"
            _ => unreachable!("random difficulty is handled above"),
        };
        while selected.len() < bootstrap_count {
            consume_attempt(started, clock, config, &mut attempts)?;
            let mut draws = [0_usize; DIFFICULTY_BOOTSTRAP_DRAWS];
            for draw in &mut draws {
                *draw = rng.next_bounded(pool.len() as u64) as usize + 1;
            }
            draws.sort_unstable();
            let selected_index = draws[order_statistic_index] - 1;
            if !unique_finite_pool {
                let key = problem_key(&pool[selected_index]);
                if !selected_expressions.insert(key) {
                    continue;
                }
            }
            selected.push(pool.remove(selected_index));
        }

        // Trim the two easiest and two hardest bootstrap selections. This keeps
        // the rank bias from the three pedagogical difficulty levels while
        // suppressing accidental effort outliers. Random mode never enters this
        // branch because trimming by effort would itself create bias.
        selected.sort_by(|left, right| {
            left.effort
                .total_cmp(&right.effort)
                .then_with(|| problem_key(left).cmp(&problem_key(right)))
                .then_with(|| left.id.cmp(&right.id))
        });
        selected
            .into_iter()
            .skip(EFFORT_TRIM_PER_SIDE)
            .take(n)
            .collect()
    };

    if identity.difficulty.value() <= 2 {
        // Easy and normal worksheets should progress from lower to higher effort
        // so the sheet itself has a pedagogical difficulty ramp. Keep the same
        // deterministic tie-breakers used during candidate selection.
        selected.sort_by(|left, right| {
            left.effort
                .total_cmp(&right.effort)
                .then_with(|| problem_key(left).cmp(&problem_key(right)))
                .then_with(|| left.id.cmp(&right.id))
        });
    } else {
        // Hard and random worksheets retain the existing shuffled presentation.
        for upper in (1..selected.len()).rev() {
            let swap_with = rng.next_bounded((upper + 1) as u64) as usize;
            selected.swap(upper, swap_with);
        }
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

fn problem_allowed_by_curriculum(registration: &ThemeRegistration, problem: &Problem) -> bool {
    if !registration
        .curriculum_path
        .iter()
        .any(|segment| segment.starts_with("小学"))
    {
        return true;
    }
    prompt_has_no_negative_values(&problem.prompt)
        && answer_has_no_negative_values(&problem.canonical_answer)
        && input_interface_has_no_negative_capability(&problem.input_interface)
}

fn prompt_has_no_negative_values(prompt: &ProblemPrompt) -> bool {
    match prompt {
        ProblemPrompt::Addition { .. } => true,
        ProblemPrompt::Arithmetic { expression } => expression_has_no_negative_values(expression),
        ProblemPrompt::LinearEquation {
            a,
            b,
            c,
            d,
            left_negative_constant_as_subtraction,
            right_negative_constant_as_subtraction,
        } => {
            [a, b, c, d].iter().all(|value| value.numerator >= 0)
                && !left_negative_constant_as_subtraction
                && !right_negative_constant_as_subtraction
        }
        ProblemPrompt::QuadraticEquation { a, b, c, .. } => {
            [a, b, c].iter().all(|value| value.numerator >= 0)
        }
        ProblemPrompt::SimultaneousEquation { a, b, c, d, e, f } => {
            [a, b, c, d, e, f].iter().all(|value| **value >= 0)
        }
        ProblemPrompt::LiarPuzzle { .. } => true,
    }
}

fn expression_has_no_negative_values(expression: &ArithmeticExpression) -> bool {
    match expression {
        ArithmeticExpression::Integer { value } => *value >= 0,
        ArithmeticExpression::Rational { value } => value.numerator >= 0,
        ArithmeticExpression::ExactDecimal { coefficient, .. } => *coefficient >= 0,
        ArithmeticExpression::Binary { left, right, .. } => {
            expression_has_no_negative_values(left) && expression_has_no_negative_values(right)
        }
    }
}

fn answer_has_no_negative_values(answer: &AnswerNode) -> bool {
    match answer {
        AnswerNode::Empty => true,
        AnswerNode::Integer(value) => *value >= 0,
        AnswerNode::ExactDecimal { coefficient, .. } => *coefficient >= 0,
        AnswerNode::NanError(raw) => !raw.contains('-') && !raw.contains('−'),
        AnswerNode::Fraction {
            numerator,
            denominator,
        } => answer_has_no_negative_values(numerator) && answer_has_no_negative_values(denominator),
        AnswerNode::MixedFraction {
            whole,
            numerator,
            denominator,
        } => {
            answer_has_no_negative_values(whole)
                && answer_has_no_negative_values(numerator)
                && answer_has_no_negative_values(denominator)
        }
        AnswerNode::Root { radicand, index } => {
            answer_has_no_negative_values(radicand)
                && index.as_deref().is_none_or(answer_has_no_negative_values)
        }
        AnswerNode::Negative(_) | AnswerNode::PlusMinus(_) => false,
        AnswerNode::Binary { left, right, .. } => {
            answer_has_no_negative_values(left) && answer_has_no_negative_values(right)
        }
        AnswerNode::Tuple(values) => values.iter().all(answer_has_no_negative_values),
        AnswerNode::Variable(_) => true,
    }
}

fn input_interface_has_no_negative_capability(input: &AnswerInputInterface) -> bool {
    match input {
        AnswerInputInterface::SimpleNumeric { allow_negative, .. } => !allow_negative,
        AnswerInputInterface::StructuredMath { allowed_structures } => {
            !allowed_structures.contains(&EditorStructure::Negative)
                && !allowed_structures.contains(&EditorStructure::PlusMinus)
        }
    }
}

fn canonicalize_commutative_expression(expression: &ArithmeticExpression) -> ArithmeticExpression {
    match expression {
        ArithmeticExpression::Integer { .. }
        | ArithmeticExpression::Rational { .. }
        | ArithmeticExpression::ExactDecimal { .. } => expression.clone(),
        ArithmeticExpression::Binary {
            operator,
            left,
            right,
        } => {
            let mut left = canonicalize_commutative_expression(left);
            let mut right = canonicalize_commutative_expression(right);
            if matches!(
                operator,
                ArithmeticOperator::Add | ArithmeticOperator::Multiply
            ) && right < left
            {
                std::mem::swap(&mut left, &mut right);
            }
            ArithmeticExpression::Binary {
                operator: *operator,
                left: Box::new(left),
                right: Box::new(right),
            }
        }
    }
}

fn problem_key(problem: &Problem) -> ProblemPrompt {
    match &problem.prompt {
        // One-digit addition and the multiplication table intentionally keep
        // ordered variants because their total populations are small.
        ProblemPrompt::Addition { left, right } => ProblemPrompt::Addition {
            left: *left,
            right: *right,
        },
        ProblemPrompt::Arithmetic { expression } => ProblemPrompt::Arithmetic {
            expression: if problem.numeric_theme_id == crate::model::THEME_ID_MULTIPLICATION_TABLE {
                expression.clone()
            } else {
                canonicalize_commutative_expression(expression)
            },
        },
        ProblemPrompt::LinearEquation { a, b, c, d, .. } => ProblemPrompt::LinearEquation {
            a: *a,
            b: *b,
            c: *c,
            d: *d,
            left_negative_constant_as_subtraction: false,
            right_negative_constant_as_subtraction: false,
        },
        ProblemPrompt::QuadraticEquation { form, a, b, c } => ProblemPrompt::QuadraticEquation {
            form: *form,
            a: *a,
            b: *b,
            c: *c,
        },
        ProblemPrompt::SimultaneousEquation { a, b, c, d, e, f } => {
            ProblemPrompt::SimultaneousEquation {
                a: *a,
                b: *b,
                c: *c,
                d: *d,
                e: *e,
                f: *f,
            }
        }
        ProblemPrompt::LiarPuzzle {
            people_count,
            statements,
        } => ProblemPrompt::LiarPuzzle {
            people_count: *people_count,
            statements: statements.clone(),
        },
    }
}

fn liar_statement_truth(statement: &LiarStatement, mask: u32) -> bool {
    let is_liar = |person: u8| ((mask >> u32::from(person - 1)) & 1) == 1;
    match *statement {
        LiarStatement::SaysLiar { person } => is_liar(person),
        LiarStatement::SaysNotLiar { person } => !is_liar(person),
        LiarStatement::ExactlyOneLiar { first, second } => is_liar(first) ^ is_liar(second),
        LiarStatement::ExactLiarCount { count } => mask.count_ones() == u32::from(count),
        LiarStatement::BothLiar { first, second } => is_liar(first) && is_liar(second),
        LiarStatement::BothNotLiar { first, second } => !is_liar(first) && !is_liar(second),
        LiarStatement::Implication {
            antecedent_person,
            antecedent_is_liar,
            consequent_person,
            consequent_is_liar,
        } => {
            let antecedent = is_liar(antecedent_person) == antecedent_is_liar;
            let consequent = is_liar(consequent_person) == consequent_is_liar;
            !antecedent || consequent
        }
    }
}

fn liar_puzzle_solutions(people_count: u8, statements: &[LiarStatement]) -> Vec<u32> {
    let mut solutions = Vec::new();
    for mask in 0_u32..(1_u32 << people_count) {
        let valid = statements.iter().enumerate().all(|(speaker, statement)| {
            let speaker_is_liar = ((mask >> speaker) & 1) == 1;
            liar_statement_truth(statement, mask) == !speaker_is_liar
        });
        if valid {
            solutions.push(mask);
        }
    }
    solutions
}

fn draw_other_person(rng: &mut DeterministicRng, people_count: u8, speaker: u8) -> u8 {
    let offset = 1 + rng.next_bounded(u64::from(people_count - 1)) as u8;
    ((speaker - 1 + offset) % people_count) + 1
}

fn draw_two_other_people(rng: &mut DeterministicRng, people_count: u8, speaker: u8) -> (u8, u8) {
    let first = draw_other_person(rng, people_count, speaker);
    let mut second = draw_other_person(rng, people_count, speaker);
    while second == first {
        second = draw_other_person(rng, people_count, speaker);
    }
    if first < second {
        (first, second)
    } else {
        (second, first)
    }
}

fn liar_puzzle_problem(
    numeric_theme_id: u32,
    rng: &mut DeterministicRng,
    id: u32,
    weights: &OperationWeights,
) -> Option<Problem> {
    let people_count = 3 + rng.next_bounded(2) as u8;
    let mut statements = Vec::with_capacity(usize::from(people_count));
    for speaker in 1..=people_count {
        let statement = match rng.next_bounded(6) {
            0 => LiarStatement::SaysLiar {
                person: draw_other_person(rng, people_count, speaker),
            },
            1 => LiarStatement::SaysNotLiar {
                person: draw_other_person(rng, people_count, speaker),
            },
            2 => {
                let (first, second) = draw_two_other_people(rng, people_count, speaker);
                LiarStatement::ExactlyOneLiar { first, second }
            }
            3 => LiarStatement::ExactLiarCount {
                count: 1 + rng.next_bounded(u64::from(people_count - 1)) as u8,
            },
            4 => {
                let (first, second) = draw_two_other_people(rng, people_count, speaker);
                LiarStatement::BothLiar { first, second }
            }
            _ => {
                let (first, second) = draw_two_other_people(rng, people_count, speaker);
                LiarStatement::BothNotLiar { first, second }
            }
        };
        statements.push(statement);
    }

    let solutions = liar_puzzle_solutions(people_count, &statements);
    if solutions.len() != 1 {
        return None;
    }
    let solution = solutions[0];
    let liar_count = solution.count_ones();
    if liar_count == 0 || liar_count == u32::from(people_count) {
        return None;
    }
    let liars = (1..=people_count)
        .filter(|person| ((solution >> u32::from(*person - 1)) & 1) == 1)
        .map(|person| AnswerNode::Integer(i64::from(person)))
        .collect::<Vec<_>>();
    let canonical_answer = AnswerNode::Tuple(liars);

    // SAT-length effort is an explicit theme exception and lives in the
    // liar-puzzle theme module rather than the reusable arithmetic model.
    let formula_length = statements
        .iter()
        .map(|statement| liar_puzzle_theme::statement_effort(statement, people_count))
        .sum::<u32>();
    let solution_graph = liar_puzzle_theme::solution_graph(&statements, people_count);
    let effort = calculate_graph_effort(&solution_graph, weights);
    debug_assert_eq!(effort.value, f64::from(formula_length));
    Some(Problem {
        schema_version: SCHEMA_VERSION,
        id,
        numeric_theme_id,
        prompt: ProblemPrompt::LiarPuzzle {
            people_count,
            statements,
        },
        input_interface: AnswerInputInterface::StructuredMath {
            allowed_structures: vec![EditorStructure::Tuple],
        },
        answer_schema: AnswerSchema::Algebraic,
        canonical_answer,
        solution_graph,
        operation_vector: effort.operation_vector,
        effort: effort.value,
    })
}

fn quadratic_one_answer_domain() -> &'static [AnswerNode] {
    static DOMAIN: OnceLock<Vec<AnswerNode>> = OnceLock::new();
    DOMAIN.get_or_init(|| {
        let mut answers = Vec::new();
        for root in 1_i64..=16 {
            answers.push(AnswerNode::PlusMinus(Box::new(AnswerNode::Integer(root))));
        }
        for radicand in 2_i64..=30 {
            if exact_square_root_i64(radicand).is_some() {
                continue;
            }
            answers.push(AnswerNode::PlusMinus(Box::new(AnswerNode::Root {
                radicand: Box::new(AnswerNode::Integer(radicand)),
                index: None,
            })));
        }
        answers
    })
}

fn exact_square_root_i64(value: i64) -> Option<i64> {
    if value < 0 {
        return None;
    }
    (0_i64..=value).find(|root| root.checked_mul(*root) == Some(value))
}

fn quadratic_one_square_value(answer: &AnswerNode) -> Option<i64> {
    let AnswerNode::PlusMinus(value) = answer else {
        return None;
    };
    match value.as_ref() {
        AnswerNode::Integer(root @ 1..=16) => root.checked_mul(*root),
        AnswerNode::Root {
            radicand,
            index: None,
        } => {
            let AnswerNode::Integer(value @ 2..=30) = radicand.as_ref() else {
                return None;
            };
            exact_square_root_i64(*value).is_none().then_some(*value)
        }
        _ => None,
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

fn linear_fraction_domain() -> &'static [RationalCoefficient] {
    static VALUES: OnceLock<Vec<RationalCoefficient>> = OnceLock::new();
    VALUES.get_or_init(|| {
        let mut values = Vec::new();
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

fn fraction_arithmetic_operand_domain() -> &'static [RationalCoefficient] {
    static VALUES: OnceLock<Vec<RationalCoefficient>> = OnceLock::new();
    VALUES.get_or_init(|| {
        let mut values = Vec::new();
        // Keep elementary fraction operands compact by bounding the sum of the
        // reduced numerator and denominator. The previous <= 10 bound left only
        // 22 values and made the finite subtraction/multiplication populations
        // too small; <= 15 gives 57 distinct non-integer positive fractions.
        for denominator in 2_i64..=14 {
            for numerator in 1_i64..=(15 - denominator) {
                let Some(value) = RationalCoefficient::new(numerator, denominator) else {
                    continue;
                };
                if value.denominator == 1 {
                    continue;
                }
                values.push(value);
            }
        }
        values.sort_unstable();
        values.dedup();
        values
    })
}

fn fraction_arithmetic_domain(
    mode: ArithmeticThemeMode,
) -> &'static [(
    RationalCoefficient,
    RationalCoefficient,
    RationalCoefficient,
)] {
    type Triple = (
        RationalCoefficient,
        RationalCoefficient,
        RationalCoefficient,
    );
    static ADDITION: OnceLock<Vec<Triple>> = OnceLock::new();
    static SUBTRACTION: OnceLock<Vec<Triple>> = OnceLock::new();
    static MULTIPLICATION: OnceLock<Vec<Triple>> = OnceLock::new();
    static DIVISION: OnceLock<Vec<Triple>> = OnceLock::new();

    fn build(mode: ArithmeticThemeMode, operator: ArithmeticOperator) -> Vec<Triple> {
        let operand_domain = fraction_arithmetic_operand_domain();
        let mut triples = Vec::new();
        for &left in operand_domain {
            for &right in operand_domain {
                // Addition and multiplication are canonicalized by problem_key.
                // Enumerate only one operand order here so the finite domain is
                // already distinct instead of constructing then discarding the
                // commutative mirror of every candidate.
                if matches!(
                    operator,
                    ArithmeticOperator::Add | ArithmeticOperator::Multiply
                ) && right < left
                {
                    continue;
                }
                let result = match operator {
                    ArithmeticOperator::Add => left.checked_add(right),
                    ArithmeticOperator::Subtract => left.subtract(right),
                    ArithmeticOperator::Multiply => left.multiply(right),
                    ArithmeticOperator::Divide => left.divide(right),
                };
                let Some(result) = result else {
                    continue;
                };
                if result.numerator <= 0 {
                    continue;
                }
                // Addition deliberately uses an answer bound independent of the
                // operand domain. Requiring the sum itself to satisfy n + d <= 15
                // leaves only 139 commutative-distinct problems and recreates a
                // strong equal-denominator bias. Subtraction and multiplication
                // stay closed over the compact operand domain.
                let result_allowed = match mode {
                    ArithmeticThemeMode::FractionAddition => {
                        result.denominator > 1 && result.numerator <= 65 && result.denominator <= 72
                    }
                    ArithmeticThemeMode::FractionSubtraction
                    | ArithmeticThemeMode::FractionMultiplication => {
                        operand_domain.contains(&result)
                    }
                    ArithmeticThemeMode::FractionDivision => unreachable!(
                        "fraction division builds its integer-inclusive domain separately"
                    ),
                    _ => unreachable!(),
                };
                if result_allowed {
                    triples.push((left, right, result));
                }
            }
        }
        triples
    }

    match mode {
        ArithmeticThemeMode::FractionAddition => ADDITION.get_or_init(|| {
            build(
                ArithmeticThemeMode::FractionAddition,
                ArithmeticOperator::Add,
            )
        }),
        ArithmeticThemeMode::FractionSubtraction => SUBTRACTION.get_or_init(|| {
            build(
                ArithmeticThemeMode::FractionSubtraction,
                ArithmeticOperator::Subtract,
            )
        }),
        ArithmeticThemeMode::FractionMultiplication => MULTIPLICATION.get_or_init(|| {
            build(
                ArithmeticThemeMode::FractionMultiplication,
                ArithmeticOperator::Multiply,
            )
        }),
        ArithmeticThemeMode::FractionDivision => DIVISION.get_or_init(|| {
            let fraction_domain = fraction_arithmetic_operand_domain();
            let mut operands = fraction_domain.to_vec();
            let integer_domain = (1_i64..=9)
                .map(|value| RationalCoefficient::new(value, 1).unwrap())
                .collect::<Vec<_>>();
            operands.extend(integer_domain.iter().copied());
            operands.sort_unstable();
            operands.dedup();
            let mut triples = Vec::new();
            for &left in &operands {
                for &right in &operands {
                    if left.is_integer() && right.is_integer() {
                        continue;
                    }
                    let Some(result) = left.divide(right) else {
                        continue;
                    };
                    // Division keeps integer operands because fraction/integer and
                    // integer/fraction are both elementary curriculum cases. The
                    // old <= 72 numerator/denominator bound admitted almost every
                    // ordered pair (3,873 candidates at n+d<=15). Close answers
                    // over the compact fraction domain plus one-digit integers
                    // instead; this preserves all three operand archetypes and
                    // integer quotients without letting the finite pool explode.
                    if result.numerator > 0
                        && (fraction_domain.contains(&result) || integer_domain.contains(&result))
                    {
                        triples.push((left, right, result));
                    }
                }
            }
            triples
        }),
        _ => unreachable!(),
    }
}

fn linear_rational_domain() -> &'static [RationalCoefficient] {
    static VALUES: OnceLock<Vec<RationalCoefficient>> = OnceLock::new();
    VALUES.get_or_init(|| {
        let mut values = linear_integer_domain().to_vec();
        values.extend_from_slice(linear_fraction_domain());
        values.sort_unstable();
        values.dedup();
        values
    })
}

fn fraction_theme_problem(
    numeric_theme_id: u32,
    mode: ArithmeticThemeMode,
    id: u32,
    weights: &OperationWeights,
    left: RationalCoefficient,
    right: RationalCoefficient,
    result: RationalCoefficient,
) -> Problem {
    let operator = match mode {
        ArithmeticThemeMode::FractionAddition => ArithmeticOperator::Add,
        ArithmeticThemeMode::FractionSubtraction => ArithmeticOperator::Subtract,
        ArithmeticThemeMode::FractionMultiplication => ArithmeticOperator::Multiply,
        ArithmeticThemeMode::FractionDivision => ArithmeticOperator::Divide,
        _ => unreachable!(),
    };
    let expression = binary_expression(
        operator,
        rational_expression(left),
        rational_expression(right),
    );
    let answer = rational_answer(result);
    let solution_graph = arithmetic_expression_graph(&expression, &answer)
        .expect("accepted fraction-domain expression must have an effort graph");
    let effort = calculate_graph_effort(&solution_graph, weights);
    Problem {
        schema_version: SCHEMA_VERSION,
        id,
        numeric_theme_id,
        prompt: ProblemPrompt::Arithmetic { expression },
        input_interface: fraction_input_interface(),
        answer_schema: match mode {
            // Addition uses an explicit result bound independent of the operand
            // domain so unlike-denominator sums remain well represented.
            ArithmeticThemeMode::FractionAddition => AnswerSchema::Rational {
                max_abs_numerator: 65,
                max_denominator: 72,
                require_reduced_fraction_form: true,
            },
            ArithmeticThemeMode::FractionSubtraction
            | ArithmeticThemeMode::FractionMultiplication
            | ArithmeticThemeMode::FractionDivision => AnswerSchema::Rational {
                max_abs_numerator: 13,
                max_denominator: 14,
                require_reduced_fraction_form: true,
            },
            _ => unreachable!(),
        },
        canonical_answer: answer,
        solution_graph,
        operation_vector: effort.operation_vector,
        effort: effort.value,
    }
}

fn arithmetic_theme_problem(
    numeric_theme_id: u32,
    mode: ArithmeticThemeMode,
    rng: &mut DeterministicRng,
    id: u32,
    weights: &OperationWeights,
) -> Option<Problem> {
    let (expression, answer, solution_graph, input_interface, answer_schema) = match mode {
        ArithmeticThemeMode::OneDigitSubtraction => {
            let b = 1_i64 + rng.next_bounded(9) as i64;
            let c = 1_i64 + rng.next_bounded(9) as i64;
            let a = b + c;
            let expression = binary_expression(
                ArithmeticOperator::Subtract,
                integer_expression(a),
                integer_expression(b),
            );
            let answer = AnswerNode::Integer(c);
            (
                expression,
                answer,
                one_digit_subtraction_graph(a as u8, b as u8),
                simple_integer_input(false),
                AnswerSchema::Integer { min: 1, max: 9 },
            )
        }
        ArithmeticThemeMode::TwoDigitAddition => {
            let a = 10_i64 + rng.next_bounded(90) as i64;
            let b = 10_i64 + rng.next_bounded(90) as i64;
            let c = a + b;
            let expression = binary_expression(
                ArithmeticOperator::Add,
                integer_expression(a),
                integer_expression(b),
            );
            let answer = AnswerNode::Integer(c);
            (
                expression,
                answer,
                two_digit_addition_graph(a as u8, b as u8),
                simple_integer_input(false),
                AnswerSchema::Integer { min: 20, max: 198 },
            )
        }
        ArithmeticThemeMode::MultiplicationTable => {
            let a = 1_i64 + rng.next_bounded(9) as i64;
            let b = 1_i64 + rng.next_bounded(9) as i64;
            let c = a * b;
            let expression = binary_expression(
                ArithmeticOperator::Multiply,
                integer_expression(a),
                integer_expression(b),
            );
            let answer = AnswerNode::Integer(c);
            (
                expression,
                answer,
                multiplication_table::solution_graph(c as u8),
                simple_integer_input(false),
                AnswerSchema::Integer { min: 1, max: 81 },
            )
        }
        ArithmeticThemeMode::Division1 => {
            let divisor = 1_i64 + rng.next_bounded(9) as i64;
            let quotient = 1_i64 + rng.next_bounded(9) as i64;
            let dividend = divisor * quotient;
            let expression = binary_expression(
                ArithmeticOperator::Divide,
                integer_expression(dividend),
                integer_expression(divisor),
            );
            let answer = AnswerNode::Integer(quotient);
            (
                expression,
                answer,
                division_table::solution_graph(dividend as u8),
                simple_integer_input(false),
                AnswerSchema::Integer { min: 1, max: 9 },
            )
        }
        ArithmeticThemeMode::DecimalAddSubtract => {
            // Grade 4: addition/subtraction with decimals. Keep operands at
            // at most three significant digits and at most three decimal places.
            let (mut left_coefficient, mut left_scale) = draw_decimal_operand(rng, 3, 3);
            let (mut right_coefficient, mut right_scale) = draw_decimal_operand(rng, 3, 3);
            let mut left_value = exact_decimal_rational(left_coefficient, left_scale)?;
            let mut right_value = exact_decimal_rational(right_coefficient, right_scale)?;
            let operator = if rng.next_bounded(2) == 0 {
                ArithmeticOperator::Add
            } else {
                ArithmeticOperator::Subtract
            };
            if operator == ArithmeticOperator::Subtract
                && rational_less_than(left_value, right_value)
            {
                std::mem::swap(&mut left_coefficient, &mut right_coefficient);
                std::mem::swap(&mut left_scale, &mut right_scale);
                std::mem::swap(&mut left_value, &mut right_value);
            }
            let result_value = match operator {
                ArithmeticOperator::Add => left_value.checked_add(right_value)?,
                ArithmeticOperator::Subtract => left_value.subtract(right_value)?,
                _ => unreachable!(),
            };
            let answer = rational_to_exact_decimal_answer(result_value, 3)?;
            let expression = binary_expression(
                operator,
                exact_decimal_expression(left_coefficient, left_scale),
                exact_decimal_expression(right_coefficient, right_scale),
            );
            let graph = arithmetic_expression_graph(&expression, &answer)?;
            (
                expression,
                answer,
                graph,
                simple_decimal_input(),
                AnswerSchema::Decimal { max_scale: 3 },
            )
        }
        ArithmeticThemeMode::DecimalMultiplyDivide => {
            // Grade 5: build on the Grade 4 decimal-by-integer cases and sample
            // them with equal source probability alongside decimal-by-decimal cases. Decimal operands use at
            // most two significant digits and at most two decimal places.
            let operator = if rng.next_bounded(2) == 0 {
                ArithmeticOperator::Multiply
            } else {
                ArithmeticOperator::Divide
            };
            let use_integer_second_operand = rng.next_bounded(2) == 0;

            let (expression, answer) = match operator {
                ArithmeticOperator::Multiply => {
                    let (left_coefficient, left_scale) = draw_decimal_operand(rng, 2, 2);
                    let left_value = exact_decimal_rational(left_coefficient, left_scale)?;
                    let (right_expression, right_value) = if use_integer_second_operand {
                        let value = 1_i64 + rng.next_bounded(9) as i64;
                        (
                            integer_expression(value),
                            RationalCoefficient::new(value, 1)?,
                        )
                    } else {
                        let (coefficient, scale) = draw_decimal_operand(rng, 2, 2);
                        (
                            exact_decimal_expression(coefficient, scale),
                            exact_decimal_rational(coefficient, scale)?,
                        )
                    };
                    let result_value = left_value.multiply(right_value)?;
                    let answer = rational_to_exact_decimal_answer(result_value, 4)?;
                    (
                        binary_expression(
                            ArithmeticOperator::Multiply,
                            exact_decimal_expression(left_coefficient, left_scale),
                            right_expression,
                        ),
                        answer,
                    )
                }
                ArithmeticOperator::Divide => {
                    // Reverse-generate from an exact quotient. This avoids
                    // repeating decimals while retaining both decimal/integer
                    // and decimal/decimal curriculum cases.
                    let (quotient_coefficient, quotient_scale) = draw_decimal_operand(rng, 2, 2);
                    let quotient_value =
                        exact_decimal_rational(quotient_coefficient, quotient_scale)?;
                    let answer = rational_to_exact_decimal_answer(quotient_value, 2)?;
                    let (divisor_expression, divisor_value) = if use_integer_second_operand {
                        let value = 1_i64 + rng.next_bounded(9) as i64;
                        (
                            integer_expression(value),
                            RationalCoefficient::new(value, 1)?,
                        )
                    } else {
                        let (coefficient, scale) = draw_decimal_operand(rng, 2, 2);
                        (
                            exact_decimal_expression(coefficient, scale),
                            exact_decimal_rational(coefficient, scale)?,
                        )
                    };
                    let dividend_value = quotient_value.multiply(divisor_value)?;
                    let dividend_expression = rational_to_arithmetic_expression(dividend_value, 4)?;
                    if arithmetic_leaf_significant_digits(&dividend_expression)? > 3 {
                        return None;
                    }
                    (
                        binary_expression(
                            ArithmeticOperator::Divide,
                            dividend_expression,
                            divisor_expression,
                        ),
                        answer,
                    )
                }
                _ => unreachable!(),
            };
            let graph = arithmetic_expression_graph(&expression, &answer)?;
            (
                expression,
                answer,
                graph,
                simple_decimal_input(),
                AnswerSchema::Decimal { max_scale: 4 },
            )
        }
        ArithmeticThemeMode::SignedArithmetic1 => {
            let term_count = 2 + rng.next_bounded(3) as usize;
            let mut terms = (0..term_count)
                .map(|_| draw_signed_integer(rng, 15))
                .collect::<Vec<_>>();
            ensure_negative_term(rng, &mut terms);
            let mut expression = integer_expression(terms[0]);
            for term in terms.iter().skip(1) {
                let operator = if rng.next_bounded(2) == 0 {
                    ArithmeticOperator::Add
                } else {
                    ArithmeticOperator::Subtract
                };
                expression = binary_expression(operator, expression, integer_expression(*term));
            }
            let value = evaluate_expression(&expression)?;
            if !value.is_integer() {
                return None;
            }
            let answer = AnswerNode::Integer(value.numerator);
            let graph = arithmetic_expression_graph(&expression, &answer)?;
            (
                expression,
                answer,
                graph,
                simple_integer_input(true),
                AnswerSchema::Integer { min: -60, max: 60 },
            )
        }
        ArithmeticThemeMode::SignedArithmetic2 => {
            let leaf_count = 2 + rng.next_bounded(3) as usize;
            let mut values = (0..leaf_count)
                .map(|_| draw_signed_integer(rng, 9))
                .collect::<Vec<_>>();
            ensure_negative_term(rng, &mut values);
            let expression = draw_integer_arithmetic_ast(rng, &values)?;
            let value = evaluate_expression(&expression)?;
            if !value.is_integer() || value.numerator.unsigned_abs() > 6561 {
                return None;
            }
            let answer = AnswerNode::Integer(value.numerator);
            let graph = arithmetic_expression_graph(&expression, &answer)?;
            (
                expression,
                answer,
                graph,
                simple_integer_input(true),
                AnswerSchema::Integer {
                    min: -6561,
                    max: 6561,
                },
            )
        }
        ArithmeticThemeMode::FractionAddition
        | ArithmeticThemeMode::FractionSubtraction
        | ArithmeticThemeMode::FractionMultiplication
        | ArithmeticThemeMode::FractionDivision => {
            let triples = fraction_arithmetic_domain(mode);
            let &(left, right, result) = &triples[rng.next_bounded(triples.len() as u64) as usize];
            return Some(fraction_theme_problem(
                numeric_theme_id,
                mode,
                id,
                weights,
                left,
                right,
                result,
            ));
        }
    };
    let result = calculate_graph_effort(&solution_graph, weights);
    Some(Problem {
        schema_version: SCHEMA_VERSION,
        id,
        numeric_theme_id,
        prompt: ProblemPrompt::Arithmetic { expression },
        input_interface,
        answer_schema,
        canonical_answer: answer,
        solution_graph,
        operation_vector: result.operation_vector,
        effort: result.value,
    })
}

fn simple_integer_input(allow_negative: bool) -> AnswerInputInterface {
    AnswerInputInterface::SimpleNumeric {
        allow_decimal: false,
        allow_negative,
    }
}

fn simple_decimal_input() -> AnswerInputInterface {
    AnswerInputInterface::SimpleNumeric {
        allow_decimal: true,
        allow_negative: false,
    }
}

fn draw_decimal_operand(
    rng: &mut DeterministicRng,
    max_significant_digits: u32,
    max_scale: u32,
) -> (i64, u32) {
    debug_assert!(max_significant_digits >= 1);
    debug_assert!(max_scale >= 1);
    let significant_digits = 1 + rng.next_bounded(u64::from(max_significant_digits)) as u32;
    let lower = if significant_digits == 1 {
        1_i64
    } else {
        10_i64.pow(significant_digits - 1)
    };
    let upper = 10_i64.pow(significant_digits) - 1;
    let coefficient = loop {
        let candidate = lower + rng.next_bounded((upper - lower + 1) as u64) as i64;
        // Keep the coefficient canonical: trailing zeroes would create visually
        // redundant spellings such as 1.20 and inflate the apparent digit count.
        if candidate % 10 != 0 {
            break candidate;
        }
    };
    let scale = 1 + rng.next_bounded(u64::from(max_scale)) as u32;
    (coefficient, scale)
}

fn exact_decimal_expression(coefficient: i64, scale: u32) -> ArithmeticExpression {
    ArithmeticExpression::ExactDecimal { coefficient, scale }
}

fn exact_decimal_rational(coefficient: i64, scale: u32) -> Option<RationalCoefficient> {
    let denominator = 10_i64.checked_pow(scale)?;
    RationalCoefficient::new(coefficient, denominator)
}

fn rational_less_than(left: RationalCoefficient, right: RationalCoefficient) -> bool {
    i128::from(left.numerator) * i128::from(right.denominator)
        < i128::from(right.numerator) * i128::from(left.denominator)
}

fn arithmetic_leaf_significant_digits(expression: &ArithmeticExpression) -> Option<usize> {
    let magnitude = match expression {
        ArithmeticExpression::Integer { value } => value.unsigned_abs(),
        ArithmeticExpression::ExactDecimal { coefficient, .. } => coefficient.unsigned_abs(),
        _ => return None,
    };
    Some(magnitude.to_string().len())
}

fn rational_to_arithmetic_expression(
    value: RationalCoefficient,
    max_scale: u32,
) -> Option<ArithmeticExpression> {
    match rational_to_exact_decimal_answer(value, max_scale)? {
        AnswerNode::Integer(value) => Some(integer_expression(value)),
        AnswerNode::ExactDecimal { coefficient, scale } => {
            Some(exact_decimal_expression(coefficient, scale))
        }
        _ => None,
    }
}

fn rational_to_exact_decimal_answer(
    value: RationalCoefficient,
    max_scale: u32,
) -> Option<AnswerNode> {
    if value.is_integer() {
        return Some(AnswerNode::Integer(value.numerator));
    }
    let mut denominator = value.denominator;
    while denominator % 2 == 0 {
        denominator /= 2;
    }
    while denominator % 5 == 0 {
        denominator /= 5;
    }
    if denominator != 1 {
        return None;
    }
    for scale in 1..=max_scale {
        let power = 10_i64.checked_pow(scale)?;
        if power % value.denominator == 0 {
            let coefficient = value.numerator.checked_mul(power / value.denominator)?;
            return Some(AnswerNode::ExactDecimal { coefficient, scale });
        }
    }
    None
}

fn fraction_input_interface() -> AnswerInputInterface {
    AnswerInputInterface::StructuredMath {
        // Decimal input remains available so mathematically equivalent finite
        // decimals can be accepted with the independently configurable
        // `fraction_form_required` warning.
        allowed_structures: vec![EditorStructure::Fraction, EditorStructure::Decimal],
    }
}

fn integer_expression(value: i64) -> ArithmeticExpression {
    ArithmeticExpression::Integer { value }
}

fn rational_expression(value: RationalCoefficient) -> ArithmeticExpression {
    ArithmeticExpression::Rational { value }
}

fn binary_expression(
    operator: ArithmeticOperator,
    left: ArithmeticExpression,
    right: ArithmeticExpression,
) -> ArithmeticExpression {
    ArithmeticExpression::Binary {
        operator,
        left: Box::new(left),
        right: Box::new(right),
    }
}

fn draw_signed_integer(rng: &mut DeterministicRng, max_abs: i64) -> i64 {
    let magnitude = 1 + rng.next_bounded(max_abs as u64) as i64;
    if rng.next_bounded(2) == 0 {
        magnitude
    } else {
        -magnitude
    }
}

fn ensure_negative_term(rng: &mut DeterministicRng, values: &mut [i64]) {
    if values.iter().all(|value| *value > 0) {
        let index = rng.next_bounded(values.len() as u64) as usize;
        values[index] = -values[index];
    }
}

fn draw_integer_arithmetic_ast(
    rng: &mut DeterministicRng,
    values: &[i64],
) -> Option<ArithmeticExpression> {
    if values.len() == 1 {
        return Some(integer_expression(values[0]));
    }
    let split = 1 + rng.next_bounded((values.len() - 1) as u64) as usize;
    let left = draw_integer_arithmetic_ast(rng, &values[..split])?;
    let right = draw_integer_arithmetic_ast(rng, &values[split..])?;
    let operator = match rng.next_bounded(4) {
        0 => ArithmeticOperator::Add,
        1 => ArithmeticOperator::Subtract,
        2 => ArithmeticOperator::Multiply,
        _ => ArithmeticOperator::Divide,
    };
    let expression = binary_expression(operator, left, right);
    let value = evaluate_expression(&expression)?;
    // Division exercises stay inside the integers at every AST node.
    value.is_integer().then_some(expression)
}

fn evaluate_expression(expression: &ArithmeticExpression) -> Option<RationalCoefficient> {
    match expression {
        ArithmeticExpression::Integer { value } => RationalCoefficient::new(*value, 1),
        ArithmeticExpression::Rational { value } => Some(*value),
        ArithmeticExpression::ExactDecimal { coefficient, scale } => {
            exact_decimal_rational(*coefficient, *scale)
        }
        ArithmeticExpression::Binary {
            operator,
            left,
            right,
        } => {
            let left = evaluate_expression(left)?;
            let right = evaluate_expression(right)?;
            match operator {
                ArithmeticOperator::Add => left.checked_add(right),
                ArithmeticOperator::Subtract => left.subtract(right),
                ArithmeticOperator::Multiply => left.multiply(right),
                ArithmeticOperator::Divide => {
                    let quotient = left.divide(right)?;
                    quotient.is_integer().then_some(quotient)
                }
            }
        }
    }
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

fn quadratic_input_interface() -> AnswerInputInterface {
    AnswerInputInterface::StructuredMath {
        allowed_structures: vec![
            EditorStructure::Fraction,
            EditorStructure::Root,
            EditorStructure::Negative,
            EditorStructure::PlusMinus,
            EditorStructure::Tuple,
            EditorStructure::Arithmetic,
        ],
    }
}

fn answer_product(left: AnswerNode, right: AnswerNode) -> AnswerNode {
    if matches!(left, AnswerNode::Integer(1)) {
        return right;
    }
    AnswerNode::Binary {
        operator: AnswerBinaryOperator::Multiply,
        left: Box::new(left),
        right: Box::new(right),
    }
}

fn simplify_square_root(value: i64) -> Option<(i64, i64)> {
    if value <= 0 {
        return None;
    }
    let mut outside = 1_i64;
    let mut inside = value;
    let mut factor = 2_i64;
    while factor.checked_mul(factor)? <= inside {
        let square = factor.checked_mul(factor)?;
        while inside % square == 0 {
            inside /= square;
            outside = outside.checked_mul(factor)?;
        }
        factor += 1;
    }
    Some((outside, inside))
}

fn quadratic_formula_answer(
    constant: i64,
    radical_coefficient: i64,
    radicand: i64,
    denominator: i64,
) -> AnswerNode {
    let radical = answer_product(
        AnswerNode::Integer(radical_coefficient),
        AnswerNode::Root {
            radicand: Box::new(AnswerNode::Integer(radicand)),
            index: None,
        },
    );
    let plus_minus = AnswerNode::PlusMinus(Box::new(radical));
    let numerator = if constant == 0 {
        plus_minus
    } else {
        AnswerNode::Binary {
            operator: AnswerBinaryOperator::Add,
            left: Box::new(AnswerNode::Integer(constant)),
            right: Box::new(plus_minus),
        }
    };
    if denominator == 1 {
        numerator
    } else {
        AnswerNode::Fraction {
            numerator: Box::new(numerator),
            denominator: Box::new(AnswerNode::Integer(denominator)),
        }
    }
}

fn gcd_i64_positive(mut left: i64, mut right: i64) -> i64 {
    left = left.abs();
    right = right.abs();
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left
}

fn simultaneous_equation_input_interface() -> AnswerInputInterface {
    AnswerInputInterface::StructuredMath {
        allowed_structures: vec![EditorStructure::Negative, EditorStructure::Tuple],
    }
}

fn simultaneous_equation_problem(
    numeric_theme_id: u32,
    rng: &mut DeterministicRng,
    id: u32,
    weights: &OperationWeights,
) -> Option<Problem> {
    let x = rng.next_bounded(31) as i64 - 15;
    let y = rng.next_bounded(31) as i64 - 15;

    let mut first_equations = Vec::new();
    for a in -15_i64..=15 {
        if a == 0 {
            continue;
        }
        for b in -15_i64..=15 {
            if b == 0 {
                continue;
            }
            let c = a.checked_mul(x)?.checked_add(b.checked_mul(y)?)?;
            if c.unsigned_abs() <= 15 {
                first_equations.push((a, b, c));
            }
        }
    }
    if first_equations.is_empty() {
        return None;
    }
    let (a, b, c) = first_equations[rng.next_bounded(first_equations.len() as u64) as usize];

    let mut second_equations = Vec::new();
    for d in -15_i64..=15 {
        if d == 0 {
            continue;
        }
        for e in -15_i64..=15 {
            if e == 0 || a.checked_mul(e)?.checked_sub(b.checked_mul(d)?)? == 0 {
                continue;
            }
            let f = d.checked_mul(x)?.checked_add(e.checked_mul(y)?)?;
            if f.unsigned_abs() <= 15 {
                second_equations.push((d, e, f));
            }
        }
    }
    if second_equations.is_empty() {
        return None;
    }
    let (d, e, f) = second_equations[rng.next_bounded(second_equations.len() as u64) as usize];

    let canonical_answer = AnswerNode::Tuple(vec![AnswerNode::Integer(x), AnswerNode::Integer(y)]);
    let solution_graph = simultaneous_equation_graph(a, b, c, d, e, f, &canonical_answer, weights);
    let effort = calculate_graph_effort(&solution_graph, weights);
    Some(Problem {
        schema_version: SCHEMA_VERSION,
        id,
        numeric_theme_id,
        prompt: ProblemPrompt::SimultaneousEquation { a, b, c, d, e, f },
        input_interface: simultaneous_equation_input_interface(),
        answer_schema: AnswerSchema::OrderedPair,
        canonical_answer,
        solution_graph,
        operation_vector: effort.operation_vector,
        effort: effort.value,
    })
}

fn quadratic_equation_problem(
    numeric_theme_id: u32,
    mode: QuadraticEquationMode,
    rng: &mut DeterministicRng,
    id: u32,
    weights: &OperationWeights,
    fixed_answer: Option<&AnswerNode>,
) -> Option<Problem> {
    let (form, a, b, c, canonical_answer, solution_graph) = match mode {
        QuadraticEquationMode::SquareReduction => {
            let a_int = 1_i64 + rng.next_bounded(9) as i64;
            let answer = fixed_answer?.clone();
            let square_value = quadratic_one_square_value(&answer)?;
            let equals_constant = rng.next_bounded(2) == 0;
            let form = if equals_constant {
                QuadraticEquationForm::SquareEqualsConstant
            } else {
                QuadraticEquationForm::SquarePlusConstantZero
            };
            let constant = if equals_constant {
                a_int.checked_mul(square_value)?
            } else {
                a_int.checked_mul(square_value)?.checked_neg()?
            };
            let a = RationalCoefficient::new(a_int, 1)?;
            let c = RationalCoefficient::new(constant, 1)?;
            let graph = quadratic_square_graph(form, a, c, &answer);
            (form, a, RationalCoefficient::zero(), c, answer, graph)
        }
        QuadraticEquationMode::Factoring => {
            let first = draw_signed_integer(rng, 9);
            let mut second = draw_signed_integer(rng, 9);
            if first == second {
                second = -second;
            }
            if first == second {
                return None;
            }
            let scale = 1_i64 + rng.next_bounded(5) as i64;
            let b_int = first.checked_add(second)?.checked_neg()?;
            let c_int = first.checked_mul(second)?;
            let mut roots = vec![AnswerNode::Integer(first), AnswerNode::Integer(second)];
            roots.sort();
            let answer = AnswerNode::Tuple(roots);
            let graph = quadratic_factoring_graph(b_int, c_int, &answer);
            (
                QuadraticEquationForm::FactoredScale,
                RationalCoefficient::new(scale, 1)?,
                RationalCoefficient::new(b_int, 1)?,
                RationalCoefficient::new(c_int, 1)?,
                answer,
                graph,
            )
        }
        QuadraticEquationMode::Formula => {
            let a_int = 1_i64 + rng.next_bounded(8) as i64;
            let b_int = rng.next_bounded(37) as i64 - 18;
            let c_int = rng.next_bounded(41) as i64 - 20;
            if c_int == 0 {
                return None;
            }
            let discriminant = b_int
                .checked_mul(b_int)?
                .checked_sub(4_i64.checked_mul(a_int)?.checked_mul(c_int)?)?;
            if discriminant <= 0 {
                return None;
            }
            let (sqrt_coefficient, radicand) = simplify_square_root(discriminant)?;
            if radicand == 1 || radicand > 99 {
                return None;
            }
            let two_a = 2_i64.checked_mul(a_int)?;
            let common = gcd_i64_positive(gcd_i64_positive(b_int, sqrt_coefficient), two_a).max(1);
            let constant = b_int.checked_neg()?.checked_div(common)?;
            let radical_coefficient = sqrt_coefficient.checked_div(common)?;
            let denominator = two_a.checked_div(common)?;
            if constant.unsigned_abs() > 9
                || radical_coefficient.unsigned_abs() > 9
                || denominator > 15
            {
                return None;
            }
            let answer =
                quadratic_formula_answer(constant, radical_coefficient, radicand, denominator);
            if !answer.is_within_size_limit() {
                return None;
            }

            // About half the candidates present all coefficients over a common
            // denominator. Clearing that denominator is then the first solution step.
            let requested_denominator = if rng.next_bounded(2) == 0 {
                1_i64
            } else {
                2_i64 + rng.next_bounded(5) as i64
            };
            let denominator_scale = if requested_denominator > 1
                && (a_int % requested_denominator != 0
                    || b_int % requested_denominator != 0
                    || c_int % requested_denominator != 0)
            {
                requested_denominator
            } else {
                1
            };
            let a = RationalCoefficient::new(a_int, denominator_scale)?;
            let b = RationalCoefficient::new(b_int, denominator_scale)?;
            let c = RationalCoefficient::new(c_int, denominator_scale)?;
            let graph = quadratic_formula_graph(a, b, c, &answer);
            (QuadraticEquationForm::Standard, a, b, c, answer, graph)
        }
    };

    let effort = calculate_graph_effort(&solution_graph, weights);
    Some(Problem {
        schema_version: SCHEMA_VERSION,
        id,
        numeric_theme_id,
        prompt: ProblemPrompt::QuadraticEquation { form, a, b, c },
        input_interface: quadratic_input_interface(),
        answer_schema: AnswerSchema::Algebraic,
        canonical_answer,
        solution_graph,
        operation_vector: effort.operation_vector,
        effort: effort.value,
    })
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

    #[test]
    fn new_arithmetic_themes_generate_with_requested_domains() {
        use crate::model::{
            THEME_ID_FRACTION_ADDITION, THEME_ID_FRACTION_MULTIPLICATION,
            THEME_ID_FRACTION_SUBTRACTION, THEME_ID_MULTIPLICATION_TABLE,
            THEME_ID_ONE_DIGIT_SUBTRACTION, THEME_ID_SIGNED_ARITHMETIC_1,
            THEME_ID_SIGNED_ARITHMETIC_2, THEME_ID_TWO_DIGIT_ADDITION,
        };
        let ids = [
            THEME_ID_ONE_DIGIT_SUBTRACTION,
            THEME_ID_TWO_DIGIT_ADDITION,
            THEME_ID_MULTIPLICATION_TABLE,
            THEME_ID_SIGNED_ARITHMETIC_1,
            THEME_ID_SIGNED_ARITHMETIC_2,
            THEME_ID_FRACTION_ADDITION,
            THEME_ID_FRACTION_SUBTRACTION,
            THEME_ID_FRACTION_MULTIPLICATION,
        ];
        for theme_id in ids {
            let worksheet = generate_worksheet_request(&GenerateWorksheetRequest {
                schema_version: SCHEMA_VERSION,
                numeric_theme_id: theme_id,
                seed: "NwA".to_owned(),
                difficulty: crate::identity::DEFAULT_DIFFICULTY,
                timeout_ms: None,
                max_attempts: None,
            })
            .unwrap_or_else(|error| panic!("theme {theme_id} failed: {error}"));
            let expected = if theme_id >= THEME_ID_FRACTION_ADDITION {
                16
            } else {
                20
            };
            assert_eq!(worksheet.problems.len(), expected);
            for problem in &worksheet.problems {
                let ProblemPrompt::Arithmetic { expression } = &problem.prompt else {
                    panic!("new arithmetic theme returned a non-arithmetic prompt");
                };
                let value =
                    evaluate_expression(expression).expect("generated expression must evaluate");
                assert_eq!(rational_answer(value), problem.canonical_answer);
                match theme_id {
                    THEME_ID_ONE_DIGIT_SUBTRACTION => {
                        let ArithmeticExpression::Binary {
                            operator: ArithmeticOperator::Subtract,
                            left,
                            right,
                        } = expression
                        else {
                            panic!("subtraction shape");
                        };
                        let (
                            ArithmeticExpression::Integer { value: a },
                            ArithmeticExpression::Integer { value: b },
                        ) = (&**left, &**right)
                        else {
                            panic!("integer subtraction operands");
                        };
                        assert!((1..=18).contains(a));
                        assert!((1..=9).contains(b));
                        assert!((1..=9).contains(&value.numerator));
                    }
                    THEME_ID_TWO_DIGIT_ADDITION => {
                        let ArithmeticExpression::Binary {
                            operator: ArithmeticOperator::Add,
                            left,
                            right,
                        } = expression
                        else {
                            panic!("addition shape");
                        };
                        let (
                            ArithmeticExpression::Integer { value: a },
                            ArithmeticExpression::Integer { value: b },
                        ) = (&**left, &**right)
                        else {
                            panic!("integer addition operands");
                        };
                        assert!((10..=99).contains(a) && (10..=99).contains(b));
                    }
                    THEME_ID_MULTIPLICATION_TABLE => {
                        let ArithmeticExpression::Binary {
                            operator: ArithmeticOperator::Multiply,
                            left,
                            right,
                        } = expression
                        else {
                            panic!("multiplication shape");
                        };
                        let (
                            ArithmeticExpression::Integer { value: a },
                            ArithmeticExpression::Integer { value: b },
                        ) = (&**left, &**right)
                        else {
                            panic!("integer multiplication operands");
                        };
                        assert!((1..=9).contains(a) && (1..=9).contains(b));
                        assert_eq!(problem.solution_graph.steps.len(), 1);
                        assert!(matches!(
                            problem.solution_graph.steps[0].operation,
                            crate::effort::Operation::BigNum { .. }
                        ));
                    }
                    THEME_ID_SIGNED_ARITHMETIC_1 => {
                        assert!((2..=4).contains(&expression_leaf_count(expression)));
                        assert!(expression_operators(expression).iter().all(|op| matches!(
                            op,
                            ArithmeticOperator::Add | ArithmeticOperator::Subtract
                        )));
                    }
                    THEME_ID_SIGNED_ARITHMETIC_2 => {
                        assert!((2..=4).contains(&expression_leaf_count(expression)));
                        assert!(value.is_integer());
                    }
                    THEME_ID_FRACTION_ADDITION
                    | THEME_ID_FRACTION_SUBTRACTION
                    | THEME_ID_FRACTION_MULTIPLICATION => {
                        let ArithmeticExpression::Binary {
                            operator,
                            left,
                            right,
                        } = expression
                        else {
                            panic!("fraction binary shape");
                        };
                        for node in [&**left, &**right] {
                            let ArithmeticExpression::Rational { value } = node else {
                                panic!("fraction operand");
                            };
                            assert!(fraction_arithmetic_operand_domain().contains(value));
                            assert!(value.numerator > 0);
                        }
                        assert!(value.numerator > 0);
                        if theme_id == THEME_ID_FRACTION_ADDITION {
                            assert!(value.denominator > 1);
                            assert!(value.numerator <= 65);
                            assert!(value.denominator <= 72);
                        } else {
                            assert!(fraction_arithmetic_operand_domain().contains(&value));
                        }
                        if theme_id == THEME_ID_FRACTION_SUBTRACTION {
                            assert_eq!(*operator, ArithmeticOperator::Subtract);
                        }
                    }
                    _ => unreachable!(),
                }
            }
        }
    }

    #[test]
    fn all_registered_themes_are_deterministic_without_duplicate_prompts() {
        for registration in crate::registry::GENERATOR_REGISTRY {
            for seed in ["A1b2", "M7x9"] {
                for difficulty_value in [1u8, 3u8, 4u8] {
                    let difficulty =
                        crate::identity::Difficulty::try_from(difficulty_value).unwrap();
                    let request = GenerateWorksheetRequest {
                        schema_version: SCHEMA_VERSION,
                        numeric_theme_id: registration.numeric_theme_id,
                        seed: seed.to_owned(),
                        difficulty,
                        timeout_ms: None,
                        max_attempts: None,
                    };
                    let first = generate_worksheet_request(&request).unwrap_or_else(|error| {
                        panic!(
                            "theme {} seed {seed} difficulty {difficulty_value} failed: {error}",
                            registration.numeric_theme_id
                        )
                    });
                    let second = generate_worksheet_request(&request).unwrap();
                    assert_eq!(
                        first, second,
                        "same request must be byte-semantically deterministic"
                    );
                    assert_eq!(first.problems.len(), registration.problem_count);
                    for left in 0..first.problems.len() {
                        for right in left + 1..first.problems.len() {
                            assert_ne!(
                                first.problems[left].prompt,
                                first.problems[right].prompt,
                                "theme {} duplicated a prompt for seed {seed} difficulty {difficulty_value}",
                                registration.numeric_theme_id
                            );
                            assert_ne!(
                                problem_key(&first.problems[left]),
                                problem_key(&first.problems[right]),
                                "theme {} duplicated a commutative-equivalent prompt for seed {seed} difficulty {difficulty_value}",
                                registration.numeric_theme_id
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn broad_seed_effort_invariants_hold_for_every_registered_theme() {
        const SEEDS: [&str; 8] = [
            "A1b2", "C3d4", "E5f6", "G7h8", "J9k1", "L2m3", "N4p5", "Q6r7",
        ];
        let repaired_themes = std::collections::HashSet::from([
            crate::model::THEME_ID_LINEAR_EQUATION_1,
            crate::model::THEME_ID_LINEAR_EQUATION_2,
            crate::model::THEME_ID_SIGNED_ARITHMETIC_1,
            crate::model::THEME_ID_SIGNED_ARITHMETIC_2,
            crate::model::THEME_ID_FRACTION_ADDITION,
            crate::model::THEME_ID_FRACTION_MULTIPLICATION,
            crate::model::THEME_ID_FRACTION_SUBTRACTION,
            crate::model::THEME_ID_FRACTION_DIVISION,
            crate::model::THEME_ID_QUADRATIC_EQUATION_1,
            crate::model::THEME_ID_QUADRATIC_EQUATION_2,
            crate::model::THEME_ID_QUADRATIC_EQUATION_3,
            crate::model::THEME_ID_DECIMAL_ADD_SUBTRACT,
            crate::model::THEME_ID_DECIMAL_MULTIPLY_DIVIDE,
            crate::model::THEME_ID_SIMULTANEOUS_EQUATION_1,
        ]);

        for registration in crate::registry::GENERATOR_REGISTRY {
            let mut distinct_vectors = std::collections::HashSet::<Vec<u64>>::new();
            for seed in SEEDS {
                for difficulty_value in 1_u8..=4 {
                    let request = GenerateWorksheetRequest {
                        schema_version: SCHEMA_VERSION,
                        numeric_theme_id: registration.numeric_theme_id,
                        seed: seed.to_owned(),
                        difficulty: crate::identity::Difficulty::try_from(difficulty_value)
                            .unwrap(),
                        timeout_ms: None,
                        max_attempts: None,
                    };
                    let first = generate_worksheet_request(&request).unwrap_or_else(|error| {
                        panic!(
                            "theme {} seed {seed} difficulty {difficulty_value} failed: {error}",
                            registration.numeric_theme_id
                        )
                    });
                    let second = generate_worksheet_request(&request).unwrap();
                    assert_eq!(first, second, "same seed/revision must be deterministic");
                    assert_eq!(
                        first.identity.generator_revision,
                        registration.generator_revision
                    );
                    assert_eq!(
                        regenerate_problem_set(&first.problem_set_id).unwrap(),
                        first,
                        "problem_set_id regeneration must preserve the same revision identity"
                    );
                    if difficulty_value <= 2 {
                        assert!(
                            first
                                .problems
                                .windows(2)
                                .all(|pair| pair[0].effort <= pair[1].effort),
                            "theme {} difficulty {difficulty_value} lost easy/normal effort sort",
                            registration.numeric_theme_id
                        );
                    }
                    for problem in &first.problems {
                        assert!(problem.effort.is_finite() && problem.effort >= 0.0);
                        assert!(problem.operation_vector.is_nonnegative_finite());
                        assert_eq!(
                            problem.solution_graph.operation_vector(),
                            problem.operation_vector,
                            "stored operation vector must equal the graph projection"
                        );
                        distinct_vectors.insert(
                            problem
                                .operation_vector
                                .as_array()
                                .iter()
                                .map(|value| value.to_bits())
                                .collect(),
                        );
                    }
                }
            }
            if repaired_themes.contains(&registration.numeric_theme_id) {
                assert!(
                    distinct_vectors.len() >= 4,
                    "theme {} collapsed to only {} operation vectors across broad seeds",
                    registration.numeric_theme_id,
                    distinct_vectors.len()
                );
            }
        }
    }

    #[test]
    fn easy_and_normal_worksheets_are_presented_in_nondecreasing_effort_order() {
        for registration in crate::registry::GENERATOR_REGISTRY {
            for difficulty_value in [1_u8, 2_u8] {
                let worksheet = generate_worksheet_request(&GenerateWorksheetRequest {
                    schema_version: SCHEMA_VERSION,
                    numeric_theme_id: registration.numeric_theme_id,
                    seed: "EfrtRder".to_owned(),
                    difficulty: crate::identity::Difficulty::try_from(difficulty_value).unwrap(),
                    timeout_ms: Some(1_000),
                    max_attempts: Some(50_000),
                })
                .unwrap_or_else(|error| {
                    panic!(
                        "theme {} difficulty {difficulty_value} failed: {error}",
                        registration.numeric_theme_id
                    )
                });
                assert!(
                    worksheet
                        .problems
                        .windows(2)
                        .all(|pair| pair[0].effort <= pair[1].effort),
                    "theme {} difficulty {difficulty_value} was not effort-sorted: {:?}",
                    registration.numeric_theme_id,
                    worksheet
                        .problems
                        .iter()
                        .map(|problem| problem.effort)
                        .collect::<Vec<_>>()
                );
            }
        }
    }

    fn assert_decimal_operand(
        expression: &ArithmeticExpression,
        max_significant_digits: usize,
        max_scale: u32,
    ) -> (i64, u32) {
        let ArithmeticExpression::ExactDecimal { coefficient, scale } = expression else {
            panic!("expected exact decimal operand");
        };
        assert!(*coefficient > 0);
        assert!((1..=max_scale).contains(scale));
        assert_ne!(coefficient % 10, 0);
        assert!(coefficient.to_string().len() <= max_significant_digits);
        (*coefficient, *scale)
    }

    #[test]
    fn decimal_add_subtract_matches_grade_four_digit_and_place_value_bounds() {
        let mut seen_operators = std::collections::HashSet::new();
        let mut seen_scales = std::collections::HashSet::new();
        let mut saw_different_places = false;
        for seed in ["A1b2", "M7x9", "Q4r6", "Z8k3"] {
            let worksheet = generate_worksheet_request(&GenerateWorksheetRequest {
                schema_version: SCHEMA_VERSION,
                numeric_theme_id: crate::model::THEME_ID_DECIMAL_ADD_SUBTRACT,
                seed: seed.to_owned(),
                difficulty: crate::identity::Difficulty::try_from(3).unwrap(),
                timeout_ms: None,
                max_attempts: None,
            })
            .unwrap();
            for problem in worksheet.problems {
                let ProblemPrompt::Arithmetic {
                    expression:
                        ArithmeticExpression::Binary {
                            operator,
                            left,
                            right,
                        },
                } = problem.prompt
                else {
                    panic!("decimal add/subtract theme must be binary arithmetic");
                };
                assert!(matches!(
                    operator,
                    ArithmeticOperator::Add | ArithmeticOperator::Subtract
                ));
                seen_operators.insert(operator);
                let (_, left_scale) = assert_decimal_operand(&left, 3, 3);
                let (_, right_scale) = assert_decimal_operand(&right, 3, 3);
                seen_scales.insert(left_scale);
                seen_scales.insert(right_scale);
                saw_different_places |= left_scale != right_scale;
                match problem.canonical_answer {
                    AnswerNode::Integer(value) => assert!(value >= 0),
                    AnswerNode::ExactDecimal { coefficient, scale } => {
                        assert!(coefficient >= 0);
                        assert!((1..=3).contains(&scale));
                    }
                    _ => panic!("decimal add/subtract answer must be finite decimal"),
                }
            }
        }
        assert_eq!(seen_operators.len(), 2);
        assert_eq!(
            seen_scales,
            std::collections::HashSet::from([1_u32, 2_u32, 3_u32])
        );
        assert!(
            saw_different_places,
            "mixed-place addition/subtraction should be represented"
        );
    }

    #[test]
    fn decimal_multiply_divide_matches_grade_five_progression_and_exact_quotients() {
        let mut seen_operators = std::collections::HashSet::new();
        let mut saw_integer_second = false;
        let mut saw_decimal_second = false;
        for seed in ["A1b2", "M7x9", "Q4r6", "Z8k3", "D3c5", "N6p8"] {
            let worksheet = generate_worksheet_request(&GenerateWorksheetRequest {
                schema_version: SCHEMA_VERSION,
                numeric_theme_id: crate::model::THEME_ID_DECIMAL_MULTIPLY_DIVIDE,
                seed: seed.to_owned(),
                difficulty: crate::identity::Difficulty::try_from(3).unwrap(),
                timeout_ms: None,
                max_attempts: None,
            })
            .unwrap();
            for problem in worksheet.problems {
                let ProblemPrompt::Arithmetic {
                    expression:
                        ArithmeticExpression::Binary {
                            operator,
                            left,
                            right,
                        },
                } = &problem.prompt
                else {
                    panic!("decimal multiply/divide theme must be binary arithmetic");
                };
                assert!(matches!(
                    operator,
                    ArithmeticOperator::Multiply | ArithmeticOperator::Divide
                ));
                seen_operators.insert(*operator);
                match right.as_ref() {
                    ArithmeticExpression::Integer { value } => {
                        saw_integer_second = true;
                        assert!((1..=9).contains(value));
                    }
                    decimal @ ArithmeticExpression::ExactDecimal { .. } => {
                        saw_decimal_second = true;
                        assert_decimal_operand(decimal, 2, 2);
                    }
                    _ => panic!("second operand must be a positive integer or bounded decimal"),
                }
                if *operator == ArithmeticOperator::Multiply {
                    assert_decimal_operand(left, 2, 2);
                    match problem.canonical_answer {
                        AnswerNode::Integer(value) => assert!(value >= 0),
                        AnswerNode::ExactDecimal { coefficient, scale } => {
                            assert!(coefficient >= 0);
                            assert!((1..=4).contains(&scale));
                        }
                        _ => panic!("decimal multiplication answer must be finite decimal"),
                    }
                } else {
                    match left.as_ref() {
                        ArithmeticExpression::Integer { value } => {
                            assert!(*value > 0);
                            assert!(value.unsigned_abs().to_string().len() <= 3);
                        }
                        ArithmeticExpression::ExactDecimal { coefficient, scale } => {
                            assert!(*coefficient > 0);
                            assert!((1..=4).contains(scale));
                            assert_ne!(coefficient % 10, 0);
                            assert!(coefficient.to_string().len() <= 3);
                        }
                        _ => panic!("division dividend must be terminating decimal"),
                    }
                    let AnswerNode::ExactDecimal { coefficient, scale } = problem.canonical_answer
                    else {
                        panic!("reverse-generated decimal division quotient must be decimal");
                    };
                    assert!(coefficient > 0);
                    assert!((1..=2).contains(&scale));
                }
            }
        }
        assert_eq!(seen_operators.len(), 2);
        assert!(
            saw_integer_second,
            "Grade 4 decimal-by-integer foundation should remain represented"
        );
        assert!(
            saw_decimal_second,
            "Grade 5 decimal-by-decimal cases should be represented"
        );
    }

    #[test]
    fn fraction_themes_allow_decimal_answers_for_configurable_fraction_form_grading() {
        for theme_id in [
            crate::model::THEME_ID_FRACTION_ADDITION,
            crate::model::THEME_ID_FRACTION_SUBTRACTION,
            crate::model::THEME_ID_FRACTION_MULTIPLICATION,
            crate::model::THEME_ID_FRACTION_DIVISION,
        ] {
            let worksheet = generate_worksheet_request(&GenerateWorksheetRequest {
                schema_version: SCHEMA_VERSION,
                numeric_theme_id: theme_id,
                seed: "F1d2".to_owned(),
                difficulty: crate::identity::Difficulty::try_from(3).unwrap(),
                timeout_ms: None,
                max_attempts: None,
            })
            .unwrap();
            for problem in worksheet.problems {
                let AnswerInputInterface::StructuredMath { allowed_structures } =
                    problem.input_interface
                else {
                    panic!("fraction theme must use structured math input");
                };
                assert!(allowed_structures.contains(&EditorStructure::Fraction));
                assert!(allowed_structures.contains(&EditorStructure::Decimal));
            }
        }
    }

    #[test]
    fn fraction_division_domain_handles_integer_divisors_by_reciprocal() {
        let one_third = RationalCoefficient::new(1, 3).unwrap();
        let two = RationalCoefficient::new(2, 1).unwrap();
        let one_sixth = RationalCoefficient::new(1, 6).unwrap();
        let domain = fraction_arithmetic_domain(ArithmeticThemeMode::FractionDivision);
        assert!(domain.contains(&(one_third, two, one_sixth)));
        assert!(domain.iter().any(|(_, _, result)| result.is_integer()));
    }

    #[test]
    fn multiplication_table_exceptions_stay_in_theme_modules() {
        let multiplication = multiplication_table::solution_graph(56).operation_vector();
        assert_eq!(
            multiplication.get(crate::effort::OperationKind::BaseTimes),
            0.0
        );
        assert_eq!(
            multiplication.get(crate::effort::OperationKind::BigNum),
            56_f64.log10()
        );

        let division = division_table::solution_graph(56).operation_vector();
        assert_eq!(division.get(crate::effort::OperationKind::BaseTimes), 3.0);
        assert_eq!(division.get(crate::effort::OperationKind::BaseDivide), 0.0);
        assert_eq!(
            division.get(crate::effort::OperationKind::BigNum),
            56_f64.log10()
        );
    }

    #[test]
    fn division_one_stays_inside_the_multiplication_table_domain() {
        for seed in ["A1b2", "M7x9", "Q4r6"] {
            let worksheet = generate_worksheet_request(&GenerateWorksheetRequest {
                schema_version: SCHEMA_VERSION,
                numeric_theme_id: crate::model::THEME_ID_DIVISION_1,
                seed: seed.to_owned(),
                difficulty: crate::identity::Difficulty::try_from(3).unwrap(),
                timeout_ms: None,
                max_attempts: None,
            })
            .unwrap();
            for problem in worksheet.problems {
                let ProblemPrompt::Arithmetic {
                    expression:
                        ArithmeticExpression::Binary {
                            operator: ArithmeticOperator::Divide,
                            left,
                            right,
                        },
                } = problem.prompt
                else {
                    panic!("division(1) must be one binary division");
                };
                let (
                    ArithmeticExpression::Integer { value: dividend },
                    ArithmeticExpression::Integer { value: divisor },
                ) = (*left, *right)
                else {
                    panic!("division(1) operands must be integers");
                };
                let AnswerNode::Integer(quotient) = problem.canonical_answer else {
                    panic!("division(1) answer must be integer");
                };
                assert!((1..=81).contains(&dividend));
                assert!((1..=9).contains(&divisor));
                assert!((1..=9).contains(&quotient));
                assert_eq!(dividend, divisor * quotient);
            }
        }
    }

    #[test]
    fn quadratic_one_uses_only_the_two_requested_square_forms() {
        for seed in ["A1b2", "M7x9", "Q4r6"] {
            let worksheet = generate_worksheet_request(&GenerateWorksheetRequest {
                schema_version: SCHEMA_VERSION,
                numeric_theme_id: crate::model::THEME_ID_QUADRATIC_EQUATION_1,
                seed: seed.to_owned(),
                difficulty: crate::identity::Difficulty::try_from(3).unwrap(),
                timeout_ms: None,
                max_attempts: None,
            })
            .unwrap();
            for problem in worksheet.problems {
                let ProblemPrompt::QuadraticEquation { form, a, b, c } = problem.prompt else {
                    panic!("quadratic(1) prompt");
                };
                assert!(b.is_zero());
                assert!(a.is_integer() && (1..=9).contains(&a.numerator));
                match form {
                    QuadraticEquationForm::SquareEqualsConstant => assert!(c.numerator > 0),
                    QuadraticEquationForm::SquarePlusConstantZero => assert!(c.numerator < 0),
                    _ => panic!("quadratic(1) emitted an unsupported form"),
                }
                let square_value = c.numerator.unsigned_abs() / a.numerator.unsigned_abs();
                let integer_root = (1_u64..=16).find(|root| root * root == square_value);
                if let Some(root) = integer_root {
                    assert!((1..=16).contains(&root));
                } else {
                    assert!((2..=30).contains(&square_value));
                }
                assert!(matches!(problem.canonical_answer, AnswerNode::PlusMinus(_)));
            }
        }
    }

    #[test]
    fn quadratic_one_exposes_the_exact_unweighted_answer_domain() {
        let domain = quadratic_one_answer_domain();
        assert_eq!(domain.len(), 41);
        assert_eq!(
            domain.iter().filter(|answer| matches!(
                answer,
                AnswerNode::PlusMinus(value) if matches!(value.as_ref(), AnswerNode::Integer(1..=16))
            )).count(),
            16,
        );
        assert_eq!(
            domain
                .iter()
                .filter(|answer| matches!(
                    answer,
                    AnswerNode::PlusMinus(value) if matches!(
                        value.as_ref(),
                        AnswerNode::Root { radicand, index: None }
                            if matches!(radicand.as_ref(), AnswerNode::Integer(2..=30))
                    )
                ))
                .count(),
            25,
        );

        let mut rng = DeterministicRng::from_seed("quadratic-one-domain");
        let weights = OperationWeights::default();
        for (index, answer) in domain.iter().enumerate() {
            let problem = QUADRATIC_EQUATION_1_GENERATOR
                .draw_candidate_for_answer(&mut rng, index as u32 + 1, &weights, answer)
                .expect("every declared quadratic(1) answer must construct a problem");
            assert_eq!(&problem.canonical_answer, answer);
        }
    }

    #[test]
    fn quadratic_two_is_reverse_generated_from_two_integer_roots() {
        let worksheet = generate_worksheet_request(&GenerateWorksheetRequest {
            schema_version: SCHEMA_VERSION,
            numeric_theme_id: crate::model::THEME_ID_QUADRATIC_EQUATION_2,
            seed: "A1b2".to_owned(),
            difficulty: crate::identity::Difficulty::try_from(3).unwrap(),
            timeout_ms: None,
            max_attempts: None,
        })
        .unwrap();
        for problem in worksheet.problems {
            let ProblemPrompt::QuadraticEquation { form, a, b, c } = problem.prompt else {
                panic!("quadratic(2) prompt");
            };
            assert_eq!(form, QuadraticEquationForm::FactoredScale);
            assert!(a.is_integer() && (1..=5).contains(&a.numerator));
            let AnswerNode::Tuple(roots) = problem.canonical_answer else {
                panic!("quadratic(2) answer must contain both roots");
            };
            assert_eq!(roots.len(), 2);
            let (AnswerNode::Integer(r1), AnswerNode::Integer(r2)) = (&roots[0], &roots[1]) else {
                panic!("quadratic(2) roots must be integers");
            };
            assert_eq!(b, RationalCoefficient::new(-(r1 + r2), 1).unwrap());
            assert_eq!(c, RationalCoefficient::new(r1 * r2, 1).unwrap());
        }
    }

    fn quadratic_formula_bounds(answer: &AnswerNode) -> Option<(i64, i64, i64, i64)> {
        let (numerator, denominator) = match answer {
            AnswerNode::Fraction {
                numerator,
                denominator,
            } => {
                let AnswerNode::Integer(denominator) = denominator.as_ref() else {
                    return None;
                };
                (numerator.as_ref(), *denominator)
            }
            value => (value, 1),
        };
        let (constant, plus_minus) = match numerator {
            AnswerNode::Binary {
                operator: AnswerBinaryOperator::Add,
                left,
                right,
            } => {
                let AnswerNode::Integer(constant) = left.as_ref() else {
                    return None;
                };
                (*constant, right.as_ref())
            }
            AnswerNode::PlusMinus(_) => (0, numerator),
            _ => return None,
        };
        let AnswerNode::PlusMinus(radical) = plus_minus else {
            return None;
        };
        let (coefficient, root) = match radical.as_ref() {
            AnswerNode::Binary {
                operator: AnswerBinaryOperator::Multiply,
                left,
                right,
            } => {
                let AnswerNode::Integer(coefficient) = left.as_ref() else {
                    return None;
                };
                (*coefficient, right.as_ref())
            }
            root @ AnswerNode::Root { .. } => (1, root),
            _ => return None,
        };
        let AnswerNode::Root {
            radicand,
            index: None,
        } = root
        else {
            return None;
        };
        let AnswerNode::Integer(radicand) = radicand.as_ref() else {
            return None;
        };
        Some((constant, coefficient, *radicand, denominator))
    }

    #[test]
    fn quadratic_three_formula_answers_obey_display_bounds_and_include_fraction_coefficients() {
        let mut saw_fraction_coefficient = false;
        for seed in ["A1b2", "M7x9", "Q4r6", "Z8k3"] {
            let worksheet = generate_worksheet_request(&GenerateWorksheetRequest {
                schema_version: SCHEMA_VERSION,
                numeric_theme_id: crate::model::THEME_ID_QUADRATIC_EQUATION_3,
                seed: seed.to_owned(),
                difficulty: crate::identity::Difficulty::try_from(3).unwrap(),
                timeout_ms: None,
                max_attempts: None,
            })
            .unwrap();
            for problem in worksheet.problems {
                let ProblemPrompt::QuadraticEquation { form, a, b, c } = &problem.prompt else {
                    panic!("quadratic(3) prompt");
                };
                assert_eq!(*form, QuadraticEquationForm::Standard);
                saw_fraction_coefficient |= !a.is_integer() || !b.is_integer() || !c.is_integer();
                let (constant, radical_coefficient, radicand, denominator) =
                    quadratic_formula_bounds(&problem.canonical_answer)
                        .expect("quadratic(3) must use the bounded quadratic-formula AST");
                assert!(constant.unsigned_abs() <= 9);
                assert!(radical_coefficient.unsigned_abs() <= 9);
                assert!((2..=99).contains(&radicand));
                assert!((1..=15).contains(&denominator));
            }
        }
        assert!(
            saw_fraction_coefficient,
            "quadratic(3) should exercise clearing denominators"
        );
    }

    #[test]
    fn fraction_arithmetic_domains_are_broad_but_bounded() {
        let operands = fraction_arithmetic_operand_domain();
        assert_eq!(operands.len(), 57);
        assert!(operands.iter().all(|value| {
            !value.is_integer() && value.numerator > 0 && value.numerator + value.denominator <= 15
        }));

        assert_eq!(
            fraction_arithmetic_domain(ArithmeticThemeMode::FractionAddition).len(),
            1_071
        );
        assert_eq!(
            fraction_arithmetic_domain(ArithmeticThemeMode::FractionSubtraction).len(),
            244
        );
        assert_eq!(
            fraction_arithmetic_domain(ArithmeticThemeMode::FractionMultiplication).len(),
            291
        );
        assert_eq!(
            fraction_arithmetic_domain(ArithmeticThemeMode::FractionDivision).len(),
            1_068
        );
    }

    #[test]
    fn fraction_division_closes_answers_over_compact_fraction_or_integer_domain() {
        let fraction_domain = fraction_arithmetic_operand_domain();
        let domain = fraction_arithmetic_domain(ArithmeticThemeMode::FractionDivision);
        let mut saw_fraction_by_fraction = false;
        let mut saw_fraction_by_integer = false;
        let mut saw_integer_by_fraction = false;
        let mut saw_integer_answer = false;

        for &(left, right, result) in domain {
            assert!(!(left.is_integer() && right.is_integer()));
            assert!(
                fraction_domain.contains(&result)
                    || (result.is_integer() && (1..=9).contains(&result.numerator)),
                "unexpected division result outside compact domain: {left:?} / {right:?} = {result:?}"
            );
            saw_fraction_by_fraction |= !left.is_integer() && !right.is_integer();
            saw_fraction_by_integer |= !left.is_integer() && right.is_integer();
            saw_integer_by_fraction |= left.is_integer() && !right.is_integer();
            saw_integer_answer |= result.is_integer();
        }

        assert!(saw_fraction_by_fraction);
        assert!(saw_fraction_by_integer);
        assert!(saw_integer_by_fraction);
        assert!(saw_integer_answer);
    }

    #[test]
    fn fraction_addition_domain_includes_standard_unlike_denominator_examples() {
        let one_third = RationalCoefficient::new(1, 3).unwrap();
        let one_fourth = RationalCoefficient::new(1, 4).unwrap();
        let seven_twelfths = RationalCoefficient::new(7, 12).unwrap();
        let domain = fraction_arithmetic_domain(ArithmeticThemeMode::FractionAddition);
        assert!(domain.contains(&(one_third, one_fourth, seven_twelfths)));
        assert!(domain
            .iter()
            .any(|(left, right, _)| { left.denominator != right.denominator }));
    }

    #[test]
    fn hard_fraction_addition_worksheets_do_not_collapse_to_equal_denominators() {
        for seed in ["Ab3Z", "M7x9", "NwA", "Em7Z", "Qp5A"] {
            let worksheet = generate_worksheet_request(&GenerateWorksheetRequest {
                schema_version: SCHEMA_VERSION,
                numeric_theme_id: crate::model::THEME_ID_FRACTION_ADDITION,
                seed: seed.to_owned(),
                difficulty: crate::identity::Difficulty::try_from(3).unwrap(),
                timeout_ms: None,
                max_attempts: None,
            })
            .unwrap();
            let unlike_count = worksheet
                .problems
                .iter()
                .filter(|problem| {
                    let ProblemPrompt::Arithmetic {
                        expression: ArithmeticExpression::Binary { left, right, .. },
                    } = &problem.prompt
                    else {
                        return false;
                    };
                    let (
                        ArithmeticExpression::Rational { value: left },
                        ArithmeticExpression::Rational { value: right },
                    ) = (&**left, &**right)
                    else {
                        return false;
                    };
                    left.denominator != right.denominator
                })
                .count();
            assert!(
                unlike_count >= worksheet.problems.len() / 2,
                "seed {seed} produced only {unlike_count} unlike-denominator additions"
            );
        }
    }

    #[test]
    fn elementary_registered_themes_never_expose_negative_values() {
        for registration in crate::registry::GENERATOR_REGISTRY
            .iter()
            .filter(|registration| {
                registration
                    .curriculum_path
                    .iter()
                    .any(|segment| segment.starts_with("小学"))
            })
        {
            let worksheet = generate_worksheet_request(&GenerateWorksheetRequest {
                schema_version: SCHEMA_VERSION,
                numeric_theme_id: registration.numeric_theme_id,
                seed: "Em7Z".to_owned(),
                difficulty: crate::identity::DEFAULT_DIFFICULTY,
                timeout_ms: None,
                max_attempts: None,
            })
            .unwrap_or_else(|error| {
                panic!(
                    "elementary theme {} failed: {error}",
                    registration.numeric_theme_id
                )
            });
            for problem in &worksheet.problems {
                assert!(problem_allowed_by_curriculum(registration, problem));
                assert!(prompt_has_no_negative_values(&problem.prompt));
                assert!(answer_has_no_negative_values(&problem.canonical_answer));
                assert!(input_interface_has_no_negative_capability(
                    &problem.input_interface
                ));
            }
        }
    }

    #[test]
    fn simultaneous_equation_one_reverse_generates_bounded_unique_integer_solutions() {
        for seed in ["A1b2", "M7x9", "Q4r6", "Z8k3"] {
            let worksheet = generate_worksheet_request(&GenerateWorksheetRequest {
                schema_version: SCHEMA_VERSION,
                numeric_theme_id: crate::model::THEME_ID_SIMULTANEOUS_EQUATION_1,
                seed: seed.to_owned(),
                difficulty: crate::identity::Difficulty::try_from(3).unwrap(),
                timeout_ms: None,
                max_attempts: None,
            })
            .unwrap();
            assert_eq!(
                worksheet.problems.len(),
                crate::model::SIMULTANEOUS_EQUATION_PROBLEM_COUNT
            );
            for problem in worksheet.problems {
                let ProblemPrompt::SimultaneousEquation { a, b, c, d, e, f } = problem.prompt
                else {
                    panic!("simultaneous-equation(1) prompt");
                };
                assert!([a, b, c, d, e, f]
                    .iter()
                    .all(|value| value.unsigned_abs() <= 15));
                assert!(a != 0 && b != 0 && d != 0 && e != 0);
                assert_ne!(a * e - b * d, 0);
                let AnswerNode::Tuple(values) = problem.canonical_answer else {
                    panic!("simultaneous-equation(1) answer must be an ordered pair");
                };
                assert_eq!(values.len(), 2);
                let (AnswerNode::Integer(x), AnswerNode::Integer(y)) = (&values[0], &values[1])
                else {
                    panic!("simultaneous-equation(1) coordinates must be integers");
                };
                assert!(x.unsigned_abs() <= 15 && y.unsigned_abs() <= 15);
                assert_eq!(a * x + b * y, c);
                assert_eq!(d * x + e * y, f);
                assert!(matches!(problem.answer_schema, AnswerSchema::OrderedPair));
                assert!(matches!(
                    problem.input_interface,
                    AnswerInputInterface::StructuredMath { ref allowed_structures }
                        if allowed_structures == &[EditorStructure::Negative, EditorStructure::Tuple]
                ));
            }
        }
    }

    #[test]
    fn liar_puzzle_generates_only_the_six_non_implication_statement_forms_with_three_or_four_people(
    ) {
        let mut seen = [false; 6];
        for seed in [
            "A1b2", "M7x9", "Q4r6", "Z8k3", "L1aR", "T2uV", "P3qX", "H4mN", "C5dK", "R6sW", "B7fJ",
            "G8vY",
        ] {
            let worksheet = generate_worksheet_request(&GenerateWorksheetRequest {
                schema_version: SCHEMA_VERSION,
                numeric_theme_id: crate::model::THEME_ID_LIAR_PUZZLE,
                seed: seed.to_owned(),
                difficulty: crate::identity::Difficulty::try_from(2).unwrap(),
                timeout_ms: Some(1_000),
                max_attempts: Some(50_000),
            })
            .unwrap();
            assert_eq!(
                worksheet.problems.len(),
                crate::model::LIAR_PUZZLE_PROBLEM_COUNT
            );
            for problem in worksheet.problems {
                let ProblemPrompt::LiarPuzzle {
                    people_count,
                    statements,
                } = &problem.prompt
                else {
                    panic!("liar puzzle prompt");
                };
                assert!((3..=4).contains(people_count));
                assert_eq!(statements.len(), usize::from(*people_count));
                let expected_effort = statements
                    .iter()
                    .map(|statement| liar_puzzle_theme::statement_effort(statement, *people_count))
                    .sum::<u32>();
                assert_eq!(problem.effort, f64::from(expected_effort));
                assert_eq!(problem.solution_graph.steps.len(), expected_effort as usize);

                for (speaker_index, statement) in statements.iter().enumerate() {
                    let speaker = speaker_index as u8 + 1;
                    let assert_person = |person: u8| {
                        assert!((1..=*people_count).contains(&person));
                        assert_ne!(person, speaker);
                    };
                    match *statement {
                        LiarStatement::SaysLiar { person } => {
                            seen[0] = true;
                            assert_person(person);
                        }
                        LiarStatement::SaysNotLiar { person } => {
                            seen[1] = true;
                            assert_person(person);
                        }
                        LiarStatement::ExactlyOneLiar { first, second } => {
                            seen[2] = true;
                            assert!(first < second);
                            assert_person(first);
                            assert_person(second);
                        }
                        LiarStatement::ExactLiarCount { count } => {
                            seen[3] = true;
                            assert!((1..*people_count).contains(&count));
                        }
                        LiarStatement::BothLiar { first, second } => {
                            seen[4] = true;
                            assert!(first < second);
                            assert_person(first);
                            assert_person(second);
                        }
                        LiarStatement::BothNotLiar { first, second } => {
                            seen[5] = true;
                            assert!(first < second);
                            assert_person(first);
                            assert_person(second);
                        }
                        LiarStatement::Implication { .. } => {
                            panic!("liar-puzzle generation must not emit implications");
                        }
                    }
                }

                let solutions = liar_puzzle_solutions(*people_count, statements);
                assert_eq!(solutions.len(), 1);
                let solution = solutions[0];
                assert!((1..u32::from(*people_count)).contains(&solution.count_ones()));
                let expected_liars = (1..=*people_count)
                    .filter(|person| ((solution >> u32::from(*person - 1)) & 1) == 1)
                    .map(|person| AnswerNode::Integer(i64::from(person)))
                    .collect::<Vec<_>>();
                assert_eq!(problem.canonical_answer, AnswerNode::Tuple(expected_liars));
            }
        }
        assert!(
            seen.into_iter().all(|value| value),
            "not all liar statement forms were generated: {seen:?}"
        );
    }

    #[test]
    fn liar_statement_truth_and_effort_match_sat_semantics() {
        // Mask 0b0101 means people 1 and 3 are liars, people 2 and 4 are honest.
        let mask = 0b0101;
        assert!(liar_statement_truth(
            &LiarStatement::SaysLiar { person: 1 },
            mask
        ));
        assert!(liar_statement_truth(
            &LiarStatement::SaysNotLiar { person: 2 },
            mask
        ));
        assert!(liar_statement_truth(
            &LiarStatement::ExactlyOneLiar {
                first: 1,
                second: 2
            },
            mask
        ));
        assert!(liar_statement_truth(
            &LiarStatement::ExactLiarCount { count: 2 },
            mask
        ));
        assert!(liar_statement_truth(
            &LiarStatement::BothLiar {
                first: 1,
                second: 3
            },
            mask
        ));
        assert!(liar_statement_truth(
            &LiarStatement::BothNotLiar {
                first: 2,
                second: 4
            },
            mask
        ));
        assert!(liar_statement_truth(
            &LiarStatement::Implication {
                antecedent_person: 1,
                antecedent_is_liar: true,
                consequent_person: 2,
                consequent_is_liar: false,
            },
            mask
        ));

        assert_eq!(
            liar_puzzle_theme::statement_effort(&LiarStatement::SaysLiar { person: 1 }, 5),
            1
        );
        assert_eq!(
            liar_puzzle_theme::statement_effort(&LiarStatement::ExactLiarCount { count: 2 }, 5),
            5
        );
        assert_eq!(
            liar_puzzle_theme::statement_effort(
                &LiarStatement::Implication {
                    antecedent_person: 1,
                    antecedent_is_liar: true,
                    consequent_person: 2,
                    consequent_is_liar: false,
                },
                5
            ),
            2
        );
    }

    fn expression_leaf_count(expression: &ArithmeticExpression) -> usize {
        match expression {
            ArithmeticExpression::Integer { .. }
            | ArithmeticExpression::Rational { .. }
            | ArithmeticExpression::ExactDecimal { .. } => 1,
            ArithmeticExpression::Binary { left, right, .. } => {
                expression_leaf_count(left) + expression_leaf_count(right)
            }
        }
    }

    fn expression_operators(expression: &ArithmeticExpression) -> Vec<ArithmeticOperator> {
        match expression {
            ArithmeticExpression::Integer { .. }
            | ArithmeticExpression::Rational { .. }
            | ArithmeticExpression::ExactDecimal { .. } => Vec::new(),
            ArithmeticExpression::Binary {
                operator,
                left,
                right,
            } => {
                let mut values = vec![*operator];
                values.extend(expression_operators(left));
                values.extend(expression_operators(right));
                values
            }
        }
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
