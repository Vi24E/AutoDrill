#![forbid(unsafe_code)]

//! Thin JSON/WASM boundary for the current domain DTOs. Generation,
//! normalization, grading, effort, identity, and retry policy stay in
//! `drill-core`; this crate only parses requests and formats stable errors.

#[cfg(any(target_arch = "wasm32", test))]
use std::cell::Cell;
#[cfg(any(target_arch = "wasm32", test))]
use std::time::Duration;

#[cfg(test)]
use drill_core::MAX_ANSWER_AST_SIZE;
#[cfg(feature = "qa-diagnostics")]
use drill_core::QA_OPERATION_VECTOR_BASIS;
#[cfg(not(target_arch = "wasm32"))]
use drill_core::{
    generate_problem_set_from_id as core_generate_problem_set_from_id,
    generate_worksheet_request as core_generate_worksheet_request,
};
#[cfg(target_arch = "wasm32")]
use drill_core::{
    generate_problem_set_from_id_with_clock, generate_worksheet_request_with_clock,
    GenerationConfig, MonotonicClock,
};
use drill_core::{
    grade_answer_with_schema as core_grade_answer_with_schema,
    parse_mathlive_answer as core_parse_mathlive_answer, AnswerInputInterface, AnswerNode,
    AnswerSchema, EditorError, GenerateWorksheetRequest, GenerationError, GradeError,
    SCHEMA_VERSION,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Value};
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(catch, js_namespace = performance, js_name = now)]
    fn performance_now() -> Result<f64, JsValue>;
}

#[cfg(target_arch = "wasm32")]
#[derive(Debug)]
struct BrowserClock {
    state: BrowserClockState,
}

#[cfg(any(target_arch = "wasm32", test))]
#[derive(Debug)]
struct BrowserClockState {
    origin_ms: f64,
    last_elapsed_ms: Cell<f64>,
    reads: Cell<u64>,
    failed: Cell<bool>,
}

#[cfg(target_arch = "wasm32")]
impl BrowserClock {
    fn try_new() -> Option<Self> {
        let origin_ms = performance_now().ok()?;
        Some(Self {
            state: BrowserClockState::try_new(origin_ms)?,
        })
    }
}

#[cfg(any(target_arch = "wasm32", test))]
impl BrowserClockState {
    fn try_new(origin_ms: f64) -> Option<Self> {
        if !origin_ms.is_finite() || origin_ms < 0.0 {
            return None;
        }
        Some(Self {
            origin_ms,
            last_elapsed_ms: Cell::new(0.0),
            reads: Cell::new(0),
            failed: Cell::new(false),
        })
    }

    fn read(&self, sample_ms: Option<f64>) -> Duration {
        let read_index = self.reads.get();
        self.reads.set(read_index.saturating_add(1));
        if self.failed.get() {
            return self.failure_duration(read_index);
        }
        let previous = self.last_elapsed_ms.get();
        let now_ms = match sample_ms {
            Some(value) if value.is_finite() && value >= self.origin_ms => value,
            _ => return self.failure_duration(read_index),
        };
        let elapsed_ms = now_ms - self.origin_ms;
        if elapsed_ms < previous {
            return self.failure_duration(read_index);
        }
        self.last_elapsed_ms.set(elapsed_ms);
        let nanos = (elapsed_ms * 1_000_000.0).min(u64::MAX as f64) as u64;
        Duration::from_nanos(nanos)
    }

    fn failure_duration(&self, read_index: u64) -> Duration {
        self.failed.set(true);
        if read_index == 0 {
            Duration::ZERO
        } else {
            Duration::MAX
        }
    }
}

#[cfg(target_arch = "wasm32")]
impl MonotonicClock for BrowserClock {
    fn now(&self) -> Duration {
        self.state.read(performance_now().ok())
    }
}

#[derive(Debug, Deserialize)]
struct GenerateProblemSetRequest {
    problem_set_id: String,
}

