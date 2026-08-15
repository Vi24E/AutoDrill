//! Versioned domain values shared by the native engine and the WASM adapter.

use serde::{Deserialize, Serialize};

use crate::answer::AnswerNode;
use crate::effort::{OperationVector, SolutionGraph};
use crate::identity::{Difficulty, ProblemSetIdentity};

pub const SCHEMA_VERSION: u16 = 4;
pub const THEME_ID_ONE_DIGIT_ADDITION: u32 = 1;
pub const THEME_ID_LINEAR_EQUATION_1: u32 = 2;
pub const THEME_ID_LINEAR_EQUATION_2: u32 = 3;
pub const THEME_ID_ONE_DIGIT_SUBTRACTION: u32 = 4;
pub const THEME_ID_TWO_DIGIT_ADDITION: u32 = 5;
pub const THEME_ID_MULTIPLICATION_TABLE: u32 = 6;
pub const THEME_ID_SIGNED_ARITHMETIC_1: u32 = 7;
pub const THEME_ID_SIGNED_ARITHMETIC_2: u32 = 8;
pub const THEME_ID_FRACTION_ADDITION: u32 = 9;
pub const THEME_ID_FRACTION_MULTIPLICATION: u32 = 10;
pub const THEME_ID_FRACTION_SUBTRACTION: u32 = 11;
pub const THEME_ID_FRACTION_DIVISION: u32 = 12;
pub const THEME_ID_DIVISION_1: u32 = 13;
pub const THEME_ID_QUADRATIC_EQUATION_1: u32 = 14;
pub const THEME_ID_QUADRATIC_EQUATION_2: u32 = 15;
pub const THEME_ID_QUADRATIC_EQUATION_3: u32 = 16;
pub const THEME_ID_DECIMAL_ADD_SUBTRACT: u32 = 17;
pub const THEME_ID_DECIMAL_MULTIPLY_DIVIDE: u32 = 18;
pub const THEME_ID_SIMULTANEOUS_EQUATION_1: u32 = 19;
pub const THEME_ID_LIAR_PUZZLE: u32 = 20;
pub const THEME_ID_FRACTION_INTEGER_MULTIPLICATION: u32 = 21;
pub const THEME_ID_FRACTION_INTEGER_DIVISION: u32 = 22;
pub const THEME_ID_FRACTION_SUMMARY_IMPROPER: u32 = 23;
pub const THEME_ID_DECIMAL_DIVISION: u32 = 24;
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
pub const GENERATOR_REVISION_ONE_DIGIT_ADDITION: u32 = 5;
pub const GENERATOR_REVISION_LINEAR_EQUATION_1: u32 = 8;
pub const GENERATOR_REVISION_LINEAR_EQUATION_2: u32 = 8;
pub const GENERATOR_REVISION_ONE_DIGIT_SUBTRACTION: u32 = 3;
pub const GENERATOR_REVISION_TWO_DIGIT_ADDITION: u32 = 3;
pub const GENERATOR_REVISION_MULTIPLICATION_TABLE: u32 = 3;
pub const GENERATOR_REVISION_SIGNED_ARITHMETIC_1: u32 = 3;
pub const GENERATOR_REVISION_SIGNED_ARITHMETIC_2: u32 = 3;
pub const GENERATOR_REVISION_FRACTION_ADDITION: u32 = 4;
pub const GENERATOR_REVISION_FRACTION_ADDITION_LEGACY: u32 = 3;
pub const GENERATOR_REVISION_FRACTION_MULTIPLICATION: u32 = 4;
pub const GENERATOR_REVISION_FRACTION_MULTIPLICATION_LEGACY: u32 = 3;
pub const GENERATOR_REVISION_FRACTION_SUBTRACTION: u32 = 4;
pub const GENERATOR_REVISION_FRACTION_SUBTRACTION_LEGACY: u32 = 3;
pub const GENERATOR_REVISION_FRACTION_DIVISION: u32 = 4;
pub const GENERATOR_REVISION_FRACTION_DIVISION_LEGACY: u32 = 3;
pub const GENERATOR_REVISION_DIVISION_1: u32 = 3;
pub const GENERATOR_REVISION_QUADRATIC_EQUATION_1: u32 = 3;
pub const GENERATOR_REVISION_QUADRATIC_EQUATION_2: u32 = 4;
pub const GENERATOR_REVISION_QUADRATIC_EQUATION_3: u32 = 3;
pub const GENERATOR_REVISION_DECIMAL_ADD_SUBTRACT: u32 = 5;
pub const GENERATOR_REVISION_DECIMAL_MULTIPLY_DIVIDE: u32 = 6;
pub const GENERATOR_REVISION_DECIMAL_MULTIPLY_DIVIDE_LEGACY: u32 = 5;
pub const GENERATOR_REVISION_FRACTION_INTEGER_MULTIPLICATION: u32 = 1;
pub const GENERATOR_REVISION_FRACTION_INTEGER_DIVISION: u32 = 1;
pub const GENERATOR_REVISION_FRACTION_SUMMARY_IMPROPER: u32 = 1;
pub const GENERATOR_REVISION_DECIMAL_DIVISION: u32 = 1;
pub const GENERATOR_REVISION_COLUMN_ADD_2DIGIT: u32 = 1;
pub const GENERATOR_REVISION_COLUMN_SUBTRACT_2DIGIT: u32 = 1;
pub const GENERATOR_REVISION_COLUMN_ADD_3_4DIGIT: u32 = 1;
pub const GENERATOR_REVISION_COLUMN_SUBTRACT_3_4DIGIT: u32 = 1;
pub const GENERATOR_REVISION_COLUMN_MULTIPLY_1DIGIT: u32 = 1;
pub const GENERATOR_REVISION_COLUMN_MULTIPLY_2DIGIT: u32 = 1;
pub const GENERATOR_REVISION_COLUMN_DIVIDE_1DIGIT: u32 = 1;
pub const GENERATOR_REVISION_COLUMN_DIVIDE_2DIGIT: u32 = 1;
pub const GENERATOR_REVISION_COLUMN_DECIMAL_ADD_SUBTRACT: u32 = 1;
pub const GENERATOR_REVISION_COLUMN_DECIMAL_MULTIPLY_INTEGER: u32 = 1;
pub const GENERATOR_REVISION_COLUMN_DECIMAL_DIVIDE_INTEGER: u32 = 1;
pub const GENERATOR_REVISION_COLUMN_DECIMAL_MULTIPLICATION: u32 = 1;
pub const GENERATOR_REVISION_COLUMN_DECIMAL_DIVISION: u32 = 1;
pub const GENERATOR_REVISION_SIMULTANEOUS_EQUATION_1: u32 = 3;
pub const GENERATOR_REVISION_LIAR_PUZZLE: u32 = 4;
pub const SKILL_ID: &str = "jp.grade1.addition.one_digit";
pub const SKILL_ID_LINEAR_EQUATION_1: &str = "jp.grade7.equation.linear.1";
pub const SKILL_ID_LINEAR_EQUATION_2: &str = "jp.grade7.equation.linear.2";
pub const SKILL_ID_ONE_DIGIT_SUBTRACTION: &str = "jp.grade1.subtraction.one_digit";
pub const SKILL_ID_TWO_DIGIT_ADDITION: &str = "jp.grade2.addition.two_digit";
pub const SKILL_ID_MULTIPLICATION_TABLE: &str = "jp.grade2.multiplication.table";
pub const SKILL_ID_SIGNED_ARITHMETIC_1: &str = "jp.grade7.signed.arithmetic.1";
pub const SKILL_ID_SIGNED_ARITHMETIC_2: &str = "jp.grade7.signed.arithmetic.2";
pub const SKILL_ID_FRACTION_ADDITION: &str = "jp.grade5.fraction.addition";
pub const SKILL_ID_FRACTION_MULTIPLICATION: &str = "jp.grade6.fraction.multiplication";
pub const SKILL_ID_FRACTION_SUBTRACTION: &str = "jp.grade5.fraction.subtraction";
pub const SKILL_ID_FRACTION_DIVISION: &str = "jp.grade6.fraction.division";
pub const SKILL_ID_DIVISION_1: &str = "jp.grade3.division.table.1";
pub const SKILL_ID_QUADRATIC_EQUATION_1: &str = "jp.grade9.equation.quadratic.1";
pub const SKILL_ID_QUADRATIC_EQUATION_2: &str = "jp.grade9.equation.quadratic.2";
pub const SKILL_ID_QUADRATIC_EQUATION_3: &str = "jp.grade9.equation.quadratic.3";
pub const SKILL_ID_DECIMAL_ADD_SUBTRACT: &str = "jp.grade4.decimal.add_subtract";
pub const SKILL_ID_DECIMAL_MULTIPLY_DIVIDE: &str = "jp.grade5.decimal.multiplication";
pub const SKILL_ID_SIMULTANEOUS_EQUATION_1: &str = "jp.grade8.equation.simultaneous.1";
pub const SKILL_ID_LIAR_PUZZLE: &str = "bonus.logic.liar_puzzle";
pub const SKILL_ID_FRACTION_INTEGER_MULTIPLICATION: &str =
    "jp.grade6.fraction.integer_multiplication";
