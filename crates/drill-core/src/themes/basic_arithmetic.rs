use crate::answer::AnswerNode;
use crate::effort::{
    arithmetic_expression_plan, integer_division_with_remainder_plan, one_digit_addition_plan,
    one_digit_subtraction_plan, two_digit_addition_plan, EffortModel, OperationWeights,
};
use crate::error::GenerationError;
use crate::generator::{
    FiniteCandidateSource, GeneratorEntry, ProblemGenerator, RandomCandidateSource,
    SamplingStrategy, SelectionDedup,
};
use crate::generator_support::{
    binary_expression, draw_bounded_arithmetic_ast, draw_signed_integer, ensure_negative_term,
    evaluate_expression, exact_decimal_expression, integer_expression, rational_answer,
    rational_expression,
};
use crate::model::{
    AnswerSchema, ArithmeticExpression, ArithmeticOperator, Problem, ProblemPrompt,
    RationalCoefficient,
};
use crate::rng::DeterministicRng;
use crate::theme::{
    CurriculumSafetyPolicy as Safety, CurriculumUnit, DedupPolicy as Dedup, SchoolGrade,
    ThemeAnswerContract as AnswerContract, ThemeInputProfile as Input,
    ThemePresentationPolicy as Presentation, ThemeRegistration, ThemeRegistrationSpec, ThemeTag,
    MULTIPLICATION_ROW_5_LAYOUT, STANDARD_20_LAYOUT,
};
use crate::themes::{division_table, multiplication_table};

pub const MIN_OPERAND: u8 = 1;
pub const MAX_OPERAND: u8 = 9;
pub const MIN_ANSWER: u8 = 1;
pub const MAX_ANSWER: u8 = 18;
pub const THEME_ID_ONE_DIGIT_ADDITION: u32 = 1;
pub const THEME_ID_ONE_DIGIT_SUBTRACTION: u32 = 4;
pub const THEME_ID_TWO_DIGIT_ADDITION: u32 = 5;
pub const THEME_ID_MULTIPLICATION_TABLE: u32 = 6;
pub const THEME_ID_SIGNED_ARITHMETIC_1: u32 = 7;
pub const THEME_ID_SIGNED_ARITHMETIC_2: u32 = 8;
pub const THEME_ID_SIGNED_MULTIPLY_DIVIDE: u32 = 67;
pub const THEME_ID_SIGNED_ARITHMETIC_MIXED_OPERANDS: u32 = 68;
pub const THEME_ID_DIVISION_1: u32 = 13;
pub const THEME_ID_ADDITION_UP_TO_10: u32 = 39;
pub const THEME_ID_SUBTRACTION_UP_TO_10: u32 = 40;
pub const THEME_ID_ADDITION_WITH_CARRY: u32 = 41;
pub const THEME_ID_SUBTRACTION_WITH_BORROW: u32 = 42;
pub const THEME_ID_MULTIPLICATION_TABLE_1: u32 = 43;
pub const THEME_ID_MULTIPLICATION_TABLE_2: u32 = 44;
pub const THEME_ID_MULTIPLICATION_TABLE_3: u32 = 45;
pub const THEME_ID_MULTIPLICATION_TABLE_4: u32 = 46;
pub const THEME_ID_MULTIPLICATION_TABLE_5: u32 = 47;
pub const THEME_ID_MULTIPLICATION_TABLE_6: u32 = 48;
pub const THEME_ID_MULTIPLICATION_TABLE_7: u32 = 49;
pub const THEME_ID_MULTIPLICATION_TABLE_8: u32 = 50;
pub const THEME_ID_MULTIPLICATION_TABLE_9: u32 = 51;
pub const THEME_ID_DIVISION_WITH_REMAINDER: u32 = 52;
pub const THEME_ID_SIMPLE_TWO_DIGIT_DIVISION: u32 = 53;
pub const GENERATOR_REVISION_ONE_DIGIT_ADDITION: u32 = 5;
pub const GENERATOR_REVISION_ONE_DIGIT_SUBTRACTION: u32 = 3;
pub const GENERATOR_REVISION_TWO_DIGIT_ADDITION: u32 = 3;
pub const GENERATOR_REVISION_MULTIPLICATION_TABLE: u32 = 3;
pub const GENERATOR_REVISION_SIGNED_ARITHMETIC_1: u32 = 3;
pub const GENERATOR_REVISION_SIGNED_ARITHMETIC_2: u32 = 3;
pub const GENERATOR_REVISION_SIGNED_MULTIPLY_DIVIDE: u32 = 1;
pub const GENERATOR_REVISION_SIGNED_ARITHMETIC_MIXED_OPERANDS: u32 = 1;
pub const GENERATOR_REVISION_DIVISION_1: u32 = 3;
pub const GENERATOR_REVISION_ADDITION_UP_TO_10: u32 = 2;
pub const GENERATOR_REVISION_SUBTRACTION_UP_TO_10: u32 = 2;
pub const GENERATOR_REVISION_ADDITION_WITH_CARRY: u32 = 2;
pub const GENERATOR_REVISION_SUBTRACTION_WITH_BORROW: u32 = 2;
pub const GENERATOR_REVISION_MULTIPLICATION_TABLE_ROW: u32 = 2;
pub const GENERATOR_REVISION_DIVISION_WITH_REMAINDER: u32 = 1;
pub const GENERATOR_REVISION_SIMPLE_TWO_DIGIT_DIVISION: u32 = 2;
pub const SKILL_ID: &str = "jp.grade1.addition.one_digit";
pub const SKILL_ID_ONE_DIGIT_SUBTRACTION: &str = "jp.grade1.subtraction.one_digit";
pub const SKILL_ID_TWO_DIGIT_ADDITION: &str = "jp.grade2.addition.two_digit";
pub const SKILL_ID_MULTIPLICATION_TABLE: &str = "jp.grade2.multiplication.table";
pub const SKILL_ID_SIGNED_ARITHMETIC_1: &str = "jp.grade7.signed.add_subtract";
pub const SKILL_ID_SIGNED_ARITHMETIC_2: &str = "jp.grade7.signed.summary.integer";
pub const SKILL_ID_SIGNED_MULTIPLY_DIVIDE: &str = "jp.grade7.signed.multiply_divide";
pub const SKILL_ID_SIGNED_ARITHMETIC_MIXED_OPERANDS: &str =
    "jp.grade7.signed.summary.mixed_operands";
pub const SKILL_ID_DIVISION_1: &str = "jp.grade3.division.table.exact";
pub const SKILL_ID_ADDITION_UP_TO_10: &str = "jp.grade1.addition.up_to_10";
pub const SKILL_ID_SUBTRACTION_UP_TO_10: &str = "jp.grade1.subtraction.up_to_10";
pub const SKILL_ID_ADDITION_WITH_CARRY: &str = "jp.grade1.addition.with_carry";
pub const SKILL_ID_SUBTRACTION_WITH_BORROW: &str = "jp.grade1.subtraction.with_borrow";
pub const SKILL_ID_MULTIPLICATION_TABLE_ROWS: [&str; 9] = [
    "jp.grade2.multiplication.table.1",
    "jp.grade2.multiplication.table.2",
    "jp.grade2.multiplication.table.3",
    "jp.grade2.multiplication.table.4",
    "jp.grade2.multiplication.table.5",
    "jp.grade2.multiplication.table.6",
    "jp.grade2.multiplication.table.7",
    "jp.grade2.multiplication.table.8",
    "jp.grade2.multiplication.table.9",
];
pub const SKILL_ID_DIVISION_WITH_REMAINDER: &str = "jp.grade3.division.table.remainder";
pub const SKILL_ID_SIMPLE_TWO_DIGIT_DIVISION: &str = "jp.grade3.division.simple_two_digit";
pub const CURRICULUM_UNIT_GRADE1_ADDITION: CurriculumUnit =
    CurriculumUnit::new("grade1-addition", "足し算");
pub const CURRICULUM_UNIT_GRADE1_SUBTRACTION: CurriculumUnit =
    CurriculumUnit::new("grade1-subtraction", "引き算");
pub const CURRICULUM_UNIT_MULTIPLICATION_TABLE: CurriculumUnit =
    CurriculumUnit::new("multiplication-table", "九九");
pub const CURRICULUM_UNIT_DIVISION_TABLE: CurriculumUnit =
    CurriculumUnit::new("division-table", "九九を使う割り算");
pub const CURRICULUM_UNIT_SIGNED_NUMBERS: CurriculumUnit =
    CurriculumUnit::new("signed-numbers", "正負の数");
pub const CURRICULUM_PATH: [&str; 4] = ["root", "小学1年生", "足し算", "一桁の足し算（まとめ）"];
pub const CURRICULUM_PATH_ONE_DIGIT_SUBTRACTION: [&str; 4] =
    ["root", "小学1年生", "引き算", "一桁の引き算（まとめ）"];
pub const CURRICULUM_PATH_TWO_DIGIT_ADDITION: [&str; 3] = ["root", "小学2年生", "二桁の足し算"];
pub const CURRICULUM_PATH_MULTIPLICATION_TABLE: [&str; 4] =
    ["root", "小学2年生", "九九", "全段混合"];
