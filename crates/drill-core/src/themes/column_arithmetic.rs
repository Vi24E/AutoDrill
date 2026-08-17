use crate::answer::AnswerNode;
use crate::effort::{
    arithmetic_expression_graph, calculate_graph_effort, integer_division_with_remainder_graph,
    OperationWeights,
};
use crate::generator::{GeneratorEntry, ProblemGenerator};
use crate::generator_support::{
    arithmetic_leaf_column_grid_cells, arithmetic_leaf_significant_digits, binary_expression,
    draw_decimal_coefficient, draw_decimal_operand, exact_decimal_expression,
    exact_decimal_rational, input_interface, integer_expression, rational_less_than,
    rational_to_arithmetic_expression, rational_to_exact_decimal_answer,
};
use crate::model::{
    AnswerInputInterface, AnswerSchema, ArithmeticExpression, ArithmeticOperator,
    ColumnMultiplicationPartial, EditorStructure, LongDivisionStep, Problem, ProblemPrompt,
    RationalCoefficient, WorkedSolution,
};
use crate::rng::DeterministicRng;
use crate::schema::SCHEMA_VERSION;
use crate::theme::{
    CurriculumSafetyPolicy as Safety, DedupPolicy as Dedup, SamplingLayerSpec,
    ThemeAnswerContract as AnswerContract, ThemeAnswerSchemaKind as Schema,
    ThemeInputProfile as Input, ThemePresentationPolicy as Presentation, ThemePromptKind as Prompt,
    ThemeRegistration, ThemeRegistrationSpec, ThemeTag, COLUMN_16_LAYOUT,
    COLUMN_DIVISION_12_LAYOUT,
};

pub const THEME_ID_COLUMN_ADD_2DIGIT: u32 = 25;
pub const THEME_ID_COLUMN_SUBTRACT_2DIGIT: u32 = 26;
pub const THEME_ID_COLUMN_ADD_3_4DIGIT: u32 = 27;
pub const THEME_ID_COLUMN_SUBTRACT_3_4DIGIT: u32 = 28;
pub const THEME_ID_COLUMN_MULTIPLY_1DIGIT: u32 = 29;
pub const THEME_ID_COLUMN_MULTIPLY_2DIGIT: u32 = 30;
pub const THEME_ID_COLUMN_DIVIDE_1DIGIT: u32 = 31;
pub const THEME_ID_COLUMN_DIVIDE_2DIGIT: u32 = 32;
pub const THEME_ID_COLUMN_DECIMAL_ADD_SUBTRACT: u32 = 33;
pub const THEME_ID_COLUMN_DECIMAL_MULTIPLY_INTEGER: u32 = 34;
pub const THEME_ID_COLUMN_DECIMAL_DIVIDE_INTEGER: u32 = 35;
pub const THEME_ID_COLUMN_DECIMAL_MULTIPLICATION: u32 = 36;
pub const THEME_ID_COLUMN_DECIMAL_DIVISION: u32 = 37;
pub const GENERATOR_REVISION_COLUMN_ADD_2DIGIT: u32 = 2;
pub const GENERATOR_REVISION_COLUMN_SUBTRACT_2DIGIT: u32 = 2;
pub const GENERATOR_REVISION_COLUMN_ADD_3_4DIGIT: u32 = 2;
pub const GENERATOR_REVISION_COLUMN_SUBTRACT_3_4DIGIT: u32 = 2;
pub const GENERATOR_REVISION_COLUMN_MULTIPLY_1DIGIT: u32 = 2;
pub const GENERATOR_REVISION_COLUMN_MULTIPLY_2DIGIT: u32 = 2;
pub const GENERATOR_REVISION_COLUMN_DIVIDE_1DIGIT: u32 = 2;
pub const GENERATOR_REVISION_COLUMN_DIVIDE_2DIGIT: u32 = 2;
pub const GENERATOR_REVISION_COLUMN_DECIMAL_ADD_SUBTRACT: u32 = 2;
pub const GENERATOR_REVISION_COLUMN_DECIMAL_MULTIPLY_INTEGER: u32 = 2;
pub const GENERATOR_REVISION_COLUMN_DECIMAL_DIVIDE_INTEGER: u32 = 2;
pub const GENERATOR_REVISION_COLUMN_DECIMAL_MULTIPLICATION: u32 = 2;
pub const GENERATOR_REVISION_COLUMN_DECIMAL_DIVISION: u32 = 2;
pub const SKILL_ID_COLUMN_ADD_2DIGIT: &str = "jp.grade2.column.addition.two_digit";
pub const SKILL_ID_COLUMN_SUBTRACT_2DIGIT: &str = "jp.grade2.column.subtraction.two_digit";
pub const SKILL_ID_COLUMN_ADD_3_4DIGIT: &str = "jp.grade3.column.addition.three_four_digit";
pub const SKILL_ID_COLUMN_SUBTRACT_3_4DIGIT: &str = "jp.grade3.column.subtraction.three_four_digit";
pub const SKILL_ID_COLUMN_MULTIPLY_1DIGIT: &str =
    "jp.grade3.column.multiplication.one_digit_multiplier";