pub const SKILL_ID_FRACTION_INTEGER_DIVISION: &str = "jp.grade6.fraction.integer_division";
pub const SKILL_ID_FRACTION_SUMMARY_IMPROPER: &str = "jp.grade6.fraction.summary_improper";
pub const SKILL_ID_DECIMAL_DIVISION: &str = "jp.grade5.decimal.division";
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
pub const CURRICULUM_PATH: [&str; 3] = ["root", "小学1年生", "一桁の足し算"];
pub const CURRICULUM_PATH_LINEAR_EQUATION_1: [&str; 4] =
    ["root", "中学1年生", "一次方程式", "一次方程式(1)"];
pub const CURRICULUM_PATH_LINEAR_EQUATION_2: [&str; 4] =
    ["root", "中学1年生", "一次方程式", "一次方程式(2)"];
pub const CURRICULUM_PATH_ONE_DIGIT_SUBTRACTION: [&str; 3] = ["root", "小学1年生", "一桁の引き算"];
pub const CURRICULUM_PATH_TWO_DIGIT_ADDITION: [&str; 3] = ["root", "小学2年生", "二桁の足し算"];
pub const CURRICULUM_PATH_MULTIPLICATION_TABLE: [&str; 3] = ["root", "小学2年生", "九九"];
pub const CURRICULUM_PATH_SIGNED_ARITHMETIC_1: [&str; 3] = ["root", "中学1年生", "負の数の計算(1)"];
pub const CURRICULUM_PATH_SIGNED_ARITHMETIC_2: [&str; 3] = ["root", "中学1年生", "負の数の計算(2)"];
pub const CURRICULUM_PATH_FRACTION_ADDITION: [&str; 3] = ["root", "小学5年生", "分数の足し算"];
pub const CURRICULUM_PATH_FRACTION_MULTIPLICATION: [&str; 3] =
    ["root", "小学6年生", "分数の掛け算"];