pub const CURRICULUM_PATH_SIGNED_ARITHMETIC_1: [&str; 4] =
    ["root", "中学1年生", "正負の数", "正負の数の加法・減法"];
pub const CURRICULUM_PATH_SIGNED_MULTIPLY_DIVIDE: [&str; 4] =
    ["root", "中学1年生", "正負の数", "正負の数の乗法・除法"];
pub const CURRICULUM_PATH_SIGNED_ARITHMETIC_2: [&str; 4] = [
    "root",
    "中学1年生",
    "正負の数",
    "正負の数の四則計算（まとめ(1)：整数中心）",
];
pub const CURRICULUM_PATH_SIGNED_ARITHMETIC_MIXED_OPERANDS: [&str; 4] = [
    "root",
    "中学1年生",
    "正負の数",
    "正負の数の四則計算（まとめ(2)：小数・分数を含む）",
];
pub const CURRICULUM_PATH_DIVISION_1: [&str; 4] = [
    "root",
    "小学3年生",
    "九九を使う割り算",
    "あまりのない割り算",
];
pub const CURRICULUM_PATH_ADDITION_UP_TO_10: [&str; 4] =
    ["root", "小学1年生", "足し算", "10までの足し算"];
pub const CURRICULUM_PATH_SUBTRACTION_UP_TO_10: [&str; 4] =
    ["root", "小学1年生", "引き算", "10までの引き算"];
pub const CURRICULUM_PATH_ADDITION_WITH_CARRY: [&str; 4] =
    ["root", "小学1年生", "足し算", "繰り上がりのある足し算"];
pub const CURRICULUM_PATH_SUBTRACTION_WITH_BORROW: [&str; 4] =
    ["root", "小学1年生", "引き算", "繰り下がりのある引き算"];
pub const CURRICULUM_PATH_MULTIPLICATION_TABLE_ROWS: [[&str; 4]; 9] = [
    ["root", "小学2年生", "九九", "1の段"],
    ["root", "小学2年生", "九九", "2の段"],
    ["root", "小学2年生", "九九", "3の段"],
    ["root", "小学2年生", "九九", "4の段"],
    ["root", "小学2年生", "九九", "5の段"],
    ["root", "小学2年生", "九九", "6の段"],
    ["root", "小学2年生", "九九", "7の段"],
    ["root", "小学2年生", "九九", "8の段"],
    ["root", "小学2年生", "九九", "9の段"],
];
pub const CURRICULUM_PATH_DIVISION_WITH_REMAINDER: [&str; 4] = [
    "root",
    "小学3年生",
    "九九を使う割り算",
    "あまりのある割り算",
];
pub const CURRICULUM_PATH_SIMPLE_TWO_DIGIT_DIVISION: [&str; 3] =
    ["root", "小学3年生", "簡単な2桁÷1桁"];

const ADDITION: &[ThemeTag] = &[ThemeTag::Addition];
const SUBTRACTION: &[ThemeTag] = &[ThemeTag::Subtraction];
const MULTIPLICATION: &[ThemeTag] = &[ThemeTag::Multiplication];
const DIVISION: &[ThemeTag] = &[ThemeTag::Division];
const NEGATIVE_NUMBERS: &[ThemeTag] = &[ThemeTag::NegativeNumbers];
const SIGNED_ADD_SUBTRACT_OPERATORS: [ArithmeticOperator; 2] =
    [ArithmeticOperator::Add, ArithmeticOperator::Subtract];
const SIGNED_MULTIPLY_DIVIDE_OPERATORS: [ArithmeticOperator; 2] =
    [ArithmeticOperator::Multiply, ArithmeticOperator::Divide];
const SIGNED_FOUR_OPERATORS: [ArithmeticOperator; 4] = [
    ArithmeticOperator::Add,
    ArithmeticOperator::Subtract,
    ArithmeticOperator::Multiply,
    ArithmeticOperator::Divide,
];

pub const ONE_DIGIT_ADDITION_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_ONE_DIGIT_ADDITION),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_ONE_DIGIT_ADDITION,
        ),
        skill_id: SKILL_ID,
        curriculum_path: &CURRICULUM_PATH,
        grade: Some(SchoolGrade::Elementary1),
        tags: ADDITION,
        safety: Safety::NonNegativeOnly,
        presentation: Presentation::STANDARD,
        dedup: Dedup::PreserveOperandOrder,
        answer_contract: AnswerContract::AdditionInteger,
        layout: STANDARD_20_LAYOUT,
    })
    .with_curriculum_unit(CURRICULUM_UNIT_GRADE1_ADDITION);

pub const ONE_DIGIT_SUBTRACTION_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_ONE_DIGIT_SUBTRACTION),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_ONE_DIGIT_SUBTRACTION,
        ),
        skill_id: SKILL_ID_ONE_DIGIT_SUBTRACTION,
        curriculum_path: &CURRICULUM_PATH_ONE_DIGIT_SUBTRACTION,
        grade: Some(SchoolGrade::Elementary1),
        tags: SUBTRACTION,
        safety: Safety::NonNegativeOnly,
        presentation: Presentation::STANDARD,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: AnswerContract::ArithmeticPositiveInteger,
        layout: STANDARD_20_LAYOUT,
    })
    .with_curriculum_unit(CURRICULUM_UNIT_GRADE1_SUBTRACTION);

pub const TWO_DIGIT_ADDITION_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_TWO_DIGIT_ADDITION),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_TWO_DIGIT_ADDITION,
        ),
        skill_id: SKILL_ID_TWO_DIGIT_ADDITION,
        curriculum_path: &CURRICULUM_PATH_TWO_DIGIT_ADDITION,
        grade: Some(SchoolGrade::Elementary2),
        tags: ADDITION,
        safety: Safety::NonNegativeOnly,
        presentation: Presentation::STANDARD,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: AnswerContract::ArithmeticPositiveInteger,
        layout: STANDARD_20_LAYOUT,
    });

pub const MULTIPLICATION_TABLE_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_MULTIPLICATION_TABLE),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_MULTIPLICATION_TABLE,
        ),
        skill_id: SKILL_ID_MULTIPLICATION_TABLE,
        curriculum_path: &CURRICULUM_PATH_MULTIPLICATION_TABLE,
        grade: Some(SchoolGrade::Elementary2),
        tags: MULTIPLICATION,
        safety: Safety::NonNegativeOnly,
        presentation: Presentation::STANDARD,
        dedup: Dedup::PreserveOperandOrder,
        answer_contract: AnswerContract::ArithmeticPositiveInteger,
        layout: STANDARD_20_LAYOUT,
    })
    .with_curriculum_unit(CURRICULUM_UNIT_MULTIPLICATION_TABLE);

pub const DIVISION_1_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_DIVISION_1),
        generator_revision: crate::theme::GeneratorRevision::new(GENERATOR_REVISION_DIVISION_1),
        skill_id: SKILL_ID_DIVISION_1,
        curriculum_path: &CURRICULUM_PATH_DIVISION_1,
        grade: Some(SchoolGrade::Elementary3),
        tags: DIVISION,
        safety: Safety::NonNegativeOnly,
        presentation: Presentation::STANDARD,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: AnswerContract::ArithmeticPositiveInteger,
        layout: STANDARD_20_LAYOUT,
    })
    .with_curriculum_unit(CURRICULUM_UNIT_DIVISION_TABLE);

pub const ADDITION_UP_TO_10_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_ADDITION_UP_TO_10),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_ADDITION_UP_TO_10,
        ),
        skill_id: SKILL_ID_ADDITION_UP_TO_10,
        curriculum_path: &CURRICULUM_PATH_ADDITION_UP_TO_10,
        grade: Some(SchoolGrade::Elementary1),
        tags: ADDITION,
        safety: Safety::NonNegativeOnly,
        presentation: Presentation::STANDARD,
        dedup: Dedup::PreserveOperandOrder,
        answer_contract: AnswerContract::ArithmeticPositiveInteger,
        layout: STANDARD_20_LAYOUT,
    })
    .with_curriculum_unit(CURRICULUM_UNIT_GRADE1_ADDITION);

pub const SUBTRACTION_UP_TO_10_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_SUBTRACTION_UP_TO_10),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_SUBTRACTION_UP_TO_10,
        ),
        skill_id: SKILL_ID_SUBTRACTION_UP_TO_10,
        curriculum_path: &CURRICULUM_PATH_SUBTRACTION_UP_TO_10,
        grade: Some(SchoolGrade::Elementary1),
        tags: SUBTRACTION,
        safety: Safety::NonNegativeOnly,
        presentation: Presentation::STANDARD,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: AnswerContract::ArithmeticPositiveInteger,
        layout: STANDARD_20_LAYOUT,
    })
    .with_curriculum_unit(CURRICULUM_UNIT_GRADE1_SUBTRACTION);

