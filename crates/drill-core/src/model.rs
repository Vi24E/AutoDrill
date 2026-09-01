//! Core domain values and validated aggregates. Boundary request/value syntax may be
//! serializable here, while aggregate wire projections live in `wire.rs`; WASM never
//! deserializes directly into `Problem` or `Worksheet`.

use serde::{Deserialize, Serialize, Serializer};
#[cfg(feature = "wire-types")]
use ts_rs::TS;

use crate::answer::AnswerNode;
use crate::effort::EffortModel;
#[cfg(test)]
use crate::effort::{OperationPlan, OperationVector};
use crate::exact::ExactRational;
use crate::identity::{Difficulty, ProblemSetIdentity};
use crate::theme::{
    ColumnAnswerPartInputPolicy, ColumnDecimalPointPolicy, ColumnInputOrder, DigitGridSpec,
    ThemeAnswerSchemaKind, ThemePromptKind, ThemeRegistration,
};

use crate::schema::SCHEMA_VERSION;

/// Maximum accepted answer AST size for interactive input.
pub const MAX_ANSWER_AST_SIZE: usize = 18;
const JS_SAFE_INTEGER_MAX: i64 = 9_007_199_254_740_991;

const fn is_js_safe_integer(value: i64) -> bool {
    value >= -JS_SAFE_INTEGER_MAX && value <= JS_SAFE_INTEGER_MAX
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[cfg_attr(feature = "wire-types", derive(TS))]
pub struct RationalCoefficient {
    #[cfg_attr(feature = "wire-types", ts(type = "number"))]
    numerator: i64,
    #[cfg_attr(feature = "wire-types", ts(type = "number"))]
    denominator: i64,
}

impl RationalCoefficient {
    pub fn new(numerator: i64, denominator: i64) -> Option<Self> {
        Self::from_exact(ExactRational::new(
            i128::from(numerator),
            i128::from(denominator),
        )?)
    }

    fn from_exact(value: ExactRational) -> Option<Self> {
        Some(Self {
            numerator: i64::try_from(value.numerator()).ok()?,
            denominator: i64::try_from(value.denominator()).ok()?,
        })
    }

    fn as_exact(self) -> Option<ExactRational> {
        ExactRational::new(i128::from(self.numerator), i128::from(self.denominator))
    }

    pub const fn zero() -> Self {
        Self {
            numerator: 0,
            denominator: 1,
        }
    }

    pub const fn numerator(self) -> i64 {
        self.numerator
    }
    pub const fn denominator(self) -> i64 {
        self.denominator
    }
    pub const fn is_zero(self) -> bool {
        self.numerator == 0
    }
    pub const fn is_integer(self) -> bool {
        self.denominator == 1
    }

    const fn has_js_safe_wire_values(self) -> bool {
        is_js_safe_integer(self.numerator) && is_js_safe_integer(self.denominator)
    }

    pub fn checked_add(self, other: Self) -> Option<Self> {
        Self::from_exact(self.as_exact()?.add(other.as_exact()?)?)
    }

    pub fn subtract(self, other: Self) -> Option<Self> {
        Self::from_exact(self.as_exact()?.subtract(other.as_exact()?)?)
    }

    pub fn multiply(self, other: Self) -> Option<Self> {
        Self::from_exact(self.as_exact()?.multiply(other.as_exact()?)?)
    }

    pub fn divide(self, other: Self) -> Option<Self> {
        Self::from_exact(self.as_exact()?.divide(other.as_exact()?)?)
    }
}

impl<'de> Deserialize<'de> for RationalCoefficient {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Repr {
            numerator: i64,
            denominator: i64,
        }

        let repr = Repr::deserialize(deserializer)?;
        Self::new(repr.numerator, repr.denominator)
            .ok_or_else(|| serde::de::Error::custom("invalid rational coefficient"))
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

pub const MAX_LIAR_PUZZLE_PEOPLE: u8 = 4;
pub const MINI_SUDOKU_SIDE: usize = 4;
pub const MINI_SUDOKU_GRID_SPEC: DigitGridSpec =
    DigitGridSpec::new(1, 4, (MINI_SUDOKU_SIDE * MINI_SUDOKU_SIDE) as u8);
pub const MINI_SUDOKU_CELL_COUNT: usize = MINI_SUDOKU_GRID_SPEC.cell_count() as usize;

macro_rules! validated_u8_newtype {
    ($name:ident, $min:expr, $max:expr, $message:literal) => {
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
        pub struct $name(u8);

        impl $name {
            pub const fn new(value: u8) -> Option<Self> {
                if value >= $min && value <= $max {
                    Some(Self(value))
                } else {
                    None
                }
            }

            pub const fn value(self) -> u8 {
                self.0
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let value = u8::deserialize(deserializer)?;
                Self::new(value).ok_or_else(|| serde::de::Error::custom($message))
            }
        }
    };
}

validated_u8_newtype!(
    PersonIndex,
    1,
    MAX_LIAR_PUZZLE_PEOPLE,
    "invalid liar-puzzle person index"
);
validated_u8_newtype!(
    PeopleCount,
    3,
    MAX_LIAR_PUZZLE_PEOPLE,
    "invalid liar-puzzle people count"
);

impl From<PersonIndex> for u8 {
    fn from(value: PersonIndex) -> Self {
        value.value()
    }
}
impl From<PersonIndex> for u32 {
    fn from(value: PersonIndex) -> Self {
        u32::from(value.value())
    }
}
impl From<PeopleCount> for u8 {
    fn from(value: PeopleCount) -> Self {
        value.value()
    }
}
impl From<PeopleCount> for u32 {
    fn from(value: PeopleCount) -> Self {
        u32::from(value.value())
    }
}
impl From<PeopleCount> for usize {
    fn from(value: PeopleCount) -> Self {
        usize::from(value.value())
    }
}
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct LiarCount(u8);

impl LiarCount {
    pub const fn new(value: u8) -> Option<Self> {
        if value >= 1 && value < MAX_LIAR_PUZZLE_PEOPLE {
            Some(Self(value))
        } else {
            None
        }
    }
    pub const fn value(self) -> u8 {
        self.0
    }
}

impl<'de> Deserialize<'de> for LiarCount {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = u8::deserialize(deserializer)?;
        Self::new(value).ok_or_else(|| serde::de::Error::custom("invalid liar count"))
    }
}

impl From<LiarCount> for u8 {
    fn from(value: LiarCount) -> Self {
        value.value()
    }
}
impl From<LiarCount> for u32 {
    fn from(value: LiarCount) -> Self {
        u32::from(value.value())
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct MiniSudokuGrid([Option<u8>; MINI_SUDOKU_CELL_COUNT]);

impl MiniSudokuGrid {
    pub fn new(cells: [Option<u8>; MINI_SUDOKU_CELL_COUNT]) -> Option<Self> {
        cells
            .iter()
            .flatten()
            .all(|digit| {
                (MINI_SUDOKU_GRID_SPEC.min_digit()..=MINI_SUDOKU_GRID_SPEC.max_digit())
                    .contains(digit)
            })
            .then_some(Self(cells))
    }
}

impl std::ops::Index<usize> for MiniSudokuGrid {
    type Output = Option<u8>;
    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl<'de> Deserialize<'de> for MiniSudokuGrid {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let cells = Vec::<Option<u8>>::deserialize(deserializer)?;
        let cells: [Option<u8>; MINI_SUDOKU_CELL_COUNT] = cells
            .try_into()
            .map_err(|_| serde::de::Error::custom("mini sudoku must contain exactly 16 cells"))?;
        Self::new(cells)
            .ok_or_else(|| serde::de::Error::custom("mini sudoku digit is outside 1..=4"))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[cfg_attr(feature = "wire-types", derive(TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LiarStatement {
    SaysLiar {
        #[cfg_attr(feature = "wire-types", ts(type = "number"))]
        person: PersonIndex,
    },
    SaysNotLiar {
        #[cfg_attr(feature = "wire-types", ts(type = "number"))]
        person: PersonIndex,
    },
    ExactlyOneLiar {
        #[cfg_attr(feature = "wire-types", ts(type = "number"))]
        first: PersonIndex,
        #[cfg_attr(feature = "wire-types", ts(type = "number"))]
        second: PersonIndex,
    },
    ExactLiarCount {
        #[cfg_attr(feature = "wire-types", ts(type = "number"))]
        count: LiarCount,
    },
    BothLiar {
        #[cfg_attr(feature = "wire-types", ts(type = "number"))]
        first: PersonIndex,
        #[cfg_attr(feature = "wire-types", ts(type = "number"))]
        second: PersonIndex,
    },
    BothNotLiar {
        #[cfg_attr(feature = "wire-types", ts(type = "number"))]
        first: PersonIndex,
        #[cfg_attr(feature = "wire-types", ts(type = "number"))]
        second: PersonIndex,
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
        #[cfg_attr(feature = "wire-types", ts(type = "number"))]
        people_count: PeopleCount,
        statements: Vec<LiarStatement>,
    },
    MiniSudoku {
        #[cfg_attr(feature = "wire-types", ts(type = "Array<number | null>"))]
        givens: MiniSudokuGrid,
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
    pub fn is_structurally_valid(&self) -> bool {
        match self {
            Self::SimpleNumeric { .. } => true,
            Self::StructuredMath { allowed_structures } => {
                !allowed_structures.is_empty()
                    && allowed_structures
                        .iter()
                        .enumerate()
                        .all(|(index, structure)| !allowed_structures[..index].contains(structure))
            }
            Self::DigitGrid {
                min_digit,
                max_digit,
                cell_count,
            } => min_digit <= max_digit && *cell_count > 0,
        }
    }

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

    /// Validate one parsed answer against this input capability contract.
    /// Rust owns these semantics; Web only validates the Serde wire shape.
    pub fn validate_answer(&self, answer: &AnswerNode) -> Result<(), crate::error::EditorError> {
        if !self.is_structurally_valid() {
            return Err(crate::error::EditorError::InputInterfaceViolation);
        }
        if !answer.is_within_size_limit() {
            return Err(crate::error::EditorError::AnswerSizeLimit {
                max_size: MAX_ANSWER_AST_SIZE,
            });
        }
        crate::input::ensure_capability(answer, self)
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
    DecimalDivisionRemainder {
        quotient_scale: u32,
        remainder_max_scale: u32,
    },
    RoundedDecimal {
        scale: u32,
    },
    OrderedPair,
    OrderedTuple {
        length: u8,
    },
    Algebraic,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkedSolution {
    kind: WorkedSolutionKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum WorkedSolutionKind {
    ColumnMultiplication {
        partial_products: Vec<ColumnMultiplicationPartial>,
    },
    LongDivision {
        divisor: i64,
        dividend_coefficient: i64,
        dividend_scale: u32,
        quotient_trailing_cells: u32,
        steps: Vec<LongDivisionStep>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ColumnMultiplicationPartial {
    value: i64,
    place: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct LongDivisionStep {
    product: i64,
    after: i64,
    product_offset: u32,
    after_offset: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ValidatedAnswerSchema(AnswerSchema);

impl ValidatedAnswerSchema {
    fn new(schema: AnswerSchema) -> Result<Self, ProblemInvariantError> {
        if schema.is_structurally_valid() {
            Ok(Self(schema))
        } else {
            Err(ProblemInvariantError::InvalidAnswerSchema)
        }
    }

    const fn as_schema(&self) -> &AnswerSchema {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CanonicalAnswer(AnswerNode);

impl CanonicalAnswer {
    fn new(
        schema: &AnswerSchema,
        contract: crate::theme::ThemeAnswerContract,
        prompt: &ProblemPrompt,
        answer: AnswerNode,
    ) -> Result<Self, ProblemInvariantError> {
        if !schema.accepts_canonical_answer(&answer) {
            return Err(ProblemInvariantError::CanonicalAnswer);
        }
        if !answer_matches_contract(contract, prompt, schema, &answer) {
            return Err(ProblemInvariantError::AnswerContract);
        }
        if !crate::semantics::prompt_accepts_canonical_answer(contract, prompt, schema, &answer) {
            return Err(ProblemInvariantError::AnswerSemantics);
        }
        Ok(Self(answer))
    }

    const fn as_node(&self) -> &AnswerNode {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Problem {
    schema_version: u16,
    id: u32,
    numeric_theme_id: u32,
    prompt: ProblemPrompt,
    input_interface: AnswerInputInterface,
    column_input: Option<ColumnArithmeticInput>,
    answer_schema: ValidatedAnswerSchema,
    canonical_answer: CanonicalAnswer,
    worked_solution: Option<WorkedSolution>,
    effort_model: EffortModel,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ProblemInvariantError {
    PromptKind,
    WireIntegerRange,
    AnswerSchemaKind,
    InvalidAnswerSchema,
    CanonicalAnswer,
    AnswerContract,
    AnswerSemantics,
    WorkedSolution,
}

impl ProblemInvariantError {
    pub(crate) const fn description(self) -> &'static str {
        match self {
            Self::PromptKind => "prompt does not satisfy the registered theme contract",
            Self::WireIntegerRange => {
                "problem contains an integer that cannot cross the JavaScript wire exactly"
            }
            Self::AnswerSchemaKind => {
                "answer schema kind does not satisfy the registered theme contract"
            }
            Self::InvalidAnswerSchema => "answer schema is structurally invalid",
            Self::CanonicalAnswer => "canonical answer does not satisfy the answer schema",
            Self::AnswerContract => {
                "canonical answer does not satisfy the registered theme contract"
            }
            Self::AnswerSemantics => "canonical answer does not solve the problem prompt",
            Self::WorkedSolution => "worked solution cannot be derived from the problem semantics",
        }
    }
}

impl LiarStatement {
    pub(crate) fn is_true_for_mask(&self, mask: u32) -> bool {
        let is_liar = |person: PersonIndex| ((mask >> u32::from(person.value() - 1)) & 1) == 1;
        match *self {
            Self::SaysLiar { person } => is_liar(person),
            Self::SaysNotLiar { person } => !is_liar(person),
            Self::ExactlyOneLiar { first, second } => is_liar(first) ^ is_liar(second),
            Self::ExactLiarCount { count } => mask.count_ones() == u32::from(count.value()),
            Self::BothLiar { first, second } => is_liar(first) && is_liar(second),
            Self::BothNotLiar { first, second } => !is_liar(first) && !is_liar(second),
        }
    }

    fn is_valid_for(&self, people_count: PeopleCount) -> bool {
        let within = |person: PersonIndex| person.value() <= people_count.value();
        match *self {
            Self::SaysLiar { person } | Self::SaysNotLiar { person } => within(person),
            Self::ExactlyOneLiar { first, second }
            | Self::BothLiar { first, second }
            | Self::BothNotLiar { first, second } => {
                first != second && within(first) && within(second)
            }
            Self::ExactLiarCount { count } => count.value() < people_count.value(),
        }
    }
}

impl ArithmeticExpression {
    fn has_js_safe_wire_values(&self) -> bool {
        match self {
            Self::Integer { value } => is_js_safe_integer(*value),
            Self::Rational { value } => value.has_js_safe_wire_values(),
            Self::ExactDecimal { coefficient, .. } => is_js_safe_integer(*coefficient),
            Self::Binary { left, right, .. } => {
                left.has_js_safe_wire_values() && right.has_js_safe_wire_values()
            }
        }
    }
}

impl ProblemPrompt {
    fn has_js_safe_wire_values(&self) -> bool {
        match self {
            Self::Addition { .. } | Self::LiarPuzzle { .. } | Self::MiniSudoku { .. } => true,
            Self::Arithmetic { expression } => expression.has_js_safe_wire_values(),
            Self::ColumnArithmetic { left, right, .. } => {
                left.has_js_safe_wire_values() && right.has_js_safe_wire_values()
            }
            Self::LinearEquation { a, b, c, d, .. } => [a, b, c, d]
                .iter()
                .all(|value| value.has_js_safe_wire_values()),
            Self::QuadraticEquation { a, b, c, .. } => [a, b, c]
                .iter()
                .all(|value| value.has_js_safe_wire_values()),
            Self::SimultaneousEquation { a, b, c, d, e, f } => [a, b, c, d, e, f]
                .iter()
                .all(|value| is_js_safe_integer(**value)),
        }
    }

    fn is_structurally_valid(&self) -> bool {
        match self {
            Self::Addition { left, right } => (1..=9).contains(left) && (1..=9).contains(right),
            Self::LiarPuzzle {
                people_count,
                statements,
            } => {
                statements.len() == usize::from(people_count.value())
                    && statements
                        .iter()
                        .all(|statement| statement.is_valid_for(*people_count))
            }
            Self::MiniSudoku { .. }
            | Self::Arithmetic { .. }
            | Self::ColumnArithmetic { .. }
            | Self::LinearEquation { .. }
            | Self::QuadraticEquation { .. }
            | Self::SimultaneousEquation { .. } => true,
        }
    }

    pub(crate) const fn theme_kind(&self) -> ThemePromptKind {
        match self {
            Self::Addition { .. } => ThemePromptKind::Addition,
            Self::Arithmetic { .. } => ThemePromptKind::Arithmetic,
            Self::ColumnArithmetic { .. } => ThemePromptKind::ColumnArithmetic,
            Self::LinearEquation { .. } => ThemePromptKind::LinearEquation,
            Self::QuadraticEquation { .. } => ThemePromptKind::QuadraticEquation,
            Self::SimultaneousEquation { .. } => ThemePromptKind::SimultaneousEquation,
            Self::LiarPuzzle { .. } => ThemePromptKind::LiarPuzzle,
            Self::MiniSudoku { .. } => ThemePromptKind::MiniSudoku,
        }
    }
}

impl AnswerSchema {
    pub(crate) const fn theme_kind(&self) -> ThemeAnswerSchemaKind {
        match self {
            Self::Integer { .. } => ThemeAnswerSchemaKind::Integer,
            Self::Rational { .. } => ThemeAnswerSchemaKind::Rational,
            Self::Decimal { .. } | Self::RoundedDecimal { .. } => ThemeAnswerSchemaKind::Decimal,
            Self::DecimalDivisionRemainder { .. } | Self::OrderedPair => {
                ThemeAnswerSchemaKind::OrderedPair
            }
            Self::OrderedTuple { .. } => ThemeAnswerSchemaKind::OrderedTuple,
            Self::Algebraic => ThemeAnswerSchemaKind::Algebraic,
        }
    }

    pub fn is_structurally_valid(&self) -> bool {
        match self {
            Self::Integer { min, max } => min <= max,
            Self::Rational {
                max_denominator, ..
            } => *max_denominator > 0,
            Self::OrderedTuple { length } => *length > 0,
            Self::Decimal { .. }
            | Self::DecimalDivisionRemainder { .. }
            | Self::RoundedDecimal { .. }
            | Self::OrderedPair
            | Self::Algebraic => true,
        }
    }

    pub fn accepts_canonical_answer(&self, answer: &AnswerNode) -> bool {
        if !answer.is_within_structural_node_limit() || !answer.is_generated_answer() {
            return false;
        }
        match self {
            Self::Integer { min, max } => crate::exact_value::rational_parts_from_answer(answer)
                .is_some_and(|(numerator, denominator)| {
                    denominator == 1
                        && numerator >= i128::from(*min)
                        && numerator <= i128::from(*max)
                }),
            Self::Rational {
                max_abs_numerator,
                max_denominator,
                ..
            } => crate::exact_value::rational_parts_from_answer(answer).is_some_and(
                |(numerator, denominator)| {
                    numerator.unsigned_abs() <= u128::from(*max_abs_numerator)
                        && denominator > 0
                        && (denominator as u128) <= u128::from(*max_denominator)
                },
            ),
            Self::Decimal { max_scale } => match answer {
                AnswerNode::Integer(_) => true,
                AnswerNode::ExactDecimal { scale, .. } => scale <= max_scale,
                _ => false,
            },
            Self::RoundedDecimal {
                scale: target_scale,
            } => match answer {
                AnswerNode::Integer(_) => true,
                AnswerNode::ExactDecimal { scale, .. } => scale <= target_scale,
                _ => false,
            },
            Self::DecimalDivisionRemainder {
                quotient_scale,
                remainder_max_scale,
            } => {
                let AnswerNode::Tuple(values) = answer else {
                    return false;
                };
                if values.len() != 2 {
                    return false;
                }
                let accepts_decimal = |value: &AnswerNode, max_scale: u32| match value {
                    AnswerNode::Integer(_) => true,
                    AnswerNode::ExactDecimal { scale, .. } => *scale <= max_scale,
                    _ => false,
                };
                accepts_decimal(&values[0], *quotient_scale)
                    && accepts_decimal(&values[1], *remainder_max_scale)
            }
            Self::OrderedPair => {
                matches!(answer, AnswerNode::Tuple(values) if values.len() == 2)
            }
            Self::OrderedTuple { length } => {
                matches!(answer, AnswerNode::Tuple(values) if values.len() == usize::from(*length))
            }
            Self::Algebraic => true,
        }
    }
}

fn answer_matches_contract(
    contract: crate::theme::ThemeAnswerContract,
    prompt: &ProblemPrompt,
    schema: &AnswerSchema,
    answer: &AnswerNode,
) -> bool {
    match contract {
        crate::theme::ThemeAnswerContract::DigitGrid(spec) => {
            matches!(
                schema,
                AnswerSchema::OrderedTuple { length } if *length == spec.cell_count()
            ) && matches!(answer, AnswerNode::Tuple(values)
            if values.len() == usize::from(spec.cell_count())
                && values.iter().all(|value| matches!(
                    value,
                    AnswerNode::Integer(digit)
                        if (i64::from(spec.min_digit())..=i64::from(spec.max_digit())).contains(digit)
                )))
        }
        crate::theme::ThemeAnswerContract::LiarPuzzle => {
            let ProblemPrompt::LiarPuzzle { people_count, .. } = prompt else {
                return false;
            };
            let AnswerNode::Tuple(values) = answer else {
                return false;
            };
            if values.is_empty() || values.len() >= usize::from(people_count.value()) {
                return false;
            }
            let mut previous = 0_i64;
            values.iter().all(|value| {
                let AnswerNode::Integer(person) = value else {
                    return false;
                };
                let valid = *person > previous && *person <= i64::from(people_count.value());
                previous = *person;
                valid
            })
        }
        _ => true,
    }
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

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "wire-types", derive(TS))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ColumnDecimalPointInput {
    None,
    Fixed { scale: u32 },
    Editable,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "wire-types", derive(TS))]
pub struct ColumnAnswerPartInput {
    pub order: ColumnInputOrder,
    pub decimal_point: ColumnDecimalPointInput,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "wire-types", derive(TS))]
pub struct ColumnArithmeticInput {
    pub single: Option<ColumnAnswerPartInput>,
    pub quotient: Option<ColumnAnswerPartInput>,
    pub remainder: Option<ColumnAnswerPartInput>,
}

fn resolve_column_answer_part_input(
    policy: Option<ColumnAnswerPartInputPolicy>,
    answer: &AnswerNode,
) -> Option<ColumnAnswerPartInput> {
    policy.map(|policy| ColumnAnswerPartInput {
        order: policy.order(),
        decimal_point: match policy.decimal_point() {
            ColumnDecimalPointPolicy::None => ColumnDecimalPointInput::None,
            ColumnDecimalPointPolicy::FixedCanonicalScale => ColumnDecimalPointInput::Fixed {
                scale: answer_scale(answer),
            },
            ColumnDecimalPointPolicy::Editable => ColumnDecimalPointInput::Editable,
        },
    })
}

fn column_arithmetic_input(
    registration: &ThemeRegistration,
    answer: &AnswerNode,
) -> Option<ColumnArithmeticInput> {
    let policy = registration.presentation().column_input()?;
    let (quotient, remainder) = match answer {
        AnswerNode::Tuple(values) => (
            values.first().unwrap_or(answer),
            values.get(1).unwrap_or(answer),
        ),
        _ => (answer, answer),
    };
    Some(ColumnArithmeticInput {
        single: resolve_column_answer_part_input(policy.single(), answer),
        quotient: resolve_column_answer_part_input(policy.quotient(), quotient),
        remainder: resolve_column_answer_part_input(policy.remainder(), remainder),
    })
}

impl WorkedSolution {
    fn for_column_arithmetic(
        operator: ArithmeticOperator,
        left: &ArithmeticExpression,
        right: &ArithmeticExpression,
        answer_schema: &AnswerSchema,
        answer: &AnswerNode,
    ) -> Option<Self> {
        let kind = match operator {
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
                WorkedSolutionKind::ColumnMultiplication { partial_products }
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
                let calculation_scale = match answer_schema {
                    AnswerSchema::RoundedDecimal { scale } => scale.checked_add(1)?,
                    _ => quotient_scale,
                };
                let target_scale = normalized_dividend_scale.max(calculation_scale);
                let appended_zeros = target_scale.checked_sub(normalized_dividend_scale)?;
                let dividend_magnitude = normalized_dividend_coefficient.unsigned_abs();
                let base_digits = format!(
                    "{:0width$}",
                    dividend_magnitude,
                    width = normalized_dividend_scale as usize + 1
                );
                let remainder_stop_digits = match answer_schema {
                    AnswerSchema::DecimalDivisionRemainder { quotient_scale, .. } => {
                        let integer_digits = base_digits
                            .len()
                            .checked_sub(usize::try_from(normalized_dividend_scale).ok()?)?;
                        Some(
                            integer_digits
                                .checked_add(usize::try_from(*quotient_scale).ok()?)?
                                .min(base_digits.len()),
                        )
                    }
                    _ => None,
                };
                let mut digits = base_digits;
                digits.extend(std::iter::repeat_n('0', appended_zeros as usize));
                let dividend_coefficient = i64::try_from(
                    dividend_magnitude.checked_mul(10_u64.checked_pow(appended_zeros)?)?,
                )
                .ok()?;

                let mut steps = Vec::new();
                let mut current = 0_i64;
                let mut started = false;
                let digit_bytes = digits.as_bytes();
                let processed_digits = remainder_stop_digits.unwrap_or(digit_bytes.len());
                for (index, byte) in digit_bytes.iter().take(processed_digits).enumerate() {
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
                WorkedSolutionKind::LongDivision {
                    divisor,
                    dividend_coefficient,
                    dividend_scale: target_scale,
                    quotient_trailing_cells: target_scale.saturating_sub(quotient_scale),
                    steps,
                }
            }
            ArithmeticOperator::Add | ArithmeticOperator::Subtract => return None,
        };
        Some(Self { kind })
    }

    fn for_prompt(
        prompt: &ProblemPrompt,
        answer_schema: &AnswerSchema,
        answer: &AnswerNode,
    ) -> Result<Option<Self>, ProblemInvariantError> {
        match prompt {
            ProblemPrompt::ColumnArithmetic {
                operator: operator @ (ArithmeticOperator::Multiply | ArithmeticOperator::Divide),
                left,
                right,
            } => Self::for_column_arithmetic(*operator, left, right, answer_schema, answer)
                .map(Some)
                .ok_or(ProblemInvariantError::WorkedSolution),
            ProblemPrompt::ColumnArithmetic {
                operator: ArithmeticOperator::Add | ArithmeticOperator::Subtract,
                ..
            }
            | ProblemPrompt::Addition { .. }
            | ProblemPrompt::Arithmetic { .. }
            | ProblemPrompt::LinearEquation { .. }
            | ProblemPrompt::QuadraticEquation { .. }
            | ProblemPrompt::SimultaneousEquation { .. }
            | ProblemPrompt::LiarPuzzle { .. }
            | ProblemPrompt::MiniSudoku { .. } => Ok(None),
        }
    }

    fn has_js_safe_wire_values(&self) -> bool {
        match &self.kind {
            WorkedSolutionKind::ColumnMultiplication { partial_products } => partial_products
                .iter()
                .all(|partial| is_js_safe_integer(partial.value)),
            WorkedSolutionKind::LongDivision {
                divisor,
                dividend_coefficient,
                steps,
                ..
            } => {
                is_js_safe_integer(*divisor)
                    && is_js_safe_integer(*dividend_coefficient)
                    && steps.iter().all(|step| {
                        is_js_safe_integer(step.product) && is_js_safe_integer(step.after)
                    })
            }
        }
    }

    pub(crate) fn to_wire(&self) -> crate::wire::WorkedSolutionWire {
        match &self.kind {
            WorkedSolutionKind::ColumnMultiplication { partial_products } => {
                crate::wire::WorkedSolutionWire::ColumnMultiplication {
                    partial_products: partial_products
                        .iter()
                        .map(|partial| crate::wire::ColumnMultiplicationPartialWire {
                            value: partial.value,
                            place: partial.place,
                        })
                        .collect(),
                }
            }
            WorkedSolutionKind::LongDivision {
                divisor,
                dividend_coefficient,
                dividend_scale,
                quotient_trailing_cells,
                steps,
            } => crate::wire::WorkedSolutionWire::LongDivision {
                divisor: *divisor,
                dividend_coefficient: *dividend_coefficient,
                dividend_scale: *dividend_scale,
                quotient_trailing_cells: *quotient_trailing_cells,
                steps: steps
                    .iter()
                    .map(|step| crate::wire::LongDivisionStepWire {
                        product: step.product,
                        after: step.after,
                        product_offset: step.product_offset,
                        after_offset: step.after_offset,
                    })
                    .collect(),
            },
        }
    }
}

impl Problem {
    pub(crate) fn generated(
        registration: &ThemeRegistration,
        id: u32,
        prompt: ProblemPrompt,
        answer_schema: AnswerSchema,
        canonical_answer: AnswerNode,
        effort_model: EffortModel,
    ) -> Result<Self, ProblemInvariantError> {
        if prompt.theme_kind() != registration.answer_contract().prompt_kind()
            || !prompt.is_structurally_valid()
        {
            return Err(ProblemInvariantError::PromptKind);
        }
        if !prompt.has_js_safe_wire_values() {
            return Err(ProblemInvariantError::WireIntegerRange);
        }
        if answer_schema.theme_kind() != registration.answer_contract().answer_schema_kind() {
            return Err(ProblemInvariantError::AnswerSchemaKind);
        }
        let answer_schema = ValidatedAnswerSchema::new(answer_schema)?;
        let canonical_answer = CanonicalAnswer::new(
            answer_schema.as_schema(),
            registration.answer_contract(),
            &prompt,
            canonical_answer,
        )?;
        let worked_solution = WorkedSolution::for_prompt(
            &prompt,
            answer_schema.as_schema(),
            canonical_answer.as_node(),
        )?;
        if worked_solution
            .as_ref()
            .is_some_and(|solution| !solution.has_js_safe_wire_values())
        {
            return Err(ProblemInvariantError::WireIntegerRange);
        }
        let column_input = column_arithmetic_input(registration, canonical_answer.as_node());
        Ok(Self {
            schema_version: SCHEMA_VERSION,
            id,
            numeric_theme_id: registration.numeric_theme_id(),
            prompt,
            input_interface: crate::input::input_interface(
                registration.answer_contract().input_profile(),
            ),
            column_input,
            answer_schema,
            canonical_answer,
            worked_solution,
            effort_model,
        })
    }

    pub const fn schema_version(&self) -> u16 {
        self.schema_version
    }

    pub const fn id(&self) -> u32 {
        self.id
    }

    pub const fn numeric_theme_id(&self) -> u32 {
        self.numeric_theme_id
    }

    pub(crate) const fn prompt(&self) -> &ProblemPrompt {
        &self.prompt
    }

    pub const fn input_interface(&self) -> &AnswerInputInterface {
        &self.input_interface
    }

    pub const fn column_input(&self) -> Option<&ColumnArithmeticInput> {
        self.column_input.as_ref()
    }

    pub const fn answer_schema(&self) -> &AnswerSchema {
        self.answer_schema.as_schema()
    }

    pub const fn canonical_answer(&self) -> &AnswerNode {
        self.canonical_answer.as_node()
    }

    pub(crate) const fn worked_solution(&self) -> Option<&WorkedSolution> {
        self.worked_solution.as_ref()
    }

    pub fn effort(&self) -> f64 {
        self.effort_model.value()
    }

    #[cfg(feature = "qa-diagnostics")]
    #[doc(hidden)]
    pub fn qa_effort_operation_vector(&self) -> Option<[f64; crate::effort::OPERATION_KIND_COUNT]> {
        self.effort_model.qa_operation_vector()
    }

    #[cfg(feature = "qa-diagnostics")]
    #[doc(hidden)]
    pub const fn qa_effort_model_kind(&self) -> &'static str {
        self.effort_model.qa_model_kind()
    }

    #[cfg(test)]
    pub(crate) fn operation_plan(&self) -> Option<&OperationPlan> {
        self.effort_model.operation_plan()
    }

    #[cfg(test)]
    pub(crate) fn operation_vector(&self) -> OperationVector {
        self.effort_model.operation_vector()
    }

    #[cfg(test)]
    pub(crate) fn theme_specific_effort(&self) -> Option<f64> {
        self.effort_model.theme_specific_value()
    }

    pub(crate) fn assign_worksheet_position(&mut self, id: u32, schema_version: u16) {
        self.id = id;
        self.schema_version = schema_version;
    }
}

impl Serialize for Problem {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        crate::wire::ProblemWire::from(self).serialize(serializer)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "wire-types", derive(TS))]
pub struct LayoutMetadata {
    pub problem_count: u32,
    pub columns: u32,
    pub rows: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Worksheet {
    identity: ProblemSetIdentity,
    skill_id: String,
    curriculum_path: Vec<String>,
    layout: LayoutMetadata,
    problems: Vec<Problem>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum WorksheetInvariantError {
    Identity,
    ProblemCount,
    ProblemIdentity,
}

impl WorksheetInvariantError {
    pub(crate) const fn description(self) -> &'static str {
        match self {
            Self::Identity => "worksheet identity does not match the registered theme",
            Self::ProblemCount => "worksheet problem count does not match the registered layout",
            Self::ProblemIdentity => {
                "worksheet contains a problem from a different schema or theme"
            }
        }
    }
}

impl Worksheet {
    pub(crate) fn generated(
        identity: ProblemSetIdentity,
        registration: &ThemeRegistration,
        problems: Vec<Problem>,
    ) -> Result<Self, WorksheetInvariantError> {
        if identity.schema_version() != SCHEMA_VERSION
            || identity.numeric_theme_id() != registration.numeric_theme_id()
            || identity.generator_revision() != registration.generator_revision()
        {
            return Err(WorksheetInvariantError::Identity);
        }
        if problems.len() != registration.layout().problem_count() {
            return Err(WorksheetInvariantError::ProblemCount);
        }
        if problems.iter().any(|problem| {
            problem.schema_version() != identity.schema_version()
                || problem.numeric_theme_id() != identity.numeric_theme_id()
        }) {
            return Err(WorksheetInvariantError::ProblemIdentity);
        }
        Ok(Self {
            identity,
            skill_id: registration.skill_id().to_owned(),
            curriculum_path: registration
                .curriculum_path()
                .iter()
                .map(|segment| (*segment).to_owned())
                .collect(),
            layout: LayoutMetadata {
                problem_count: registration.layout().problem_count_wire(),
                columns: registration.layout().columns_wire(),
                rows: registration.layout().rows_wire(),
            },
            problems,
        })
    }

    pub const fn schema_version(&self) -> u16 {
        self.identity.schema_version()
    }
    pub fn problem_set_id(&self) -> String {
        self.identity.to_string()
    }
    pub const fn identity(&self) -> &ProblemSetIdentity {
        &self.identity
    }
    pub fn skill_id(&self) -> &str {
        &self.skill_id
    }
    pub fn curriculum_path(&self) -> &[String] {
        &self.curriculum_path
    }
    pub const fn layout(&self) -> &LayoutMetadata {
        &self.layout
    }
    pub fn problems(&self) -> &[Problem] {
        &self.problems
    }
    #[cfg(test)]
    pub(crate) fn into_problems(self) -> Vec<Problem> {
        self.problems
    }
}

impl Serialize for Worksheet {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        crate::wire::WorksheetWire::from(self).serialize(serializer)
    }
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

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GradeResult {
    status: GradeStatus,
    expected: AnswerNode,
    actual: AnswerNode,
    warnings: Vec<GradeWarning>,
}

impl GradeResult {
    pub(crate) fn new(
        status: GradeStatus,
        expected: AnswerNode,
        actual: AnswerNode,
        warnings: Vec<GradeWarning>,
    ) -> Self {
        Self {
            status,
            expected,
            actual,
            warnings,
        }
    }

    pub const fn status(&self) -> GradeStatus {
        self.status
    }
    pub fn is_correct(&self) -> bool {
        matches!(self.status, GradeStatus::Correct)
    }
    pub const fn expected(&self) -> &AnswerNode {
        &self.expected
    }
    pub const fn actual(&self) -> &AnswerNode {
        &self.actual
    }
    pub fn warnings(&self) -> &[GradeWarning] {
        &self.warnings
    }
}

impl Serialize for GradeResult {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        crate::wire::GradeResultWire::from(self).serialize(serializer)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "wire-types", derive(TS))]
pub struct GenerateWorksheetRequest {
    pub schema_version: u16,
    pub numeric_theme_id: u32,
    pub seed: String,
    pub difficulty: Difficulty,
    #[serde(default)]
    #[cfg_attr(feature = "wire-types", ts(type = "number | null"))]
    pub timeout_ms: Option<u64>,
    #[serde(default)]
    #[cfg_attr(feature = "wire-types", ts(type = "number | null"))]
    pub max_attempts: Option<u64>,
}

impl GenerateWorksheetRequest {
    pub fn new(numeric_theme_id: u32, seed: impl Into<String>, difficulty: Difficulty) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            numeric_theme_id,
            seed: seed.into(),
            difficulty,
            timeout_ms: None,
            max_attempts: None,
        }
    }
}

#[cfg(test)]
mod invariant_tests {
    use super::*;
    use crate::effort::OperationPlan;
    use crate::themes::basic_arithmetic::{
        ONE_DIGIT_ADDITION_REGISTRATION, ONE_DIGIT_SUBTRACTION_REGISTRATION,
    };
    use crate::themes::column_arithmetic::COLUMN_MULTIPLY_1DIGIT_REGISTRATION;
    use crate::themes::equations::{
        LINEAR_EQUATION_1_REGISTRATION, QUADRATIC_EQUATION_1_REGISTRATION,
        SIMULTANEOUS_EQUATION_1_REGISTRATION,
    };
    use crate::themes::liar_puzzle::LIAR_PUZZLE_REGISTRATION;
    use crate::themes::mini_sudoku::MINI_SUDOKU_REGISTRATION;

    fn empty_effort() -> EffortModel {
        EffortModel::operations(OperationPlan::default())
    }

    #[test]
    fn generated_problem_rejects_integers_that_are_not_exact_in_javascript() {
        let unsafe_value = JS_SAFE_INTEGER_MAX + 1;
        assert_eq!(
            Problem::generated(
                &SIMULTANEOUS_EQUATION_1_REGISTRATION,
                1,
                ProblemPrompt::SimultaneousEquation {
                    a: unsafe_value,
                    b: 1,
                    c: 1,
                    d: 1,
                    e: 2,
                    f: 2,
                },
                AnswerSchema::OrderedPair,
                AnswerNode::Tuple(vec![AnswerNode::Integer(0), AnswerNode::Integer(1)]),
                empty_effort(),
            ),
            Err(ProblemInvariantError::WireIntegerRange)
        );
    }

    #[test]
    fn generated_problem_rejects_invalid_schema_and_out_of_range_answer() {
        let prompt = ProblemPrompt::Addition { left: 1, right: 1 };
        assert_eq!(
            Problem::generated(
                &ONE_DIGIT_ADDITION_REGISTRATION,
                1,
                prompt.clone(),
                AnswerSchema::Integer { min: 2, max: 1 },
                AnswerNode::Integer(2),
                empty_effort(),
            ),
            Err(ProblemInvariantError::InvalidAnswerSchema)
        );
        assert_eq!(
            Problem::generated(
                &ONE_DIGIT_ADDITION_REGISTRATION,
                1,
                prompt,
                AnswerSchema::Integer { min: 1, max: 18 },
                AnswerNode::Integer(19),
                empty_effort(),
            ),
            Err(ProblemInvariantError::CanonicalAnswer)
        );
    }

    #[test]
    fn digit_grid_contract_validates_tuple_length_and_digit_domain_from_one_spec() {
        let givens = MiniSudokuGrid::new([None; MINI_SUDOKU_CELL_COUNT]).unwrap();
        let prompt = ProblemPrompt::MiniSudoku { givens };
        assert_eq!(
            Problem::generated(
                &MINI_SUDOKU_REGISTRATION,
                1,
                prompt.clone(),
                AnswerSchema::OrderedTuple { length: 1 },
                AnswerNode::Tuple(vec![AnswerNode::Integer(1)]),
                empty_effort(),
            ),
            Err(ProblemInvariantError::AnswerContract)
        );
        assert_eq!(
            Problem::generated(
                &MINI_SUDOKU_REGISTRATION,
                1,
                prompt,
                AnswerSchema::OrderedTuple {
                    length: MINI_SUDOKU_GRID_SPEC.cell_count(),
                },
                AnswerNode::Tuple(vec![AnswerNode::Integer(5); MINI_SUDOKU_CELL_COUNT]),
                empty_effort(),
            ),
            Err(ProblemInvariantError::AnswerContract)
        );
    }

    #[test]
    fn liar_count_type_and_statement_validation_encode_only_nontrivial_counts() {
        assert!(LiarCount::new(0).is_none());
        assert!(LiarCount::new(1).is_some());
        assert!(LiarCount::new(3).is_some());
        assert!(LiarCount::new(MAX_LIAR_PUZZLE_PEOPLE).is_none());

        let count_three = LiarStatement::ExactLiarCount {
            count: LiarCount::new(3).unwrap(),
        };
        assert!(!count_three.is_valid_for(PeopleCount::new(3).unwrap()));
        assert!(count_three.is_valid_for(PeopleCount::new(4).unwrap()));
    }

    #[test]
    fn liar_contract_rejects_answers_outside_the_people_domain() {
        let people_count = PeopleCount::new(3).unwrap();
        let statements = vec![
            LiarStatement::SaysLiar {
                person: PersonIndex::new(2).unwrap(),
            },
            LiarStatement::SaysNotLiar {
                person: PersonIndex::new(3).unwrap(),
            },
            LiarStatement::ExactLiarCount {
                count: LiarCount::new(1).unwrap(),
            },
        ];
        assert_eq!(
            Problem::generated(
                &LIAR_PUZZLE_REGISTRATION,
                1,
                ProblemPrompt::LiarPuzzle {
                    people_count,
                    statements,
                },
                AnswerSchema::Algebraic,
                AnswerNode::Tuple(vec![AnswerNode::Integer(999)]),
                empty_effort(),
            ),
            Err(ProblemInvariantError::AnswerContract)
        );
    }

    #[test]
    fn generated_problem_rejects_prompt_answer_semantic_mismatches_across_families() {
        let wrong_addition = Problem::generated(
            &ONE_DIGIT_ADDITION_REGISTRATION,
            1,
            ProblemPrompt::Addition { left: 1, right: 1 },
            AnswerSchema::Integer { min: 1, max: 18 },
            AnswerNode::Integer(3),
            empty_effort(),
        );
        assert_eq!(wrong_addition, Err(ProblemInvariantError::AnswerSemantics));

        let wrong_arithmetic = Problem::generated(
            &ONE_DIGIT_SUBTRACTION_REGISTRATION,
            1,
            ProblemPrompt::Arithmetic {
                expression: ArithmeticExpression::Binary {
                    operator: ArithmeticOperator::Subtract,
                    left: Box::new(ArithmeticExpression::Integer { value: 5 }),
                    right: Box::new(ArithmeticExpression::Integer { value: 2 }),
                },
            },
            AnswerSchema::Integer { min: 1, max: 9 },
            AnswerNode::Integer(4),
            empty_effort(),
        );
        assert_eq!(
            wrong_arithmetic,
            Err(ProblemInvariantError::AnswerSemantics)
        );

        let wrong_column = Problem::generated(
            &COLUMN_MULTIPLY_1DIGIT_REGISTRATION,
            1,
            ProblemPrompt::ColumnArithmetic {
                operator: ArithmeticOperator::Multiply,
                left: ArithmeticExpression::Integer { value: 12 },
                right: ArithmeticExpression::Integer { value: 3 },
            },
            AnswerSchema::Integer { min: 0, max: 100 },
            AnswerNode::Integer(35),
            empty_effort(),
        );
        assert_eq!(wrong_column, Err(ProblemInvariantError::AnswerSemantics));

        let wrong_linear = Problem::generated(
            &LINEAR_EQUATION_1_REGISTRATION,
            1,
            ProblemPrompt::LinearEquation {
                a: RationalCoefficient::new(1, 1).unwrap(),
                b: RationalCoefficient::zero(),
                c: RationalCoefficient::zero(),
                d: RationalCoefficient::new(1, 1).unwrap(),
                left_negative_constant_as_subtraction: false,
                right_negative_constant_as_subtraction: false,
            },
            AnswerSchema::Integer { min: -15, max: 15 },
            AnswerNode::Integer(2),
            empty_effort(),
        );
        assert_eq!(wrong_linear, Err(ProblemInvariantError::AnswerSemantics));

        let wrong_quadratic = Problem::generated(
            &QUADRATIC_EQUATION_1_REGISTRATION,
            1,
            ProblemPrompt::QuadraticEquation {
                form: QuadraticEquationForm::SquareEqualsConstant,
                a: RationalCoefficient::new(1, 1).unwrap(),
                b: RationalCoefficient::zero(),
                c: RationalCoefficient::new(4, 1).unwrap(),
            },
            AnswerSchema::Algebraic,
            AnswerNode::PlusMinus(Box::new(AnswerNode::Integer(3))),
            empty_effort(),
        );
        assert_eq!(wrong_quadratic, Err(ProblemInvariantError::AnswerSemantics));

        let wrong_simultaneous = Problem::generated(
            &SIMULTANEOUS_EQUATION_1_REGISTRATION,
            1,
            ProblemPrompt::SimultaneousEquation {
                a: 1,
                b: 1,
                c: 3,
                d: 1,
                e: -1,
                f: 1,
            },
            AnswerSchema::OrderedPair,
            AnswerNode::Tuple(vec![AnswerNode::Integer(1), AnswerNode::Integer(2)]),
            empty_effort(),
        );
        assert_eq!(
            wrong_simultaneous,
            Err(ProblemInvariantError::AnswerSemantics)
        );

        let people_count = PeopleCount::new(3).unwrap();
        let statements = vec![
            LiarStatement::SaysLiar {
                person: PersonIndex::new(2).unwrap(),
            },
            LiarStatement::SaysLiar {
                person: PersonIndex::new(3).unwrap(),
            },
            LiarStatement::ExactlyOneLiar {
                first: PersonIndex::new(1).unwrap(),
                second: PersonIndex::new(2).unwrap(),
            },
        ];
        assert_eq!(
            crate::semantics::liar_solutions(people_count, &statements),
            vec![2]
        );
        let wrong_liar = Problem::generated(
            &LIAR_PUZZLE_REGISTRATION,
            1,
            ProblemPrompt::LiarPuzzle {
                people_count,
                statements,
            },
            AnswerSchema::Algebraic,
            AnswerNode::Tuple(vec![AnswerNode::Integer(1)]),
            empty_effort(),
        );
        assert_eq!(wrong_liar, Err(ProblemInvariantError::AnswerSemantics));

        let solved = [1_u8, 2, 3, 4, 3, 4, 1, 2, 2, 1, 4, 3, 4, 3, 2, 1];
        let givens = MiniSudokuGrid::new(std::array::from_fn(|index| {
            (index + 1 != MINI_SUDOKU_CELL_COUNT).then_some(solved[index])
        }))
        .unwrap();
        let mut wrong_board = solved;
        wrong_board[MINI_SUDOKU_CELL_COUNT - 1] = 2;
        let wrong_sudoku = Problem::generated(
            &MINI_SUDOKU_REGISTRATION,
            1,
            ProblemPrompt::MiniSudoku { givens },
            AnswerSchema::OrderedTuple {
                length: MINI_SUDOKU_GRID_SPEC.cell_count(),
            },
            AnswerNode::Tuple(
                wrong_board
                    .into_iter()
                    .map(|digit| AnswerNode::Integer(i64::from(digit)))
                    .collect(),
            ),
            empty_effort(),
        );
        assert_eq!(wrong_sudoku, Err(ProblemInvariantError::AnswerSemantics));
    }

    #[test]
    fn valid_column_multiplication_derives_its_worked_solution() {
        let problem = Problem::generated(
            &COLUMN_MULTIPLY_1DIGIT_REGISTRATION,
            1,
            ProblemPrompt::ColumnArithmetic {
                operator: ArithmeticOperator::Multiply,
                left: ArithmeticExpression::Integer { value: 12 },
                right: ArithmeticExpression::Integer { value: 3 },
            },
            AnswerSchema::Integer { min: 0, max: 100 },
            AnswerNode::Integer(36),
            empty_effort(),
        )
        .unwrap();
        assert!(problem.worked_solution().is_some());
    }

    #[test]
    fn worked_solution_is_derived_from_prompt_and_rejects_unrenderable_column_semantics() {
        let nested = ArithmeticExpression::Binary {
            operator: ArithmeticOperator::Add,
            left: Box::new(ArithmeticExpression::Integer { value: 2 }),
            right: Box::new(ArithmeticExpression::Integer { value: 3 }),
        };
        assert_eq!(
            Problem::generated(
                &COLUMN_MULTIPLY_1DIGIT_REGISTRATION,
                1,
                ProblemPrompt::ColumnArithmetic {
                    operator: ArithmeticOperator::Multiply,
                    left: nested,
                    right: ArithmeticExpression::Integer { value: 4 },
                },
                AnswerSchema::Integer { min: 0, max: 100 },
                AnswerNode::Integer(20),
                empty_effort(),
            ),
            Err(ProblemInvariantError::WorkedSolution)
        );
    }
}
