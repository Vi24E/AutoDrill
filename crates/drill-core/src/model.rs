//! Versioned domain values shared by the native engine and the WASM adapter.

use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: u16 = 1;
pub const SKILL_ID: &str = "jp.grade1.addition.one_digit.1";
pub const GENERATOR_VERSION: &str = "addition-one-digit-v1";
pub const CURRICULUM_PATH: [&str; 3] = ["root", "小学1年生", "1けたのたしざん(1)"];
pub const DEFAULT_PROBLEM_COUNT: usize = 20;
pub const DEFAULT_COLUMNS: usize = 2;
pub const DEFAULT_ROWS: usize = 10;
pub const MIN_OPERAND: u8 = 1;
pub const MAX_OPERAND: u8 = 9;
pub const MIN_ANSWER: u8 = 1;
pub const MAX_ANSWER: u8 = 18;
pub const MAX_ANSWER_AST_SIZE: usize = 18;

/// The typed answer AST currently supported by alpha 1.0.
///
/// The enum is intentionally open for future answer node kinds.  An empty
/// value is a first-class node so an editor never has to encode a draft as an
/// untyped string or a sentinel integer.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum AnswerNode {
    #[default]
    Empty,
    Integer(i64),
}

impl AnswerNode {
    pub const fn empty() -> Self {
        Self::Empty
    }

    pub const fn integer(value: i64) -> Self {
        Self::Integer(value)
    }

    pub fn as_integer(&self) -> Option<i64> {
        match self {
            Self::Integer(value) => Some(*value),
            Self::Empty => None,
        }
    }

    pub fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }

    /// Return the structural size of this answer AST.
    ///
    /// Alpha integer nodes count one per decimal digit; the empty draft is
    /// zero. Future composite nodes add one for the parent plus every child's
    /// size, so `frac(num(12), num(42))` will have size five.
    pub fn size(&self) -> usize {
        match self {
            Self::Empty => 0,
            Self::Integer(value) => decimal_digit_count(value.unsigned_abs()),
        }
    }

    pub fn is_within_size_limit(&self) -> bool {
        self.size() <= MAX_ANSWER_AST_SIZE
    }
}

fn decimal_digit_count(mut value: u64) -> usize {
    if value == 0 {
        return 1;
    }
    let mut digits = 0;
    while value > 0 {
        value /= 10;
        digits += 1;
    }
    digits
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OperationCounts {
    pub additions: u32,
    pub carries: u32,
}

impl OperationCounts {
    pub const fn one_digit_addition(carry: bool) -> Self {
        Self {
            additions: 1,
            carries: if carry { 1 } else { 0 },
        }
    }

    pub const fn zero() -> Self {
        Self {
            additions: 0,
            carries: 0,
        }
    }

    pub const fn total(&self) -> u32 {
        self.additions + self.carries
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Problem {
    pub schema_version: u16,
    pub id: u32,
    pub left: u8,
    pub right: u8,
    pub answer: u8,
    pub operation_counts: OperationCounts,
}

impl Problem {
    pub fn answer_node(&self) -> AnswerNode {
        AnswerNode::Integer(i64::from(self.answer))
    }

    pub fn ordered_pair(&self) -> (u8, u8) {
        (self.left, self.right)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LayoutMetadata {
    pub problem_count: usize,
    pub columns: usize,
    pub rows: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Worksheet {
    pub schema_version: u16,
    pub skill_id: String,
    pub curriculum_path: Vec<String>,
    pub layout: LayoutMetadata,
    pub generator_version: String,
    /// Kept as a string at every boundary so JavaScript never rounds a seed.
    pub seed: String,
    pub problems: Vec<Problem>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EditorState {
    pub answer: AnswerNode,
    pub cursor: usize,
    pub committed: bool,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            answer: AnswerNode::Empty,
            cursor: 0,
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
    #[serde(alias = "insert")]
    InsertDigit {
        digit: u8,
    },
    Backspace,
    Delete,
    #[serde(alias = "left")]
    MoveLeft,
    #[serde(alias = "right")]
    MoveRight,
    Clear,
    Commit,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GradeStatus {
    Correct,
    Incorrect,
    Unanswered,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GradeResult {
    pub status: GradeStatus,
    pub is_correct: bool,
    pub expected: AnswerNode,
    pub actual: AnswerNode,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EffortWeights {
    pub addition: u32,
    pub carry: u32,
}

impl Default for EffortWeights {
    fn default() -> Self {
        Self {
            addition: 1,
            carry: 1,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EffortResult {
    pub value: u32,
    pub operation_counts: OperationCounts,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GenerateProblemRequest {
    #[serde(default = "default_schema_version")]
    pub schema_version: u16,
    #[serde(default)]
    pub seed: String,
}

impl Default for GenerateProblemRequest {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            seed: String::new(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GenerateWorksheetRequest {
    #[serde(default = "default_schema_version")]
    pub schema_version: u16,
    #[serde(default)]
    pub seed: String,
    #[serde(default)]
    pub problem_count: Option<usize>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    #[serde(default)]
    pub max_attempts: Option<u64>,
}

impl Default for GenerateWorksheetRequest {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            seed: String::new(),
            problem_count: None,
            timeout_ms: None,
            max_attempts: None,
        }
    }
}

fn default_schema_version() -> u16 {
    SCHEMA_VERSION
}
