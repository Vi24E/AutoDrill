use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};

/// Search/filter taxonomy only. Behavioural capabilities belong to typed policy
/// values rather than being duplicated here as tags.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemeTag {
    Addition,
    Subtraction,
    Multiplication,
    Division,
    Fractions,
    Decimals,
    NegativeNumbers,
    Equations,
    LinearEquation,
    SimultaneousEquation,
    QuadraticEquation,
    Bonus,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CurriculumSafetyPolicy {
    NonNegativeOnly,
    Unrestricted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FractionPresentationPolicy {
    None,
    MixedNumberWhenImproper,
    KeepImproperFraction,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DedupPolicy {
    CanonicalizeCommutative,
    PreserveOperandOrder,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WorksheetPresentation {
    Standard,
    Equation,
    Grid,
    ColumnArithmetic,
}

/// Presentation policy with mutually consistent capabilities.
///
/// In particular, column arithmetic necessarily uses the page grid and carries
/// the print recommendation. Those facts cannot drift independently.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ThemePresentationPolicy {
    worksheet: WorksheetPresentation,
    fraction: FractionPresentationPolicy,
}

impl ThemePresentationPolicy {
    pub const STANDARD: Self = Self {
        worksheet: WorksheetPresentation::Standard,
        fraction: FractionPresentationPolicy::None,
    };

    pub const EQUATION: Self = Self {
        worksheet: WorksheetPresentation::Equation,
        fraction: FractionPresentationPolicy::None,
    };

    pub const WORKSHEET_GRID: Self = Self {
        worksheet: WorksheetPresentation::Grid,
        fraction: FractionPresentationPolicy::None,
    };

    pub const COLUMN_ARITHMETIC: Self = Self {
        worksheet: WorksheetPresentation::ColumnArithmetic,
        fraction: FractionPresentationPolicy::None,
    };

    pub const fn with_fraction(mut self, fraction: FractionPresentationPolicy) -> Self {
        self.fraction = fraction;
        self
    }

    pub const fn worksheet_grid(self) -> bool {
        matches!(
            self.worksheet,
            WorksheetPresentation::Grid | WorksheetPresentation::ColumnArithmetic
        )
    }

    pub const fn column_arithmetic(self) -> bool {
        matches!(self.worksheet, WorksheetPresentation::ColumnArithmetic)
    }

    pub const fn print_recommended(self) -> bool {
        self.column_arithmetic()
    }

    pub const fn equation_layout(self) -> bool {
        matches!(self.worksheet, WorksheetPresentation::Equation)
    }

    pub const fn fraction(self) -> FractionPresentationPolicy {
        self.fraction
    }
}

/// Keep the public Web contract stable while deriving every boolean capability
/// from the single internal worksheet presentation variant.
impl Serialize for ThemePresentationPolicy {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("ThemePresentationPolicy", 5)?;
        state.serialize_field("worksheet_grid", &self.worksheet_grid())?;
        state.serialize_field("column_arithmetic", &self.column_arithmetic())?;
        state.serialize_field("print_recommended", &self.print_recommended())?;
        state.serialize_field("equation_layout", &self.equation_layout())?;
        state.serialize_field("fraction", &self.fraction)?;
        state.end()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemePromptKind {
    Addition,
    Arithmetic,
    ColumnArithmetic,
    LinearEquation,
    QuadraticEquation,
    SimultaneousEquation,
    LiarPuzzle,
    MiniSudoku,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemeAnswerSchemaKind {
    Integer,
    Rational,
    Decimal,
    OrderedPair,
    Algebraic,
    OrderedTuple,
}

/// Validated school grade. Bonus themes use `None` in registration rather than
/// inventing a sentinel grade.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SchoolGrade {
    Elementary1,
    Elementary2,
    Elementary3,
    Elementary4,
    Elementary5,
    Elementary6,
    JuniorHigh1,
    JuniorHigh2,
    JuniorHigh3,
}

impl SchoolGrade {
    pub const fn value(self) -> u8 {
        match self {
            Self::Elementary1 => 1,
            Self::Elementary2 => 2,
            Self::Elementary3 => 3,
            Self::Elementary4 => 4,
            Self::Elementary5 => 5,
            Self::Elementary6 => 6,
            Self::JuniorHigh1 => 7,
            Self::JuniorHigh2 => 8,
            Self::JuniorHigh3 => 9,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct DigitGridSpec {
    min_digit: u8,
    max_digit: u8,
    cell_count: u8,
}

impl DigitGridSpec {
    pub const fn new(min_digit: u8, max_digit: u8, cell_count: u8) -> Self {
        assert!(min_digit <= max_digit, "digit-grid minimum exceeds maximum");
        assert!(cell_count > 0, "digit-grid must contain at least one cell");
        Self {
            min_digit,
            max_digit,
            cell_count,
        }
    }

    pub const fn min_digit(self) -> u8 {
        self.min_digit
    }

    pub const fn max_digit(self) -> u8 {
        self.max_digit
    }

    pub const fn cell_count(self) -> u8 {
        self.cell_count
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemeInputProfile {
    SimplePositive,
    SimpleSigned,
    SimpleDecimal,
    Fraction,
    SignedRational,
    LinearEquation,
    QuadraticEquation,
    SimultaneousEquation,
    JuniorHighFull,
    TupleOnly,
    DigitGrid(DigitGridSpec),
}

/// The finite set of answer contracts that actually exists in AutoDrill.
///
/// This replaces the former unrestricted cartesian product of prompt kind,
/// answer schema kind and input profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ThemeAnswerContract {
    AdditionInteger,
    ArithmeticPositiveInteger,
    ArithmeticSignedInteger,
    ArithmeticSignedRational,
    ArithmeticDecimal,
    ArithmeticFraction,
    LinearInteger,
    LinearRational,
    QuadraticAlgebraic,
    SimultaneousPair,
    ColumnInteger,
    ColumnIntegerDivision,
    ColumnDecimal,
    LiarPuzzle,
    DigitGrid(DigitGridSpec),
}

impl ThemeAnswerContract {
    pub const fn prompt_kind(self) -> ThemePromptKind {
        match self {
            Self::AdditionInteger => ThemePromptKind::Addition,
            Self::ArithmeticPositiveInteger
            | Self::ArithmeticSignedInteger
            | Self::ArithmeticSignedRational
            | Self::ArithmeticDecimal
            | Self::ArithmeticFraction => ThemePromptKind::Arithmetic,
            Self::LinearInteger | Self::LinearRational => ThemePromptKind::LinearEquation,
            Self::QuadraticAlgebraic => ThemePromptKind::QuadraticEquation,
            Self::SimultaneousPair => ThemePromptKind::SimultaneousEquation,
            Self::ColumnInteger | Self::ColumnIntegerDivision | Self::ColumnDecimal => {
                ThemePromptKind::ColumnArithmetic
            }
            Self::LiarPuzzle => ThemePromptKind::LiarPuzzle,
            Self::DigitGrid(_) => ThemePromptKind::MiniSudoku,
        }
    }

    pub const fn answer_schema_kind(self) -> ThemeAnswerSchemaKind {
        match self {
            Self::AdditionInteger
            | Self::ArithmeticPositiveInteger
            | Self::ArithmeticSignedInteger
            | Self::LinearInteger
            | Self::ColumnInteger => ThemeAnswerSchemaKind::Integer,
            Self::ArithmeticSignedRational | Self::ArithmeticFraction | Self::LinearRational => {
                ThemeAnswerSchemaKind::Rational
            }
            Self::ArithmeticDecimal | Self::ColumnDecimal => ThemeAnswerSchemaKind::Decimal,
            Self::SimultaneousPair | Self::ColumnIntegerDivision => {
                ThemeAnswerSchemaKind::OrderedPair
            }
            Self::QuadraticAlgebraic | Self::LiarPuzzle => ThemeAnswerSchemaKind::Algebraic,
            Self::DigitGrid(_) => ThemeAnswerSchemaKind::OrderedTuple,
        }
    }

    pub const fn input_profile(self) -> ThemeInputProfile {
        match self {
            Self::AdditionInteger | Self::ArithmeticPositiveInteger | Self::ColumnInteger => {
                ThemeInputProfile::SimplePositive
            }
            Self::ArithmeticSignedInteger => ThemeInputProfile::SimpleSigned,
            Self::ArithmeticSignedRational => ThemeInputProfile::SignedRational,
            Self::ArithmeticDecimal | Self::ColumnDecimal => ThemeInputProfile::SimpleDecimal,
            Self::ArithmeticFraction => ThemeInputProfile::Fraction,
            Self::LinearInteger | Self::LinearRational => ThemeInputProfile::LinearEquation,
            Self::QuadraticAlgebraic => ThemeInputProfile::QuadraticEquation,
            Self::SimultaneousPair => ThemeInputProfile::SimultaneousEquation,
            Self::ColumnIntegerDivision | Self::LiarPuzzle => ThemeInputProfile::TupleOnly,
            Self::DigitGrid(spec) => ThemeInputProfile::DigitGrid(spec),
        }
    }

    pub const fn digit_grid(self) -> Option<DigitGridSpec> {
        match self {
            Self::DigitGrid(spec) => Some(spec),
            _ => None,
        }
    }
}

impl Serialize for ThemeAnswerContract {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("ThemeAnswerContract", 3)?;
        state.serialize_field("prompt_kind", &self.prompt_kind())?;
        state.serialize_field("answer_schema_kind", &self.answer_schema_kind())?;
        state.serialize_field("input_profile", &self.input_profile())?;
        state.end()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct WorksheetLayoutProfile {
    problem_count: u32,
    columns: u32,
    rows: u32,
}

impl WorksheetLayoutProfile {
    const fn new(problem_count: u32, columns: u32, rows: u32) -> Self {
        assert!(
            problem_count > 0,
            "worksheet must contain at least one problem"
        );
        assert!(columns > 0, "worksheet must contain at least one column");
        assert!(rows > 0, "worksheet must contain at least one row");
        assert!(
            columns * rows >= problem_count,
            "worksheet grid cannot hold all problems"
        );
        Self {
            problem_count,
            columns,
            rows,
        }
    }

    pub const fn problem_count(self) -> usize {
        self.problem_count as usize
    }

    #[cfg(test)]
    pub const fn columns(self) -> usize {
        self.columns as usize
    }

    #[cfg(test)]
    pub const fn rows(self) -> usize {
        self.rows as usize
    }

    pub const fn problem_count_wire(self) -> u32 {
        self.problem_count
    }

    pub const fn columns_wire(self) -> u32 {
        self.columns
    }

    pub const fn rows_wire(self) -> u32 {
        self.rows
    }
}

pub const STANDARD_20_LAYOUT: WorksheetLayoutProfile = WorksheetLayoutProfile::new(20, 2, 10);
pub const COMPACT_16_LAYOUT: WorksheetLayoutProfile = WorksheetLayoutProfile::new(16, 2, 8);
pub const EQUATION_PAIR_12_LAYOUT: WorksheetLayoutProfile = WorksheetLayoutProfile::new(12, 2, 6);
pub const LIAR_6_LAYOUT: WorksheetLayoutProfile = WorksheetLayoutProfile::new(6, 1, 6);
pub const PUZZLE_4_LAYOUT: WorksheetLayoutProfile = WorksheetLayoutProfile::new(4, 2, 2);
pub const COLUMN_16_LAYOUT: WorksheetLayoutProfile = WorksheetLayoutProfile::new(16, 4, 4);
pub const COLUMN_DIVISION_12_LAYOUT: WorksheetLayoutProfile = WorksheetLayoutProfile::new(12, 4, 3);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SamplingLayerSpec {
    pub weight: u32,
    pub minimum: usize,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ThemeId(u32);

impl ThemeId {
    pub const fn new(value: u32) -> Self {
        assert!(value > 0, "theme ID must be nonzero");
        Self(value)
    }
    pub const fn value(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct GeneratorRevision(u32);

impl GeneratorRevision {
    pub const fn new(value: u32) -> Self {
        assert!(value > 0, "generator revision must be nonzero");
        Self(value)
    }
    pub const fn value(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThemeRegistrationSpec {
    pub numeric_theme_id: ThemeId,
    pub generator_revision: GeneratorRevision,
    pub skill_id: &'static str,
    pub curriculum_path: &'static [&'static str],
    pub grade: Option<SchoolGrade>,
    pub tags: &'static [ThemeTag],
    pub safety: CurriculumSafetyPolicy,
    pub presentation: ThemePresentationPolicy,
    pub dedup: DedupPolicy,
    pub answer_contract: ThemeAnswerContract,
    pub layout: WorksheetLayoutProfile,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThemeRegistration {
    theme_id: ThemeId,
    revision: GeneratorRevision,
    skill_id: &'static str,
    curriculum_path: &'static [&'static str],
    grade: Option<SchoolGrade>,
    tags: &'static [ThemeTag],
    safety: CurriculumSafetyPolicy,
    presentation: ThemePresentationPolicy,
    dedup: DedupPolicy,
    answer_contract: ThemeAnswerContract,
    /// Theme-level interactive editor grammar used by the Web input shell.
    editor_input_profile: ThemeInputProfile,
    layout: WorksheetLayoutProfile,
}

impl ThemeRegistration {
    pub const fn new(spec: ThemeRegistrationSpec) -> Self {
        Self {
            theme_id: spec.numeric_theme_id,
            revision: spec.generator_revision,
            skill_id: spec.skill_id,
            curriculum_path: spec.curriculum_path,
            grade: spec.grade,
            tags: spec.tags,
            safety: spec.safety,
            presentation: spec.presentation,
            dedup: spec.dedup,
            answer_contract: spec.answer_contract,
            editor_input_profile: spec.answer_contract.input_profile(),
            layout: spec.layout,
        }
    }

    pub const fn numeric_theme_id(self) -> u32 {
        self.theme_id.value()
    }

    pub const fn generator_revision(self) -> u32 {
        self.revision.value()
    }

    pub const fn skill_id(self) -> &'static str {
        self.skill_id
    }

    pub const fn curriculum_path(self) -> &'static [&'static str] {
        self.curriculum_path
    }

    pub const fn grade(self) -> Option<SchoolGrade> {
        self.grade
    }

    pub const fn tags(self) -> &'static [ThemeTag] {
        self.tags
    }

    pub const fn safety(self) -> CurriculumSafetyPolicy {
        self.safety
    }

    pub const fn presentation(self) -> ThemePresentationPolicy {
        self.presentation
    }

    pub const fn dedup(self) -> DedupPolicy {
        self.dedup
    }

    pub const fn answer_contract(self) -> ThemeAnswerContract {
        self.answer_contract
    }

    pub const fn editor_input_profile(self) -> ThemeInputProfile {
        self.editor_input_profile
    }

    pub const fn layout(self) -> WorksheetLayoutProfile {
        self.layout
    }

    pub const fn with_editor_input_profile(
        mut self,
        editor_input_profile: ThemeInputProfile,
    ) -> Self {
        self.editor_input_profile = editor_input_profile;
        self
    }
}
