use crate::answer::{AnswerBinaryOperator, AnswerNode};
use crate::effort::{
    linear_equation_plan, quadratic_factoring_plan, quadratic_formula_plan, quadratic_square_plan,
    simultaneous_equation_plan, EffortModel, OperationWeights,
};
use crate::error::GenerationError;
use crate::exact::{exact_square_root_u128, gcd_u64, square_free_sqrt_decomposition};
use crate::generator::{
    AnswerConditionedCandidateSource, BootstrapDedup, GeneratorEntry, LayeredCandidateSource,
    ProblemGenerator, RandomCandidateSource, SamplingStrategy,
};
use crate::generator_support::{draw_signed_integer, rational_answer};
use crate::model::{
    AnswerSchema, Problem, ProblemPrompt, QuadraticEquationForm, RationalCoefficient,
};
use crate::rng::DeterministicRng;
use crate::theme::{
    CurriculumSafetyPolicy as Safety, DedupPolicy as Dedup, SamplingLayerSpec, SchoolGrade,
    ThemeAnswerContract as AnswerContract, ThemeInputProfile as Input,
    ThemePresentationPolicy as Presentation, ThemeRegistration, ThemeRegistrationSpec, ThemeTag,
    COMPACT_16_LAYOUT, EQUATION_PAIR_12_LAYOUT,
};
use std::sync::OnceLock;

pub const LINEAR_EQUATION_PROBLEM_COUNT: usize = COMPACT_16_LAYOUT.problem_count();
pub const LINEAR_EQUATION_COLUMNS: usize = COMPACT_16_LAYOUT.columns();
pub const LINEAR_EQUATION_ROWS: usize = COMPACT_16_LAYOUT.rows();
pub const THEME_ID_LINEAR_EQUATION_1: u32 = 2;
pub const THEME_ID_LINEAR_EQUATION_2: u32 = 3;
pub const THEME_ID_QUADRATIC_EQUATION_1: u32 = 14;
pub const THEME_ID_QUADRATIC_EQUATION_2: u32 = 15;
pub const THEME_ID_QUADRATIC_EQUATION_3: u32 = 16;
pub const THEME_ID_SIMULTANEOUS_EQUATION_1: u32 = 19;
pub const GENERATOR_REVISION_LINEAR_EQUATION_1: u32 = 8;
pub const GENERATOR_REVISION_LINEAR_EQUATION_2: u32 = 8;
pub const GENERATOR_REVISION_QUADRATIC_EQUATION_1: u32 = 3;
pub const GENERATOR_REVISION_QUADRATIC_EQUATION_2: u32 = 4;
pub const GENERATOR_REVISION_QUADRATIC_EQUATION_3: u32 = 3;
pub const GENERATOR_REVISION_SIMULTANEOUS_EQUATION_1: u32 = 3;
pub const SKILL_ID_LINEAR_EQUATION_1: &str = "jp.grade7.equation.linear.1";
pub const SKILL_ID_LINEAR_EQUATION_2: &str = "jp.grade7.equation.linear.2";
pub const SKILL_ID_QUADRATIC_EQUATION_1: &str = "jp.grade9.equation.quadratic.1";
pub const SKILL_ID_QUADRATIC_EQUATION_2: &str = "jp.grade9.equation.quadratic.2";
pub const SKILL_ID_QUADRATIC_EQUATION_3: &str = "jp.grade9.equation.quadratic.3";
pub const SKILL_ID_SIMULTANEOUS_EQUATION_1: &str = "jp.grade8.equation.simultaneous.1";
pub const CURRICULUM_PATH_LINEAR_EQUATION_1: [&str; 4] =
    ["root", "中学1年生", "一次方程式", "一次方程式(1)"];
pub const CURRICULUM_PATH_LINEAR_EQUATION_2: [&str; 4] =
    ["root", "中学1年生", "一次方程式", "一次方程式(2)"];
pub const CURRICULUM_PATH_QUADRATIC_EQUATION_1: [&str; 4] =
    ["root", "中学3年生", "二次方程式", "二次方程式(1)"];
pub const CURRICULUM_PATH_QUADRATIC_EQUATION_2: [&str; 4] =
    ["root", "中学3年生", "二次方程式", "二次方程式(2)"];
