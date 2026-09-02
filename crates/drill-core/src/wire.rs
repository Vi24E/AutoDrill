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
    pub column_input: Option<crate::model::ColumnArithmeticInput>,
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
            column_input: problem.column_input().copied(),
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
    identity: ProblemSetIdentity,
    problem_set_id: String,
    layout: LayoutMetadata,
    problems: Vec<ProblemWire>,
}

impl From<&Worksheet> for WorksheetWire {
    fn from(worksheet: &Worksheet) -> Self {
        Self {
            schema_version: worksheet.schema_version(),
            identity: worksheet.identity().clone(),
            problem_set_id: worksheet.problem_set_id(),
            layout: worksheet.layout().clone(),
            problems: worksheet.problems().iter().map(ProblemWire::from).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::generate_worksheet_request;
    use crate::model::GenerateWorksheetRequest;
    use crate::themes::basic_arithmetic::{
        THEME_ID_MULTIPLICATION_TABLE, THEME_ID_ONE_DIGIT_ADDITION,
    };
    use crate::themes::column_arithmetic::{
        THEME_ID_COLUMN_DECIMAL_MULTIPLICATION, THEME_ID_COLUMN_DIVIDE_2DIGIT_BY_1DIGIT,
        THEME_ID_COLUMN_MULTIPLY_1DIGIT,
    };
    use crate::Difficulty;

    fn problem_wire(theme_id: u32) -> serde_json::Value {
        let worksheet = generate_worksheet_request(&GenerateWorksheetRequest::new(
            theme_id,
            "Ab3Z",
            Difficulty::try_from(2).unwrap(),
        ))
        .unwrap();
        serde_json::to_value(ProblemWire::from(&worksheet.problems()[0])).unwrap()
    }

    #[test]
    fn worksheet_wire_contains_only_current_web_consumers() {
        let worksheet = generate_worksheet_request(&GenerateWorksheetRequest::new(
            THEME_ID_ONE_DIGIT_ADDITION,
            "Ab3Z",
            Difficulty::try_from(2).unwrap(),
        ))
        .unwrap();
        let wire = serde_json::to_value(WorksheetWire::from(&worksheet)).unwrap();

        for internal_field in ["skill_id", "curriculum_path"] {
            assert!(wire.get(internal_field).is_none());
        }
        for required_field in [
            "schema_version",
            "identity",
            "problem_set_id",
            "layout",
            "problems",
        ] {
            assert!(wire.get(required_field).is_some());
        }
        assert_eq!(wire["problem_set_id"], worksheet.problem_set_id());
    }

    #[test]
    fn column_input_policy_is_resolved_into_problem_wire_metadata() {
        let ordinary = problem_wire(THEME_ID_COLUMN_MULTIPLY_1DIGIT);
        assert_eq!(
            ordinary["column_input"]["single"]["order"],
            "least_significant_first"
        );
        assert_eq!(
            ordinary["column_input"]["single"]["decimal_point"]["type"],
            "none"
        );

        let division = problem_wire(THEME_ID_COLUMN_DIVIDE_2DIGIT_BY_1DIGIT);
        assert_eq!(
            division["column_input"]["quotient"]["order"],
            "natural_division_flow"
        );
        assert_eq!(division["column_input"]["remainder"]["order"], "big_endian");

        let decimal_multiplication = problem_wire(THEME_ID_COLUMN_DECIMAL_MULTIPLICATION);
        assert_eq!(
            decimal_multiplication["column_input"]["single"]["decimal_point"]["type"],
            "editable"
        );
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