#[derive(Debug, Deserialize)]
struct ParseMathLiveAnswerRequest {
    schema_version: u16,
    input_interface: AnswerInputInterface,
    latex: String,
}

#[derive(Debug, Deserialize)]
struct GradeAnswerRequest {
    schema_version: u16,
    expected: AnswerNode,
    actual: AnswerNode,
    #[serde(default)]
    answer_schema: Option<AnswerSchema>,
    input_interface: AnswerInputInterface,
}

#[cfg(feature = "qa-diagnostics")]
#[derive(Debug, Serialize)]
struct QaProblemEffortDiagnostics {
    problem_index: usize,
    effort: f64,
    effort_model: &'static str,
    operation_vector: Option<Vec<f64>>,
}

#[cfg(feature = "qa-diagnostics")]
#[derive(Debug, Serialize)]
struct QaWorksheetWithEffortDiagnostics {
    worksheet: drill_core::Worksheet,
    operation_vector_basis: Vec<&'static str>,
    problems: Vec<QaProblemEffortDiagnostics>,
}

#[derive(Debug, Serialize)]
struct ApiResponse<T: Serialize> {
    schema_version: u16,
    ok: bool,
    data: Option<T>,
    error: Option<ApiError>,
}

#[derive(Debug, Serialize)]
struct ApiError {
    code: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<Value>,
}

#[wasm_bindgen]
pub fn generate_worksheet(input_json: &str) -> String {
    respond_with(input_json, |request: GenerateWorksheetRequest| {
        generate_worksheet_for_platform(&request).map_err(generation_error)
    })
}

#[wasm_bindgen]
pub fn generate_problem_set(input_json: &str) -> String {
    respond_with(input_json, |request: GenerateProblemSetRequest| {
        generate_problem_set_for_platform(&request.problem_set_id).map_err(generation_error)
    })
}