pub const SKILL_ID_COLUMN_MULTIPLY_2DIGIT: &str =
    "jp.grade3.column.multiplication.two_digit_multiplier";
pub const SKILL_ID_COLUMN_DIVIDE_1DIGIT: &str = "jp.grade3.column.division.one_digit_divisor";
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
pub const CURRICULUM_PATH_COLUMN_DIVIDE_1DIGIT: [&str; 4] =
    ["root", "小学3年生", "除法", "一桁で割る割り算の筆算"];
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

const ADDITION: &[ThemeTag] = &[
    ThemeTag::Addition,
    ThemeTag::ColumnArithmetic,
    ThemeTag::PrintRecommended,
];
const SUBTRACTION: &[ThemeTag] = &[
    ThemeTag::Subtraction,
    ThemeTag::ColumnArithmetic,
    ThemeTag::PrintRecommended,
];
const MULTIPLICATION: &[ThemeTag] = &[
    ThemeTag::Multiplication,
    ThemeTag::ColumnArithmetic,
    ThemeTag::PrintRecommended,
];
const DIVISION: &[ThemeTag] = &[
    ThemeTag::Division,
    ThemeTag::ColumnArithmetic,
    ThemeTag::PrintRecommended,
];
const DECIMAL_ADD_SUBTRACT: &[ThemeTag] = &[
    ThemeTag::Decimals,
    ThemeTag::Addition,
    ThemeTag::Subtraction,
    ThemeTag::ColumnArithmetic,
    ThemeTag::PrintRecommended,
];
const DECIMAL_MULTIPLICATION: &[ThemeTag] = &[
    ThemeTag::Decimals,
    ThemeTag::Multiplication,
    ThemeTag::ColumnArithmetic,
    ThemeTag::PrintRecommended,
];
const DECIMAL_DIVISION: &[ThemeTag] = &[
    ThemeTag::Decimals,
    ThemeTag::Division,
    ThemeTag::ColumnArithmetic,
    ThemeTag::PrintRecommended,
];

pub const DECIMAL_ADD_SUBTRACT_LAYERS: [SamplingLayerSpec; 2] = [
    SamplingLayerSpec {
        key: "addition",
        weight: 1,
        minimum: 0,
    },
    SamplingLayerSpec {
        key: "subtraction",
        weight: 1,
        minimum: 0,
    },
];

const INTEGER_COLUMN: AnswerContract = AnswerContract {
    prompt_kind: Prompt::ColumnArithmetic,
    answer_schema_kind: Schema::Integer,
    input_profile: Input::SimplePositive,
};
const INTEGER_DIVISION_COLUMN: AnswerContract = AnswerContract {
    prompt_kind: Prompt::ColumnArithmetic,
    answer_schema_kind: Schema::OrderedPair,
    input_profile: Input::TupleOnly,
};
const DECIMAL_COLUMN: AnswerContract = AnswerContract {
    prompt_kind: Prompt::ColumnArithmetic,
    answer_schema_kind: Schema::Decimal,
    input_profile: Input::SimpleDecimal,
};

pub const COLUMN_ADD_2DIGIT_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: THEME_ID_COLUMN_ADD_2DIGIT,
        generator_revision: GENERATOR_REVISION_COLUMN_ADD_2DIGIT,
        skill_id: SKILL_ID_COLUMN_ADD_2DIGIT,
        curriculum_path: &CURRICULUM_PATH_COLUMN_ADD_2DIGIT,
        grade: Some(2),
        tags: ADDITION,
        safety: Safety::NonNegativeOnly,
        presentation: Presentation::COLUMN_ARITHMETIC,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: INTEGER_COLUMN,
        layout: COLUMN_16_LAYOUT,
    });

