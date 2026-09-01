use std::sync::OnceLock;

use crate::effort::{
    arithmetic_expression_plan, EffortModel, Operation, OperationPlan, OperationWeights,
};
use crate::error::GenerationError;
use crate::generator::{
    ConstructiveLayeredCandidateSource, GeneratorEntry, ProblemGenerator, RandomCandidateSource,
    SamplingStrategy, SelectionDedup,
};
use crate::generator_support::{
    binary_expression, mixed_number_answer, rational_answer, rational_expression,
};
use crate::model::{
    AnswerSchema, ArithmeticExpression, ArithmeticOperator, Problem, ProblemPrompt,
    RationalCoefficient,
};
use crate::rng::DeterministicRng;
use crate::theme::{
    CurriculumSafetyPolicy as Safety, CurriculumUnit, DedupPolicy as Dedup,
    FractionPresentationPolicy as FractionPresentation, SamplingLayerSpec, SchoolGrade,
    ThemeAnswerContract as AnswerContract, ThemePresentationPolicy as Presentation,
    ThemeRegistration, ThemeRegistrationSpec, ThemeTag, COMPACT_16_LAYOUT,
};

pub const THEME_ID_FRACTION_ADDITION: u32 = 9;
pub const THEME_ID_FRACTION_MULTIPLICATION: u32 = 10;
pub const THEME_ID_FRACTION_SUBTRACTION: u32 = 11;
pub const THEME_ID_FRACTION_DIVISION: u32 = 12;
pub const THEME_ID_FRACTION_INTEGER_MULTIPLICATION: u32 = 21;
pub const THEME_ID_FRACTION_INTEGER_DIVISION: u32 = 22;
pub const THEME_ID_FRACTION_SUMMARY_IMPROPER: u32 = 23;
pub const THEME_ID_FRACTION_ADDITION_SAME_DENOMINATOR: u32 = 55;
pub const THEME_ID_FRACTION_SUBTRACTION_SAME_DENOMINATOR: u32 = 56;
pub const THEME_ID_FRACTION_ADDITION_UNLIKE_DENOMINATOR: u32 = 57;
pub const THEME_ID_FRACTION_SUBTRACTION_UNLIKE_DENOMINATOR: u32 = 58;
pub const GENERATOR_REVISION_FRACTION_ADDITION: u32 = 5;
pub const GENERATOR_REVISION_FRACTION_MULTIPLICATION: u32 = 5;
pub const GENERATOR_REVISION_FRACTION_SUBTRACTION: u32 = 5;
pub const GENERATOR_REVISION_FRACTION_DIVISION: u32 = 6;
pub const GENERATOR_REVISION_FRACTION_INTEGER_MULTIPLICATION: u32 = 2;
pub const GENERATOR_REVISION_FRACTION_INTEGER_DIVISION: u32 = 2;
pub const GENERATOR_REVISION_FRACTION_SUMMARY_IMPROPER: u32 = 3;
pub const GENERATOR_REVISION_FRACTION_ADDITION_SAME_DENOMINATOR: u32 = 1;
pub const GENERATOR_REVISION_FRACTION_SUBTRACTION_SAME_DENOMINATOR: u32 = 1;
pub const GENERATOR_REVISION_FRACTION_ADDITION_UNLIKE_DENOMINATOR: u32 = 1;
pub const GENERATOR_REVISION_FRACTION_SUBTRACTION_UNLIKE_DENOMINATOR: u32 = 1;
pub const SKILL_ID_FRACTION_ADDITION: &str = "jp.grade5.fraction.addition.summary";
pub const SKILL_ID_FRACTION_ADDITION_SAME_DENOMINATOR: &str =
    "jp.grade4.fraction.addition.same_denominator";
pub const SKILL_ID_FRACTION_SUBTRACTION_SAME_DENOMINATOR: &str =
    "jp.grade4.fraction.subtraction.same_denominator";
pub const SKILL_ID_FRACTION_ADDITION_UNLIKE_DENOMINATOR: &str =
    "jp.grade5.fraction.addition.unlike_denominator";
pub const SKILL_ID_FRACTION_SUBTRACTION_UNLIKE_DENOMINATOR: &str =
    "jp.grade5.fraction.subtraction.unlike_denominator";
pub const SKILL_ID_FRACTION_MULTIPLICATION: &str = "jp.grade6.fraction.multiplication";
pub const SKILL_ID_FRACTION_SUBTRACTION: &str = "jp.grade5.fraction.subtraction.summary";
pub const SKILL_ID_FRACTION_DIVISION: &str = "jp.grade6.fraction.division";
pub const SKILL_ID_FRACTION_INTEGER_MULTIPLICATION: &str =
    "jp.grade6.fraction.integer_multiplication";
