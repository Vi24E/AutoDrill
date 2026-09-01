use crate::answer::AnswerNode;
use crate::effort::{
    arithmetic_expression_plan, integer_division_with_remainder_plan, EffortModel, OperationWeights,
};
use crate::error::GenerationError;
use crate::generator::{
    GeneratorEntry, LayeredCandidateSource, ProblemGenerator, RandomCandidateSource,
    SamplingStrategy, SelectionDedup,
};
use crate::generator_support::{
    arithmetic_leaf_column_grid_cells, arithmetic_leaf_significant_digits, binary_expression,
    draw_decimal_coefficient, draw_decimal_operand, draw_decimal_operand_with_significant_digits,
    exact_decimal_expression, exact_decimal_rational, integer_expression, rational_less_than,
    rational_to_arithmetic_expression, rational_to_exact_decimal_answer,
};
use crate::model::{AnswerSchema, ArithmeticOperator, Problem, ProblemPrompt, RationalCoefficient};
use crate::rng::DeterministicRng;
use crate::theme::{
    CurriculumSafetyPolicy as Safety, CurriculumUnit, DedupPolicy as Dedup, SamplingLayerSpec,
    SchoolGrade, ThemeAnswerContract as AnswerContract, ThemePresentationPolicy as Presentation,
    ThemeRegistration, ThemeRegistrationSpec, ThemeTag, COLUMN_16_LAYOUT,
    COLUMN_DIVISION_12_LAYOUT,
};

pub const THEME_ID_COLUMN_ADD_2DIGIT: u32 = 25;
pub const THEME_ID_COLUMN_SUBTRACT_2DIGIT: u32 = 26;
pub const THEME_ID_COLUMN_ADD_3_4DIGIT: u32 = 27;
pub const THEME_ID_COLUMN_SUBTRACT_3_4DIGIT: u32 = 28;
pub const THEME_ID_COLUMN_MULTIPLY_1DIGIT: u32 = 29;
pub const THEME_ID_COLUMN_MULTIPLY_2DIGIT: u32 = 30;
pub const THEME_ID_COLUMN_DIVIDE_2DIGIT_BY_1DIGIT: u32 = 31;
pub const THEME_ID_COLUMN_DIVIDE_2DIGIT: u32 = 32;
pub const THEME_ID_COLUMN_DECIMAL_ADD_SUBTRACT: u32 = 33;
pub const THEME_ID_COLUMN_DECIMAL_MULTIPLY_INTEGER: u32 = 34;
pub const THEME_ID_COLUMN_DECIMAL_DIVIDE_INTEGER: u32 = 35;
pub const THEME_ID_COLUMN_DECIMAL_MULTIPLICATION: u32 = 36;
pub const THEME_ID_COLUMN_DECIMAL_DIVISION: u32 = 37;
pub const THEME_ID_COLUMN_DIVIDE_3DIGIT_BY_1DIGIT: u32 = 54;
pub const GENERATOR_REVISION_COLUMN_ADD_2DIGIT: u32 = 2;
pub const GENERATOR_REVISION_COLUMN_SUBTRACT_2DIGIT: u32 = 2;
pub const GENERATOR_REVISION_COLUMN_ADD_3_4DIGIT: u32 = 2;
pub const GENERATOR_REVISION_COLUMN_SUBTRACT_3_4DIGIT: u32 = 2;
pub const GENERATOR_REVISION_COLUMN_MULTIPLY_1DIGIT: u32 = 2;
pub const GENERATOR_REVISION_COLUMN_MULTIPLY_2DIGIT: u32 = 2;
pub const GENERATOR_REVISION_COLUMN_DIVIDE_2DIGIT_BY_1DIGIT: u32 = 3;
pub const GENERATOR_REVISION_COLUMN_DIVIDE_2DIGIT: u32 = 2;
pub const GENERATOR_REVISION_COLUMN_DECIMAL_ADD_SUBTRACT: u32 = 2;
pub const GENERATOR_REVISION_COLUMN_DECIMAL_MULTIPLY_INTEGER: u32 = 2;
pub const GENERATOR_REVISION_COLUMN_DECIMAL_DIVIDE_INTEGER: u32 = 2;
pub const GENERATOR_REVISION_COLUMN_DECIMAL_MULTIPLICATION: u32 = 3;
pub const GENERATOR_REVISION_COLUMN_DECIMAL_DIVISION: u32 = 2;
pub const GENERATOR_REVISION_COLUMN_DIVIDE_3DIGIT_BY_1DIGIT: u32 = 1;
pub const SKILL_ID_COLUMN_ADD_2DIGIT: &str = "jp.grade2.column.addition.two_digit";
pub const SKILL_ID_COLUMN_SUBTRACT_2DIGIT: &str = "jp.grade2.column.subtraction.two_digit";
pub const SKILL_ID_COLUMN_ADD_3_4DIGIT: &str = "jp.grade3.column.addition.three_four_digit";
pub const SKILL_ID_COLUMN_SUBTRACT_3_4DIGIT: &str = "jp.grade3.column.subtraction.three_four_digit";
pub const SKILL_ID_COLUMN_MULTIPLY_1DIGIT: &str =
    "jp.grade3.column.multiplication.one_digit_multiplier";
pub const SKILL_ID_COLUMN_MULTIPLY_2DIGIT: &str =
    "jp.grade3.column.multiplication.two_digit_multiplier";
pub const SKILL_ID_COLUMN_DIVIDE_2DIGIT_BY_1DIGIT: &str =
    "jp.grade4.column.division.two_digit_by_one_digit";
pub const SKILL_ID_COLUMN_DIVIDE_3DIGIT_BY_1DIGIT: &str =
    "jp.grade4.column.division.three_digit_by_one_digit";
pub const SKILL_ID_COLUMN_DIVIDE_2DIGIT: &str = "jp.grade4.column.division.two_digit_divisor";
pub const SKILL_ID_COLUMN_DECIMAL_ADD_SUBTRACT: &str = "jp.grade4.column.decimal.add_subtract";
pub const SKILL_ID_COLUMN_DECIMAL_MULTIPLY_INTEGER: &str =
    "jp.grade4.column.decimal.multiply_integer";
pub const SKILL_ID_COLUMN_DECIMAL_DIVIDE_INTEGER: &str = "jp.grade4.column.decimal.divide_integer";
pub const SKILL_ID_COLUMN_DECIMAL_MULTIPLICATION: &str = "jp.grade5.column.decimal.multiplication";
pub const SKILL_ID_COLUMN_DECIMAL_DIVISION: &str = "jp.grade5.column.decimal.division";
pub const CURRICULUM_PATH_COLUMN_ADD_2DIGIT: [&str; 4] =
    ["root", "小学2年生", "加法，減法", "二桁の足し算の筆算"];