pub const CURRICULUM_PATH_QUADRATIC_EQUATION_3: [&str; 4] =
    ["root", "中学3年生", "二次方程式", "二次方程式(3)"];
pub const CURRICULUM_PATH_SIMULTANEOUS_EQUATION_1: [&str; 4] =
    ["root", "中学2年生", "連立方程式", "連立方程式(1)"];

const LINEAR: &[ThemeTag] = &[ThemeTag::Equations, ThemeTag::LinearEquation];
const QUADRATIC: &[ThemeTag] = &[ThemeTag::Equations, ThemeTag::QuadraticEquation];
const SIMULTANEOUS: &[ThemeTag] = &[ThemeTag::Equations, ThemeTag::SimultaneousEquation];

pub const QUADRATIC_FACTORING_LAYERS: [SamplingLayerSpec; 3] = [
    SamplingLayerSpec {
        weight: 0,
        minimum: 2,
    },
    SamplingLayerSpec {
        weight: 0,
        minimum: 2,
    },
    SamplingLayerSpec {
        weight: 1,
        minimum: 0,
    },
];

pub const LINEAR_EQUATION_1_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_LINEAR_EQUATION_1),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_LINEAR_EQUATION_1,
        ),
        skill_id: SKILL_ID_LINEAR_EQUATION_1,
        curriculum_path: &CURRICULUM_PATH_LINEAR_EQUATION_1,
        grade: Some(SchoolGrade::JuniorHigh1),
        tags: LINEAR,
        safety: Safety::Unrestricted,
        presentation: Presentation::EQUATION,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: AnswerContract::LinearInteger,
        layout: COMPACT_16_LAYOUT,
    })
    .with_editor_input_profile(Input::JuniorHighFull);
pub const LINEAR_EQUATION_2_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_LINEAR_EQUATION_2),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_LINEAR_EQUATION_2,
        ),
        skill_id: SKILL_ID_LINEAR_EQUATION_2,
        curriculum_path: &CURRICULUM_PATH_LINEAR_EQUATION_2,
        grade: Some(SchoolGrade::JuniorHigh1),
        tags: LINEAR,
        safety: Safety::Unrestricted,
        presentation: Presentation::EQUATION,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: AnswerContract::LinearRational,
        layout: COMPACT_16_LAYOUT,
    })
    .with_editor_input_profile(Input::JuniorHighFull);
pub const QUADRATIC_EQUATION_1_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_QUADRATIC_EQUATION_1),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_QUADRATIC_EQUATION_1,
        ),
        skill_id: SKILL_ID_QUADRATIC_EQUATION_1,
        curriculum_path: &CURRICULUM_PATH_QUADRATIC_EQUATION_1,
        grade: Some(SchoolGrade::JuniorHigh3),
        tags: QUADRATIC,
        safety: Safety::Unrestricted,
        presentation: Presentation::EQUATION,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: AnswerContract::QuadraticAlgebraic,
        layout: COMPACT_16_LAYOUT,
    })
    .with_editor_input_profile(Input::JuniorHighFull);
pub const QUADRATIC_EQUATION_2_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_QUADRATIC_EQUATION_2),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_QUADRATIC_EQUATION_2,
        ),
        skill_id: SKILL_ID_QUADRATIC_EQUATION_2,
        curriculum_path: &CURRICULUM_PATH_QUADRATIC_EQUATION_2,
        grade: Some(SchoolGrade::JuniorHigh3),
        tags: QUADRATIC,
        safety: Safety::Unrestricted,
        presentation: Presentation::EQUATION,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: AnswerContract::QuadraticAlgebraic,
        layout: COMPACT_16_LAYOUT,
    })
    .with_editor_input_profile(Input::JuniorHighFull);
pub const QUADRATIC_EQUATION_3_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_QUADRATIC_EQUATION_3),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_QUADRATIC_EQUATION_3,
        ),
        skill_id: SKILL_ID_QUADRATIC_EQUATION_3,
        curriculum_path: &CURRICULUM_PATH_QUADRATIC_EQUATION_3,
        grade: Some(SchoolGrade::JuniorHigh3),
        tags: QUADRATIC,
        safety: Safety::Unrestricted,
        presentation: Presentation::EQUATION,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: AnswerContract::QuadraticAlgebraic,
        layout: COMPACT_16_LAYOUT,
    })
    .with_editor_input_profile(Input::JuniorHighFull);