pub const SKILL_ID_FRACTION_INTEGER_DIVISION: &str = "jp.grade6.fraction.integer_division";
pub const SKILL_ID_FRACTION_SUMMARY_IMPROPER: &str = "jp.grade6.fraction.summary_improper";
pub const CURRICULUM_PATH_FRACTION_ADDITION: [&str; 4] = [
    "root",
    "小学5年生",
    "分数の加法，減法",
    "分数の足し算（まとめ）",
];
pub const CURRICULUM_PATH_FRACTION_ADDITION_SAME_DENOMINATOR: [&str; 4] = [
    "root",
    "小学4年生",
    "同分母の分数の加法，減法",
    "同分母の分数の足し算",
];
pub const CURRICULUM_PATH_FRACTION_SUBTRACTION_SAME_DENOMINATOR: [&str; 4] = [
    "root",
    "小学4年生",
    "同分母の分数の加法，減法",
    "同分母の分数の引き算",
];
pub const CURRICULUM_PATH_FRACTION_ADDITION_UNLIKE_DENOMINATOR: [&str; 4] = [
    "root",
    "小学5年生",
    "分数の加法，減法",
    "異分母の分数の足し算",
];
pub const CURRICULUM_PATH_FRACTION_SUBTRACTION_UNLIKE_DENOMINATOR: [&str; 4] = [
    "root",
    "小学5年生",
    "分数の加法，減法",
    "異分母の分数の引き算",
];
pub const CURRICULUM_PATH_FRACTION_MULTIPLICATION: [&str; 3] =
    ["root", "小学6年生", "分数の掛け算"];
pub const CURRICULUM_PATH_FRACTION_SUBTRACTION: [&str; 4] = [
    "root",
    "小学5年生",
    "分数の加法，減法",
    "分数の引き算（まとめ）",
];
pub const CURRICULUM_PATH_FRACTION_DIVISION: [&str; 3] = ["root", "小学6年生", "分数の割り算"];
pub const CURRICULUM_PATH_FRACTION_INTEGER_MULTIPLICATION: [&str; 3] =
    ["root", "小学6年生", "分数と整数の掛け算"];
pub const CURRICULUM_PATH_FRACTION_INTEGER_DIVISION: [&str; 3] =
    ["root", "小学6年生", "分数と整数の割り算"];
pub const CURRICULUM_PATH_FRACTION_SUMMARY_IMPROPER: [&str; 3] =
    ["root", "小学6年生", "分数総まとめ(仮分数)"];

pub const CURRICULUM_UNIT_GRADE4_FRACTION_ADD_SUBTRACT: CurriculumUnit =
    CurriculumUnit::new("grade4-fraction-add-subtract", "同分母の分数の加法，減法");
pub const CURRICULUM_UNIT_GRADE5_FRACTION_ADD_SUBTRACT: CurriculumUnit =
    CurriculumUnit::new("grade5-fraction-add-subtract", "分数の加法，減法");

const ADDITION: &[ThemeTag] = &[ThemeTag::Fractions, ThemeTag::Addition];
const SUBTRACTION: &[ThemeTag] = &[ThemeTag::Fractions, ThemeTag::Subtraction];
const MULTIPLICATION: &[ThemeTag] = &[ThemeTag::Fractions, ThemeTag::Multiplication];
const DIVISION: &[ThemeTag] = &[ThemeTag::Fractions, ThemeTag::Division];
const SUMMARY: &[ThemeTag] = &[
    ThemeTag::Fractions,
    ThemeTag::Addition,
    ThemeTag::Subtraction,
    ThemeTag::Multiplication,
    ThemeTag::Division,
];

pub const SUMMARY_LAYERS: [SamplingLayerSpec; 4] = [
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

const MIXED_PRESENTATION: Presentation =
    Presentation::STANDARD.with_fraction(FractionPresentation::MixedNumberWhenImproper);
const IMPROPER_PRESENTATION: Presentation =
    Presentation::STANDARD.with_fraction(FractionPresentation::KeepImproperFraction);
const FRACTION_ANSWER: AnswerContract = AnswerContract::ArithmeticFraction;

pub const FRACTION_ADDITION_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_FRACTION_ADDITION),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_FRACTION_ADDITION,
        ),
        skill_id: SKILL_ID_FRACTION_ADDITION,
        curriculum_path: &CURRICULUM_PATH_FRACTION_ADDITION,
        grade: Some(SchoolGrade::Elementary5),
        tags: ADDITION,
        safety: Safety::NonNegativeOnly,
        presentation: MIXED_PRESENTATION,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: FRACTION_ANSWER,
        layout: COMPACT_16_LAYOUT,
    })
    .with_curriculum_unit(CURRICULUM_UNIT_GRADE5_FRACTION_ADD_SUBTRACT);
pub const FRACTION_SUBTRACTION_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_FRACTION_SUBTRACTION),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_FRACTION_SUBTRACTION,
        ),
        skill_id: SKILL_ID_FRACTION_SUBTRACTION,
        curriculum_path: &CURRICULUM_PATH_FRACTION_SUBTRACTION,
        grade: Some(SchoolGrade::Elementary5),
        tags: SUBTRACTION,
        safety: Safety::NonNegativeOnly,
        presentation: MIXED_PRESENTATION,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: FRACTION_ANSWER,
        layout: COMPACT_16_LAYOUT,
    })
    .with_curriculum_unit(CURRICULUM_UNIT_GRADE5_FRACTION_ADD_SUBTRACT);