pub const CURRICULUM_PATH_COLUMN_SUBTRACT_2DIGIT: [&str; 4] =
    ["root", "小学2年生", "加法，減法", "二桁の引き算の筆算"];
pub const CURRICULUM_PATH_COLUMN_ADD_3_4DIGIT: [&str; 4] =
    ["root", "小学3年生", "加法，減法", "三・四桁の足し算の筆算"];
pub const CURRICULUM_PATH_COLUMN_SUBTRACT_3_4DIGIT: [&str; 4] =
    ["root", "小学3年生", "加法，減法", "三・四桁の引き算の筆算"];
pub const CURRICULUM_PATH_COLUMN_MULTIPLY_1DIGIT: [&str; 4] =
    ["root", "小学3年生", "乗法", "一桁をかける掛け算の筆算"];
pub const CURRICULUM_PATH_COLUMN_MULTIPLY_2DIGIT: [&str; 4] =
    ["root", "小学3年生", "乗法", "二桁をかける掛け算の筆算"];
pub const CURRICULUM_PATH_COLUMN_DIVIDE_2DIGIT_BY_1DIGIT: [&str; 4] =
    ["root", "小学4年生", "整数の除法", "2桁÷1桁の筆算"];
pub const CURRICULUM_PATH_COLUMN_DIVIDE_3DIGIT_BY_1DIGIT: [&str; 4] =
    ["root", "小学4年生", "整数の除法", "3桁÷1桁の筆算"];
pub const CURRICULUM_PATH_COLUMN_DIVIDE_2DIGIT: [&str; 4] =
    ["root", "小学4年生", "整数の除法", "二桁で割る割り算の筆算"];
pub const CURRICULUM_PATH_COLUMN_DECIMAL_ADD_SUBTRACT: [&str; 4] = [
    "root",
    "小学4年生",
    "小数の仕組みとその計算",
    "小数の足し算と引き算の筆算",
];
pub const CURRICULUM_PATH_COLUMN_DECIMAL_MULTIPLY_INTEGER: [&str; 4] = [
    "root",
    "小学4年生",
    "小数の仕組みとその計算",
    "小数と整数の掛け算の筆算",
];
pub const CURRICULUM_PATH_COLUMN_DECIMAL_DIVIDE_INTEGER: [&str; 4] = [
    "root",
    "小学4年生",
    "小数の仕組みとその計算",
    "小数と整数の割り算の筆算",
];
pub const CURRICULUM_PATH_COLUMN_DECIMAL_MULTIPLICATION: [&str; 4] = [
    "root",
    "小学5年生",
    "小数の乗法，除法",
    "小数の掛け算の筆算",
];
pub const CURRICULUM_PATH_COLUMN_DECIMAL_DIVISION: [&str; 4] = [
    "root",
    "小学5年生",
    "小数の乗法，除法",
    "小数の割り算の筆算",
];

pub const CURRICULUM_UNIT_GRADE2_COLUMN_ADD_SUBTRACT: CurriculumUnit =
    CurriculumUnit::new("grade2-column-add-subtract", "加法，減法");
pub const CURRICULUM_UNIT_GRADE3_COLUMN_ADD_SUBTRACT: CurriculumUnit =
    CurriculumUnit::new("grade3-column-add-subtract", "加法，減法");
pub const CURRICULUM_UNIT_GRADE3_COLUMN_MULTIPLICATION: CurriculumUnit =
    CurriculumUnit::new("grade3-column-multiplication", "乗法");
pub const CURRICULUM_UNIT_GRADE4_INTEGER_DIVISION: CurriculumUnit =
    CurriculumUnit::new("grade4-integer-division", "整数の除法");
pub const CURRICULUM_UNIT_GRADE4_DECIMAL: CurriculumUnit =
    CurriculumUnit::new("grade4-decimal", "小数の仕組みとその計算");
pub const CURRICULUM_UNIT_GRADE5_DECIMAL: CurriculumUnit =
    CurriculumUnit::new("grade5-decimal", "小数の乗法，除法");

const ADDITION: &[ThemeTag] = &[ThemeTag::Addition];
const SUBTRACTION: &[ThemeTag] = &[ThemeTag::Subtraction];
const MULTIPLICATION: &[ThemeTag] = &[ThemeTag::Multiplication];
const DIVISION: &[ThemeTag] = &[ThemeTag::Division];
const DECIMAL_ADD_SUBTRACT: &[ThemeTag] = &[
    ThemeTag::Decimals,
    ThemeTag::Addition,
    ThemeTag::Subtraction,
];
const DECIMAL_MULTIPLICATION: &[ThemeTag] = &[ThemeTag::Decimals, ThemeTag::Multiplication];
const DECIMAL_DIVISION: &[ThemeTag] = &[ThemeTag::Decimals, ThemeTag::Division];

pub const DECIMAL_ADD_SUBTRACT_LAYERS: [SamplingLayerSpec; 2] = [
    SamplingLayerSpec {
        weight: 1,
        minimum: 0,
    },
    SamplingLayerSpec {
        weight: 1,
        minimum: 0,
    },
];

const INTEGER_COLUMN: AnswerContract = AnswerContract::ColumnInteger;
const INTEGER_DIVISION_COLUMN: AnswerContract = AnswerContract::ColumnIntegerDivision;
const DECIMAL_COLUMN: AnswerContract = AnswerContract::ColumnDecimal;

pub const COLUMN_ADD_2DIGIT_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_COLUMN_ADD_2DIGIT),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_COLUMN_ADD_2DIGIT,
        ),
        skill_id: SKILL_ID_COLUMN_ADD_2DIGIT,
        curriculum_path: &CURRICULUM_PATH_COLUMN_ADD_2DIGIT,
        grade: Some(SchoolGrade::Elementary2),
        tags: ADDITION,
        safety: Safety::NonNegativeOnly,
        presentation: Presentation::COLUMN_ARITHMETIC,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: INTEGER_COLUMN,
        layout: COLUMN_16_LAYOUT,
    })
    .with_curriculum_unit(CURRICULUM_UNIT_GRADE2_COLUMN_ADD_SUBTRACT);

pub const COLUMN_SUBTRACT_2DIGIT_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_COLUMN_SUBTRACT_2DIGIT),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_COLUMN_SUBTRACT_2DIGIT,
        ),
        skill_id: SKILL_ID_COLUMN_SUBTRACT_2DIGIT,
        curriculum_path: &CURRICULUM_PATH_COLUMN_SUBTRACT_2DIGIT,
        grade: Some(SchoolGrade::Elementary2),
        tags: SUBTRACTION,
        safety: Safety::NonNegativeOnly,
        presentation: Presentation::COLUMN_ARITHMETIC,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: INTEGER_COLUMN,
        layout: COLUMN_16_LAYOUT,
    })
    .with_curriculum_unit(CURRICULUM_UNIT_GRADE2_COLUMN_ADD_SUBTRACT);

