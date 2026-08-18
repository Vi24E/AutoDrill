use std::collections::BTreeMap;

use serde::Serialize;

use crate::effort::OPERATION_KIND_COUNT;
use crate::input::input_interface;
use crate::model::{AnswerInputInterface, EditorStructure, GradeWarning, MAX_ANSWER_AST_SIZE};
use crate::registry::{active_registrations, RegistryError};
use crate::schema::SCHEMA_VERSION;
use crate::theme::{
    CurriculumSafetyPolicy, DedupPolicy, ThemeAnswerContract, ThemePresentationPolicy, ThemeTag,
};

#[derive(Debug, Serialize)]
pub struct WebContract<'a> {
    pub schema_version: u16,
    pub operation_kind_count: usize,
    pub max_answer_ast_size: usize,
    pub themes: BTreeMap<u32, WebThemeContract<'a>>,
    pub grade_warning_codes: Vec<String>,
    pub editor_structures: Vec<EditorStructure>,
}

#[derive(Debug, Serialize)]
pub struct WebThemeContract<'a> {
    pub numeric_theme_id: u32,
    pub generator_revision: u32,
    pub skill_id: &'a str,
    pub curriculum_path: &'a [&'a str],
    pub grade: Option<u8>,
    pub tags: &'a [ThemeTag],
    pub safety: CurriculumSafetyPolicy,
    pub presentation: ThemePresentationPolicy,
    pub dedup: DedupPolicy,
    pub answer_contract: ThemeAnswerContract,
    pub input_interface: AnswerInputInterface,
    pub editor_input_interface: AnswerInputInterface,
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
pub fn web_contract() -> Result<WebContract<'static>, RegistryError> {
    let themes = active_registrations()?
        .into_iter()
        .map(|registration| {
            (
                registration.numeric_theme_id(),
                WebThemeContract {
                    numeric_theme_id: registration.numeric_theme_id(),
                    generator_revision: registration.generator_revision(),
                    skill_id: registration.skill_id(),
                    curriculum_path: registration.curriculum_path(),
                    grade: registration.grade().map(crate::theme::SchoolGrade::value),
                    tags: registration.tags(),
                    safety: registration.safety(),
                    presentation: registration.presentation(),
                    dedup: registration.dedup(),
                    answer_contract: registration.answer_contract(),
                    input_interface: input_interface(
                        registration.answer_contract().input_profile(),
                    ),
                    editor_input_interface: input_interface(registration.editor_input_profile()),
                    layout: WebLayoutContract {
                        problem_count: registration.layout().problem_count(),
                        columns: registration.layout().columns(),
                        rows: registration.layout().rows(),
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

    Ok(WebContract {
        schema_version: SCHEMA_VERSION,
        operation_kind_count: OPERATION_KIND_COUNT,
        max_answer_ast_size: MAX_ANSWER_AST_SIZE,
        themes,
        grade_warning_codes,
        editor_structures: EditorStructure::ALL.to_vec(),
    })
}