pub const FRACTION_ADDITION_SAME_DENOMINATOR_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_FRACTION_ADDITION_SAME_DENOMINATOR),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_FRACTION_ADDITION_SAME_DENOMINATOR,
        ),
        skill_id: SKILL_ID_FRACTION_ADDITION_SAME_DENOMINATOR,
        curriculum_path: &CURRICULUM_PATH_FRACTION_ADDITION_SAME_DENOMINATOR,
        grade: Some(SchoolGrade::Elementary4),
        tags: ADDITION,
        safety: Safety::NonNegativeOnly,
        presentation: MIXED_PRESENTATION,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: FRACTION_ANSWER,
        layout: COMPACT_16_LAYOUT,
    })
    .with_curriculum_unit(CURRICULUM_UNIT_GRADE4_FRACTION_ADD_SUBTRACT);
pub const FRACTION_SUBTRACTION_SAME_DENOMINATOR_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(
            THEME_ID_FRACTION_SUBTRACTION_SAME_DENOMINATOR,
        ),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_FRACTION_SUBTRACTION_SAME_DENOMINATOR,
        ),
        skill_id: SKILL_ID_FRACTION_SUBTRACTION_SAME_DENOMINATOR,
        curriculum_path: &CURRICULUM_PATH_FRACTION_SUBTRACTION_SAME_DENOMINATOR,
        grade: Some(SchoolGrade::Elementary4),
        tags: SUBTRACTION,
        safety: Safety::NonNegativeOnly,
        presentation: MIXED_PRESENTATION,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: FRACTION_ANSWER,
        layout: COMPACT_16_LAYOUT,
    })
    .with_curriculum_unit(CURRICULUM_UNIT_GRADE4_FRACTION_ADD_SUBTRACT);
pub const FRACTION_ADDITION_UNLIKE_DENOMINATOR_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_FRACTION_ADDITION_UNLIKE_DENOMINATOR),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_FRACTION_ADDITION_UNLIKE_DENOMINATOR,
        ),
        skill_id: SKILL_ID_FRACTION_ADDITION_UNLIKE_DENOMINATOR,
        curriculum_path: &CURRICULUM_PATH_FRACTION_ADDITION_UNLIKE_DENOMINATOR,
        grade: Some(SchoolGrade::Elementary5),
        tags: ADDITION,
        safety: Safety::NonNegativeOnly,
        presentation: MIXED_PRESENTATION,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: FRACTION_ANSWER,
        layout: COMPACT_16_LAYOUT,
    })
    .with_curriculum_unit(CURRICULUM_UNIT_GRADE5_FRACTION_ADD_SUBTRACT);
pub const FRACTION_SUBTRACTION_UNLIKE_DENOMINATOR_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(
            THEME_ID_FRACTION_SUBTRACTION_UNLIKE_DENOMINATOR,
        ),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_FRACTION_SUBTRACTION_UNLIKE_DENOMINATOR,
        ),
        skill_id: SKILL_ID_FRACTION_SUBTRACTION_UNLIKE_DENOMINATOR,
        curriculum_path: &CURRICULUM_PATH_FRACTION_SUBTRACTION_UNLIKE_DENOMINATOR,
        grade: Some(SchoolGrade::Elementary5),
        tags: SUBTRACTION,
        safety: Safety::NonNegativeOnly,
        presentation: MIXED_PRESENTATION,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: FRACTION_ANSWER,
        layout: COMPACT_16_LAYOUT,
    })
    .with_curriculum_unit(CURRICULUM_UNIT_GRADE5_FRACTION_ADD_SUBTRACT);
pub const FRACTION_MULTIPLICATION_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_FRACTION_MULTIPLICATION),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_FRACTION_MULTIPLICATION,
        ),
        skill_id: SKILL_ID_FRACTION_MULTIPLICATION,
        curriculum_path: &CURRICULUM_PATH_FRACTION_MULTIPLICATION,
        grade: Some(SchoolGrade::Elementary6),
        tags: MULTIPLICATION,
        safety: Safety::NonNegativeOnly,
        presentation: MIXED_PRESENTATION,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: FRACTION_ANSWER,
        layout: COMPACT_16_LAYOUT,
    });
pub const FRACTION_DIVISION_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_FRACTION_DIVISION),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_FRACTION_DIVISION,
        ),
        skill_id: SKILL_ID_FRACTION_DIVISION,
        curriculum_path: &CURRICULUM_PATH_FRACTION_DIVISION,
        grade: Some(SchoolGrade::Elementary6),
        tags: DIVISION,
        safety: Safety::NonNegativeOnly,
        presentation: MIXED_PRESENTATION,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: FRACTION_ANSWER,
        layout: COMPACT_16_LAYOUT,
    });
pub const FRACTION_INTEGER_MULTIPLICATION_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_FRACTION_INTEGER_MULTIPLICATION),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_FRACTION_INTEGER_MULTIPLICATION,
        ),
        skill_id: SKILL_ID_FRACTION_INTEGER_MULTIPLICATION,
        curriculum_path: &CURRICULUM_PATH_FRACTION_INTEGER_MULTIPLICATION,
        grade: Some(SchoolGrade::Elementary6),
        tags: MULTIPLICATION,
        safety: Safety::NonNegativeOnly,
        presentation: MIXED_PRESENTATION,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: FRACTION_ANSWER,
        layout: COMPACT_16_LAYOUT,
    });
