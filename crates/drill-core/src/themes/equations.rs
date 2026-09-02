use crate::answer::{AnswerBinaryOperator, AnswerNode};
use crate::effort::{
    linear_expression_equation_plan, quadratic_equation_plan, simultaneous_equation_plan,
    EffortModel, OperationWeights,
};
use crate::error::GenerationError;
use crate::exact::{gcd_u64, square_free_sqrt_decomposition};
use crate::generator::{
    AnswerConditionedCandidateSource, GeneratorEntry, LayeredCandidateSource, ProblemGenerator,
    RandomCandidateSource, SamplingStrategy, SelectionDedup,
};
use crate::generator_support::{draw_signed_integer, exact_decimal_rational, rational_answer};
use crate::model::{
    AnswerSchema, LinearEquationSurface, LinearExpression, LinearScalar, LinearVariable, Problem,
    ProblemPrompt, QuadraticEquationSurface, QuadraticExpression, QuadraticSolveMethod,
    RationalCoefficient, SimultaneousSolveMethod,
};
use crate::rng::DeterministicRng;
use crate::theme::{
    CurriculumSafetyPolicy as Safety, CurriculumUnit, DedupPolicy as Dedup, SamplingLayerSpec,
    SchoolGrade, ThemeAnswerContract as AnswerContract, ThemeInputProfile as Input,
    ThemePresentationPolicy as Presentation, ThemeRegistration, ThemeRegistrationSpec, ThemeTag,
    COMPACT_16_LAYOUT, EQUATION_PAIR_12_LAYOUT,
};
use std::sync::OnceLock;

pub const THEME_ID_LINEAR_EQUATION_1: u32 = 2;
pub const THEME_ID_LINEAR_EQUATION_2: u32 = 3;
pub const THEME_ID_LINEAR_EQUATION_SIMPLE: u32 = 69;
pub const THEME_ID_LINEAR_EQUATION_3: u32 = 70;
pub const THEME_ID_QUADRATIC_EQUATION_1: u32 = 14;
pub const THEME_ID_QUADRATIC_EQUATION_2: u32 = 15;
pub const THEME_ID_QUADRATIC_EQUATION_3: u32 = 16;
pub const THEME_ID_QUADRATIC_EQUATION_4: u32 = 74;
pub const THEME_ID_SIMULTANEOUS_EQUATION_ELIMINATION: u32 = 19;
pub const THEME_ID_SIMULTANEOUS_EQUATION_SUBSTITUTION: u32 = 71;
pub const THEME_ID_SIMULTANEOUS_EQUATION_SUMMARY_1: u32 = 72;
pub const THEME_ID_SIMULTANEOUS_EQUATION_SUMMARY_2: u32 = 73;
pub const GENERATOR_REVISION_LINEAR_EQUATION_1: u32 = 9;
pub const GENERATOR_REVISION_LINEAR_EQUATION_2: u32 = 9;
pub const GENERATOR_REVISION_LINEAR_EQUATION_SIMPLE: u32 = 1;
pub const GENERATOR_REVISION_LINEAR_EQUATION_3: u32 = 1;
pub const GENERATOR_REVISION_QUADRATIC_EQUATION_1: u32 = 4;
pub const GENERATOR_REVISION_QUADRATIC_EQUATION_2: u32 = 5;
pub const GENERATOR_REVISION_QUADRATIC_EQUATION_3: u32 = 4;
pub const GENERATOR_REVISION_QUADRATIC_EQUATION_4: u32 = 1;
pub const GENERATOR_REVISION_SIMULTANEOUS_EQUATION_ELIMINATION: u32 = 4;
pub const GENERATOR_REVISION_SIMULTANEOUS_EQUATION_SUBSTITUTION: u32 = 1;
pub const GENERATOR_REVISION_SIMULTANEOUS_EQUATION_SUMMARY_1: u32 = 1;
pub const GENERATOR_REVISION_SIMULTANEOUS_EQUATION_SUMMARY_2: u32 = 1;
pub const SKILL_ID_LINEAR_EQUATION_1: &str = "jp.grade7.equation.linear.1";
pub const SKILL_ID_LINEAR_EQUATION_2: &str = "jp.grade7.equation.linear.2";
pub const SKILL_ID_LINEAR_EQUATION_SIMPLE: &str = "jp.grade7.equation.linear.simple";
pub const SKILL_ID_LINEAR_EQUATION_3: &str = "jp.grade7.equation.linear.3";
pub const SKILL_ID_QUADRATIC_EQUATION_1: &str = "jp.grade9.equation.quadratic.1";
pub const SKILL_ID_QUADRATIC_EQUATION_2: &str = "jp.grade9.equation.quadratic.2";
pub const SKILL_ID_QUADRATIC_EQUATION_3: &str = "jp.grade9.equation.quadratic.3";
pub const SKILL_ID_QUADRATIC_EQUATION_4: &str = "jp.grade9.equation.quadratic.4";
pub const SKILL_ID_SIMULTANEOUS_EQUATION_ELIMINATION: &str =
    "jp.grade8.equation.simultaneous.elimination";
pub const SKILL_ID_SIMULTANEOUS_EQUATION_SUBSTITUTION: &str =
    "jp.grade8.equation.simultaneous.substitution";
pub const SKILL_ID_SIMULTANEOUS_EQUATION_SUMMARY_1: &str =
    "jp.grade8.equation.simultaneous.summary.1";
pub const SKILL_ID_SIMULTANEOUS_EQUATION_SUMMARY_2: &str =
    "jp.grade8.equation.simultaneous.summary.2";
pub const CURRICULUM_PATH_LINEAR_EQUATION_SIMPLE: [&str; 4] =
    ["root", "中学1年生", "一次方程式", "簡単な一次方程式"];
pub const CURRICULUM_PATH_LINEAR_EQUATION_1: [&str; 4] =
    ["root", "中学1年生", "一次方程式", "一次方程式(1)：基本形"];
pub const CURRICULUM_PATH_LINEAR_EQUATION_2: [&str; 4] = [
    "root",
    "中学1年生",
    "一次方程式",
    "一次方程式(2)：括弧・整数係数中心",
];
pub const CURRICULUM_PATH_LINEAR_EQUATION_3: [&str; 4] = [
    "root",
    "中学1年生",
    "一次方程式",
    "一次方程式(3)：括弧・分数・小数係数",
];
pub const CURRICULUM_PATH_QUADRATIC_EQUATION_1: [&str; 4] =
    ["root", "中学3年生", "二次方程式", "二次方程式(1)"];
pub const CURRICULUM_PATH_QUADRATIC_EQUATION_2: [&str; 4] =
    ["root", "中学3年生", "二次方程式", "二次方程式(2)"];
pub const CURRICULUM_PATH_QUADRATIC_EQUATION_3: [&str; 4] =
    ["root", "中学3年生", "二次方程式", "二次方程式(3)"];
pub const CURRICULUM_PATH_QUADRATIC_EQUATION_4: [&str; 4] =
    ["root", "中学3年生", "二次方程式", "二次方程式(4)"];
pub const CURRICULUM_PATH_SIMULTANEOUS_EQUATION_ELIMINATION: [&str; 4] =
    ["root", "中学2年生", "連立方程式", "連立方程式（加減法）"];
pub const CURRICULUM_PATH_SIMULTANEOUS_EQUATION_SUBSTITUTION: [&str; 4] =
    ["root", "中学2年生", "連立方程式", "連立方程式（代入法）"];
pub const CURRICULUM_PATH_SIMULTANEOUS_EQUATION_SUMMARY_1: [&str; 4] =
    ["root", "中学2年生", "連立方程式", "連立方程式（まとめ(1)）"];
pub const CURRICULUM_PATH_SIMULTANEOUS_EQUATION_SUMMARY_2: [&str; 4] =
    ["root", "中学2年生", "連立方程式", "連立方程式（まとめ(2)）"];

pub const CURRICULUM_UNIT_LINEAR_EQUATION: CurriculumUnit =
    CurriculumUnit::new("linear-equation", "一次方程式");
pub const CURRICULUM_UNIT_QUADRATIC_EQUATION: CurriculumUnit =
    CurriculumUnit::new("quadratic-equation", "二次方程式");
pub const CURRICULUM_UNIT_SIMULTANEOUS_EQUATION: CurriculumUnit =
    CurriculumUnit::new("simultaneous-equation", "連立方程式");

const LINEAR: &[ThemeTag] = &[ThemeTag::Equations, ThemeTag::LinearEquation];
const QUADRATIC: &[ThemeTag] = &[ThemeTag::Equations, ThemeTag::QuadraticEquation];
const SIMULTANEOUS: &[ThemeTag] = &[ThemeTag::Equations, ThemeTag::SimultaneousEquation];

pub const QUADRATIC_SQUARE_ROOT_LAYERS: [SamplingLayerSpec; 2] = [
    SamplingLayerSpec {
        weight: 1,
        minimum: 4,
    },
    SamplingLayerSpec {
        weight: 1,
        minimum: 4,
    },
];

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

pub const QUADRATIC_COMPREHENSIVE_LAYERS: [SamplingLayerSpec; 9] = [SamplingLayerSpec {
    weight: 1,
    minimum: 1,
}; 9];

pub const LINEAR_EQUATION_SIMPLE_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_LINEAR_EQUATION_SIMPLE),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_LINEAR_EQUATION_SIMPLE,
        ),
        skill_id: SKILL_ID_LINEAR_EQUATION_SIMPLE,
        curriculum_path: &CURRICULUM_PATH_LINEAR_EQUATION_SIMPLE,
        grade: Some(SchoolGrade::JuniorHigh1),
        tags: LINEAR,
        safety: Safety::Unrestricted,
        presentation: Presentation::EQUATION,
        dedup: Dedup::PreserveOperandOrder,
        answer_contract: AnswerContract::LinearInteger,
        layout: COMPACT_16_LAYOUT,
    })
    .with_curriculum_unit(CURRICULUM_UNIT_LINEAR_EQUATION)
    .with_editor_input_profile(Input::JuniorHighFull);

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
    .with_curriculum_unit(CURRICULUM_UNIT_LINEAR_EQUATION)
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
    .with_curriculum_unit(CURRICULUM_UNIT_LINEAR_EQUATION)
    .with_editor_input_profile(Input::JuniorHighFull);
