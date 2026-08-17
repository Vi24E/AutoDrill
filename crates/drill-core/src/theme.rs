use serde::Serialize;

use crate::effort::OperationKind;

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
    ColumnArithmetic,
    PrintRecommended,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct ThemePresentationPolicy {
    pub column_arithmetic: bool,
    pub print_recommended: bool,
    pub equation_layout: bool,
    pub fraction: FractionPresentationPolicy,
}

impl ThemePresentationPolicy {
    pub const STANDARD: Self = Self {
        column_arithmetic: false,
        print_recommended: false,
        equation_layout: false,
        fraction: FractionPresentationPolicy::None,
    };

    pub const EQUATION: Self = Self {
        equation_layout: true,
        ..Self::STANDARD
    };

    pub const COLUMN_ARITHMETIC: Self = Self {
        column_arithmetic: true,
        print_recommended: true,
        ..Self::STANDARD
    };

    pub const fn with_fraction(mut self, fraction: FractionPresentationPolicy) -> Self {
        self.fraction = fraction;
        self
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
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemeAnswerSchemaKind {
    Integer,
    Rational,
    Decimal,
    OrderedPair,
    Algebraic,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemeInputProfile {
    SimplePositive,
    SimpleSigned,
    SimpleDecimal,
    Fraction,
    ImproperFraction,
    SignedRational,
    LinearEquation,
    QuadraticEquation,
    SimultaneousEquation,
    JuniorHighFull,
    TupleOnly,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct ThemeAnswerContract {
    pub prompt_kind: ThemePromptKind,
    pub answer_schema_kind: ThemeAnswerSchemaKind,
    pub input_profile: ThemeInputProfile,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct WorksheetLayoutProfile {
    pub problem_count: usize,
    pub columns: usize,
    pub rows: usize,
}

pub const STANDARD_20_LAYOUT: WorksheetLayoutProfile = WorksheetLayoutProfile {
    problem_count: 20,
    columns: 2,
    rows: 10,
};
pub const COMPACT_16_LAYOUT: WorksheetLayoutProfile = WorksheetLayoutProfile {
    problem_count: 16,
    columns: 2,
    rows: 8,
};
pub const EQUATION_PAIR_12_LAYOUT: WorksheetLayoutProfile = WorksheetLayoutProfile {
    problem_count: 12,
    columns: 2,
    rows: 6,
};
pub const LIAR_6_LAYOUT: WorksheetLayoutProfile = WorksheetLayoutProfile {
    problem_count: 6,
    columns: 1,
    rows: 6,
};
pub const COLUMN_16_LAYOUT: WorksheetLayoutProfile = WorksheetLayoutProfile {
    problem_count: 16,
    columns: 4,
    rows: 4,
};
pub const COLUMN_DIVISION_12_LAYOUT: WorksheetLayoutProfile = WorksheetLayoutProfile {
    problem_count: 12,
    columns: 4,
    rows: 3,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SamplingLayerSpec {
    pub key: &'static str,
    pub weight: u32,
    pub minimum: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThemeRegistrationSpec {
    pub numeric_theme_id: u32,
    pub generator_revision: u32,
    pub skill_id: &'static str,
    pub curriculum_path: &'static [&'static str],
    pub grade: Option<u8>,
    pub tags: &'static [ThemeTag],
    pub safety: CurriculumSafetyPolicy,
    pub presentation: ThemePresentationPolicy,
    pub dedup: DedupPolicy,
    pub answer_contract: ThemeAnswerContract,
    pub layout: WorksheetLayoutProfile,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThemeRegistration {
    pub numeric_theme_id: u32,
    pub generator_revision: u32,
    pub skill_id: &'static str,
    pub curriculum_path: &'static [&'static str],
    /// Japanese school grade encoded independently of labels/routes: 1..=6 for
    /// elementary, 7..=9 for junior high. Bonus themes use None.
    pub grade: Option<u8>,
    pub tags: &'static [ThemeTag],
    pub safety: CurriculumSafetyPolicy,
    pub presentation: ThemePresentationPolicy,
    pub dedup: DedupPolicy,
    pub answer_contract: ThemeAnswerContract,
    /// Theme-level interactive editor grammar used by the Web input shell.
    pub editor_input_profile: ThemeInputProfile,
    pub layout: WorksheetLayoutProfile,
    pub operation_weight_overrides: &'static [(OperationKind, f64)],
}

impl ThemeRegistration {
    pub const fn new(spec: ThemeRegistrationSpec) -> Self {
        Self {
            numeric_theme_id: spec.numeric_theme_id,
            generator_revision: spec.generator_revision,
            skill_id: spec.skill_id,
            curriculum_path: spec.curriculum_path,
            grade: spec.grade,
            tags: spec.tags,
            safety: spec.safety,
            presentation: spec.presentation,
            dedup: spec.dedup,
            answer_contract: spec.answer_contract,
            editor_input_profile: spec.answer_contract.input_profile,
            layout: spec.layout,
            operation_weight_overrides: &[],
        }
    }

    pub const fn with_editor_input_profile(
        mut self,
        editor_input_profile: ThemeInputProfile,
    ) -> Self {
        self.editor_input_profile = editor_input_profile;
        self
    }
}
