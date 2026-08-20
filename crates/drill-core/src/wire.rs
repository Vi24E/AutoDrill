use serde::Serialize;
#[cfg(feature = "wire-types")]
use ts_rs::TS;

use crate::answer::AnswerNode;
use crate::identity::ProblemSetIdentity;
use crate::model::{
    AnswerInputInterface, AnswerSchema, GradeResult, GradeStatus, GradeWarning, LayoutMetadata,
    Problem, ProblemPrompt, Worksheet,
};

#[derive(Clone, Debug, Serialize)]
#[cfg_attr(feature = "wire-types", derive(TS))]
#[cfg_attr(feature = "wire-types", ts(rename = "GradeResult"))]
pub struct GradeResultWire {
    pub status: GradeStatus,
    pub is_correct: bool,
    pub expected: AnswerNode,
    pub actual: AnswerNode,
    pub warnings: Vec<GradeWarning>,
}

impl From<&GradeResult> for GradeResultWire {
    fn from(result: &GradeResult) -> Self {
        Self {
            status: result.status(),
            is_correct: result.is_correct(),
            expected: result.expected().clone(),
            actual: result.actual().clone(),
            warnings: result.warnings().to_vec(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[cfg_attr(feature = "wire-types", derive(TS))]
#[cfg_attr(feature = "wire-types", ts(rename = "ColumnMultiplicationPartial"))]
pub(crate) struct ColumnMultiplicationPartialWire {
    #[cfg_attr(feature = "wire-types", ts(type = "number"))]
    pub value: i64,
    pub place: u32,
}

#[derive(Clone, Debug, Serialize)]
#[cfg_attr(feature = "wire-types", derive(TS))]
#[cfg_attr(feature = "wire-types", ts(rename = "LongDivisionStep"))]
pub(crate) struct LongDivisionStepWire {
    #[cfg_attr(feature = "wire-types", ts(type = "number"))]
    pub product: i64,
    #[cfg_attr(feature = "wire-types", ts(type = "number"))]
    pub after: i64,
    pub product_offset: u32,
    pub after_offset: u32,
}

#[derive(Clone, Debug, Serialize)]
#[cfg_attr(feature = "wire-types", derive(TS))]
#[cfg_attr(feature = "wire-types", ts(rename = "WorkedSolution"))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum WorkedSolutionWire {
    ColumnMultiplication {
        partial_products: Vec<ColumnMultiplicationPartialWire>,
    },
    LongDivision {
        #[cfg_attr(feature = "wire-types", ts(type = "number"))]
        divisor: i64,
        #[cfg_attr(feature = "wire-types", ts(type = "number"))]
        dividend_coefficient: i64,
        dividend_scale: u32,
        quotient_trailing_cells: u32,
        steps: Vec<LongDivisionStepWire>,
    },
}

/// Serialized representation of a generated problem.
///
/// `Problem` is the validated domain aggregate. This DTO owns the wire shape
/// consumed by WASM/Web. Internal effort diagnostics stay inside Rust until a
/// concrete cross-language consumer exists.
#[derive(Clone, Debug, Serialize)]
#[cfg_attr(feature = "wire-types", derive(TS))]
#[cfg_attr(feature = "wire-types", ts(rename = "Problem"))]
pub(crate) struct ProblemWire {
    pub schema_version: u16,
    pub id: u32,
    pub numeric_theme_id: u32,
    pub prompt: ProblemPrompt,
    pub input_interface: AnswerInputInterface,
    pub answer_schema: AnswerSchema,
    pub canonical_answer: AnswerNode,
    pub worked_solution: Option<WorkedSolutionWire>,
}

impl From<&Problem> for ProblemWire {
    fn from(problem: &Problem) -> Self {
        Self {
            schema_version: problem.schema_version(),
            id: problem.id(),
            numeric_theme_id: problem.numeric_theme_id(),
            prompt: problem.prompt().clone(),
            input_interface: problem.input_interface().clone(),
            answer_schema: problem.answer_schema().clone(),
            canonical_answer: problem.canonical_answer().clone(),
            worked_solution: problem.worked_solution().map(|solution| solution.to_wire()),
        }
    }
}

/// Serialized worksheet representation. The domain worksheet may evolve its
/// internal ownership without forcing Web DTO concerns back into generation.
#[derive(Clone, Debug, Serialize)]
#[cfg_attr(feature = "wire-types", derive(TS))]
#[cfg_attr(feature = "wire-types", ts(rename = "Worksheet"))]
pub struct WorksheetWire {
    schema_version: u16,
    problem_set_id: String,
    identity: ProblemSetIdentity,
    skill_id: String,
    curriculum_path: Vec<String>,
    layout: LayoutMetadata,
    problems: Vec<ProblemWire>,
}

impl From<&Worksheet> for WorksheetWire {
    fn from(worksheet: &Worksheet) -> Self {
        Self {
            schema_version: worksheet.schema_version(),
            problem_set_id: worksheet.problem_set_id(),
            identity: worksheet.identity().clone(),
            skill_id: worksheet.skill_id().to_owned(),
            curriculum_path: worksheet.curriculum_path().to_vec(),
            layout: worksheet.layout().clone(),
            problems: worksheet.problems().iter().map(ProblemWire::from).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::generate_problem_request;
    use crate::model::GenerateProblemRequest;
    use crate::themes::basic_arithmetic::{
        THEME_ID_MULTIPLICATION_TABLE, THEME_ID_ONE_DIGIT_ADDITION,
    };
    use crate::themes::column_arithmetic::THEME_ID_COLUMN_MULTIPLY_1DIGIT;

    fn problem_wire(theme_id: u32) -> serde_json::Value {
        let problem =
            generate_problem_request(&GenerateProblemRequest::new(theme_id, "Ab3Z")).unwrap();
        serde_json::to_value(ProblemWire::from(&problem)).unwrap()
    }

    #[test]
    fn problem_wire_matches_current_cross_language_surface() {
        let ordinary = problem_wire(THEME_ID_ONE_DIGIT_ADDITION);
        assert!(ordinary["worked_solution"].is_null());

        let theme_specific = problem_wire(THEME_ID_MULTIPLICATION_TABLE);
        assert!(theme_specific["worked_solution"].is_null());

        let worked = problem_wire(THEME_ID_COLUMN_MULTIPLY_1DIGIT);
        assert!(worked["worked_solution"].is_object());

        for wire in [&ordinary, &theme_specific, &worked] {
            for internal_effort_field in [
                "operation_plan",
                "operation_vector",
                "theme_specific_effort",
                "effort",
            ] {
                assert!(wire.get(internal_effort_field).is_none());
            }
        }
    }
}
