//! Versioned domain values shared by the native engine and the WASM adapter.

use serde::{Deserialize, Serialize};

use crate::answer::AnswerNode;
use crate::effort::{OperationVector, SolutionGraph};
use crate::identity::{Difficulty, ProblemSetIdentity};

pub const SCHEMA_VERSION: u16 = 3;
pub const THEME_ID_ONE_DIGIT_ADDITION: u32 = 1;
pub const THEME_ID_LINEAR_EQUATION_1: u32 = 2;
pub const THEME_ID_LINEAR_EQUATION_2: u32 = 3;
pub const GENERATOR_REVISION_ONE_DIGIT_ADDITION: u32 = 3;
pub const GENERATOR_REVISION_LINEAR_EQUATION_1: u32 = 6;
pub const GENERATOR_REVISION_LINEAR_EQUATION_2: u32 = 6;
pub const SKILL_ID: &str = "jp.grade1.addition.one_digit";
pub const SKILL_ID_LINEAR_EQUATION_1: &str = "jp.grade7.equation.linear.1";
pub const SKILL_ID_LINEAR_EQUATION_2: &str = "jp.grade7.equation.linear.2";
pub const CURRICULUM_PATH: [&str; 3] = ["root", "小学1年生", "一桁の足し算"];
pub const CURRICULUM_PATH_LINEAR_EQUATION_1: [&str; 4] =
    ["root", "中学1年生", "一次方程式", "一次方程式(1)"];
pub const CURRICULUM_PATH_LINEAR_EQUATION_2: [&str; 4] =
    ["root", "中学1年生", "一次方程式", "一次方程式(2)"];
pub const DEFAULT_PROBLEM_COUNT: usize = 20;
pub const DEFAULT_COLUMNS: usize = 2;
pub const DEFAULT_ROWS: usize = 10;
pub const LINEAR_EQUATION_PROBLEM_COUNT: usize = 16;
pub const LINEAR_EQUATION_COLUMNS: usize = 2;
pub const LINEAR_EQUATION_ROWS: usize = 8;
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

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProblemPrompt {
    Addition {
        left: u8,
        right: u8,
    },
    LinearEquation {
        a: RationalCoefficient,
        b: RationalCoefficient,
        c: RationalCoefficient,
        d: RationalCoefficient,
        left_negative_constant_as_subtraction: bool,
        right_negative_constant_as_subtraction: bool,
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
            ProblemPrompt::LinearEquation { .. } => panic!("ordered_pair is addition-only"),
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
    RedundantDecimal,
    FractionFormRequired,
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
