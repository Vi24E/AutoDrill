use serde::Serialize;
#[cfg(feature = "wire-types")]
use ts_rs::TS;

use crate::answer::AnswerNode;
use crate::effort::{Operation, OperationPlan, OperationVector};
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
#[cfg_attr(feature = "wire-types", ts(rename = "OperationPlan"))]
pub struct OperationPlanWire {
    pub operations: Vec<Operation>,
}

impl From<&OperationPlan> for OperationPlanWire {
    fn from(plan: &OperationPlan) -> Self {
        Self {
            operations: plan.operations().to_vec(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[cfg_attr(feature = "wire-types", derive(TS))]
#[cfg_attr(feature = "wire-types", ts(rename = "ColumnMultiplicationPartial"))]
pub struct ColumnMultiplicationPartialWire {
    #[cfg_attr(feature = "wire-types", ts(type = "number"))]
    pub value: i64,
    pub place: u32,
}

#[derive(Clone, Debug, Serialize)]
#[cfg_attr(feature = "wire-types", derive(TS))]
#[cfg_attr(feature = "wire-types", ts(rename = "LongDivisionStep"))]
pub struct LongDivisionStepWire {
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
pub enum WorkedSolutionWire {
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
/// `Problem` is the validated domain aggregate. This DTO owns the compatibility
/// shape consumed by WASM/Web, including effort fields that are derived from the
/// single internal `EffortModel` source of truth.
#[derive(Clone, Debug, Serialize)]
#[cfg_attr(feature = "wire-types", derive(TS))]
#[cfg_attr(feature = "wire-types", ts(rename = "Problem"))]
pub struct ProblemWire {
    pub schema_version: u16,
    pub id: u32,
    pub numeric_theme_id: u32,
    pub prompt: ProblemPrompt,
    pub input_interface: AnswerInputInterface,
    pub answer_schema: AnswerSchema,
    pub canonical_answer: AnswerNode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worked_solution: Option<WorkedSolutionWire>,
    pub operation_plan: OperationPlanWire,
    pub operation_vector: OperationVector,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme_specific_effort: Option<f64>,
    pub effort: f64,
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
            operation_plan: problem
                .operation_plan()
                .map(OperationPlanWire::from)
                .unwrap_or_else(|| OperationPlanWire {
                    operations: Vec::new(),
                }),
            operation_vector: problem.operation_vector(),
            theme_specific_effort: problem.theme_specific_effort(),
            effort: problem.effort(),
        }
    }
}

/// Serialized worksheet representation. The domain worksheet may evolve its
/// internal ownership without forcing Web DTO concerns back into generation.
#[derive(Clone, Debug, Serialize)]
#[cfg_attr(feature = "wire-types", derive(TS))]
#[cfg_attr(feature = "wire-types", ts(rename = "Worksheet"))]
pub struct WorksheetWire {
    pub schema_version: u16,
    pub problem_set_id: String,
    pub identity: ProblemSetIdentity,
    pub skill_id: String,
    pub curriculum_path: Vec<String>,
    pub layout: LayoutMetadata,
    pub problems: Vec<ProblemWire>,
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
