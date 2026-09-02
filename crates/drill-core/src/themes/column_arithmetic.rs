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
pub const THEME_ID_COLUMN_ADD_2DIGIT_NO_CARRY: u32 = 59;
pub const THEME_ID_COLUMN_ADD_2DIGIT_WITH_CARRY: u32 = 60;
pub const THEME_ID_COLUMN_SUBTRACT_2DIGIT_NO_BORROW: u32 = 61;
pub const THEME_ID_COLUMN_SUBTRACT_2DIGIT_WITH_BORROW: u32 = 62;
pub const THEME_ID_COLUMN_DECIMAL_ADDITION: u32 = 63;
pub const THEME_ID_COLUMN_DECIMAL_SUBTRACTION: u32 = 64;
pub const THEME_ID_COLUMN_DECIMAL_DIVISION_REMAINDER: u32 = 65;
pub const THEME_ID_COLUMN_DECIMAL_DIVISION_ROUNDED: u32 = 66;
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
pub const GENERATOR_REVISION_COLUMN_ADD_2DIGIT_NO_CARRY: u32 = 1;
pub const GENERATOR_REVISION_COLUMN_ADD_2DIGIT_WITH_CARRY: u32 = 1;
pub const GENERATOR_REVISION_COLUMN_SUBTRACT_2DIGIT_NO_BORROW: u32 = 1;
pub const GENERATOR_REVISION_COLUMN_SUBTRACT_2DIGIT_WITH_BORROW: u32 = 1;
pub const GENERATOR_REVISION_COLUMN_DECIMAL_ADDITION: u32 = 1;
pub const GENERATOR_REVISION_COLUMN_DECIMAL_SUBTRACTION: u32 = 1;
pub const GENERATOR_REVISION_COLUMN_DECIMAL_DIVISION_REMAINDER: u32 = 1;
pub const GENERATOR_REVISION_COLUMN_DECIMAL_DIVISION_ROUNDED: u32 = 1;
pub const SKILL_ID_COLUMN_ADD_2DIGIT: &str = "jp.grade2.column.addition.two_digit.summary";
pub const SKILL_ID_COLUMN_SUBTRACT_2DIGIT: &str = "jp.grade2.column.subtraction.two_digit.summary";
pub const SKILL_ID_COLUMN_ADD_2DIGIT_NO_CARRY: &str =
    "jp.grade2.column.addition.two_digit.no_carry";
pub const SKILL_ID_COLUMN_ADD_2DIGIT_WITH_CARRY: &str =
    "jp.grade2.column.addition.two_digit.with_carry";
pub const SKILL_ID_COLUMN_SUBTRACT_2DIGIT_NO_BORROW: &str =
    "jp.grade2.column.subtraction.two_digit.no_borrow";
pub const SKILL_ID_COLUMN_SUBTRACT_2DIGIT_WITH_BORROW: &str =
    "jp.grade2.column.subtraction.two_digit.with_borrow";
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
pub const SKILL_ID_COLUMN_DECIMAL_ADD_SUBTRACT: &str =
    "jp.grade4.column.decimal.add_subtract.summary";
pub const SKILL_ID_COLUMN_DECIMAL_ADDITION: &str = "jp.grade4.column.decimal.addition";
pub const SKILL_ID_COLUMN_DECIMAL_SUBTRACTION: &str = "jp.grade4.column.decimal.subtraction";
pub const SKILL_ID_COLUMN_DECIMAL_MULTIPLY_INTEGER: &str =
    "jp.grade4.column.decimal.multiply_integer";
pub const SKILL_ID_COLUMN_DECIMAL_DIVIDE_INTEGER: &str = "jp.grade4.column.decimal.divide_integer";
pub const SKILL_ID_COLUMN_DECIMAL_MULTIPLICATION: &str = "jp.grade5.column.decimal.multiplication";
pub const SKILL_ID_COLUMN_DECIMAL_DIVISION: &str = "jp.grade5.column.decimal.division";
pub const SKILL_ID_COLUMN_DECIMAL_DIVISION_REMAINDER: &str =
    "jp.grade5.column.decimal.division.remainder";
pub const SKILL_ID_COLUMN_DECIMAL_DIVISION_ROUNDED: &str =
    "jp.grade5.column.decimal.division.rounded";
pub const CURRICULUM_PATH_COLUMN_ADD_2DIGIT: [&str; 4] = [
    "root",
    "小学2年生",
    "加法，減法",
    "二桁の足し算の筆算（まとめ）",
];
pub const CURRICULUM_PATH_COLUMN_SUBTRACT_2DIGIT: [&str; 4] = [
    "root",
    "小学2年生",
    "加法，減法",
    "二桁の引き算の筆算（まとめ）",
];
pub const CURRICULUM_PATH_COLUMN_ADD_2DIGIT_NO_CARRY: [&str; 4] =
    ["root", "小学2年生", "加法，減法", "足し算・繰り上がりなし"];