pub const ADDITION_WITH_CARRY_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_ADDITION_WITH_CARRY),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_ADDITION_WITH_CARRY,
        ),
        skill_id: SKILL_ID_ADDITION_WITH_CARRY,
        curriculum_path: &CURRICULUM_PATH_ADDITION_WITH_CARRY,
        grade: Some(SchoolGrade::Elementary1),
        tags: ADDITION,
        safety: Safety::NonNegativeOnly,
        presentation: Presentation::STANDARD,
        dedup: Dedup::PreserveOperandOrder,
        answer_contract: AnswerContract::ArithmeticPositiveInteger,
        layout: STANDARD_20_LAYOUT,
    })
    .with_curriculum_unit(CURRICULUM_UNIT_GRADE1_ADDITION);

pub const SUBTRACTION_WITH_BORROW_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_SUBTRACTION_WITH_BORROW),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_SUBTRACTION_WITH_BORROW,
        ),
        skill_id: SKILL_ID_SUBTRACTION_WITH_BORROW,
        curriculum_path: &CURRICULUM_PATH_SUBTRACTION_WITH_BORROW,
        grade: Some(SchoolGrade::Elementary1),
        tags: SUBTRACTION,
        safety: Safety::NonNegativeOnly,
        presentation: Presentation::STANDARD,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: AnswerContract::ArithmeticPositiveInteger,
        layout: STANDARD_20_LAYOUT,
    })
    .with_curriculum_unit(CURRICULUM_UNIT_GRADE1_SUBTRACTION);

macro_rules! multiplication_row_registration {
    ($name:ident, $theme_id:expr, $index:expr) => {
        pub const $name: ThemeRegistration = ThemeRegistration::new(ThemeRegistrationSpec {
            numeric_theme_id: crate::theme::ThemeId::new($theme_id),
            generator_revision: crate::theme::GeneratorRevision::new(
                GENERATOR_REVISION_MULTIPLICATION_TABLE_ROW,
            ),
            skill_id: SKILL_ID_MULTIPLICATION_TABLE_ROWS[$index],
            curriculum_path: &CURRICULUM_PATH_MULTIPLICATION_TABLE_ROWS[$index],
            grade: Some(SchoolGrade::Elementary2),
            tags: MULTIPLICATION,
            safety: Safety::NonNegativeOnly,
            presentation: Presentation::STANDARD,
            dedup: Dedup::PreserveOperandOrder,
            answer_contract: AnswerContract::ArithmeticPositiveInteger,
            layout: MULTIPLICATION_ROW_5_LAYOUT,
        })
        .with_curriculum_unit(CURRICULUM_UNIT_MULTIPLICATION_TABLE);
    };
}
multiplication_row_registration!(
    MULTIPLICATION_TABLE_1_REGISTRATION,
    THEME_ID_MULTIPLICATION_TABLE_1,
    0
);
multiplication_row_registration!(
    MULTIPLICATION_TABLE_2_REGISTRATION,
    THEME_ID_MULTIPLICATION_TABLE_2,
    1
);
multiplication_row_registration!(
    MULTIPLICATION_TABLE_3_REGISTRATION,
    THEME_ID_MULTIPLICATION_TABLE_3,
    2
);
multiplication_row_registration!(
    MULTIPLICATION_TABLE_4_REGISTRATION,
    THEME_ID_MULTIPLICATION_TABLE_4,
    3
);
multiplication_row_registration!(
    MULTIPLICATION_TABLE_5_REGISTRATION,
    THEME_ID_MULTIPLICATION_TABLE_5,
    4
);
multiplication_row_registration!(
    MULTIPLICATION_TABLE_6_REGISTRATION,
    THEME_ID_MULTIPLICATION_TABLE_6,
    5
);
multiplication_row_registration!(
    MULTIPLICATION_TABLE_7_REGISTRATION,
    THEME_ID_MULTIPLICATION_TABLE_7,
    6
);
multiplication_row_registration!(
    MULTIPLICATION_TABLE_8_REGISTRATION,
    THEME_ID_MULTIPLICATION_TABLE_8,
    7
);
multiplication_row_registration!(
    MULTIPLICATION_TABLE_9_REGISTRATION,
    THEME_ID_MULTIPLICATION_TABLE_9,
    8
);

pub const DIVISION_WITH_REMAINDER_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_DIVISION_WITH_REMAINDER),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_DIVISION_WITH_REMAINDER,
        ),
        skill_id: SKILL_ID_DIVISION_WITH_REMAINDER,
        curriculum_path: &CURRICULUM_PATH_DIVISION_WITH_REMAINDER,
        grade: Some(SchoolGrade::Elementary3),
        tags: DIVISION,
        safety: Safety::NonNegativeOnly,
        presentation: Presentation::STANDARD,
        dedup: Dedup::PreserveOperandOrder,
        answer_contract: AnswerContract::ArithmeticIntegerDivision,
        layout: STANDARD_20_LAYOUT,
    })
    .with_curriculum_unit(CURRICULUM_UNIT_DIVISION_TABLE);

pub const SIMPLE_TWO_DIGIT_DIVISION_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_SIMPLE_TWO_DIGIT_DIVISION),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_SIMPLE_TWO_DIGIT_DIVISION,
        ),
        skill_id: SKILL_ID_SIMPLE_TWO_DIGIT_DIVISION,
        curriculum_path: &CURRICULUM_PATH_SIMPLE_TWO_DIGIT_DIVISION,
        grade: Some(SchoolGrade::Elementary3),
        tags: DIVISION,
        safety: Safety::NonNegativeOnly,
        presentation: Presentation::STANDARD,
        dedup: Dedup::PreserveOperandOrder,
        answer_contract: AnswerContract::ArithmeticPositiveInteger,
        layout: STANDARD_20_LAYOUT,
    });

pub const SIGNED_ARITHMETIC_1_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_SIGNED_ARITHMETIC_1),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_SIGNED_ARITHMETIC_1,
        ),
        skill_id: SKILL_ID_SIGNED_ARITHMETIC_1,
        curriculum_path: &CURRICULUM_PATH_SIGNED_ARITHMETIC_1,
        grade: Some(SchoolGrade::JuniorHigh1),
        tags: NEGATIVE_NUMBERS,
        safety: Safety::Unrestricted,
        presentation: Presentation::STANDARD,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: AnswerContract::ArithmeticSignedInteger,
        layout: STANDARD_20_LAYOUT,
    })
    .with_curriculum_unit(CURRICULUM_UNIT_SIGNED_NUMBERS)
    .with_editor_input_profile(Input::JuniorHighFull);

pub const SIGNED_ARITHMETIC_2_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_SIGNED_ARITHMETIC_2),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_SIGNED_ARITHMETIC_2,
        ),
        skill_id: SKILL_ID_SIGNED_ARITHMETIC_2,
        curriculum_path: &CURRICULUM_PATH_SIGNED_ARITHMETIC_2,
        grade: Some(SchoolGrade::JuniorHigh1),
        tags: NEGATIVE_NUMBERS,
        safety: Safety::Unrestricted,
        presentation: Presentation::STANDARD,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: AnswerContract::ArithmeticSignedRational,
        layout: STANDARD_20_LAYOUT,
    })
    .with_curriculum_unit(CURRICULUM_UNIT_SIGNED_NUMBERS)
    .with_editor_input_profile(Input::JuniorHighFull);

pub const SIGNED_MULTIPLY_DIVIDE_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_SIGNED_MULTIPLY_DIVIDE),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_SIGNED_MULTIPLY_DIVIDE,
        ),
        skill_id: SKILL_ID_SIGNED_MULTIPLY_DIVIDE,
        curriculum_path: &CURRICULUM_PATH_SIGNED_MULTIPLY_DIVIDE,
        grade: Some(SchoolGrade::JuniorHigh1),
        tags: NEGATIVE_NUMBERS,
        safety: Safety::Unrestricted,
        presentation: Presentation::STANDARD,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: AnswerContract::ArithmeticSignedRational,
        layout: STANDARD_20_LAYOUT,
    })
    .with_curriculum_unit(CURRICULUM_UNIT_SIGNED_NUMBERS)
    .with_editor_input_profile(Input::JuniorHighFull);

pub const SIGNED_ARITHMETIC_MIXED_OPERANDS_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: crate::theme::ThemeId::new(THEME_ID_SIGNED_ARITHMETIC_MIXED_OPERANDS),
        generator_revision: crate::theme::GeneratorRevision::new(
            GENERATOR_REVISION_SIGNED_ARITHMETIC_MIXED_OPERANDS,
        ),
        skill_id: SKILL_ID_SIGNED_ARITHMETIC_MIXED_OPERANDS,
        curriculum_path: &CURRICULUM_PATH_SIGNED_ARITHMETIC_MIXED_OPERANDS,
        grade: Some(SchoolGrade::JuniorHigh1),
        tags: NEGATIVE_NUMBERS,
        safety: Safety::Unrestricted,
        presentation: Presentation::STANDARD,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: AnswerContract::ArithmeticSignedRational,
        layout: STANDARD_20_LAYOUT,
    })
    .with_curriculum_unit(CURRICULUM_UNIT_SIGNED_NUMBERS)
    .with_editor_input_profile(Input::JuniorHighFull);