pub const COLUMN_SUBTRACT_2DIGIT_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: THEME_ID_COLUMN_SUBTRACT_2DIGIT,
        generator_revision: GENERATOR_REVISION_COLUMN_SUBTRACT_2DIGIT,
        skill_id: SKILL_ID_COLUMN_SUBTRACT_2DIGIT,
        curriculum_path: &CURRICULUM_PATH_COLUMN_SUBTRACT_2DIGIT,
        grade: Some(2),
        tags: SUBTRACTION,
        safety: Safety::NonNegativeOnly,
        presentation: Presentation::COLUMN_ARITHMETIC,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: INTEGER_COLUMN,
        layout: COLUMN_16_LAYOUT,
    });

pub const COLUMN_ADD_3_4DIGIT_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: THEME_ID_COLUMN_ADD_3_4DIGIT,
        generator_revision: GENERATOR_REVISION_COLUMN_ADD_3_4DIGIT,
        skill_id: SKILL_ID_COLUMN_ADD_3_4DIGIT,
        curriculum_path: &CURRICULUM_PATH_COLUMN_ADD_3_4DIGIT,
        grade: Some(3),
        tags: ADDITION,
        safety: Safety::NonNegativeOnly,
        presentation: Presentation::COLUMN_ARITHMETIC,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: INTEGER_COLUMN,
        layout: COLUMN_16_LAYOUT,
    });

pub const COLUMN_SUBTRACT_3_4DIGIT_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: THEME_ID_COLUMN_SUBTRACT_3_4DIGIT,
        generator_revision: GENERATOR_REVISION_COLUMN_SUBTRACT_3_4DIGIT,
        skill_id: SKILL_ID_COLUMN_SUBTRACT_3_4DIGIT,
        curriculum_path: &CURRICULUM_PATH_COLUMN_SUBTRACT_3_4DIGIT,
        grade: Some(3),
        tags: SUBTRACTION,
        safety: Safety::NonNegativeOnly,
        presentation: Presentation::COLUMN_ARITHMETIC,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: INTEGER_COLUMN,
        layout: COLUMN_16_LAYOUT,
    });

pub const COLUMN_MULTIPLY_1DIGIT_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: THEME_ID_COLUMN_MULTIPLY_1DIGIT,
        generator_revision: GENERATOR_REVISION_COLUMN_MULTIPLY_1DIGIT,
        skill_id: SKILL_ID_COLUMN_MULTIPLY_1DIGIT,
        curriculum_path: &CURRICULUM_PATH_COLUMN_MULTIPLY_1DIGIT,
        grade: Some(3),
        tags: MULTIPLICATION,
        safety: Safety::NonNegativeOnly,
        presentation: Presentation::COLUMN_ARITHMETIC,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: INTEGER_COLUMN,
        layout: COLUMN_16_LAYOUT,
    });

pub const COLUMN_MULTIPLY_2DIGIT_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: THEME_ID_COLUMN_MULTIPLY_2DIGIT,
        generator_revision: GENERATOR_REVISION_COLUMN_MULTIPLY_2DIGIT,
        skill_id: SKILL_ID_COLUMN_MULTIPLY_2DIGIT,
        curriculum_path: &CURRICULUM_PATH_COLUMN_MULTIPLY_2DIGIT,
        grade: Some(3),
        tags: MULTIPLICATION,
        safety: Safety::NonNegativeOnly,
        presentation: Presentation::COLUMN_ARITHMETIC,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: INTEGER_COLUMN,
        layout: COLUMN_16_LAYOUT,
    });

pub const COLUMN_DIVIDE_1DIGIT_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: THEME_ID_COLUMN_DIVIDE_1DIGIT,
        generator_revision: GENERATOR_REVISION_COLUMN_DIVIDE_1DIGIT,
        skill_id: SKILL_ID_COLUMN_DIVIDE_1DIGIT,
        curriculum_path: &CURRICULUM_PATH_COLUMN_DIVIDE_1DIGIT,
        grade: Some(3),
        tags: DIVISION,
        safety: Safety::NonNegativeOnly,
        presentation: Presentation::COLUMN_ARITHMETIC,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: INTEGER_DIVISION_COLUMN,
        layout: COLUMN_DIVISION_12_LAYOUT,
    });

pub const COLUMN_DIVIDE_2DIGIT_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: THEME_ID_COLUMN_DIVIDE_2DIGIT,
        generator_revision: GENERATOR_REVISION_COLUMN_DIVIDE_2DIGIT,
        skill_id: SKILL_ID_COLUMN_DIVIDE_2DIGIT,
        curriculum_path: &CURRICULUM_PATH_COLUMN_DIVIDE_2DIGIT,
        grade: Some(4),
        tags: DIVISION,
        safety: Safety::NonNegativeOnly,
        presentation: Presentation::COLUMN_ARITHMETIC,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: INTEGER_DIVISION_COLUMN,
        layout: COLUMN_DIVISION_12_LAYOUT,
    });