pub const SIMULTANEOUS_EQUATION_1_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_SIMULTANEOUS_EQUATION_1),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_SIMULTANEOUS_EQUATION_1,
        ),
        skill_id: SKILL_ID_SIMULTANEOUS_EQUATION_1,
        curriculum_path: &CURRICULUM_PATH_SIMULTANEOUS_EQUATION_1,
        grade: Some(SchoolGrade::JuniorHigh2),
        tags: SIMULTANEOUS,
        safety: Safety::Unrestricted,
        presentation: Presentation::EQUATION,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: AnswerContract::SimultaneousPair,
        layout: EQUATION_PAIR_12_LAYOUT,
    })
    .with_editor_input_profile(Input::JuniorHighFull);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum LinearEquationMode {
    IntegerSolution,
    RationalSolution,
}

#[derive(Debug)]
pub(crate) struct LinearEquationGenerator {
    registration: &'static ThemeRegistration,
    pub(crate) mode: LinearEquationMode,
}
impl ProblemGenerator for LinearEquationGenerator {
    fn registration(&self) -> &'static ThemeRegistration {
        self.registration
    }

    fn sampling_strategy(&self) -> Result<SamplingStrategy<'_>, crate::error::SamplingError> {
        SamplingStrategy::answer_conditioned(self)
    }
}

impl AnswerConditionedCandidateSource for LinearEquationGenerator {
    fn answer_domain(&self) -> &'static [AnswerNode] {
        linear_answer_domain(self.mode)
    }

    fn draw_candidate_for_answer(
        &self,
        rng: &mut DeterministicRng,
        ordinal: u32,
        weights: &OperationWeights,
        answer: &AnswerNode,
    ) -> Result<Option<Problem>, GenerationError> {
        let Some(solution) = crate::exact_value::rational_coefficient_from_answer(answer) else {
            return Ok(None);
        };
        let shape = rng.next_bounded(4);
        let prefer_reduction = self.mode == LinearEquationMode::RationalSolution
            && !solution.is_zero()
            && rng.next_bounded(4) != 0;
        let coefficients = if prefer_reduction {
            draw_reduction_conditioned_coefficients(rng, self.mode, shape, solution)
                .or_else(|| draw_conditioned_coefficients(rng, self.mode, shape, solution))
        } else {
            draw_conditioned_coefficients(rng, self.mode, shape, solution)
        };
        let Some((a, b, c, d)) = coefficients else {
            return Ok(None);
        };
        let left_negative_constant_as_subtraction = b.numerator() < 0 && rng.next_bounded(2) == 0;
        let right_negative_constant_as_subtraction = d.numerator() < 0 && rng.next_bounded(2) == 0;
        linear_equation_problem(
            ordinal,
            self.registration,
            self.mode,
            a,
            b,
            c,
            d,
            left_negative_constant_as_subtraction,
            right_negative_constant_as_subtraction,
            solution,
            weights,
        )
        .map(Some)
    }
}

#[derive(Debug)]
pub(crate) struct SimultaneousEquationGenerator {
    registration: &'static ThemeRegistration,
}

impl ProblemGenerator for SimultaneousEquationGenerator {
    fn registration(&self) -> &'static ThemeRegistration {
        self.registration
    }

    fn sampling_strategy(&self) -> Result<SamplingStrategy<'_>, crate::error::SamplingError> {
        Ok(SamplingStrategy::random(
            self,
            BootstrapDedup::AllowDuplicates,
        ))
    }
}

impl RandomCandidateSource for SimultaneousEquationGenerator {
    fn draw_candidate(
        &self,
        rng: &mut DeterministicRng,
        ordinal: u32,
        weights: &OperationWeights,
    ) -> Result<Option<Problem>, GenerationError> {
        simultaneous_equation_problem(self.registration, rng, ordinal, weights).transpose()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum QuadraticEquationMode {
    SquareReduction,
    Factoring,
    Formula,
}

#[derive(Debug)]
pub(crate) struct QuadraticEquationGenerator {
    registration: &'static ThemeRegistration,
    mode: QuadraticEquationMode,
}

impl ProblemGenerator for QuadraticEquationGenerator {
    fn registration(&self) -> &'static ThemeRegistration {
        self.registration
    }

    fn sampling_strategy(&self) -> Result<SamplingStrategy<'_>, crate::error::SamplingError> {
        match self.mode {
            QuadraticEquationMode::SquareReduction => SamplingStrategy::answer_conditioned(self),
            QuadraticEquationMode::Factoring => SamplingStrategy::layered(
                self,
                BootstrapDedup::AllowDuplicates,
                self.registration.layout().problem_count(),
            ),
            QuadraticEquationMode::Formula => Ok(SamplingStrategy::random(
                self,
                BootstrapDedup::AllowDuplicates,
            )),
        }
    }
}