pub const COLUMN_ADD_3_4DIGIT_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_COLUMN_ADD_3_4DIGIT),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_COLUMN_ADD_3_4DIGIT,
        ),
        skill_id: SKILL_ID_COLUMN_ADD_3_4DIGIT,
        curriculum_path: &CURRICULUM_PATH_COLUMN_ADD_3_4DIGIT,
        grade: Some(SchoolGrade::Elementary3),
        tags: ADDITION,
        safety: Safety::NonNegativeOnly,
        presentation: Presentation::COLUMN_ARITHMETIC,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: INTEGER_COLUMN,
        layout: COLUMN_16_LAYOUT,
    })
    .with_curriculum_unit(CURRICULUM_UNIT_GRADE3_COLUMN_ADD_SUBTRACT);

pub const COLUMN_SUBTRACT_3_4DIGIT_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_COLUMN_SUBTRACT_3_4DIGIT),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_COLUMN_SUBTRACT_3_4DIGIT,
        ),
        skill_id: SKILL_ID_COLUMN_SUBTRACT_3_4DIGIT,
        curriculum_path: &CURRICULUM_PATH_COLUMN_SUBTRACT_3_4DIGIT,
        grade: Some(SchoolGrade::Elementary3),
        tags: SUBTRACTION,
        safety: Safety::NonNegativeOnly,
        presentation: Presentation::COLUMN_ARITHMETIC,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: INTEGER_COLUMN,
        layout: COLUMN_16_LAYOUT,
    })
    .with_curriculum_unit(CURRICULUM_UNIT_GRADE3_COLUMN_ADD_SUBTRACT);

pub const COLUMN_MULTIPLY_1DIGIT_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_COLUMN_MULTIPLY_1DIGIT),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_COLUMN_MULTIPLY_1DIGIT,
        ),
        skill_id: SKILL_ID_COLUMN_MULTIPLY_1DIGIT,
        curriculum_path: &CURRICULUM_PATH_COLUMN_MULTIPLY_1DIGIT,
        grade: Some(SchoolGrade::Elementary3),
        tags: MULTIPLICATION,
        safety: Safety::NonNegativeOnly,
        presentation: Presentation::COLUMN_ARITHMETIC,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: INTEGER_COLUMN,
        layout: COLUMN_16_LAYOUT,
    })
    .with_curriculum_unit(CURRICULUM_UNIT_GRADE3_COLUMN_MULTIPLICATION);

pub const COLUMN_MULTIPLY_2DIGIT_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_COLUMN_MULTIPLY_2DIGIT),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_COLUMN_MULTIPLY_2DIGIT,
        ),
        skill_id: SKILL_ID_COLUMN_MULTIPLY_2DIGIT,
        curriculum_path: &CURRICULUM_PATH_COLUMN_MULTIPLY_2DIGIT,
        grade: Some(SchoolGrade::Elementary3),
        tags: MULTIPLICATION,
        safety: Safety::NonNegativeOnly,
        presentation: Presentation::COLUMN_ARITHMETIC,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: INTEGER_COLUMN,
        layout: COLUMN_16_LAYOUT,
    })
    .with_curriculum_unit(CURRICULUM_UNIT_GRADE3_COLUMN_MULTIPLICATION);

pub const COLUMN_DIVIDE_2DIGIT_BY_1DIGIT_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_COLUMN_DIVIDE_2DIGIT_BY_1DIGIT),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_COLUMN_DIVIDE_2DIGIT_BY_1DIGIT,
        ),
        skill_id: SKILL_ID_COLUMN_DIVIDE_2DIGIT_BY_1DIGIT,
        curriculum_path: &CURRICULUM_PATH_COLUMN_DIVIDE_2DIGIT_BY_1DIGIT,
        grade: Some(SchoolGrade::Elementary4),
        tags: DIVISION,
        safety: Safety::NonNegativeOnly,
        presentation: Presentation::COLUMN_DIVISION,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: INTEGER_DIVISION_COLUMN,
        layout: COLUMN_DIVISION_12_LAYOUT,
    })
    .with_curriculum_unit(CURRICULUM_UNIT_GRADE4_INTEGER_DIVISION);

pub const COLUMN_DIVIDE_3DIGIT_BY_1DIGIT_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_COLUMN_DIVIDE_3DIGIT_BY_1DIGIT),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_COLUMN_DIVIDE_3DIGIT_BY_1DIGIT,
        ),
        skill_id: SKILL_ID_COLUMN_DIVIDE_3DIGIT_BY_1DIGIT,
        curriculum_path: &CURRICULUM_PATH_COLUMN_DIVIDE_3DIGIT_BY_1DIGIT,
        grade: Some(SchoolGrade::Elementary4),
        tags: DIVISION,
        safety: Safety::NonNegativeOnly,
        presentation: Presentation::COLUMN_DIVISION,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: INTEGER_DIVISION_COLUMN,
        layout: COLUMN_DIVISION_12_LAYOUT,
    })
    .with_curriculum_unit(CURRICULUM_UNIT_GRADE4_INTEGER_DIVISION);

pub const COLUMN_DIVIDE_2DIGIT_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_COLUMN_DIVIDE_2DIGIT),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_COLUMN_DIVIDE_2DIGIT,
        ),
        skill_id: SKILL_ID_COLUMN_DIVIDE_2DIGIT,
        curriculum_path: &CURRICULUM_PATH_COLUMN_DIVIDE_2DIGIT,
        grade: Some(SchoolGrade::Elementary4),
        tags: DIVISION,
        safety: Safety::NonNegativeOnly,
        presentation: Presentation::COLUMN_DIVISION,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: INTEGER_DIVISION_COLUMN,
        layout: COLUMN_DIVISION_12_LAYOUT,
    })
    .with_curriculum_unit(CURRICULUM_UNIT_GRADE4_INTEGER_DIVISION);

pub const COLUMN_DECIMAL_ADD_SUBTRACT_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_COLUMN_DECIMAL_ADD_SUBTRACT),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_COLUMN_DECIMAL_ADD_SUBTRACT,
        ),
        skill_id: SKILL_ID_COLUMN_DECIMAL_ADD_SUBTRACT,
        curriculum_path: &CURRICULUM_PATH_COLUMN_DECIMAL_ADD_SUBTRACT,
        grade: Some(SchoolGrade::Elementary4),
        tags: DECIMAL_ADD_SUBTRACT,
        safety: Safety::NonNegativeOnly,
        presentation: Presentation::COLUMN_DECIMAL,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: DECIMAL_COLUMN,
        layout: COLUMN_16_LAYOUT,
    })
    .with_curriculum_unit(CURRICULUM_UNIT_GRADE4_DECIMAL);

