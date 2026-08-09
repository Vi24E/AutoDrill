use crate::effort::{OperationKind, OperationWeights, WeightProfile};
use crate::model::{
    CURRICULUM_PATH, CURRICULUM_PATH_LINEAR_EQUATION_1, CURRICULUM_PATH_LINEAR_EQUATION_2,
    DEFAULT_COLUMNS, DEFAULT_PROBLEM_COUNT, DEFAULT_ROWS, GENERATOR_REVISION_LINEAR_EQUATION_1,
    GENERATOR_REVISION_LINEAR_EQUATION_2, GENERATOR_REVISION_ONE_DIGIT_ADDITION,
    LINEAR_EQUATION_COLUMNS, LINEAR_EQUATION_PROBLEM_COUNT, LINEAR_EQUATION_ROWS, SKILL_ID,
    SKILL_ID_LINEAR_EQUATION_1, SKILL_ID_LINEAR_EQUATION_2, THEME_ID_LINEAR_EQUATION_1,
    THEME_ID_LINEAR_EQUATION_2, THEME_ID_ONE_DIGIT_ADDITION,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThemeRegistration {
    pub numeric_theme_id: u32,
    pub generator_revision: u32,
    pub skill_id: &'static str,
    pub curriculum_path: &'static [&'static str],
    pub problem_count: usize,
    pub columns: usize,
    pub rows: usize,
    /// Theme-specific multiplier overrides. Alpha 1.1 intentionally has none;
    /// this boundary allows future weighting without duplicating solution graphs.
    pub operation_weight_overrides: &'static [(OperationKind, f64)],
}

pub const ONE_DIGIT_ADDITION_REGISTRATION: ThemeRegistration = ThemeRegistration {
    numeric_theme_id: THEME_ID_ONE_DIGIT_ADDITION,
    generator_revision: GENERATOR_REVISION_ONE_DIGIT_ADDITION,
    skill_id: SKILL_ID,
    curriculum_path: &CURRICULUM_PATH,
    problem_count: DEFAULT_PROBLEM_COUNT,
    columns: DEFAULT_COLUMNS,
    rows: DEFAULT_ROWS,
    operation_weight_overrides: &[],
};

pub const LINEAR_EQUATION_1_REGISTRATION: ThemeRegistration = ThemeRegistration {
    numeric_theme_id: THEME_ID_LINEAR_EQUATION_1,
    generator_revision: GENERATOR_REVISION_LINEAR_EQUATION_1,
    skill_id: SKILL_ID_LINEAR_EQUATION_1,
    curriculum_path: &CURRICULUM_PATH_LINEAR_EQUATION_1,
    problem_count: LINEAR_EQUATION_PROBLEM_COUNT,
    columns: LINEAR_EQUATION_COLUMNS,
    rows: LINEAR_EQUATION_ROWS,
    operation_weight_overrides: &[],
};

pub const LINEAR_EQUATION_2_REGISTRATION: ThemeRegistration = ThemeRegistration {
    numeric_theme_id: THEME_ID_LINEAR_EQUATION_2,
    generator_revision: GENERATOR_REVISION_LINEAR_EQUATION_2,
    skill_id: SKILL_ID_LINEAR_EQUATION_2,
    curriculum_path: &CURRICULUM_PATH_LINEAR_EQUATION_2,
    problem_count: LINEAR_EQUATION_PROBLEM_COUNT,
    columns: LINEAR_EQUATION_COLUMNS,
    rows: LINEAR_EQUATION_ROWS,
    operation_weight_overrides: &[],
};

pub const GENERATOR_REGISTRY: [ThemeRegistration; 3] = [
    ONE_DIGIT_ADDITION_REGISTRATION,
    LINEAR_EQUATION_1_REGISTRATION,
    LINEAR_EQUATION_2_REGISTRATION,
];

pub fn active_registration(numeric_theme_id: u32) -> Option<&'static ThemeRegistration> {
    GENERATOR_REGISTRY
        .iter()
        .find(|registration| registration.numeric_theme_id == numeric_theme_id)
}

pub fn registration(
    numeric_theme_id: u32,
    generator_revision: u32,
) -> Option<&'static ThemeRegistration> {
    GENERATOR_REGISTRY.iter().find(|registration| {
        registration.numeric_theme_id == numeric_theme_id
            && registration.generator_revision == generator_revision
    })
}

pub fn resolved_weights(registration: &ThemeRegistration) -> OperationWeights {
    let mut profile = WeightProfile::default();
    for &(kind, multiplier) in registration.operation_weight_overrides {
        // Static registry definitions are authored in Rust and acceptance tests
        // assert validity, so an invalid value is an implementation defect.
        profile
            .theme
            .override_multiplier(kind, multiplier)
            .expect("registry weight multiplier must be finite and nonnegative");
    }
    profile.resolve(&OperationWeights::default())
}
