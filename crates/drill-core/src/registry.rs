use crate::effort::{OperationKind, OperationWeights, WeightProfile};
use crate::model::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThemeRegistration {
    pub numeric_theme_id: u32,
    pub generator_revision: u32,
    pub skill_id: &'static str,
    pub curriculum_path: &'static [&'static str],
    pub problem_count: usize,
    pub columns: usize,
    pub rows: usize,
    pub operation_weight_overrides: &'static [(OperationKind, f64)],
}

const fn standard_registration(
    numeric_theme_id: u32,
    generator_revision: u32,
    skill_id: &'static str,
    curriculum_path: &'static [&'static str],
) -> ThemeRegistration {
    ThemeRegistration {
        numeric_theme_id,
        generator_revision,
        skill_id,
        curriculum_path,
        problem_count: DEFAULT_PROBLEM_COUNT,
        columns: DEFAULT_COLUMNS,
        rows: DEFAULT_ROWS,
        operation_weight_overrides: &[],
    }
}

pub const ONE_DIGIT_ADDITION_REGISTRATION: ThemeRegistration = standard_registration(
    THEME_ID_ONE_DIGIT_ADDITION,
    GENERATOR_REVISION_ONE_DIGIT_ADDITION,
    SKILL_ID,
    &CURRICULUM_PATH,
);
pub const ONE_DIGIT_SUBTRACTION_REGISTRATION: ThemeRegistration = standard_registration(
    THEME_ID_ONE_DIGIT_SUBTRACTION,
    GENERATOR_REVISION_ONE_DIGIT_SUBTRACTION,
    SKILL_ID_ONE_DIGIT_SUBTRACTION,
    &CURRICULUM_PATH_ONE_DIGIT_SUBTRACTION,
);
pub const TWO_DIGIT_ADDITION_REGISTRATION: ThemeRegistration = standard_registration(
    THEME_ID_TWO_DIGIT_ADDITION,
    GENERATOR_REVISION_TWO_DIGIT_ADDITION,
    SKILL_ID_TWO_DIGIT_ADDITION,
    &CURRICULUM_PATH_TWO_DIGIT_ADDITION,
);
pub const MULTIPLICATION_TABLE_REGISTRATION: ThemeRegistration = standard_registration(
    THEME_ID_MULTIPLICATION_TABLE,
    GENERATOR_REVISION_MULTIPLICATION_TABLE,
    SKILL_ID_MULTIPLICATION_TABLE,
    &CURRICULUM_PATH_MULTIPLICATION_TABLE,
);
pub const SIGNED_ARITHMETIC_1_REGISTRATION: ThemeRegistration = standard_registration(
    THEME_ID_SIGNED_ARITHMETIC_1,
    GENERATOR_REVISION_SIGNED_ARITHMETIC_1,
    SKILL_ID_SIGNED_ARITHMETIC_1,
    &CURRICULUM_PATH_SIGNED_ARITHMETIC_1,
);
pub const SIGNED_ARITHMETIC_2_REGISTRATION: ThemeRegistration = standard_registration(
    THEME_ID_SIGNED_ARITHMETIC_2,
    GENERATOR_REVISION_SIGNED_ARITHMETIC_2,
    SKILL_ID_SIGNED_ARITHMETIC_2,
    &CURRICULUM_PATH_SIGNED_ARITHMETIC_2,
);

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
pub const FRACTION_ADDITION_REGISTRATION: ThemeRegistration = ThemeRegistration {
    numeric_theme_id: THEME_ID_FRACTION_ADDITION,
    generator_revision: GENERATOR_REVISION_FRACTION_ADDITION,
    skill_id: SKILL_ID_FRACTION_ADDITION,
    curriculum_path: &CURRICULUM_PATH_FRACTION_ADDITION,
    problem_count: LINEAR_EQUATION_PROBLEM_COUNT,
    columns: LINEAR_EQUATION_COLUMNS,
    rows: LINEAR_EQUATION_ROWS,
    operation_weight_overrides: &[],
};
pub const FRACTION_MULTIPLICATION_REGISTRATION: ThemeRegistration = ThemeRegistration {
    numeric_theme_id: THEME_ID_FRACTION_MULTIPLICATION,
    generator_revision: GENERATOR_REVISION_FRACTION_MULTIPLICATION,
    skill_id: SKILL_ID_FRACTION_MULTIPLICATION,
    curriculum_path: &CURRICULUM_PATH_FRACTION_MULTIPLICATION,
    problem_count: LINEAR_EQUATION_PROBLEM_COUNT,
    columns: LINEAR_EQUATION_COLUMNS,
    rows: LINEAR_EQUATION_ROWS,
    operation_weight_overrides: &[],
};
pub const FRACTION_SUBTRACTION_REGISTRATION: ThemeRegistration = ThemeRegistration {
    numeric_theme_id: THEME_ID_FRACTION_SUBTRACTION,
    generator_revision: GENERATOR_REVISION_FRACTION_SUBTRACTION,
    skill_id: SKILL_ID_FRACTION_SUBTRACTION,
    curriculum_path: &CURRICULUM_PATH_FRACTION_SUBTRACTION,
    problem_count: LINEAR_EQUATION_PROBLEM_COUNT,
    columns: LINEAR_EQUATION_COLUMNS,
    rows: LINEAR_EQUATION_ROWS,
    operation_weight_overrides: &[],
};
pub const FRACTION_DIVISION_REGISTRATION: ThemeRegistration = ThemeRegistration {
    numeric_theme_id: THEME_ID_FRACTION_DIVISION,
    generator_revision: GENERATOR_REVISION_FRACTION_DIVISION,
    skill_id: SKILL_ID_FRACTION_DIVISION,
    curriculum_path: &CURRICULUM_PATH_FRACTION_DIVISION,
    problem_count: LINEAR_EQUATION_PROBLEM_COUNT,
    columns: LINEAR_EQUATION_COLUMNS,
    rows: LINEAR_EQUATION_ROWS,
    operation_weight_overrides: &[],
};
pub const DIVISION_1_REGISTRATION: ThemeRegistration = standard_registration(
    THEME_ID_DIVISION_1,
    GENERATOR_REVISION_DIVISION_1,
    SKILL_ID_DIVISION_1,
    &CURRICULUM_PATH_DIVISION_1,
);
pub const QUADRATIC_EQUATION_1_REGISTRATION: ThemeRegistration = ThemeRegistration {
    numeric_theme_id: THEME_ID_QUADRATIC_EQUATION_1,
    generator_revision: GENERATOR_REVISION_QUADRATIC_EQUATION_1,
    skill_id: SKILL_ID_QUADRATIC_EQUATION_1,
    curriculum_path: &CURRICULUM_PATH_QUADRATIC_EQUATION_1,
    problem_count: LINEAR_EQUATION_PROBLEM_COUNT,
    columns: LINEAR_EQUATION_COLUMNS,
    rows: LINEAR_EQUATION_ROWS,
    operation_weight_overrides: &[],
};
pub const QUADRATIC_EQUATION_2_REGISTRATION: ThemeRegistration = ThemeRegistration {
    numeric_theme_id: THEME_ID_QUADRATIC_EQUATION_2,
    generator_revision: GENERATOR_REVISION_QUADRATIC_EQUATION_2,
    skill_id: SKILL_ID_QUADRATIC_EQUATION_2,
    curriculum_path: &CURRICULUM_PATH_QUADRATIC_EQUATION_2,
    problem_count: LINEAR_EQUATION_PROBLEM_COUNT,
    columns: LINEAR_EQUATION_COLUMNS,
    rows: LINEAR_EQUATION_ROWS,
    operation_weight_overrides: &[],
};
pub const QUADRATIC_EQUATION_3_REGISTRATION: ThemeRegistration = ThemeRegistration {
    numeric_theme_id: THEME_ID_QUADRATIC_EQUATION_3,
    generator_revision: GENERATOR_REVISION_QUADRATIC_EQUATION_3,
    skill_id: SKILL_ID_QUADRATIC_EQUATION_3,
    curriculum_path: &CURRICULUM_PATH_QUADRATIC_EQUATION_3,
    problem_count: LINEAR_EQUATION_PROBLEM_COUNT,
    columns: LINEAR_EQUATION_COLUMNS,
    rows: LINEAR_EQUATION_ROWS,
    operation_weight_overrides: &[],
};
pub const DECIMAL_ADD_SUBTRACT_REGISTRATION: ThemeRegistration = standard_registration(
    THEME_ID_DECIMAL_ADD_SUBTRACT,
    GENERATOR_REVISION_DECIMAL_ADD_SUBTRACT,
    SKILL_ID_DECIMAL_ADD_SUBTRACT,
    &CURRICULUM_PATH_DECIMAL_ADD_SUBTRACT,
);
pub const DECIMAL_MULTIPLY_DIVIDE_REGISTRATION: ThemeRegistration = standard_registration(
    THEME_ID_DECIMAL_MULTIPLY_DIVIDE,
    GENERATOR_REVISION_DECIMAL_MULTIPLY_DIVIDE,
    SKILL_ID_DECIMAL_MULTIPLY_DIVIDE,
    &CURRICULUM_PATH_DECIMAL_MULTIPLY_DIVIDE,
);