impl AnswerConditionedCandidateSource for QuadraticEquationGenerator {
    fn answer_domain(&self) -> &'static [AnswerNode] {
        quadratic_one_answer_domain()
    }

    fn draw_candidate_for_answer(
        &self,
        rng: &mut DeterministicRng,
        ordinal: u32,
        weights: &OperationWeights,
        answer: &AnswerNode,
    ) -> Result<Option<Problem>, GenerationError> {
        quadratic_equation_problem(
            self.registration,
            self.mode,
            rng,
            ordinal,
            weights,
            Some(answer),
        )
        .transpose()
    }
}

impl RandomCandidateSource for QuadraticEquationGenerator {
    fn draw_candidate(
        &self,
        rng: &mut DeterministicRng,
        ordinal: u32,
        weights: &OperationWeights,
    ) -> Result<Option<Problem>, GenerationError> {
        quadratic_equation_problem(self.registration, self.mode, rng, ordinal, weights, None)
            .transpose()
    }
}

impl LayeredCandidateSource for QuadraticEquationGenerator {
    fn layers(&self) -> &'static [SamplingLayerSpec] {
        &QUADRATIC_FACTORING_LAYERS
    }

    fn draw_candidate(
        &self,
        rng: &mut DeterministicRng,
        ordinal: u32,
        weights: &OperationWeights,
    ) -> Result<Option<Problem>, GenerationError> {
        quadratic_equation_problem(self.registration, self.mode, rng, ordinal, weights, None)
            .transpose()
    }

    fn layer_of(&self, problem: &Problem) -> usize {
        let ProblemPrompt::QuadraticEquation { b, c, .. } = problem.prompt() else {
            unreachable!("quadratic factoring generator always emits quadratic prompts");
        };
        if b.numerator() == 0 && c.numerator() < 0 {
            return 0;
        }
        let b_numerator = i128::from(b.numerator());
        let discriminant = b_numerator * b_numerator - 4 * i128::from(c.numerator());
        if b.denominator() == 1 && c.denominator() == 1 && discriminant == 0 {
            1
        } else {
            2
        }
    }
}

pub(crate) static LINEAR_EQUATION_1_GENERATOR: LinearEquationGenerator = LinearEquationGenerator {
    registration: &LINEAR_EQUATION_1_REGISTRATION,
    mode: LinearEquationMode::IntegerSolution,
};
pub(crate) static LINEAR_EQUATION_2_GENERATOR: LinearEquationGenerator = LinearEquationGenerator {
    registration: &LINEAR_EQUATION_2_REGISTRATION,
    mode: LinearEquationMode::RationalSolution,
};
pub(crate) static SIMULTANEOUS_EQUATION_1_GENERATOR: SimultaneousEquationGenerator =
    SimultaneousEquationGenerator {
        registration: &SIMULTANEOUS_EQUATION_1_REGISTRATION,
    };
pub(crate) static QUADRATIC_EQUATION_1_GENERATOR: QuadraticEquationGenerator =
    QuadraticEquationGenerator {
        registration: &QUADRATIC_EQUATION_1_REGISTRATION,
        mode: QuadraticEquationMode::SquareReduction,
    };
pub(crate) static QUADRATIC_EQUATION_2_GENERATOR: QuadraticEquationGenerator =
    QuadraticEquationGenerator {
        registration: &QUADRATIC_EQUATION_2_REGISTRATION,
        mode: QuadraticEquationMode::Factoring,
    };
pub(crate) static QUADRATIC_EQUATION_3_GENERATOR: QuadraticEquationGenerator =
    QuadraticEquationGenerator {
        registration: &QUADRATIC_EQUATION_3_REGISTRATION,
        mode: QuadraticEquationMode::Formula,
    };