pub const FRACTION_INTEGER_DIVISION_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_FRACTION_INTEGER_DIVISION),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_FRACTION_INTEGER_DIVISION,
        ),
        skill_id: SKILL_ID_FRACTION_INTEGER_DIVISION,
        curriculum_path: &CURRICULUM_PATH_FRACTION_INTEGER_DIVISION,
        grade: Some(SchoolGrade::Elementary6),
        tags: DIVISION,
        safety: Safety::NonNegativeOnly,
        presentation: MIXED_PRESENTATION,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: FRACTION_ANSWER,
        layout: COMPACT_16_LAYOUT,
    });
pub const FRACTION_SUMMARY_IMPROPER_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_FRACTION_SUMMARY_IMPROPER),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_FRACTION_SUMMARY_IMPROPER,
        ),
        skill_id: SKILL_ID_FRACTION_SUMMARY_IMPROPER,
        curriculum_path: &CURRICULUM_PATH_FRACTION_SUMMARY_IMPROPER,
        grade: Some(SchoolGrade::Elementary6),
        tags: SUMMARY,
        safety: Safety::NonNegativeOnly,
        presentation: IMPROPER_PRESENTATION,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: FRACTION_ANSWER,
        layout: COMPACT_16_LAYOUT,
    });

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DenominatorRelation {
    Any,
    Equal,
    Unequal,
}