pub const COLUMN_DECIMAL_ADD_SUBTRACT_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: THEME_ID_COLUMN_DECIMAL_ADD_SUBTRACT,
        generator_revision: GENERATOR_REVISION_COLUMN_DECIMAL_ADD_SUBTRACT,
        skill_id: SKILL_ID_COLUMN_DECIMAL_ADD_SUBTRACT,
        curriculum_path: &CURRICULUM_PATH_COLUMN_DECIMAL_ADD_SUBTRACT,
        grade: Some(4),
        tags: DECIMAL_ADD_SUBTRACT,
        safety: Safety::NonNegativeOnly,
        presentation: Presentation::COLUMN_ARITHMETIC,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: DECIMAL_COLUMN,
        layout: COLUMN_16_LAYOUT,
    });

pub const COLUMN_DECIMAL_MULTIPLY_INTEGER_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: THEME_ID_COLUMN_DECIMAL_MULTIPLY_INTEGER,
        generator_revision: GENERATOR_REVISION_COLUMN_DECIMAL_MULTIPLY_INTEGER,
        skill_id: SKILL_ID_COLUMN_DECIMAL_MULTIPLY_INTEGER,
        curriculum_path: &CURRICULUM_PATH_COLUMN_DECIMAL_MULTIPLY_INTEGER,
        grade: Some(4),
        tags: DECIMAL_MULTIPLICATION,
        safety: Safety::NonNegativeOnly,
        presentation: Presentation::COLUMN_ARITHMETIC,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: DECIMAL_COLUMN,
        layout: COLUMN_16_LAYOUT,
    });

pub const COLUMN_DECIMAL_DIVIDE_INTEGER_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: THEME_ID_COLUMN_DECIMAL_DIVIDE_INTEGER,
        generator_revision: GENERATOR_REVISION_COLUMN_DECIMAL_DIVIDE_INTEGER,
        skill_id: SKILL_ID_COLUMN_DECIMAL_DIVIDE_INTEGER,
        curriculum_path: &CURRICULUM_PATH_COLUMN_DECIMAL_DIVIDE_INTEGER,
        grade: Some(4),
        tags: DECIMAL_DIVISION,
        safety: Safety::NonNegativeOnly,
        presentation: Presentation::COLUMN_ARITHMETIC,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: DECIMAL_COLUMN,
        layout: COLUMN_DIVISION_12_LAYOUT,
    });

pub const COLUMN_DECIMAL_MULTIPLICATION_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: THEME_ID_COLUMN_DECIMAL_MULTIPLICATION,
        generator_revision: GENERATOR_REVISION_COLUMN_DECIMAL_MULTIPLICATION,
        skill_id: SKILL_ID_COLUMN_DECIMAL_MULTIPLICATION,
        curriculum_path: &CURRICULUM_PATH_COLUMN_DECIMAL_MULTIPLICATION,
        grade: Some(5),
        tags: DECIMAL_MULTIPLICATION,
        safety: Safety::NonNegativeOnly,
        presentation: Presentation::COLUMN_ARITHMETIC,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: DECIMAL_COLUMN,
        layout: COLUMN_16_LAYOUT,
    });