pub const COLUMN_DECIMAL_MULTIPLY_INTEGER_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_COLUMN_DECIMAL_MULTIPLY_INTEGER),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_COLUMN_DECIMAL_MULTIPLY_INTEGER,
        ),
        skill_id: SKILL_ID_COLUMN_DECIMAL_MULTIPLY_INTEGER,
        curriculum_path: &CURRICULUM_PATH_COLUMN_DECIMAL_MULTIPLY_INTEGER,
        grade: Some(SchoolGrade::Elementary4),
        tags: DECIMAL_MULTIPLICATION,
        safety: Safety::NonNegativeOnly,
        presentation: Presentation::COLUMN_DECIMAL,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: DECIMAL_COLUMN,
        layout: COLUMN_16_LAYOUT,
    })
    .with_curriculum_unit(CURRICULUM_UNIT_GRADE4_DECIMAL);

pub const COLUMN_DECIMAL_DIVIDE_INTEGER_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_COLUMN_DECIMAL_DIVIDE_INTEGER),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_COLUMN_DECIMAL_DIVIDE_INTEGER,
        ),
        skill_id: SKILL_ID_COLUMN_DECIMAL_DIVIDE_INTEGER,
        curriculum_path: &CURRICULUM_PATH_COLUMN_DECIMAL_DIVIDE_INTEGER,
        grade: Some(SchoolGrade::Elementary4),
        tags: DECIMAL_DIVISION,
        safety: Safety::NonNegativeOnly,
        presentation: Presentation::COLUMN_DECIMAL_DIVISION,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: DECIMAL_COLUMN,
        layout: COLUMN_DIVISION_12_LAYOUT,
    })
    .with_curriculum_unit(CURRICULUM_UNIT_GRADE4_DECIMAL);

pub const COLUMN_DECIMAL_MULTIPLICATION_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_COLUMN_DECIMAL_MULTIPLICATION),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_COLUMN_DECIMAL_MULTIPLICATION,
        ),
        skill_id: SKILL_ID_COLUMN_DECIMAL_MULTIPLICATION,
        curriculum_path: &CURRICULUM_PATH_COLUMN_DECIMAL_MULTIPLICATION,
        grade: Some(SchoolGrade::Elementary5),
        tags: DECIMAL_MULTIPLICATION,
        safety: Safety::NonNegativeOnly,
        presentation: Presentation::COLUMN_DECIMAL_MULTIPLICATION,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: DECIMAL_COLUMN,
        layout: COLUMN_16_LAYOUT,
    })
    .with_curriculum_unit(CURRICULUM_UNIT_GRADE5_DECIMAL);

pub const COLUMN_DECIMAL_DIVISION_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_COLUMN_DECIMAL_DIVISION),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_COLUMN_DECIMAL_DIVISION,
        ),
        skill_id: SKILL_ID_COLUMN_DECIMAL_DIVISION,
        curriculum_path: &CURRICULUM_PATH_COLUMN_DECIMAL_DIVISION,
        grade: Some(SchoolGrade::Elementary5),
        tags: DECIMAL_DIVISION,
        safety: Safety::NonNegativeOnly,
        presentation: Presentation::COLUMN_DECIMAL_DIVISION,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: DECIMAL_COLUMN,
        layout: COLUMN_DIVISION_12_LAYOUT,
    })
    .with_curriculum_unit(CURRICULUM_UNIT_GRADE5_DECIMAL);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Mode {
    AddTwoDigit,
    SubtractTwoDigit,
    AddThreeFourDigit,
    SubtractThreeFourDigit,
    MultiplyOneDigit,
    MultiplyTwoDigit,
    DivideTwoDigitByOneDigit,
    DivideThreeDigitByOneDigit,
    DivideTwoDigitDivisor,
    DecimalAddSubtract,
    DecimalMultiplyInteger,
    DecimalDivideInteger,
    DecimalMultiplication,
    DecimalDivision,
}

#[derive(Debug)]
pub(crate) struct Generator {
    registration: &'static ThemeRegistration,
    mode: Mode,
}

impl ProblemGenerator for Generator {
    fn registration(&self) -> &'static ThemeRegistration {
        self.registration
    }

    fn sampling_strategy(&self) -> Result<SamplingStrategy<'_>, crate::error::SamplingError> {
        if self.mode == Mode::DecimalAddSubtract {
            SamplingStrategy::layered(
                self,
                SelectionDedup::AllowDuplicates,
                self.registration.layout().problem_count(),
            )
        } else {
            Ok(SamplingStrategy::random(
                self,
                SelectionDedup::AllowDuplicates,
            ))
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
        draw_problem(self.registration, self.mode, rng, ordinal, weights).transpose()
    }
}

impl LayeredCandidateSource for Generator {
    fn layers(&self) -> &'static [SamplingLayerSpec] {
        &DECIMAL_ADD_SUBTRACT_LAYERS
    }

    fn draw_candidate(
        &self,
        rng: &mut DeterministicRng,
        ordinal: u32,
        weights: &OperationWeights,
    ) -> Result<Option<Problem>, GenerationError> {
        draw_problem(self.registration, self.mode, rng, ordinal, weights).transpose()
    }

    fn layer_of(&self, problem: &Problem) -> usize {
        let ProblemPrompt::ColumnArithmetic { operator, .. } = problem.prompt() else {
            unreachable!("column decimal add/sub generator always emits column prompts");
        };
        match operator {
            ArithmeticOperator::Add => 0,
            ArithmeticOperator::Subtract => 1,
            ArithmeticOperator::Multiply | ArithmeticOperator::Divide => {
                unreachable!("column decimal add/sub generator emitted the wrong operator")
            }
        }
    }
}

macro_rules! generator {
    ($name:ident, $registration:ident, $mode:ident) => {
        pub(crate) static $name: Generator = Generator {
            registration: &$registration,
            mode: Mode::$mode,
        };
    };
}