#[cfg(feature = "qa-diagnostics")]
/// Local QA-only generation endpoint. The production Worksheet wire remains
/// unchanged; effort diagnostics are exposed only to the explicit QA consumer.
#[wasm_bindgen]
pub fn generate_qa_worksheet_with_effort(input_json: &str) -> String {
    respond_with(input_json, |request: GenerateWorksheetRequest| {
        let worksheet = generate_worksheet_for_platform(&request).map_err(generation_error)?;
        let problems = worksheet
            .problems()
            .iter()
            .enumerate()
            .map(|(problem_index, problem)| QaProblemEffortDiagnostics {
                problem_index,
                effort: problem.effort(),
                effort_model: problem.qa_effort_model_kind(),
                operation_vector: problem
                    .qa_effort_operation_vector()
                    .map(|vector| vector.to_vec()),
            })
            .collect();
        Ok(QaWorksheetWithEffortDiagnostics {
            worksheet,
            operation_vector_basis: QA_OPERATION_VECTOR_BASIS.to_vec(),
            problems,
        })
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn generate_worksheet_for_platform(
    request: &GenerateWorksheetRequest,
) -> Result<drill_core::Worksheet, GenerationError> {
    core_generate_worksheet_request(request)
}

#[cfg(target_arch = "wasm32")]
fn generate_worksheet_for_platform(
    request: &GenerateWorksheetRequest,
) -> Result<drill_core::Worksheet, GenerationError> {
    let config = GenerationConfig::from_request(request);
    let clock = BrowserClock::try_new().ok_or_else(|| GenerationError::timeout(config.timeout))?;
    generate_worksheet_request_with_clock(request, &clock)
}

#[cfg(not(target_arch = "wasm32"))]
fn generate_problem_set_for_platform(
    problem_set_id: &str,
) -> Result<drill_core::Worksheet, GenerationError> {
    core_generate_problem_set_from_id(problem_set_id)
}

#[cfg(target_arch = "wasm32")]
fn generate_problem_set_for_platform(
    problem_set_id: &str,
) -> Result<drill_core::Worksheet, GenerationError> {
    let config = GenerationConfig::default();
    let clock = BrowserClock::try_new().ok_or_else(|| GenerationError::timeout(config.timeout))?;
    generate_problem_set_from_id_with_clock(problem_set_id, &clock)
}

#[wasm_bindgen]
pub fn parse_mathlive_answer(input_json: &str) -> String {
    respond_with(input_json, |request: ParseMathLiveAnswerRequest| {
        validate_schema(request.schema_version)?;
        core_parse_mathlive_answer(&request.latex, &request.input_interface).map_err(editor_error)
    })
}

#[wasm_bindgen]
pub fn grade_answer(input_json: &str) -> String {
    respond_with(input_json, |request: GradeAnswerRequest| {
        validate_schema(request.schema_version)?;
        request
            .input_interface
            .validate_answer(&request.expected)
            .map_err(editor_error)?;
        request
            .input_interface
            .validate_answer(&request.actual)
            .map_err(editor_error)?;
        core_grade_answer_with_schema(
            &request.expected,
            &request.actual,
            request.answer_schema.as_ref(),
        )
        .map_err(grade_error)
    })
}

fn respond_with<Request, Response, F>(input_json: &str, handler: F) -> String
where
    Request: DeserializeOwned,
    Response: Serialize,
    F: FnOnce(Request) -> Result<Response, ApiError>,
{
    let request = match serde_json::from_str(input_json) {
        Ok(request) => request,
        Err(error) => return error_response(invalid_request(&error.to_string())),
    };
    match handler(request) {
        Ok(data) => success_response(data),
        Err(error) => error_response(error),
    }
}

fn validate_schema(schema_version: u16) -> Result<(), ApiError> {
    if schema_version == SCHEMA_VERSION {
        Ok(())
    } else {
        Err(ApiError {
            code: "unsupported_schema_version".to_owned(),
            message: format!(
                "schema_version {schema_version} is unsupported; expected {SCHEMA_VERSION}"
            ),
            details: Some(json!({ "expected": SCHEMA_VERSION, "received": schema_version })),
        })
    }
}

fn generation_error(error: GenerationError) -> ApiError {
    let details = match &error {
        GenerationError::Timeout { timeout_ms } => Some(json!({ "timeout_ms": timeout_ms })),
        GenerationError::AttemptLimit {
            attempts,
            max_attempts,
        } => Some(json!({ "attempts": attempts, "max_attempts": max_attempts })),
        GenerationError::UnsupportedSchemaVersion { received, expected } => {
            Some(json!({ "received": received, "expected": expected }))
        }
        GenerationError::UnknownTheme { numeric_theme_id } => {
            Some(json!({ "numeric_theme_id": numeric_theme_id }))
        }
        GenerationError::UnknownGeneratorRevision {
            numeric_theme_id,
            generator_revision,
        } => Some(json!({
            "numeric_theme_id": numeric_theme_id,
            "generator_revision": generator_revision
        })),
        GenerationError::InvalidIdentity(drill_core::IdentityError::UnsupportedSchemaVersion {
            received,
            expected,
        }) => Some(json!({ "received": received, "expected": expected })),
        GenerationError::InvalidGeneratedProblem { reason }
        | GenerationError::InvalidGeneratedWorksheet { reason } => {
            Some(json!({ "reason": reason }))
        }
        GenerationError::InvalidIdentity(_)
        | GenerationError::InvalidRegistry(_)
        | GenerationError::InvalidSampling(_) => None,
    };
    ApiError {
        code: error.code().to_owned(),
        message: error.to_string(),
        details,
    }
}

fn grade_error(error: GradeError) -> ApiError {
    ApiError {
        code: error.code().to_owned(),
        message: error.to_string(),
        details: None,
    }
}

fn editor_error(error: EditorError) -> ApiError {
    let (code, details) = match &error {
        EditorError::AnswerSizeLimit { max_size } => (
            "answer_ast_size_limit",
            Some(json!({ "max_size": max_size })),
        ),
        EditorError::StructureNotAllowed { structure } => (
            "input_structure_not_allowed",
            Some(json!({ "structure": structure })),
        ),
        EditorError::InputInterfaceViolation => ("input_interface_violation", None),
    };
    ApiError {
        code: code.to_owned(),
        message: error.to_string(),
        details,
    }
}

fn invalid_request(message: &str) -> ApiError {
    ApiError {
        code: "invalid_request".to_owned(),
        message: message.to_owned(),
        details: None,
    }
}

fn success_response<T: Serialize>(data: T) -> String {
    serde_json::to_string(&ApiResponse {
        schema_version: SCHEMA_VERSION,
        ok: true,
        data: Some(data),
        error: None,
    })
    .unwrap_or_else(|_| serialization_failure())
}

fn error_response(error: ApiError) -> String {
    serde_json::to_string(&ApiResponse::<Value> {
        schema_version: SCHEMA_VERSION,
        ok: false,
        data: None,
        error: Some(error),
    })
    .unwrap_or_else(|_| serialization_failure())
}

fn serialization_failure() -> String {
    format!(
        "{{\"schema_version\":{SCHEMA_VERSION},\"ok\":false,\"data\":null,\"error\":{{\"code\":\"serialization_error\",\"message\":\"serialization failed\"}}}}"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(output: &str) -> Value {
        serde_json::from_str(output).unwrap()
    }

    fn simple_input_interface() -> Value {
        json!({
            "type": "simple_numeric",
            "allow_decimal": false,
            "allow_negative": false
        })
    }

    fn nested_negative(depth: usize) -> Value {
        let mut value = json!({"type": "integer", "value": "1"});
        for _ in 0..depth {
            value = json!({"type": "negative", "value": value});
        }
        value
    }

    #[test]
    fn worksheet_boundary_matches_current_schema_and_identity() {
        let output = generate_worksheet(
            &json!({
                "schema_version": SCHEMA_VERSION,
                "numeric_theme_id": 1,
                "seed": "Ab3Z",
                "difficulty": 3
            })
            .to_string(),
        );
        let value = parse(&output);
        assert_eq!(value["schema_version"], SCHEMA_VERSION);
        assert_eq!(value["ok"], true);
        assert!(value["data"]["identity"]["generator_revision"]
            .as_u64()
            .is_some_and(|revision| revision > 0));
        assert_eq!(value["data"]["identity"]["numeric_theme_id"], 1);
        assert_eq!(value["data"]["identity"]["seed"], "Ab3Z");
        assert_eq!(value["data"]["identity"]["difficulty"], 3);
        let identity: drill_core::ProblemSetIdentity =
            serde_json::from_value(value["data"]["identity"].clone()).unwrap();
        assert_eq!(value["data"]["problem_set_id"], identity.to_string());
        for removed_field in ["skill_id", "curriculum_path"] {
            assert!(value["data"].get(removed_field).is_none());
        }
        assert_eq!(value["data"]["layout"]["problem_count"], 20);
        assert_eq!(value["data"]["problems"][0]["prompt"]["kind"], "addition");
        assert_eq!(value["data"]["problems"][0]["answer_schema"]["min"], "1");
        assert_eq!(value["data"]["problems"][0]["answer_schema"]["max"], "18");
        assert_eq!(
            value["data"]["problems"][0]["input_interface"],
            simple_input_interface()
        );
        assert!(value["data"]["problems"][0]["canonical_answer"]["value"].is_string());
        let problem = &value["data"]["problems"][0];
        assert!(problem["worked_solution"].is_null());
        for internal_effort_field in [
            "operation_plan",
            "operation_vector",
            "theme_specific_effort",
            "effort",
        ] {
            assert!(problem.get(internal_effort_field).is_none());
        }
    }

    #[test]
    fn problem_set_id_replay_round_trips_and_fails_closed() {
        let generated = parse(&generate_worksheet(
            &json!({
                "schema_version": SCHEMA_VERSION,
                "numeric_theme_id": 1,
                "seed": "Ab3Z",
                "difficulty": 3
            })
            .to_string(),
        ));
        let problem_set_id = generated["data"]["problem_set_id"].as_str().unwrap();
        let replayed = parse(&generate_problem_set(
            &json!({ "problem_set_id": problem_set_id }).to_string(),
        ));
        assert_eq!(replayed["ok"], true);
        assert_eq!(replayed["data"], generated["data"]);

        let malformed = parse(&generate_problem_set(
            &json!({ "problem_set_id": "not-an-identity" }).to_string(),
        ));
        assert_eq!(malformed["error"]["code"], "invalid_problem_set_identity");

        let unsupported_schema = parse(&generate_problem_set(
            &json!({ "problem_set_id": "6-1-1-Ab3Z-3" }).to_string(),
        ));
        assert_eq!(
            unsupported_schema["error"]["code"],
            "unsupported_schema_version"
        );

        let mut parts = problem_set_id.split('-').collect::<Vec<_>>();
        parts[2] = "4294967295";
        let unknown_revision = parse(&generate_problem_set(
            &json!({ "problem_set_id": parts.join("-") }).to_string(),
        ));
        assert_eq!(
            unknown_revision["error"]["code"],
            "unknown_generator_revision"
        );
    }

    #[cfg(feature = "qa-diagnostics")]
    #[test]
    fn qa_effort_endpoint_keeps_diagnostics_outside_the_worksheet_wire() {
        let output = generate_qa_worksheet_with_effort(
            &json!({
                "schema_version": SCHEMA_VERSION,
                "numeric_theme_id": 2,
                "seed": "QaV1",
                "difficulty": 4
            })
            .to_string(),
        );
        let value = parse(&output);
        assert_eq!(value["ok"], true);
        assert_eq!(
            value["data"]["operation_vector_basis"]
                .as_array()
                .unwrap()
                .len(),
            QA_OPERATION_VECTOR_BASIS.len()
        );
        let diagnostics = value["data"]["problems"].as_array().unwrap();
        let worksheet_problems = value["data"]["worksheet"]["problems"].as_array().unwrap();
        assert_eq!(diagnostics.len(), worksheet_problems.len());
        assert!(diagnostics
            .iter()
            .all(|problem| problem["effort"].as_f64().is_some()));
        assert!(diagnostics
            .iter()
            .any(|problem| problem["operation_vector"].is_array()));
        for problem in worksheet_problems {
            assert!(problem.get("effort").is_none());
            assert!(problem.get("operation_vector").is_none());
        }
    }

    #[test]
    fn non_current_schema_requests_fail_closed_at_wasm_boundary() {
        let request = parse(&generate_worksheet(
            r#"{"schema_version":2,"numeric_theme_id":1,"seed":"Ab3Z","difficulty":3}"#,
        ));
        assert_eq!(request["error"]["code"], "unsupported_schema_version");
        let missing_schema = parse(&generate_worksheet(
            r#"{"numeric_theme_id":1,"seed":"Ab3Z","difficulty":3}"#,
        ));
        assert_eq!(missing_schema["error"]["code"], "invalid_request");
    }

    #[test]
    fn timeout_and_attempt_errors_remain_distinct() {
        let timeout = parse(&generate_worksheet(
            &json!({
                "schema_version": SCHEMA_VERSION,
                "numeric_theme_id": 1,
                "seed": "Ab3Z",
                "difficulty": 3,
                "timeout_ms": 0
            })
            .to_string(),
        ));
        assert_eq!(timeout["error"]["code"], "generation_timeout");

        let attempts = parse(&generate_worksheet(
            &json!({
                "schema_version": SCHEMA_VERSION,
                "numeric_theme_id": 1,
                "seed": "Ab3Z",
                "difficulty": 3,
                "timeout_ms": 1000,
                "max_attempts": 0
            })
            .to_string(),
        ));
        assert_eq!(attempts["error"]["code"], "generation_attempt_limit");
    }
    #[test]
    fn partially_empty_fraction_mixed_and_root_answers_stay_accepted() {
        for answer in [
            json!({
                "type": "fraction",
                "value": {
                    "numerator": {"type": "empty"},
                    "denominator": {"type": "integer", "value": "2"}
                }
            }),
            json!({
                "type": "mixed_fraction",
                "value": {
                    "whole": {"type": "integer", "value": "1"},
                    "numerator": {"type": "empty"},
                    "denominator": {"type": "empty"}
                }
            }),
            json!({
                "type": "root",
                "value": {
                    "radicand": {"type": "empty"},
                    "index": {"type": "empty"}
                }
            }),
        ] {
            let graded = parse(&grade_answer(
                &json!({
                    "schema_version": SCHEMA_VERSION,
                    "input_interface": {
                        "type": "structured_math",
                        "allowed_structures": ["fraction", "mixed_fraction", "root"]
                    },
                    "expected": answer.clone(),
                    "actual": answer
                })
                .to_string(),
            ));
            assert_eq!(graded["ok"], true);
            assert_eq!(graded["data"]["is_correct"], true);
        }
    }

    #[test]
    fn parse_mathlive_delegates_interface_validation_to_core() {
        let rejected = parse(&parse_mathlive_answer(
            &json!({
                "schema_version": SCHEMA_VERSION,
                "latex": "1",
                "input_interface": {
                    "type": "structured_math",
                    "allowed_structures": []
                }
            })
            .to_string(),
        ));
        assert_eq!(rejected["ok"], false);
        assert_eq!(rejected["error"]["code"], "input_interface_violation");
    }

    #[test]
    fn grade_answer_preserves_boundary_then_core_validation_precedence() {
        let invalid_interface = parse(&grade_answer(
            &json!({
                "schema_version": SCHEMA_VERSION,
                "input_interface": {
                    "type": "structured_math",
                    "allowed_structures": []
                },
                "expected": {"type": "integer", "value": "1"},
                "actual": {"type": "integer", "value": "1"}
            })
            .to_string(),
        ));
        assert_eq!(
            invalid_interface["error"]["code"],
            "input_interface_violation"
        );

        // AnswerNode deserialization already bounds external JSON before the
        // grade handler runs, so a second explicit size traversal is redundant.
        let oversized = nested_negative(MAX_ANSWER_AST_SIZE + 1);
        let oversized_answer = parse(&grade_answer(
            &json!({
                "schema_version": SCHEMA_VERSION,
                "input_interface": {
                    "type": "structured_math",
                    "allowed_structures": ["negative"]
                },
                "expected": oversized.clone(),
                "actual": oversized
            })
            .to_string(),
        ));
        assert_eq!(oversized_answer["error"]["code"], "invalid_request");
    }

    #[test]
    fn grade_answer_delegates_input_capability_validation_to_core() {
        let rejected = parse(&grade_answer(
            &json!({
                "schema_version": SCHEMA_VERSION,
                "input_interface": simple_input_interface(),
                "expected": {"type": "integer", "value": "1"},
                "actual": {
                    "type": "fraction",
                    "value": {
                        "numerator": {"type": "integer", "value": "1"},
                        "denominator": {"type": "integer", "value": "2"}
                    }
                }
            })
            .to_string(),
        ));
        assert_eq!(rejected["ok"], false);
        assert_eq!(rejected["error"]["code"], "input_structure_not_allowed");
    }

    #[test]
    fn grade_error_codes_survive_the_wasm_boundary() {
        let invalid_schema = parse(&grade_answer(
            &json!({
                "schema_version": SCHEMA_VERSION,
                "input_interface": simple_input_interface(),
                "expected": {"type": "integer", "value": "1"},
                "actual": {"type": "integer", "value": "1"},
                "answer_schema": {
                    "kind": "rational",
                    "max_abs_numerator": 1,
                    "max_denominator": 0,
                    "require_reduced_fraction_form": true
                }
            })
            .to_string(),
        ));
        assert_eq!(invalid_schema["ok"], false);
        assert_eq!(invalid_schema["error"]["code"], "invalid_answer_schema");

        let expected_outside = parse(&grade_answer(
            &json!({
                "schema_version": SCHEMA_VERSION,
                "input_interface": simple_input_interface(),
                "expected": {"type": "integer", "value": "1"},
                "actual": {"type": "integer", "value": "1"},
                "answer_schema": {"kind": "integer", "min": "0", "max": "0"}
            })
            .to_string(),
        ));
        assert_eq!(expected_outside["ok"], false);
        assert_eq!(
            expected_outside["error"]["code"],
            "expected_answer_outside_schema"
        );
    }

    #[test]
    fn answer_i64_payloads_cross_json_as_exact_decimal_strings() {
        let exact = "999999999999999999";
        let graded = parse(&grade_answer(
            &json!({
                "schema_version": SCHEMA_VERSION,
                "input_interface": simple_input_interface(),
                "expected": {"type": "integer", "value": exact},
                "actual": {"type": "integer", "value": exact}
            })
            .to_string(),
        ));
        assert_eq!(graded["data"]["is_correct"], true);
        assert_eq!(graded["data"]["actual"]["value"], exact);
    }

    #[test]
    fn mathlive_public_boundary_round_trip_covers_structured_surface() {
        let interface = json!({
            "type": "structured_math",
            "allowed_structures": [
                "fraction",
                "mixed_fraction",
                "decimal",
                "root",
                "negative",
                "plus_minus",
                "tuple",
                "arithmetic"
            ]
        });
        for latex in [
            "12",
            "-12",
            "1.25",
            r"\frac{3}{4}",
            r"1\frac{1}{2}",
            r"−1\frac{1}{2}",
            r"\sqrt{16}",
            r"\pm2",
            "2,-2",
            "2+3*4",
        ] {
            let parsed = parse(&parse_mathlive_answer(
                &json!({
                    "schema_version": SCHEMA_VERSION,
                    "input_interface": interface.clone(),
                    "latex": latex
                })
                .to_string(),
            ));
            assert_eq!(parsed["ok"], true, "parse failed for {latex}: {parsed}");
            let actual = parsed["data"].clone();

            let graded = parse(&grade_answer(
                &json!({
                    "schema_version": SCHEMA_VERSION,
                    "input_interface": interface.clone(),
                    "expected": actual.clone(),
                    "actual": actual
                })
                .to_string(),
            ));
            assert_eq!(graded["ok"], true, "grade failed for {latex}: {graded}");
            assert_eq!(
                graded["data"]["is_correct"], true,
                "round-trip mismatch for {latex}: {graded}"
            );
        }

        let malformed = r"\frac{1}";
        let parsed = parse(&parse_mathlive_answer(
            &json!({
                "schema_version": SCHEMA_VERSION,
                "input_interface": interface.clone(),
                "latex": malformed
            })
            .to_string(),
        ));
        assert_eq!(parsed["ok"], true);
        assert_eq!(parsed["data"]["type"], "nan_error");
        assert_eq!(parsed["data"]["value"], malformed);

        let too_deep = format!(
            "{}1{}",
            r"\sqrt{".repeat(MAX_ANSWER_AST_SIZE + 20),
            "}".repeat(MAX_ANSWER_AST_SIZE + 20)
        );
        let too_large = "1".repeat(5_000);
        for latex in [too_deep, too_large] {
            let rejected = parse(&parse_mathlive_answer(
                &json!({
                    "schema_version": SCHEMA_VERSION,
                    "input_interface": interface.clone(),
                    "latex": latex
                })
                .to_string(),
            ));
            assert_eq!(rejected["ok"], false);
            assert_eq!(rejected["error"]["code"], "answer_ast_size_limit");
        }
    }

    #[test]
    fn browser_clock_failures_latch_to_timeout() {
        let state = BrowserClockState::try_new(100.0).unwrap();
        let started = state.read(Some(101.0));
        let failed = state.read(None);
        assert_eq!(failed, Duration::MAX);
        assert!(failed.saturating_sub(started) >= drill_core::GenerationConfig::default().timeout);
        assert_eq!(state.read(Some(102.0)), Duration::MAX);
    }
}