pub const CURRICULUM_PATH_COLUMN_ADD_2DIGIT_WITH_CARRY: [&str; 4] =
    ["root", "小学2年生", "加法，減法", "足し算・繰り上がりあり"];
pub const CURRICULUM_PATH_COLUMN_SUBTRACT_2DIGIT_NO_BORROW: [&str; 4] =
    ["root", "小学2年生", "加法，減法", "引き算・繰り下がりなし"];
pub const CURRICULUM_PATH_COLUMN_SUBTRACT_2DIGIT_WITH_BORROW: [&str; 4] =
    ["root", "小学2年生", "加法，減法", "引き算・繰り下がりあり"];
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
    "小数の足し算と引き算（まとめ）",
];
pub const CURRICULUM_PATH_COLUMN_DECIMAL_ADDITION: [&str; 4] = [
    "root",
    "小学4年生",
    "小数の仕組みとその計算",
    "小数の足し算の筆算",
];
pub const CURRICULUM_PATH_COLUMN_DECIMAL_SUBTRACTION: [&str; 4] = [
    "root",
    "小学4年生",
    "小数の仕組みとその計算",
    "小数の引き算の筆算",
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
pub const CURRICULUM_PATH_COLUMN_DECIMAL_DIVISION_REMAINDER: [&str; 4] = [
    "root",
    "小学5年生",
    "小数の乗法，除法",
    "余りを答える小数の割り算の筆算",
];
pub const CURRICULUM_PATH_COLUMN_DECIMAL_DIVISION_ROUNDED: [&str; 4] = [
    "root",
    "小学5年生",
    "小数の乗法，除法",
    "商を四捨五入する小数の割り算の筆算",
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
const DECIMAL_ADDITION: &[ThemeTag] = &[ThemeTag::Decimals, ThemeTag::Addition];
const DECIMAL_SUBTRACTION: &[ThemeTag] = &[ThemeTag::Decimals, ThemeTag::Subtraction];
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
const DECIMAL_DIVISION_REMAINDER_COLUMN: AnswerContract =
    AnswerContract::ColumnDecimalDivisionRemainder;
const DECIMAL_DIVISION_ROUNDED_COLUMN: AnswerContract =
    AnswerContract::ColumnDecimalDivisionRounded;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct DecimalDivisionResultPolicy {
    answer_scale: u32,
}

impl DecimalDivisionResultPolicy {
    const fn answer_scale(self) -> u32 {
        self.answer_scale
    }

    fn calculation_scale(self) -> Option<u32> {
        self.answer_scale.checked_add(1)
    }

    fn remainder_max_scale(self, divisor_scale: u32) -> Option<u32> {
        self.calculation_scale()?.checked_add(divisor_scale)
    }
}

const DECIMAL_DIVISION_RESULT_POLICY: DecimalDivisionResultPolicy =
    DecimalDivisionResultPolicy { answer_scale: 1 };

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

pub const COLUMN_ADD_2DIGIT_NO_CARRY_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_COLUMN_ADD_2DIGIT_NO_CARRY),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_COLUMN_ADD_2DIGIT_NO_CARRY,
        ),
        skill_id: SKILL_ID_COLUMN_ADD_2DIGIT_NO_CARRY,
        curriculum_path: &CURRICULUM_PATH_COLUMN_ADD_2DIGIT_NO_CARRY,
        grade: Some(SchoolGrade::Elementary2),
        tags: ADDITION,
        safety: Safety::NonNegativeOnly,
        presentation: Presentation::COLUMN_ARITHMETIC,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: INTEGER_COLUMN,
        layout: COLUMN_16_LAYOUT,
    })
    .with_curriculum_unit(CURRICULUM_UNIT_GRADE2_COLUMN_ADD_SUBTRACT);

pub const COLUMN_ADD_2DIGIT_WITH_CARRY_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_COLUMN_ADD_2DIGIT_WITH_CARRY),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_COLUMN_ADD_2DIGIT_WITH_CARRY,
        ),
        skill_id: SKILL_ID_COLUMN_ADD_2DIGIT_WITH_CARRY,
        curriculum_path: &CURRICULUM_PATH_COLUMN_ADD_2DIGIT_WITH_CARRY,
        grade: Some(SchoolGrade::Elementary2),
        tags: ADDITION,
        safety: Safety::NonNegativeOnly,
        presentation: Presentation::COLUMN_ARITHMETIC,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: INTEGER_COLUMN,
        layout: COLUMN_16_LAYOUT,
    })
    .with_curriculum_unit(CURRICULUM_UNIT_GRADE2_COLUMN_ADD_SUBTRACT);

