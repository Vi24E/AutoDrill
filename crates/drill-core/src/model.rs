//! Versioned domain values shared by the native engine and the WASM adapter.

use serde::{Deserialize, Serialize};
#[cfg(feature = "wire-types")]
use ts_rs::TS;

use crate::answer::AnswerNode;
use crate::effort::{OperationVector, SolutionGraph};
use crate::exact::gcd_u64;
use crate::identity::{Difficulty, ProblemSetIdentity};

use crate::schema::SCHEMA_VERSION;
use crate::themes::basic_arithmetic::THEME_ID_ONE_DIGIT_ADDITION;

/// Maximum accepted answer AST size for interactive input.
pub const MAX_ANSWER_AST_SIZE: usize = 18;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[cfg_attr(feature = "wire-types", derive(TS))]
pub struct RationalCoefficient {
    #[cfg_attr(feature = "wire-types", ts(type = "number"))]
    pub numerator: i64,
    #[cfg_attr(feature = "wire-types", ts(type = "number"))]
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
        let divisor = gcd_u64(numerator.unsigned_abs(), denominator as u64) as i64;
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

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[cfg_attr(feature = "wire-types", derive(TS))]
#[serde(rename_all = "snake_case")]
pub enum ArithmeticOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[cfg_attr(feature = "wire-types", derive(TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ArithmeticExpression {
    Integer {
        #[cfg_attr(feature = "wire-types", ts(type = "number"))]
        value: i64,
    },
    Rational {
        value: RationalCoefficient,
    },
    ExactDecimal {
        #[cfg_attr(feature = "wire-types", ts(type = "number"))]
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
#[cfg_attr(feature = "wire-types", derive(TS))]
#[serde(rename_all = "snake_case")]
pub enum QuadraticEquationForm {
    SquareEqualsConstant,
    SquarePlusConstantZero,
    FactoredScale,
    Standard,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[cfg_attr(feature = "wire-types", derive(TS))]
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
#[cfg_attr(feature = "wire-types", derive(TS))]
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
        #[cfg_attr(feature = "wire-types", ts(type = "number"))]
        a: i64,
        #[cfg_attr(feature = "wire-types", ts(type = "number"))]
        b: i64,
        #[cfg_attr(feature = "wire-types", ts(type = "number"))]
        c: i64,
        #[cfg_attr(feature = "wire-types", ts(type = "number"))]
        d: i64,
        #[cfg_attr(feature = "wire-types", ts(type = "number"))]
        e: i64,
        #[cfg_attr(feature = "wire-types", ts(type = "number"))]
        f: i64,
    },
    LiarPuzzle {
        people_count: u8,
        statements: Vec<LiarStatement>,
    },
    MiniSudoku {
        givens: Vec<Option<u8>>,
    },
}

/// The answer-entry UI is a capability-bearing data value, independent of the
/// public wire schema. It describes which editor affordances may be exposed
/// for one problem; it does not change AnswerSchema or mathematical semantics.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "wire-types", derive(TS))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AnswerInputInterface {
    SimpleNumeric {
        allow_decimal: bool,
        allow_negative: bool,
    },
    StructuredMath {
        allowed_structures: Vec<EditorStructure>,
    },
    DigitGrid {
        min_digit: u8,
        max_digit: u8,
        cell_count: u8,
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
            Self::DigitGrid { .. } => false,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "wire-types", derive(TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AnswerSchema {
    Integer {
        #[serde(with = "crate::exact::i64_decimal_string")]
        #[cfg_attr(feature = "wire-types", ts(type = "string"))]
        min: i64,
        #[serde(with = "crate::exact::i64_decimal_string")]
        #[cfg_attr(feature = "wire-types", ts(type = "string"))]
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
    OrderedTuple { length: u8 },
    Algebraic,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "wire-types", derive(TS))]
pub struct ColumnMultiplicationPartial {
    #[cfg_attr(feature = "wire-types", ts(type = "number"))]
    pub value: i64,
    pub place: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "wire-types", derive(TS))]
pub struct LongDivisionStep {
    #[cfg_attr(feature = "wire-types", ts(type = "number"))]
    pub product: i64,
    #[cfg_attr(feature = "wire-types", ts(type = "number"))]
    pub after: i64,
    pub product_offset: u32,
    pub after_offset: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "wire-types", derive(TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WorkedSolution {
    ColumnMultiplication {
        partial_products: Vec<ColumnMultiplicationPartial>,
    },
    LongDivision {
        #[cfg_attr(feature = "wire-types", ts(type = "number"))]
        divisor: i64,
        #[cfg_attr(feature = "wire-types", ts(type = "number"))]
        dividend_coefficient: i64,
        dividend_scale: u32,
        quotient_trailing_cells: u32,
        steps: Vec<LongDivisionStep>,
    },
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[cfg_attr(feature = "wire-types", derive(TS))]
pub struct Problem {
    pub schema_version: u16,
    pub id: u32,
    pub numeric_theme_id: u32,
    pub prompt: ProblemPrompt,
    pub input_interface: AnswerInputInterface,
    pub answer_schema: AnswerSchema,
    pub canonical_answer: AnswerNode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worked_solution: Option<WorkedSolution>,
    pub solution_graph: SolutionGraph,
    pub operation_vector: OperationVector,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme_specific_effort: Option<f64>,
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
            | ProblemPrompt::LiarPuzzle { .. }
            | ProblemPrompt::MiniSudoku { .. } => {
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
#[cfg_attr(feature = "wire-types", derive(TS))]
pub struct LayoutMetadata {
    pub problem_count: usize,
    pub columns: usize,
    pub rows: usize,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[cfg_attr(feature = "wire-types", derive(TS))]
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
#[cfg_attr(feature = "wire-types", derive(TS))]
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

macro_rules! define_editor_actions {
    ($( $variant:ident $( { $($field:ident : $ty:ty),* $(,)? } )? ),+ $(,)?) => {
        #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
        #[cfg_attr(feature = "wire-types", derive(TS))]
        #[serde(tag = "type", rename_all = "snake_case")]
        pub enum EditorAction {
            $( $variant $( { $($field: $ty),* } )? ),+
        }

        impl EditorAction {
            /// Canonical Serde discriminants for Web runtime validation.
            /// Generated from the same declaration as the enum, so adding a
            /// variant cannot require a second TypeScript inventory.
            pub fn wire_types() -> Vec<String> {
                vec![
                    $(
                        serde_json::to_value(Self::$variant $( { $($field: <$ty as Default>::default()),* } )?)
                            .expect("editor action must serialize")
                            .get("type")
                            .and_then(serde_json::Value::as_str)
                            .expect("editor action wire value must contain a type")
                            .to_owned()
                    ),+
                ]
            }
        }
    };
}

macro_rules! define_editor_structures {
    ($first:ident $(, $rest:ident)* $(,)?) => {
        #[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
        #[cfg_attr(feature = "wire-types", derive(TS))]
        #[serde(rename_all = "snake_case")]
        pub enum EditorStructure {
            #[default]
            $first,
            $($rest),*
        }

        impl EditorStructure {
            pub const ALL: &'static [Self] = &[Self::$first, $(Self::$rest),*];
        }
    };
}

define_editor_structures!(
    Fraction,
    MixedFraction,
    Decimal,
    Root,
    Negative,
    PlusMinus,
    Tuple,
    Arithmetic,
);

define_editor_actions!(
    InsertDigit { digit: u8 },
    Backspace,
    Delete,
    MoveLeft,
    MoveRight,
    InsertStructure { structure: EditorStructure },
    SelectSlot { path: Vec<usize>, cursor: usize },
    Clear,
    Commit,
);

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "wire-types", derive(TS))]
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
        #[cfg_attr(feature = "wire-types", derive(TS))]
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
#[cfg_attr(feature = "wire-types", derive(TS))]
pub struct GradeResult {
    pub status: GradeStatus,
    pub is_correct: bool,
    pub expected: AnswerNode,
    pub actual: AnswerNode,
    pub warnings: Vec<GradeWarning>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "wire-types", derive(TS))]
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
#[cfg_attr(feature = "wire-types", derive(TS))]
pub struct GenerateWorksheetRequest {
    pub schema_version: u16,
    #[serde(default = "default_theme_id")]
    pub numeric_theme_id: u32,
    #[serde(default)]
    pub seed: String,
    #[serde(default)]
    pub difficulty: Difficulty,
    #[serde(default)]
    #[cfg_attr(feature = "wire-types", ts(type = "number | null"))]
    pub timeout_ms: Option<u64>,
    #[serde(default)]
    #[cfg_attr(feature = "wire-types", ts(type = "number | null"))]
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
