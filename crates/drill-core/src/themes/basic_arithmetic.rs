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
    binary_expression, draw_bounded_rational_arithmetic_ast, draw_signed_integer,
    ensure_negative_term, evaluate_expression, integer_expression, rational_answer,
};
use crate::model::{AnswerSchema, ArithmeticOperator, Problem, ProblemPrompt};
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
pub const GENERATOR_REVISION_DIVISION_1: u32 = 3;
pub const GENERATOR_REVISION_ADDITION_UP_TO_10: u32 = 1;
pub const GENERATOR_REVISION_SUBTRACTION_UP_TO_10: u32 = 1;
pub const GENERATOR_REVISION_ADDITION_WITH_CARRY: u32 = 1;
pub const GENERATOR_REVISION_SUBTRACTION_WITH_BORROW: u32 = 1;
pub const GENERATOR_REVISION_MULTIPLICATION_TABLE_ROW: u32 = 1;
pub const GENERATOR_REVISION_DIVISION_WITH_REMAINDER: u32 = 1;
pub const GENERATOR_REVISION_SIMPLE_TWO_DIGIT_DIVISION: u32 = 1;
pub const SKILL_ID: &str = "jp.grade1.addition.one_digit";
pub const SKILL_ID_ONE_DIGIT_SUBTRACTION: &str = "jp.grade1.subtraction.one_digit";
pub const SKILL_ID_TWO_DIGIT_ADDITION: &str = "jp.grade2.addition.two_digit";
pub const SKILL_ID_MULTIPLICATION_TABLE: &str = "jp.grade2.multiplication.table";
pub const SKILL_ID_SIGNED_ARITHMETIC_1: &str = "jp.grade7.signed.arithmetic.1";
pub const SKILL_ID_SIGNED_ARITHMETIC_2: &str = "jp.grade7.signed.arithmetic.2";
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
pub const CURRICULUM_PATH: [&str; 4] = ["root", "小学1年生", "足し算", "一桁の足し算（まとめ）"];
pub const CURRICULUM_PATH_ONE_DIGIT_SUBTRACTION: [&str; 4] =
    ["root", "小学1年生", "引き算", "一桁の引き算（まとめ）"];
pub const CURRICULUM_PATH_TWO_DIGIT_ADDITION: [&str; 3] = ["root", "小学2年生", "二桁の足し算"];
pub const CURRICULUM_PATH_MULTIPLICATION_TABLE: [&str; 4] =
    ["root", "小学2年生", "九九", "全段混合"];