impl DenominatorRelation {
    fn accepts(self, left: RationalCoefficient, right: RationalCoefficient) -> bool {
        match self {
            Self::Any => true,
            Self::Equal => left.denominator() == right.denominator(),
            Self::Unequal => left.denominator() != right.denominator(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Mode {
    Addition,
    Subtraction,
    Multiplication,
    Division,
    IntegerMultiplication,
    IntegerDivision,
    SummaryImproper,
}

#[derive(Clone, Copy, Debug)]
struct FractionEntry {
    operator: ArithmeticOperator,
    left: RationalCoefficient,
    right: RationalCoefficient,
    result: RationalCoefficient,
}

#[derive(Debug)]
pub(crate) struct Generator {
    registration: &'static ThemeRegistration,
    mode: Mode,
    denominator_relation: DenominatorRelation,
}

impl ProblemGenerator for Generator {
    fn registration(&self) -> &'static ThemeRegistration {
        self.registration
    }

    fn sampling_strategy(&self) -> Result<SamplingStrategy<'_>, crate::error::SamplingError> {
        if self.mode == Mode::SummaryImproper {
            SamplingStrategy::constructive_layered(
                self,
                SelectionDedup::Deduplicate,
                1,
                self.registration.layout().problem_count(),
            )
        } else {
            Ok(SamplingStrategy::random(self, SelectionDedup::Deduplicate))
        }
    }
}

impl RandomCandidateSource for Generator {
    fn draw_candidate(
        &self,
        rng: &mut DeterministicRng,
        ordinal: u32,
        weights: &OperationWeights,
    ) -> Result<Option<Problem>, GenerationError> {
        let Some(entry) = draw_constructive_entry(self.mode, self.denominator_relation, rng) else {
            return Ok(None);
        };
        build_problem(self.registration, self.mode, ordinal, weights, entry).map(Some)
    }
}

impl ConstructiveLayeredCandidateSource for Generator {
    fn layers(&self) -> &'static [SamplingLayerSpec] {
        &SUMMARY_LAYERS
    }

    fn layer_of(&self, problem: &Problem) -> usize {
        let ProblemPrompt::Arithmetic {
            expression: ArithmeticExpression::Binary { operator, .. },
        } = problem.prompt()
        else {
            unreachable!("fraction summary generator always emits arithmetic binary prompts");
        };
        match operator {
            ArithmeticOperator::Add => 0,
            ArithmeticOperator::Subtract => 1,
            ArithmeticOperator::Multiply => 2,
            ArithmeticOperator::Divide => 3,
        }
    }

    fn draw_candidate_for_layer(
        &self,
        rng: &mut DeterministicRng,
        ordinal: u32,
        weights: &OperationWeights,
        layer: usize,
    ) -> Result<Option<Problem>, GenerationError> {
        let Some(entry) = draw_summary_entry_for_layer(layer, rng) else {
            return Ok(None);
        };
        build_problem(self.registration, self.mode, ordinal, weights, entry).map(Some)
    }
}

macro_rules! generator {
    ($name:ident, $registration:ident, $mode:ident, $relation:ident) => {
        pub(crate) static $name: Generator = Generator {
            registration: &$registration,
            mode: Mode::$mode,
            denominator_relation: DenominatorRelation::$relation,
        };
    };
}

generator!(
    ADDITION_GENERATOR,
    FRACTION_ADDITION_REGISTRATION,
    Addition,
    Any
);
generator!(
    SUBTRACTION_GENERATOR,
    FRACTION_SUBTRACTION_REGISTRATION,
    Subtraction,
    Any
);
generator!(
    ADDITION_SAME_DENOMINATOR_GENERATOR,
    FRACTION_ADDITION_SAME_DENOMINATOR_REGISTRATION,
    Addition,
    Equal
);
generator!(
    SUBTRACTION_SAME_DENOMINATOR_GENERATOR,
    FRACTION_SUBTRACTION_SAME_DENOMINATOR_REGISTRATION,
    Subtraction,
    Equal
);
generator!(
    ADDITION_UNLIKE_DENOMINATOR_GENERATOR,
    FRACTION_ADDITION_UNLIKE_DENOMINATOR_REGISTRATION,
    Addition,
    Unequal
);
generator!(
    SUBTRACTION_UNLIKE_DENOMINATOR_GENERATOR,
    FRACTION_SUBTRACTION_UNLIKE_DENOMINATOR_REGISTRATION,
    Subtraction,
    Unequal
);
generator!(
    MULTIPLICATION_GENERATOR,
    FRACTION_MULTIPLICATION_REGISTRATION,
    Multiplication,
    Any
);
generator!(
    DIVISION_GENERATOR,
    FRACTION_DIVISION_REGISTRATION,
    Division,
    Any
);
generator!(
    INTEGER_MULTIPLICATION_GENERATOR,
    FRACTION_INTEGER_MULTIPLICATION_REGISTRATION,
    IntegerMultiplication,
    Any
);
generator!(
    INTEGER_DIVISION_GENERATOR,
    FRACTION_INTEGER_DIVISION_REGISTRATION,
    IntegerDivision,
    Any
);
generator!(
    SUMMARY_IMPROPER_GENERATOR,
    FRACTION_SUMMARY_IMPROPER_REGISTRATION,
    SummaryImproper,
    Any
);

fn draw_pair_from_domain(
    rng: &mut DeterministicRng,
    domain: &[RationalCoefficient],
    unordered: bool,
) -> Option<(RationalCoefficient, RationalCoefficient)> {
    if domain.is_empty() {
        return None;
    }
    if !unordered {
        let left = domain[rng.next_bounded(domain.len() as u64) as usize];
        let right = domain[rng.next_bounded(domain.len() as u64) as usize];
        return Some((left, right));
    }

    // Uniformly sample one pair from the upper triangle i <= j. Sampling two
    // independent operands and sorting would double the weight of off-diagonal
    // candidates, so use a triangular index instead.
    let pair_count = domain.len().checked_mul(domain.len() + 1)? / 2;
    let mut index = rng.next_bounded(pair_count as u64) as usize;
    for left_index in 0..domain.len() {
        let row_len = domain.len() - left_index;
        if index < row_len {
            return Some((domain[left_index], domain[left_index + index]));
        }
        index -= row_len;
    }
    None
}

fn small_positive_integer(value: RationalCoefficient) -> bool {
    value.denominator() == 1 && (1..=9).contains(&value.numerator())
}

fn draw_standard_fraction_entry(
    mode: Mode,
    denominator_relation: DenominatorRelation,
    rng: &mut DeterministicRng,
) -> Option<FractionEntry> {
    let operator = operator_for_mode(mode)?;
    let unordered = matches!(
        operator,
        ArithmeticOperator::Add | ArithmeticOperator::Multiply
    );
    let (left, right) = draw_pair_from_domain(rng, operand_domain(), unordered)?;
    if !denominator_relation.accepts(left, right) {
        return None;
    }
    let result = match operator {
        ArithmeticOperator::Add => left.checked_add(right),
        ArithmeticOperator::Subtract => left.subtract(right),
        ArithmeticOperator::Multiply => left.multiply(right),
        ArithmeticOperator::Divide => left.divide(right),
    }?;
    if result.numerator() <= 0 {
        return None;
    }
    let allowed = match mode {
        Mode::Addition => {
            result.denominator() > 1 && result.numerator() <= 65 && result.denominator() <= 72
        }
        Mode::Subtraction | Mode::Multiplication => operand_domain().contains(&result),
        Mode::Division => operand_domain().contains(&result) || small_positive_integer(result),
        _ => return None,
    };
    allowed.then_some(FractionEntry {
        operator,
        left,
        right,
        result,
    })
}

fn draw_integer_fraction_entry(mode: Mode, rng: &mut DeterministicRng) -> Option<FractionEntry> {
    let fraction = operand_domain()[rng.next_bounded(operand_domain().len() as u64) as usize];
    let integer = RationalCoefficient::new(1 + rng.next_bounded(9) as i64, 1)?;
    let operator = operator_for_mode(mode)?;
    let (left, right) = match operator {
        ArithmeticOperator::Multiply => (fraction, integer),
        ArithmeticOperator::Divide if rng.next_bounded(2) == 0 => (fraction, integer),
        ArithmeticOperator::Divide => (integer, fraction),
        _ => return None,
    };
    let result = match operator {
        ArithmeticOperator::Multiply => left.multiply(right),
        ArithmeticOperator::Divide => left.divide(right),
        _ => return None,
    }?;
    let allowed = result.numerator() > 0
        && (operand_domain().contains(&result) || small_positive_integer(result));
    allowed.then_some(FractionEntry {
        operator,
        left,
        right,
        result,
    })
}

fn summary_operand_domain() -> &'static [RationalCoefficient] {
    static VALUES: OnceLock<Vec<RationalCoefficient>> = OnceLock::new();
    VALUES.get_or_init(|| {
        let mut values = operand_domain().to_vec();
        values.extend((1_i64..=9).map(|value| RationalCoefficient::new(value, 1).unwrap()));
        values.sort_unstable();
        values.dedup();
        values
    })
}