generator!(
    ADD_2DIGIT_GENERATOR,
    COLUMN_ADD_2DIGIT_REGISTRATION,
    AddTwoDigit
);
generator!(
    SUBTRACT_2DIGIT_GENERATOR,
    COLUMN_SUBTRACT_2DIGIT_REGISTRATION,
    SubtractTwoDigit
);
generator!(
    ADD_3_4DIGIT_GENERATOR,
    COLUMN_ADD_3_4DIGIT_REGISTRATION,
    AddThreeFourDigit
);
generator!(
    SUBTRACT_3_4DIGIT_GENERATOR,
    COLUMN_SUBTRACT_3_4DIGIT_REGISTRATION,
    SubtractThreeFourDigit
);
generator!(
    MULTIPLY_1DIGIT_GENERATOR,
    COLUMN_MULTIPLY_1DIGIT_REGISTRATION,
    MultiplyOneDigit
);
generator!(
    MULTIPLY_2DIGIT_GENERATOR,
    COLUMN_MULTIPLY_2DIGIT_REGISTRATION,
    MultiplyTwoDigit
);
generator!(
    DIVIDE_2DIGIT_BY_1DIGIT_GENERATOR,
    COLUMN_DIVIDE_2DIGIT_BY_1DIGIT_REGISTRATION,
    DivideTwoDigitByOneDigit
);
generator!(
    DIVIDE_3DIGIT_BY_1DIGIT_GENERATOR,
    COLUMN_DIVIDE_3DIGIT_BY_1DIGIT_REGISTRATION,
    DivideThreeDigitByOneDigit
);
generator!(
    DIVIDE_2DIGIT_GENERATOR,
    COLUMN_DIVIDE_2DIGIT_REGISTRATION,
    DivideTwoDigitDivisor
);
generator!(
    DECIMAL_ADD_SUBTRACT_GENERATOR,
    COLUMN_DECIMAL_ADD_SUBTRACT_REGISTRATION,
    DecimalAddSubtract
);
generator!(
    DECIMAL_MULTIPLY_INTEGER_GENERATOR,
    COLUMN_DECIMAL_MULTIPLY_INTEGER_REGISTRATION,
    DecimalMultiplyInteger
);
generator!(
    DECIMAL_DIVIDE_INTEGER_GENERATOR,
    COLUMN_DECIMAL_DIVIDE_INTEGER_REGISTRATION,
    DecimalDivideInteger
);
generator!(
    DECIMAL_MULTIPLICATION_GENERATOR,
    COLUMN_DECIMAL_MULTIPLICATION_REGISTRATION,
    DecimalMultiplication
);
generator!(
    DECIMAL_DIVISION_GENERATOR,
    COLUMN_DECIMAL_DIVISION_REGISTRATION,
    DecimalDivision
);

fn draw_integer_with_digits(rng: &mut DeterministicRng, digits: u32) -> Option<i64> {
    if digits == 0 {
        return None;
    }
    let lower = if digits == 1 {
        1
    } else {
        10_i64.checked_pow(digits - 1)?
    };
    let upper = 10_i64.checked_pow(digits)?.checked_sub(1)?;
    let width = upper.checked_sub(lower)?.checked_add(1)?;
    let offset = i64::try_from(rng.next_bounded(u64::try_from(width).ok()?)).ok()?;
    lower.checked_add(offset)
}

fn draw_three_or_four_digit_integer(rng: &mut DeterministicRng) -> Option<i64> {
    let digits = if rng.next_bounded(2) == 0 { 3 } else { 4 };
    draw_integer_with_digits(rng, digits)
}

fn draw_column_remainder(rng: &mut DeterministicRng, divisor: i64) -> Option<i64> {
    let positive_remainders = divisor.checked_sub(1)?;
    if positive_remainders <= 0 {
        return None;
    }
    if rng.next_bounded(2) == 0 {
        Some(0)
    } else {
        let offset =
            i64::try_from(rng.next_bounded(u64::try_from(positive_remainders).ok()?)).ok()?;
        offset.checked_add(1)
    }
}

