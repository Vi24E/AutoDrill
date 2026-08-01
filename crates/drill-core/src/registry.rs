use crate::effort::{OperationKind, OperationWeights, WeightProfile};
use crate::model::{
    CURRICULUM_PATH, DEFAULT_COLUMNS, DEFAULT_PROBLEM_COUNT, DEFAULT_ROWS,
    GENERATOR_REVISION_ONE_DIGIT_ADDITION, SKILL_ID, THEME_ID_ONE_DIGIT_ADDITION,
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

pub const GENERATOR_REGISTRY: [ThemeRegistration; 1] = [ONE_DIGIT_ADDITION_REGISTRATION];

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
