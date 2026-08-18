#[cfg(test)]
use std::cell::Cell;
use std::collections::HashSet;
use std::rc::Rc;
use std::time::{Duration, Instant};

use crate::answer::AnswerNode;
use crate::effort::OperationWeights;
use crate::error::{GenerationError, SamplingError};
use crate::identity::{validate_seed, ProblemSetIdentity};
use crate::model::{
    AnswerInputInterface, ArithmeticExpression, ArithmeticOperator, EditorStructure,
    GenerateProblemRequest, GenerateWorksheetRequest, Problem, ProblemPrompt, Worksheet,
};
use crate::registry::{active_registration, registration};
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

pub(crate) trait ProblemGenerator: Sync {
    fn registration(&self) -> &'static ThemeRegistration;
    fn sampling_strategy(&self) -> Result<SamplingStrategy<'_>, SamplingError>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BootstrapDedup {
    AllowDuplicates,
    Deduplicate,
}

#[derive(Clone, Copy)]
struct AnswerDomain {
    values: &'static [AnswerNode],
}

impl AnswerDomain {
    fn new(values: &'static [AnswerNode]) -> Result<Self, SamplingError> {
        if values.is_empty() {
            Err(SamplingError::EmptyAnswerDomain)
        } else {
            Ok(Self { values })
        }
    }

    fn choose<'a>(&'a self, rng: &mut DeterministicRng) -> &'a AnswerNode {
        &self.values[rng.next_bounded(self.values.len() as u64) as usize]
    }
}

#[derive(Clone, Copy)]
struct SamplingLayers {
    specs: &'static [SamplingLayerSpec],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct LayerIndex(usize);

impl LayerIndex {
    const fn value(self) -> usize {
        self.0
    }
}

impl SamplingLayers {
    fn new(
        specs: &'static [SamplingLayerSpec],
        problem_count: usize,
    ) -> Result<Self, SamplingError> {
        if specs.is_empty() {
            return Err(SamplingError::EmptyLayers);
        }
        let minimum_total = specs.iter().map(|layer| layer.minimum).sum::<usize>();
        if minimum_total > problem_count {
            return Err(SamplingError::LayerMinimumExceedsWorksheet {
                minimum_total,
                problem_count,
            });
        }
        Ok(Self { specs })
    }

    const fn specs(self) -> &'static [SamplingLayerSpec] {
        self.specs
    }

    fn index(self, index: usize) -> Result<LayerIndex, SamplingError> {
        if index < self.specs.len() {
            Ok(LayerIndex(index))
        } else {
            Err(SamplingError::LayerOutOfRange {
                index,
                layer_count: self.specs.len(),
            })
        }
    }

    fn choose(self, rng: &mut DeterministicRng) -> LayerIndex {
        LayerIndex(rng.next_bounded(self.specs.len() as u64) as usize)
    }
}

enum SamplingStrategyKind<'a> {
    Random {
        source: &'a dyn RandomCandidateSource,
        bootstrap_dedup: BootstrapDedup,
    },
    AnswerConditioned {
        source: &'a dyn AnswerConditionedCandidateSource,
        domain: AnswerDomain,
    },
    Layered {
        source: &'a dyn LayeredCandidateSource,
        layers: SamplingLayers,
        bootstrap_dedup: BootstrapDedup,
    },
    ConstructiveLayered {
        source: &'a dyn ConstructiveLayeredCandidateSource,
        layers: SamplingLayers,
        bootstrap_dedup: BootstrapDedup,
        bootstrap_multiplier: std::num::NonZeroUsize,
    },
}

pub(crate) struct SamplingStrategy<'a> {
    kind: SamplingStrategyKind<'a>,
}

impl<'a> SamplingStrategy<'a> {
    pub(crate) fn random(
        source: &'a dyn RandomCandidateSource,
        bootstrap_dedup: BootstrapDedup,
    ) -> Self {
        Self {
            kind: SamplingStrategyKind::Random {
                source,
                bootstrap_dedup,
            },
        }
    }

    pub(crate) fn answer_conditioned(
        source: &'a dyn AnswerConditionedCandidateSource,
    ) -> Result<Self, SamplingError> {
        Ok(Self {
            kind: SamplingStrategyKind::AnswerConditioned {
                source,
                domain: AnswerDomain::new(source.answer_domain())?,
            },
        })
    }

    pub(crate) fn layered(
        source: &'a dyn LayeredCandidateSource,
        bootstrap_dedup: BootstrapDedup,
        problem_count: usize,
    ) -> Result<Self, SamplingError> {
        Ok(Self {
            kind: SamplingStrategyKind::Layered {
                source,
                layers: SamplingLayers::new(source.layers(), problem_count)?,
                bootstrap_dedup,
            },
        })
    }

    pub(crate) fn constructive_layered(
        source: &'a dyn ConstructiveLayeredCandidateSource,
        bootstrap_dedup: BootstrapDedup,
        bootstrap_multiplier: usize,
        problem_count: usize,
    ) -> Result<Self, SamplingError> {
        let bootstrap_multiplier = std::num::NonZeroUsize::new(bootstrap_multiplier)
            .ok_or(SamplingError::ZeroBootstrapMultiplier)?;
        Ok(Self {
            kind: SamplingStrategyKind::ConstructiveLayered {
                source,
                layers: SamplingLayers::new(source.layers(), problem_count)?,
                bootstrap_dedup,
                bootstrap_multiplier,
            },
        })
    }

    pub(crate) fn is_layered(&self) -> bool {
        self.layers().is_some()
    }