pub const CURRICULUM_PATH_FRACTION_SUBTRACTION: [&str; 3] = ["root", "小学5年生", "分数の引き算"];
pub const CURRICULUM_PATH_FRACTION_DIVISION: [&str; 3] = ["root", "小学6年生", "分数の割り算"];
pub const CURRICULUM_PATH_FRACTION_INTEGER_MULTIPLICATION: [&str; 3] =
    ["root", "小学6年生", "分数と整数の掛け算"];
pub const CURRICULUM_PATH_FRACTION_INTEGER_DIVISION: [&str; 3] =
    ["root", "小学6年生", "分数と整数の割り算"];
pub const CURRICULUM_PATH_FRACTION_SUMMARY_IMPROPER: [&str; 3] =
    ["root", "小学6年生", "分数総まとめ(仮分数)"];
pub const CURRICULUM_PATH_DIVISION_1: [&str; 3] = ["root", "小学3年生", "割り算(1)"];
pub const CURRICULUM_PATH_QUADRATIC_EQUATION_1: [&str; 4] =
    ["root", "中学3年生", "二次方程式", "二次方程式(1)"];
pub const CURRICULUM_PATH_QUADRATIC_EQUATION_2: [&str; 4] =
    ["root", "中学3年生", "二次方程式", "二次方程式(2)"];
pub const CURRICULUM_PATH_QUADRATIC_EQUATION_3: [&str; 4] =
    ["root", "中学3年生", "二次方程式", "二次方程式(3)"];