// Current column-arithmetic candidate rules. Pre-release history lives in Git;
// production registers only the current generator for each theme.
fn draw_problem(
    registration: &ThemeRegistration,
    mode: Mode,
    rng: &mut DeterministicRng,
    id: u32,
    _weights: &OperationWeights,
) -> Option<Result<Problem, GenerationError>> {
    let (operator, left, right, answer, operation_plan, answer_schema) = match mode {
        Mode::AddTwoDigit => {
            let left_value = draw_integer_with_digits(rng, 2)?;
            let right_value = draw_integer_with_digits(rng, 2)?;
            let answer = AnswerNode::Integer(left_value.checked_add(right_value)?);
            let left = integer_expression(left_value);
            let right = integer_expression(right_value);
            let expression =
                binary_expression(ArithmeticOperator::Add, left.clone(), right.clone());
            let plan = arithmetic_expression_plan(&expression, &answer)?;
            (
                ArithmeticOperator::Add,
                left,
                right,
                answer,
                plan,
                AnswerSchema::Integer { min: 20, max: 198 },
            )
        }
        Mode::SubtractTwoDigit => {
            let first = draw_integer_with_digits(rng, 2)?;
            let second = draw_integer_with_digits(rng, 2)?;
            let (left_value, right_value) = if first >= second {
                (first, second)
            } else {
                (second, first)
            };
            let answer = AnswerNode::Integer(left_value - right_value);
            let left = integer_expression(left_value);
            let right = integer_expression(right_value);
            let expression =
                binary_expression(ArithmeticOperator::Subtract, left.clone(), right.clone());
            let plan = arithmetic_expression_plan(&expression, &answer)?;
            (
                ArithmeticOperator::Subtract,
                left,
                right,
                answer,
                plan,
                AnswerSchema::Integer { min: 0, max: 89 },
            )
        }
        Mode::AddThreeFourDigit => {
            let left_value = draw_three_or_four_digit_integer(rng)?;
            let right_value = draw_three_or_four_digit_integer(rng)?;
            let answer = AnswerNode::Integer(left_value.checked_add(right_value)?);
            let left = integer_expression(left_value);
            let right = integer_expression(right_value);
            let expression =
                binary_expression(ArithmeticOperator::Add, left.clone(), right.clone());
            let plan = arithmetic_expression_plan(&expression, &answer)?;
            (
                ArithmeticOperator::Add,
                left,
                right,
                answer,
                plan,
                AnswerSchema::Integer {
                    min: 200,
                    max: 19_998,
                },
            )
        }
        Mode::SubtractThreeFourDigit => {
            let first = draw_three_or_four_digit_integer(rng)?;
            let second = draw_three_or_four_digit_integer(rng)?;
            let (left_value, right_value) = if first >= second {
                (first, second)
            } else {
                (second, first)
            };
            let answer = AnswerNode::Integer(left_value - right_value);
            let left = integer_expression(left_value);
            let right = integer_expression(right_value);
            let expression =
                binary_expression(ArithmeticOperator::Subtract, left.clone(), right.clone());
            let plan = arithmetic_expression_plan(&expression, &answer)?;
            (
                ArithmeticOperator::Subtract,
                left,
                right,
                answer,
                plan,
                AnswerSchema::Integer { min: 0, max: 9_899 },
            )
        }
        Mode::MultiplyOneDigit => {
            let multiplicand = if rng.next_bounded(2) == 0 {
                draw_integer_with_digits(rng, 2)?
            } else {
                draw_integer_with_digits(rng, 3)?
            };
            let multiplier = 2 + rng.next_bounded(8) as i64;
            let answer = AnswerNode::Integer(multiplicand.checked_mul(multiplier)?);
            let left = integer_expression(multiplicand);
            let right = integer_expression(multiplier);
            let expression =
                binary_expression(ArithmeticOperator::Multiply, left.clone(), right.clone());
            let plan = arithmetic_expression_plan(&expression, &answer)?;
            (
                ArithmeticOperator::Multiply,
                left,
                right,
                answer,
                plan,
                AnswerSchema::Integer {
                    min: 20,
                    max: 8_991,
                },
            )
        }
        Mode::MultiplyTwoDigit => {
            let multiplicand = if rng.next_bounded(2) == 0 {
                draw_integer_with_digits(rng, 2)?
            } else {
                draw_integer_with_digits(rng, 3)?
            };
            let multiplier = draw_integer_with_digits(rng, 2)?;
            let answer = AnswerNode::Integer(multiplicand.checked_mul(multiplier)?);
            let left = integer_expression(multiplicand);
            let right = integer_expression(multiplier);
            let expression =
                binary_expression(ArithmeticOperator::Multiply, left.clone(), right.clone());
            let plan = arithmetic_expression_plan(&expression, &answer)?;
            (
                ArithmeticOperator::Multiply,
                left,
                right,
                answer,
                plan,
                AnswerSchema::Integer {
                    min: 100,
                    max: 98_901,
                },
            )
        }
        Mode::DivideTwoDigitByOneDigit | Mode::DivideThreeDigitByOneDigit => {
            let dividend_digits = if mode == Mode::DivideTwoDigitByOneDigit {
                2
            } else {
                3
            };
            let dividend = draw_integer_with_digits(rng, dividend_digits)?;
            let divisor = 2 + rng.next_bounded(8) as i64;
            let quotient = dividend / divisor;
            let remainder = dividend % divisor;
            let answer = AnswerNode::Tuple(vec![
                AnswerNode::Integer(quotient),
                AnswerNode::Integer(remainder),
            ]);
            let plan = integer_division_with_remainder_plan(dividend, divisor, &answer)?;
            (
                ArithmeticOperator::Divide,
                integer_expression(dividend),
                integer_expression(divisor),
                answer,
                plan,
                AnswerSchema::OrderedPair,
            )
        }
        Mode::DivideTwoDigitDivisor => {
            let divisor = draw_integer_with_digits(rng, 2)?;
            let quotient = 2 + rng.next_bounded(98) as i64;
            let remainder = draw_column_remainder(rng, divisor)?;
            let dividend = divisor.checked_mul(quotient)?.checked_add(remainder)?;
            let answer = AnswerNode::Tuple(vec![
                AnswerNode::Integer(quotient),
                AnswerNode::Integer(remainder),
            ]);
            let plan = integer_division_with_remainder_plan(dividend, divisor, &answer)?;
            (
                ArithmeticOperator::Divide,
                integer_expression(dividend),
                integer_expression(divisor),
                answer,
                plan,
                AnswerSchema::OrderedPair,
            )
        }
        Mode::DecimalAddSubtract => {
            let (mut left_coefficient, mut left_scale) = draw_decimal_operand(rng, 3, 3)?;
            let (mut right_coefficient, mut right_scale) = draw_decimal_operand(rng, 3, 3)?;
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
            let result = if operator == ArithmeticOperator::Add {
                left_value.checked_add(right_value)?
            } else {
                left_value.subtract(right_value)?
            };
            let answer = rational_to_exact_decimal_answer(result, 3)?;
            let left = exact_decimal_expression(left_coefficient, left_scale);
            let right = exact_decimal_expression(right_coefficient, right_scale);
            let expression = binary_expression(operator, left.clone(), right.clone());
            let plan = arithmetic_expression_plan(&expression, &answer)?;
            (
                operator,
                left,
                right,
                answer,
                plan,
                AnswerSchema::Decimal { max_scale: 3 },
            )
        }
        Mode::DecimalMultiplyInteger => {
            let (coefficient, scale) = draw_decimal_operand(rng, 3, 2)?;
            let integer = 2 + rng.next_bounded(8) as i64;
            let left_value = exact_decimal_rational(coefficient, scale)?;
            let result = left_value.multiply(RationalCoefficient::new(integer, 1)?)?;
            let answer = rational_to_exact_decimal_answer(result, 3)?;
            let left = exact_decimal_expression(coefficient, scale);
            let right = integer_expression(integer);
            let expression =
                binary_expression(ArithmeticOperator::Multiply, left.clone(), right.clone());
            let plan = arithmetic_expression_plan(&expression, &answer)?;
            (
                ArithmeticOperator::Multiply,
                left,
                right,
                answer,
                plan,
                AnswerSchema::Decimal { max_scale: 3 },
            )
        }
        Mode::DecimalDivideInteger => {
            let (quotient_coefficient, quotient_scale) = draw_decimal_operand(rng, 3, 2)?;
            let quotient = exact_decimal_rational(quotient_coefficient, quotient_scale)?;
            let divisor = 2 + rng.next_bounded(8) as i64;
            let dividend = quotient.multiply(RationalCoefficient::new(divisor, 1)?)?;
            let left = rational_to_arithmetic_expression(dividend, 3)?;
            if arithmetic_leaf_significant_digits(&left)? > 4 {
                return None;
            }
            let right = integer_expression(divisor);
            let answer = rational_to_exact_decimal_answer(quotient, 2)?;
            let expression =
                binary_expression(ArithmeticOperator::Divide, left.clone(), right.clone());
            let plan = arithmetic_expression_plan(&expression, &answer)?;
            (
                ArithmeticOperator::Divide,
                left,
                right,
                answer,
                plan,
                AnswerSchema::Decimal { max_scale: 2 },
            )
        }
        Mode::DecimalMultiplication => {
            // A column-multiplication exercise should not collapse to a one-digit
            // multiplication fact after removing the decimal points. Choose from
            // the three allowed significant-digit shapes directly so at least one
            // operand always has two significant digits.
            let (left_digits, right_digits) = match rng.next_bounded(3) {
                0 => (1, 2),
                1 => (2, 1),
                _ => (2, 2),
            };
            let (left_coefficient, left_scale) =
                draw_decimal_operand_with_significant_digits(rng, left_digits, 2)?;
            let (right_coefficient, right_scale) =
                draw_decimal_operand_with_significant_digits(rng, right_digits, 2)?;
            let left_value = exact_decimal_rational(left_coefficient, left_scale)?;
            let right_value = exact_decimal_rational(right_coefficient, right_scale)?;
            let result = left_value.multiply(right_value)?;
            let answer = rational_to_exact_decimal_answer(result, 4)?;
            let left = exact_decimal_expression(left_coefficient, left_scale);
            let right = exact_decimal_expression(right_coefficient, right_scale);
            let expression =
                binary_expression(ArithmeticOperator::Multiply, left.clone(), right.clone());
            let plan = arithmetic_expression_plan(&expression, &answer)?;
            (
                ArithmeticOperator::Multiply,
                left,
                right,
                answer,
                plan,
                AnswerSchema::Decimal { max_scale: 4 },
            )
        }
        Mode::DecimalDivision => {
            // Generate directly inside the printable 4-column long-division
            // domain instead of drawing broad decimals and rejecting most of
            // them. Tenths divisors pair with tenths quotients; hundredths
            // divisors pair with a one-digit integer quotient. Both exercise
            // decimal-point shifting while keeping the original vertical setup
            // at six page-grid cells or fewer without shrinking the font.
            let divisor_scale = 1 + rng.next_bounded(2) as u32;
            let divisor_coefficient = draw_decimal_coefficient(rng, 2)?;
            let divisor = exact_decimal_rational(divisor_coefficient, divisor_scale)?;
            let (quotient, answer) = if divisor_scale == 1 {
                let quotient_coefficient = draw_decimal_coefficient(rng, 2)?;
                let quotient_scale = 1;
                let quotient = exact_decimal_rational(quotient_coefficient, quotient_scale)?;
                let answer = rational_to_exact_decimal_answer(quotient, 1)?;
                (quotient, answer)
            } else {
                let quotient_integer = 2 + rng.next_bounded(8) as i64;
                let quotient = RationalCoefficient::new(quotient_integer, 1)?;
                (quotient, AnswerNode::Integer(quotient_integer))
            };
            let dividend = quotient.multiply(divisor)?;
            let left = rational_to_arithmetic_expression(dividend, 4)?;
            let right = exact_decimal_expression(divisor_coefficient, divisor_scale);
            debug_assert!(
                arithmetic_leaf_column_grid_cells(&left)?
                    + arithmetic_leaf_column_grid_cells(&right)?
                    <= 6
            );
            let expression =
                binary_expression(ArithmeticOperator::Divide, left.clone(), right.clone());
            let plan = arithmetic_expression_plan(&expression, &answer)?;
            (
                ArithmeticOperator::Divide,
                left,
                right,
                answer,
                plan,
                AnswerSchema::Decimal { max_scale: 2 },
            )
        }
    };

    Some(
        Problem::generated(
            registration,
            id,
            ProblemPrompt::ColumnArithmetic {
                operator,
                left,
                right,
            },
            answer_schema,
            answer,
            EffortModel::operations(operation_plan),
        )
        .map_err(GenerationError::from),
    )
}