#[derive(Clone, Copy, Debug)]
enum Mode {
    OneDigitAddition,
    OneDigitSubtraction,
    TwoDigitAddition,
    AdditionUpTo10,
    SubtractionUpTo10,
    AdditionWithCarry,
    SubtractionWithBorrow,
    MultiplicationTable(Option<u8>),
    DivisionTable,
    DivisionWithRemainder,
    SimpleTwoDigitDivision,
    SignedAddSubtract,
    SignedMultiplyDivide,
    SignedSummaryInteger,
    SignedSummaryMixedOperands,
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
        if FiniteBasicDomain::for_mode(self.mode).is_some() {
            Ok(SamplingStrategy::finite(
                self,
                SelectionDedup::AllowDuplicates,
            ))
        } else {
            Ok(SamplingStrategy::random(
                self,
                SelectionDedup::AllowDuplicates,
            ))
        }
    }
}

impl FiniteCandidateSource for Generator {
    fn candidate_count(&self) -> usize {
        FiniteBasicDomain::for_mode(self.mode)
            .expect("finite sampling strategy is only used by finite basic-arithmetic modes")
            .candidate_count()
    }

    fn candidate_at(
        &self,
        index: usize,
        _weights: &OperationWeights,
    ) -> Result<Problem, GenerationError> {
        FiniteBasicDomain::for_mode(self.mode)
            .and_then(|domain| domain.problem_at(self.registration, index))
            .ok_or(GenerationError::InvalidGeneratedProblem {
                reason: "finite candidate index is outside the declared domain",
            })?
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

macro_rules! generator {
    ($name:ident, $registration:ident, $mode:expr) => {
        pub(crate) static $name: Generator = Generator {
            registration: &$registration,
            mode: $mode,
        };
    };
}

generator!(
    ONE_DIGIT_ADDITION_GENERATOR,
    ONE_DIGIT_ADDITION_REGISTRATION,
    Mode::OneDigitAddition
);
generator!(
    ONE_DIGIT_SUBTRACTION_GENERATOR,
    ONE_DIGIT_SUBTRACTION_REGISTRATION,
    Mode::OneDigitSubtraction
);
generator!(
    TWO_DIGIT_ADDITION_GENERATOR,
    TWO_DIGIT_ADDITION_REGISTRATION,
    Mode::TwoDigitAddition
);
generator!(
    MULTIPLICATION_TABLE_GENERATOR,
    MULTIPLICATION_TABLE_REGISTRATION,
    Mode::MultiplicationTable(None)
);
generator!(
    DIVISION_1_GENERATOR,
    DIVISION_1_REGISTRATION,
    Mode::DivisionTable
);
generator!(
    ADDITION_UP_TO_10_GENERATOR,
    ADDITION_UP_TO_10_REGISTRATION,
    Mode::AdditionUpTo10
);
generator!(
    SUBTRACTION_UP_TO_10_GENERATOR,
    SUBTRACTION_UP_TO_10_REGISTRATION,
    Mode::SubtractionUpTo10
);
generator!(
    ADDITION_WITH_CARRY_GENERATOR,
    ADDITION_WITH_CARRY_REGISTRATION,
    Mode::AdditionWithCarry
);
generator!(
    SUBTRACTION_WITH_BORROW_GENERATOR,
    SUBTRACTION_WITH_BORROW_REGISTRATION,
    Mode::SubtractionWithBorrow
);
generator!(
    MULTIPLICATION_TABLE_1_GENERATOR,
    MULTIPLICATION_TABLE_1_REGISTRATION,
    Mode::MultiplicationTable(Some(1))
);
generator!(
    MULTIPLICATION_TABLE_2_GENERATOR,
    MULTIPLICATION_TABLE_2_REGISTRATION,
    Mode::MultiplicationTable(Some(2))
);
generator!(
    MULTIPLICATION_TABLE_3_GENERATOR,
    MULTIPLICATION_TABLE_3_REGISTRATION,
    Mode::MultiplicationTable(Some(3))
);
generator!(
    MULTIPLICATION_TABLE_4_GENERATOR,
    MULTIPLICATION_TABLE_4_REGISTRATION,
    Mode::MultiplicationTable(Some(4))
);
generator!(
    MULTIPLICATION_TABLE_5_GENERATOR,
    MULTIPLICATION_TABLE_5_REGISTRATION,
    Mode::MultiplicationTable(Some(5))
);
generator!(
    MULTIPLICATION_TABLE_6_GENERATOR,
    MULTIPLICATION_TABLE_6_REGISTRATION,
    Mode::MultiplicationTable(Some(6))
);
generator!(
    MULTIPLICATION_TABLE_7_GENERATOR,
    MULTIPLICATION_TABLE_7_REGISTRATION,
    Mode::MultiplicationTable(Some(7))
);
generator!(
    MULTIPLICATION_TABLE_8_GENERATOR,
    MULTIPLICATION_TABLE_8_REGISTRATION,
    Mode::MultiplicationTable(Some(8))
);
generator!(
    MULTIPLICATION_TABLE_9_GENERATOR,
    MULTIPLICATION_TABLE_9_REGISTRATION,
    Mode::MultiplicationTable(Some(9))
);
generator!(
    DIVISION_WITH_REMAINDER_GENERATOR,
    DIVISION_WITH_REMAINDER_REGISTRATION,
    Mode::DivisionWithRemainder
);
generator!(
    SIMPLE_TWO_DIGIT_DIVISION_GENERATOR,
    SIMPLE_TWO_DIGIT_DIVISION_REGISTRATION,
    Mode::SimpleTwoDigitDivision
);
generator!(
    SIGNED_ARITHMETIC_1_GENERATOR,
    SIGNED_ARITHMETIC_1_REGISTRATION,
    Mode::SignedAddSubtract
);
generator!(
    SIGNED_ARITHMETIC_2_GENERATOR,
    SIGNED_ARITHMETIC_2_REGISTRATION,
    Mode::SignedSummaryInteger
);
generator!(
    SIGNED_MULTIPLY_DIVIDE_GENERATOR,
    SIGNED_MULTIPLY_DIVIDE_REGISTRATION,
    Mode::SignedMultiplyDivide
);
generator!(
    SIGNED_ARITHMETIC_MIXED_OPERANDS_GENERATOR,
    SIGNED_ARITHMETIC_MIXED_OPERANDS_REGISTRATION,
    Mode::SignedSummaryMixedOperands
);

fn integer_arithmetic_problem(
    registration: &ThemeRegistration,
    id: u32,
    operator: ArithmeticOperator,
    left: i64,
    right: i64,
    answer_schema: AnswerSchema,
) -> Result<Problem, GenerationError> {
    let result = match operator {
        ArithmeticOperator::Add => left.checked_add(right),
        ArithmeticOperator::Subtract => left.checked_sub(right),
        ArithmeticOperator::Multiply => left.checked_mul(right),
        ArithmeticOperator::Divide => left.checked_div(right),
    }
    .ok_or(GenerationError::InvalidGeneratedProblem {
        reason: "finite integer arithmetic candidate overflowed or divided by zero",
    })?;
    let expression = binary_expression(
        operator,
        integer_expression(left),
        integer_expression(right),
    );
    let answer = AnswerNode::Integer(result);
    let plan = arithmetic_expression_plan(&expression, &answer).ok_or(
        GenerationError::InvalidGeneratedProblem {
            reason: "finite integer arithmetic candidate has no effort plan",
        },
    )?;
    Problem::generated(
        registration,
        id,
        ProblemPrompt::Arithmetic { expression },
        answer_schema,
        answer,
        EffortModel::operations(plan),
    )
    .map_err(GenerationError::from)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FiniteBasicDomain {
    AdditionUpTo10,
    SubtractionUpTo10,
    AdditionWithCarry,
    SubtractionWithBorrow,
    MultiplicationTable(u8),
    SimpleTwoDigitDivision,
}

impl FiniteBasicDomain {
    fn for_mode(mode: Mode) -> Option<Self> {
        match mode {
            Mode::AdditionUpTo10 => Some(Self::AdditionUpTo10),
            Mode::SubtractionUpTo10 => Some(Self::SubtractionUpTo10),
            Mode::AdditionWithCarry => Some(Self::AdditionWithCarry),
            Mode::SubtractionWithBorrow => Some(Self::SubtractionWithBorrow),
            Mode::MultiplicationTable(Some(row)) if (1..=9).contains(&row) => {
                Some(Self::MultiplicationTable(row))
            }
            Mode::SimpleTwoDigitDivision => Some(Self::SimpleTwoDigitDivision),
            _ => None,
        }
    }

    fn candidate_count(self) -> usize {
        match self {
            Self::AdditionUpTo10 | Self::SubtractionUpTo10 => triangular_number(9),
            Self::AdditionWithCarry | Self::SubtractionWithBorrow => triangular_number(8),
            Self::MultiplicationTable(_) => 9,
            Self::SimpleTwoDigitDivision => (2_i64..=9)
                .map(|divisor| {
                    let max_digit_quotient = 9 / divisor;
                    usize::try_from(max_digit_quotient * (max_digit_quotient + 1))
                        .expect("single-digit division domain size fits usize")
                })
                .sum(),
        }
    }

    fn problem_at(
        self,
        registration: &ThemeRegistration,
        index: usize,
    ) -> Option<Result<Problem, GenerationError>> {
        let id = u32::try_from(index.checked_add(1)?).unwrap_or(u32::MAX);
        match self {
            Self::MultiplicationTable(row) => {
                let right = i64::try_from(index.checked_add(1)?).ok()?;
                if right > 9 {
                    return None;
                }
                let left = i64::from(row);
                let answer_value = left.checked_mul(right)?;
                let expression = binary_expression(
                    ArithmeticOperator::Multiply,
                    integer_expression(left),
                    integer_expression(right),
                );
                let Some(effort_model) =
                    EffortModel::theme_specific(multiplication_table::effort(answer_value as u8))
                else {
                    return Some(Err(GenerationError::InvalidGeneratedProblem {
                        reason: "multiplication-table fact has no finite effort value",
                    }));
                };
                Some(
                    Problem::generated(
                        registration,
                        id,
                        ProblemPrompt::Arithmetic { expression },
                        AnswerSchema::Integer { min: 1, max: 81 },
                        AnswerNode::Integer(answer_value),
                        effort_model,
                    )
                    .map_err(GenerationError::from),
                )
            }
            Self::SimpleTwoDigitDivision => {
                let (row, offset) = row_position(
                    index,
                    (2_i64..=9).map(|divisor| {
                        let max_digit_quotient = 9 / divisor;
                        usize::try_from(max_digit_quotient * (max_digit_quotient + 1))
                            .expect("single-digit division row size fits usize")
                    }),
                )?;
                let divisor = i64::try_from(row.checked_add(2)?).ok()?;
                let max_digit_quotient = 9 / divisor;
                let ones_count = usize::try_from(max_digit_quotient + 1).ok()?;
                let tens_quotient = i64::try_from(offset / ones_count + 1).ok()?;
                let ones_quotient = i64::try_from(offset % ones_count).ok()?;
                let dividend = (divisor * tens_quotient) * 10 + divisor * ones_quotient;
                Some(integer_arithmetic_problem(
                    registration,
                    id,
                    ArithmeticOperator::Divide,
                    dividend,
                    divisor,
                    AnswerSchema::Integer { min: 10, max: 44 },
                ))
            }
            domain => {
                let (operator, answer_schema, left, right) =
                    domain.arithmetic_operands_at(index)?;
                Some(integer_arithmetic_problem(
                    registration,
                    id,
                    operator,
                    left,
                    right,
                    answer_schema,
                ))
            }
        }
    }

    fn arithmetic_operands_at(
        self,
        index: usize,
    ) -> Option<(ArithmeticOperator, AnswerSchema, i64, i64)> {
        match self {
            Self::AdditionUpTo10 => {
                let (row, offset) = row_position(index, (1_usize..=9).rev())?;
                let left = i64::try_from(row.checked_add(1)?).ok()?;
                let right = i64::try_from(offset.checked_add(1)?).ok()?;
                Some((
                    ArithmeticOperator::Add,
                    AnswerSchema::Integer { min: 2, max: 10 },
                    left,
                    right,
                ))
            }
            Self::SubtractionUpTo10 => {
                let (row, offset) = row_position(index, 1_usize..=9)?;
                let left = i64::try_from(row.checked_add(2)?).ok()?;
                let right = i64::try_from(offset.checked_add(1)?).ok()?;
                Some((
                    ArithmeticOperator::Subtract,
                    AnswerSchema::Integer { min: 1, max: 9 },
                    left,
                    right,
                ))
            }
            Self::AdditionWithCarry => {
                let (row, offset) = row_position(index, 1_usize..=8)?;
                let left = i64::try_from(row.checked_add(2)?).ok()?;
                let first_right = 11_i64.checked_sub(left)?;
                let right = first_right.checked_add(i64::try_from(offset).ok()?)?;
                Some((
                    ArithmeticOperator::Add,
                    AnswerSchema::Integer { min: 11, max: 18 },
                    left,
                    right,
                ))
            }
            Self::SubtractionWithBorrow => {
                let (row, offset) = row_position(index, 1_usize..=8)?;
                let right = i64::try_from(row.checked_add(2)?).ok()?;
                let first_result = 11_i64.checked_sub(right)?;
                let result = first_result.checked_add(i64::try_from(offset).ok()?)?;
                let left = right.checked_add(result)?;
                Some((
                    ArithmeticOperator::Subtract,
                    AnswerSchema::Integer { min: 1, max: 9 },
                    left,
                    right,
                ))
            }
            Self::MultiplicationTable(_) | Self::SimpleTwoDigitDivision => None,
        }
    }
}

const fn triangular_number(n: usize) -> usize {
    n * (n + 1) / 2
}

fn row_position(
    mut index: usize,
    row_lengths: impl Iterator<Item = usize>,
) -> Option<(usize, usize)> {
    for (row, row_len) in row_lengths.enumerate() {
        if index < row_len {
            return Some((row, index));
        }
        index -= row_len;
    }
    None
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SignedNonIntegerOperandKind {
    Decimal,
    Fraction,
}

fn signed_integer_leaves(
    rng: &mut DeterministicRng,
    leaf_count: usize,
    max_abs: i64,
) -> Option<Vec<ArithmeticExpression>> {
    let mut values = (0..leaf_count)
        .map(|_| draw_signed_integer(rng, max_abs))
        .collect::<Option<Vec<_>>>()?;
    ensure_negative_term(rng, &mut values)?;
    Some(values.into_iter().map(integer_expression).collect())
}

fn draw_signed_noninteger_leaf(
    rng: &mut DeterministicRng,
    kind: SignedNonIntegerOperandKind,
) -> Option<ArithmeticExpression> {
    let sign = if rng.next_bounded(2) == 0 {
        -1_i64
    } else {
        1_i64
    };
    match kind {
        SignedNonIntegerOperandKind::Decimal => {
            let scale = 1 + rng.next_bounded(2) as u32;
            let magnitude = 1 + rng.next_bounded(99) as i64;
            if scale == 1 && magnitude % 10 == 0 {
                return None;
            }
            Some(exact_decimal_expression(
                sign.checked_mul(magnitude)?,
                scale,
            ))
        }
        SignedNonIntegerOperandKind::Fraction => {
            let denominator = 2 + rng.next_bounded(8) as i64;
            let numerator = sign.checked_mul(1 + rng.next_bounded(9) as i64)?;
            let value = RationalCoefficient::new(numerator, denominator)?;
            (!value.is_integer()).then(|| rational_expression(value))
        }
    }
}

fn leaf_is_negative(expression: &ArithmeticExpression) -> bool {
    match expression {
        ArithmeticExpression::Integer { value } => *value < 0,
        ArithmeticExpression::Rational { value } => value.numerator() < 0,
        ArithmeticExpression::ExactDecimal { coefficient, .. } => *coefficient < 0,
        ArithmeticExpression::Binary { .. } => false,
    }
}

fn negate_leaf(expression: &mut ArithmeticExpression) -> Option<()> {
    match expression {
        ArithmeticExpression::Integer { value } => *value = value.checked_neg()?,
        ArithmeticExpression::Rational { value } => {
            *value =
                RationalCoefficient::new(value.numerator().checked_neg()?, value.denominator())?;
        }
        ArithmeticExpression::ExactDecimal { coefficient, .. } => {
            *coefficient = coefficient.checked_neg()?;
        }
        ArithmeticExpression::Binary { .. } => return None,
    }
    Some(())
}

fn signed_mixed_operand_leaves(
    rng: &mut DeterministicRng,
    leaf_count: usize,
    kind: SignedNonIntegerOperandKind,
) -> Option<Vec<ArithmeticExpression>> {
    let required_noninteger = rng.next_bounded(leaf_count as u64) as usize;
    let mut leaves = Vec::with_capacity(leaf_count);
    for index in 0..leaf_count {
        if index == required_noninteger || rng.next_bounded(3) == 0 {
            leaves.push(draw_signed_noninteger_leaf(rng, kind)?);
        } else {
            leaves.push(integer_expression(draw_signed_integer(rng, 9)?));
        }
    }
    if !leaves.iter().any(leaf_is_negative) {
        let index = rng.next_bounded(leaf_count as u64) as usize;
        negate_leaf(&mut leaves[index])?;
    }
    Some(leaves)
}

fn draw_problem(
    registration: &ThemeRegistration,
    mode: Mode,
    rng: &mut DeterministicRng,
    id: u32,
    weights: &OperationWeights,
) -> Option<Result<Problem, GenerationError>> {
    if mode_matches_addition(mode) {
        let (left, right) = rng.next_ordered_pair();
        return Some(one_digit_addition_problem(id, left, right, weights));
    }

    let (expression, answer, effort_model, answer_schema) = match mode {
        Mode::OneDigitAddition => unreachable!(),
        Mode::OneDigitSubtraction => {
            let b = 1_i64 + rng.next_bounded(9) as i64;
            let c = 1_i64 + rng.next_bounded(9) as i64;
            let a = b + c;
            let expression = binary_expression(
                ArithmeticOperator::Subtract,
                integer_expression(a),
                integer_expression(b),
            );
            (
                expression,
                AnswerNode::Integer(c),
                EffortModel::operations(one_digit_subtraction_plan(a as u8, b as u8)?),
                AnswerSchema::Integer { min: 1, max: 9 },
            )
        }
        Mode::TwoDigitAddition => {
            let a = 10_i64 + rng.next_bounded(90) as i64;
            let b = 10_i64 + rng.next_bounded(90) as i64;
            let c = a + b;
            let expression = binary_expression(
                ArithmeticOperator::Add,
                integer_expression(a),
                integer_expression(b),
            );
            (
                expression,
                AnswerNode::Integer(c),
                EffortModel::operations(two_digit_addition_plan(a as u8, b as u8)?),
                AnswerSchema::Integer { min: 20, max: 198 },
            )
        }
        Mode::AdditionUpTo10 => {
            let left = 1_i64 + rng.next_bounded(9) as i64;
            let right = 1_i64 + rng.next_bounded(u64::try_from(10 - left).ok()?) as i64;
            let result = left + right;
            let expression = binary_expression(
                ArithmeticOperator::Add,
                integer_expression(left),
                integer_expression(right),
            );
            let answer = AnswerNode::Integer(result);
            let plan = arithmetic_expression_plan(&expression, &answer)?;
            (
                expression,
                answer,
                EffortModel::operations(plan),
                AnswerSchema::Integer { min: 2, max: 10 },
            )
        }
        Mode::SubtractionUpTo10 => {
            let left = 2_i64 + rng.next_bounded(9) as i64;
            let right = 1_i64 + rng.next_bounded(u64::try_from(left - 1).ok()?) as i64;
            let result = left - right;
            let expression = binary_expression(
                ArithmeticOperator::Subtract,
                integer_expression(left),
                integer_expression(right),
            );
            let answer = AnswerNode::Integer(result);
            let plan = arithmetic_expression_plan(&expression, &answer)?;
            (
                expression,
                answer,
                EffortModel::operations(plan),
                AnswerSchema::Integer { min: 1, max: 9 },
            )
        }
        Mode::AdditionWithCarry => {
            let left = 1_i64 + rng.next_bounded(9) as i64;
            let right = 1_i64 + rng.next_bounded(9) as i64;
            let result = left + right;
            if result <= 10 {
                return None;
            }
            let expression = binary_expression(
                ArithmeticOperator::Add,
                integer_expression(left),
                integer_expression(right),
            );
            let answer = AnswerNode::Integer(result);
            let plan = arithmetic_expression_plan(&expression, &answer)?;
            (
                expression,
                answer,
                EffortModel::operations(plan),
                AnswerSchema::Integer { min: 11, max: 18 },
            )
        }
        Mode::SubtractionWithBorrow => {
            let right = 2_i64 + rng.next_bounded(8) as i64;
            let result = 1_i64 + rng.next_bounded(9) as i64;
            let left = right + result;
            if left <= 10 {
                return None;
            }
            let expression = binary_expression(
                ArithmeticOperator::Subtract,
                integer_expression(left),
                integer_expression(right),
            );
            let answer = AnswerNode::Integer(result);
            let plan = arithmetic_expression_plan(&expression, &answer)?;
            (
                expression,
                answer,
                EffortModel::operations(plan),
                AnswerSchema::Integer { min: 1, max: 9 },
            )
        }
        Mode::MultiplicationTable(fixed_left) => {
            let a = i64::from(fixed_left.unwrap_or_else(|| 1 + rng.next_bounded(9) as u8));
            let b = 1_i64 + rng.next_bounded(9) as i64;
            let c = a * b;
            let expression = binary_expression(
                ArithmeticOperator::Multiply,
                integer_expression(a),
                integer_expression(b),
            );
            (
                expression,
                AnswerNode::Integer(c),
                EffortModel::theme_specific(multiplication_table::effort(c as u8))?,
                AnswerSchema::Integer { min: 1, max: 81 },
            )
        }
        Mode::DivisionTable => {
            let divisor = 1_i64 + rng.next_bounded(9) as i64;
            let quotient = 1_i64 + rng.next_bounded(9) as i64;
            let dividend = divisor * quotient;
            let expression = binary_expression(
                ArithmeticOperator::Divide,
                integer_expression(dividend),
                integer_expression(divisor),
            );
            (
                expression,
                AnswerNode::Integer(quotient),
                EffortModel::operations(division_table::operation_plan(dividend as u8)),
                AnswerSchema::Integer { min: 1, max: 9 },
            )
        }
        Mode::DivisionWithRemainder => {
            let divisor = 2_i64 + rng.next_bounded(8) as i64;
            let quotient = 1_i64 + rng.next_bounded(9) as i64;
            let remainder = 1_i64 + rng.next_bounded(u64::try_from(divisor - 1).ok()?) as i64;
            let dividend = divisor.checked_mul(quotient)?.checked_add(remainder)?;
            let expression = binary_expression(
                ArithmeticOperator::Divide,
                integer_expression(dividend),
                integer_expression(divisor),
            );
            let answer = AnswerNode::Tuple(vec![
                AnswerNode::Integer(quotient),
                AnswerNode::Integer(remainder),
            ]);
            let plan = integer_division_with_remainder_plan(dividend, divisor, &answer)?;
            (
                expression,
                answer,
                EffortModel::operations(plan),
                AnswerSchema::OrderedPair,
            )
        }
        Mode::SimpleTwoDigitDivision => {
            let divisor = 2_i64 + rng.next_bounded(8) as i64;
            let max_digit_quotient = 9 / divisor;
            let tens_quotient =
                1_i64 + rng.next_bounded(u64::try_from(max_digit_quotient).ok()?) as i64;
            let ones_quotient =
                rng.next_bounded(u64::try_from(max_digit_quotient + 1).ok()?) as i64;
            let dividend = (divisor * tens_quotient) * 10 + divisor * ones_quotient;
            let quotient = tens_quotient * 10 + ones_quotient;
            let expression = binary_expression(
                ArithmeticOperator::Divide,
                integer_expression(dividend),
                integer_expression(divisor),
            );
            let answer = AnswerNode::Integer(quotient);
            let plan = arithmetic_expression_plan(&expression, &answer)?;
            (
                expression,
                answer,
                EffortModel::operations(plan),
                AnswerSchema::Integer { min: 10, max: 44 },
            )
        }
        Mode::SignedAddSubtract => {
            let term_count = 2 + rng.next_bounded(3) as usize;
            let mut terms = (0..term_count)
                .map(|_| draw_signed_integer(rng, 15))
                .collect::<Option<Vec<_>>>()?;
            ensure_negative_term(rng, &mut terms)?;
            let mut expression = integer_expression(terms[0]);
            for term in terms.iter().skip(1) {
                let operator = SIGNED_ADD_SUBTRACT_OPERATORS
                    [rng.next_bounded(SIGNED_ADD_SUBTRACT_OPERATORS.len() as u64) as usize];
                expression = binary_expression(operator, expression, integer_expression(*term));
            }
            let value = evaluate_expression(&expression)?;
            if !value.is_integer() {
                return None;
            }
            let answer = AnswerNode::Integer(value.numerator());
            let plan = arithmetic_expression_plan(&expression, &answer)?;
            (
                expression,
                answer,
                EffortModel::operations(plan),
                AnswerSchema::Integer { min: -60, max: 60 },
            )
        }
        Mode::SignedMultiplyDivide => {
            let leaf_count = 2 + rng.next_bounded(3) as usize;
            let leaves = signed_integer_leaves(rng, leaf_count, 9)?;
            let expression =
                draw_bounded_arithmetic_ast(rng, &leaves, &SIGNED_MULTIPLY_DIVIDE_OPERATORS)?;
            let value = evaluate_expression(&expression)?;
            if value.numerator().unsigned_abs() > 200 || value.denominator() > 36 {
                return None;
            }
            let answer = rational_answer(value);
            let plan = arithmetic_expression_plan(&expression, &answer)?;
            (
                expression,
                answer,
                EffortModel::operations(plan),
                AnswerSchema::Rational {
                    max_abs_numerator: 200,
                    max_denominator: 36,
                    require_reduced_fraction_form: true,
                },
            )
        }
        Mode::SignedSummaryInteger => {
            let leaf_count = 2 + rng.next_bounded(3) as usize;
            let leaves = signed_integer_leaves(rng, leaf_count, 9)?;
            let expression = draw_bounded_arithmetic_ast(rng, &leaves, &SIGNED_FOUR_OPERATORS)?;
            let value = evaluate_expression(&expression)?;
            if value.numerator().unsigned_abs() > 200 || value.denominator() > 36 {
                return None;
            }
            let answer = rational_answer(value);
            let plan = arithmetic_expression_plan(&expression, &answer)?;
            (
                expression,
                answer,
                EffortModel::operations(plan),
                AnswerSchema::Rational {
                    max_abs_numerator: 200,
                    max_denominator: 36,
                    require_reduced_fraction_form: true,
                },
            )
        }
        Mode::SignedSummaryMixedOperands => {
            let leaf_count = 2 + rng.next_bounded(3) as usize;
            let kind = if rng.next_bounded(2) == 0 {
                SignedNonIntegerOperandKind::Decimal
            } else {
                SignedNonIntegerOperandKind::Fraction
            };
            let leaves = signed_mixed_operand_leaves(rng, leaf_count, kind)?;
            let expression = draw_bounded_arithmetic_ast(rng, &leaves, &SIGNED_FOUR_OPERATORS)?;
            let value = evaluate_expression(&expression)?;
            if value.numerator().unsigned_abs() > 200 || value.denominator() > 36 {
                return None;
            }
            let answer = rational_answer(value);
            let plan = arithmetic_expression_plan(&expression, &answer)?;
            (
                expression,
                answer,
                EffortModel::operations(plan),
                AnswerSchema::Rational {
                    max_abs_numerator: 200,
                    max_denominator: 36,
                    require_reduced_fraction_form: true,
                },
            )
        }
    };
    Some(
        Problem::generated(
            registration,
            id,
            ProblemPrompt::Arithmetic { expression },
            answer_schema,
            answer,
            effort_model,
        )
        .map_err(GenerationError::from),
    )
}

pub(crate) fn one_digit_addition_problem(
    id: u32,
    left: u8,
    right: u8,
    _weights: &OperationWeights,
) -> Result<Problem, GenerationError> {
    if !(MIN_OPERAND..=MAX_OPERAND).contains(&left) || !(MIN_OPERAND..=MAX_OPERAND).contains(&right)
    {
        return Err(GenerationError::InvalidGeneratedProblem {
            reason:
                "one-digit addition generator received an operand outside its registered domain",
        });
    }
    let answer = left + right;
    let effort_model = EffortModel::operations(one_digit_addition_plan(left, right).ok_or(
        GenerationError::InvalidGeneratedProblem {
            reason: "one-digit addition effort operands are outside the modeled domain",
        },
    )?);
    Problem::generated(
        &ONE_DIGIT_ADDITION_REGISTRATION,
        id,
        ProblemPrompt::Addition { left, right },
        AnswerSchema::Integer {
            min: i64::from(MIN_ANSWER),
            max: i64::from(MAX_ANSWER),
        },
        AnswerNode::Integer(i64::from(answer)),
        effort_model,
    )
    .map_err(GenerationError::from)
}

const fn mode_matches_addition(mode: Mode) -> bool {
    matches!(mode, Mode::OneDigitAddition)
}

/// Current generators owned by this theme family.
pub(crate) static GENERATORS: [GeneratorEntry; 24] = [
    GeneratorEntry::current(&ONE_DIGIT_ADDITION_GENERATOR),
    GeneratorEntry::current(&ONE_DIGIT_SUBTRACTION_GENERATOR),
    GeneratorEntry::current(&TWO_DIGIT_ADDITION_GENERATOR),
    GeneratorEntry::current(&MULTIPLICATION_TABLE_GENERATOR),
    GeneratorEntry::current(&SIGNED_ARITHMETIC_1_GENERATOR),
    GeneratorEntry::current(&SIGNED_MULTIPLY_DIVIDE_GENERATOR),
    GeneratorEntry::current(&SIGNED_ARITHMETIC_2_GENERATOR),
    GeneratorEntry::current(&SIGNED_ARITHMETIC_MIXED_OPERANDS_GENERATOR),
    GeneratorEntry::current(&DIVISION_1_GENERATOR),
    GeneratorEntry::current(&ADDITION_UP_TO_10_GENERATOR),
    GeneratorEntry::current(&SUBTRACTION_UP_TO_10_GENERATOR),
    GeneratorEntry::current(&ADDITION_WITH_CARRY_GENERATOR),
    GeneratorEntry::current(&SUBTRACTION_WITH_BORROW_GENERATOR),
    GeneratorEntry::current(&MULTIPLICATION_TABLE_1_GENERATOR),
    GeneratorEntry::current(&MULTIPLICATION_TABLE_2_GENERATOR),
    GeneratorEntry::current(&MULTIPLICATION_TABLE_3_GENERATOR),
    GeneratorEntry::current(&MULTIPLICATION_TABLE_4_GENERATOR),
    GeneratorEntry::current(&MULTIPLICATION_TABLE_5_GENERATOR),
    GeneratorEntry::current(&MULTIPLICATION_TABLE_6_GENERATOR),
    GeneratorEntry::current(&MULTIPLICATION_TABLE_7_GENERATOR),
    GeneratorEntry::current(&MULTIPLICATION_TABLE_8_GENERATOR),
    GeneratorEntry::current(&MULTIPLICATION_TABLE_9_GENERATOR),
    GeneratorEntry::current(&DIVISION_WITH_REMAINDER_GENERATOR),
    GeneratorEntry::current(&SIMPLE_TWO_DIGIT_DIVISION_GENERATOR),
];

#[cfg(test)]
mod tests {
    use super::*;

    fn draw_accepted(
        registration: &'static ThemeRegistration,
        mode: Mode,
        rng: &mut DeterministicRng,
        ordinal: u32,
    ) -> Problem {
        let weights = OperationWeights::default();
        for _ in 0..10_000 {
            if let Some(problem) = draw_problem(registration, mode, rng, ordinal, &weights) {
                return problem.expect("generated candidate must satisfy its problem contract");
            }
        }
        panic!("local basic-arithmetic generator failed to produce a candidate");
    }

    fn binary_integer_operands(problem: &Problem) -> (ArithmeticOperator, i64, i64) {
        let ProblemPrompt::Arithmetic { expression } = problem.prompt() else {
            panic!("expected arithmetic prompt");
        };
        let crate::model::ArithmeticExpression::Binary {
            operator,
            left,
            right,
        } = expression
        else {
            panic!("expected binary expression");
        };
        let (
            crate::model::ArithmeticExpression::Integer { value: left },
            crate::model::ArithmeticExpression::Integer { value: right },
        ) = (&**left, &**right)
        else {
            panic!("expected integer operands");
        };
        (*operator, *left, *right)
    }

    #[test]
    fn finite_basic_domains_are_indexable_without_materializing_problem_vectors() {
        let cases = [
            (&ADDITION_UP_TO_10_REGISTRATION, Mode::AdditionUpTo10),
            (&SUBTRACTION_UP_TO_10_REGISTRATION, Mode::SubtractionUpTo10),
            (&ADDITION_WITH_CARRY_REGISTRATION, Mode::AdditionWithCarry),
            (
                &SUBTRACTION_WITH_BORROW_REGISTRATION,
                Mode::SubtractionWithBorrow,
            ),
            (
                &MULTIPLICATION_TABLE_7_REGISTRATION,
                Mode::MultiplicationTable(Some(7)),
            ),
            (
                &SIMPLE_TWO_DIGIT_DIVISION_REGISTRATION,
                Mode::SimpleTwoDigitDivision,
            ),
        ];
        for (registration, mode) in cases {
            let domain =
                FiniteBasicDomain::for_mode(mode).expect("finite mode must declare a domain");
            let count = domain.candidate_count();
            assert!(count > 0);
            let mut prompts = std::collections::BTreeSet::new();
            for index in 0..count {
                let problem = domain
                    .problem_at(registration, index)
                    .expect("declared finite index must exist")
                    .expect("finite candidate must satisfy the problem contract");
                assert!(prompts.insert(problem.prompt().clone()));
            }
            assert_eq!(prompts.len(), count);
            assert!(domain.problem_at(registration, count).is_none());
        }
    }

    #[test]
    fn grade1_dedicated_themes_fix_the_intended_carry_domains() {
        let mut rng = DeterministicRng::from_seed("Grade1DedicatedA1");
        let cases = [
            (&ADDITION_UP_TO_10_REGISTRATION, Mode::AdditionUpTo10),
            (&SUBTRACTION_UP_TO_10_REGISTRATION, Mode::SubtractionUpTo10),
            (&ADDITION_WITH_CARRY_REGISTRATION, Mode::AdditionWithCarry),
            (
                &SUBTRACTION_WITH_BORROW_REGISTRATION,
                Mode::SubtractionWithBorrow,
            ),
        ];
        for (registration, mode) in cases {
            for ordinal in 1..=256 {
                let problem = draw_accepted(registration, mode, &mut rng, ordinal);
                let (operator, left, right) = binary_integer_operands(&problem);
                match mode {
                    Mode::AdditionUpTo10 => {
                        assert_eq!(operator, ArithmeticOperator::Add);
                        assert!((1..=9).contains(&left) && (1..=9).contains(&right));
                        assert!(left + right <= 10);
                    }
                    Mode::SubtractionUpTo10 => {
                        assert_eq!(operator, ArithmeticOperator::Subtract);
                        assert!((2..=10).contains(&left));
                        assert!((1..left).contains(&right));
                    }
                    Mode::AdditionWithCarry => {
                        assert_eq!(operator, ArithmeticOperator::Add);
                        assert!((1..=9).contains(&left) && (1..=9).contains(&right));
                        assert!(left + right >= 11);
                    }
                    Mode::SubtractionWithBorrow => {
                        assert_eq!(operator, ArithmeticOperator::Subtract);
                        assert!((11..=18).contains(&left));
                        assert!((2..=9).contains(&right));
                        assert!(left % 10 < right);
                    }
                    _ => unreachable!(),
                }
            }
        }
    }

    #[test]
    fn multiplication_table_rows_fix_exactly_one_left_factor() {
        let mut rng = DeterministicRng::from_seed("MultiplicationRowsB2");
        let rows = [
            (&MULTIPLICATION_TABLE_1_REGISTRATION, 1),
            (&MULTIPLICATION_TABLE_2_REGISTRATION, 2),
            (&MULTIPLICATION_TABLE_3_REGISTRATION, 3),
            (&MULTIPLICATION_TABLE_4_REGISTRATION, 4),
            (&MULTIPLICATION_TABLE_5_REGISTRATION, 5),
            (&MULTIPLICATION_TABLE_6_REGISTRATION, 6),
            (&MULTIPLICATION_TABLE_7_REGISTRATION, 7),
            (&MULTIPLICATION_TABLE_8_REGISTRATION, 8),
            (&MULTIPLICATION_TABLE_9_REGISTRATION, 9),
        ];
        for (registration, row) in rows {
            for ordinal in 1..=64 {
                let problem = draw_accepted(
                    registration,
                    Mode::MultiplicationTable(Some(row)),
                    &mut rng,
                    ordinal,
                );
                let (operator, left, right) = binary_integer_operands(&problem);
                assert_eq!(operator, ArithmeticOperator::Multiply);
                assert_eq!(left, i64::from(row));
                assert!((1..=9).contains(&right));
            }
        }
        assert_eq!(
            MULTIPLICATION_TABLE_REGISTRATION.curriculum_unit(),
            CURRICULUM_UNIT_MULTIPLICATION_TABLE
        );
        for registration in rows.map(|(registration, _)| registration) {
            assert_eq!(
                registration.curriculum_unit(),
                CURRICULUM_UNIT_MULTIPLICATION_TABLE
            );
        }
    }

    #[test]
    fn grade3_division_themes_keep_quotient_and_decomposition_invariants() {
        let mut rng = DeterministicRng::from_seed("Grade3DivisionC3");
        for ordinal in 1..=256 {
            let exact = draw_accepted(
                &DIVISION_1_REGISTRATION,
                Mode::DivisionTable,
                &mut rng,
                ordinal,
            );
            let (operator, dividend, divisor) = binary_integer_operands(&exact);
            assert_eq!(operator, ArithmeticOperator::Divide);
            let quotient = exact.canonical_answer().as_integer().unwrap();
            assert!((1..=9).contains(&quotient));
            assert_eq!(dividend, divisor * quotient);

            let remainder_problem = draw_accepted(
                &DIVISION_WITH_REMAINDER_REGISTRATION,
                Mode::DivisionWithRemainder,
                &mut rng,
                ordinal,
            );
            let (_, dividend, divisor) = binary_integer_operands(&remainder_problem);
            let AnswerNode::Tuple(parts) = remainder_problem.canonical_answer() else {
                panic!("remainder theme must answer with quotient and remainder");
            };
            let quotient = parts[0].as_integer().unwrap();
            let remainder = parts[1].as_integer().unwrap();
            assert!((1..=9).contains(&quotient));
            assert!(0 < remainder && remainder < divisor);
            assert_eq!(dividend, divisor * quotient + remainder);

            let simple = draw_accepted(
                &SIMPLE_TWO_DIGIT_DIVISION_REGISTRATION,
                Mode::SimpleTwoDigitDivision,
                &mut rng,
                ordinal,
            );
            let (_, dividend, divisor) = binary_integer_operands(&simple);
            assert!((10..=99).contains(&dividend));
            assert!((2..=9).contains(&divisor));
            assert_eq!(
                (dividend / 10) % divisor,
                0,
                "tens digit must divide independently"
            );
            assert_eq!(
                (dividend % 10) % divisor,
                0,
                "ones digit must divide independently"
            );
            let quotient = simple.canonical_answer().as_integer().unwrap();
            assert!((10..=44).contains(&quotient));
            assert_eq!(dividend, divisor * quotient);
        }
    }
    #[derive(Default)]
    struct SignedExpressionFacts {
        operators: Vec<ArithmeticOperator>,
        integer_leaves: usize,
        decimal_leaves: usize,
        fraction_leaves: usize,
        has_negative_leaf: bool,
    }

    fn collect_signed_expression_facts(
        expression: &ArithmeticExpression,
        facts: &mut SignedExpressionFacts,
    ) {
        match expression {
            ArithmeticExpression::Integer { value } => {
                facts.integer_leaves += 1;
                facts.has_negative_leaf |= *value < 0;
            }
            ArithmeticExpression::ExactDecimal { coefficient, .. } => {
                facts.decimal_leaves += 1;
                facts.has_negative_leaf |= *coefficient < 0;
            }
            ArithmeticExpression::Rational { value } => {
                facts.fraction_leaves += 1;
                facts.has_negative_leaf |= value.numerator() < 0;
            }
            ArithmeticExpression::Binary {
                operator,
                left,
                right,
            } => {
                facts.operators.push(*operator);
                collect_signed_expression_facts(left, facts);
                collect_signed_expression_facts(right, facts);
            }
        }
    }

    #[test]
    fn signed_number_themes_fix_operator_and_operand_domains() {
        use crate::generator::generate_worksheet_request;
        use crate::model::GenerateWorksheetRequest;
        use crate::schema::SCHEMA_VERSION;

        let cases = [
            (
                THEME_ID_SIGNED_ARITHMETIC_1,
                &SIGNED_ADD_SUBTRACT_OPERATORS[..],
                false,
            ),
            (
                THEME_ID_SIGNED_MULTIPLY_DIVIDE,
                &SIGNED_MULTIPLY_DIVIDE_OPERATORS[..],
                false,
            ),
            (
                THEME_ID_SIGNED_ARITHMETIC_2,
                &SIGNED_FOUR_OPERATORS[..],
                false,
            ),
            (
                THEME_ID_SIGNED_ARITHMETIC_MIXED_OPERANDS,
                &SIGNED_FOUR_OPERATORS[..],
                true,
            ),
        ];
        let mut saw_decimal_operand = false;
        let mut saw_fraction_operand = false;

        for (theme_id, allowed_operators, requires_noninteger_operand) in cases {
            let registration = crate::registry::active_registration(theme_id)
                .unwrap()
                .unwrap();
            assert_eq!(
                registration.curriculum_unit(),
                CURRICULUM_UNIT_SIGNED_NUMBERS
            );
            for difficulty in 1..=4 {
                for seed in ["SnA1", "SnB2", "SnC3"] {
                    let worksheet = generate_worksheet_request(&GenerateWorksheetRequest {
                        schema_version: SCHEMA_VERSION,
                        numeric_theme_id: theme_id,
                        seed: seed.to_owned(),
                        difficulty: crate::identity::Difficulty::try_from(difficulty).unwrap(),
                        timeout_ms: Some(1_000),
                        max_attempts: Some(50_000),
                    })
                    .unwrap_or_else(|error| {
                        panic!("signed theme {theme_id} d{difficulty} failed for {seed}: {error}")
                    });

                    for problem in worksheet.problems() {
                        let ProblemPrompt::Arithmetic { expression } = problem.prompt() else {
                            panic!("signed-number theme returned non-arithmetic prompt");
                        };
                        let mut facts = SignedExpressionFacts::default();
                        collect_signed_expression_facts(expression, &mut facts);
                        assert!(
                            facts.has_negative_leaf,
                            "theme {theme_id} must contain a negative operand"
                        );
                        assert!(!facts.operators.is_empty());
                        assert!(facts
                            .operators
                            .iter()
                            .all(|operator| allowed_operators.contains(operator)));
                        if requires_noninteger_operand {
                            assert!(facts.decimal_leaves + facts.fraction_leaves > 0);
                            // Each problem stays in one exact representation family so the existing
                            // decimal/fraction effort semantics remain authoritative.
                            assert!(facts.decimal_leaves == 0 || facts.fraction_leaves == 0);
                            saw_decimal_operand |= facts.decimal_leaves > 0;
                            saw_fraction_operand |= facts.fraction_leaves > 0;
                        } else {
                            assert_eq!(facts.decimal_leaves, 0);
                            assert_eq!(facts.fraction_leaves, 0);
                            assert!(facts.integer_leaves >= 2);
                        }
                    }
                }
            }
        }

        assert!(
            saw_decimal_operand,
            "summary(2) support must contain decimal literal operands"
        );
        assert!(
            saw_fraction_operand,
            "summary(2) support must contain fraction literal operands"
        );
    }
}
