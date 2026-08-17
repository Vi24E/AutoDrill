use std::cell::Cell;
use std::collections::HashSet;
use std::time::{Duration, Instant};

use crate::answer::AnswerNode;
use crate::effort::OperationWeights;
use crate::error::GenerationError;
use crate::identity::{validate_seed, ProblemSetIdentity};
use crate::model::{
    AnswerInputInterface, ArithmeticExpression, ArithmeticOperator, EditorStructure,
    GenerateProblemRequest, GenerateWorksheetRequest, LayoutMetadata, Problem, ProblemPrompt,
    Worksheet,
};
use crate::registry::{active_registration, registration, resolved_weights};
use crate::rng::DeterministicRng;
use crate::schema::SCHEMA_VERSION;
use crate::theme::{CurriculumSafetyPolicy, DedupPolicy, SamplingLayerSpec, ThemeRegistration};
use crate::themes::{
    basic_arithmetic as basic_theme, column_arithmetic as column_theme, decimals as decimal_theme,
    equations as equation_theme, fractions as fraction_theme, liar_puzzle as liar_puzzle_theme,
    mini_sudoku as mini_sudoku_theme,
};

// Deterministic max_attempts is the primary work budget. Wall-clock time is only
// an emergency browser watchdog and must tolerate cold WASM/browser scheduling.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(15);
pub const DEFAULT_MAX_ATTEMPTS: u64 = 10_000;
const CANDIDATE_POOL_MULTIPLIER: usize = 8;
const DIFFICULTY_BOOTSTRAP_DRAWS: usize = 5;
const EFFORT_TRIM_PER_SIDE: usize = 2;
const DIVERSITY_MULTIPLIER: usize = 2;
const TIMEOUT_CHECK_ATTEMPT_INTERVAL: u64 = 64;

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

/// Registry-backed generator interface. Only the current generator revision for
/// each theme is registered during pre-release development.
#[derive(Clone, Copy)]
pub(crate) struct GeneratorEntry {
    pub generator: &'static dyn ProblemGenerator,
}