/// Current generators owned by this theme family.
pub(crate) static GENERATORS: [GeneratorEntry; 14] = [
    GeneratorEntry::current(&ADD_2DIGIT_GENERATOR),
    GeneratorEntry::current(&SUBTRACT_2DIGIT_GENERATOR),
    GeneratorEntry::current(&ADD_3_4DIGIT_GENERATOR),
    GeneratorEntry::current(&SUBTRACT_3_4DIGIT_GENERATOR),
    GeneratorEntry::current(&MULTIPLY_1DIGIT_GENERATOR),
    GeneratorEntry::current(&MULTIPLY_2DIGIT_GENERATOR),
    GeneratorEntry::current(&DIVIDE_2DIGIT_BY_1DIGIT_GENERATOR),
    GeneratorEntry::current(&DIVIDE_3DIGIT_BY_1DIGIT_GENERATOR),
    GeneratorEntry::current(&DIVIDE_2DIGIT_GENERATOR),
    GeneratorEntry::current(&DECIMAL_ADD_SUBTRACT_GENERATOR),
    GeneratorEntry::current(&DECIMAL_MULTIPLY_INTEGER_GENERATOR),
    GeneratorEntry::current(&DECIMAL_DIVIDE_INTEGER_GENERATOR),
    GeneratorEntry::current(&DECIMAL_MULTIPLICATION_GENERATOR),
    GeneratorEntry::current(&DECIMAL_DIVISION_GENERATOR),
];

#[cfg(test)]
mod curriculum_tests {
    use super::*;
    use crate::generator::generate_worksheet_request;
    use crate::model::{ArithmeticExpression, GenerateWorksheetRequest};
    use crate::schema::SCHEMA_VERSION;

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

    fn column_leaf_value(expression: &ArithmeticExpression) -> RationalCoefficient {
        match expression {
            ArithmeticExpression::Integer { value } => RationalCoefficient::new(*value, 1).unwrap(),
            ArithmeticExpression::ExactDecimal { coefficient, scale } => {
                crate::generator_support::exact_decimal_rational(*coefficient, *scale).unwrap()
            }
            other => panic!("column arithmetic must use a scalar display operand: {other:?}"),
        }
    }

    #[test]
    fn local_draw_helpers_reject_invalid_bounds_without_rng_panics() {
        let mut rng = DeterministicRng::from_seed("ColumnHelperBounds");
        assert_eq!(draw_integer_with_digits(&mut rng, 0), None);
        assert_eq!(draw_integer_with_digits(&mut rng, u32::MAX), None);
        assert_eq!(draw_column_remainder(&mut rng, i64::MIN), None);
        assert_eq!(draw_column_remainder(&mut rng, 0), None);
        assert_eq!(draw_column_remainder(&mut rng, 1), None);

        let two_digit = draw_integer_with_digits(&mut rng, 2).unwrap();
        assert!((10..=99).contains(&two_digit));
        let remainder = draw_column_remainder(&mut rng, 7).unwrap();
        assert!((0..7).contains(&remainder));
    }