fn draw_summary_entry_for_layer(layer: usize, rng: &mut DeterministicRng) -> Option<FractionEntry> {
    match layer {
        0 => draw_standard_fraction_entry(Mode::Addition, DenominatorRelation::Any, rng),
        1 => draw_standard_fraction_entry(Mode::Subtraction, DenominatorRelation::Any, rng),
        2 | 3 => {
            let operator = if layer == 2 {
                ArithmeticOperator::Multiply
            } else {
                ArithmeticOperator::Divide
            };
            let operands = summary_operand_domain();
            let (left, right) =
                draw_pair_from_domain(rng, operands, operator == ArithmeticOperator::Multiply)?;
            if left.is_integer() && right.is_integer() {
                return None;
            }
            let result = match operator {
                ArithmeticOperator::Multiply => left.multiply(right),
                ArithmeticOperator::Divide => left.divide(right),
                _ => unreachable!(),
            }?;
            (result.numerator() > 0 && result.numerator() <= 200 && result.denominator() <= 196)
                .then_some(FractionEntry {
                    operator,
                    left,
                    right,
                    result,
                })
        }
        _ => None,
    }
}

fn draw_summary_entry(rng: &mut DeterministicRng) -> Option<FractionEntry> {
    draw_summary_entry_for_layer(rng.next_bounded(4) as usize, rng)
}

fn draw_constructive_entry(
    mode: Mode,
    denominator_relation: DenominatorRelation,
    rng: &mut DeterministicRng,
) -> Option<FractionEntry> {
    match mode {
        Mode::Addition | Mode::Subtraction | Mode::Multiplication | Mode::Division => {
            draw_standard_fraction_entry(mode, denominator_relation, rng)
        }
        Mode::IntegerMultiplication | Mode::IntegerDivision => {
            draw_integer_fraction_entry(mode, rng)
        }
        Mode::SummaryImproper => draw_summary_entry(rng),
    }
}