    fn layers(&self) -> Option<SamplingLayers> {
        match self.kind {
            SamplingStrategyKind::Layered { layers, .. }
            | SamplingStrategyKind::ConstructiveLayered { layers, .. } => Some(layers),
            SamplingStrategyKind::Random { .. }
            | SamplingStrategyKind::AnswerConditioned { .. } => None,
        }
    }

    fn bootstrap_dedup(&self) -> BootstrapDedup {
        match self.kind {
            SamplingStrategyKind::Random {
                bootstrap_dedup, ..
            }
            | SamplingStrategyKind::Layered {
                bootstrap_dedup, ..
            }
            | SamplingStrategyKind::ConstructiveLayered {
                bootstrap_dedup, ..
            } => bootstrap_dedup,
            SamplingStrategyKind::AnswerConditioned { .. } => BootstrapDedup::AllowDuplicates,
        }
    }

    fn bootstrap_multiplier(&self) -> usize {
        match self.kind {
            SamplingStrategyKind::Layered { layers, .. } => layers.specs().len(),
            SamplingStrategyKind::ConstructiveLayered {
                bootstrap_multiplier,
                ..
            } => bootstrap_multiplier.get(),
            SamplingStrategyKind::Random { .. }
            | SamplingStrategyKind::AnswerConditioned { .. } => 1,
        }
    }

    fn layer_of(&self, problem: &Problem) -> Result<Option<LayerIndex>, SamplingError> {
        match self.kind {
            SamplingStrategyKind::Layered { source, layers, .. } => {
                layers.index(source.layer_of(problem)).map(Some)
            }
            SamplingStrategyKind::ConstructiveLayered { source, layers, .. } => {
                layers.index(source.layer_of(problem)).map(Some)
            }
            SamplingStrategyKind::Random { .. }
            | SamplingStrategyKind::AnswerConditioned { .. } => Ok(None),
        }
    }

    fn validate_candidate_contract(
        &self,
        requested_answer: Option<&AnswerNode>,
        requested_layer: Option<LayerIndex>,
        problem: &Problem,
    ) -> Result<(), SamplingError> {
        match self.kind {
            SamplingStrategyKind::AnswerConditioned { .. } => {
                let expected = requested_answer.ok_or(SamplingError::EmptyAnswerDomain)?;
                if problem.canonical_answer() != expected {
                    return Err(SamplingError::AnswerConditionMismatch);
                }
            }
            SamplingStrategyKind::Layered { source, layers, .. } => {
                layers.index(source.layer_of(problem))?;
            }
            SamplingStrategyKind::ConstructiveLayered { source, layers, .. } => {
                let requested = requested_layer.ok_or(SamplingError::EmptyLayers)?;
                let actual = layers.index(source.layer_of(problem))?;
                if actual != requested {
                    return Err(SamplingError::RequestedLayerMismatch {
                        requested: requested.value(),
                        actual: actual.value(),
                    });
                }
            }
            SamplingStrategyKind::Random { .. } => {}
        }
        Ok(())
    }
}

pub(crate) trait RandomCandidateSource: Sync {
    fn draw_candidate(
        &self,
        rng: &mut DeterministicRng,
        ordinal: u32,
        weights: &OperationWeights,
    ) -> Result<Option<Problem>, GenerationError>;
}

pub(crate) trait AnswerConditionedCandidateSource: Sync {
    fn answer_domain(&self) -> &'static [AnswerNode];
    fn draw_candidate_for_answer(
        &self,
        rng: &mut DeterministicRng,
        ordinal: u32,
        weights: &OperationWeights,
        answer: &AnswerNode,
    ) -> Result<Option<Problem>, GenerationError>;
}

pub(crate) trait LayeredCandidateSource: Sync {
    fn layers(&self) -> &'static [SamplingLayerSpec];
    fn draw_candidate(
        &self,
        rng: &mut DeterministicRng,
        ordinal: u32,
        weights: &OperationWeights,
    ) -> Result<Option<Problem>, GenerationError>;
    fn layer_of(&self, problem: &Problem) -> usize;
}