pub const LINEAR_EQUATION_3_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_LINEAR_EQUATION_3),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_LINEAR_EQUATION_3,
        ),
        skill_id: SKILL_ID_LINEAR_EQUATION_3,
        curriculum_path: &CURRICULUM_PATH_LINEAR_EQUATION_3,
        grade: Some(SchoolGrade::JuniorHigh1),
        tags: LINEAR,
        safety: Safety::Unrestricted,
        presentation: Presentation::EQUATION,
        dedup: Dedup::PreserveOperandOrder,
        answer_contract: AnswerContract::LinearRational,
        layout: COMPACT_16_LAYOUT,
    })
    .with_curriculum_unit(CURRICULUM_UNIT_LINEAR_EQUATION)
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
    .with_curriculum_unit(CURRICULUM_UNIT_QUADRATIC_EQUATION)
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
    .with_curriculum_unit(CURRICULUM_UNIT_QUADRATIC_EQUATION)
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
    .with_curriculum_unit(CURRICULUM_UNIT_QUADRATIC_EQUATION)
    .with_editor_input_profile(Input::JuniorHighFull);
pub const QUADRATIC_EQUATION_4_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_QUADRATIC_EQUATION_4),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_QUADRATIC_EQUATION_4,
        ),
        skill_id: SKILL_ID_QUADRATIC_EQUATION_4,
        curriculum_path: &CURRICULUM_PATH_QUADRATIC_EQUATION_4,
        grade: Some(SchoolGrade::JuniorHigh3),
        tags: QUADRATIC,
        safety: Safety::Unrestricted,
        presentation: Presentation::EQUATION,
        dedup: Dedup::PreserveOperandOrder,
        answer_contract: AnswerContract::QuadraticAlgebraic,
        layout: COMPACT_16_LAYOUT,
    })
    .with_curriculum_unit(CURRICULUM_UNIT_QUADRATIC_EQUATION)
    .with_editor_input_profile(Input::JuniorHighFull);
pub const SIMULTANEOUS_EQUATION_ELIMINATION_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_SIMULTANEOUS_EQUATION_ELIMINATION),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_SIMULTANEOUS_EQUATION_ELIMINATION,
        ),
        skill_id: SKILL_ID_SIMULTANEOUS_EQUATION_ELIMINATION,
        curriculum_path: &CURRICULUM_PATH_SIMULTANEOUS_EQUATION_ELIMINATION,
        grade: Some(SchoolGrade::JuniorHigh2),
        tags: SIMULTANEOUS,
        safety: Safety::Unrestricted,
        presentation: Presentation::EQUATION,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: AnswerContract::SimultaneousPair,
        layout: EQUATION_PAIR_12_LAYOUT,
    })
    .with_curriculum_unit(CURRICULUM_UNIT_SIMULTANEOUS_EQUATION)
    .with_editor_input_profile(Input::JuniorHighFull);
pub const SIMULTANEOUS_EQUATION_SUBSTITUTION_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_SIMULTANEOUS_EQUATION_SUBSTITUTION),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_SIMULTANEOUS_EQUATION_SUBSTITUTION,
        ),
        skill_id: SKILL_ID_SIMULTANEOUS_EQUATION_SUBSTITUTION,
        curriculum_path: &CURRICULUM_PATH_SIMULTANEOUS_EQUATION_SUBSTITUTION,
        grade: Some(SchoolGrade::JuniorHigh2),
        tags: SIMULTANEOUS,
        safety: Safety::Unrestricted,
        presentation: Presentation::EQUATION,
        dedup: Dedup::PreserveOperandOrder,
        answer_contract: AnswerContract::SimultaneousPair,
        layout: EQUATION_PAIR_12_LAYOUT,
    })
    .with_curriculum_unit(CURRICULUM_UNIT_SIMULTANEOUS_EQUATION)
    .with_editor_input_profile(Input::JuniorHighFull);
pub const SIMULTANEOUS_EQUATION_SUMMARY_1_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_SIMULTANEOUS_EQUATION_SUMMARY_1),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_SIMULTANEOUS_EQUATION_SUMMARY_1,
        ),
        skill_id: SKILL_ID_SIMULTANEOUS_EQUATION_SUMMARY_1,
        curriculum_path: &CURRICULUM_PATH_SIMULTANEOUS_EQUATION_SUMMARY_1,
        grade: Some(SchoolGrade::JuniorHigh2),
        tags: SIMULTANEOUS,
        safety: Safety::Unrestricted,
        presentation: Presentation::EQUATION,
        dedup: Dedup::PreserveOperandOrder,
        answer_contract: AnswerContract::SimultaneousPair,
        layout: EQUATION_PAIR_12_LAYOUT,
    })
    .with_curriculum_unit(CURRICULUM_UNIT_SIMULTANEOUS_EQUATION)
    .with_editor_input_profile(Input::JuniorHighFull);
pub const SIMULTANEOUS_EQUATION_SUMMARY_2_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_SIMULTANEOUS_EQUATION_SUMMARY_2),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_SIMULTANEOUS_EQUATION_SUMMARY_2,
        ),
        skill_id: SKILL_ID_SIMULTANEOUS_EQUATION_SUMMARY_2,
        curriculum_path: &CURRICULUM_PATH_SIMULTANEOUS_EQUATION_SUMMARY_2,
        grade: Some(SchoolGrade::JuniorHigh2),
        tags: SIMULTANEOUS,
        safety: Safety::Unrestricted,
        presentation: Presentation::EQUATION,
        dedup: Dedup::PreserveOperandOrder,
        answer_contract: AnswerContract::SimultaneousPair,
        layout: EQUATION_PAIR_12_LAYOUT,
    })
    .with_curriculum_unit(CURRICULUM_UNIT_SIMULTANEOUS_EQUATION)
    .with_editor_input_profile(Input::JuniorHighFull);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum LinearSolutionDomain {
    Integer,
    Rational,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LinearSurfaceMode {
    Simple,
    Basic,
    ParenthesizedInteger,
    Comprehensive,
}

#[derive(Debug)]
pub(crate) struct LinearEquationGenerator {
    registration: &'static ThemeRegistration,
    solution_domain: LinearSolutionDomain,
    surface_mode: LinearSurfaceMode,
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
        linear_answer_domain(self.solution_domain)
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
        let expressions = match self.surface_mode {
            LinearSurfaceMode::Simple => draw_simple_linear_equation(rng, solution),
            LinearSurfaceMode::Basic => draw_basic_linear_equation(rng, solution),
            LinearSurfaceMode::ParenthesizedInteger => {
                draw_parenthesized_integer_equation(rng, solution)
            }
            LinearSurfaceMode::Comprehensive => draw_comprehensive_linear_equation(rng, solution),
        };
        let Some((left, right)) = expressions else {
            return Ok(None);
        };
        linear_equation_problem(
            ordinal,
            self.registration,
            self.solution_domain,
            left,
            right,
            solution,
            weights,
        )
        .map(Some)
    }
}

pub const SIMULTANEOUS_BASIC_LAYERS: [SamplingLayerSpec; 2] = [
    SamplingLayerSpec {
        weight: 1,
        minimum: 2,
    },
    SamplingLayerSpec {
        weight: 1,
        minimum: 2,
    },
];

pub const SIMULTANEOUS_TRANSFORMED_LAYERS: [SamplingLayerSpec; 6] = [
    SamplingLayerSpec {
        weight: 1,
        minimum: 1,
    },
    SamplingLayerSpec {
        weight: 1,
        minimum: 1,
    },
    SamplingLayerSpec {
        weight: 1,
        minimum: 1,
    },
    SamplingLayerSpec {
        weight: 1,
        minimum: 1,
    },
    SamplingLayerSpec {
        weight: 1,
        minimum: 1,
    },
    SamplingLayerSpec {
        weight: 1,
        minimum: 1,
    },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SimultaneousEquationMode {
    Elimination,
    Substitution,
    SummaryBasic,
    SummaryTransformed,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum SimultaneousSurfaceTransform {
    Parentheses,
    Fraction,
    Decimal,
}

#[derive(Debug)]
pub(crate) struct SimultaneousEquationGenerator {
    registration: &'static ThemeRegistration,
    mode: SimultaneousEquationMode,
}

impl ProblemGenerator for SimultaneousEquationGenerator {
    fn registration(&self) -> &'static ThemeRegistration {
        self.registration
    }

    fn sampling_strategy(&self) -> Result<SamplingStrategy<'_>, crate::error::SamplingError> {
        match self.mode {
            SimultaneousEquationMode::Elimination | SimultaneousEquationMode::Substitution => Ok(
                SamplingStrategy::random(self, SelectionDedup::AllowDuplicates),
            ),
            SimultaneousEquationMode::SummaryBasic
            | SimultaneousEquationMode::SummaryTransformed => SamplingStrategy::layered(
                self,
                SelectionDedup::AllowDuplicates,
                self.registration.layout().problem_count(),
            ),
        }
    }
}

impl RandomCandidateSource for SimultaneousEquationGenerator {
    fn draw_candidate(
        &self,
        rng: &mut DeterministicRng,
        ordinal: u32,
        weights: &OperationWeights,
    ) -> Result<Option<Problem>, GenerationError> {
        simultaneous_equation_problem(self.registration, self.mode, rng, ordinal, weights)
            .transpose()
    }
}