    #[test]
    fn column_arithmetic_themes_follow_curriculum_domains_and_print_layouts() {
        use crate::themes::column_arithmetic::*;
        const IDS: [u32; 14] = [
            THEME_ID_COLUMN_ADD_2DIGIT,
            THEME_ID_COLUMN_SUBTRACT_2DIGIT,
            THEME_ID_COLUMN_ADD_3_4DIGIT,
            THEME_ID_COLUMN_SUBTRACT_3_4DIGIT,
            THEME_ID_COLUMN_MULTIPLY_1DIGIT,
            THEME_ID_COLUMN_MULTIPLY_2DIGIT,
            THEME_ID_COLUMN_DIVIDE_2DIGIT_BY_1DIGIT,
            THEME_ID_COLUMN_DIVIDE_3DIGIT_BY_1DIGIT,
            THEME_ID_COLUMN_DIVIDE_2DIGIT,
            THEME_ID_COLUMN_DECIMAL_ADD_SUBTRACT,
            THEME_ID_COLUMN_DECIMAL_MULTIPLY_INTEGER,
            THEME_ID_COLUMN_DECIMAL_DIVIDE_INTEGER,
            THEME_ID_COLUMN_DECIMAL_MULTIPLICATION,
            THEME_ID_COLUMN_DECIMAL_DIVISION,
        ];
        let seeds = ["CoA1", "CoB2", "CoC3", "CoD4", "CoE5", "CoF6"];

        for theme_id in IDS {
            let registration = crate::registry::active_registration(theme_id)
                .unwrap()
                .unwrap();
            let is_division = matches!(
                theme_id,
                THEME_ID_COLUMN_DIVIDE_2DIGIT_BY_1DIGIT
                    | THEME_ID_COLUMN_DIVIDE_3DIGIT_BY_1DIGIT
                    | THEME_ID_COLUMN_DIVIDE_2DIGIT
                    | THEME_ID_COLUMN_DECIMAL_DIVIDE_INTEGER
                    | THEME_ID_COLUMN_DECIMAL_DIVISION
            );
            let (expected_count, expected_columns, expected_rows) =
                if is_division { (12, 4, 3) } else { (16, 4, 4) };
            assert_eq!(registration.layout().problem_count(), expected_count);
            assert_eq!(
                (
                    registration.layout().columns(),
                    registration.layout().rows()
                ),
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
                assert_eq!(worksheet.layout().problem_count, expected_count as u32);
                assert_eq!(
                    (worksheet.layout().columns, worksheet.layout().rows),
                    (expected_columns as u32, expected_rows as u32)
                );
                assert_eq!(worksheet.problems().len(), expected_count);

                for problem in worksheet.into_problems() {
                    let ProblemPrompt::ColumnArithmetic {
                        operator,
                        left,
                        right,
                    } = problem.prompt()
                    else {
                        panic!("column theme {theme_id} returned a non-column prompt");
                    };
                    let left_value = column_leaf_value(left);
                    let right_value = column_leaf_value(right);
                    assert!(left_value.numerator() >= 0 && right_value.numerator() >= 0);

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
                            assert!((10..=99).contains(&left_value.numerator()));
                            assert!((10..=99).contains(&right_value.numerator()));
                            assert_eq!(left_value.denominator(), 1);
                            assert_eq!(right_value.denominator(), 1);
                        }
                        THEME_ID_COLUMN_SUBTRACT_2DIGIT => {
                            assert_eq!(*operator, ArithmeticOperator::Subtract);
                            assert!((10..=99).contains(&left_value.numerator()));
                            assert!((10..=99).contains(&right_value.numerator()));
                            assert!(left_value.numerator() >= right_value.numerator());
                        }
                        THEME_ID_COLUMN_ADD_3_4DIGIT | THEME_ID_COLUMN_SUBTRACT_3_4DIGIT => {
                            assert!(matches!(
                                operator,
                                ArithmeticOperator::Add | ArithmeticOperator::Subtract
                            ));
                            assert!((100..=9_999).contains(&left_value.numerator()));
                            assert!((100..=9_999).contains(&right_value.numerator()));
                            if *operator == ArithmeticOperator::Subtract {
                                assert!(left_value.numerator() >= right_value.numerator());
                            }
                        }
                        THEME_ID_COLUMN_MULTIPLY_1DIGIT => {
                            assert_eq!(*operator, ArithmeticOperator::Multiply);
                            assert!((10..=999).contains(&left_value.numerator()));
                            assert!((2..=9).contains(&right_value.numerator()));
                        }
                        THEME_ID_COLUMN_MULTIPLY_2DIGIT => {
                            assert_eq!(*operator, ArithmeticOperator::Multiply);
                            assert!((10..=999).contains(&left_value.numerator()));
                            assert!((10..=99).contains(&right_value.numerator()));
                        }
                        THEME_ID_COLUMN_DIVIDE_2DIGIT_BY_1DIGIT
                        | THEME_ID_COLUMN_DIVIDE_3DIGIT_BY_1DIGIT
                        | THEME_ID_COLUMN_DIVIDE_2DIGIT => {
                            assert_eq!(*operator, ArithmeticOperator::Divide);
                            assert_eq!(left_value.denominator(), 1);
                            assert_eq!(right_value.denominator(), 1);
                            let dividend = left_value.numerator();
                            let divisor = right_value.numerator();
                            match theme_id {
                                THEME_ID_COLUMN_DIVIDE_2DIGIT_BY_1DIGIT => {
                                    assert!((10..=99).contains(&dividend));
                                    assert!((2..=9).contains(&divisor));
                                }
                                THEME_ID_COLUMN_DIVIDE_3DIGIT_BY_1DIGIT => {
                                    assert!((100..=999).contains(&dividend));
                                    assert!((2..=9).contains(&divisor));
                                }
                                THEME_ID_COLUMN_DIVIDE_2DIGIT => {
                                    assert!((10..=99).contains(&divisor));
                                }
                                _ => unreachable!(),
                            }
                            let (quotient, remainder) =
                                quotient_remainder(problem.canonical_answer());
                            assert_eq!(quotient, dividend / divisor);
                            assert_eq!(remainder, dividend % divisor);
                            assert!((0..divisor).contains(&remainder));
                            assert_eq!(dividend, divisor * quotient + remainder);
                            assert_eq!(problem.answer_schema(), &AnswerSchema::OrderedPair);
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
                            assert!((2..=9).contains(&right_value.numerator()));
                            assert_eq!(right_value.denominator(), 1);
                        }
                        THEME_ID_COLUMN_DECIMAL_DIVIDE_INTEGER => {
                            assert_eq!(*operator, ArithmeticOperator::Divide);
                            assert!((2..=9).contains(&right_value.numerator()));
                            assert_eq!(right_value.denominator(), 1);
                        }
                        THEME_ID_COLUMN_DECIMAL_MULTIPLICATION => {
                            assert_eq!(*operator, ArithmeticOperator::Multiply);
                            assert!(matches!(left, ArithmeticExpression::ExactDecimal { .. }));
                            assert!(matches!(right, ArithmeticExpression::ExactDecimal { .. }));
                            let left_digits = arithmetic_leaf_significant_digits(left).unwrap();
                            let right_digits = arithmetic_leaf_significant_digits(right).unwrap();
                            assert!((1..=2).contains(&left_digits));
                            assert!((1..=2).contains(&right_digits));
                            assert!(
                                left_digits == 2 || right_digits == 2,
                                "decimal column multiplication must not reduce to one significant digit × one significant digit"
                            );
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
                        crate::normalize::normalize_answer(problem.canonical_answer()),
                        "theme {theme_id} canonical answer disagrees with its displayed operands"
                    );
                }
            }
        }
    }
}