pub const COLUMN_SUBTRACT_2DIGIT_NO_BORROW_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_COLUMN_SUBTRACT_2DIGIT_NO_BORROW),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_COLUMN_SUBTRACT_2DIGIT_NO_BORROW,
        ),
        skill_id: SKILL_ID_COLUMN_SUBTRACT_2DIGIT_NO_BORROW,
        curriculum_path: &CURRICULUM_PATH_COLUMN_SUBTRACT_2DIGIT_NO_BORROW,
        grade: Some(SchoolGrade::Elementary2),
        tags: SUBTRACTION,
        safety: Safety::NonNegativeOnly,
        presentation: Presentation::COLUMN_ARITHMETIC,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: INTEGER_COLUMN,
        layout: COLUMN_16_LAYOUT,
    })
    .with_curriculum_unit(CURRICULUM_UNIT_GRADE2_COLUMN_ADD_SUBTRACT);

pub const COLUMN_SUBTRACT_2DIGIT_WITH_BORROW_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_COLUMN_SUBTRACT_2DIGIT_WITH_BORROW),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_COLUMN_SUBTRACT_2DIGIT_WITH_BORROW,
        ),
        skill_id: SKILL_ID_COLUMN_SUBTRACT_2DIGIT_WITH_BORROW,
        curriculum_path: &CURRICULUM_PATH_COLUMN_SUBTRACT_2DIGIT_WITH_BORROW,
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

pub const COLUMN_DECIMAL_ADDITION_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_COLUMN_DECIMAL_ADDITION),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_COLUMN_DECIMAL_ADDITION,
        ),
        skill_id: SKILL_ID_COLUMN_DECIMAL_ADDITION,
        curriculum_path: &CURRICULUM_PATH_COLUMN_DECIMAL_ADDITION,
        grade: Some(SchoolGrade::Elementary4),
        tags: DECIMAL_ADDITION,
        safety: Safety::NonNegativeOnly,
        presentation: Presentation::COLUMN_DECIMAL,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: DECIMAL_COLUMN,
        layout: COLUMN_16_LAYOUT,
    })
    .with_curriculum_unit(CURRICULUM_UNIT_GRADE4_DECIMAL);

pub const COLUMN_DECIMAL_SUBTRACTION_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_COLUMN_DECIMAL_SUBTRACTION),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_COLUMN_DECIMAL_SUBTRACTION,
        ),
        skill_id: SKILL_ID_COLUMN_DECIMAL_SUBTRACTION,
        curriculum_path: &CURRICULUM_PATH_COLUMN_DECIMAL_SUBTRACTION,
        grade: Some(SchoolGrade::Elementary4),
        tags: DECIMAL_SUBTRACTION,
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

pub const COLUMN_DECIMAL_DIVISION_REMAINDER_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_COLUMN_DECIMAL_DIVISION_REMAINDER),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_COLUMN_DECIMAL_DIVISION_REMAINDER,
        ),
        skill_id: SKILL_ID_COLUMN_DECIMAL_DIVISION_REMAINDER,
        curriculum_path: &CURRICULUM_PATH_COLUMN_DECIMAL_DIVISION_REMAINDER,
        grade: Some(SchoolGrade::Elementary5),
        tags: DECIMAL_DIVISION,
        safety: Safety::NonNegativeOnly,
        presentation: Presentation::COLUMN_DECIMAL_DIVISION_REMAINDER,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: DECIMAL_DIVISION_REMAINDER_COLUMN,
        layout: COLUMN_DIVISION_12_LAYOUT,
    })
    .with_curriculum_unit(CURRICULUM_UNIT_GRADE5_DECIMAL)
    .with_answer_decimal_scale(DECIMAL_DIVISION_RESULT_POLICY.answer_scale());

pub const COLUMN_DECIMAL_DIVISION_ROUNDED_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_COLUMN_DECIMAL_DIVISION_ROUNDED),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_COLUMN_DECIMAL_DIVISION_ROUNDED,
        ),
        skill_id: SKILL_ID_COLUMN_DECIMAL_DIVISION_ROUNDED,
        curriculum_path: &CURRICULUM_PATH_COLUMN_DECIMAL_DIVISION_ROUNDED,
        grade: Some(SchoolGrade::Elementary5),
        tags: DECIMAL_DIVISION,
        safety: Safety::NonNegativeOnly,
        presentation: Presentation::COLUMN_DECIMAL_DIVISION,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: DECIMAL_DIVISION_ROUNDED_COLUMN,
        layout: COLUMN_DIVISION_12_LAYOUT,
    })
    .with_curriculum_unit(CURRICULUM_UNIT_GRADE5_DECIMAL)
    .with_answer_decimal_scale(DECIMAL_DIVISION_RESULT_POLICY.answer_scale());

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
    DecimalAddition,
    DecimalSubtraction,
    DecimalMultiplyInteger,
    DecimalDivideInteger,
    DecimalMultiplication,
    DecimalDivision,
    DecimalDivisionRemainder,
    DecimalDivisionRounded,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RegroupingPolicy {
    Any,
    None,
    Required,
}