impl GeneratorEntry {
    pub const fn current(generator: &'static dyn ProblemGenerator) -> Self {
        Self { generator }
    }
}

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

    fn sampling_layers(&self) -> Option<&'static [SamplingLayerSpec]> {
        None
    }

    fn sampling_layer(&self, _problem: &Problem) -> Option<usize> {
        None
    }

    /// Layered generators may return a smaller multiplier when they constructively
    /// draw layers with a known balanced distribution.
    fn bootstrap_layer_multiplier(&self) -> usize {
        self.sampling_layers()
            .map_or(1, |layers| layers.len().max(1))
    }

    /// Constructive samplers may draw the same prompt more than once. They can
    /// opt into prompt-level pool deduplication before difficulty selection.
    fn deduplicate_bootstrap_pool(&self) -> bool {
        false
    }

    /// A layered theme can opt into direct per-layer bootstrap generation.
    /// This prevents rejection-rate differences between archetypes from
    /// distorting the accepted bootstrap population.
    fn constructive_layer_sampling(&self) -> bool {
        false
    }

    fn draw_candidate_for_layer(
        &self,
        _rng: &mut DeterministicRng,
        _ordinal: u32,
        _weights: &OperationWeights,
        _layer: usize,
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

pub(crate) fn registered_generator_entries() -> impl Iterator<Item = GeneratorEntry> {
    basic_theme::GENERATORS
        .iter()
        .copied()
        .chain(fraction_theme::GENERATORS.iter().copied())
        .chain(decimal_theme::GENERATORS.iter().copied())
        .chain(column_theme::GENERATORS.iter().copied())
        .chain(equation_theme::GENERATORS.iter().copied())
        .chain(liar_puzzle_theme::GENERATORS.iter().copied())
        .chain(mini_sudoku_theme::GENERATORS.iter().copied())
}

pub fn registered_generator(
    numeric_theme_id: u32,
    generator_revision: u32,
) -> Option<&'static dyn ProblemGenerator> {
    registered_generator_entries()
        .map(|entry| entry.generator)
        .find(|generator| {
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

fn layered_quotas(layers: &[SamplingLayerSpec], problem_count: usize) -> Vec<usize> {
    let minimum_total: usize = layers.iter().map(|layer| layer.minimum).sum();
    assert!(
        minimum_total <= problem_count,
        "layer minimum quota exceeds worksheet size"
    );
    let mut quotas = layers.iter().map(|layer| layer.minimum).collect::<Vec<_>>();
    let remaining = problem_count - minimum_total;
    if remaining == 0 {
        return quotas;
    }
    let total_weight: u64 = layers.iter().map(|layer| u64::from(layer.weight)).sum();
    if total_weight == 0 {
        let layer_count = quotas.len();
        for offset in 0..remaining {
            quotas[offset % layer_count] += 1;
        }
        return quotas;
    }
    let mut remainders = Vec::with_capacity(layers.len());
    let mut assigned = 0_usize;
    for (index, layer) in layers.iter().enumerate() {
        let scaled = (remaining as u64) * u64::from(layer.weight);
        let base = (scaled / total_weight) as usize;
        quotas[index] += base;
        assigned += base;
        remainders.push((scaled % total_weight, index));
    }
    remainders.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    for &(_, index) in remainders.iter().take(remaining - assigned) {
        quotas[index] += 1;
    }
    quotas
}

fn layered_pool_has_capacity(
    generator: &dyn ProblemGenerator,
    pool: &[Problem],
    problem_count: usize,
    difficulty: u8,
) -> bool {
    let Some(layers) = generator.sampling_layers() else {
        return true;
    };
    let quotas = layered_quotas(layers, problem_count);
    let mut distinct = (0..layers.len())
        .map(|_| HashSet::new())
        .collect::<Vec<_>>();
    for problem in pool {
        if let Some(layer) = generator.sampling_layer(problem) {
            if let Some(keys) = distinct.get_mut(layer) {
                keys.insert(problem_key(generator.registration(), problem));
            }
        }
    }
    quotas.into_iter().enumerate().all(|(index, quota)| {
        let required = if difficulty == 4 {
            quota
        } else {
            quota + EFFORT_TRIM_PER_SIDE * 2
        };
        distinct[index].len() >= required
    })
}

#[allow(clippy::too_many_arguments)]
fn select_layered_candidates<C: MonotonicClock + ?Sized>(
    pool: Vec<Problem>,
    generator: &dyn ProblemGenerator,
    problem_count: usize,
    difficulty: u8,
    rng: &mut DeterministicRng,
    unique_finite_pool: bool,
    started: Duration,
    clock: &C,
    config: &GenerationConfig,
    attempts: &mut u64,
) -> Result<Vec<Problem>, GenerationError> {
    let layers = generator
        .sampling_layers()
        .expect("layered selection requires layers");
    let quotas = layered_quotas(layers, problem_count);
    let mut layer_pools = (0..layers.len()).map(|_| Vec::new()).collect::<Vec<_>>();
    for problem in pool {
        if let Some(layer) = generator.sampling_layer(&problem) {
            if let Some(layer_pool) = layer_pools.get_mut(layer) {
                layer_pool.push(problem);
            }
        }
    }
    let mut selected = Vec::with_capacity(problem_count);
    for (layer_index, quota) in quotas.into_iter().enumerate() {
        if quota == 0 {
            continue;
        }
        let layer_pool = std::mem::take(&mut layer_pools[layer_index]);
        let minimum_pool = if difficulty == 4 {
            quota
        } else {
            quota + EFFORT_TRIM_PER_SIDE * 2
        };
        if layer_pool.len() < minimum_pool {
            return Err(GenerationError::AttemptLimit {
                attempts: *attempts,
                max_attempts: config.max_attempts,
            });
        }
        selected.extend(select_candidates_from_pool(
            layer_pool,
            generator,
            quota,
            difficulty,
            rng,
            unique_finite_pool,
            started,
            clock,
            config,
            attempts,
        )?);
    }
    Ok(selected)
}

#[allow(clippy::too_many_arguments)]
fn select_candidates_from_pool<C: MonotonicClock + ?Sized>(
    mut pool: Vec<Problem>,
    generator: &dyn ProblemGenerator,
    count: usize,
    difficulty: u8,
    rng: &mut DeterministicRng,
    unique_finite_pool: bool,
    started: Duration,
    clock: &C,
    config: &GenerationConfig,
    attempts: &mut u64,
) -> Result<Vec<Problem>, GenerationError> {
    if generator.deduplicate_bootstrap_pool() {
        let mut distinct = HashSet::with_capacity(pool.len());
        pool.retain(|problem| distinct.insert(problem_key(generator.registration(), problem)));
    }

    if difficulty == 4 {
        let mut selected = Vec::with_capacity(count);
        let mut selected_expressions = HashSet::with_capacity(count);
        while selected.len() < count {
            consume_attempt(started, clock, config, attempts)?;
            let selected_index = rng.next_bounded(pool.len() as u64) as usize;
            let candidate = pool.swap_remove(selected_index);
            if selected_expressions.insert(problem_key(generator.registration(), &candidate)) {
                selected.push(candidate);
            }
        }
        return Ok(selected);
    }

    pool.sort_by(|left, right| {
        left.effort
            .total_cmp(&right.effort)
            .then_with(|| {
                problem_key(generator.registration(), left)
                    .cmp(&problem_key(generator.registration(), right))
            })
            .then_with(|| left.id.cmp(&right.id))
    });
    let bootstrap_count = count + EFFORT_TRIM_PER_SIDE * 2;
    let mut selected = Vec::with_capacity(bootstrap_count);
    let mut selected_expressions = HashSet::with_capacity(bootstrap_count);
    let order_statistic_index = match difficulty {
        1 => 0,
        2 => 2,
        3 => 4,
        _ => unreachable!("random difficulty is handled above"),
    };
    while selected.len() < bootstrap_count {
        consume_attempt(started, clock, config, attempts)?;
        let mut draws = [0_usize; DIFFICULTY_BOOTSTRAP_DRAWS];
        for draw in &mut draws {
            *draw = rng.next_bounded(pool.len() as u64) as usize + 1;
        }
        draws.sort_unstable();
        let selected_index = draws[order_statistic_index] - 1;
        if !unique_finite_pool {
            let key = problem_key(generator.registration(), &pool[selected_index]);
            if !selected_expressions.insert(key) {
                continue;
            }
        }
        selected.push(pool.remove(selected_index));
    }
    selected.sort_by(|left, right| {
        left.effort
            .total_cmp(&right.effort)
            .then_with(|| {
                problem_key(generator.registration(), left)
                    .cmp(&problem_key(generator.registration(), right))
            })
            .then_with(|| left.id.cmp(&right.id))
    });
    Ok(selected
        .into_iter()
        .skip(EFFORT_TRIM_PER_SIDE)
        .take(count)
        .collect())
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
    let n = registration.layout.problem_count;
    let finite_distinct_count = generator.finite_distinct_candidate_count();
    let unique_finite_pool = finite_distinct_count.is_some();
    let pool_size = CANDIDATE_POOL_MULTIPLIER * n * generator.bootstrap_layer_multiplier();
    let required_diversity = DIVERSITY_MULTIPLIER * n;

    let pool = if let Some(finite_count) = finite_distinct_count {
        // Draw a uniform subset of domain indices without replacement.
        // Runtime therefore scales with the bootstrap pool requested by
        // the selector, not with the size of the mathematical domain.
        let target_pool_size = pool_size.min(finite_count);
        let mut sampled_indices = HashSet::with_capacity(target_pool_size);
        let mut distinct = HashSet::with_capacity(target_pool_size);
        let mut candidate_pool = Vec::with_capacity(target_pool_size);

        loop {
            let enough_candidates = candidate_pool.len() >= target_pool_size;
            let enough_diversity = distinct.len() >= required_diversity.min(finite_count);
            let enough_layers = layered_pool_has_capacity(
                generator,
                &candidate_pool,
                n,
                identity.difficulty.value(),
            );
            if enough_candidates && enough_diversity && enough_layers {
                break;
            }
            if sampled_indices.len() == finite_count {
                break;
            }

            consume_attempt(started, clock, config, &mut attempts)?;
            let index = rng.next_bounded(finite_count as u64) as usize;
            if !sampled_indices.insert(index) {
                continue;
            }
            let ordinal = u32::try_from(attempts).unwrap_or(u32::MAX);
            let Some(problem) = generator.draw_finite_candidate(index, ordinal, &weights) else {
                continue;
            };
            if !problem_allowed_by_curriculum(registration, &problem) {
                continue;
            }
            let key = problem_key(registration, &problem);
            if !distinct.insert(key) {
                continue;
            }
            candidate_pool.push(problem);
        }

        if candidate_pool.len() < n
            || distinct.len() < required_diversity.min(finite_count)
            || !layered_pool_has_capacity(
                generator,
                &candidate_pool,
                n,
                identity.difficulty.value(),
            )
        {
            return Err(GenerationError::AttemptLimit {
                attempts,
                max_attempts: config.max_attempts,
            });
        }
        candidate_pool
    } else if generator.constructive_layer_sampling() {
        let layers = generator
            .sampling_layers()
            .expect("constructive layer sampling requires declared layers");
        let pool_quotas = layered_quotas(layers, pool_size);
        let mut candidate_pool = Vec::with_capacity(pool_size);
        let mut distinct = HashSet::with_capacity(pool_size);
        for (layer_index, target) in pool_quotas.into_iter().enumerate() {
            let mut accepted = 0_usize;
            while accepted < target {
                consume_attempt(started, clock, config, &mut attempts)?;
                let ordinal = u32::try_from(attempts).unwrap_or(u32::MAX);
                let Some(problem) =
                    generator.draw_candidate_for_layer(&mut rng, ordinal, &weights, layer_index)
                else {
                    continue;
                };
                if !problem_allowed_by_curriculum(registration, &problem) {
                    continue;
                }
                let key = problem_key(registration, &problem);
                if generator.deduplicate_bootstrap_pool() {
                    if !distinct.insert(key) {
                        continue;
                    }
                } else {
                    distinct.insert(key);
                }
                candidate_pool.push(problem);
                accepted += 1;
            }
        }
        check_timeout(started, clock, config)?;
        candidate_pool
    } else {
        loop {
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
                    if !problem_allowed_by_curriculum(registration, &problem) {
                        continue;
                    }
                    if fixed_answer
                        .as_ref()
                        .is_some_and(|answer| problem.canonical_answer != *answer)
                    {
                        continue;
                    }
                    distinct.insert(problem_key(registration, &problem));
                    candidate_pool.push(problem);
                    break;
                }
            }
            check_timeout(started, clock, config)?;
            if distinct.len() >= required_diversity
                && layered_pool_has_capacity(
                    generator,
                    &candidate_pool,
                    n,
                    identity.difficulty.value(),
                )
            {
                break candidate_pool;
            }
            // The full pool is discarded. The next loop consumes fresh attempts
            // and fresh deterministic RNG draws.
        }
    };

    let mut selected = if generator.sampling_layers().is_some() {
        select_layered_candidates(
            pool,
            generator,
            n,
            identity.difficulty.value(),
            &mut rng,
            unique_finite_pool,
            started,
            clock,
            config,
            &mut attempts,
        )?
    } else {
        select_candidates_from_pool(
            pool,
            generator,
            n,
            identity.difficulty.value(),
            &mut rng,
            unique_finite_pool,
            started,
            clock,
            config,
            &mut attempts,
        )?
    };

    if identity.difficulty.value() <= 2 {
        // Easy and normal worksheets should progress from lower to higher effort
        // so the sheet itself has a pedagogical difficulty ramp. Keep the same
        // deterministic tie-breakers used during candidate selection.
        selected.sort_by(|left, right| {
            left.effort
                .total_cmp(&right.effort)
                .then_with(|| {
                    problem_key(generator.registration(), left)
                        .cmp(&problem_key(generator.registration(), right))
                })
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
        problem.schema_version = identity.schema_version;
    }
    check_timeout(started, clock, config)?;

    Ok(Worksheet {
        schema_version: identity.schema_version,
        problem_set_id: identity.to_string(),
        identity: identity.clone(),
        skill_id: registration.skill_id.to_owned(),
        curriculum_path: registration
            .curriculum_path
            .iter()
            .map(|segment| (*segment).to_owned())
            .collect(),
        layout: LayoutMetadata {
            problem_count: registration.layout.problem_count,
            columns: registration.layout.columns,
            rows: registration.layout.rows,
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
    // `max_attempts` is the deterministic primary budget. BrowserClock crosses
    // the WASM/JS boundary, so sampling wall time on every rejected candidate
    // is both expensive and unnecessary for an emergency watchdog. Check at
    // the start and then periodically; generation also checks at phase ends.
    if (*attempts).is_multiple_of(TIMEOUT_CHECK_ATTEMPT_INTERVAL) {
        check_timeout(started, clock, config)?;
    }
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
    match registration.safety {
        CurriculumSafetyPolicy::Unrestricted => true,
        CurriculumSafetyPolicy::NonNegativeOnly => {
            prompt_has_no_negative_values(&problem.prompt)
                && answer_has_no_negative_values(&problem.canonical_answer)
                && input_interface_has_no_negative_capability(&problem.input_interface)
        }
    }
}

fn prompt_has_no_negative_values(prompt: &ProblemPrompt) -> bool {
    match prompt {
        ProblemPrompt::Addition { .. } => true,
        ProblemPrompt::Arithmetic { expression } => expression_has_no_negative_values(expression),
        ProblemPrompt::ColumnArithmetic { left, right, .. } => {
            expression_has_no_negative_values(left) && expression_has_no_negative_values(right)
        }
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
        ProblemPrompt::LiarPuzzle { .. } | ProblemPrompt::MiniSudoku { .. } => true,
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
        AnswerInputInterface::DigitGrid { .. } => true,
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

fn problem_key(registration: &ThemeRegistration, problem: &Problem) -> ProblemPrompt {
    match &problem.prompt {
        // One-digit addition and the multiplication table intentionally keep
        // ordered variants because their total populations are small.
        ProblemPrompt::Addition { left, right } => ProblemPrompt::Addition {
            left: *left,
            right: *right,
        },
        ProblemPrompt::Arithmetic { expression } => ProblemPrompt::Arithmetic {
            expression: match registration.dedup {
                DedupPolicy::PreserveOperandOrder => expression.clone(),
                DedupPolicy::CanonicalizeCommutative => {
                    canonicalize_commutative_expression(expression)
                }
            },
        },
        ProblemPrompt::ColumnArithmetic {
            operator,
            left,
            right,
        } => ProblemPrompt::ColumnArithmetic {
            operator: *operator,
            left: left.clone(),
            right: right.clone(),
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
        ProblemPrompt::MiniSudoku { givens } => ProblemPrompt::MiniSudoku { givens: givens.clone() },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::answer::AnswerBinaryOperator;
    use crate::model::{AnswerSchema, LiarStatement, QuadraticEquationForm, RationalCoefficient};
    use crate::themes::{division_table, multiplication_table};
    use proptest::prelude::*;

    struct ConstantGenerator;

    impl ProblemGenerator for ConstantGenerator {
        fn registration(&self) -> &'static ThemeRegistration {
            &basic_theme::ONE_DIGIT_ADDITION_REGISTRATION
        }

        fn draw_candidate(
            &self,
            _rng: &mut DeterministicRng,
            ordinal: u32,
            weights: &OperationWeights,
        ) -> Option<Problem> {
            Some(basic_theme::one_digit_addition_problem(
                ordinal, 1, 1, weights,
            ))
        }
    }

    #[test]
    fn insufficient_diversity_regenerates_until_attempt_limit() {
        let identity = ProblemSetIdentity::new(
            basic_theme::THEME_ID_ONE_DIGIT_ADDITION,
            basic_theme::GENERATOR_REVISION_ONE_DIGIT_ADDITION,
            "Ab3Z",
            crate::identity::DEFAULT_DIFFICULTY,
        )
        .unwrap();
        let config = GenerationConfig::default().with_max_attempts(100);
        let clock = StepClock::new(Duration::ZERO, Duration::ZERO);
        let error = generate_with_generator(
            &identity,
            &basic_theme::ONE_DIGIT_ADDITION_REGISTRATION,
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
        let integer = equation_theme::linear_solution_domain(
            equation_theme::LinearEquationMode::IntegerSolution,
        );
        assert_eq!(integer.len(), 31);
        assert_eq!(integer.first().unwrap().numerator, -15);
        assert_eq!(integer.last().unwrap().numerator, 15);
        assert!(integer.iter().all(|value| value.denominator == 1));

        let rational = equation_theme::linear_solution_domain(
            equation_theme::LinearEquationMode::RationalSolution,
        );
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
        for generator in [
            &equation_theme::LINEAR_EQUATION_1_GENERATOR,
            &equation_theme::LINEAR_EQUATION_2_GENERATOR,
        ] {
            let mut rng = DeterministicRng::from_seed("AllAns7");
            let weights = resolved_weights(generator.registration());
            for answer in equation_theme::linear_answer_domain(generator.mode) {
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
        let integer = equation_theme::linear_integer_domain_with_zero();
        assert_eq!(integer.len(), 31);
        assert_eq!(integer.iter().filter(|value| value.is_zero()).count(), 1);
        assert_eq!(integer.first().unwrap().numerator, -15);
        assert_eq!(integer.last().unwrap().numerator, 15);

        let rational = equation_theme::linear_rational_domain_with_zero();
        assert_eq!(
            rational.len(),
            equation_theme::linear_rational_domain().len() + 1
        );
        assert_eq!(rational.iter().filter(|value| value.is_zero()).count(), 1);
    }

    #[test]
    fn new_arithmetic_themes_generate_with_requested_domains() {
        use crate::themes::basic_arithmetic::{
            THEME_ID_MULTIPLICATION_TABLE, THEME_ID_ONE_DIGIT_SUBTRACTION,
            THEME_ID_SIGNED_ARITHMETIC_1, THEME_ID_SIGNED_ARITHMETIC_2,
            THEME_ID_TWO_DIGIT_ADDITION,
        };
        use crate::themes::fractions::{
            THEME_ID_FRACTION_ADDITION, THEME_ID_FRACTION_MULTIPLICATION,
            THEME_ID_FRACTION_SUBTRACTION,
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
                let value = crate::generator_support::evaluate_expression(expression)
                    .expect("generated expression must evaluate");
                assert_eq!(
                    crate::normalize::normalize_answer(&crate::generator_support::rational_answer(
                        value
                    )),
                    crate::normalize::normalize_answer(&problem.canonical_answer),
                );
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
                        let answer = a * b;
                        assert!(problem.solution_graph.steps.is_empty());
                        assert_eq!(problem.operation_vector, crate::effort::OperationVector::zero());
                        assert_eq!(problem.theme_specific_effort, Some((answer as f64).log10()));
                        assert_eq!(problem.effort, (answer as f64).log10());
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
                        assert!(value.numerator.unsigned_abs() <= 200);
                        assert!(value.denominator <= 36);
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
                            assert!(fraction_theme::operand_domain_v1().contains(value));
                            assert!(value.numerator > 0);
                        }
                        assert!(value.numerator > 0);
                        if theme_id == THEME_ID_FRACTION_ADDITION {
                            assert!(value.denominator > 1);
                            assert!(value.numerator <= 65);
                            assert!(value.denominator <= 72);
                        } else {
                            assert!(fraction_theme::operand_domain_v1().contains(&value));
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
        for registration in crate::registry::active_registrations() {
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
                    assert_eq!(first.problems.len(), registration.layout.problem_count);
                    for left in 0..first.problems.len() {
                        for right in left + 1..first.problems.len() {
                            assert_ne!(
                                first.problems[left].prompt,
                                first.problems[right].prompt,
                                "theme {} duplicated a prompt for seed {seed} difficulty {difficulty_value}",
                                registration.numeric_theme_id
                            );
                            assert_ne!(
                                problem_key(registration, &first.problems[left]),
                                problem_key(registration, &first.problems[right]),
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
            equation_theme::THEME_ID_LINEAR_EQUATION_1,
            equation_theme::THEME_ID_LINEAR_EQUATION_2,
            basic_theme::THEME_ID_SIGNED_ARITHMETIC_1,
            basic_theme::THEME_ID_SIGNED_ARITHMETIC_2,
            fraction_theme::THEME_ID_FRACTION_ADDITION,
            fraction_theme::THEME_ID_FRACTION_MULTIPLICATION,
            fraction_theme::THEME_ID_FRACTION_SUBTRACTION,
            fraction_theme::THEME_ID_FRACTION_DIVISION,
            equation_theme::THEME_ID_QUADRATIC_EQUATION_1,
            equation_theme::THEME_ID_QUADRATIC_EQUATION_2,
            equation_theme::THEME_ID_QUADRATIC_EQUATION_3,
            decimal_theme::THEME_ID_DECIMAL_ADD_SUBTRACT,
            decimal_theme::THEME_ID_DECIMAL_MULTIPLY_DIVIDE,
            equation_theme::THEME_ID_SIMULTANEOUS_EQUATION_1,
        ]);

        for registration in crate::registry::active_registrations() {
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
                        if let Some(special) = problem.theme_specific_effort {
                            assert!(special.is_finite() && special >= 0.0);
                            assert_eq!(problem.effort, special);
                            assert!(problem.solution_graph.steps.is_empty());
                            assert_eq!(problem.operation_vector, crate::effort::OperationVector::zero());
                        } else {
                            let expected = crate::registry::resolved_weights(registration)
                                .weighted_sum(&problem.operation_vector);
                            assert!((problem.effort - expected).abs() < 1e-12);
                        }
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
        for registration in crate::registry::active_registrations() {
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
                numeric_theme_id: decimal_theme::THEME_ID_DECIMAL_ADD_SUBTRACT,
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
    fn decimal_multiplication_and_division_are_independent_themes() {
        for (theme_id, expected_operator) in [
            (
                decimal_theme::THEME_ID_DECIMAL_MULTIPLY_DIVIDE,
                ArithmeticOperator::Multiply,
            ),
            (
                decimal_theme::THEME_ID_DECIMAL_DIVISION,
                ArithmeticOperator::Divide,
            ),
        ] {
            let mut saw_integer_second = false;
            let mut saw_decimal_second = false;
            for seed in ["A1b2", "M7x9", "Q4r6", "Z8k3", "D3c5", "N6p8"] {
                let worksheet = generate_worksheet_request(&GenerateWorksheetRequest {
                    schema_version: SCHEMA_VERSION,
                    numeric_theme_id: theme_id,
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
                        panic!("decimal theme must be binary arithmetic");
                    };
                    assert_eq!(*operator, expected_operator);
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
                    if expected_operator == ArithmeticOperator::Multiply {
                        assert_decimal_operand(left, 2, 2);
                        assert!(matches!(
                            problem.canonical_answer,
                            AnswerNode::Integer(_) | AnswerNode::ExactDecimal { .. }
                        ));
                    } else {
                        match problem.canonical_answer {
                            AnswerNode::ExactDecimal { coefficient, scale } => {
                                assert!(coefficient > 0);
                                assert!((1..=2).contains(&scale));
                            }
                            AnswerNode::Integer(value) => assert!(value > 0),
                            _ => {
                                panic!("reverse-generated decimal division quotient must be exact")
                            }
                        }
                    }
                }
            }
            assert!(saw_integer_second);
            assert!(saw_decimal_second);
        }
    }

    #[test]
    fn fraction_themes_allow_decimal_answers_for_configurable_fraction_form_grading() {
        for theme_id in [
            fraction_theme::THEME_ID_FRACTION_ADDITION,
            fraction_theme::THEME_ID_FRACTION_SUBTRACTION,
            fraction_theme::THEME_ID_FRACTION_MULTIPLICATION,
            fraction_theme::THEME_ID_FRACTION_DIVISION,
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
    fn fraction_and_integer_division_units_keep_operand_curricula_separate() {
        let mut saw_integer_left = false;
        let mut saw_integer_right = false;
        for seed in ["F1d2", "Ab3Z", "M7x9", "Qp5A"] {
            let standard = generate_worksheet_request(&GenerateWorksheetRequest {
                schema_version: SCHEMA_VERSION,
                numeric_theme_id: fraction_theme::THEME_ID_FRACTION_DIVISION,
                seed: seed.to_owned(),
                difficulty: crate::identity::Difficulty::try_from(3).unwrap(),
                timeout_ms: None,
                max_attempts: None,
            })
            .unwrap();
            for problem in standard.problems {
                let ProblemPrompt::Arithmetic {
                    expression: ArithmeticExpression::Binary { left, right, .. },
                } = problem.prompt
                else {
                    panic!("fraction division must generate binary arithmetic");
                };
                assert!(!matches!(*left, ArithmeticExpression::Integer { .. }));
                assert!(!matches!(*right, ArithmeticExpression::Integer { .. }));
            }

            let integer = generate_worksheet_request(&GenerateWorksheetRequest {
                schema_version: SCHEMA_VERSION,
                numeric_theme_id: fraction_theme::THEME_ID_FRACTION_INTEGER_DIVISION,
                seed: seed.to_owned(),
                difficulty: crate::identity::Difficulty::try_from(3).unwrap(),
                timeout_ms: None,
                max_attempts: None,
            })
            .unwrap();
            for problem in integer.problems {
                let ProblemPrompt::Arithmetic {
                    expression: ArithmeticExpression::Binary { left, right, .. },
                } = problem.prompt
                else {
                    panic!("fraction/integer division must generate binary arithmetic");
                };
                let is_integer_operand = |expression: &ArithmeticExpression| match expression {
                    ArithmeticExpression::Integer { .. } => true,
                    ArithmeticExpression::Rational { value } => value.is_integer(),
                    _ => false,
                };
                let left_integer = is_integer_operand(&left);
                let right_integer = is_integer_operand(&right);
                assert_ne!(left_integer, right_integer);
                saw_integer_left |= left_integer;
                saw_integer_right |= right_integer;
            }
        }
        assert!(saw_integer_left && saw_integer_right);
    }

    #[test]
    fn theme_specific_effort_stays_out_of_unrelated_operation_primitives() {
        assert_eq!(multiplication_table::effort(56), 56_f64.log10());

        let multiplication = generate_worksheet_request(&GenerateWorksheetRequest {
            schema_version: SCHEMA_VERSION,
            numeric_theme_id: basic_theme::THEME_ID_MULTIPLICATION_TABLE,
            seed: "Kuku56".to_owned(),
            difficulty: crate::identity::DEFAULT_DIFFICULTY,
            timeout_ms: None,
            max_attempts: None,
        }).unwrap();
        assert!(multiplication.problems.iter().all(|problem| {
            problem.theme_specific_effort == Some(problem.effort)
                && problem.solution_graph.steps.is_empty()
                && problem.operation_vector == crate::effort::OperationVector::zero()
        }));

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
                numeric_theme_id: basic_theme::THEME_ID_DIVISION_1,
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
                numeric_theme_id: equation_theme::THEME_ID_QUADRATIC_EQUATION_1,
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
        let domain = equation_theme::quadratic_one_answer_domain();
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
            let problem = equation_theme::QUADRATIC_EQUATION_1_GENERATOR
                .draw_candidate_for_answer(&mut rng, index as u32 + 1, &weights, answer)
                .expect("every declared quadratic(1) answer must construct a problem");
            assert_eq!(&problem.canonical_answer, answer);
        }
    }

    #[test]
    fn quadratic_two_is_reverse_generated_from_two_integer_roots() {
        let worksheet = generate_worksheet_request(&GenerateWorksheetRequest {
            schema_version: SCHEMA_VERSION,
            numeric_theme_id: equation_theme::THEME_ID_QUADRATIC_EQUATION_2,
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
            assert_eq!(a, RationalCoefficient::new(1, 1).unwrap());
            match problem.canonical_answer {
                AnswerNode::Integer(root) => {
                    assert_eq!(b, RationalCoefficient::new(-2 * root, 1).unwrap());
                    assert_eq!(c, RationalCoefficient::new(root * root, 1).unwrap());
                }
                AnswerNode::Tuple(roots) => {
                    assert_eq!(roots.len(), 2);
                    let (AnswerNode::Integer(r1), AnswerNode::Integer(r2)) = (&roots[0], &roots[1])
                    else {
                        panic!("quadratic(2) roots must be integers");
                    };
                    assert_ne!(r1, r2);
                    assert_eq!(b, RationalCoefficient::new(-(r1 + r2), 1).unwrap());
                    assert_eq!(c, RationalCoefficient::new(r1 * r2, 1).unwrap());
                }
                other => panic!("unexpected quadratic(2) answer {other:?}"),
            }
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
                numeric_theme_id: equation_theme::THEME_ID_QUADRATIC_EQUATION_3,
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
    fn hard_fraction_addition_worksheets_do_not_collapse_to_equal_denominators() {
        for seed in ["Ab3Z", "M7x9", "NwA", "Em7Z", "Qp5A"] {
            let worksheet = generate_worksheet_request(&GenerateWorksheetRequest {
                schema_version: SCHEMA_VERSION,
                numeric_theme_id: fraction_theme::THEME_ID_FRACTION_ADDITION,
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
        for registration in crate::registry::active_registrations()
            .into_iter()
            .filter(|registration| registration.grade.is_some_and(|grade| grade <= 6))
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
                numeric_theme_id: equation_theme::THEME_ID_SIMULTANEOUS_EQUATION_1,
                seed: seed.to_owned(),
                difficulty: crate::identity::Difficulty::try_from(3).unwrap(),
                timeout_ms: None,
                max_attempts: None,
            })
            .unwrap();
            assert_eq!(
                worksheet.problems.len(),
                equation_theme::SIMULTANEOUS_EQUATION_1_REGISTRATION
                    .layout
                    .problem_count
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
    fn liar_puzzle_is_not_a_layered_theme() {
        let generator = registered_generator(
            liar_puzzle_theme::THEME_ID_LIAR_PUZZLE,
            liar_puzzle_theme::GENERATOR_REVISION_LIAR_PUZZLE,
        )
        .expect("liar puzzle generator must be registered");
        assert!(generator.sampling_layers().is_none());
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
                numeric_theme_id: liar_puzzle_theme::THEME_ID_LIAR_PUZZLE,
                seed: seed.to_owned(),
                difficulty: crate::identity::Difficulty::try_from(2).unwrap(),
                timeout_ms: Some(1_000),
                max_attempts: Some(50_000),
            })
            .unwrap();
            assert_eq!(
                worksheet.problems.len(),
                liar_puzzle_theme::LIAR_PUZZLE_REGISTRATION
                    .layout
                    .problem_count
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

                let solutions = liar_puzzle_theme::solutions(*people_count, statements);
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
        assert!(liar_puzzle_theme::statement_truth(
            &LiarStatement::SaysLiar { person: 1 },
            mask
        ));
        assert!(liar_puzzle_theme::statement_truth(
            &LiarStatement::SaysNotLiar { person: 2 },
            mask
        ));
        assert!(liar_puzzle_theme::statement_truth(
            &LiarStatement::ExactlyOneLiar {
                first: 1,
                second: 2
            },
            mask
        ));
        assert!(liar_puzzle_theme::statement_truth(
            &LiarStatement::ExactLiarCount { count: 2 },
            mask
        ));
        assert!(liar_puzzle_theme::statement_truth(
            &LiarStatement::BothLiar {
                first: 1,
                second: 3
            },
            mask
        ));
        assert!(liar_puzzle_theme::statement_truth(
            &LiarStatement::BothNotLiar {
                first: 2,
                second: 4
            },
            mask
        ));
        assert!(liar_puzzle_theme::statement_truth(
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

    fn column_leaf_value(expression: &ArithmeticExpression) -> RationalCoefficient {
        match expression {
            ArithmeticExpression::Integer { value } => RationalCoefficient::new(*value, 1).unwrap(),
            ArithmeticExpression::ExactDecimal { coefficient, scale } => {
                crate::generator_support::exact_decimal_rational(*coefficient, *scale).unwrap()
            }
            other => panic!("column arithmetic must use a scalar display operand: {other:?}"),
        }
    }

    fn answer_integer(answer: &AnswerNode) -> i64 {
        let AnswerNode::Integer(value) = answer else {
            panic!("expected integer answer, got {answer:?}");
        };
        *value
    }

    fn quotient_remainder(answer: &AnswerNode) -> (i64, i64) {
        let AnswerNode::Tuple(values) = answer else {
            panic!("column division answer must be an ordered pair");
        };
        assert_eq!(values.len(), 2);
        (answer_integer(&values[0]), answer_integer(&values[1]))
    }

    #[test]
    fn column_arithmetic_themes_follow_curriculum_domains_and_print_layouts() {
        use crate::themes::column_arithmetic::*;
        const IDS: [u32; 13] = [
            THEME_ID_COLUMN_ADD_2DIGIT,
            THEME_ID_COLUMN_SUBTRACT_2DIGIT,
            THEME_ID_COLUMN_ADD_3_4DIGIT,
            THEME_ID_COLUMN_SUBTRACT_3_4DIGIT,
            THEME_ID_COLUMN_MULTIPLY_1DIGIT,
            THEME_ID_COLUMN_MULTIPLY_2DIGIT,
            THEME_ID_COLUMN_DIVIDE_1DIGIT,
            THEME_ID_COLUMN_DIVIDE_2DIGIT,
            THEME_ID_COLUMN_DECIMAL_ADD_SUBTRACT,
            THEME_ID_COLUMN_DECIMAL_MULTIPLY_INTEGER,
            THEME_ID_COLUMN_DECIMAL_DIVIDE_INTEGER,
            THEME_ID_COLUMN_DECIMAL_MULTIPLICATION,
            THEME_ID_COLUMN_DECIMAL_DIVISION,
        ];
        let seeds = ["CoA1", "CoB2", "CoC3", "CoD4", "CoE5", "CoF6"];

        for theme_id in IDS {
            let registration = crate::registry::active_registration(theme_id).unwrap();
            let is_division = matches!(
                theme_id,
                THEME_ID_COLUMN_DIVIDE_1DIGIT
                    | THEME_ID_COLUMN_DIVIDE_2DIGIT
                    | THEME_ID_COLUMN_DECIMAL_DIVIDE_INTEGER
                    | THEME_ID_COLUMN_DECIMAL_DIVISION
            );
            let (expected_count, expected_columns, expected_rows) =
                if is_division { (12, 4, 3) } else { (16, 4, 4) };
            assert_eq!(registration.layout.problem_count, expected_count);
            assert_eq!(
                (registration.layout.columns, registration.layout.rows),
                (expected_columns, expected_rows)
            );
            for seed in seeds {
                let worksheet = generate_worksheet_request(&GenerateWorksheetRequest {
                    schema_version: SCHEMA_VERSION,
                    numeric_theme_id: theme_id,
                    seed: seed.to_owned(),
                    difficulty: crate::identity::Difficulty::try_from(4).unwrap(),
                    timeout_ms: Some(1_000),
                    max_attempts: Some(50_000),
                })
                .unwrap_or_else(|error| {
                    panic!("column theme {theme_id} failed for {seed}: {error}")
                });
                assert_eq!(worksheet.layout.problem_count, expected_count);
                assert_eq!(
                    (worksheet.layout.columns, worksheet.layout.rows),
                    (expected_columns, expected_rows)
                );
                assert_eq!(worksheet.problems.len(), expected_count);

                for problem in worksheet.problems {
                    let ProblemPrompt::ColumnArithmetic {
                        operator,
                        left,
                        right,
                    } = &problem.prompt
                    else {
                        panic!("column theme {theme_id} returned a non-column prompt");
                    };
                    let left_value = column_leaf_value(left);
                    let right_value = column_leaf_value(right);
                    assert!(left_value.numerator >= 0 && right_value.numerator >= 0);

                    let expression = crate::generator_support::binary_expression(
                        *operator,
                        left.clone(),
                        right.clone(),
                    );
                    let expected = crate::generator_support::evaluate_expression(&expression)
                        .expect("column expression evaluates exactly");
                    match theme_id {
                        THEME_ID_COLUMN_ADD_2DIGIT => {
                            assert_eq!(*operator, ArithmeticOperator::Add);
                            assert!((10..=99).contains(&left_value.numerator));
                            assert!((10..=99).contains(&right_value.numerator));
                            assert_eq!(left_value.denominator, 1);
                            assert_eq!(right_value.denominator, 1);
                        }
                        THEME_ID_COLUMN_SUBTRACT_2DIGIT => {
                            assert_eq!(*operator, ArithmeticOperator::Subtract);
                            assert!((10..=99).contains(&left_value.numerator));
                            assert!((10..=99).contains(&right_value.numerator));
                            assert!(left_value.numerator >= right_value.numerator);
                        }
                        THEME_ID_COLUMN_ADD_3_4DIGIT | THEME_ID_COLUMN_SUBTRACT_3_4DIGIT => {
                            assert!(matches!(
                                operator,
                                ArithmeticOperator::Add | ArithmeticOperator::Subtract
                            ));
                            assert!((100..=9_999).contains(&left_value.numerator));
                            assert!((100..=9_999).contains(&right_value.numerator));
                            if *operator == ArithmeticOperator::Subtract {
                                assert!(left_value.numerator >= right_value.numerator);
                            }
                        }
                        THEME_ID_COLUMN_MULTIPLY_1DIGIT => {
                            assert_eq!(*operator, ArithmeticOperator::Multiply);
                            assert!((10..=999).contains(&left_value.numerator));
                            assert!((2..=9).contains(&right_value.numerator));
                        }
                        THEME_ID_COLUMN_MULTIPLY_2DIGIT => {
                            assert_eq!(*operator, ArithmeticOperator::Multiply);
                            assert!((10..=999).contains(&left_value.numerator));
                            assert!((10..=99).contains(&right_value.numerator));
                        }
                        THEME_ID_COLUMN_DIVIDE_1DIGIT | THEME_ID_COLUMN_DIVIDE_2DIGIT => {
                            assert_eq!(*operator, ArithmeticOperator::Divide);
                            assert_eq!(left_value.denominator, 1);
                            assert_eq!(right_value.denominator, 1);
                            let divisor = right_value.numerator;
                            if theme_id == THEME_ID_COLUMN_DIVIDE_1DIGIT {
                                assert!((2..=9).contains(&divisor));
                            } else {
                                assert!((10..=99).contains(&divisor));
                            }
                            let (quotient, remainder) =
                                quotient_remainder(&problem.canonical_answer);
                            if theme_id == THEME_ID_COLUMN_DIVIDE_1DIGIT {
                                assert!((10..=99).contains(&quotient));
                            } else {
                                assert!((2..=99).contains(&quotient));
                            }
                            assert!((0..divisor).contains(&remainder));
                            assert_eq!(left_value.numerator, divisor * quotient + remainder);
                            assert_eq!(problem.answer_schema, AnswerSchema::OrderedPair);
                            continue;
                        }
                        THEME_ID_COLUMN_DECIMAL_ADD_SUBTRACT => {
                            assert!(matches!(
                                operator,
                                ArithmeticOperator::Add | ArithmeticOperator::Subtract
                            ));
                            assert!(matches!(left, ArithmeticExpression::ExactDecimal { .. }));
                            assert!(matches!(right, ArithmeticExpression::ExactDecimal { .. }));
                            if *operator == ArithmeticOperator::Subtract {
                                assert!(!crate::generator_support::rational_less_than(
                                    left_value,
                                    right_value
                                ));
                            }
                        }
                        THEME_ID_COLUMN_DECIMAL_MULTIPLY_INTEGER => {
                            assert_eq!(*operator, ArithmeticOperator::Multiply);
                            assert!(matches!(left, ArithmeticExpression::ExactDecimal { .. }));
                            assert!((2..=9).contains(&right_value.numerator));
                            assert_eq!(right_value.denominator, 1);
                        }
                        THEME_ID_COLUMN_DECIMAL_DIVIDE_INTEGER => {
                            assert_eq!(*operator, ArithmeticOperator::Divide);
                            assert!((2..=9).contains(&right_value.numerator));
                            assert_eq!(right_value.denominator, 1);
                        }
                        THEME_ID_COLUMN_DECIMAL_MULTIPLICATION => {
                            assert_eq!(*operator, ArithmeticOperator::Multiply);
                            assert!(matches!(left, ArithmeticExpression::ExactDecimal { .. }));
                            assert!(matches!(right, ArithmeticExpression::ExactDecimal { .. }));
                        }
                        THEME_ID_COLUMN_DECIMAL_DIVISION => {
                            assert_eq!(*operator, ArithmeticOperator::Divide);
                            assert!(matches!(right, ArithmeticExpression::ExactDecimal { .. }));
                            assert!(
                                crate::generator_support::arithmetic_leaf_column_grid_cells(left)
                                    .unwrap()
                                    + crate::generator_support::arithmetic_leaf_column_grid_cells(
                                        right
                                    )
                                    .unwrap()
                                    <= 6,
                                "decimal column division must fit the printable long-division grid"
                            );
                        }
                        _ => unreachable!(),
                    }
                    assert_eq!(
                        crate::normalize::normalize_answer(
                            &crate::generator_support::rational_answer(expected)
                        ),
                        crate::normalize::normalize_answer(&problem.canonical_answer),
                        "theme {theme_id} canonical answer disagrees with its displayed operands"
                    );
                }
            }
        }
    }

    #[test]
    fn column_arithmetic_difficulty_tracks_shared_scalar_effort() {
        use crate::themes::column_arithmetic::*;
        const IDS: [u32; 13] = [
            THEME_ID_COLUMN_ADD_2DIGIT,
            THEME_ID_COLUMN_SUBTRACT_2DIGIT,
            THEME_ID_COLUMN_ADD_3_4DIGIT,
            THEME_ID_COLUMN_SUBTRACT_3_4DIGIT,
            THEME_ID_COLUMN_MULTIPLY_1DIGIT,
            THEME_ID_COLUMN_MULTIPLY_2DIGIT,
            THEME_ID_COLUMN_DIVIDE_1DIGIT,
            THEME_ID_COLUMN_DIVIDE_2DIGIT,
            THEME_ID_COLUMN_DECIMAL_ADD_SUBTRACT,
            THEME_ID_COLUMN_DECIMAL_MULTIPLY_INTEGER,
            THEME_ID_COLUMN_DECIMAL_DIVIDE_INTEGER,
            THEME_ID_COLUMN_DECIMAL_MULTIPLICATION,
            THEME_ID_COLUMN_DECIMAL_DIVISION,
        ];
        const SEEDS: [&str; 8] = [
            "EfA1", "EfB2", "EfC3", "EfD4", "EfE5", "EfF6", "EfG7", "EfH8",
        ];

        for theme_id in IDS {
            let mut means = [0.0_f64; 3];
            for difficulty_value in 1_u8..=3 {
                let mut total = 0.0;
                let mut count = 0_usize;
                for seed in SEEDS {
                    let worksheet = generate_worksheet_request(&GenerateWorksheetRequest {
                        schema_version: SCHEMA_VERSION,
                        numeric_theme_id: theme_id,
                        seed: seed.to_owned(),
                        difficulty: crate::identity::Difficulty::try_from(difficulty_value)
                            .unwrap(),
                        timeout_ms: Some(1_000),
                        max_attempts: Some(50_000),
                    })
                    .unwrap_or_else(|error| {
                        panic!("theme {theme_id} difficulty {difficulty_value} failed: {error}")
                    });
                    total += worksheet
                        .problems
                        .iter()
                        .map(|problem| problem.effort)
                        .sum::<f64>();
                    count += worksheet.problems.len();
                }
                means[(difficulty_value - 1) as usize] = total / count as f64;
            }
            assert!(
                means[0] <= means[1] && means[1] <= means[2],
                "column theme {theme_id} lost scalar effort separation: {means:?}"
            );
        }
    }

    #[test]
    fn layered_themes_enforce_declared_quotas_at_every_difficulty() {
        assert_eq!(
            layered_quotas(&equation_theme::QUADRATIC_FACTORING_LAYERS, 16),
            vec![2, 2, 12]
        );
        assert_eq!(
            layered_quotas(&equation_theme::QUADRATIC_FACTORING_LAYERS, 20),
            vec![2, 2, 16]
        );
        let cases: &[(u32, &dyn ProblemGenerator, &[usize])] = &[
            (
                decimal_theme::THEME_ID_DECIMAL_ADD_SUBTRACT,
                &decimal_theme::ADD_SUBTRACT_GENERATOR,
                &[10, 10],
            ),
            (
                column_theme::THEME_ID_COLUMN_DECIMAL_ADD_SUBTRACT,
                &column_theme::DECIMAL_ADD_SUBTRACT_GENERATOR,
                &[8, 8],
            ),
            (
                fraction_theme::THEME_ID_FRACTION_SUMMARY_IMPROPER,
                &fraction_theme::SUMMARY_IMPROPER_GENERATOR,
                &[4, 4, 4, 4],
            ),
            (
                equation_theme::THEME_ID_QUADRATIC_EQUATION_2,
                &equation_theme::QUADRATIC_EQUATION_2_GENERATOR,
                &[2, 2, 12],
            ),
        ];
        for &(theme_id, generator, expected) in cases {
            for difficulty_value in 1_u8..=4 {
                for seed in ["LyrA1", "LyrB2", "LyrC3"] {
                    let worksheet = generate_worksheet_request(&GenerateWorksheetRequest {
                        schema_version: SCHEMA_VERSION,
                        numeric_theme_id: theme_id,
                        seed: seed.to_owned(),
                        difficulty: crate::identity::Difficulty::try_from(difficulty_value)
                            .unwrap(),
                        timeout_ms: Some(1_000),
                        max_attempts: Some(50_000),
                    })
                    .unwrap();
                    let mut counts = vec![0_usize; expected.len()];
                    for problem in &worksheet.problems {
                        let layer = generator
                            .sampling_layer(problem)
                            .expect("layered problem must classify");
                        counts[layer] += 1;
                    }
                    assert_eq!(
                        counts, expected,
                        "theme {theme_id} difficulty {difficulty_value}"
                    );
                }
            }
        }
    }

    #[test]
    fn layered_difficulty_selection_remains_scalar_within_each_layer() {
        let cases: &[(u32, &dyn ProblemGenerator)] = &[
            (
                decimal_theme::THEME_ID_DECIMAL_ADD_SUBTRACT,
                &decimal_theme::ADD_SUBTRACT_GENERATOR,
            ),
            (
                column_theme::THEME_ID_COLUMN_DECIMAL_ADD_SUBTRACT,
                &column_theme::DECIMAL_ADD_SUBTRACT_GENERATOR,
            ),
            (
                fraction_theme::THEME_ID_FRACTION_SUMMARY_IMPROPER,
                &fraction_theme::SUMMARY_IMPROPER_GENERATOR,
            ),
            (
                equation_theme::THEME_ID_QUADRATIC_EQUATION_2,
                &equation_theme::QUADRATIC_EQUATION_2_GENERATOR,
            ),
        ];
        let seeds = ["DfA1", "DfB2", "DfC3", "DfD4", "DfE5", "DfF6"];
        for &(theme_id, generator) in cases {
            let layer_count = generator.sampling_layers().unwrap().len();
            let mut means = vec![vec![0.0_f64; layer_count]; 3];
            let mut counts = vec![vec![0_usize; layer_count]; 3];
            for difficulty_value in 1_u8..=3 {
                for seed in seeds {
                    let worksheet = generate_worksheet_request(&GenerateWorksheetRequest {
                        schema_version: SCHEMA_VERSION,
                        numeric_theme_id: theme_id,
                        seed: seed.to_owned(),
                        difficulty: crate::identity::Difficulty::try_from(difficulty_value)
                            .unwrap(),
                        timeout_ms: Some(1_000),
                        max_attempts: Some(50_000),
                    })
                    .unwrap();
                    for problem in &worksheet.problems {
                        let layer = generator.sampling_layer(problem).unwrap();
                        means[(difficulty_value - 1) as usize][layer] += problem.effort;
                        counts[(difficulty_value - 1) as usize][layer] += 1;
                    }
                }
            }
            for difficulty in 0..3 {
                for layer in 0..layer_count {
                    means[difficulty][layer] /= counts[difficulty][layer] as f64;
                }
            }
            for (layer, _) in means[0].iter().enumerate().take(layer_count) {
                assert!(
                    means[0][layer] <= means[1][layer] && means[1][layer] <= means[2][layer],
                    "theme {theme_id} layer {layer} lost scalar effort separation: {:?}",
                    [means[0][layer], means[1][layer], means[2][layer]]
                );
            }
        }
    }

    #[test]
    fn signed_arithmetic_two_is_no_longer_division_starved() {
        fn operator_index(operator: ArithmeticOperator) -> usize {
            match operator {
                ArithmeticOperator::Add => 0,
                ArithmeticOperator::Subtract => 1,
                ArithmeticOperator::Multiply => 2,
                ArithmeticOperator::Divide => 3,
            }
        }

        let mut rng = DeterministicRng::from_seed("SignedStats");
        let weights = resolved_weights(&basic_theme::SIGNED_ARITHMETIC_2_REGISTRATION);
        let mut raw_counts = [0_usize; 4];
        let mut raw_answers = [0_usize; 2];
        let mut raw_efforts = Vec::new();
        let mut accepted = 0_u32;
        let mut ordinal = 1_u32;
        while accepted < 5_000 {
            if let Some(problem) = basic_theme::SIGNED_ARITHMETIC_2_GENERATOR
                .draw_candidate(&mut rng, ordinal, &weights)
            {
                let ProblemPrompt::Arithmetic { expression } = &problem.prompt else {
                    unreachable!()
                };
                for operator in expression_operators(expression) {
                    raw_counts[operator_index(operator)] += 1;
                }
                match problem.canonical_answer {
                    AnswerNode::Integer(_) => raw_answers[0] += 1,
                    AnswerNode::Fraction { .. } => raw_answers[1] += 1,
                    _ => panic!("signed arithmetic answer must be exact rational"),
                }
                let value = crate::generator_support::evaluate_expression(expression).unwrap();
                assert!(value.numerator.unsigned_abs() <= 200 && value.denominator <= 36);
                raw_efforts.push(problem.effort);
                accepted += 1;
            }
            ordinal += 1;
        }
        let raw_total: usize = raw_counts.iter().sum();
        let raw_division_ratio = raw_counts[3] as f64 / raw_total as f64;
        assert!(
            raw_division_ratio >= 0.15,
            "raw division ratio remained starved: {raw_division_ratio}"
        );
        assert!(raw_answers[1] > 0, "fractional answers should now occur");
        raw_efforts.sort_by(f64::total_cmp);
        eprintln!(
            "SIGNED2_RAW operators={raw_counts:?} division_ratio={raw_division_ratio:.4} answers(integer,fraction)={raw_answers:?} effort(min,median,max)=({:.3},{:.3},{:.3})",
            raw_efforts[0],
            raw_efforts[raw_efforts.len() / 2],
            raw_efforts[raw_efforts.len() - 1]
        );

        let seeds = [
            "SgA1", "SgB2", "SgC3", "SgD4", "SgE5", "SgF6", "SgG7", "SgH8", "SgJ9", "SgK1",
        ];
        for difficulty_value in 1_u8..=4 {
            let mut counts = [0_usize; 4];
            let mut answers = [0_usize; 2];
            let mut efforts = Vec::new();
            for seed in seeds {
                let worksheet = generate_worksheet_request(&GenerateWorksheetRequest {
                    schema_version: SCHEMA_VERSION,
                    numeric_theme_id: basic_theme::THEME_ID_SIGNED_ARITHMETIC_2,
                    seed: seed.to_owned(),
                    difficulty: crate::identity::Difficulty::try_from(difficulty_value).unwrap(),
                    timeout_ms: Some(1_000),
                    max_attempts: Some(50_000),
                })
                .unwrap();
                for problem in worksheet.problems {
                    let ProblemPrompt::Arithmetic { expression } = &problem.prompt else {
                        unreachable!()
                    };
                    for operator in expression_operators(expression) {
                        counts[operator_index(operator)] += 1;
                    }
                    match problem.canonical_answer {
                        AnswerNode::Integer(_) => answers[0] += 1,
                        AnswerNode::Fraction { .. } => answers[1] += 1,
                        _ => panic!("signed arithmetic answer must be rational"),
                    }
                    efforts.push(problem.effort);
                }
            }
            let total: usize = counts.iter().sum();
            let division_ratio = counts[3] as f64 / total as f64;
            assert!(
                division_ratio >= 0.05,
                "difficulty {difficulty_value} eliminated division: {division_ratio}"
            );
            efforts.sort_by(f64::total_cmp);
            eprintln!(
                "SIGNED2_D{difficulty_value} operators={counts:?} division_ratio={division_ratio:.4} answers(integer,fraction)={answers:?} effort(min,median,max)=({:.3},{:.3},{:.3})",
                efforts[0], efforts[efforts.len()/2], efforts[efforts.len()-1]
            );
        }
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
                basic_theme::THEME_ID_ONE_DIGIT_ADDITION,
                basic_theme::GENERATOR_REVISION_ONE_DIGIT_ADDITION,
                "Dvrs",
                crate::identity::DEFAULT_DIFFICULTY,
            ).unwrap();
            let max_attempts = 100 + extra;
            let config = GenerationConfig::default().with_max_attempts(max_attempts);
            let clock = StepClock::new(Duration::ZERO, Duration::ZERO);
            let error = generate_with_generator(
                &identity,
                &basic_theme::ONE_DIGIT_ADDITION_REGISTRATION,
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