pub const CURRICULUM_PATH_DECIMAL_ADD_SUBTRACT: [&str; 3] =
    ["root", "小学4年生", "小数の足し算と引き算"];
pub const CURRICULUM_PATH_DECIMAL_MULTIPLY_DIVIDE: [&str; 3] =
    ["root", "小学5年生", "小数の掛け算"];
pub const CURRICULUM_PATH_DECIMAL_MULTIPLY_DIVIDE_LEGACY: [&str; 3] =
    ["root", "小学5年生", "小数の掛け算と割り算"];
pub const CURRICULUM_PATH_DECIMAL_DIVISION: [&str; 3] = ["root", "小学5年生", "小数の割り算"];
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
pub const CURRICULUM_PATH_SIMULTANEOUS_EQUATION_1: [&str; 4] =
    ["root", "中学2年生", "連立方程式", "連立方程式(1)"];
pub const CURRICULUM_PATH_LIAR_PUZZLE: [&str; 3] = ["root", "おまけ", "うそつきだれだ"];
pub const DEFAULT_PROBLEM_COUNT: usize = 20;
pub const DEFAULT_COLUMNS: usize = 2;
pub const DEFAULT_ROWS: usize = 10;
pub const LINEAR_EQUATION_PROBLEM_COUNT: usize = 16;
pub const LINEAR_EQUATION_COLUMNS: usize = 2;
pub const LINEAR_EQUATION_ROWS: usize = 8;
pub const SIMULTANEOUS_EQUATION_PROBLEM_COUNT: usize = 12;
pub const SIMULTANEOUS_EQUATION_COLUMNS: usize = 2;
pub const SIMULTANEOUS_EQUATION_ROWS: usize = 6;
pub const LIAR_PUZZLE_PROBLEM_COUNT: usize = 6;
pub const LIAR_PUZZLE_COLUMNS: usize = 1;
pub const LIAR_PUZZLE_ROWS: usize = 6;
pub const COLUMN_ARITHMETIC_PROBLEM_COUNT: usize = 16;
pub const COLUMN_ARITHMETIC_COLUMNS: usize = 4;
pub const COLUMN_ARITHMETIC_ROWS: usize = 4;
/// Long division needs more vertical working room at the same readable font size.
pub const COLUMN_DIVISION_PROBLEM_COUNT: usize = 12;
pub const COLUMN_DIVISION_COLUMNS: usize = 4;
pub const COLUMN_DIVISION_ROWS: usize = 3;
pub const MIN_OPERAND: u8 = 1;
pub const MAX_OPERAND: u8 = 9;
pub const MIN_ANSWER: u8 = 1;
pub const MAX_ANSWER: u8 = 18;
pub const MAX_ANSWER_AST_SIZE: usize = 18;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct RationalCoefficient {
    pub numerator: i64,
    pub denominator: i64,
}

impl RationalCoefficient {
    pub fn new(numerator: i64, denominator: i64) -> Option<Self> {
        if denominator == 0 {
            return None;
        }
        let (mut numerator, mut denominator) = if denominator < 0 {
            (numerator.checked_neg()?, denominator.checked_neg()?)
        } else {
            (numerator, denominator)
        };
        if numerator == 0 {
            return Some(Self {
                numerator: 0,
                denominator: 1,
            });
        }
        let divisor = gcd_i64(numerator.unsigned_abs(), denominator as u64) as i64;
        numerator /= divisor;
        denominator /= divisor;
        Some(Self {
            numerator,
            denominator,
        })
    }

    pub const fn zero() -> Self {
        Self {
            numerator: 0,
            denominator: 1,
        }
    }

    pub fn is_zero(self) -> bool {
        self.numerator == 0
    }

    pub fn is_integer(self) -> bool {
        self.denominator == 1
    }

    pub fn checked_add(self, other: Self) -> Option<Self> {
        let left = self.numerator.checked_mul(other.denominator)?;
        let right = other.numerator.checked_mul(self.denominator)?;
        Self::new(
            left.checked_add(right)?,
            self.denominator.checked_mul(other.denominator)?,
        )
    }

    pub fn subtract(self, other: Self) -> Option<Self> {
        let left = self.numerator.checked_mul(other.denominator)?;
        let right = other.numerator.checked_mul(self.denominator)?;
        Self::new(
            left.checked_sub(right)?,
            self.denominator.checked_mul(other.denominator)?,
        )
    }

