use crate::answer::{AnswerBinaryOperator, AnswerNode};
use crate::effort::{
    calculate_graph_effort, linear_equation_graph, quadratic_factoring_graph,
    quadratic_formula_graph, quadratic_square_graph, simultaneous_equation_graph, OperationWeights,
};
use crate::exact::gcd_u64;
use crate::generator::{GeneratorEntry, ProblemGenerator};
use crate::generator_support::{draw_signed_integer, input_interface, rational_answer};
use crate::model::{
    AnswerSchema, Problem, ProblemPrompt, QuadraticEquationForm, RationalCoefficient,
};
use crate::rng::DeterministicRng;
use crate::schema::SCHEMA_VERSION;
use crate::theme::{
    CurriculumSafetyPolicy as Safety, DedupPolicy as Dedup, SamplingLayerSpec,
    ThemeAnswerContract as AnswerContract, ThemeAnswerSchemaKind as Schema,
    ThemeInputProfile as Input, ThemePresentationPolicy as Presentation, ThemePromptKind as Prompt,
    ThemeRegistration, ThemeRegistrationSpec, ThemeTag, COMPACT_16_LAYOUT, EQUATION_PAIR_12_LAYOUT,
};
use std::sync::OnceLock;

pub const LINEAR_EQUATION_PROBLEM_COUNT: usize = COMPACT_16_LAYOUT.problem_count;
pub const LINEAR_EQUATION_COLUMNS: usize = COMPACT_16_LAYOUT.columns;
pub const LINEAR_EQUATION_ROWS: usize = COMPACT_16_LAYOUT.rows;
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
        key: "difference_of_squares",
        weight: 0,
        minimum: 2,
    },
    SamplingLayerSpec {
        key: "perfect_square",
        weight: 0,
        minimum: 2,
    },
    SamplingLayerSpec {
        key: "general",
        weight: 1,
        minimum: 0,
    },
];

pub const LINEAR_EQUATION_1_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: THEME_ID_LINEAR_EQUATION_1,
        generator_revision: GENERATOR_REVISION_LINEAR_EQUATION_1,
        skill_id: SKILL_ID_LINEAR_EQUATION_1,
        curriculum_path: &CURRICULUM_PATH_LINEAR_EQUATION_1,
        grade: Some(7),
        tags: LINEAR,
        safety: Safety::Unrestricted,
        presentation: Presentation::EQUATION,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: AnswerContract {
            prompt_kind: Prompt::LinearEquation,
            answer_schema_kind: Schema::Integer,
            input_profile: Input::LinearEquation,
        },
        layout: COMPACT_16_LAYOUT,
    })
    .with_editor_input_profile(Input::JuniorHighFull);
pub const LINEAR_EQUATION_2_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: THEME_ID_LINEAR_EQUATION_2,
        generator_revision: GENERATOR_REVISION_LINEAR_EQUATION_2,
        skill_id: SKILL_ID_LINEAR_EQUATION_2,
        curriculum_path: &CURRICULUM_PATH_LINEAR_EQUATION_2,
        grade: Some(7),
        tags: LINEAR,
        safety: Safety::Unrestricted,
        presentation: Presentation::EQUATION,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: AnswerContract {
            prompt_kind: Prompt::LinearEquation,
            answer_schema_kind: Schema::Rational,
            input_profile: Input::LinearEquation,
        },
        layout: COMPACT_16_LAYOUT,
    })
    .with_editor_input_profile(Input::JuniorHighFull);
pub const QUADRATIC_EQUATION_1_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: THEME_ID_QUADRATIC_EQUATION_1,
        generator_revision: GENERATOR_REVISION_QUADRATIC_EQUATION_1,
        skill_id: SKILL_ID_QUADRATIC_EQUATION_1,
        curriculum_path: &CURRICULUM_PATH_QUADRATIC_EQUATION_1,
        grade: Some(9),
        tags: QUADRATIC,
        safety: Safety::Unrestricted,
        presentation: Presentation::EQUATION,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: AnswerContract {
            prompt_kind: Prompt::QuadraticEquation,
            answer_schema_kind: Schema::Algebraic,
            input_profile: Input::QuadraticEquation,
        },
        layout: COMPACT_16_LAYOUT,
    })
    .with_editor_input_profile(Input::JuniorHighFull);