pub const SIMULTANEOUS_EQUATION_1_REGISTRATION: ThemeRegistration = ThemeRegistration {
    numeric_theme_id: THEME_ID_SIMULTANEOUS_EQUATION_1,
    generator_revision: GENERATOR_REVISION_SIMULTANEOUS_EQUATION_1,
    skill_id: SKILL_ID_SIMULTANEOUS_EQUATION_1,
    curriculum_path: &CURRICULUM_PATH_SIMULTANEOUS_EQUATION_1,
    problem_count: SIMULTANEOUS_EQUATION_PROBLEM_COUNT,
    columns: SIMULTANEOUS_EQUATION_COLUMNS,
    rows: SIMULTANEOUS_EQUATION_ROWS,
    operation_weight_overrides: &[],
};

pub const LIAR_PUZZLE_REGISTRATION: ThemeRegistration = ThemeRegistration {
    numeric_theme_id: THEME_ID_LIAR_PUZZLE,
    generator_revision: GENERATOR_REVISION_LIAR_PUZZLE,
    skill_id: SKILL_ID_LIAR_PUZZLE,
    curriculum_path: &CURRICULUM_PATH_LIAR_PUZZLE,
    problem_count: LIAR_PUZZLE_PROBLEM_COUNT,
    columns: LIAR_PUZZLE_COLUMNS,
    rows: LIAR_PUZZLE_ROWS,
    operation_weight_overrides: &[],
};