pub(crate) trait ConstructiveLayeredCandidateSource: Sync {
    fn layers(&self) -> &'static [SamplingLayerSpec];
    fn layer_of(&self, problem: &Problem) -> usize;
    fn draw_candidate_for_layer(
        &self,
        rng: &mut DeterministicRng,
        ordinal: u32,
        weights: &OperationWeights,
        layer: usize,
    ) -> Result<Option<Problem>, GenerationError>;
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

pub(crate) fn registered_generator(
    numeric_theme_id: u32,
    generator_revision: u32,
) -> Result<Option<&'static dyn ProblemGenerator>, crate::registry::RegistryError> {
    crate::registry::generator_for_revision(numeric_theme_id, generator_revision)
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

#[cfg(test)]
#[derive(Debug)]
pub(crate) struct StepClock {
    current: Cell<Duration>,
    step: Duration,
}

#[cfg(test)]
impl StepClock {
    pub(crate) fn new(start: Duration, step: Duration) -> Self {
        Self {
            current: Cell::new(start),
            step,
        }
    }
}

#[cfg(test)]
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
        active_registration(request.numeric_theme_id)?.ok_or(GenerationError::UnknownTheme {
            numeric_theme_id: request.numeric_theme_id,
        })?;
    let generator = registered_generator(
        registration.numeric_theme_id(),
        registration.generator_revision(),
    )?
    .ok_or(GenerationError::UnknownGeneratorRevision {
        numeric_theme_id: registration.numeric_theme_id(),
        generator_revision: registration.generator_revision(),
    })?;
    let mut rng = DeterministicRng::from_seed(&request.seed);
    let weights = OperationWeights::default();
    let strategy = generator.sampling_strategy()?;
    let fixed_answer = match &strategy.kind {
        SamplingStrategyKind::AnswerConditioned { domain, .. } => Some(domain.choose(&mut rng)),
        _ => None,
    };
    for ordinal in 1..=DEFAULT_MAX_ATTEMPTS {
        let (problem, requested_layer) = match &strategy.kind {
            SamplingStrategyKind::Random { source, .. } => (
                source.draw_candidate(&mut rng, ordinal as u32, &weights)?,
                None,
            ),
            SamplingStrategyKind::Layered { source, .. } => (
                source.draw_candidate(&mut rng, ordinal as u32, &weights)?,
                None,
            ),
            SamplingStrategyKind::AnswerConditioned { source, .. } => {
                let answer = fixed_answer.ok_or(SamplingError::EmptyAnswerDomain)?;
                (
                    source.draw_candidate_for_answer(&mut rng, ordinal as u32, &weights, answer)?,
                    None,
                )
            }
            SamplingStrategyKind::ConstructiveLayered { source, layers, .. } => {
                let layer = layers.choose(&mut rng);
                (
                    source.draw_candidate_for_layer(
                        &mut rng,
                        ordinal as u32,
                        &weights,
                        layer.value(),
                    )?,
                    Some(layer),
                )
            }
        };
        if let Some(problem) = problem {
            strategy.validate_candidate_contract(fixed_answer, requested_layer, &problem)?;
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
        active_registration(request.numeric_theme_id)?.ok_or(GenerationError::UnknownTheme {
            numeric_theme_id: request.numeric_theme_id,
        })?;
    let identity = ProblemSetIdentity::new(
        registration.numeric_theme_id(),
        registration.generator_revision(),
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
    if identity.schema_version() != SCHEMA_VERSION {
        return Err(GenerationError::UnsupportedSchemaVersion {
            received: identity.schema_version(),
            expected: SCHEMA_VERSION,
        });
    }
    let registration = registration(identity.numeric_theme_id(), identity.generator_revision())?
        .ok_or(GenerationError::UnknownGeneratorRevision {
            numeric_theme_id: identity.numeric_theme_id(),
            generator_revision: identity.generator_revision(),
        })?;
    let generator =
        registered_generator(identity.numeric_theme_id(), identity.generator_revision())?.ok_or(
            GenerationError::UnknownGeneratorRevision {
                numeric_theme_id: identity.numeric_theme_id(),
                generator_revision: identity.generator_revision(),
            },
        )?;
    generate_with_generator(identity, registration, generator, config, clock)
}

fn layered_quotas(layers: SamplingLayers, problem_count: usize) -> Vec<usize> {
    let specs = layers.specs();
    let minimum_total: usize = specs.iter().map(|layer| layer.minimum).sum();
    debug_assert!(minimum_total <= problem_count);
    let mut quotas = specs.iter().map(|layer| layer.minimum).collect::<Vec<_>>();
    let remaining = problem_count - minimum_total;
    if remaining == 0 {
        return quotas;
    }
    let total_weight: u64 = specs.iter().map(|layer| u64::from(layer.weight)).sum();
    if total_weight == 0 {
        let layer_count = quotas.len();
        for offset in 0..remaining {
            quotas[offset % layer_count] += 1;
        }
        return quotas;
    }
    let mut remainders = Vec::with_capacity(specs.len());
    let mut assigned = 0_usize;
    for (index, layer) in specs.iter().enumerate() {
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
    strategy: &SamplingStrategy<'_>,
    pool: &[Candidate],
    problem_count: usize,
    difficulty: u8,
) -> Result<bool, GenerationError> {
    let Some(layers) = strategy.layers() else {
        return Ok(true);
    };
    let quotas = layered_quotas(layers, problem_count);
    let mut distinct = (0..layers.specs().len())
        .map(|_| HashSet::new())
        .collect::<Vec<_>>();
    for candidate in pool {
        let layer = strategy
            .layer_of(&candidate.problem)?
            .ok_or(SamplingError::EmptyLayers)?;
        distinct[layer.value()].insert(&candidate.key);
    }
    Ok(quotas.into_iter().enumerate().all(|(index, quota)| {
        let required = if difficulty == 4 {
            quota
        } else {
            quota + EFFORT_TRIM_PER_SIDE * 2
        };
        distinct[index].len() >= required
    }))
}

#[allow(clippy::too_many_arguments)]
fn select_layered_candidates<C: MonotonicClock + ?Sized>(
    pool: Vec<Candidate>,
    strategy: &SamplingStrategy<'_>,
    problem_count: usize,
    difficulty: u8,
    rng: &mut DeterministicRng,
    started: Duration,
    clock: &C,
    config: &GenerationConfig,
    attempts: &mut u64,
) -> Result<Vec<Candidate>, GenerationError> {
    let layers = strategy.layers().ok_or(SamplingError::EmptyLayers)?;
    let quotas = layered_quotas(layers, problem_count);
    let mut layer_pools = (0..layers.specs().len())
        .map(|_| Vec::new())
        .collect::<Vec<_>>();
    for candidate in pool {
        let layer = strategy
            .layer_of(&candidate.problem)?
            .ok_or(SamplingError::EmptyLayers)?;
        layer_pools[layer.value()].push(candidate);
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
            strategy.bootstrap_dedup(),
            quota,
            difficulty,
            rng,
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
    mut pool: Vec<Candidate>,
    bootstrap_dedup: BootstrapDedup,
    count: usize,
    difficulty: u8,
    rng: &mut DeterministicRng,
    started: Duration,
    clock: &C,
    config: &GenerationConfig,
    attempts: &mut u64,
) -> Result<Vec<Candidate>, GenerationError> {
    if bootstrap_dedup == BootstrapDedup::Deduplicate {
        let mut distinct = HashSet::with_capacity(pool.len());
        pool.retain(|candidate| distinct.insert(Rc::clone(&candidate.key)));
    }

    if difficulty == 4 {
        let mut selected = Vec::with_capacity(count);
        let mut selected_expressions = HashSet::with_capacity(count);
        while selected.len() < count {
            consume_attempt(started, clock, config, attempts)?;
            let selected_index = rng.next_bounded(pool.len() as u64) as usize;
            let candidate = pool.swap_remove(selected_index);
            if selected_expressions.insert(Rc::clone(&candidate.key)) {
                selected.push(candidate);
            }
        }
        return Ok(selected);
    }

    pool.sort_by(|left, right| {
        left.problem
            .effort()
            .total_cmp(&right.problem.effort())
            .then_with(|| left.key.cmp(&right.key))
            .then_with(|| left.problem.id().cmp(&right.problem.id()))
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
        if !selected_expressions.insert(Rc::clone(&pool[selected_index].key)) {
            continue;
        }
        selected.push(pool.remove(selected_index));
    }
    selected.sort_by(|left, right| {
        left.problem
            .effort()
            .total_cmp(&right.problem.effort())
            .then_with(|| left.key.cmp(&right.key))
            .then_with(|| left.problem.id().cmp(&right.problem.id()))
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
    let mut rng = DeterministicRng::from_seed(identity.seed());
    let weights = OperationWeights::default();
    let n = registration.layout().problem_count();
    let strategy = generator.sampling_strategy()?;
    let pool_size = CANDIDATE_POOL_MULTIPLIER * n * strategy.bootstrap_multiplier();
    let required_diversity = DIVERSITY_MULTIPLIER * n;

    let pool = match &strategy.kind {
        SamplingStrategyKind::ConstructiveLayered {
            source,
            layers,
            bootstrap_dedup,
            ..
        } => {
            let pool_quotas = layered_quotas(*layers, pool_size);
            let mut candidate_pool = Vec::with_capacity(pool_size);
            let mut distinct = HashSet::with_capacity(pool_size);
            for (layer_index, target) in pool_quotas.into_iter().enumerate() {
                let mut accepted = 0_usize;
                while accepted < target {
                    consume_attempt(started, clock, config, &mut attempts)?;
                    let ordinal = u32::try_from(attempts).unwrap_or(u32::MAX);
                    let Some(problem) = source.draw_candidate_for_layer(
                        &mut rng,
                        ordinal,
                        &weights,
                        layer_index,
                    )?
                    else {
                        continue;
                    };
                    let requested_layer = layers.index(layer_index)?;
                    strategy.validate_candidate_contract(None, Some(requested_layer), &problem)?;
                    if !problem_allowed_by_curriculum(registration, &problem) {
                        continue;
                    }
                    let candidate = Candidate::new(registration, problem);
                    if *bootstrap_dedup == BootstrapDedup::Deduplicate {
                        if !distinct.insert(Rc::clone(&candidate.key)) {
                            continue;
                        }
                    } else {
                        distinct.insert(Rc::clone(&candidate.key));
                    }
                    candidate_pool.push(candidate);
                    accepted += 1;
                }
            }
            check_timeout(started, clock, config)?;
            candidate_pool
        }
        SamplingStrategyKind::Random { .. }
        | SamplingStrategyKind::AnswerConditioned { .. }
        | SamplingStrategyKind::Layered { .. } => loop {
            let mut candidate_pool = Vec::with_capacity(pool_size);
            let mut distinct = HashSet::with_capacity(pool_size);
            while candidate_pool.len() < pool_size {
                // Answer-conditioned generators sample one canonical answer and
                // keep it fixed across construction retries for this candidate.
                let fixed_answer = match &strategy.kind {
                    SamplingStrategyKind::AnswerConditioned { domain, .. } => {
                        Some(domain.choose(&mut rng))
                    }
                    _ => None,
                };
                loop {
                    consume_attempt(started, clock, config, &mut attempts)?;
                    let ordinal = u32::try_from(attempts).unwrap_or(u32::MAX);
                    let problem = match &strategy.kind {
                        SamplingStrategyKind::Random { source, .. } => {
                            source.draw_candidate(&mut rng, ordinal, &weights)?
                        }
                        SamplingStrategyKind::Layered { source, .. } => {
                            source.draw_candidate(&mut rng, ordinal, &weights)?
                        }
                        SamplingStrategyKind::AnswerConditioned { source, .. } => {
                            let answer = fixed_answer.ok_or(SamplingError::EmptyAnswerDomain)?;
                            source.draw_candidate_for_answer(&mut rng, ordinal, &weights, answer)?
                        }
                        SamplingStrategyKind::ConstructiveLayered { .. } => unreachable!(),
                    };
                    let Some(problem) = problem else {
                        continue;
                    };
                    strategy.validate_candidate_contract(fixed_answer, None, &problem)?;
                    if !problem_allowed_by_curriculum(registration, &problem) {
                        continue;
                    }
                    let candidate = Candidate::new(registration, problem);
                    distinct.insert(Rc::clone(&candidate.key));
                    candidate_pool.push(candidate);
                    break;
                }
            }
            check_timeout(started, clock, config)?;
            if distinct.len() >= required_diversity
                && layered_pool_has_capacity(
                    &strategy,
                    &candidate_pool,
                    n,
                    identity.difficulty().value(),
                )?
            {
                break candidate_pool;
            }
            // The full pool is discarded. The next loop consumes fresh attempts
            // and fresh deterministic RNG draws.
        },
    };

    let mut selected = if strategy.is_layered() {
        select_layered_candidates(
            pool,
            &strategy,
            n,
            identity.difficulty().value(),
            &mut rng,
            started,
            clock,
            config,
            &mut attempts,
        )?
    } else {
        select_candidates_from_pool(
            pool,
            strategy.bootstrap_dedup(),
            n,
            identity.difficulty().value(),
            &mut rng,
            started,
            clock,
            config,
            &mut attempts,
        )?
    };

    if identity.difficulty().value() <= 2 {
        // Easy and normal worksheets should progress from lower to higher effort
        // so the sheet itself has a pedagogical difficulty ramp. Keep the same
        // deterministic tie-breakers used during candidate selection.
        selected.sort_by(|left, right| {
            left.problem
                .effort()
                .total_cmp(&right.problem.effort())
                .then_with(|| left.key.cmp(&right.key))
                .then_with(|| left.problem.id().cmp(&right.problem.id()))
        });
    } else {
        // Hard and random worksheets retain the existing shuffled presentation.
        for upper in (1..selected.len()).rev() {
            let swap_with = rng.next_bounded((upper + 1) as u64) as usize;
            selected.swap(upper, swap_with);
        }
    }
    for (index, candidate) in selected.iter_mut().enumerate() {
        candidate
            .problem
            .assign_worksheet_position((index + 1) as u32, identity.schema_version());
    }
    check_timeout(started, clock, config)?;

    Worksheet::generated(
        identity.clone(),
        registration,
        selected
            .into_iter()
            .map(|candidate| candidate.problem)
            .collect(),
    )
    .map_err(GenerationError::from)
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
    match registration.safety() {
        CurriculumSafetyPolicy::Unrestricted => true,
        CurriculumSafetyPolicy::NonNegativeOnly => {
            prompt_has_no_negative_values(problem.prompt())
                && answer_has_no_negative_values(problem.canonical_answer())
                && input_interface_has_no_negative_capability(problem.input_interface())
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
            [a, b, c, d].iter().all(|value| value.numerator() >= 0)
                && !left_negative_constant_as_subtraction
                && !right_negative_constant_as_subtraction
        }
        ProblemPrompt::QuadraticEquation { a, b, c, .. } => {
            [a, b, c].iter().all(|value| value.numerator() >= 0)
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
        ArithmeticExpression::Rational { value } => value.numerator() >= 0,
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

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
enum ProblemKey {
    Addition {
        left: u8,
        right: u8,
    },
    Arithmetic(ArithmeticExpression),
    ColumnArithmetic {
        operator: ArithmeticOperator,
        left: ArithmeticExpression,
        right: ArithmeticExpression,
    },
    LinearEquation {
        a: crate::model::RationalCoefficient,
        b: crate::model::RationalCoefficient,
        c: crate::model::RationalCoefficient,
        d: crate::model::RationalCoefficient,
    },
    QuadraticEquation {
        form: crate::model::QuadraticEquationForm,
        a: crate::model::RationalCoefficient,
        b: crate::model::RationalCoefficient,
        c: crate::model::RationalCoefficient,
    },
    SimultaneousEquation {
        a: i64,
        b: i64,
        c: i64,
        d: i64,
        e: i64,
        f: i64,
    },
    LiarPuzzle {
        people_count: crate::model::PeopleCount,
        statements: Vec<crate::model::LiarStatement>,
    },
    MiniSudoku(crate::model::MiniSudokuGrid),
}

impl ProblemKey {
    fn from_problem(registration: &ThemeRegistration, problem: &Problem) -> Self {
        match problem.prompt() {
            ProblemPrompt::Addition { left, right } => Self::Addition {
                left: *left,
                right: *right,
            },
            ProblemPrompt::Arithmetic { expression } => {
                Self::Arithmetic(match registration.dedup() {
                    DedupPolicy::PreserveOperandOrder => expression.clone(),
                    DedupPolicy::CanonicalizeCommutative => {
                        canonicalize_commutative_expression(expression)
                    }
                })
            }
            ProblemPrompt::ColumnArithmetic {
                operator,
                left,
                right,
            } => Self::ColumnArithmetic {
                operator: *operator,
                left: left.clone(),
                right: right.clone(),
            },
            ProblemPrompt::LinearEquation { a, b, c, d, .. } => Self::LinearEquation {
                a: *a,
                b: *b,
                c: *c,
                d: *d,
            },
            ProblemPrompt::QuadraticEquation { form, a, b, c } => Self::QuadraticEquation {
                form: *form,
                a: *a,
                b: *b,
                c: *c,
            },
            ProblemPrompt::SimultaneousEquation { a, b, c, d, e, f } => {
                Self::SimultaneousEquation {
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
            } => Self::LiarPuzzle {
                people_count: *people_count,
                statements: statements.clone(),
            },
            ProblemPrompt::MiniSudoku { givens } => Self::MiniSudoku(givens.clone()),
        }
    }
}

#[derive(Debug)]
struct Candidate {
    key: Rc<ProblemKey>,
    problem: Problem,
}

impl Candidate {
    fn new(registration: &ThemeRegistration, problem: Problem) -> Self {
        let key = Rc::new(ProblemKey::from_problem(registration, &problem));
        Self { key, problem }
    }
}

#[cfg(test)]
fn problem_key(registration: &ThemeRegistration, problem: &Problem) -> ProblemKey {
    ProblemKey::from_problem(registration, problem)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    struct ConstantGenerator;

    impl ProblemGenerator for ConstantGenerator {
        fn registration(&self) -> &'static ThemeRegistration {
            &basic_theme::ONE_DIGIT_ADDITION_REGISTRATION
        }

        fn sampling_strategy(&self) -> Result<SamplingStrategy<'_>, SamplingError> {
            Ok(SamplingStrategy::random(
                self,
                BootstrapDedup::AllowDuplicates,
            ))
        }
    }

    impl RandomCandidateSource for ConstantGenerator {
        fn draw_candidate(
            &self,
            _rng: &mut DeterministicRng,
            ordinal: u32,
            weights: &OperationWeights,
        ) -> Result<Option<Problem>, GenerationError> {
            basic_theme::one_digit_addition_problem(ordinal, 1, 1, weights).map(Some)
        }
    }

    struct EmptyAnswerSource;

    impl AnswerConditionedCandidateSource for EmptyAnswerSource {
        fn answer_domain(&self) -> &'static [AnswerNode] {
            &[]
        }

        fn draw_candidate_for_answer(
            &self,
            _rng: &mut DeterministicRng,
            _ordinal: u32,
            _weights: &OperationWeights,
            _answer: &AnswerNode,
        ) -> Result<Option<Problem>, GenerationError> {
            Ok(None)
        }
    }

    struct WrongAnswerSource;

    static EXPECTED_ANSWER: [AnswerNode; 1] = [AnswerNode::Integer(2)];

    impl AnswerConditionedCandidateSource for WrongAnswerSource {
        fn answer_domain(&self) -> &'static [AnswerNode] {
            &EXPECTED_ANSWER
        }

        fn draw_candidate_for_answer(
            &self,
            _rng: &mut DeterministicRng,
            ordinal: u32,
            weights: &OperationWeights,
            _answer: &AnswerNode,
        ) -> Result<Option<Problem>, GenerationError> {
            basic_theme::one_digit_addition_problem(ordinal, 1, 2, weights).map(Some)
        }
    }

    struct WrongConstructiveLayerSource;

    static TWO_LAYERS: [SamplingLayerSpec; 2] = [
        SamplingLayerSpec {
            weight: 1,
            minimum: 0,
        },
        SamplingLayerSpec {
            weight: 1,
            minimum: 0,
        },
    ];

    impl ConstructiveLayeredCandidateSource for WrongConstructiveLayerSource {
        fn layers(&self) -> &'static [SamplingLayerSpec] {
            &TWO_LAYERS
        }

        fn layer_of(&self, _problem: &Problem) -> usize {
            0
        }

        fn draw_candidate_for_layer(
            &self,
            _rng: &mut DeterministicRng,
            ordinal: u32,
            weights: &OperationWeights,
            _layer: usize,
        ) -> Result<Option<Problem>, GenerationError> {
            basic_theme::one_digit_addition_problem(ordinal, 1, 1, weights).map(Some)
        }
    }

    struct InvalidLayerSource {
        layers: &'static [SamplingLayerSpec],
        classified_layer: usize,
    }

    impl LayeredCandidateSource for InvalidLayerSource {
        fn layers(&self) -> &'static [SamplingLayerSpec] {
            self.layers
        }

        fn draw_candidate(
            &self,
            _rng: &mut DeterministicRng,
            _ordinal: u32,
            _weights: &OperationWeights,
        ) -> Result<Option<Problem>, GenerationError> {
            Ok(None)
        }

        fn layer_of(&self, _problem: &Problem) -> usize {
            self.classified_layer
        }
    }

    #[test]
    fn sampling_strategy_rejects_invalid_capability_values_before_sampling() {
        assert_eq!(
            SamplingStrategy::answer_conditioned(&EmptyAnswerSource).err(),
            Some(SamplingError::EmptyAnswerDomain)
        );

        let empty_layers = InvalidLayerSource {
            layers: &[],
            classified_layer: 0,
        };
        assert_eq!(
            SamplingStrategy::layered(&empty_layers, BootstrapDedup::AllowDuplicates, 20).err(),
            Some(SamplingError::EmptyLayers)
        );

        static TOO_LARGE_MINIMUM: [SamplingLayerSpec; 1] = [SamplingLayerSpec {
            weight: 1,
            minimum: 21,
        }];
        let excessive_minimum = InvalidLayerSource {
            layers: &TOO_LARGE_MINIMUM,
            classified_layer: 0,
        };
        assert_eq!(
            SamplingStrategy::layered(&excessive_minimum, BootstrapDedup::AllowDuplicates, 20,)
                .err(),
            Some(SamplingError::LayerMinimumExceedsWorksheet {
                minimum_total: 21,
                problem_count: 20,
            })
        );
    }

    #[test]
    fn layered_classifier_is_converted_to_a_bounded_layer_index() {
        static ONE_LAYER: [SamplingLayerSpec; 1] = [SamplingLayerSpec {
            weight: 1,
            minimum: 0,
        }];
        let source = InvalidLayerSource {
            layers: &ONE_LAYER,
            classified_layer: 1,
        };
        let strategy =
            SamplingStrategy::layered(&source, BootstrapDedup::AllowDuplicates, 20).unwrap();
        let problem =
            basic_theme::one_digit_addition_problem(1, 1, 1, &OperationWeights::default()).unwrap();
        assert_eq!(
            strategy.layer_of(&problem),
            Err(SamplingError::LayerOutOfRange {
                index: 1,
                layer_count: 1,
            })
        );
    }

    #[test]
    fn answer_conditioned_candidate_mismatch_is_an_immediate_sampling_error() {
        let source = WrongAnswerSource;
        let strategy = SamplingStrategy::answer_conditioned(&source).unwrap();
        let problem =
            basic_theme::one_digit_addition_problem(1, 1, 2, &OperationWeights::default()).unwrap();
        assert_eq!(
            strategy.validate_candidate_contract(Some(&EXPECTED_ANSWER[0]), None, &problem),
            Err(SamplingError::AnswerConditionMismatch)
        );
    }

    #[test]
    fn constructive_layer_mismatch_is_an_immediate_sampling_error() {
        let source = WrongConstructiveLayerSource;
        let strategy =
            SamplingStrategy::constructive_layered(&source, BootstrapDedup::AllowDuplicates, 1, 20)
                .unwrap();
        let requested = strategy.layers().unwrap().index(1).unwrap();
        let problem =
            basic_theme::one_digit_addition_problem(1, 1, 1, &OperationWeights::default()).unwrap();
        assert_eq!(
            strategy.validate_candidate_contract(None, Some(requested), &problem),
            Err(SamplingError::RequestedLayerMismatch {
                requested: 1,
                actual: 0,
            })
        );
    }

    #[test]
    fn candidate_key_clones_share_one_semantic_key_allocation() {
        let problem =
            basic_theme::one_digit_addition_problem(1, 1, 1, &OperationWeights::default()).unwrap();
        let candidate = Candidate::new(&basic_theme::ONE_DIGIT_ADDITION_REGISTRATION, problem);
        let shared = Rc::clone(&candidate.key);
        assert!(Rc::ptr_eq(&candidate.key, &shared));
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
    fn all_registered_themes_are_deterministic_without_duplicate_prompts() {
        for registration in crate::registry::active_registrations().unwrap() {
            for seed in ["A1b2", "M7x9"] {
                for difficulty_value in [1u8, 3u8, 4u8] {
                    let difficulty =
                        crate::identity::Difficulty::try_from(difficulty_value).unwrap();
                    let request = GenerateWorksheetRequest {
                        schema_version: SCHEMA_VERSION,
                        numeric_theme_id: registration.numeric_theme_id(),
                        seed: seed.to_owned(),
                        difficulty,
                        timeout_ms: None,
                        max_attempts: None,
                    };
                    let first = generate_worksheet_request(&request).unwrap_or_else(|error| {
                        panic!(
                            "theme {} seed {seed} difficulty {difficulty_value} failed: {error}",
                            registration.numeric_theme_id()
                        )
                    });
                    let second = generate_worksheet_request(&request).unwrap();
                    assert_eq!(
                        first, second,
                        "same request must be byte-semantically deterministic"
                    );
                    assert_eq!(
                        first.problems().len(),
                        registration.layout().problem_count()
                    );
                    for left in 0..first.problems().len() {
                        for right in left + 1..first.problems().len() {
                            assert_ne!(
                                first.problems()[left].prompt(),
                                first.problems()[right].prompt(),
                                "theme {} duplicated a prompt for seed {seed} difficulty {difficulty_value}",
                                registration.numeric_theme_id()
                            );
                            assert_ne!(
                                problem_key(registration, &first.problems()[left]),
                                problem_key(registration, &first.problems()[right]),
                                "theme {} duplicated a commutative-equivalent prompt for seed {seed} difficulty {difficulty_value}",
                                registration.numeric_theme_id()
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
        for registration in crate::registry::active_registrations().unwrap() {
            for seed in SEEDS {
                for difficulty_value in 1_u8..=4 {
                    let request = GenerateWorksheetRequest {
                        schema_version: SCHEMA_VERSION,
                        numeric_theme_id: registration.numeric_theme_id(),
                        seed: seed.to_owned(),
                        difficulty: crate::identity::Difficulty::try_from(difficulty_value)
                            .unwrap(),
                        timeout_ms: None,
                        max_attempts: None,
                    };
                    let first = generate_worksheet_request(&request).unwrap_or_else(|error| {
                        panic!(
                            "theme {} seed {seed} difficulty {difficulty_value} failed: {error}",
                            registration.numeric_theme_id()
                        )
                    });
                    let second = generate_worksheet_request(&request).unwrap();
                    assert_eq!(first, second, "same seed/revision must be deterministic");
                    assert_eq!(
                        first.identity().generator_revision(),
                        registration.generator_revision()
                    );
                    assert_eq!(
                        regenerate_problem_set(&first.problem_set_id()).unwrap(),
                        first,
                        "problem_set_id regeneration must preserve the same revision identity"
                    );
                    if difficulty_value <= 2 {
                        assert!(
                            first
                                .problems()
                                .windows(2)
                                .all(|pair| pair[0].effort() <= pair[1].effort()),
                            "theme {} difficulty {difficulty_value} lost easy/normal effort sort",
                            registration.numeric_theme_id()
                        );
                    }
                    for problem in first.problems() {
                        assert!(problem.effort().is_finite() && problem.effort() >= 0.0);
                        assert!(problem.operation_vector().is_nonnegative_finite());
                        if let Some(special) = problem.theme_specific_effort() {
                            assert!(special.is_finite() && special >= 0.0);
                            assert_eq!(problem.effort(), special);
                            assert!(problem.operation_plan().is_none());
                            assert_eq!(
                                problem.operation_vector(),
                                crate::effort::OperationVector::zero()
                            );
                        } else {
                            assert_eq!(
                                problem
                                    .operation_plan()
                                    .expect("operation-based effort must carry an operation plan")
                                    .operation_vector(),
                                problem.operation_vector(),
                                "derived operation vector must equal the operation-plan projection"
                            );
                            let expected = OperationWeights::default()
                                .weighted_sum(&problem.operation_vector());
                            assert!((problem.effort() - expected).abs() < 1e-12);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn easy_and_normal_worksheets_are_presented_in_nondecreasing_effort_order() {
        for registration in crate::registry::active_registrations().unwrap() {
            for difficulty_value in [1_u8, 2_u8] {
                let worksheet = generate_worksheet_request(&GenerateWorksheetRequest {
                    schema_version: SCHEMA_VERSION,
                    numeric_theme_id: registration.numeric_theme_id(),
                    seed: "EfrtRder".to_owned(),
                    difficulty: crate::identity::Difficulty::try_from(difficulty_value).unwrap(),
                    timeout_ms: Some(1_000),
                    max_attempts: Some(50_000),
                })
                .unwrap_or_else(|error| {
                    panic!(
                        "theme {} difficulty {difficulty_value} failed: {error}",
                        registration.numeric_theme_id()
                    )
                });
                assert!(
                    worksheet
                        .problems()
                        .windows(2)
                        .all(|pair| pair[0].effort() <= pair[1].effort()),
                    "theme {} difficulty {difficulty_value} was not effort-sorted: {:?}",
                    registration.numeric_theme_id(),
                    worksheet
                        .problems()
                        .iter()
                        .map(|problem| problem.effort())
                        .collect::<Vec<_>>()
                );
            }
        }
    }

    #[test]
    fn elementary_registered_themes_never_expose_negative_values() {
        for registration in crate::registry::active_registrations()
            .unwrap()
            .into_iter()
            .filter(|registration| {
                registration
                    .grade()
                    .is_some_and(|grade| grade.is_elementary())
            })
        {
            let worksheet = generate_worksheet_request(&GenerateWorksheetRequest {
                schema_version: SCHEMA_VERSION,
                numeric_theme_id: registration.numeric_theme_id(),
                seed: "Em7Z".to_owned(),
                difficulty: crate::identity::DEFAULT_DIFFICULTY,
                timeout_ms: None,
                max_attempts: None,
            })
            .unwrap_or_else(|error| {
                panic!(
                    "elementary theme {} failed: {error}",
                    registration.numeric_theme_id()
                )
            });
            for problem in worksheet.problems() {
                assert!(problem_allowed_by_curriculum(registration, problem));
                assert!(prompt_has_no_negative_values(problem.prompt()));
                assert!(answer_has_no_negative_values(problem.canonical_answer()));
                assert!(input_interface_has_no_negative_capability(
                    problem.input_interface()
                ));
            }
        }
    }

    #[test]
    fn layered_themes_enforce_declared_quotas_at_every_difficulty() {
        let mut discovered = 0_usize;
        for entry in registered_generator_entries() {
            let generator = entry.generator;
            let strategy = generator.sampling_strategy().unwrap();
            let Some(layers) = strategy.layers() else {
                continue;
            };
            discovered += 1;
            let registration = generator.registration();
            let expected = layered_quotas(layers, registration.layout().problem_count());
            for difficulty_value in 1_u8..=4 {
                for seed in ["LyrA1", "LyrB2", "LyrC3"] {
                    let worksheet = generate_worksheet_request(&GenerateWorksheetRequest {
                        schema_version: SCHEMA_VERSION,
                        numeric_theme_id: registration.numeric_theme_id(),
                        seed: seed.to_owned(),
                        difficulty: crate::identity::Difficulty::try_from(difficulty_value)
                            .unwrap(),
                        timeout_ms: Some(1_000),
                        max_attempts: Some(50_000),
                    })
                    .unwrap();
                    let mut counts = vec![0_usize; layers.specs().len()];
                    for problem in worksheet.problems() {
                        let layer = strategy
                            .layer_of(problem)
                            .unwrap()
                            .expect("layered strategy must classify every generated problem");
                        counts[layer.value()] += 1;
                    }
                    assert_eq!(
                        counts,
                        expected,
                        "theme {} difficulty {difficulty_value}",
                        registration.numeric_theme_id()
                    );
                }
            }
        }
        assert!(
            discovered > 0,
            "registry must expose at least one layered strategy"
        );
    }

    #[test]
    fn layered_difficulty_selection_remains_scalar_within_each_layer() {
        let seeds = ["DfA1", "DfB2", "DfC3", "DfD4", "DfE5", "DfF6"];
        let mut discovered = 0_usize;
        for entry in registered_generator_entries() {
            let generator = entry.generator;
            let strategy = generator.sampling_strategy().unwrap();
            let Some(layers) = strategy.layers() else {
                continue;
            };
            discovered += 1;
            let registration = generator.registration();
            let layer_count = layers.specs().len();
            let mut means = vec![vec![0.0_f64; layer_count]; 3];
            let mut counts = vec![vec![0_usize; layer_count]; 3];
            for difficulty_value in 1_u8..=3 {
                for seed in seeds {
                    let worksheet = generate_worksheet_request(&GenerateWorksheetRequest {
                        schema_version: SCHEMA_VERSION,
                        numeric_theme_id: registration.numeric_theme_id(),
                        seed: seed.to_owned(),
                        difficulty: crate::identity::Difficulty::try_from(difficulty_value)
                            .unwrap(),
                        timeout_ms: Some(1_000),
                        max_attempts: Some(50_000),
                    })
                    .unwrap();
                    for problem in worksheet.problems() {
                        let layer = strategy
                            .layer_of(problem)
                            .unwrap()
                            .expect("layered strategy must classify every generated problem");
                        means[(difficulty_value - 1) as usize][layer.value()] += problem.effort();
                        counts[(difficulty_value - 1) as usize][layer.value()] += 1;
                    }
                }
            }
            for difficulty in 0..3 {
                for layer in 0..layer_count {
                    assert!(counts[difficulty][layer] > 0);
                    means[difficulty][layer] /= counts[difficulty][layer] as f64;
                }
            }
            for (layer, _) in means[0].iter().enumerate().take(layer_count) {
                assert!(
                    means[0][layer] <= means[1][layer] && means[1][layer] <= means[2][layer],
                    "theme {} layer {layer} lost scalar effort separation: {:?}",
                    registration.numeric_theme_id(),
                    [means[0][layer], means[1][layer], means[2][layer]]
                );
            }
        }
        assert!(
            discovered > 0,
            "registry must expose at least one layered strategy"
        );
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