pub const CURRICULUM_PATH_SIGNED_ARITHMETIC_1: [&str; 3] = ["root", "中学1年生", "負の数の計算(1)"];
pub const CURRICULUM_PATH_SIGNED_ARITHMETIC_2: [&str; 3] = ["root", "中学1年生", "負の数の計算(2)"];
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
    SignedArithmetic1,
    SignedArithmetic2,
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
        if matches!(
            self.mode,
            Mode::AdditionUpTo10
                | Mode::SubtractionUpTo10
                | Mode::AdditionWithCarry
                | Mode::SubtractionWithBorrow
                | Mode::MultiplicationTable(Some(_))
                | Mode::SimpleTwoDigitDivision
        ) {
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
    fn candidates(&self, _weights: &OperationWeights) -> Result<Vec<Problem>, GenerationError> {
        finite_basic_candidates(self.registration, self.mode)
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
    Mode::SignedArithmetic1
);
generator!(
    SIGNED_ARITHMETIC_2_GENERATOR,
    SIGNED_ARITHMETIC_2_REGISTRATION,
    Mode::SignedArithmetic2
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

fn finite_basic_candidates(
    registration: &ThemeRegistration,
    mode: Mode,
) -> Result<Vec<Problem>, GenerationError> {
    let mut operands = Vec::new();
    if matches!(mode, Mode::SimpleTwoDigitDivision) {
        let mut candidates = Vec::new();
        for divisor in 2_i64..=9 {
            let max_digit_quotient = 9 / divisor;
            for tens_quotient in 1_i64..=max_digit_quotient {
                for ones_quotient in 0_i64..=max_digit_quotient {
                    let dividend = (divisor * tens_quotient) * 10 + divisor * ones_quotient;
                    candidates.push(integer_arithmetic_problem(
                        registration,
                        u32::try_from(candidates.len() + 1).unwrap_or(u32::MAX),
                        ArithmeticOperator::Divide,
                        dividend,
                        divisor,
                        AnswerSchema::Integer { min: 10, max: 44 },
                    )?);
                }
            }
        }
        return Ok(candidates);
    }

    if let Mode::MultiplicationTable(Some(row)) = mode {
        return (1_i64..=9)
            .enumerate()
            .map(|(index, right)| {
                let left = i64::from(row);
                let answer_value = left * right;
                let expression = binary_expression(
                    ArithmeticOperator::Multiply,
                    integer_expression(left),
                    integer_expression(right),
                );
                Problem::generated(
                    registration,
                    u32::try_from(index + 1).unwrap_or(u32::MAX),
                    ProblemPrompt::Arithmetic { expression },
                    AnswerSchema::Integer { min: 1, max: 81 },
                    AnswerNode::Integer(answer_value),
                    EffortModel::theme_specific(multiplication_table::effort(answer_value as u8))
                        .ok_or(GenerationError::InvalidGeneratedProblem {
                        reason: "multiplication-table fact has no finite effort value",
                    })?,
                )
                .map_err(GenerationError::from)
            })
            .collect();
    }

    let (operator, answer_schema) = match mode {
        Mode::AdditionUpTo10 => {
            for left in 1_i64..=9 {
                for right in 1_i64..=(10 - left) {
                    operands.push((left, right));
                }
            }
            (
                ArithmeticOperator::Add,
                AnswerSchema::Integer { min: 2, max: 10 },
            )
        }
        Mode::SubtractionUpTo10 => {
            for left in 2_i64..=10 {
                for right in 1_i64..left {
                    operands.push((left, right));
                }
            }
            (
                ArithmeticOperator::Subtract,
                AnswerSchema::Integer { min: 1, max: 9 },
            )
        }
        Mode::AdditionWithCarry => {
            for left in 1_i64..=9 {
                for right in 1_i64..=9 {
                    if left + right > 10 {
                        operands.push((left, right));
                    }
                }
            }
            (
                ArithmeticOperator::Add,
                AnswerSchema::Integer { min: 11, max: 18 },
            )
        }
        Mode::SubtractionWithBorrow => {
            for right in 2_i64..=9 {
                for result in 1_i64..=9 {
                    let left = right + result;
                    if left > 10 {
                        operands.push((left, right));
                    }
                }
            }
            (
                ArithmeticOperator::Subtract,
                AnswerSchema::Integer { min: 1, max: 9 },
            )
        }
        _ => unreachable!("finite candidate enumeration is only used by grade-1 focused themes"),
    };

    operands
        .into_iter()
        .enumerate()
        .map(|(index, (left, right))| {
            integer_arithmetic_problem(
                registration,
                u32::try_from(index + 1).unwrap_or(u32::MAX),
                operator,
                left,
                right,
                answer_schema.clone(),
            )
        })
        .collect()
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
        Mode::SignedArithmetic1 => {
            let term_count = 2 + rng.next_bounded(3) as usize;
            let mut terms = (0..term_count)
                .map(|_| draw_signed_integer(rng, 15))
                .collect::<Option<Vec<_>>>()?;
            ensure_negative_term(rng, &mut terms)?;
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
            let answer = AnswerNode::Integer(value.numerator());
            let plan = arithmetic_expression_plan(&expression, &answer)?;
            (
                expression,
                answer,
                EffortModel::operations(plan),
                AnswerSchema::Integer { min: -60, max: 60 },
            )
        }
        Mode::SignedArithmetic2 => {
            let leaf_count = 2 + rng.next_bounded(3) as usize;
            let mut values = (0..leaf_count)
                .map(|_| draw_signed_integer(rng, 9))
                .collect::<Option<Vec<_>>>()?;
            ensure_negative_term(rng, &mut values)?;
            let expression = draw_bounded_rational_arithmetic_ast(rng, &values)?;
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
pub(crate) static GENERATORS: [GeneratorEntry; 22] = [
    GeneratorEntry::current(&ONE_DIGIT_ADDITION_GENERATOR),
    GeneratorEntry::current(&ONE_DIGIT_SUBTRACTION_GENERATOR),
    GeneratorEntry::current(&TWO_DIGIT_ADDITION_GENERATOR),
    GeneratorEntry::current(&MULTIPLICATION_TABLE_GENERATOR),
    GeneratorEntry::current(&SIGNED_ARITHMETIC_1_GENERATOR),
    GeneratorEntry::current(&SIGNED_ARITHMETIC_2_GENERATOR),
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
}