pub(crate) fn quadratic_one_answer_domain() -> &'static [AnswerNode] {
    static DOMAIN: OnceLock<Vec<AnswerNode>> = OnceLock::new();
    DOMAIN.get_or_init(|| {
        let mut answers = Vec::new();
        for root in 1_i64..=16 {
            answers.push(AnswerNode::PlusMinus(Box::new(AnswerNode::Integer(root))));
        }
        for radicand in 2_i64..=30 {
            if exact_square_root_u128(radicand as u128).is_some() {
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
            exact_square_root_u128(*value as u128)
                .is_none()
                .then_some(*value)
        }
        _ => None,
    }
}

pub(crate) fn linear_answer_domain(mode: LinearEquationMode) -> &'static [AnswerNode] {
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

pub(crate) fn linear_solution_domain(mode: LinearEquationMode) -> &'static [RationalCoefficient] {
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
                    if value.denominator() != denominator {
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
            RationalCoefficient::new(sign.checked_mul(k)?.checked_mul(solution.denominator())?, 1)?;
        let b_total =
            RationalCoefficient::new(sign.checked_mul(k)?.checked_mul(solution.numerator())?, 1)?;
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

pub(crate) fn linear_integer_domain_with_zero() -> &'static [RationalCoefficient] {
    static VALUES: OnceLock<Vec<RationalCoefficient>> = OnceLock::new();
    VALUES.get_or_init(|| {
        (-15_i64..=15)
            .map(|value| RationalCoefficient::new(value, 1).expect("integer coefficient"))
            .collect()
    })
}

pub(crate) fn linear_rational_domain_with_zero() -> &'static [RationalCoefficient] {
    static VALUES: OnceLock<Vec<RationalCoefficient>> = OnceLock::new();
    VALUES.get_or_init(|| {
        let mut values = linear_rational_domain().to_vec();
        values.push(RationalCoefficient::zero());
        values.sort_unstable_by_key(|value| (value.denominator(), value.numerator()));
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
                if positive.denominator() == 1 {
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

pub(crate) fn linear_rational_domain() -> &'static [RationalCoefficient] {
    static VALUES: OnceLock<Vec<RationalCoefficient>> = OnceLock::new();
    VALUES.get_or_init(|| {
        let mut values = linear_integer_domain().to_vec();
        values.extend_from_slice(linear_fraction_domain());
        values.sort_unstable();
        values.dedup();
        values
    })
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

fn simultaneous_equation_problem(
    registration: &ThemeRegistration,
    rng: &mut DeterministicRng,
    id: u32,
    weights: &OperationWeights,
) -> Option<Result<Problem, GenerationError>> {
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
    let operation_plan = simultaneous_equation_plan(a, b, c, d, e, f, &canonical_answer, weights)?;
    Some(
        Problem::generated(
            registration,
            id,
            ProblemPrompt::SimultaneousEquation { a, b, c, d, e, f },
            AnswerSchema::OrderedPair,
            canonical_answer,
            EffortModel::operations(operation_plan),
        )
        .map_err(GenerationError::from),
    )
}

fn quadratic_equation_problem(
    registration: &ThemeRegistration,
    mode: QuadraticEquationMode,
    rng: &mut DeterministicRng,
    id: u32,
    _weights: &OperationWeights,
    fixed_answer: Option<&AnswerNode>,
) -> Option<Result<Problem, GenerationError>> {
    let (form, a, b, c, canonical_answer, operation_plan) = match mode {
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
            let plan = quadratic_square_plan(form, a, c, &answer)?;
            (form, a, RationalCoefficient::zero(), c, answer, plan)
        }
        QuadraticEquationMode::Factoring => {
            let first = draw_signed_integer(rng, 9)?;
            let second = draw_signed_integer(rng, 9)?;
            // The three pedagogical archetypes are sampled in separate layers.
            // Repeated roots are therefore intentionally retained, and the
            // externally redundant scale is fixed to one: multiplying the whole
            // equation by 2..5 adds visual noise without changing the factoring
            // strategy being measured by effort.
            let scale = 1_i64;
            let b_int = first.checked_add(second)?.checked_neg()?;
            let c_int = first.checked_mul(second)?;
            let answer = if first == second {
                AnswerNode::Integer(first)
            } else {
                let mut roots = vec![AnswerNode::Integer(first), AnswerNode::Integer(second)];
                roots.sort();
                AnswerNode::Tuple(roots)
            };
            let plan = quadratic_factoring_plan(b_int, c_int, &answer)?;
            (
                QuadraticEquationForm::FactoredScale,
                RationalCoefficient::new(scale, 1)?,
                RationalCoefficient::new(b_int, 1)?,
                RationalCoefficient::new(c_int, 1)?,
                answer,
                plan,
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
            let (sqrt_coefficient, radicand) =
                square_free_sqrt_decomposition(u64::try_from(discriminant).ok()?)?;
            let sqrt_coefficient = i64::try_from(sqrt_coefficient).ok()?;
            let radicand = i64::try_from(radicand).ok()?;
            if radicand == 1 || radicand > 99 {
                return None;
            }
            let two_a = 2_i64.checked_mul(a_int)?;
            let common = gcd_u64(
                gcd_u64(b_int.unsigned_abs(), sqrt_coefficient.unsigned_abs()),
                two_a.unsigned_abs(),
            )
            .max(1) as i64;
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
            let plan = quadratic_formula_plan(a, b, c, &answer)?;
            (QuadraticEquationForm::Standard, a, b, c, answer, plan)
        }
    };

    Some(
        Problem::generated(
            registration,
            id,
            ProblemPrompt::QuadraticEquation { form, a, b, c },
            AnswerSchema::Algebraic,
            canonical_answer,
            EffortModel::operations(operation_plan),
        )
        .map_err(GenerationError::from),
    )
}

#[allow(clippy::too_many_arguments)]
fn linear_equation_problem(
    id: u32,
    registration: &ThemeRegistration,
    mode: LinearEquationMode,
    a: RationalCoefficient,
    b: RationalCoefficient,
    c: RationalCoefficient,
    d: RationalCoefficient,
    left_negative_constant_as_subtraction: bool,
    right_negative_constant_as_subtraction: bool,
    solution: RationalCoefficient,
    _weights: &OperationWeights,
) -> Result<Problem, GenerationError> {
    let canonical_answer = rational_answer(solution);
    let operation_plan = linear_equation_plan(a, b, c, d, &canonical_answer).ok_or(
        GenerationError::InvalidGeneratedProblem {
            reason: "linear-equation effort model rejected generated coefficients",
        },
    )?;
    let answer_schema = match mode {
        LinearEquationMode::IntegerSolution => AnswerSchema::Integer { min: -15, max: 15 },
        LinearEquationMode::RationalSolution => AnswerSchema::Rational {
            max_abs_numerator: 20,
            max_denominator: 12,
            require_reduced_fraction_form: true,
        },
    };
    Problem::generated(
        registration,
        id,
        ProblemPrompt::LinearEquation {
            a,
            b,
            c,
            d,
            left_negative_constant_as_subtraction,
            right_negative_constant_as_subtraction,
        },
        answer_schema,
        canonical_answer,
        EffortModel::operations(operation_plan),
    )
    .map_err(GenerationError::from)
}

/// Current generators owned by this theme family.
pub(crate) static GENERATORS: [GeneratorEntry; 6] = [
    GeneratorEntry::current(&LINEAR_EQUATION_1_GENERATOR),
    GeneratorEntry::current(&LINEAR_EQUATION_2_GENERATOR),
    GeneratorEntry::current(&QUADRATIC_EQUATION_1_GENERATOR),
    GeneratorEntry::current(&QUADRATIC_EQUATION_2_GENERATOR),
    GeneratorEntry::current(&QUADRATIC_EQUATION_3_GENERATOR),
    GeneratorEntry::current(&SIMULTANEOUS_EQUATION_1_GENERATOR),
];

#[cfg(test)]
mod curriculum_tests {
    use super::*;
    use crate::answer::{AnswerBinaryOperator, AnswerNode};
    use crate::effort::OperationWeights;
    use crate::generator::{generate_worksheet_request, AnswerConditionedCandidateSource};
    use crate::model::{AnswerInputInterface, EditorStructure, GenerateWorksheetRequest};
    use crate::schema::SCHEMA_VERSION;

    #[test]
    fn linear_answer_support_matches_requested_domain() {
        let integer = linear_solution_domain(LinearEquationMode::IntegerSolution);
        assert_eq!(integer.len(), 31);
        assert_eq!(integer.first().unwrap().numerator(), -15);
        assert_eq!(integer.last().unwrap().numerator(), 15);
        assert!(integer.iter().all(|value| value.denominator() == 1));

        let rational = linear_solution_domain(LinearEquationMode::RationalSolution);
        let mut unique = rational.to_vec();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), rational.len());
        assert_eq!(rational.iter().filter(|value| value.is_zero()).count(), 1);
        for value in rational {
            match value.denominator() {
                1 => assert!(value.numerator().abs() <= 15),
                2 => assert!(value.numerator().unsigned_abs() <= 20),
                3..=12 => assert!(value.numerator().unsigned_abs() <= 15),
                other => panic!("unexpected reduced denominator {other}"),
            }
        }
    }

    #[test]
    fn every_linear_answer_support_value_can_generate_an_equation() {
        for generator in [&LINEAR_EQUATION_1_GENERATOR, &LINEAR_EQUATION_2_GENERATOR] {
            let mut rng = DeterministicRng::from_seed("AllAns7");
            let weights = OperationWeights::default();
            for answer in linear_answer_domain(generator.mode) {
                let generated = (1_u32..=2_000).find_map(|ordinal| {
                    generator
                        .draw_candidate_for_answer(&mut rng, ordinal, &weights, answer)
                        .expect("candidate construction must preserve the problem contract")
                });
                let problem = generated.unwrap_or_else(|| {
                    panic!("could not generate an equation for answer {answer:?}")
                });
                assert_eq!(problem.canonical_answer(), answer);
            }
        }
    }

    #[test]
    fn linear_constant_support_contains_zero_exactly_once() {
        let integer = linear_integer_domain_with_zero();
        assert_eq!(integer.len(), 31);
        assert_eq!(integer.iter().filter(|value| value.is_zero()).count(), 1);
        assert_eq!(integer.first().unwrap().numerator(), -15);
        assert_eq!(integer.last().unwrap().numerator(), 15);

        let rational = linear_rational_domain_with_zero();
        assert_eq!(rational.len(), linear_rational_domain().len() + 1);
        assert_eq!(rational.iter().filter(|value| value.is_zero()).count(), 1);
    }

    #[test]
    fn quadratic_one_uses_only_the_two_requested_square_forms() {
        for seed in ["A1b2", "M7x9", "Q4r6"] {
            let worksheet = generate_worksheet_request(&GenerateWorksheetRequest {
                schema_version: SCHEMA_VERSION,
                numeric_theme_id: THEME_ID_QUADRATIC_EQUATION_1,
                seed: seed.to_owned(),
                difficulty: crate::identity::Difficulty::try_from(3).unwrap(),
                timeout_ms: None,
                max_attempts: None,
            })
            .unwrap();
            for problem in worksheet.into_problems() {
                let ProblemPrompt::QuadraticEquation { form, a, b, c } = problem.prompt() else {
                    panic!("quadratic(1) prompt");
                };
                assert!(b.is_zero());
                assert!(a.is_integer() && (1..=9).contains(&a.numerator()));
                match form {
                    QuadraticEquationForm::SquareEqualsConstant => assert!(c.numerator() > 0),
                    QuadraticEquationForm::SquarePlusConstantZero => assert!(c.numerator() < 0),
                    _ => panic!("quadratic(1) emitted an unsupported form"),
                }
                let square_value = c.numerator().unsigned_abs() / a.numerator().unsigned_abs();
                let integer_root = (1_u64..=16).find(|root| root * root == square_value);
                if let Some(root) = integer_root {
                    assert!((1..=16).contains(&root));
                } else {
                    assert!((2..=30).contains(&square_value));
                }
                assert!(matches!(
                    problem.canonical_answer(),
                    AnswerNode::PlusMinus(_)
                ));
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
                .expect("candidate construction must preserve the problem contract")
                .expect("every declared quadratic(1) answer must construct a problem");
            assert_eq!(problem.canonical_answer(), answer);
        }
    }

    #[test]
    fn quadratic_two_is_reverse_generated_from_two_integer_roots() {
        let worksheet = generate_worksheet_request(&GenerateWorksheetRequest {
            schema_version: SCHEMA_VERSION,
            numeric_theme_id: THEME_ID_QUADRATIC_EQUATION_2,
            seed: "A1b2".to_owned(),
            difficulty: crate::identity::Difficulty::try_from(3).unwrap(),
            timeout_ms: None,
            max_attempts: None,
        })
        .unwrap();
        for problem in worksheet.into_problems() {
            let ProblemPrompt::QuadraticEquation { form, a, b, c } = problem.prompt() else {
                panic!("quadratic(2) prompt");
            };
            assert_eq!(*form, QuadraticEquationForm::FactoredScale);
            assert_eq!(*a, RationalCoefficient::new(1, 1).unwrap());
            match problem.canonical_answer() {
                AnswerNode::Integer(root) => {
                    assert_eq!(*b, RationalCoefficient::new(-2 * root, 1).unwrap());
                    assert_eq!(*c, RationalCoefficient::new(root * root, 1).unwrap());
                }
                AnswerNode::Tuple(roots) => {
                    assert_eq!(roots.len(), 2);
                    let (AnswerNode::Integer(r1), AnswerNode::Integer(r2)) = (&roots[0], &roots[1])
                    else {
                        panic!("quadratic(2) roots must be integers");
                    };
                    assert_ne!(r1, r2);
                    assert_eq!(*b, RationalCoefficient::new(-(r1 + r2), 1).unwrap());
                    assert_eq!(*c, RationalCoefficient::new(r1 * r2, 1).unwrap());
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
                numeric_theme_id: THEME_ID_QUADRATIC_EQUATION_3,
                seed: seed.to_owned(),
                difficulty: crate::identity::Difficulty::try_from(3).unwrap(),
                timeout_ms: None,
                max_attempts: None,
            })
            .unwrap();
            for problem in worksheet.into_problems() {
                let ProblemPrompt::QuadraticEquation { form, a, b, c } = problem.prompt() else {
                    panic!("quadratic(3) prompt");
                };
                assert_eq!(*form, QuadraticEquationForm::Standard);
                saw_fraction_coefficient |= !a.is_integer() || !b.is_integer() || !c.is_integer();
                let (constant, radical_coefficient, radicand, denominator) =
                    quadratic_formula_bounds(problem.canonical_answer())
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
    fn simultaneous_equation_one_reverse_generates_bounded_unique_integer_solutions() {
        for seed in ["A1b2", "M7x9", "Q4r6", "Z8k3"] {
            let worksheet = generate_worksheet_request(&GenerateWorksheetRequest {
                schema_version: SCHEMA_VERSION,
                numeric_theme_id: THEME_ID_SIMULTANEOUS_EQUATION_1,
                seed: seed.to_owned(),
                difficulty: crate::identity::Difficulty::try_from(3).unwrap(),
                timeout_ms: None,
                max_attempts: None,
            })
            .unwrap();
            assert_eq!(
                worksheet.problems().len(),
                SIMULTANEOUS_EQUATION_1_REGISTRATION
                    .layout()
                    .problem_count()
            );
            for problem in worksheet.into_problems() {
                let ProblemPrompt::SimultaneousEquation { a, b, c, d, e, f } = problem.prompt()
                else {
                    panic!("simultaneous-equation(1) prompt");
                };
                assert!([a, b, c, d, e, f]
                    .iter()
                    .all(|value| value.unsigned_abs() <= 15));
                assert!(*a != 0 && *b != 0 && *d != 0 && *e != 0);
                assert_ne!(a * e - b * d, 0);
                let AnswerNode::Tuple(values) = problem.canonical_answer() else {
                    panic!("simultaneous-equation(1) answer must be an ordered pair");
                };
                assert_eq!(values.len(), 2);
                let (AnswerNode::Integer(x), AnswerNode::Integer(y)) = (&values[0], &values[1])
                else {
                    panic!("simultaneous-equation(1) coordinates must be integers");
                };
                assert!(x.unsigned_abs() <= 15 && y.unsigned_abs() <= 15);
                assert_eq!(*a * x + *b * y, *c);
                assert_eq!(*d * x + *e * y, *f);
                assert!(matches!(problem.answer_schema(), AnswerSchema::OrderedPair));
                assert!(matches!(
                    problem.input_interface(),
                    AnswerInputInterface::StructuredMath { ref allowed_structures }
                        if allowed_structures == &[EditorStructure::Negative, EditorStructure::Tuple]
                ));
            }
        }
    }
}