pub const COLUMN_DECIMAL_DIVISION_REGISTRATION: ThemeRegistration =
    ThemeRegistration::new(ThemeRegistrationSpec {
        numeric_theme_id: THEME_ID_COLUMN_DECIMAL_DIVISION,
        generator_revision: GENERATOR_REVISION_COLUMN_DECIMAL_DIVISION,
        skill_id: SKILL_ID_COLUMN_DECIMAL_DIVISION,
        curriculum_path: &CURRICULUM_PATH_COLUMN_DECIMAL_DIVISION,
        grade: Some(5),
        tags: DECIMAL_DIVISION,
        safety: Safety::NonNegativeOnly,
        presentation: Presentation::COLUMN_ARITHMETIC,
        dedup: Dedup::CanonicalizeCommutative,
        answer_contract: DECIMAL_COLUMN,
        layout: COLUMN_DIVISION_12_LAYOUT,
    });

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Mode {
    AddTwoDigit,
    SubtractTwoDigit,
    AddThreeFourDigit,
    SubtractThreeFourDigit,
    MultiplyOneDigit,
    MultiplyTwoDigit,
    DivideOneDigit,
    DivideTwoDigit,
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

    fn sampling_layers(&self) -> Option<&'static [SamplingLayerSpec]> {
        (self.mode == Mode::DecimalAddSubtract).then_some(&DECIMAL_ADD_SUBTRACT_LAYERS)
    }

    fn sampling_layer(&self, problem: &Problem) -> Option<usize> {
        if self.mode != Mode::DecimalAddSubtract {
            return None;
        }
        let ProblemPrompt::ColumnArithmetic { operator, .. } = &problem.prompt else {
            return None;
        };
        match operator {
            ArithmeticOperator::Add => Some(0),
            ArithmeticOperator::Subtract => Some(1),
            _ => None,
        }
    }

    fn draw_candidate(
        &self,
        rng: &mut DeterministicRng,
        ordinal: u32,
        weights: &OperationWeights,
    ) -> Option<Problem> {
        draw_problem(self.registration, self.mode, rng, ordinal, weights)
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
    DIVIDE_1DIGIT_GENERATOR,
    COLUMN_DIVIDE_1DIGIT_REGISTRATION,
    DivideOneDigit
);
generator!(
    DIVIDE_2DIGIT_GENERATOR,
    COLUMN_DIVIDE_2DIGIT_REGISTRATION,
    DivideTwoDigit
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

fn simple_integer_input(allow_negative: bool) -> AnswerInputInterface {
    input_interface(if allow_negative {
        Input::SimpleSigned
    } else {
        Input::SimplePositive
    })
}

fn simple_decimal_input() -> AnswerInputInterface {
    input_interface(Input::SimpleDecimal)
}

fn leaf_scaled_integer(expression: &ArithmeticExpression) -> Option<(i64, u32)> {
    match expression {
        ArithmeticExpression::Integer { value } => Some((*value, 0)),
        ArithmeticExpression::ExactDecimal { coefficient, scale } => Some((*coefficient, *scale)),
        _ => None,
    }
}

fn quotient_answer(answer: &AnswerNode) -> &AnswerNode {
    match answer {
        AnswerNode::Tuple(values) => values.first().unwrap_or(answer),
        _ => answer,
    }
}

fn answer_scale(answer: &AnswerNode) -> u32 {
    match answer {
        AnswerNode::ExactDecimal { scale, .. } => *scale,
        _ => 0,
    }
}

fn build_worked_solution(
    operator: ArithmeticOperator,
    left: &ArithmeticExpression,
    right: &ArithmeticExpression,
    answer: &AnswerNode,
) -> Option<WorkedSolution> {
    match operator {
        ArithmeticOperator::Multiply => {
            let (left_coefficient, _) = leaf_scaled_integer(left)?;
            let (right_coefficient, _) = leaf_scaled_integer(right)?;
            let multiplicand = left_coefficient.unsigned_abs();
            let multiplier_digits = right_coefficient.unsigned_abs().to_string();
            let partial_products = multiplier_digits
                .bytes()
                .rev()
                .enumerate()
                .map(|(place, digit)| {
                    let digit = u64::from(digit - b'0');
                    let value = multiplicand.checked_mul(digit)?;
                    Some(ColumnMultiplicationPartial {
                        value: i64::try_from(value).ok()?,
                        place: u32::try_from(place).ok()?,
                    })
                })
                .collect::<Option<Vec<_>>>()?;
            Some(WorkedSolution::ColumnMultiplication { partial_products })
        }
        ArithmeticOperator::Divide => {
            let (mut normalized_dividend_coefficient, left_scale) = leaf_scaled_integer(left)?;
            let (right_coefficient, right_scale) = leaf_scaled_integer(right)?;
            let normalized_dividend_scale = if right_scale <= left_scale {
                left_scale - right_scale
            } else {
                normalized_dividend_coefficient = normalized_dividend_coefficient
                    .checked_mul(10_i64.checked_pow(right_scale - left_scale)?)?;
                0
            };
            let divisor = i64::try_from(right_coefficient.unsigned_abs()).ok()?;
            if divisor == 0 {
                return None;
            }
            let quotient_scale = answer_scale(quotient_answer(answer));
            let target_scale = normalized_dividend_scale.max(quotient_scale);
            let appended_zeros = target_scale.checked_sub(normalized_dividend_scale)?;
            let dividend_magnitude = normalized_dividend_coefficient.unsigned_abs();
            let base_digits = format!(
                "{:0width$}",
                dividend_magnitude,
                width = normalized_dividend_scale as usize + 1
            );
            let mut digits = base_digits;
            digits.extend(std::iter::repeat_n('0', appended_zeros as usize));
            let dividend_coefficient =
                i64::try_from(dividend_magnitude.checked_mul(10_u64.checked_pow(appended_zeros)?)?)
                    .ok()?;

            let mut steps = Vec::new();
            let mut current = 0_i64;
            let mut started = false;
            let digit_bytes = digits.as_bytes();
            for (index, byte) in digit_bytes.iter().enumerate() {
                let digit = i64::from(byte - b'0');
                current = current.checked_mul(10)?.checked_add(digit)?;
                let quotient_digit = current / divisor;
                let has_more = index + 1 < digit_bytes.len();
                if !started && quotient_digit == 0 && has_more {
                    continue;
                }
                started = true;
                let product = quotient_digit.checked_mul(divisor)?;
                let remainder = current.checked_sub(product)?;
                let product_offset = u32::try_from(digit_bytes.len() - index - 1).ok()?;
                let after = if has_more {
                    remainder
                        .checked_mul(10)?
                        .checked_add(i64::from(digit_bytes[index + 1] - b'0'))?
                } else {
                    remainder
                };
                let after_offset = if has_more {
                    product_offset.saturating_sub(1)
                } else {
                    product_offset
                };
                steps.push(LongDivisionStep {
                    product,
                    after,
                    product_offset,
                    after_offset,
                });
                current = remainder;
            }
            Some(WorkedSolution::LongDivision {
                divisor,
                dividend_coefficient,
                dividend_scale: target_scale,
                quotient_trailing_cells: target_scale.saturating_sub(quotient_scale),
                steps,
            })
        }
        ArithmeticOperator::Add | ArithmeticOperator::Subtract => None,
    }
}

fn draw_integer_with_digits(rng: &mut DeterministicRng, digits: u32) -> i64 {
    debug_assert!(digits >= 1);
    let lower = if digits == 1 {
        1
    } else {
        10_i64.pow(digits - 1)
    };
    let upper = 10_i64.pow(digits) - 1;
    lower + rng.next_bounded((upper - lower + 1) as u64) as i64
}

fn draw_three_or_four_digit_integer(rng: &mut DeterministicRng) -> i64 {
    let digits = if rng.next_bounded(2) == 0 { 3 } else { 4 };
    draw_integer_with_digits(rng, digits)
}

fn draw_column_remainder(rng: &mut DeterministicRng, divisor: i64) -> i64 {
    debug_assert!(divisor >= 2);
    if rng.next_bounded(2) == 0 {
        0
    } else {
        1 + rng.next_bounded((divisor - 1) as u64) as i64
    }
}

// Current column-arithmetic candidate rules. Pre-release history lives in Git;
// production registers only the current generator for each theme.
fn draw_problem(
    registration: &ThemeRegistration,
    mode: Mode,
    rng: &mut DeterministicRng,
    id: u32,
    weights: &OperationWeights,
) -> Option<Problem> {
    let (operator, left, right, answer, solution_graph, input_interface, answer_schema) = match mode
    {
        Mode::AddTwoDigit => {
            let left_value = draw_integer_with_digits(rng, 2);
            let right_value = draw_integer_with_digits(rng, 2);
            let answer = AnswerNode::Integer(left_value.checked_add(right_value)?);
            let left = integer_expression(left_value);
            let right = integer_expression(right_value);
            let expression =
                binary_expression(ArithmeticOperator::Add, left.clone(), right.clone());
            let graph = arithmetic_expression_graph(&expression, &answer)?;
            (
                ArithmeticOperator::Add,
                left,
                right,
                answer,
                graph,
                simple_integer_input(false),
                AnswerSchema::Integer { min: 20, max: 198 },
            )
        }
        Mode::SubtractTwoDigit => {
            let first = draw_integer_with_digits(rng, 2);
            let second = draw_integer_with_digits(rng, 2);
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
            let graph = arithmetic_expression_graph(&expression, &answer)?;
            (
                ArithmeticOperator::Subtract,
                left,
                right,
                answer,
                graph,
                simple_integer_input(false),
                AnswerSchema::Integer { min: 0, max: 89 },
            )
        }
        Mode::AddThreeFourDigit => {
            let left_value = draw_three_or_four_digit_integer(rng);
            let right_value = draw_three_or_four_digit_integer(rng);
            let answer = AnswerNode::Integer(left_value.checked_add(right_value)?);
            let left = integer_expression(left_value);
            let right = integer_expression(right_value);
            let expression =
                binary_expression(ArithmeticOperator::Add, left.clone(), right.clone());
            let graph = arithmetic_expression_graph(&expression, &answer)?;
            (
                ArithmeticOperator::Add,
                left,
                right,
                answer,
                graph,
                simple_integer_input(false),
                AnswerSchema::Integer {
                    min: 200,
                    max: 19_998,
                },
            )
        }
        Mode::SubtractThreeFourDigit => {
            let first = draw_three_or_four_digit_integer(rng);
            let second = draw_three_or_four_digit_integer(rng);
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
            let graph = arithmetic_expression_graph(&expression, &answer)?;
            (
                ArithmeticOperator::Subtract,
                left,
                right,
                answer,
                graph,
                simple_integer_input(false),
                AnswerSchema::Integer { min: 0, max: 9_899 },
            )
        }
        Mode::MultiplyOneDigit => {
            let multiplicand = if rng.next_bounded(2) == 0 {
                draw_integer_with_digits(rng, 2)
            } else {
                draw_integer_with_digits(rng, 3)
            };
            let multiplier = 2 + rng.next_bounded(8) as i64;
            let answer = AnswerNode::Integer(multiplicand.checked_mul(multiplier)?);
            let left = integer_expression(multiplicand);
            let right = integer_expression(multiplier);
            let expression =
                binary_expression(ArithmeticOperator::Multiply, left.clone(), right.clone());
            let graph = arithmetic_expression_graph(&expression, &answer)?;
            (
                ArithmeticOperator::Multiply,
                left,
                right,
                answer,
                graph,
                simple_integer_input(false),
                AnswerSchema::Integer {
                    min: 20,
                    max: 8_991,
                },
            )
        }
        Mode::MultiplyTwoDigit => {
            let multiplicand = if rng.next_bounded(2) == 0 {
                draw_integer_with_digits(rng, 2)
            } else {
                draw_integer_with_digits(rng, 3)
            };
            let multiplier = draw_integer_with_digits(rng, 2);
            let answer = AnswerNode::Integer(multiplicand.checked_mul(multiplier)?);
            let left = integer_expression(multiplicand);
            let right = integer_expression(multiplier);
            let expression =
                binary_expression(ArithmeticOperator::Multiply, left.clone(), right.clone());
            let graph = arithmetic_expression_graph(&expression, &answer)?;
            (
                ArithmeticOperator::Multiply,
                left,
                right,
                answer,
                graph,
                simple_integer_input(false),
                AnswerSchema::Integer {
                    min: 100,
                    max: 98_901,
                },
            )
        }
        Mode::DivideOneDigit | Mode::DivideTwoDigit => {
            let divisor = if mode == Mode::DivideOneDigit {
                2 + rng.next_bounded(8) as i64
            } else {
                draw_integer_with_digits(rng, 2)
            };
            let quotient = if mode == Mode::DivideOneDigit {
                draw_integer_with_digits(rng, 2)
            } else {
                2 + rng.next_bounded(98) as i64
            };
            let remainder = draw_column_remainder(rng, divisor);
            let dividend = divisor.checked_mul(quotient)?.checked_add(remainder)?;
            let answer = AnswerNode::Tuple(vec![
                AnswerNode::Integer(quotient),
                AnswerNode::Integer(remainder),
            ]);
            let graph = integer_division_with_remainder_graph(dividend, divisor, &answer);
            (
                ArithmeticOperator::Divide,
                integer_expression(dividend),
                integer_expression(divisor),
                answer,
                graph,
                AnswerInputInterface::StructuredMath {
                    allowed_structures: vec![EditorStructure::Tuple],
                },
                AnswerSchema::OrderedPair,
            )
        }
        Mode::DecimalAddSubtract => {
            let (mut left_coefficient, mut left_scale) = draw_decimal_operand(rng, 3, 3);
            let (mut right_coefficient, mut right_scale) = draw_decimal_operand(rng, 3, 3);
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
            let graph = arithmetic_expression_graph(&expression, &answer)?;
            (
                operator,
                left,
                right,
                answer,
                graph,
                simple_decimal_input(),
                AnswerSchema::Decimal { max_scale: 3 },
            )
        }
        Mode::DecimalMultiplyInteger => {
            let (coefficient, scale) = draw_decimal_operand(rng, 3, 2);
            let integer = 2 + rng.next_bounded(8) as i64;
            let left_value = exact_decimal_rational(coefficient, scale)?;
            let result = left_value.multiply(RationalCoefficient::new(integer, 1)?)?;
            let answer = rational_to_exact_decimal_answer(result, 3)?;
            let left = exact_decimal_expression(coefficient, scale);
            let right = integer_expression(integer);
            let expression =
                binary_expression(ArithmeticOperator::Multiply, left.clone(), right.clone());
            let graph = arithmetic_expression_graph(&expression, &answer)?;
            (
                ArithmeticOperator::Multiply,
                left,
                right,
                answer,
                graph,
                simple_decimal_input(),
                AnswerSchema::Decimal { max_scale: 3 },
            )
        }
        Mode::DecimalDivideInteger => {
            let (quotient_coefficient, quotient_scale) = draw_decimal_operand(rng, 3, 2);
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
            let graph = arithmetic_expression_graph(&expression, &answer)?;
            (
                ArithmeticOperator::Divide,
                left,
                right,
                answer,
                graph,
                simple_decimal_input(),
                AnswerSchema::Decimal { max_scale: 2 },
            )
        }
        Mode::DecimalMultiplication => {
            let (left_coefficient, left_scale) = draw_decimal_operand(rng, 2, 2);
            let (right_coefficient, right_scale) = draw_decimal_operand(rng, 2, 2);
            let left_value = exact_decimal_rational(left_coefficient, left_scale)?;
            let right_value = exact_decimal_rational(right_coefficient, right_scale)?;
            let result = left_value.multiply(right_value)?;
            let answer = rational_to_exact_decimal_answer(result, 4)?;
            let left = exact_decimal_expression(left_coefficient, left_scale);
            let right = exact_decimal_expression(right_coefficient, right_scale);
            let expression =
                binary_expression(ArithmeticOperator::Multiply, left.clone(), right.clone());
            let graph = arithmetic_expression_graph(&expression, &answer)?;
            (
                ArithmeticOperator::Multiply,
                left,
                right,
                answer,
                graph,
                simple_decimal_input(),
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
            let divisor_coefficient = draw_decimal_coefficient(rng, 2);
            let divisor = exact_decimal_rational(divisor_coefficient, divisor_scale)?;
            let (quotient, answer) = if divisor_scale == 1 {
                let quotient_coefficient = draw_decimal_coefficient(rng, 2);
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
            let graph = arithmetic_expression_graph(&expression, &answer)?;
            (
                ArithmeticOperator::Divide,
                left,
                right,
                answer,
                graph,
                simple_decimal_input(),
                AnswerSchema::Decimal { max_scale: 2 },
            )
        }
    };

    let effort = calculate_graph_effort(&solution_graph, weights);
    let worked_solution = build_worked_solution(operator, &left, &right, &answer);
    Some(Problem {
        schema_version: SCHEMA_VERSION,
        id,
        numeric_theme_id: registration.numeric_theme_id,
        prompt: ProblemPrompt::ColumnArithmetic {
            operator,
            left,
            right,
        },
        input_interface,
        answer_schema,
        canonical_answer: answer,
        worked_solution,
        solution_graph,
        operation_vector: effort.operation_vector,
        effort: effort.value,
    })
}

/// Current generators owned by this theme family.
pub(crate) static GENERATORS: [GeneratorEntry; 13] = [
    GeneratorEntry::current(&ADD_2DIGIT_GENERATOR),
    GeneratorEntry::current(&SUBTRACT_2DIGIT_GENERATOR),
    GeneratorEntry::current(&ADD_3_4DIGIT_GENERATOR),
    GeneratorEntry::current(&SUBTRACT_3_4DIGIT_GENERATOR),
    GeneratorEntry::current(&MULTIPLY_1DIGIT_GENERATOR),
    GeneratorEntry::current(&MULTIPLY_2DIGIT_GENERATOR),
    GeneratorEntry::current(&DIVIDE_1DIGIT_GENERATOR),
    GeneratorEntry::current(&DIVIDE_2DIGIT_GENERATOR),
    GeneratorEntry::current(&DECIMAL_ADD_SUBTRACT_GENERATOR),
    GeneratorEntry::current(&DECIMAL_MULTIPLY_INTEGER_GENERATOR),
    GeneratorEntry::current(&DECIMAL_DIVIDE_INTEGER_GENERATOR),
    GeneratorEntry::current(&DECIMAL_MULTIPLICATION_GENERATOR),
    GeneratorEntry::current(&DECIMAL_DIVISION_GENERATOR),
];