    pub fn multiply(self, other: Self) -> Option<Self> {
        Self::new(
            self.numerator.checked_mul(other.numerator)?,
            self.denominator.checked_mul(other.denominator)?,
        )
    }

    pub fn divide(self, other: Self) -> Option<Self> {
        if other.numerator == 0 {
            return None;
        }
        Self::new(
            self.numerator.checked_mul(other.denominator)?,
            self.denominator.checked_mul(other.numerator)?,
        )
    }
}

fn gcd_i64(mut left: u64, mut right: u64) -> u64 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ArithmeticOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ArithmeticExpression {
    Integer {
        value: i64,
    },
    Rational {
        value: RationalCoefficient,
    },
    ExactDecimal {
        coefficient: i64,
        scale: u32,
    },
    Binary {
        operator: ArithmeticOperator,
        left: Box<ArithmeticExpression>,
        right: Box<ArithmeticExpression>,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QuadraticEquationForm {
    SquareEqualsConstant,
    SquarePlusConstantZero,
    FactoredScale,
    Standard,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LiarStatement {
    SaysLiar {
        person: u8,
    },
    SaysNotLiar {
        person: u8,
    },
    ExactlyOneLiar {
        first: u8,
        second: u8,
    },
    ExactLiarCount {
        count: u8,
    },
    BothLiar {
        first: u8,
        second: u8,
    },
    BothNotLiar {
        first: u8,
        second: u8,
    },
    Implication {
        antecedent_person: u8,
        antecedent_is_liar: bool,
        consequent_person: u8,
        consequent_is_liar: bool,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProblemPrompt {
    Addition {
        left: u8,
        right: u8,
    },
    Arithmetic {
        expression: ArithmeticExpression,
    },
    ColumnArithmetic {
        operator: ArithmeticOperator,
        left: ArithmeticExpression,
        right: ArithmeticExpression,
    },
    LinearEquation {
        a: RationalCoefficient,
        b: RationalCoefficient,
        c: RationalCoefficient,
        d: RationalCoefficient,
        left_negative_constant_as_subtraction: bool,
        right_negative_constant_as_subtraction: bool,
    },
    QuadraticEquation {
        form: QuadraticEquationForm,
        a: RationalCoefficient,
        b: RationalCoefficient,
        c: RationalCoefficient,
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
        people_count: u8,
        statements: Vec<LiarStatement>,
    },
}

/// The answer-entry UI is a capability-bearing data value, independent of the
/// public wire schema. It describes which editor affordances may be exposed
/// for one problem; it does not change AnswerSchema or mathematical semantics.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AnswerInputInterface {
    SimpleNumeric {
        allow_decimal: bool,
        allow_negative: bool,
    },
    StructuredMath {
        allowed_structures: Vec<EditorStructure>,
    },
}

impl AnswerInputInterface {
    pub fn allows_structure(&self, structure: EditorStructure) -> bool {
        match self {
            Self::SimpleNumeric {
                allow_decimal,
                allow_negative,
            } => match structure {
                EditorStructure::Decimal => *allow_decimal,
                EditorStructure::Negative => *allow_negative,
                _ => false,
            },
            Self::StructuredMath { allowed_structures } => allowed_structures.contains(&structure),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AnswerSchema {
    Integer {
        #[serde(with = "crate::exact::i64_decimal_string")]
        min: i64,
        #[serde(with = "crate::exact::i64_decimal_string")]
        max: i64,
    },
    Rational {
        max_abs_numerator: u32,
        max_denominator: u32,
        require_reduced_fraction_form: bool,
    },
    Decimal {
        max_scale: u32,
    },
    OrderedPair,
    Algebraic,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Problem {
    pub schema_version: u16,
    pub id: u32,
    pub numeric_theme_id: u32,
    pub prompt: ProblemPrompt,
    pub input_interface: AnswerInputInterface,
    pub answer_schema: AnswerSchema,
    pub canonical_answer: AnswerNode,
    pub solution_graph: SolutionGraph,
    pub operation_vector: OperationVector,
    pub effort: f64,
}

impl Problem {
    pub fn answer_node(&self) -> AnswerNode {
        self.canonical_answer.clone()
    }

    pub fn ordered_pair(&self) -> (u8, u8) {
        match self.prompt {
            ProblemPrompt::Addition { left, right } => (left, right),
            ProblemPrompt::Arithmetic { .. }
            | ProblemPrompt::ColumnArithmetic { .. }
            | ProblemPrompt::LinearEquation { .. }
            | ProblemPrompt::QuadraticEquation { .. }
            | ProblemPrompt::SimultaneousEquation { .. }
            | ProblemPrompt::LiarPuzzle { .. } => {
                panic!("ordered_pair is addition-only")
            }
        }
    }

    pub fn left(&self) -> u8 {
        self.ordered_pair().0
    }

    pub fn right(&self) -> u8 {
        self.ordered_pair().1
    }

    pub fn answer(&self) -> u8 {
        self.canonical_answer
            .as_integer()
            .and_then(|value| u8::try_from(value).ok())
            .expect("registered one-digit addition answers fit u8")
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LayoutMetadata {
    pub problem_count: usize,
    pub columns: usize,
    pub rows: usize,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Worksheet {
    pub schema_version: u16,
    pub problem_set_id: String,
    pub identity: ProblemSetIdentity,
    pub skill_id: String,
    pub curriculum_path: Vec<String>,
    pub layout: LayoutMetadata,
    pub problems: Vec<Problem>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EditorState {
    pub answer: AnswerNode,
    pub cursor: usize,
    pub active_path: Vec<usize>,
    pub committed: bool,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            answer: AnswerNode::Empty,
            cursor: 0,
            active_path: Vec::new(),
            committed: false,
        }
    }
}

impl EditorState {
    pub fn empty() -> Self {
        Self::default()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EditorAction {
    InsertDigit { digit: u8 },
    Backspace,
    Delete,
    MoveLeft,
    MoveRight,
    InsertStructure { structure: EditorStructure },
    SelectSlot { path: Vec<usize>, cursor: usize },
    Clear,
    Commit,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EditorStructure {
    Fraction,
    MixedFraction,
    Decimal,
    Root,
    Negative,
    PlusMinus,
    Tuple,
    Arithmetic,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GradeStatus {
    Correct,
    Incorrect,
    Unanswered,
}

/// Stable warning identifiers. UI copy is deliberately owned by the client so
/// wording can change without changing the grading contract. The enum and its
/// exported variant list are generated from one declaration to prevent drift.
macro_rules! define_grade_warnings {
    ($($variant:ident),+ $(,)?) => {
        #[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
        #[serde(rename_all = "snake_case")]
        pub enum GradeWarning {
            $($variant),+
        }

        impl GradeWarning {
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];
        }
    };
}

define_grade_warnings!(
    FractionNotReduced,
    RedundantNegative,
    RedundantPlusMinus,
    RedundantDecimal,
    DuplicateSolution,
    SolutionListRequired,
    FractionFormRequired,
    MixedFractionFormRequired,
    IntegerFormRequired,
);

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GradeResult {
    pub status: GradeStatus,
    pub is_correct: bool,
    pub expected: AnswerNode,
    pub actual: AnswerNode,
    pub warnings: Vec<GradeWarning>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GenerateProblemRequest {
    pub schema_version: u16,
    #[serde(default = "default_theme_id")]
    pub numeric_theme_id: u32,
    #[serde(default)]
    pub seed: String,
}

impl Default for GenerateProblemRequest {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            numeric_theme_id: THEME_ID_ONE_DIGIT_ADDITION,
            seed: String::new(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GenerateWorksheetRequest {
    pub schema_version: u16,
    #[serde(default = "default_theme_id")]
    pub numeric_theme_id: u32,
    #[serde(default)]
    pub seed: String,
    #[serde(default)]
    pub difficulty: Difficulty,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    #[serde(default)]
    pub max_attempts: Option<u64>,
}

impl Default for GenerateWorksheetRequest {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            numeric_theme_id: THEME_ID_ONE_DIGIT_ADDITION,
            seed: String::new(),
            difficulty: Difficulty::default(),
            timeout_ms: None,
            max_attempts: None,
        }
    }
}

const fn default_theme_id() -> u32 {
    THEME_ID_ONE_DIGIT_ADDITION
}
