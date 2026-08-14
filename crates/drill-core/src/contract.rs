use std::collections::BTreeMap;

use serde::Serialize;

use crate::effort::OPERATION_KIND_COUNT;
use crate::model::{GradeWarning, SCHEMA_VERSION};
use crate::registry::GENERATOR_REGISTRY;

#[derive(Debug, Serialize)]
pub struct WebContract<'a> {
    pub schema_version: u16,
    pub operation_kind_count: usize,
    pub themes: BTreeMap<u32, WebThemeContract<'a>>,
    pub grade_warning_codes: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct WebThemeContract<'a> {
    pub numeric_theme_id: u32,
    pub generator_revision: u32,
    pub skill_id: &'a str,
    pub curriculum_path: &'a [&'a str],
    pub layout: WebLayoutContract,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct WebLayoutContract {
    pub problem_count: usize,
    pub columns: usize,
    pub rows: usize,
}

/// Build the compatibility contract consumed by the Web application.
///
/// Values in this contract are intentionally limited to cross-language fields
/// that must never be duplicated by hand in TypeScript. Presentation-only Web
/// metadata such as route slugs and worksheet titles remains Web-owned.
pub fn web_contract() -> WebContract<'static> {
    let themes = GENERATOR_REGISTRY
        .iter()
        .map(|registration| {
            (
                registration.numeric_theme_id,
                WebThemeContract {
                    numeric_theme_id: registration.numeric_theme_id,
                    generator_revision: registration.generator_revision,
                    skill_id: registration.skill_id,
                    curriculum_path: registration.curriculum_path,
                    layout: WebLayoutContract {
                        problem_count: registration.problem_count,
                        columns: registration.columns,
                        rows: registration.rows,
                    },
                },
            )
        })
        .collect();
    let grade_warning_codes = GradeWarning::ALL
        .iter()
        .map(|warning| {
            serde_json::to_value(warning)
                .expect("grade warning must serialize")
                .as_str()
                .expect("grade warning wire value must be a string")
                .to_owned()
        })
        .collect();

    WebContract {
        schema_version: SCHEMA_VERSION,
        operation_kind_count: OPERATION_KIND_COUNT,
        themes,
        grade_warning_codes,
    }
}