// Canonical operand domain for the current fraction curriculum.
pub(crate) fn operand_domain() -> &'static [RationalCoefficient] {
    static VALUES: OnceLock<Vec<RationalCoefficient>> = OnceLock::new();
    VALUES.get_or_init(|| {
        let mut values = Vec::new();
        for denominator in 2_i64..=14 {
            for numerator in 1_i64..=(15 - denominator) {
                let Some(value) = RationalCoefficient::new(numerator, denominator) else {
                    continue;
                };
                if value.denominator() == 1 {
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

fn operator_for_mode(mode: Mode) -> Option<ArithmeticOperator> {
    match mode {
        Mode::Addition => Some(ArithmeticOperator::Add),
        Mode::Subtraction => Some(ArithmeticOperator::Subtract),
        Mode::Multiplication | Mode::IntegerMultiplication => Some(ArithmeticOperator::Multiply),
        Mode::Division | Mode::IntegerDivision => Some(ArithmeticOperator::Divide),
        Mode::SummaryImproper => None,
    }
}

fn build_problem(
    registration: &ThemeRegistration,
    mode: Mode,
    id: u32,
    _weights: &OperationWeights,
    entry: FractionEntry,
) -> Result<Problem, GenerationError> {
    let FractionEntry {
        operator,
        left,
        right,
        result,
    } = entry;
    let expression = binary_expression(
        operator,
        rational_expression(left),
        rational_expression(right),
    );
    let answer = if mode == Mode::SummaryImproper {
        rational_answer(result)
    } else {
        mixed_number_answer(result)
    };
    let self_division = operator == ArithmeticOperator::Divide && left == right;
    let operation_plan = if self_division {
        OperationPlan::new(vec![Operation::FractionSelfDivision])
    } else {
        arithmetic_expression_plan(&expression, &rational_answer(result)).ok_or(
            GenerationError::InvalidGeneratedProblem {
                reason: "fraction generator produced an expression unsupported by the effort model",
            },
        )?
    };
    let (max_abs_numerator, max_denominator) = match mode {
        Mode::Addition => (65, 72),
        Mode::Subtraction
        | Mode::Multiplication
        | Mode::Division
        | Mode::IntegerMultiplication
        | Mode::IntegerDivision => (13, 14),
        Mode::SummaryImproper => (200, 196),
    };
    Problem::generated(
        registration,
        id,
        ProblemPrompt::Arithmetic { expression },
        AnswerSchema::Rational {
            max_abs_numerator,
            max_denominator,
            require_reduced_fraction_form: true,
        },
        answer,
        EffortModel::operations(operation_plan),
    )
    .map_err(GenerationError::from)
}

/// Current generators owned by this theme family.
pub(crate) static GENERATORS: [GeneratorEntry; 11] = [
    GeneratorEntry::current(&ADDITION_GENERATOR),
    GeneratorEntry::current(&SUBTRACTION_GENERATOR),
    GeneratorEntry::current(&ADDITION_SAME_DENOMINATOR_GENERATOR),
    GeneratorEntry::current(&SUBTRACTION_SAME_DENOMINATOR_GENERATOR),
    GeneratorEntry::current(&ADDITION_UNLIKE_DENOMINATOR_GENERATOR),
    GeneratorEntry::current(&SUBTRACTION_UNLIKE_DENOMINATOR_GENERATOR),
    GeneratorEntry::current(&MULTIPLICATION_GENERATOR),
    GeneratorEntry::current(&DIVISION_GENERATOR),
    GeneratorEntry::current(&INTEGER_MULTIPLICATION_GENERATOR),
    GeneratorEntry::current(&INTEGER_DIVISION_GENERATOR),
    GeneratorEntry::current(&SUMMARY_IMPROPER_GENERATOR),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effort::OperationKind;

    #[test]
    fn fraction_self_division_uses_one_unit_primitive() {
        let value = RationalCoefficient::new(2, 3).unwrap();
        let one = RationalCoefficient::new(1, 1).unwrap();
        let problem = build_problem(
            &FRACTION_DIVISION_REGISTRATION,
            Mode::Division,
            1,
            &OperationWeights::default(),
            FractionEntry {
                operator: ArithmeticOperator::Divide,
                left: value,
                right: value,
                result: one,
            },
        )
        .expect("fraction candidate must satisfy its problem contract");
        assert_eq!(problem.effort(), 1.0);
        assert_eq!(
            problem
                .operation_vector()
                .get(OperationKind::FractionSelfDivision),
            1.0
        );
    }
}

#[cfg(test)]
mod curriculum_tests {
    use super::*;
    use crate::generator::{generate_worksheet_request, ConstructiveLayeredCandidateSource};
    use crate::identity::Difficulty;
    use crate::model::{EditorStructure, GenerateWorksheetRequest};
    use crate::schema::SCHEMA_VERSION;

    fn draw_entry(
        mode: Mode,
        denominator_relation: DenominatorRelation,
        rng: &mut DeterministicRng,
    ) -> FractionEntry {
        for _ in 0..10_000 {
            if let Some(entry) = draw_constructive_entry(mode, denominator_relation, rng) {
                return entry;
            }
        }
        panic!("fraction mode failed to produce an accepted entry");
    }

    #[test]
    fn fraction_modes_obey_family_owned_operand_and_result_domains() {
        let mut rng = DeterministicRng::from_seed("FractionDomainA1");
        for mode in [
            Mode::Addition,
            Mode::Subtraction,
            Mode::Multiplication,
            Mode::Division,
        ] {
            for _ in 0..512 {
                let entry = draw_entry(mode, DenominatorRelation::Any, &mut rng);
                assert!(operand_domain().contains(&entry.left));
                assert!(operand_domain().contains(&entry.right));
                assert!(entry.left.numerator() > 0 && entry.right.numerator() > 0);
                assert!(entry.result.numerator() > 0);
                match mode {
                    Mode::Addition => {
                        assert_eq!(entry.operator, ArithmeticOperator::Add);
                        assert!(entry.result.denominator() > 1);
                        assert!(entry.result.numerator() <= 65);
                        assert!(entry.result.denominator() <= 72);
                    }
                    Mode::Subtraction => {
                        assert_eq!(entry.operator, ArithmeticOperator::Subtract);
                        assert!(operand_domain().contains(&entry.result));
                    }
                    Mode::Multiplication => {
                        assert_eq!(entry.operator, ArithmeticOperator::Multiply);
                        assert!(operand_domain().contains(&entry.result));
                    }
                    Mode::Division => {
                        assert_eq!(entry.operator, ArithmeticOperator::Divide);
                        assert!(
                            operand_domain().contains(&entry.result)
                                || small_positive_integer(entry.result)
                        );
                    }
                    _ => unreachable!(),
                }
            }
        }
    }

    #[test]
    fn integer_fraction_units_keep_integer_operands_out_of_standard_units() {
        let mut rng = DeterministicRng::from_seed("FractionIntegerB2");
        for _ in 0..512 {
            for mode in [Mode::Multiplication, Mode::Division] {
                let entry = draw_entry(mode, DenominatorRelation::Any, &mut rng);
                assert!(!entry.left.is_integer());
                assert!(!entry.right.is_integer());
            }
            let multiplication = draw_entry(
                Mode::IntegerMultiplication,
                DenominatorRelation::Any,
                &mut rng,
            );
            assert!(!multiplication.left.is_integer());
            assert!(multiplication.right.is_integer());

            let division = draw_entry(Mode::IntegerDivision, DenominatorRelation::Any, &mut rng);
            assert_ne!(division.left.is_integer(), division.right.is_integer());
        }
    }

    #[test]
    fn fraction_answer_contract_allows_decimal_compatibility_input() {
        for registration in [
            &FRACTION_ADDITION_REGISTRATION,
            &FRACTION_SUBTRACTION_REGISTRATION,
            &FRACTION_ADDITION_SAME_DENOMINATOR_REGISTRATION,
            &FRACTION_SUBTRACTION_SAME_DENOMINATOR_REGISTRATION,
            &FRACTION_ADDITION_UNLIKE_DENOMINATOR_REGISTRATION,
            &FRACTION_SUBTRACTION_UNLIKE_DENOMINATOR_REGISTRATION,
            &FRACTION_MULTIPLICATION_REGISTRATION,
            &FRACTION_DIVISION_REGISTRATION,
            &FRACTION_INTEGER_MULTIPLICATION_REGISTRATION,
            &FRACTION_INTEGER_DIVISION_REGISTRATION,
            &FRACTION_SUMMARY_IMPROPER_REGISTRATION,
        ] {
            let interface =
                crate::input::input_interface(registration.answer_contract().input_profile());
            assert!(interface.allows_structure(EditorStructure::Decimal));
        }
    }

    fn fraction_prompt_denominators(problem: &Problem) -> (ArithmeticOperator, i64, i64) {
        let ProblemPrompt::Arithmetic {
            expression:
                ArithmeticExpression::Binary {
                    operator,
                    left,
                    right,
                },
        } = problem.prompt()
        else {
            panic!("fraction curriculum theme returned a non-binary arithmetic prompt");
        };
        let (
            ArithmeticExpression::Rational { value: left },
            ArithmeticExpression::Rational { value: right },
        ) = (&**left, &**right)
        else {
            panic!("fraction curriculum theme returned non-rational operands");
        };
        (*operator, left.denominator(), right.denominator())
    }

    #[test]
    fn dedicated_fraction_add_subtract_themes_enforce_denominator_relation() {
        let cases = [
            (
                THEME_ID_FRACTION_ADDITION_SAME_DENOMINATOR,
                ArithmeticOperator::Add,
                DenominatorRelation::Equal,
            ),
            (
                THEME_ID_FRACTION_SUBTRACTION_SAME_DENOMINATOR,
                ArithmeticOperator::Subtract,
                DenominatorRelation::Equal,
            ),
            (
                THEME_ID_FRACTION_ADDITION_UNLIKE_DENOMINATOR,
                ArithmeticOperator::Add,
                DenominatorRelation::Unequal,
            ),
            (
                THEME_ID_FRACTION_SUBTRACTION_UNLIKE_DENOMINATOR,
                ArithmeticOperator::Subtract,
                DenominatorRelation::Unequal,
            ),
        ];
        for (theme_id, expected_operator, relation) in cases {
            for difficulty in 1..=4 {
                for seed in ["A1b2", "M7x9", "Q4r6"] {
                    let worksheet = generate_worksheet_request(&GenerateWorksheetRequest {
                        schema_version: SCHEMA_VERSION,
                        numeric_theme_id: theme_id,
                        seed: seed.to_owned(),
                        difficulty: Difficulty::try_from(difficulty).unwrap(),
                        timeout_ms: Some(1_000),
                        max_attempts: Some(50_000),
                    })
                    .unwrap_or_else(|error| {
                        panic!("fraction relation theme {theme_id} failed at d{difficulty} seed={seed}: {error}")
                    });
                    for problem in worksheet.problems() {
                        let (operator, left_denominator, right_denominator) =
                            fraction_prompt_denominators(problem);
                        assert_eq!(operator, expected_operator);
                        match relation {
                            DenominatorRelation::Equal => {
                                assert_eq!(left_denominator, right_denominator);
                            }
                            DenominatorRelation::Unequal => {
                                assert_ne!(left_denominator, right_denominator);
                            }
                            DenominatorRelation::Any => unreachable!(),
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn fraction_add_subtract_curriculum_units_match_grade_boundary() {
        for registration in [
            &FRACTION_ADDITION_SAME_DENOMINATOR_REGISTRATION,
            &FRACTION_SUBTRACTION_SAME_DENOMINATOR_REGISTRATION,
        ] {
            assert_eq!(registration.grade(), Some(SchoolGrade::Elementary4));
            assert_eq!(
                registration.curriculum_unit(),
                CURRICULUM_UNIT_GRADE4_FRACTION_ADD_SUBTRACT
            );
        }
        for registration in [
            &FRACTION_ADDITION_UNLIKE_DENOMINATOR_REGISTRATION,
            &FRACTION_SUBTRACTION_UNLIKE_DENOMINATOR_REGISTRATION,
            &FRACTION_ADDITION_REGISTRATION,
            &FRACTION_SUBTRACTION_REGISTRATION,
        ] {
            assert_eq!(registration.grade(), Some(SchoolGrade::Elementary5));
            assert_eq!(
                registration.curriculum_unit(),
                CURRICULUM_UNIT_GRADE5_FRACTION_ADD_SUBTRACT
            );
        }
        assert!(CURRICULUM_PATH_FRACTION_ADDITION
            .last()
            .unwrap()
            .contains("まとめ"));
        assert!(CURRICULUM_PATH_FRACTION_SUBTRACTION
            .last()
            .unwrap()
            .contains("まとめ"));
    }

    #[test]
    fn summary_layer_classifier_is_family_owned_and_total() {
        let mut rng = DeterministicRng::from_seed("FractionLayerC3");
        let weights = OperationWeights::default();
        for layer in 0..SUMMARY_LAYERS.len() {
            for ordinal in 1..=32 {
                let problem = (0..10_000)
                    .find_map(|_| {
                        SUMMARY_IMPROPER_GENERATOR
                            .draw_candidate_for_layer(&mut rng, ordinal, &weights, layer)
                            .expect("candidate construction must preserve the problem contract")
                    })
                    .expect("every summary layer must have accepted candidates");
                assert_eq!(SUMMARY_IMPROPER_GENERATOR.layer_of(&problem), layer);
            }
        }
    }
}
