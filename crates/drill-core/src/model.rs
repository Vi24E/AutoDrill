//! Versioned domain values shared by the native engine and the WASM adapter.

use serde::{Deserialize, Serialize};

use crate::answer::AnswerNode;
use crate::effort::{OperationVector, SolutionGraph};
use crate::identity::{Difficulty, ProblemSetIdentity};

pub const SCHEMA_VERSION: u16 = 2;
pub const THEME_ID_ONE_DIGIT_ADDITION: u32 = 1;
pub const GENERATOR_REVISION_ONE_DIGIT_ADDITION: u32 = 2;
pub const SKILL_ID: &str = "jp.grade1.addition.one_digit";
pub const CURRICULUM_PATH: [&str; 3] = ["root", "小学1年生", "一桁の足し算"];
pub const DEFAULT_PROBLEM_COUNT: usize = 20;
pub const DEFAULT_COLUMNS: usize = 2;
pub const DEFAULT_ROWS: usize = 10;
pub const MIN_OPERAND: u8 = 1;
pub const MAX_OPERAND: u8 = 9;
pub const MIN_ANSWER: u8 = 1;
pub const MAX_ANSWER: u8 = 18;
pub const MAX_ANSWER_AST_SIZE: usize = 18;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProblemPrompt {
    Addition { left: u8, right: u8 },
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
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Problem {
    pub schema_version: u16,
    pub id: u32,
    pub numeric_theme_id: u32,
    pub prompt: ProblemPrompt,
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

/// Stable warning identifiers. UI copy is deliberately owned by the client so
/// wording can change without changing the grading contract.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GradeWarning {
    FractionNotReduced,
    RedundantNegative,
    RedundantDecimal,
}

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
    #[serde(default = "default_schema_version")]
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
    #[serde(default = "default_schema_version")]
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

const fn default_schema_version() -> u16 {
    SCHEMA_VERSION
}

const fn default_theme_id() -> u32 {
    THEME_ID_ONE_DIGIT_ADDITION
}