pub const QUADRATIC_EQUATION_2_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: THEME_ID_QUADRATIC_EQUATION_2,
        generator_revision: GENERATOR_REVISION_QUADRATIC_EQUATION_2,
        skill_id: SKILL_ID_QUADRATIC_EQUATION_2,
        curriculum_path: &CURRICULUM_PATH_QUADRATIC_EQUATION_2,
        grade: Some(9),
        tags: QUADRATIC,
        safety: Safety::Unrestricted,
        presentation: Presentation::EQUATION,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: AnswerContract {
            prompt_kind: Prompt::QuadraticEquation,
            answer_schema_kind: Schema::Algebraic,
            input_profile: Input::QuadraticEquation,
        },
        layout: COMPACT_16_LAYOUT,
    })
    .with_editor_input_profile(Input::JuniorHighFull);
pub const QUADRATIC_EQUATION_3_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: THEME_ID_QUADRATIC_EQUATION_3,
        generator_revision: GENERATOR_REVISION_QUADRATIC_EQUATION_3,
        skill_id: SKILL_ID_QUADRATIC_EQUATION_3,
        curriculum_path: &CURRICULUM_PATH_QUADRATIC_EQUATION_3,
        grade: Some(9),
        tags: QUADRATIC,
        safety: Safety::Unrestricted,
        presentation: Presentation::EQUATION,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: AnswerContract {
            prompt_kind: Prompt::QuadraticEquation,
            answer_schema_kind: Schema::Algebraic,
            input_profile: Input::QuadraticEquation,
        },
        layout: COMPACT_16_LAYOUT,
    })
    .with_editor_input_profile(Input::JuniorHighFull);
pub const SIMULTANEOUS_EQUATION_1_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: THEME_ID_SIMULTANEOUS_EQUATION_1,
        generator_revision: GENERATOR_REVISION_SIMULTANEOUS_EQUATION_1,
        skill_id: SKILL_ID_SIMULTANEOUS_EQUATION_1,
        curriculum_path: &CURRICULUM_PATH_SIMULTANEOUS_EQUATION_1,
        grade: Some(8),
        tags: SIMULTANEOUS,
        safety: Safety::Unrestricted,
        presentation: Presentation::EQUATION,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: AnswerContract {
            prompt_kind: Prompt::SimultaneousEquation,
            answer_schema_kind: Schema::OrderedPair,
            input_profile: Input::SimultaneousEquation,
        },
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
pub(crate) struct SimultaneousEquationGenerator {
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
    fn sampling_layers(&self) -> Option<&'static [SamplingLayerSpec]> {
        (self.mode == QuadraticEquationMode::Factoring).then_some(&QUADRATIC_FACTORING_LAYERS)
    }
    fn sampling_layer(&self, problem: &Problem) -> Option<usize> {
        if self.mode != QuadraticEquationMode::Factoring {
            return None;
        }
        let ProblemPrompt::QuadraticEquation { b, c, .. } = &problem.prompt else {
            return None;
        };
        if b.numerator == 0 && c.numerator < 0 {
            return Some(0);
        }
        let discriminant = b
            .numerator
            .checked_mul(b.numerator)?
            .checked_sub(4_i64.checked_mul(c.numerator)?)?;
        if b.denominator == 1 && c.denominator == 1 && discriminant == 0 {
            Some(1)
        } else {
            Some(2)
        }
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
        input_interface: input_interface(Input::SimultaneousEquation),
        answer_schema: AnswerSchema::OrderedPair,
        canonical_answer,
        worked_solution: None,
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
            let second = draw_signed_integer(rng, 9);
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
        input_interface: input_interface(Input::QuadraticEquation),
        answer_schema: AnswerSchema::Algebraic,
        canonical_answer,
        worked_solution: None,
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
        input_interface: input_interface(Input::LinearEquation),
        answer_schema,
        canonical_answer,
        worked_solution: None,
        solution_graph,
        operation_vector: result.operation_vector,
        effort: result.value,
    }
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