impl LayeredCandidateSource for SimultaneousEquationGenerator {
    fn layers(&self) -> &'static [SamplingLayerSpec] {
        match self.mode {
            SimultaneousEquationMode::SummaryBasic => &SIMULTANEOUS_BASIC_LAYERS,
            SimultaneousEquationMode::SummaryTransformed => &SIMULTANEOUS_TRANSFORMED_LAYERS,
            _ => &[],
        }
    }

    fn draw_candidate(
        &self,
        rng: &mut DeterministicRng,
        ordinal: u32,
        weights: &OperationWeights,
    ) -> Result<Option<Problem>, GenerationError> {
        simultaneous_equation_problem(self.registration, self.mode, rng, ordinal, weights)
            .transpose()
    }

    fn layer_of(&self, problem: &Problem) -> usize {
        let ProblemPrompt::SimultaneousEquation {
            equations,
            solve_method,
        } = problem.prompt()
        else {
            unreachable!("simultaneous generator always emits simultaneous prompts");
        };
        let method_index = match solve_method {
            SimultaneousSolveMethod::Elimination => 0,
            SimultaneousSolveMethod::Substitution => 1,
        };
        match self.mode {
            SimultaneousEquationMode::SummaryBasic => method_index,
            SimultaneousEquationMode::SummaryTransformed => {
                let transform_index = match simultaneous_surface_transform(equations) {
                    Some(SimultaneousSurfaceTransform::Parentheses) => 0,
                    Some(SimultaneousSurfaceTransform::Fraction) => 1,
                    Some(SimultaneousSurfaceTransform::Decimal) => 2,
                    None => unreachable!("summary(2) always emits a transformed surface"),
                };
                method_index * 3 + transform_index
            }
            _ => unreachable!("only summary simultaneous generators use layered sampling"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum QuadraticEquationMode {
    SquareRoot,
    Factoring,
    Formula,
    Comprehensive,
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
            QuadraticEquationMode::SquareRoot
            | QuadraticEquationMode::Factoring
            | QuadraticEquationMode::Comprehensive => SamplingStrategy::layered(
                self,
                SelectionDedup::AllowDuplicates,
                self.registration.layout().problem_count(),
            ),
            QuadraticEquationMode::Formula => Ok(SamplingStrategy::random(
                self,
                SelectionDedup::AllowDuplicates,
            )),
        }
    }
}

impl RandomCandidateSource for QuadraticEquationGenerator {
    fn draw_candidate(
        &self,
        rng: &mut DeterministicRng,
        ordinal: u32,
        weights: &OperationWeights,
    ) -> Result<Option<Problem>, GenerationError> {
        quadratic_equation_problem(self.registration, self.mode, rng, ordinal, weights).transpose()
    }
}

impl LayeredCandidateSource for QuadraticEquationGenerator {
    fn layers(&self) -> &'static [SamplingLayerSpec] {
        match self.mode {
            QuadraticEquationMode::SquareRoot => &QUADRATIC_SQUARE_ROOT_LAYERS,
            QuadraticEquationMode::Factoring => &QUADRATIC_FACTORING_LAYERS,
            QuadraticEquationMode::Comprehensive => &QUADRATIC_COMPREHENSIVE_LAYERS,
            QuadraticEquationMode::Formula => {
                unreachable!("formula generator does not use layered sampling")
            }
        }
    }

    fn draw_candidate(
        &self,
        rng: &mut DeterministicRng,
        ordinal: u32,
        weights: &OperationWeights,
    ) -> Result<Option<Problem>, GenerationError> {
        quadratic_equation_problem(self.registration, self.mode, rng, ordinal, weights).transpose()
    }

    fn layer_of(&self, problem: &Problem) -> usize {
        let ProblemPrompt::QuadraticEquation {
            equation,
            solve_method,
        } = problem.prompt()
        else {
            unreachable!("quadratic generator always emits quadratic prompts");
        };
        match self.mode {
            QuadraticEquationMode::SquareRoot => {
                usize::from(quadratic_equation_has_shifted_square(equation))
            }
            QuadraticEquationMode::Factoring => {
                let (_, b, c) = crate::semantics::normalize_quadratic_equation(equation)
                    .expect("factoring surface must normalize");
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
            QuadraticEquationMode::Comprehensive => {
                let method_index = match solve_method {
                    QuadraticSolveMethod::SquareRoot => 0,
                    QuadraticSolveMethod::Factoring => 1,
                    QuadraticSolveMethod::Formula => 2,
                };
                let transform_index = match quadratic_surface_transform(equation) {
                    Some(QuadraticSurfaceTransform::Parentheses) => 0,
                    Some(QuadraticSurfaceTransform::Fraction) => 1,
                    Some(QuadraticSurfaceTransform::Decimal) => 2,
                    None => unreachable!("quadratic(4) always emits a transformed surface"),
                };
                method_index * 3 + transform_index
            }
            QuadraticEquationMode::Formula => {
                unreachable!("formula generator does not use layered sampling")
            }
        }
    }
}

pub(crate) static LINEAR_EQUATION_SIMPLE_GENERATOR: LinearEquationGenerator =
    LinearEquationGenerator {
        registration: &LINEAR_EQUATION_SIMPLE_REGISTRATION,
        solution_domain: LinearSolutionDomain::Integer,
        surface_mode: LinearSurfaceMode::Simple,
    };
pub(crate) static LINEAR_EQUATION_1_GENERATOR: LinearEquationGenerator = LinearEquationGenerator {
    registration: &LINEAR_EQUATION_1_REGISTRATION,
    solution_domain: LinearSolutionDomain::Integer,
    surface_mode: LinearSurfaceMode::Basic,
};
pub(crate) static LINEAR_EQUATION_2_GENERATOR: LinearEquationGenerator = LinearEquationGenerator {
    registration: &LINEAR_EQUATION_2_REGISTRATION,
    solution_domain: LinearSolutionDomain::Rational,
    surface_mode: LinearSurfaceMode::ParenthesizedInteger,
};
pub(crate) static LINEAR_EQUATION_3_GENERATOR: LinearEquationGenerator = LinearEquationGenerator {
    registration: &LINEAR_EQUATION_3_REGISTRATION,
    solution_domain: LinearSolutionDomain::Rational,
    surface_mode: LinearSurfaceMode::Comprehensive,
};
pub(crate) static SIMULTANEOUS_EQUATION_ELIMINATION_GENERATOR: SimultaneousEquationGenerator =
    SimultaneousEquationGenerator {
        registration: &SIMULTANEOUS_EQUATION_ELIMINATION_REGISTRATION,
        mode: SimultaneousEquationMode::Elimination,
    };
pub(crate) static SIMULTANEOUS_EQUATION_SUBSTITUTION_GENERATOR: SimultaneousEquationGenerator =
    SimultaneousEquationGenerator {
        registration: &SIMULTANEOUS_EQUATION_SUBSTITUTION_REGISTRATION,
        mode: SimultaneousEquationMode::Substitution,
    };
pub(crate) static SIMULTANEOUS_EQUATION_SUMMARY_1_GENERATOR: SimultaneousEquationGenerator =
    SimultaneousEquationGenerator {
        registration: &SIMULTANEOUS_EQUATION_SUMMARY_1_REGISTRATION,
        mode: SimultaneousEquationMode::SummaryBasic,
    };
pub(crate) static SIMULTANEOUS_EQUATION_SUMMARY_2_GENERATOR: SimultaneousEquationGenerator =
    SimultaneousEquationGenerator {
        registration: &SIMULTANEOUS_EQUATION_SUMMARY_2_REGISTRATION,
        mode: SimultaneousEquationMode::SummaryTransformed,
    };
pub(crate) static QUADRATIC_EQUATION_1_GENERATOR: QuadraticEquationGenerator =
    QuadraticEquationGenerator {
        registration: &QUADRATIC_EQUATION_1_REGISTRATION,
        mode: QuadraticEquationMode::SquareRoot,
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
pub(crate) static QUADRATIC_EQUATION_4_GENERATOR: QuadraticEquationGenerator =
    QuadraticEquationGenerator {
        registration: &QUADRATIC_EQUATION_4_REGISTRATION,
        mode: QuadraticEquationMode::Comprehensive,
    };

pub(crate) fn linear_answer_domain(mode: LinearSolutionDomain) -> &'static [AnswerNode] {
    static INTEGER: OnceLock<Vec<AnswerNode>> = OnceLock::new();
    static RATIONAL: OnceLock<Vec<AnswerNode>> = OnceLock::new();
    match mode {
        LinearSolutionDomain::Integer => {
            INTEGER.get_or_init(|| (-15_i64..=15).map(AnswerNode::Integer).collect())
        }
        LinearSolutionDomain::Rational => RATIONAL.get_or_init(|| {
            linear_solution_domain(mode)
                .iter()
                .copied()
                .map(rational_answer)
                .collect()
        }),
    }
}

pub(crate) fn linear_solution_domain(mode: LinearSolutionDomain) -> &'static [RationalCoefficient] {
    static INTEGER: OnceLock<Vec<RationalCoefficient>> = OnceLock::new();
    static RATIONAL: OnceLock<Vec<RationalCoefficient>> = OnceLock::new();
    match mode {
        LinearSolutionDomain::Integer => INTEGER.get_or_init(|| {
            (-15_i64..=15)
                .map(|value| RationalCoefficient::new(value, 1).expect("integer solution"))
                .collect()
        }),
        LinearSolutionDomain::Rational => RATIONAL.get_or_init(|| {
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

fn integer_scalar(value: i64) -> LinearScalar {
    LinearScalar::Integer { value }
}

fn scalar_rational(value: LinearScalar) -> Option<RationalCoefficient> {
    match value {
        LinearScalar::Integer { value } => RationalCoefficient::new(value, 1),
        LinearScalar::Fraction { value } => Some(value),
        LinearScalar::ExactDecimal { coefficient, scale } => {
            exact_decimal_rational(coefficient, scale)
        }
    }
}

fn scalar_abs(value: LinearScalar) -> Option<LinearScalar> {
    match value {
        LinearScalar::Integer { value } => Some(integer_scalar(value.checked_abs()?)),
        LinearScalar::Fraction { value } => Some(LinearScalar::Fraction {
            value: RationalCoefficient::new(value.numerator().checked_abs()?, value.denominator())?,
        }),
        LinearScalar::ExactDecimal { coefficient, scale } => Some(LinearScalar::ExactDecimal {
            coefficient: coefficient.checked_abs()?,
            scale,
        }),
    }
}

fn scalar_from_rational(value: RationalCoefficient) -> Option<LinearScalar> {
    if value.is_integer() {
        return Some(integer_scalar(value.numerator()));
    }
    Some(LinearScalar::Fraction { value })
}

fn linear_affine_expression_for(
    variable_name: LinearVariable,
    coefficient: LinearScalar,
    constant: LinearScalar,
) -> Option<LinearExpression> {
    let coefficient_value = scalar_rational(coefficient)?;
    let constant_value = scalar_rational(constant)?;
    if coefficient_value.is_zero() {
        return Some(LinearExpression::Constant { value: constant });
    }
    let variable = if coefficient_value == RationalCoefficient::new(1, 1)? {
        LinearExpression::Variable {
            variable: variable_name,
        }
    } else {
        LinearExpression::Scale {
            factor: coefficient,
            expression: Box::new(LinearExpression::Variable {
                variable: variable_name,
            }),
        }
    };
    if constant_value.is_zero() {
        return Some(variable);
    }
    let constant_expression = LinearExpression::Constant {
        value: scalar_abs(constant)?,
    };
    Some(if constant_value.numerator() < 0 {
        LinearExpression::Subtract {
            left: Box::new(variable),
            right: Box::new(constant_expression),
        }
    } else {
        LinearExpression::Add {
            left: Box::new(variable),
            right: Box::new(constant_expression),
        }
    })
}

fn linear_affine_expression(
    coefficient: LinearScalar,
    constant: LinearScalar,
) -> Option<LinearExpression> {
    linear_affine_expression_for(LinearVariable::X, coefficient, constant)
}

fn draw_nonzero_integer(rng: &mut DeterministicRng, max_abs: i64) -> i64 {
    let magnitude = 1 + rng.next_bounded(max_abs as u64) as i64;
    if rng.next_bounded(2) == 0 {
        -magnitude
    } else {
        magnitude
    }
}

fn bounded_integer(value: RationalCoefficient, max_abs: i64) -> Option<i64> {
    (value.denominator() == 1 && value.numerator().unsigned_abs() <= max_abs as u64)
        .then_some(value.numerator())
}

fn draw_simple_linear_equation(
    rng: &mut DeterministicRng,
    solution: RationalCoefficient,
) -> Option<(LinearExpression, LinearExpression)> {
    let x = bounded_integer(solution, 15)?;
    if rng.next_bounded(2) == 0 {
        let addend = draw_nonzero_integer(rng, 12);
        let right = x.checked_add(addend)?;
        if right.unsigned_abs() > 30 {
            return None;
        }
        Some((
            linear_affine_expression(integer_scalar(1), integer_scalar(addend))?,
            LinearExpression::Constant {
                value: integer_scalar(right),
            },
        ))
    } else {
        let coefficient = draw_nonzero_integer(rng, 9);
        if coefficient.unsigned_abs() == 1 {
            return None;
        }
        let right = coefficient.checked_mul(x)?;
        if right.unsigned_abs() > 90 {
            return None;
        }
        Some((
            linear_affine_expression(integer_scalar(coefficient), integer_scalar(0))?,
            LinearExpression::Constant {
                value: integer_scalar(right),
            },
        ))
    }
}

fn draw_basic_linear_equation(
    rng: &mut DeterministicRng,
    solution: RationalCoefficient,
) -> Option<(LinearExpression, LinearExpression)> {
    let shape = rng.next_bounded(4);
    let (a, b, c, d) =
        draw_conditioned_coefficients(rng, LinearSolutionDomain::Integer, shape, solution)?;
    Some((
        linear_affine_expression(scalar_from_rational(a)?, scalar_from_rational(b)?)?,
        linear_affine_expression(scalar_from_rational(c)?, scalar_from_rational(d)?)?,
    ))
}

fn draw_parenthesized_integer_equation(
    rng: &mut DeterministicRng,
    solution: RationalCoefficient,
) -> Option<(LinearExpression, LinearExpression)> {
    let factor = draw_nonzero_integer(rng, 5);
    if factor.unsigned_abs() == 1 {
        return None;
    }
    let inner_coefficient = draw_nonzero_integer(rng, 5);
    let inner_constant = draw_nonzero_integer(rng, 9);
    let a = RationalCoefficient::new(factor.checked_mul(inner_coefficient)?, 1)?;
    let b = RationalCoefficient::new(factor.checked_mul(inner_constant)?, 1)?;
    let right_coefficient = if rng.next_bounded(3) == 0 {
        0
    } else {
        draw_nonzero_integer(rng, 8)
    };
    let c = RationalCoefficient::new(right_coefficient, 1)?;
    if a == c {
        return None;
    }
    let d = a.subtract(c)?.multiply(solution)?.checked_add(b)?;
    let d = bounded_integer(d, 40)?;
    let inner = linear_affine_expression(
        integer_scalar(inner_coefficient),
        integer_scalar(inner_constant),
    )?;
    let left = LinearExpression::Scale {
        factor: integer_scalar(factor),
        expression: Box::new(inner),
    };
    let right = linear_affine_expression(integer_scalar(right_coefficient), integer_scalar(d))?;
    Some((left, right))
}

fn draw_fraction_scalar(rng: &mut DeterministicRng) -> Option<LinearScalar> {
    let denominator = 2 + rng.next_bounded(7) as i64;
    let numerator_abs = 1 + rng.next_bounded(9) as i64;
    let numerator = if rng.next_bounded(2) == 0 {
        -numerator_abs
    } else {
        numerator_abs
    };
    let value = RationalCoefficient::new(numerator, denominator)?;
    (!value.is_integer()).then_some(LinearScalar::Fraction { value })
}

fn draw_decimal_scalar(rng: &mut DeterministicRng) -> Option<LinearScalar> {
    let scale = 1 + rng.next_bounded(2) as u32;
    let max = if scale == 1 { 25 } else { 250 };
    let magnitude = 1 + rng.next_bounded(max) as i64;
    if magnitude % 10 == 0 {
        return None;
    }
    let coefficient = if rng.next_bounded(2) == 0 {
        -magnitude
    } else {
        magnitude
    };
    Some(LinearScalar::ExactDecimal { coefficient, scale })
}

fn draw_comprehensive_linear_equation(
    rng: &mut DeterministicRng,
    solution: RationalCoefficient,
) -> Option<(LinearExpression, LinearExpression)> {
    let factor = if rng.next_bounded(2) == 0 {
        draw_fraction_scalar(rng)?
    } else {
        draw_decimal_scalar(rng)?
    };
    let factor_value = scalar_rational(factor)?;
    let inner_coefficient = draw_nonzero_integer(rng, 5);
    let inner_constant = draw_nonzero_integer(rng, 9);
    let a = factor_value.multiply(RationalCoefficient::new(inner_coefficient, 1)?)?;
    let b = factor_value.multiply(RationalCoefficient::new(inner_constant, 1)?)?;
    let right_coefficient = if rng.next_bounded(3) == 0 {
        0
    } else {
        draw_nonzero_integer(rng, 6)
    };
    let c = RationalCoefficient::new(right_coefficient, 1)?;
    if a == c {
        return None;
    }
    let d = a.subtract(c)?.multiply(solution)?.checked_add(b)?;
    if d.numerator().unsigned_abs() > 80 || d.denominator() > 24 {
        return None;
    }
    let inner = linear_affine_expression(
        integer_scalar(inner_coefficient),
        integer_scalar(inner_constant),
    )?;
    let left = LinearExpression::Scale {
        factor,
        expression: Box::new(inner),
    };
    let right =
        linear_affine_expression(integer_scalar(right_coefficient), scalar_from_rational(d)?)?;
    Some((left, right))
}

fn coefficient_domain(
    mode: LinearSolutionDomain,
    allow_zero: bool,
) -> &'static [RationalCoefficient] {
    match (mode, allow_zero) {
        (LinearSolutionDomain::Integer, false) => linear_integer_domain(),
        (LinearSolutionDomain::Integer, true) => linear_integer_domain_with_zero(),
        (LinearSolutionDomain::Rational, false) => linear_rational_domain(),
        (LinearSolutionDomain::Rational, true) => linear_rational_domain_with_zero(),
    }
}

fn coefficient_allowed(
    value: RationalCoefficient,
    mode: LinearSolutionDomain,
    allow_zero: bool,
) -> bool {
    coefficient_domain(mode, allow_zero).contains(&value)
}

/// Pick (minuend, subtrahend) uniformly from admissible pairs satisfying
/// `minuend - subtrahend = difference`.
fn draw_difference_pair(
    rng: &mut DeterministicRng,
    mode: LinearSolutionDomain,
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
    mode: LinearSolutionDomain,
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

fn draw_nonzero_linear_coefficient(
    rng: &mut DeterministicRng,
    mode: LinearSolutionDomain,
) -> RationalCoefficient {
    let domain = match mode {
        LinearSolutionDomain::Integer => linear_integer_domain(),
        LinearSolutionDomain::Rational => linear_rational_domain(),
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
    let radical = if radicand == 1 {
        AnswerNode::Integer(radical_coefficient)
    } else {
        answer_product(
            AnswerNode::Integer(radical_coefficient),
            AnswerNode::Root {
                radicand: Box::new(AnswerNode::Integer(radicand)),
                index: None,
            },
        )
    };
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

fn linear_variable_term(variable: LinearVariable, coefficient: i64) -> Option<LinearExpression> {
    if coefficient == 0 {
        return None;
    }
    if coefficient == 1 {
        Some(LinearExpression::Variable { variable })
    } else {
        Some(LinearExpression::Scale {
            factor: integer_scalar(coefficient),
            expression: Box::new(LinearExpression::Variable { variable }),
        })
    }
}

fn standard_simultaneous_equation(a: i64, b: i64, c: i64) -> Option<LinearEquationSurface> {
    if a == 0 || b == 0 {
        return None;
    }
    let x_term = linear_variable_term(LinearVariable::X, a)?;
    let y_term = linear_variable_term(LinearVariable::Y, b.checked_abs()?)?;
    let left = if b < 0 {
        LinearExpression::Subtract {
            left: Box::new(x_term),
            right: Box::new(y_term),
        }
    } else {
        LinearExpression::Add {
            left: Box::new(x_term),
            right: Box::new(y_term),
        }
    };
    Some(LinearEquationSurface {
        left,
        right: LinearExpression::Constant {
            value: integer_scalar(c),
        },
    })
}

fn draw_elimination_system(
    rng: &mut DeterministicRng,
    x: i64,
    y: i64,
) -> Option<[LinearEquationSurface; 2]> {
    for _ in 0..64 {
        let eliminate_x = rng.next_bounded(2) == 0;
        let shared = draw_nonzero_integer(rng, 5);
        let second_shared = if rng.next_bounded(2) == 0 {
            shared
        } else {
            -shared
        };
        let other_first = draw_nonzero_integer(rng, 5);
        let other_second = draw_nonzero_integer(rng, 5);
        let (a, b, d, e) = if eliminate_x {
            (shared, other_first, second_shared, other_second)
        } else {
            (other_first, shared, other_second, second_shared)
        };
        let determinant = a.checked_mul(e)?.checked_sub(b.checked_mul(d)?)?;
        if determinant == 0 {
            continue;
        }
        let c = a.checked_mul(x)?.checked_add(b.checked_mul(y)?)?;
        let f = d.checked_mul(x)?.checked_add(e.checked_mul(y)?)?;
        if c.unsigned_abs() > 15 || f.unsigned_abs() > 15 {
            continue;
        }
        return Some([
            standard_simultaneous_equation(a, b, c)?,
            standard_simultaneous_equation(d, e, f)?,
        ]);
    }
    None
}

fn draw_substitution_system(
    rng: &mut DeterministicRng,
    x: i64,
    y: i64,
) -> Option<[LinearEquationSurface; 2]> {
    for _ in 0..64 {
        let isolate_x = rng.next_bounded(2) == 0;
        let slope = draw_nonzero_integer(rng, 3);
        let (isolated_value, other_value, rhs_variable) = if isolate_x {
            (x, y, LinearVariable::Y)
        } else {
            (y, x, LinearVariable::X)
        };
        let intercept = isolated_value.checked_sub(slope.checked_mul(other_value)?)?;
        if intercept.unsigned_abs() > 15 {
            continue;
        }
        let a = draw_nonzero_integer(rng, 6);
        let b = draw_nonzero_integer(rng, 6);
        let c = a.checked_mul(x)?.checked_add(b.checked_mul(y)?)?;
        if c.unsigned_abs() > 15 {
            continue;
        }
        let isolated_coefficients = if isolate_x {
            (1_i64, slope.checked_neg()?)
        } else {
            (slope.checked_neg()?, 1_i64)
        };
        if isolated_coefficients
            .0
            .checked_mul(b)?
            .checked_sub(isolated_coefficients.1.checked_mul(a)?)?
            == 0
        {
            continue;
        }
        let isolated = LinearEquationSurface {
            left: LinearExpression::Variable {
                variable: if isolate_x {
                    LinearVariable::X
                } else {
                    LinearVariable::Y
                },
            },
            right: linear_affine_expression_for(
                rhs_variable,
                integer_scalar(slope),
                integer_scalar(intercept),
            )?,
        };
        return Some([isolated, standard_simultaneous_equation(a, b, c)?]);
    }
    None
}

fn scaled_linear_constant(value: LinearScalar, factor: LinearScalar) -> Option<LinearScalar> {
    if let (LinearScalar::Integer { value }, LinearScalar::ExactDecimal { coefficient, scale }) =
        (value, factor)
    {
        let mut coefficient = coefficient.checked_mul(value)?;
        if coefficient == 0 {
            return Some(integer_scalar(0));
        }
        let mut scale = scale;
        while scale > 0 && coefficient % 10 == 0 {
            coefficient /= 10;
            scale -= 1;
        }
        return if scale == 0 {
            Some(integer_scalar(coefficient))
        } else {
            Some(LinearScalar::ExactDecimal { coefficient, scale })
        };
    }
    let left = scalar_rational(value)?;
    let right = scalar_rational(factor)?;
    scalar_from_rational(RationalCoefficient::new(
        left.numerator().checked_mul(right.numerator())?,
        left.denominator().checked_mul(right.denominator())?,
    )?)
}

fn transformed_linear_expression(
    expression: LinearExpression,
    factor: LinearScalar,
) -> Option<LinearExpression> {
    if let LinearExpression::Constant { value } = expression {
        return Some(LinearExpression::Constant {
            value: scaled_linear_constant(value, factor)?,
        });
    }
    Some(LinearExpression::Scale {
        factor,
        expression: Box::new(expression),
    })
}

fn transform_simultaneous_system(
    equations: [LinearEquationSurface; 2],
    transform: SimultaneousSurfaceTransform,
    rng: &mut DeterministicRng,
) -> Option<[LinearEquationSurface; 2]> {
    let factor = match transform {
        SimultaneousSurfaceTransform::Parentheses => integer_scalar(2 + rng.next_bounded(2) as i64),
        SimultaneousSurfaceTransform::Fraction => LinearScalar::Fraction {
            value: RationalCoefficient::new(1, 2 + rng.next_bounded(2) as i64)?,
        },
        SimultaneousSurfaceTransform::Decimal => LinearScalar::ExactDecimal {
            coefficient: if rng.next_bounded(2) == 0 { 5 } else { 2 },
            scale: 1,
        },
    };
    let [first, second] = equations;
    let transform_equation = |equation: LinearEquationSurface| -> Option<LinearEquationSurface> {
        Some(LinearEquationSurface {
            left: transformed_linear_expression(equation.left, factor)?,
            right: transformed_linear_expression(equation.right, factor)?,
        })
    };
    Some([transform_equation(first)?, transform_equation(second)?])
}

fn expression_surface_transform(
    expression: &LinearExpression,
) -> Option<SimultaneousSurfaceTransform> {
    match expression {
        LinearExpression::Variable { .. } | LinearExpression::Constant { .. } => None,
        LinearExpression::Add { left, right } | LinearExpression::Subtract { left, right } => {
            expression_surface_transform(left).or_else(|| expression_surface_transform(right))
        }
        LinearExpression::Scale { factor, expression } => match factor {
            LinearScalar::Fraction { .. } => Some(SimultaneousSurfaceTransform::Fraction),
            LinearScalar::ExactDecimal { .. } => Some(SimultaneousSurfaceTransform::Decimal),
            LinearScalar::Integer { .. }
                if matches!(
                    expression.as_ref(),
                    LinearExpression::Add { .. } | LinearExpression::Subtract { .. }
                ) =>
            {
                Some(SimultaneousSurfaceTransform::Parentheses)
            }
            LinearScalar::Integer { .. } => expression_surface_transform(expression),
        },
    }
}

fn simultaneous_surface_transform(
    equations: &[LinearEquationSurface; 2],
) -> Option<SimultaneousSurfaceTransform> {
    [
        SimultaneousSurfaceTransform::Fraction,
        SimultaneousSurfaceTransform::Decimal,
        SimultaneousSurfaceTransform::Parentheses,
    ]
    .into_iter()
    .find(|&desired| {
        equations.iter().any(|equation| {
            [
                expression_surface_transform(&equation.left),
                expression_surface_transform(&equation.right),
            ]
            .contains(&Some(desired))
        })
    })
}

fn simultaneous_equation_problem(
    registration: &ThemeRegistration,
    mode: SimultaneousEquationMode,
    rng: &mut DeterministicRng,
    id: u32,
    weights: &OperationWeights,
) -> Option<Result<Problem, GenerationError>> {
    let x = rng.next_bounded(31) as i64 - 15;
    let y = rng.next_bounded(31) as i64 - 15;
    let solve_method = match mode {
        SimultaneousEquationMode::Elimination => SimultaneousSolveMethod::Elimination,
        SimultaneousEquationMode::Substitution => SimultaneousSolveMethod::Substitution,
        SimultaneousEquationMode::SummaryBasic | SimultaneousEquationMode::SummaryTransformed => {
            if rng.next_bounded(2) == 0 {
                SimultaneousSolveMethod::Elimination
            } else {
                SimultaneousSolveMethod::Substitution
            }
        }
    };
    let mut equations = match solve_method {
        SimultaneousSolveMethod::Elimination => draw_elimination_system(rng, x, y)?,
        SimultaneousSolveMethod::Substitution => draw_substitution_system(rng, x, y)?,
    };
    if mode == SimultaneousEquationMode::SummaryTransformed {
        let transform = match rng.next_bounded(3) {
            0 => SimultaneousSurfaceTransform::Parentheses,
            1 => SimultaneousSurfaceTransform::Fraction,
            _ => SimultaneousSurfaceTransform::Decimal,
        };
        equations = transform_simultaneous_system(equations, transform, rng)?;
    }

    let canonical_answer = AnswerNode::Tuple(vec![AnswerNode::Integer(x), AnswerNode::Integer(y)]);
    let operation_plan =
        simultaneous_equation_plan(&equations, solve_method, &canonical_answer, weights)?;
    Some(
        Problem::generated(
            registration,
            id,
            ProblemPrompt::SimultaneousEquation {
                equations,
                solve_method,
            },
            AnswerSchema::OrderedPair,
            canonical_answer,
            EffortModel::operations(operation_plan),
        )
        .map_err(GenerationError::from),
    )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum QuadraticSurfaceTransform {
    Parentheses,
    Fraction,
    Decimal,
}

fn quadratic_linear(expression: LinearExpression) -> QuadraticExpression {
    QuadraticExpression::Linear { expression }
}

fn quadratic_zero() -> QuadraticExpression {
    quadratic_linear(LinearExpression::Constant {
        value: integer_scalar(0),
    })
}

fn quadratic_one() -> QuadraticExpression {
    quadratic_linear(LinearExpression::Constant {
        value: integer_scalar(1),
    })
}

fn quadratic_x() -> QuadraticExpression {
    quadratic_linear(LinearExpression::Variable {
        variable: LinearVariable::X,
    })
}

fn positive_rational(value: RationalCoefficient) -> Option<RationalCoefficient> {
    RationalCoefficient::new(value.numerator().checked_abs()?, value.denominator())
}

fn quadratic_scaled(
    coefficient: RationalCoefficient,
    expression: QuadraticExpression,
) -> Option<QuadraticExpression> {
    if coefficient == RationalCoefficient::new(1, 1)? {
        return Some(expression);
    }
    Some(QuadraticExpression::Scale {
        factor: scalar_from_rational(coefficient)?,
        expression: Box::new(expression),
    })
}

fn quadratic_append_term(
    expression: QuadraticExpression,
    coefficient: RationalCoefficient,
    term: QuadraticExpression,
) -> Option<QuadraticExpression> {
    if coefficient.is_zero() {
        return Some(expression);
    }
    let term = quadratic_scaled(positive_rational(coefficient)?, term)?;
    Some(if coefficient.numerator() < 0 {
        QuadraticExpression::Subtract {
            left: Box::new(expression),
            right: Box::new(term),
        }
    } else {
        QuadraticExpression::Add {
            left: Box::new(expression),
            right: Box::new(term),
        }
    })
}

fn quadratic_standard_expression(
    a: RationalCoefficient,
    b: RationalCoefficient,
    c: RationalCoefficient,
) -> Option<QuadraticExpression> {
    if a.is_zero() {
        return None;
    }
    let square = QuadraticExpression::Square {
        expression: LinearExpression::Variable {
            variable: LinearVariable::X,
        },
    };
    let mut expression = quadratic_scaled(a, square)?;
    expression = quadratic_append_term(expression, b, quadratic_x())?;
    quadratic_append_term(expression, c, quadratic_one())
}

fn quadratic_square_expression(shift: i64) -> Option<QuadraticExpression> {
    let inner =
        linear_affine_expression_for(LinearVariable::X, integer_scalar(1), integer_scalar(shift))?;
    Some(QuadraticExpression::Square { expression: inner })
}

fn quadratic_square_value(rng: &mut DeterministicRng) -> Option<(i64, i64, i64)> {
    if rng.next_bounded(2) == 0 {
        let root = 1_i64 + rng.next_bounded(16) as i64;
        return Some((root.checked_mul(root)?, root, 1));
    }
    for _ in 0..32 {
        let value = 2_i64 + rng.next_bounded(29) as i64;
        let (coefficient, radicand) = square_free_sqrt_decomposition(u64::try_from(value).ok()?)?;
        if radicand == 1 {
            continue;
        }
        return Some((
            value,
            i64::try_from(coefficient).ok()?,
            i64::try_from(radicand).ok()?,
        ));
    }
    None
}

fn quadratic_square_root_candidate(
    rng: &mut DeterministicRng,
) -> Option<(QuadraticEquationSurface, QuadraticSolveMethod, AnswerNode)> {
    let shifted = rng.next_bounded(2) == 1;
    let shift = if shifted {
        draw_signed_integer(rng, 5)?
    } else {
        0
    };
    let (square_value, radical_coefficient, radicand) = quadratic_square_value(rng)?;
    let a_int = 1_i64 + rng.next_bounded(9) as i64;
    let constant = a_int.checked_mul(square_value)?.checked_neg()?;
    let square = quadratic_square_expression(shift)?;
    let leading = quadratic_scaled(RationalCoefficient::new(a_int, 1)?, square)?;
    let left = QuadraticExpression::Subtract {
        left: Box::new(leading),
        right: Box::new(quadratic_linear(LinearExpression::Constant {
            value: integer_scalar(constant.checked_neg()?),
        })),
    };
    let answer = quadratic_formula_answer(shift.checked_neg()?, radical_coefficient, radicand, 1);
    Some((
        QuadraticEquationSurface {
            left,
            right: quadratic_zero(),
        },
        QuadraticSolveMethod::SquareRoot,
        answer,
    ))
}

fn quadratic_factoring_candidate(
    rng: &mut DeterministicRng,
) -> Option<(QuadraticEquationSurface, QuadraticSolveMethod, AnswerNode)> {
    let first = draw_signed_integer(rng, 9)?;
    let second = draw_signed_integer(rng, 9)?;
    let b = first.checked_add(second)?.checked_neg()?;
    let c = first.checked_mul(second)?;
    let answer = if first == second {
        AnswerNode::Integer(first)
    } else {
        let mut roots = vec![AnswerNode::Integer(first), AnswerNode::Integer(second)];
        roots.sort();
        AnswerNode::Tuple(roots)
    };
    Some((
        QuadraticEquationSurface {
            left: quadratic_standard_expression(
                RationalCoefficient::new(1, 1)?,
                RationalCoefficient::new(b, 1)?,
                RationalCoefficient::new(c, 1)?,
            )?,
            right: quadratic_zero(),
        },
        QuadraticSolveMethod::Factoring,
        answer,
    ))
}

fn quadratic_formula_candidate(
    rng: &mut DeterministicRng,
) -> Option<(QuadraticEquationSurface, QuadraticSolveMethod, AnswerNode)> {
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
    if radicand > 99 {
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
    if constant.unsigned_abs() > 9 || radical_coefficient.unsigned_abs() > 9 || denominator > 15 {
        return None;
    }
    let answer = quadratic_formula_answer(constant, radical_coefficient, radicand, denominator);
    if !answer.is_within_size_limit() {
        return None;
    }
    Some((
        QuadraticEquationSurface {
            left: quadratic_standard_expression(
                RationalCoefficient::new(a_int, 1)?,
                RationalCoefficient::new(b_int, 1)?,
                RationalCoefficient::new(c_int, 1)?,
            )?,
            right: quadratic_zero(),
        },
        QuadraticSolveMethod::Formula,
        answer,
    ))
}

fn quadratic_surface_transform(
    equation: &QuadraticEquationSurface,
) -> Option<QuadraticSurfaceTransform> {
    let QuadraticExpression::Scale { factor, expression } = &equation.left else {
        return None;
    };
    match factor {
        LinearScalar::Fraction { .. } => Some(QuadraticSurfaceTransform::Fraction),
        LinearScalar::ExactDecimal { .. } => Some(QuadraticSurfaceTransform::Decimal),
        LinearScalar::Integer { .. }
            if matches!(
                expression.as_ref(),
                QuadraticExpression::Add { .. } | QuadraticExpression::Subtract { .. }
            ) =>
        {
            Some(QuadraticSurfaceTransform::Parentheses)
        }
        LinearScalar::Integer { .. } => None,
    }
}

fn quadratic_expression_has_shifted_square(expression: &QuadraticExpression) -> bool {
    match expression {
        QuadraticExpression::Square { expression } => {
            crate::semantics::normalize_linear_expression(expression)
                .is_some_and(|(x, y, constant)| !x.is_zero() && y.is_zero() && !constant.is_zero())
        }
        QuadraticExpression::Linear { .. } => false,
        QuadraticExpression::Add { left, right }
        | QuadraticExpression::Subtract { left, right } => {
            quadratic_expression_has_shifted_square(left)
                || quadratic_expression_has_shifted_square(right)
        }
        QuadraticExpression::Scale { expression, .. } => {
            quadratic_expression_has_shifted_square(expression)
        }
    }
}

fn quadratic_equation_has_shifted_square(equation: &QuadraticEquationSurface) -> bool {
    quadratic_expression_has_shifted_square(&equation.left)
        || quadratic_expression_has_shifted_square(&equation.right)
}

fn transform_quadratic_equation(
    equation: QuadraticEquationSurface,
    transform: QuadraticSurfaceTransform,
    rng: &mut DeterministicRng,
) -> Option<QuadraticEquationSurface> {
    let factor = match transform {
        QuadraticSurfaceTransform::Parentheses => integer_scalar(2 + rng.next_bounded(2) as i64),
        QuadraticSurfaceTransform::Fraction => LinearScalar::Fraction {
            value: RationalCoefficient::new(1, 2 + rng.next_bounded(2) as i64)?,
        },
        QuadraticSurfaceTransform::Decimal => {
            if rng.next_bounded(2) == 0 {
                LinearScalar::ExactDecimal {
                    coefficient: 5,
                    scale: 1,
                }
            } else {
                LinearScalar::ExactDecimal {
                    coefficient: 2,
                    scale: 1,
                }
            }
        }
    };
    Some(QuadraticEquationSurface {
        left: QuadraticExpression::Scale {
            factor,
            expression: Box::new(equation.left),
        },
        right: equation.right,
    })
}

fn quadratic_equation_problem(
    registration: &ThemeRegistration,
    mode: QuadraticEquationMode,
    rng: &mut DeterministicRng,
    id: u32,
    _weights: &OperationWeights,
) -> Option<Result<Problem, GenerationError>> {
    let (equation, solve_method, canonical_answer) = match mode {
        QuadraticEquationMode::SquareRoot => quadratic_square_root_candidate(rng)?,
        QuadraticEquationMode::Factoring => quadratic_factoring_candidate(rng)?,
        QuadraticEquationMode::Formula => quadratic_formula_candidate(rng)?,
        QuadraticEquationMode::Comprehensive => {
            let candidate = match rng.next_bounded(3) {
                0 => quadratic_square_root_candidate(rng)?,
                1 => quadratic_factoring_candidate(rng)?,
                _ => quadratic_formula_candidate(rng)?,
            };
            let transform = match rng.next_bounded(3) {
                0 => QuadraticSurfaceTransform::Parentheses,
                1 => QuadraticSurfaceTransform::Fraction,
                _ => QuadraticSurfaceTransform::Decimal,
            };
            let (base_equation, solve_method, answer) = candidate;
            (
                transform_quadratic_equation(base_equation, transform, rng)?,
                solve_method,
                answer,
            )
        }
    };
    let operation_plan = quadratic_equation_plan(&equation, solve_method, &canonical_answer)?;
    Some(
        Problem::generated(
            registration,
            id,
            ProblemPrompt::QuadraticEquation {
                equation,
                solve_method,
            },
            AnswerSchema::Algebraic,
            canonical_answer,
            EffortModel::operations(operation_plan),
        )
        .map_err(GenerationError::from),
    )
}

fn linear_equation_problem(
    id: u32,
    registration: &ThemeRegistration,
    solution_domain: LinearSolutionDomain,
    left: LinearExpression,
    right: LinearExpression,
    solution: RationalCoefficient,
    _weights: &OperationWeights,
) -> Result<Problem, GenerationError> {
    let canonical_answer = rational_answer(solution);
    let operation_plan = linear_expression_equation_plan(&left, &right, &canonical_answer).ok_or(
        GenerationError::InvalidGeneratedProblem {
            reason: "linear-equation effort model rejected generated expression",
        },
    )?;
    let answer_schema = match solution_domain {
        LinearSolutionDomain::Integer => AnswerSchema::Integer { min: -15, max: 15 },
        LinearSolutionDomain::Rational => AnswerSchema::Rational {
            max_abs_numerator: 20,
            max_denominator: 12,
            require_reduced_fraction_form: true,
        },
    };
    Problem::generated(
        registration,
        id,
        ProblemPrompt::LinearEquation { left, right },
        answer_schema,
        canonical_answer,
        EffortModel::operations(operation_plan),
    )
    .map_err(GenerationError::from)
}

/// Current generators owned by this theme family.
pub(crate) static GENERATORS: [GeneratorEntry; 12] = [
    GeneratorEntry::current(&LINEAR_EQUATION_SIMPLE_GENERATOR),
    GeneratorEntry::current(&LINEAR_EQUATION_1_GENERATOR),
    GeneratorEntry::current(&LINEAR_EQUATION_2_GENERATOR),
    GeneratorEntry::current(&LINEAR_EQUATION_3_GENERATOR),
    GeneratorEntry::current(&QUADRATIC_EQUATION_1_GENERATOR),
    GeneratorEntry::current(&QUADRATIC_EQUATION_2_GENERATOR),
    GeneratorEntry::current(&QUADRATIC_EQUATION_3_GENERATOR),
    GeneratorEntry::current(&QUADRATIC_EQUATION_4_GENERATOR),
    GeneratorEntry::current(&SIMULTANEOUS_EQUATION_ELIMINATION_GENERATOR),
    GeneratorEntry::current(&SIMULTANEOUS_EQUATION_SUBSTITUTION_GENERATOR),
    GeneratorEntry::current(&SIMULTANEOUS_EQUATION_SUMMARY_1_GENERATOR),
    GeneratorEntry::current(&SIMULTANEOUS_EQUATION_SUMMARY_2_GENERATOR),
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
        let integer = linear_solution_domain(LinearSolutionDomain::Integer);
        assert_eq!(integer.len(), 31);
        assert_eq!(integer.first().unwrap().numerator(), -15);
        assert_eq!(integer.last().unwrap().numerator(), 15);
        assert!(integer.iter().all(|value| value.denominator() == 1));

        let rational = linear_solution_domain(LinearSolutionDomain::Rational);
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
        for generator in [
            &LINEAR_EQUATION_SIMPLE_GENERATOR,
            &LINEAR_EQUATION_1_GENERATOR,
            &LINEAR_EQUATION_2_GENERATOR,
            &LINEAR_EQUATION_3_GENERATOR,
        ] {
            let mut rng = DeterministicRng::from_seed("AllAns7");
            let weights = OperationWeights::default();
            for answer in linear_answer_domain(generator.solution_domain) {
                let generated = (1_u32..=5_000).find_map(|ordinal| {
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
    fn quadratic_one_mixes_direct_and_shifted_square_surfaces() {
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
            let mut saw_direct = false;
            let mut saw_shifted = false;
            for problem in worksheet.into_problems() {
                let ProblemPrompt::QuadraticEquation {
                    equation,
                    solve_method,
                } = problem.prompt()
                else {
                    panic!("quadratic(1) prompt");
                };
                assert_eq!(*solve_method, QuadraticSolveMethod::SquareRoot);
                assert!(matches!(
                    equation.right,
                    QuadraticExpression::Linear {
                        expression: LinearExpression::Constant {
                            value: LinearScalar::Integer { value: 0 }
                        }
                    }
                ));
                let (a, _, _) = crate::semantics::normalize_quadratic_equation(equation).unwrap();
                assert!(a.is_integer() && a.numerator() > 0);
                let shifted = quadratic_equation_has_shifted_square(equation);
                saw_shifted |= shifted;
                saw_direct |= !shifted;
            }
            assert!(saw_direct, "quadratic(1) must retain ax^2+b=0 surfaces");
            assert!(
                saw_shifted,
                "quadratic(1) must include a(x+c)^2+b=0 surfaces"
            );
        }
    }

    #[test]
    fn quadratic_two_is_monic_and_reverse_generated_from_integer_roots() {
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
            let ProblemPrompt::QuadraticEquation {
                equation,
                solve_method,
            } = problem.prompt()
            else {
                panic!("quadratic(2) prompt");
            };
            assert_eq!(*solve_method, QuadraticSolveMethod::Factoring);
            let (a, b, c) = crate::semantics::normalize_quadratic_equation(equation).unwrap();
            assert_eq!(a, RationalCoefficient::new(1, 1).unwrap());
            assert!(b.is_integer() && c.is_integer());
            match problem.canonical_answer() {
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
        match radical.as_ref() {
            AnswerNode::Integer(coefficient) => Some((constant, *coefficient, 1, denominator)),
            AnswerNode::Binary {
                operator: AnswerBinaryOperator::Multiply,
                left,
                right,
            } => {
                let AnswerNode::Integer(coefficient) = left.as_ref() else {
                    return None;
                };
                let AnswerNode::Root {
                    radicand,
                    index: None,
                } = right.as_ref()
                else {
                    return None;
                };
                let AnswerNode::Integer(radicand) = radicand.as_ref() else {
                    return None;
                };
                Some((constant, *coefficient, *radicand, denominator))
            }
            AnswerNode::Root {
                radicand,
                index: None,
            } => {
                let AnswerNode::Integer(radicand) = radicand.as_ref() else {
                    return None;
                };
                Some((constant, 1, *radicand, denominator))
            }
            _ => None,
        }
    }

    #[test]
    fn quadratic_three_uses_integer_coefficients_and_keeps_square_discriminants() {
        let mut rng = DeterministicRng::from_seed("quadratic-three-support");
        let weights = OperationWeights::default();
        let mut saw_square_discriminant = false;
        let mut saw_nonsquare_discriminant = false;
        for ordinal in 1..=2_000 {
            let Some(problem) = RandomCandidateSource::draw_candidate(
                &QUADRATIC_EQUATION_3_GENERATOR,
                &mut rng,
                ordinal,
                &weights,
            )
            .expect("quadratic(3) candidate construction") else {
                continue;
            };
            let ProblemPrompt::QuadraticEquation {
                equation,
                solve_method,
            } = problem.prompt()
            else {
                panic!("quadratic(3) prompt");
            };
            assert_eq!(*solve_method, QuadraticSolveMethod::Formula);
            let (a, b, c) = crate::semantics::normalize_quadratic_equation(equation).unwrap();
            assert!(a.is_integer() && b.is_integer() && c.is_integer());
            let discriminant = b.numerator() * b.numerator() - 4 * a.numerator() * c.numerator();
            assert!(discriminant > 0);
            let square = crate::exact::exact_square_root_u128(discriminant as u128).is_some();
            saw_square_discriminant |= square;
            saw_nonsquare_discriminant |= !square;
            let (constant, radical_coefficient, radicand, denominator) =
                quadratic_formula_bounds(problem.canonical_answer())
                    .expect("quadratic(3) must use the bounded quadratic-formula AST");
            assert!(constant.unsigned_abs() <= 9);
            assert!(radical_coefficient.unsigned_abs() <= 9);
            assert!((1..=99).contains(&radicand));
            assert!((1..=15).contains(&denominator));
            if saw_square_discriminant && saw_nonsquare_discriminant {
                break;
            }
        }
        assert!(
            saw_square_discriminant,
            "quadratic(3) must support perfect-square D"
        );
        assert!(
            saw_nonsquare_discriminant,
            "quadratic(3) must retain irrational roots"
        );
    }

    #[test]
    fn quadratic_four_covers_all_methods_and_transformed_surfaces() {
        let worksheet = generate_worksheet_request(&GenerateWorksheetRequest {
            schema_version: SCHEMA_VERSION,
            numeric_theme_id: THEME_ID_QUADRATIC_EQUATION_4,
            seed: "A1b2".to_owned(),
            difficulty: crate::identity::Difficulty::try_from(3).unwrap(),
            timeout_ms: None,
            max_attempts: Some(50_000),
        })
        .unwrap();
        let mut layers = [false; 9];
        for problem in worksheet.problems() {
            let ProblemPrompt::QuadraticEquation {
                equation,
                solve_method: _,
            } = problem.prompt()
            else {
                panic!("quadratic(4) prompt");
            };
            assert!(crate::semantics::normalize_quadratic_equation(equation).is_some());
            let layer = QUADRATIC_EQUATION_4_GENERATOR.layer_of(problem);
            layers[layer] = true;
        }
        assert!(layers.into_iter().all(|covered| covered));
    }

    #[derive(Default)]
    struct LinearSurfaceFacts {
        parenthesized_scales: usize,
        fraction_scalars: usize,
        decimal_scalars: usize,
    }

    fn collect_linear_scalar(value: LinearScalar, facts: &mut LinearSurfaceFacts) {
        match value {
            LinearScalar::Integer { .. } => {}
            LinearScalar::Fraction { .. } => facts.fraction_scalars += 1,
            LinearScalar::ExactDecimal { .. } => facts.decimal_scalars += 1,
        }
    }

    fn collect_linear_surface(expression: &LinearExpression, facts: &mut LinearSurfaceFacts) {
        match expression {
            LinearExpression::Variable { .. } => {}
            LinearExpression::Constant { value } => collect_linear_scalar(*value, facts),
            LinearExpression::Add { left, right } | LinearExpression::Subtract { left, right } => {
                collect_linear_surface(left, facts);
                collect_linear_surface(right, facts);
            }
            LinearExpression::Scale { factor, expression } => {
                collect_linear_scalar(*factor, facts);
                if matches!(
                    expression.as_ref(),
                    LinearExpression::Add { .. } | LinearExpression::Subtract { .. }
                ) {
                    facts.parenthesized_scales += 1;
                }
                collect_linear_surface(expression, facts);
            }
        }
    }

    fn simple_linear_shape(
        left: &LinearExpression,
        right: &LinearExpression,
    ) -> Option<&'static str> {
        if !matches!(right, LinearExpression::Constant { .. }) {
            return None;
        }
        match left {
            LinearExpression::Add { left, right } | LinearExpression::Subtract { left, right }
                if matches!(left.as_ref(), LinearExpression::Variable { .. })
                    && matches!(right.as_ref(), LinearExpression::Constant { .. }) =>
            {
                Some("x+a=b")
            }
            LinearExpression::Scale { expression, .. }
                if matches!(expression.as_ref(), LinearExpression::Variable { .. }) =>
            {
                Some("ax=b")
            }
            _ => None,
        }
    }

    #[test]
    fn linear_equation_curriculum_modes_fix_surface_archetypes() {
        use crate::generator::generate_worksheet_request;
        use crate::model::GenerateWorksheetRequest;
        use crate::schema::SCHEMA_VERSION;

        let mut simple_shapes = std::collections::BTreeSet::new();
        let mut linear_two_saw_integer_solution = false;
        let mut linear_two_saw_fraction_solution = false;
        let mut linear_three_saw_integer_solution = false;
        let mut linear_three_saw_fraction_solution = false;
        let mut linear_three_saw_fraction_surface = false;
        let mut linear_three_saw_decimal_surface = false;

        for difficulty in 1..=4 {
            for seed in ["LnA1", "LnB2", "LnC3", "LnD4"] {
                for theme_id in [
                    THEME_ID_LINEAR_EQUATION_SIMPLE,
                    THEME_ID_LINEAR_EQUATION_1,
                    THEME_ID_LINEAR_EQUATION_2,
                    THEME_ID_LINEAR_EQUATION_3,
                ] {
                    let worksheet = generate_worksheet_request(&GenerateWorksheetRequest {
                        schema_version: SCHEMA_VERSION,
                        numeric_theme_id: theme_id,
                        seed: seed.to_owned(),
                        difficulty: crate::identity::Difficulty::try_from(difficulty).unwrap(),
                        timeout_ms: Some(1_000),
                        max_attempts: Some(50_000),
                    })
                    .unwrap_or_else(|error| {
                        panic!("linear theme {theme_id} d{difficulty} failed for {seed}: {error}")
                    });
                    for problem in worksheet.problems() {
                        let ProblemPrompt::LinearEquation { left, right } = problem.prompt() else {
                            panic!("linear theme returned non-linear prompt");
                        };
                        let mut facts = LinearSurfaceFacts::default();
                        collect_linear_surface(left, &mut facts);
                        collect_linear_surface(right, &mut facts);
                        let (a, y_left, b) =
                            crate::semantics::normalize_linear_expression(left).unwrap();
                        assert!(y_left.is_zero());
                        let (c, y_right, d) =
                            crate::semantics::normalize_linear_expression(right).unwrap();
                        assert!(y_right.is_zero());
                        assert_ne!(a, c);

                        match theme_id {
                            THEME_ID_LINEAR_EQUATION_SIMPLE => {
                                simple_shapes.insert(
                                    simple_linear_shape(left, right).expect("simple linear shape"),
                                );
                                assert_eq!(facts.parenthesized_scales, 0);
                                assert_eq!(facts.fraction_scalars + facts.decimal_scalars, 0);
                            }
                            THEME_ID_LINEAR_EQUATION_1 => {
                                assert_eq!(facts.parenthesized_scales, 0);
                                assert_eq!(facts.fraction_scalars + facts.decimal_scalars, 0);
                                assert!([a, b, c, d].iter().all(|value| value.is_integer()));
                            }
                            THEME_ID_LINEAR_EQUATION_2 => {
                                assert!(facts.parenthesized_scales >= 1);
                                assert_eq!(facts.fraction_scalars + facts.decimal_scalars, 0);
                                assert!([a, b, c, d].iter().all(|value| value.is_integer()));
                                linear_two_saw_integer_solution |=
                                    matches!(problem.canonical_answer(), AnswerNode::Integer(_));
                                linear_two_saw_fraction_solution |= matches!(
                                    problem.canonical_answer(),
                                    AnswerNode::Fraction { .. }
                                );
                            }
                            THEME_ID_LINEAR_EQUATION_3 => {
                                assert!(facts.parenthesized_scales >= 1);
                                assert!(facts.fraction_scalars + facts.decimal_scalars >= 1);
                                linear_three_saw_fraction_surface |= facts.fraction_scalars > 0;
                                linear_three_saw_decimal_surface |= facts.decimal_scalars > 0;
                                linear_three_saw_integer_solution |=
                                    matches!(problem.canonical_answer(), AnswerNode::Integer(_));
                                linear_three_saw_fraction_solution |= matches!(
                                    problem.canonical_answer(),
                                    AnswerNode::Fraction { .. }
                                );
                            }
                            _ => unreachable!(),
                        }
                    }
                }
            }
        }
        assert_eq!(
            simple_shapes,
            std::collections::BTreeSet::from(["ax=b", "x+a=b"])
        );
        assert!(linear_two_saw_integer_solution && linear_two_saw_fraction_solution);
        assert!(linear_three_saw_integer_solution && linear_three_saw_fraction_solution);
        assert!(linear_three_saw_fraction_surface && linear_three_saw_decimal_surface);
    }

    fn assert_ordered_integer_pair(problem: &Problem) {
        let AnswerNode::Tuple(values) = problem.canonical_answer() else {
            panic!("simultaneous answer must be an ordered pair");
        };
        let [AnswerNode::Integer(x), AnswerNode::Integer(y)] = values.as_slice() else {
            panic!("simultaneous coordinates must be integers");
        };
        assert!(x.unsigned_abs() <= 15 && y.unsigned_abs() <= 15);
        assert!(matches!(problem.answer_schema(), AnswerSchema::OrderedPair));
        assert!(matches!(
            problem.input_interface(),
            AnswerInputInterface::StructuredMath { ref allowed_structures }
                if allowed_structures == &[EditorStructure::Negative, EditorStructure::Tuple]
        ));
    }

    fn assert_direct_elimination(equations: &[LinearEquationSurface; 2]) {
        let normalized = equations.clone().map(|equation| {
            crate::semantics::normalize_linear_equation(&equation)
                .expect("simultaneous equation must normalize exactly")
        });
        let [(a, b, c), (d, e, f)] = normalized;
        assert!([a, b, c, d, e, f].iter().all(|value| value.is_integer()));
        assert!(a.numerator() != 0 && b.numerator() != 0);
        assert!(d.numerator() != 0 && e.numerator() != 0);
        assert!(
            a.numerator().unsigned_abs() == d.numerator().unsigned_abs()
                || b.numerator().unsigned_abs() == e.numerator().unsigned_abs()
        );
        assert!([a, b, c, d, e, f]
            .iter()
            .all(|value| value.numerator().unsigned_abs() <= 15));
    }

    fn assert_direct_substitution(equations: &[LinearEquationSurface; 2]) {
        assert!(equations.iter().any(|equation| {
            matches!(equation.left, LinearExpression::Variable { .. })
                && matches!(
                    equation.right,
                    LinearExpression::Variable { .. }
                        | LinearExpression::Scale { .. }
                        | LinearExpression::Add { .. }
                        | LinearExpression::Subtract { .. }
                )
        }));
        for equation in equations {
            let (x, y, rhs) = crate::semantics::normalize_linear_equation(equation)
                .expect("simultaneous equation must normalize exactly");
            assert!([x, y, rhs].iter().all(|value| value.is_integer()));
            assert!([x, y, rhs]
                .iter()
                .all(|value| value.numerator().unsigned_abs() <= 15));
        }
    }

    #[test]
    fn simultaneous_dedicated_themes_encode_the_requested_method_in_the_surface() {
        for (theme_id, expected_method) in [
            (
                THEME_ID_SIMULTANEOUS_EQUATION_ELIMINATION,
                SimultaneousSolveMethod::Elimination,
            ),
            (
                THEME_ID_SIMULTANEOUS_EQUATION_SUBSTITUTION,
                SimultaneousSolveMethod::Substitution,
            ),
        ] {
            for seed in ["A1b2", "M7x9", "Q4r6"] {
                let worksheet = generate_worksheet_request(&GenerateWorksheetRequest {
                    schema_version: SCHEMA_VERSION,
                    numeric_theme_id: theme_id,
                    seed: seed.to_owned(),
                    difficulty: crate::identity::Difficulty::try_from(3).unwrap(),
                    timeout_ms: Some(1_000),
                    max_attempts: Some(50_000),
                })
                .unwrap();
                for problem in worksheet.problems() {
                    let ProblemPrompt::SimultaneousEquation {
                        equations,
                        solve_method,
                    } = problem.prompt()
                    else {
                        panic!("simultaneous theme returned a different prompt kind");
                    };
                    assert_eq!(*solve_method, expected_method);
                    match expected_method {
                        SimultaneousSolveMethod::Elimination => {
                            assert_direct_elimination(equations)
                        }
                        SimultaneousSolveMethod::Substitution => {
                            assert_direct_substitution(equations)
                        }
                    }
                    assert_ordered_integer_pair(problem);
                }
            }
        }
    }

    #[test]
    fn simultaneous_summary_one_mixes_both_basic_methods() {
        for seed in ["A1b2", "M7x9", "Q4r6"] {
            let worksheet = generate_worksheet_request(&GenerateWorksheetRequest {
                schema_version: SCHEMA_VERSION,
                numeric_theme_id: THEME_ID_SIMULTANEOUS_EQUATION_SUMMARY_1,
                seed: seed.to_owned(),
                difficulty: crate::identity::Difficulty::try_from(3).unwrap(),
                timeout_ms: Some(1_000),
                max_attempts: Some(50_000),
            })
            .unwrap();
            let mut methods = std::collections::BTreeSet::new();
            for problem in worksheet.problems() {
                let ProblemPrompt::SimultaneousEquation {
                    equations,
                    solve_method,
                } = problem.prompt()
                else {
                    panic!("summary(1) must emit simultaneous prompts");
                };
                assert_eq!(simultaneous_surface_transform(equations), None);
                methods.insert(*solve_method);
                match solve_method {
                    SimultaneousSolveMethod::Elimination => assert_direct_elimination(equations),
                    SimultaneousSolveMethod::Substitution => assert_direct_substitution(equations),
                }
                assert_ordered_integer_pair(problem);
            }
            assert_eq!(
                methods.len(),
                2,
                "summary(1) must exercise method selection"
            );
        }
    }

    #[test]
    fn simultaneous_summary_two_covers_parentheses_fraction_decimal_and_both_methods() {
        for seed in ["A1b2", "M7x9", "Q4r6"] {
            let worksheet = generate_worksheet_request(&GenerateWorksheetRequest {
                schema_version: SCHEMA_VERSION,
                numeric_theme_id: THEME_ID_SIMULTANEOUS_EQUATION_SUMMARY_2,
                seed: seed.to_owned(),
                difficulty: crate::identity::Difficulty::try_from(3).unwrap(),
                timeout_ms: Some(1_000),
                max_attempts: Some(50_000),
            })
            .unwrap();
            let mut methods = std::collections::BTreeSet::new();
            let mut transforms = std::collections::BTreeSet::new();
            for problem in worksheet.problems() {
                let ProblemPrompt::SimultaneousEquation {
                    equations,
                    solve_method,
                } = problem.prompt()
                else {
                    panic!("summary(2) must emit simultaneous prompts");
                };
                methods.insert(*solve_method);
                transforms.insert(
                    simultaneous_surface_transform(equations)
                        .expect("summary(2) must carry an explicit transformed surface"),
                );
                assert_ordered_integer_pair(problem);
            }
            assert_eq!(methods.len(), 2);
            assert_eq!(
                transforms,
                std::collections::BTreeSet::from([
                    SimultaneousSurfaceTransform::Parentheses,
                    SimultaneousSurfaceTransform::Fraction,
                    SimultaneousSurfaceTransform::Decimal,
                ])
            );
        }
    }
}