impl RegroupingPolicy {
    fn accepts(self, requires_regrouping: bool) -> bool {
        match self {
            Self::Any => true,
            Self::None => !requires_regrouping,
            Self::Required => requires_regrouping,
        }
    }
}

#[derive(Debug)]
pub(crate) struct Generator {
    registration: &'static ThemeRegistration,
    mode: Mode,
    regrouping: RegroupingPolicy,
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
        draw_problem(
            self.registration,
            self.mode,
            self.regrouping,
            rng,
            ordinal,
            weights,
        )
        .transpose()
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
        draw_problem(
            self.registration,
            self.mode,
            self.regrouping,
            rng,
            ordinal,
            weights,
        )
        .transpose()
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
    ($name:ident, $registration:ident, $mode:ident, $regrouping:ident) => {
        pub(crate) static $name: Generator = Generator {
            registration: &$registration,
            mode: Mode::$mode,
            regrouping: RegroupingPolicy::$regrouping,
        };
    };
    ($name:ident, $registration:ident, $mode:ident) => {
        generator!($name, $registration, $mode, Any);
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
    ADD_2DIGIT_NO_CARRY_GENERATOR,
    COLUMN_ADD_2DIGIT_NO_CARRY_REGISTRATION,
    AddTwoDigit,
    None
);
generator!(
    ADD_2DIGIT_WITH_CARRY_GENERATOR,
    COLUMN_ADD_2DIGIT_WITH_CARRY_REGISTRATION,
    AddTwoDigit,
    Required
);
generator!(
    SUBTRACT_2DIGIT_NO_BORROW_GENERATOR,
    COLUMN_SUBTRACT_2DIGIT_NO_BORROW_REGISTRATION,
    SubtractTwoDigit,
    None
);
generator!(
    SUBTRACT_2DIGIT_WITH_BORROW_GENERATOR,
    COLUMN_SUBTRACT_2DIGIT_WITH_BORROW_REGISTRATION,
    SubtractTwoDigit,
    Required
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
    DECIMAL_ADDITION_GENERATOR,
    COLUMN_DECIMAL_ADDITION_REGISTRATION,
    DecimalAddition
);
generator!(
    DECIMAL_SUBTRACTION_GENERATOR,
    COLUMN_DECIMAL_SUBTRACTION_REGISTRATION,
    DecimalSubtraction
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
generator!(
    DECIMAL_DIVISION_REMAINDER_GENERATOR,
    COLUMN_DECIMAL_DIVISION_REMAINDER_REGISTRATION,
    DecimalDivisionRemainder
);
generator!(
    DECIMAL_DIVISION_ROUNDED_GENERATOR,
    COLUMN_DECIMAL_DIVISION_ROUNDED_REGISTRATION,
    DecimalDivisionRounded
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

fn addition_requires_regrouping(left: i64, right: i64) -> bool {
    let ones_carry = left % 10 + right % 10 >= 10;
    let carry = i64::from(ones_carry);
    let tens_carry = left / 10 + right / 10 + carry >= 10;
    ones_carry || tens_carry
}

fn subtraction_requires_regrouping(left: i64, right: i64) -> bool {
    debug_assert!(left >= right);
    left % 10 < right % 10
}

// Current column-arithmetic candidate rules. Pre-release history lives in Git;
// production registers only the current generator for each theme.
fn draw_problem(
    registration: &ThemeRegistration,
    mode: Mode,
    regrouping: RegroupingPolicy,
    rng: &mut DeterministicRng,
    id: u32,
    _weights: &OperationWeights,
) -> Option<Result<Problem, GenerationError>> {
    let (operator, left, right, answer, operation_plan, answer_schema) = match mode {
        Mode::AddTwoDigit => {
            let left_value = draw_integer_with_digits(rng, 2)?;
            let right_value = draw_integer_with_digits(rng, 2)?;
            if !regrouping.accepts(addition_requires_regrouping(left_value, right_value)) {
                return None;
            }
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
            if !regrouping.accepts(subtraction_requires_regrouping(left_value, right_value)) {
                return None;
            }
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
        Mode::DecimalAddSubtract | Mode::DecimalAddition | Mode::DecimalSubtraction => {
            let (mut left_coefficient, mut left_scale) = draw_decimal_operand(rng, 3, 3)?;
            let (mut right_coefficient, mut right_scale) = draw_decimal_operand(rng, 3, 3)?;
            let mut left_value = exact_decimal_rational(left_coefficient, left_scale)?;
            let mut right_value = exact_decimal_rational(right_coefficient, right_scale)?;
            let operator = match mode {
                Mode::DecimalAddition => ArithmeticOperator::Add,
                Mode::DecimalSubtraction => ArithmeticOperator::Subtract,
                Mode::DecimalAddSubtract if rng.next_bounded(2) == 0 => ArithmeticOperator::Add,
                Mode::DecimalAddSubtract => ArithmeticOperator::Subtract,
                _ => unreachable!(),
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
        Mode::DecimalDivisionRemainder | Mode::DecimalDivisionRounded => {
            // Keep operand scale as ordinary variation while the Rust-owned result
            // policy controls answer precision. Both themes calculate one guard digit
            // beyond that scale, then either stop and report the remainder or round.
            // A small positive decimal divisor keeps the complete long-division
            // setup inside the existing four-column worksheet lane.
            let divisor_coefficient = 2 + rng.next_bounded(18) as i64;
            let divisor_scale = 1;
            let divisor = exact_decimal_rational(divisor_coefficient, divisor_scale)?;
            let policy = DECIMAL_DIVISION_RESULT_POLICY;
            let answer_scale = policy.answer_scale();
            let calculation_scale = policy.calculation_scale()?;
            let answer_factor = 10_i64.checked_pow(answer_scale)?;
            let calculation_factor = 10_i64.checked_pow(calculation_scale)?;
            let quotient_whole = 1 + rng.next_bounded(4) as i64;
            let quotient_fraction = rng.next_bounded(u64::try_from(answer_factor).ok()?) as i64;
            let guard_digit = 1 + rng.next_bounded(9) as i64;
            let exact_quotient_coefficient = quotient_whole
                .checked_mul(calculation_factor)?
                .checked_add(quotient_fraction.checked_mul(10)?)?
                .checked_add(guard_digit)?;
            let exact_quotient =
                exact_decimal_rational(exact_quotient_coefficient, calculation_scale)?;
            let dividend = exact_quotient.multiply(divisor)?;
            let left = rational_to_arithmetic_expression(dividend, 4)?;
            let right = exact_decimal_expression(divisor_coefficient, divisor_scale);
            debug_assert!(
                arithmetic_leaf_column_grid_cells(&left)?
                    + arithmetic_leaf_column_grid_cells(&right)?
                    <= 6
            );
            let expression =
                binary_expression(ArithmeticOperator::Divide, left.clone(), right.clone());

            if mode == Mode::DecimalDivisionRemainder {
                let quotient_coefficient = exact_quotient_coefficient / 10;
                let quotient = AnswerNode::ExactDecimal {
                    coefficient: quotient_coefficient,
                    scale: answer_scale,
                };
                let quotient_value = exact_decimal_rational(quotient_coefficient, answer_scale)?;
                let remainder = dividend.subtract(divisor.multiply(quotient_value)?)?;
                let remainder_max_scale = policy.remainder_max_scale(divisor_scale)?;
                let remainder_answer =
                    rational_to_exact_decimal_answer(remainder, remainder_max_scale)?;
                let answer = AnswerNode::Tuple(vec![quotient, remainder_answer]);
                let plan = arithmetic_expression_plan(&expression, &answer)?;
                (
                    ArithmeticOperator::Divide,
                    left,
                    right,
                    answer,
                    plan,
                    AnswerSchema::DecimalDivisionRemainder {
                        quotient_scale: answer_scale,
                        remainder_max_scale,
                    },
                )
            } else {
                let rounded_coefficient =
                    exact_quotient_coefficient / 10 + i64::from(guard_digit >= 5);
                let answer = AnswerNode::ExactDecimal {
                    coefficient: rounded_coefficient,
                    scale: answer_scale,
                };
                let plan = arithmetic_expression_plan(&expression, &answer)?;
                (
                    ArithmeticOperator::Divide,
                    left,
                    right,
                    answer,
                    plan,
                    AnswerSchema::RoundedDecimal {
                        scale: answer_scale,
                    },
                )
            }
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
pub(crate) static GENERATORS: [GeneratorEntry; 22] = [
    GeneratorEntry::current(&ADD_2DIGIT_GENERATOR),
    GeneratorEntry::current(&SUBTRACT_2DIGIT_GENERATOR),
    GeneratorEntry::current(&ADD_2DIGIT_NO_CARRY_GENERATOR),
    GeneratorEntry::current(&ADD_2DIGIT_WITH_CARRY_GENERATOR),
    GeneratorEntry::current(&SUBTRACT_2DIGIT_NO_BORROW_GENERATOR),
    GeneratorEntry::current(&SUBTRACT_2DIGIT_WITH_BORROW_GENERATOR),
    GeneratorEntry::current(&ADD_3_4DIGIT_GENERATOR),
    GeneratorEntry::current(&SUBTRACT_3_4DIGIT_GENERATOR),
    GeneratorEntry::current(&MULTIPLY_1DIGIT_GENERATOR),
    GeneratorEntry::current(&MULTIPLY_2DIGIT_GENERATOR),
    GeneratorEntry::current(&DIVIDE_2DIGIT_BY_1DIGIT_GENERATOR),
    GeneratorEntry::current(&DIVIDE_3DIGIT_BY_1DIGIT_GENERATOR),
    GeneratorEntry::current(&DIVIDE_2DIGIT_GENERATOR),
    GeneratorEntry::current(&DECIMAL_ADD_SUBTRACT_GENERATOR),
    GeneratorEntry::current(&DECIMAL_ADDITION_GENERATOR),
    GeneratorEntry::current(&DECIMAL_SUBTRACTION_GENERATOR),
    GeneratorEntry::current(&DECIMAL_MULTIPLY_INTEGER_GENERATOR),
    GeneratorEntry::current(&DECIMAL_DIVIDE_INTEGER_GENERATOR),
    GeneratorEntry::current(&DECIMAL_MULTIPLICATION_GENERATOR),
    GeneratorEntry::current(&DECIMAL_DIVISION_GENERATOR),
    GeneratorEntry::current(&DECIMAL_DIVISION_REMAINDER_GENERATOR),
    GeneratorEntry::current(&DECIMAL_DIVISION_ROUNDED_GENERATOR),
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
    fn two_digit_dedicated_themes_fix_regrouping_presence_without_count_taxonomy() {
        let cases = [
            (
                THEME_ID_COLUMN_ADD_2DIGIT_NO_CARRY,
                ArithmeticOperator::Add,
                false,
            ),
            (
                THEME_ID_COLUMN_ADD_2DIGIT_WITH_CARRY,
                ArithmeticOperator::Add,
                true,
            ),
            (
                THEME_ID_COLUMN_SUBTRACT_2DIGIT_NO_BORROW,
                ArithmeticOperator::Subtract,
                false,
            ),
            (
                THEME_ID_COLUMN_SUBTRACT_2DIGIT_WITH_BORROW,
                ArithmeticOperator::Subtract,
                true,
            ),
        ];
        for (theme_id, expected_operator, expected_regrouping) in cases {
            for difficulty in 1..=4 {
                for seed in ["RgA1", "RgB2", "RgC3"] {
                    let worksheet = generate_worksheet_request(&GenerateWorksheetRequest {
                        schema_version: SCHEMA_VERSION,
                        numeric_theme_id: theme_id,
                        seed: seed.to_owned(),
                        difficulty: crate::identity::Difficulty::try_from(difficulty).unwrap(),
                        timeout_ms: Some(1_000),
                        max_attempts: Some(50_000),
                    })
                    .unwrap_or_else(|error| {
                        panic!(
                            "regrouping theme {theme_id} d{difficulty} failed for {seed}: {error}"
                        )
                    });
                    for problem in worksheet.problems() {
                        let ProblemPrompt::ColumnArithmetic {
                            operator,
                            left,
                            right,
                        } = problem.prompt()
                        else {
                            panic!("regrouping theme returned a non-column prompt");
                        };
                        assert_eq!(*operator, expected_operator);
                        let left = column_leaf_value(left).numerator();
                        let right = column_leaf_value(right).numerator();
                        let actual = match operator {
                            ArithmeticOperator::Add => addition_requires_regrouping(left, right),
                            ArithmeticOperator::Subtract => {
                                subtraction_requires_regrouping(left, right)
                            }
                            _ => unreachable!(),
                        };
                        assert_eq!(actual, expected_regrouping);
                    }
                }
            }
        }
    }

    #[test]
    fn decimal_add_subtract_dedicated_themes_fix_only_the_operator() {
        for (theme_id, expected_operator) in [
            (THEME_ID_COLUMN_DECIMAL_ADDITION, ArithmeticOperator::Add),
            (
                THEME_ID_COLUMN_DECIMAL_SUBTRACTION,
                ArithmeticOperator::Subtract,
            ),
        ] {
            for difficulty in 1..=4 {
                for seed in ["DcA1", "DcB2", "DcC3"] {
                    let worksheet = generate_worksheet_request(&GenerateWorksheetRequest {
                        schema_version: SCHEMA_VERSION,
                        numeric_theme_id: theme_id,
                        seed: seed.to_owned(),
                        difficulty: crate::identity::Difficulty::try_from(difficulty).unwrap(),
                        timeout_ms: Some(1_000),
                        max_attempts: Some(50_000),
                    })
                    .unwrap_or_else(|error| {
                        panic!(
                            "decimal operator theme {theme_id} d{difficulty} failed for {seed}: {error}"
                        )
                    });
                    for problem in worksheet.problems() {
                        let ProblemPrompt::ColumnArithmetic { operator, .. } = problem.prompt()
                        else {
                            panic!("decimal operator theme returned a non-column prompt");
                        };
                        assert_eq!(*operator, expected_operator);
                    }
                }
            }
        }
    }

    #[test]
    fn decimal_division_result_scale_is_rust_owned_theme_metadata() {
        let expected = Some(DECIMAL_DIVISION_RESULT_POLICY.answer_scale());
        assert_eq!(
            COLUMN_DECIMAL_DIVISION_REMAINDER_REGISTRATION.answer_decimal_scale(),
            expected
        );
        assert_eq!(
            COLUMN_DECIMAL_DIVISION_ROUNDED_REGISTRATION.answer_decimal_scale(),
            expected
        );
        assert_eq!(
            COLUMN_DECIMAL_DIVISION_REGISTRATION.answer_decimal_scale(),
            None
        );
    }

    #[test]
    fn decimal_division_result_themes_enforce_remainder_and_rounding_semantics() {
        use crate::exact_value::rational_parts_from_answer;
        use crate::semantics::evaluate_expression;

        for difficulty in 1..=4 {
            for seed in ["DdR1", "DdR2", "DdR3"] {
                let remainder_worksheet = generate_worksheet_request(&GenerateWorksheetRequest {
                    schema_version: SCHEMA_VERSION,
                    numeric_theme_id: THEME_ID_COLUMN_DECIMAL_DIVISION_REMAINDER,
                    seed: seed.to_owned(),
                    difficulty: crate::identity::Difficulty::try_from(difficulty).unwrap(),
                    timeout_ms: Some(1_000),
                    max_attempts: Some(50_000),
                })
                .unwrap_or_else(|error| panic!("decimal remainder d{difficulty} {seed}: {error}"));

                for problem in remainder_worksheet.problems() {
                    let ProblemPrompt::ColumnArithmetic {
                        operator: ArithmeticOperator::Divide,
                        left,
                        right,
                    } = problem.prompt()
                    else {
                        panic!("decimal remainder theme returned a non-division prompt");
                    };
                    assert_eq!(
                        problem.answer_schema(),
                        &AnswerSchema::DecimalDivisionRemainder {
                            quotient_scale: DECIMAL_DIVISION_RESULT_POLICY.answer_scale(),
                            remainder_max_scale: DECIMAL_DIVISION_RESULT_POLICY
                                .remainder_max_scale(1)
                                .unwrap(),
                        }
                    );
                    let AnswerNode::Tuple(values) = problem.canonical_answer() else {
                        panic!("decimal remainder answer is not a pair");
                    };
                    let dividend = evaluate_expression(left).unwrap();
                    let divisor = evaluate_expression(right).unwrap();
                    let (quotient_n, quotient_d) = rational_parts_from_answer(&values[0]).unwrap();
                    let (remainder_n, remainder_d) =
                        rational_parts_from_answer(&values[1]).unwrap();
                    let quotient = RationalCoefficient::new(
                        i64::try_from(quotient_n).unwrap(),
                        i64::try_from(quotient_d).unwrap(),
                    )
                    .unwrap();
                    let remainder = RationalCoefficient::new(
                        i64::try_from(remainder_n).unwrap(),
                        i64::try_from(remainder_d).unwrap(),
                    )
                    .unwrap();
                    assert_eq!(
                        divisor
                            .multiply(quotient)
                            .unwrap()
                            .checked_add(remainder)
                            .unwrap(),
                        dividend
                    );
                    let answer_factor = 10_i64.pow(DECIMAL_DIVISION_RESULT_POLICY.answer_scale());
                    let bound = divisor
                        .multiply(RationalCoefficient::new(1, answer_factor).unwrap())
                        .unwrap();
                    assert!(remainder.numerator() > 0);
                    assert!(rational_less_than(remainder, bound));
                }

                let rounded_worksheet = generate_worksheet_request(&GenerateWorksheetRequest {
                    schema_version: SCHEMA_VERSION,
                    numeric_theme_id: THEME_ID_COLUMN_DECIMAL_DIVISION_ROUNDED,
                    seed: seed.to_owned(),
                    difficulty: crate::identity::Difficulty::try_from(difficulty).unwrap(),
                    timeout_ms: Some(1_000),
                    max_attempts: Some(50_000),
                })
                .unwrap_or_else(|error| panic!("decimal rounded d{difficulty} {seed}: {error}"));

                for problem in rounded_worksheet.problems() {
                    let ProblemPrompt::ColumnArithmetic {
                        operator: ArithmeticOperator::Divide,
                        left,
                        right,
                    } = problem.prompt()
                    else {
                        panic!("decimal rounded theme returned a non-division prompt");
                    };
                    assert_eq!(
                        problem.answer_schema(),
                        &AnswerSchema::RoundedDecimal {
                            scale: DECIMAL_DIVISION_RESULT_POLICY.answer_scale(),
                        }
                    );
                    let exact = evaluate_expression(left)
                        .unwrap()
                        .divide(evaluate_expression(right).unwrap())
                        .unwrap();
                    let calculation_factor =
                        10_i64.pow(DECIMAL_DIVISION_RESULT_POLICY.calculation_scale().unwrap());
                    let answer_factor = 10_i64.pow(DECIMAL_DIVISION_RESULT_POLICY.answer_scale());
                    let exact_calculation_units =
                        exact.numerator() * calculation_factor / exact.denominator();
                    assert_ne!(exact_calculation_units % 10, 0);
                    let rounded_answer_units = exact_calculation_units / 10
                        + i64::from((exact_calculation_units % 10) >= 5);
                    let expected =
                        RationalCoefficient::new(rounded_answer_units, answer_factor).unwrap();
                    let (answer_n, answer_d) =
                        rational_parts_from_answer(problem.canonical_answer()).unwrap();
                    let actual = RationalCoefficient::new(
                        i64::try_from(answer_n).unwrap(),
                        i64::try_from(answer_d).unwrap(),
                    )
                    .unwrap();
                    assert_eq!(actual, expected);
                }
            }
        }
    }

    #[test]
    fn column_arithmetic_themes_follow_curriculum_domains_and_print_layouts() {
        use crate::themes::column_arithmetic::*;
        const IDS: [u32; 22] = [
            THEME_ID_COLUMN_ADD_2DIGIT,
            THEME_ID_COLUMN_SUBTRACT_2DIGIT,
            THEME_ID_COLUMN_ADD_2DIGIT_NO_CARRY,
            THEME_ID_COLUMN_ADD_2DIGIT_WITH_CARRY,
            THEME_ID_COLUMN_SUBTRACT_2DIGIT_NO_BORROW,
            THEME_ID_COLUMN_SUBTRACT_2DIGIT_WITH_BORROW,
            THEME_ID_COLUMN_ADD_3_4DIGIT,
            THEME_ID_COLUMN_SUBTRACT_3_4DIGIT,
            THEME_ID_COLUMN_MULTIPLY_1DIGIT,
            THEME_ID_COLUMN_MULTIPLY_2DIGIT,
            THEME_ID_COLUMN_DIVIDE_2DIGIT_BY_1DIGIT,
            THEME_ID_COLUMN_DIVIDE_3DIGIT_BY_1DIGIT,
            THEME_ID_COLUMN_DIVIDE_2DIGIT,
            THEME_ID_COLUMN_DECIMAL_ADD_SUBTRACT,
            THEME_ID_COLUMN_DECIMAL_ADDITION,
            THEME_ID_COLUMN_DECIMAL_SUBTRACTION,
            THEME_ID_COLUMN_DECIMAL_MULTIPLY_INTEGER,
            THEME_ID_COLUMN_DECIMAL_DIVIDE_INTEGER,
            THEME_ID_COLUMN_DECIMAL_MULTIPLICATION,
            THEME_ID_COLUMN_DECIMAL_DIVISION,
            THEME_ID_COLUMN_DECIMAL_DIVISION_REMAINDER,
            THEME_ID_COLUMN_DECIMAL_DIVISION_ROUNDED,
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
                    | THEME_ID_COLUMN_DECIMAL_DIVISION_REMAINDER
                    | THEME_ID_COLUMN_DECIMAL_DIVISION_ROUNDED
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
                        THEME_ID_COLUMN_ADD_2DIGIT
                        | THEME_ID_COLUMN_ADD_2DIGIT_NO_CARRY
                        | THEME_ID_COLUMN_ADD_2DIGIT_WITH_CARRY => {
                            assert_eq!(*operator, ArithmeticOperator::Add);
                            assert!((10..=99).contains(&left_value.numerator()));
                            assert!((10..=99).contains(&right_value.numerator()));
                            assert_eq!(left_value.denominator(), 1);
                            assert_eq!(right_value.denominator(), 1);
                        }
                        THEME_ID_COLUMN_SUBTRACT_2DIGIT
                        | THEME_ID_COLUMN_SUBTRACT_2DIGIT_NO_BORROW
                        | THEME_ID_COLUMN_SUBTRACT_2DIGIT_WITH_BORROW => {
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
                        THEME_ID_COLUMN_DECIMAL_ADD_SUBTRACT
                        | THEME_ID_COLUMN_DECIMAL_ADDITION
                        | THEME_ID_COLUMN_DECIMAL_SUBTRACTION => {
                            let expected_operator = match theme_id {
                                THEME_ID_COLUMN_DECIMAL_ADDITION => Some(ArithmeticOperator::Add),
                                THEME_ID_COLUMN_DECIMAL_SUBTRACTION => {
                                    Some(ArithmeticOperator::Subtract)
                                }
                                THEME_ID_COLUMN_DECIMAL_ADD_SUBTRACT => None,
                                _ => unreachable!(),
                            };
                            if let Some(expected_operator) = expected_operator {
                                assert_eq!(*operator, expected_operator);
                            } else {
                                assert!(matches!(
                                    operator,
                                    ArithmeticOperator::Add | ArithmeticOperator::Subtract
                                ));
                            }
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
                        THEME_ID_COLUMN_DECIMAL_DIVISION
                        | THEME_ID_COLUMN_DECIMAL_DIVISION_REMAINDER
                        | THEME_ID_COLUMN_DECIMAL_DIVISION_ROUNDED => {
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
                            if theme_id != THEME_ID_COLUMN_DECIMAL_DIVISION {
                                continue;
                            }
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