pub const GENERATOR_REGISTRY: [ThemeRegistration; 20] = [
    ONE_DIGIT_ADDITION_REGISTRATION,
    LINEAR_EQUATION_1_REGISTRATION,
    LINEAR_EQUATION_2_REGISTRATION,
    ONE_DIGIT_SUBTRACTION_REGISTRATION,
    TWO_DIGIT_ADDITION_REGISTRATION,
    MULTIPLICATION_TABLE_REGISTRATION,
    SIGNED_ARITHMETIC_1_REGISTRATION,
    SIGNED_ARITHMETIC_2_REGISTRATION,
    FRACTION_ADDITION_REGISTRATION,
    FRACTION_MULTIPLICATION_REGISTRATION,
    FRACTION_SUBTRACTION_REGISTRATION,
    FRACTION_DIVISION_REGISTRATION,
    DIVISION_1_REGISTRATION,
    QUADRATIC_EQUATION_1_REGISTRATION,
    QUADRATIC_EQUATION_2_REGISTRATION,
    QUADRATIC_EQUATION_3_REGISTRATION,
    DECIMAL_ADD_SUBTRACT_REGISTRATION,
    DECIMAL_MULTIPLY_DIVIDE_REGISTRATION,
    SIMULTANEOUS_EQUATION_1_REGISTRATION,
    LIAR_PUZZLE_REGISTRATION,
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
        profile
            .theme
            .override_multiplier(kind, multiplier)
            .expect("registry weight multiplier must be finite and nonnegative");
    }
    profile.resolve(&OperationWeights::default())
}
