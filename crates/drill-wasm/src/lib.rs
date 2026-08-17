#![forbid(unsafe_code)]

//! Thin JSON/WASM boundary for the current domain DTOs. Generation,
//! normalization, grading, effort, identity, and retry policy stay in
//! `drill-core`; this crate only parses requests and formats stable errors.

#[cfg(any(target_arch = "wasm32", test))]
use std::cell::Cell;
#[cfg(any(target_arch = "wasm32", test))]
use std::time::Duration;

#[cfg(not(target_arch = "wasm32"))]
use drill_core::generate_worksheet_request as core_generate_worksheet_request;
use drill_core::{
    apply_editor_action as core_apply_editor_action, calculate_effort as core_calculate_effort,
    generate_problem_request as core_generate_problem_request,
    grade_answer_with_schema as core_grade_answer_with_schema,
    normalize_answer as core_normalize_answer, parse_mathlive_answer as core_parse_mathlive_answer,
    AnswerInputInterface, AnswerNode, AnswerSchema, EditorAction, EditorError, EditorState,
    EffortWeights, GenerateProblemRequest, GenerateWorksheetRequest, GenerationError, Problem,
    ProblemSetIdentity, MAX_ANSWER_AST_SIZE, SCHEMA_VERSION,
};
#[cfg(target_arch = "wasm32")]
use drill_core::{
    generate_identity_with_clock, generate_worksheet_request_with_clock, GenerationConfig,
    MonotonicClock,
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
struct EditorActionRequest {
    schema_version: u16,
    input_interface: AnswerInputInterface,
    state: EditorState,
    action: EditorAction,
}

#[derive(Debug, Deserialize)]
struct ParseMathLiveAnswerRequest {
    schema_version: u16,
    input_interface: AnswerInputInterface,
    latex: String,
}

#[derive(Debug, Deserialize)]
struct NormalizeAnswerRequest {
    schema_version: u16,
    answer: AnswerNode,
}

#[derive(Debug, Deserialize)]
struct GradeAnswerRequest {
    schema_version: u16,
    expected: AnswerNode,
    actual: AnswerNode,
    #[serde(default)]
    answer_schema: Option<AnswerSchema>,
}

#[derive(Debug, Deserialize)]
struct CalculateEffortRequest {
    schema_version: u16,
    problem: Problem,
    #[serde(default)]
    weights: Option<EffortWeights>,
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
pub fn generate_problem(input_json: &str) -> String {
    respond_with(input_json, |request: GenerateProblemRequest| {
        validate_schema(request.schema_version)?;
        core_generate_problem_request(&request).map_err(generation_error)
    })
}

#[wasm_bindgen]
pub fn generate_worksheet(input_json: &str) -> String {
    respond_with(input_json, |request: GenerateWorksheetRequest| {
        validate_schema(request.schema_version)?;
        generate_worksheet_for_platform(&request).map_err(generation_error)
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

#[wasm_bindgen]
pub fn regenerate_problem_set(input_json: &str) -> String {
    respond_with(input_json, |problem_set_id: String| {
        let identity: ProblemSetIdentity = problem_set_id
            .parse()
            .map_err(|error| generation_error(GenerationError::InvalidIdentity(error)))?;
        regenerate_for_platform(&identity).map_err(generation_error)
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn regenerate_for_platform(
    identity: &ProblemSetIdentity,
) -> Result<drill_core::Worksheet, GenerationError> {
    drill_core::regenerate_problem_set(&identity.to_string())
}

#[cfg(target_arch = "wasm32")]
fn regenerate_for_platform(
    identity: &ProblemSetIdentity,
) -> Result<drill_core::Worksheet, GenerationError> {
    let config = GenerationConfig::default();
    let clock = BrowserClock::try_new().ok_or_else(|| GenerationError::timeout(config.timeout))?;
    generate_identity_with_clock(identity, &config, &clock)
}

#[wasm_bindgen]
pub fn apply_editor_action(input_json: &str) -> String {
    respond_with(input_json, |request: EditorActionRequest| {
        validate_schema(request.schema_version)?;
        core_apply_editor_action(&request.state, &request.action, &request.input_interface)
            .map_err(editor_error)
    })
}

#[wasm_bindgen]
pub fn parse_mathlive_answer(input_json: &str) -> String {
    respond_with(input_json, |request: ParseMathLiveAnswerRequest| {
        validate_schema(request.schema_version)?;
        core_parse_mathlive_answer(&request.latex, &request.input_interface).map_err(editor_error)
    })
}

#[wasm_bindgen]
pub fn normalize_answer(input_json: &str) -> String {
    respond_with(input_json, |request: NormalizeAnswerRequest| {
        validate_schema(request.schema_version)?;
        validate_answer_size(&request.answer)?;
        Ok(core_normalize_answer(&request.answer))
    })
}

#[wasm_bindgen]
pub fn grade_answer(input_json: &str) -> String {
    respond_with(input_json, |request: GradeAnswerRequest| {
        validate_schema(request.schema_version)?;
        validate_answer_size(&request.expected)?;
        validate_answer_size(&request.actual)?;
        Ok(core_grade_answer_with_schema(
            &request.expected,
            &request.actual,
            request.answer_schema.as_ref(),
        ))
    })
}

#[wasm_bindgen]
pub fn calculate_effort(input_json: &str) -> String {
    respond_with(input_json, |request: CalculateEffortRequest| {
        validate_schema(request.schema_version)?;
        validate_schema(request.problem.schema_version)?;
        Ok(core_calculate_effort(
            &request.problem,
            &request.weights.unwrap_or_default(),
        ))
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
        GenerationError::InvalidIdentity(_) => None,
    };
    ApiError {
        code: error.code().to_owned(),
        message: error.to_string(),
        details,
    }
}

fn editor_error(error: EditorError) -> ApiError {
    let (code, details) = match &error {
        EditorError::InvalidDigit => ("editor_invalid_digit", None),
        EditorError::AnswerSizeLimit { max_size } => (
            "answer_ast_size_limit",
            Some(json!({ "max_size": max_size })),
        ),
        EditorError::IntegerOverflow => ("editor_integer_overflow", None),
        EditorError::NegativeDraft => ("editor_negative_draft", None),
        EditorError::UnsupportedDraftNode => ("editor_unsupported_draft_node", None),
        EditorError::InvalidPath => ("editor_invalid_path", None),
        EditorError::InvalidDecimalDraft => ("editor_invalid_decimal_draft", None),
        EditorError::StructureNotAllowed { structure } => (
            "editor_structure_not_allowed",
            Some(json!({ "structure": structure })),
        ),
        EditorError::InputInterfaceViolation => ("editor_input_interface_violation", None),
    };
    ApiError {
        code: code.to_owned(),
        message: error.to_string(),
        details,
    }
}

fn validate_answer_size(answer: &AnswerNode) -> Result<(), ApiError> {
    if answer.is_within_size_limit() {
        Ok(())
    } else {
        Err(editor_error(EditorError::AnswerSizeLimit {
            max_size: MAX_ANSWER_AST_SIZE,
        }))
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

    fn structured_input_interface() -> Value {
        json!({
            "type": "structured_math",
            "allowed_structures": [
                "fraction", "mixed_fraction", "decimal", "root", "negative",
                "plus_minus", "tuple"
            ]
        })
    }

    #[test]
    fn worksheet_boundary_matches_current_schema_and_identity() {
        let output = generate_worksheet(
            r#"{"schema_version":6,"numeric_theme_id":1,"seed":"Ab3Z","difficulty":3}"#,
        );
        let value = parse(&output);
        assert_eq!(value["schema_version"], SCHEMA_VERSION);
        assert_eq!(value["ok"], true);
        assert_eq!(
            value["data"]["problem_set_id"],
            format!(
                "{}-1-{}-Ab3Z-3",
                SCHEMA_VERSION,
                drill_core::GENERATOR_REVISION_ONE_DIGIT_ADDITION
            )
        );
        assert_eq!(value["data"]["identity"]["numeric_theme_id"], 1);
        assert_eq!(
            value["data"]["identity"]["generator_revision"],
            drill_core::GENERATOR_REVISION_ONE_DIGIT_ADDITION
        );
        assert_eq!(value["data"]["layout"]["problem_count"], 20);
        assert_eq!(value["data"]["problems"][0]["prompt"]["kind"], "addition");
        assert_eq!(value["data"]["problems"][0]["answer_schema"]["min"], "1");
        assert_eq!(value["data"]["problems"][0]["answer_schema"]["max"], "18");
        assert_eq!(
            value["data"]["problems"][0]["input_interface"],
            simple_input_interface()
        );
        assert!(value["data"]["problems"][0]["canonical_answer"]["value"].is_string());
        let big_num_step = value["data"]["problems"][0]["solution_graph"]["steps"]
            .as_array()
            .unwrap()
            .iter()
            .find(|step| step["operation"]["kind"] == "big_num")
            .unwrap();
        assert!(big_num_step["operation"]["magnitude"].is_string());

        let regenerated_id = format!(
            "\"{}-1-{}-Ab3Z-3\"",
            SCHEMA_VERSION,
            drill_core::GENERATOR_REVISION_ONE_DIGIT_ADDITION
        );
        let regenerated = parse(&regenerate_problem_set(&regenerated_id));
        assert_eq!(regenerated["data"], value["data"]);
    }

    #[test]
    fn non_current_schema_requests_and_ids_fail_closed_at_wasm_boundary() {
        let request = parse(&generate_worksheet(
            r#"{"schema_version":2,"numeric_theme_id":1,"seed":"Ab3Z","difficulty":3}"#,
        ));
        assert_eq!(request["error"]["code"], "unsupported_schema_version");
        let id = parse(&regenerate_problem_set(r#""2-1-2-Ab3Z-3""#));
        assert_eq!(id["error"]["code"], "unsupported_schema_version");

        let missing_schema = parse(&generate_worksheet(
            r#"{"numeric_theme_id":1,"seed":"Ab3Z","difficulty":3}"#,
        ));
        assert_eq!(missing_schema["error"]["code"], "invalid_request");

        let missing_editor_path = parse(&apply_editor_action(
            r#"{"schema_version":6,"input_interface":{"type":"simple_numeric","allow_decimal":false,"allow_negative":false},"state":{"answer":{"type":"empty"},"cursor":0,"committed":false},"action":{"type":"insert_digit","digit":4}}"#,
        ));
        assert_eq!(missing_editor_path["error"]["code"], "invalid_request");
    }

    #[test]
    fn timeout_and_attempt_errors_remain_distinct() {
        let timeout = parse(&generate_worksheet(
            r#"{"schema_version":6,"numeric_theme_id":1,"seed":"Ab3Z","difficulty":3,"timeout_ms":0}"#,
        ));
        assert_eq!(timeout["error"]["code"], "generation_timeout");

        let attempts = parse(&generate_worksheet(
            r#"{"schema_version":6,"numeric_theme_id":1,"seed":"Ab3Z","difficulty":3,"timeout_ms":1000,"max_attempts":0}"#,
        ));
        assert_eq!(attempts["error"]["code"], "generation_attempt_limit");
    }

    #[test]
    fn ast_editor_grade_and_effort_use_matching_json_dtos() {
        let edited = parse(&apply_editor_action(
            r#"{"schema_version":6,"input_interface":{"type":"simple_numeric","allow_decimal":false,"allow_negative":false},"state":{"answer":{"type":"empty"},"cursor":0,"active_path":[],"committed":false},"action":{"type":"insert_digit","digit":4}}"#,
        ));
        assert_eq!(
            edited["data"]["answer"],
            json!({"type":"integer","value":"4"})
        );
        assert_eq!(edited["data"]["active_path"], json!([]));

        let structured = parse(&apply_editor_action(
            r#"{"schema_version":6,"input_interface":{"type":"structured_math","allowed_structures":["fraction"]},"state":{"answer":{"type":"empty"},"cursor":0,"active_path":[],"committed":false},"action":{"type":"insert_structure","structure":"fraction"}}"#,
        ));
        assert_eq!(
            structured["data"]["answer"],
            json!({"type":"fraction","value":{"numerator":{"type":"empty"},"denominator":{"type":"empty"}}})
        );
        assert_eq!(structured["data"]["active_path"], json!([0]));

        let oversized_decimal = parse(&apply_editor_action(
            r#"{"schema_version":6,"input_interface":{"type":"simple_numeric","allow_decimal":false,"allow_negative":false},"state":{"answer":{"type":"exact_decimal","value":{"coefficient":"0","scale":4294967295}},"cursor":0,"active_path":[],"committed":false},"action":{"type":"insert_digit","digit":1}}"#,
        ));
        assert_eq!(oversized_decimal["ok"], false);
        assert_eq!(oversized_decimal["error"]["code"], "answer_ast_size_limit");

        let normalized = parse(&normalize_answer(
            r#"{"schema_version":6,"answer":{"type":"exact_decimal","value":{"coefficient":"300","scale":3}}}"#,
        ));
        assert_eq!(
            normalized["data"],
            json!({"type":"fraction","value":{"numerator":{"type":"integer","value":"3"},"denominator":{"type":"integer","value":"10"}}})
        );

        let graded = parse(&grade_answer(
            r#"{"schema_version":6,"expected":{"type":"integer","value":"4"},"actual":{"type":"integer","value":"4"}}"#,
        ));
        assert_eq!(graded["data"]["is_correct"], true);
        assert_eq!(graded["data"]["warnings"], json!([]));

        let equivalent_fraction = parse(&grade_answer(
            r#"{"schema_version":6,"expected":{"type":"fraction","value":{"numerator":{"type":"integer","value":"1"},"denominator":{"type":"integer","value":"2"}}},"actual":{"type":"exact_decimal","value":{"coefficient":"5","scale":1}}}"#,
        ));
        assert_eq!(equivalent_fraction["data"]["is_correct"], true);
        assert_eq!(equivalent_fraction["data"]["warnings"], json!([]));

        let reducible_fraction = parse(&grade_answer(
            r#"{"schema_version":6,"expected":{"type":"fraction","value":{"numerator":{"type":"integer","value":"1"},"denominator":{"type":"integer","value":"2"}}},"actual":{"type":"fraction","value":{"numerator":{"type":"integer","value":"2"},"denominator":{"type":"integer","value":"4"}}}}"#,
        ));
        assert_eq!(reducible_fraction["data"]["is_correct"], true);
        assert_eq!(
            reducible_fraction["data"]["warnings"],
            json!(["fraction_not_reduced"])
        );

        let generated = parse(&generate_problem(
            r#"{"schema_version":6,"numeric_theme_id":1,"seed":"Ab3Z"}"#,
        ));
        let effort_request = json!({
            "schema_version": SCHEMA_VERSION,
            "problem": generated["data"]
        });
        let effort = parse(&calculate_effort(&effort_request.to_string()));
        assert_eq!(effort["ok"], true);
        assert!(effort["data"]["value"].as_f64().unwrap() >= 3.0);
    }

    #[test]
    fn editor_interface_is_authoritative_and_nan_error_is_recoverable() {
        let blocked = parse(&apply_editor_action(
            &json!({
                "schema_version": SCHEMA_VERSION,
                "input_interface": simple_input_interface(),
                "state": {"answer": {"type": "empty"}, "cursor": 0, "active_path": [], "committed": false},
                "action": {"type": "insert_structure", "structure": "fraction"}
            })
            .to_string(),
        ));
        assert_eq!(blocked["error"]["code"], "editor_structure_not_allowed");
        assert_eq!(blocked["error"]["details"]["structure"], "fraction");

        for structure in ["decimal", "negative"] {
            let blocked = parse(&apply_editor_action(
                &json!({
                    "schema_version": SCHEMA_VERSION,
                    "input_interface": simple_input_interface(),
                    "state": {"answer": {"type": "empty"}, "cursor": 0, "active_path": [], "committed": false},
                    "action": {"type": "insert_structure", "structure": structure}
                })
                .to_string(),
            ));
            assert_eq!(blocked["error"]["code"], "editor_structure_not_allowed");
            assert_eq!(blocked["error"]["details"]["structure"], structure);
        }

        let nan = parse(&apply_editor_action(
            &json!({
                "schema_version": SCHEMA_VERSION,
                "input_interface": simple_input_interface(),
                "state": {
                    "answer": {"type": "nan_error", "value": "3.1.4.5"},
                    "cursor": 7,
                    "active_path": [],
                    "committed": false
                },
                "action": {"type": "insert_digit", "digit": 9}
            })
            .to_string(),
        ));
        assert_eq!(
            nan["data"]["answer"],
            json!({"type": "nan_error", "value": "3.1.4.59"})
        );

        let graded = parse(&grade_answer(
            r#"{"schema_version":6,"expected":{"type":"integer","value":"4"},"actual":{"type":"nan_error","value":"3.1.4.5"}}"#,
        ));
        assert_eq!(graded["data"]["is_correct"], false);
        assert_eq!(graded["data"]["warnings"], json!([]));
    }

    #[test]
    fn editor_boundary_rejects_bad_positions_and_capability_violations() {
        let cleared_malformed = parse(&apply_editor_action(
            &json!({
                "schema_version": SCHEMA_VERSION,
                "input_interface": simple_input_interface(),
                "state": {"answer": {"type": "variable", "value": "x"}, "cursor": 999, "active_path": [9], "committed": true},
                "action": {"type": "clear"}
            })
            .to_string(),
        ));
        assert_eq!(cleared_malformed["ok"], true);
        assert_eq!(cleared_malformed["data"]["answer"]["type"], "empty");

        let invalid_path = parse(&apply_editor_action(
            &json!({
                "schema_version": SCHEMA_VERSION,
                "input_interface": simple_input_interface(),
                "state": {"answer": {"type": "integer", "value": "12"}, "cursor": 0, "active_path": [7], "committed": false},
                "action": {"type": "commit"}
            })
            .to_string(),
        ));
        assert_eq!(invalid_path["error"]["code"], "editor_invalid_path");

        let invalid_cursor = parse(&apply_editor_action(
            &json!({
                "schema_version": SCHEMA_VERSION,
                "input_interface": simple_input_interface(),
                "state": {"answer": {"type": "integer", "value": "12"}, "cursor": 3, "active_path": [], "committed": false},
                "action": {"type": "commit"}
            })
            .to_string(),
        ));
        assert_eq!(invalid_cursor["error"]["code"], "editor_invalid_path");

        let unicode = parse(&apply_editor_action(
            &json!({
                "schema_version": SCHEMA_VERSION,
                "input_interface": simple_input_interface(),
                "state": {"answer": {"type": "nan_error", "value": "😀1"}, "cursor": 1, "active_path": [], "committed": false},
                "action": {"type": "insert_digit", "digit": 9}
            })
            .to_string(),
        ));
        assert_eq!(
            unicode["data"]["answer"],
            json!({"type": "nan_error", "value": "😀91"})
        );
        assert_eq!(unicode["data"]["cursor"], 2);

        let select_invalid = parse(&apply_editor_action(
            &json!({
                "schema_version": SCHEMA_VERSION,
                "input_interface": structured_input_interface(),
                "state": {"answer": {"type": "tuple", "value": [
                    {"type": "integer", "value": "1"}, {"type": "integer", "value": "2"}
                ]}, "cursor": 1, "active_path": [0], "committed": false},
                "action": {"type": "select_slot", "path": [9], "cursor": 0}
            })
            .to_string(),
        ));
        assert_eq!(select_invalid["error"]["code"], "editor_invalid_path");

        let select_missing_cursor = parse(&apply_editor_action(
            &json!({
                "schema_version": SCHEMA_VERSION,
                "input_interface": structured_input_interface(),
                "state": {"answer": {"type": "tuple", "value": [
                    {"type": "integer", "value": "1"}, {"type": "integer", "value": "2"}
                ]}, "cursor": 1, "active_path": [0], "committed": false},
                "action": {"type": "select_slot", "path": [1]}
            })
            .to_string(),
        ));
        assert_eq!(select_missing_cursor["error"]["code"], "invalid_request");

        let select_cursor = parse(&apply_editor_action(
            &json!({
                "schema_version": SCHEMA_VERSION,
                "input_interface": structured_input_interface(),
                "state": {"answer": {"type": "tuple", "value": [
                    {"type": "integer", "value": "1"}, {"type": "integer", "value": "2"}
                ]}, "cursor": 1, "active_path": [0], "committed": false},
                "action": {"type": "select_slot", "path": [1], "cursor": 2}
            })
            .to_string(),
        ));
        assert_eq!(select_cursor["error"]["code"], "editor_invalid_path");

        let existing_decimal = parse(&apply_editor_action(
            &json!({
                "schema_version": SCHEMA_VERSION,
                "input_interface": simple_input_interface(),
                "state": {"answer": {"type": "exact_decimal", "value": {"coefficient": "12", "scale": 1}}, "cursor": 3, "active_path": [], "committed": false},
                "action": {"type": "commit"}
            })
            .to_string(),
        ));
        assert_eq!(
            existing_decimal["error"]["code"],
            "editor_input_interface_violation"
        );

        let structured_existing = parse(&apply_editor_action(
            &json!({
                "schema_version": SCHEMA_VERSION,
                "input_interface": {"type": "structured_math", "allowed_structures": ["fraction"]},
                "state": {"answer": {"type": "tuple", "value": [{"type": "integer", "value": "1"}]}, "cursor": 1, "active_path": [0], "committed": false},
                "action": {"type": "commit"}
            })
            .to_string(),
        ));
        assert_eq!(
            structured_existing["error"]["code"],
            "editor_input_interface_violation"
        );

        let signed_nested = parse(&apply_editor_action(
            &json!({
                "schema_version": SCHEMA_VERSION,
                "input_interface": {"type": "structured_math", "allowed_structures": ["fraction"]},
                "state": {
                    "answer": {"type": "fraction", "value": {
                        "numerator": {"type": "integer", "value": "-1"},
                        "denominator": {"type": "integer", "value": "2"}
                    }},
                    "cursor": 1,
                    "active_path": [1],
                    "committed": false
                },
                "action": {"type": "commit"}
            })
            .to_string(),
        ));
        assert_eq!(
            signed_nested["error"]["code"],
            "editor_input_interface_violation"
        );

        let retained_nan = parse(&apply_editor_action(
            &json!({
                "schema_version": SCHEMA_VERSION,
                "input_interface": simple_input_interface(),
                "state": {"answer": {"type": "nan_error", "value": "1.2"}, "cursor": 2, "active_path": [], "committed": false},
                "action": {"type": "delete"}
            })
            .to_string(),
        ));
        assert_eq!(
            retained_nan["data"]["answer"],
            json!({"type": "nan_error", "value": "1."})
        );

        let digits_recovered = parse(&apply_editor_action(
            &json!({
                "schema_version": SCHEMA_VERSION,
                "input_interface": simple_input_interface(),
                "state": {"answer": {"type": "nan_error", "value": "12x3"}, "cursor": 2, "active_path": [], "committed": false},
                "action": {"type": "delete"}
            })
            .to_string(),
        ));
        assert_eq!(
            digits_recovered["data"]["answer"],
            json!({"type": "integer", "value": "123"})
        );
    }

    #[test]
    fn oversized_answers_are_rejected_at_editor_and_answer_boundaries() {
        let oversized_answer = json!({
            "type": "tuple",
            "value": (0..=MAX_ANSWER_AST_SIZE)
                .map(|_| json!({"type": "integer", "value": "1"}))
                .collect::<Vec<_>>()
        });
        let oversized_empty_answer = json!({
            "type": "tuple",
            "value": (0..=MAX_ANSWER_AST_SIZE)
                .map(|_| json!({"type": "empty"}))
                .collect::<Vec<_>>()
        });
        let oversized_state = json!({
            "answer": oversized_answer.clone(),
            "cursor": 0,
            "active_path": [0],
            "committed": false
        });
        let assert_size_limit = |value: &Value| {
            assert_eq!(value["ok"], false);
            assert_eq!(value["error"]["code"], "answer_ast_size_limit");
            assert_eq!(value["error"]["details"]["max_size"], MAX_ANSWER_AST_SIZE);
        };

        let commit = parse(&apply_editor_action(
            &json!({
                "schema_version": SCHEMA_VERSION,
                "input_interface": structured_input_interface(),
                "state": oversized_state.clone(),
                "action": {"type": "commit"}
            })
            .to_string(),
        ));
        assert_size_limit(&commit);

        let select_slot = parse(&apply_editor_action(
            &json!({
                "schema_version": SCHEMA_VERSION,
                "input_interface": structured_input_interface(),
                "state": oversized_state.clone(),
                "action": {"type": "select_slot", "path": [0], "cursor": 0}
            })
            .to_string(),
        ));
        assert_size_limit(&select_slot);

        let oversized_empty_state = json!({
            "answer": oversized_empty_answer.clone(),
            "cursor": 0,
            "active_path": [0],
            "committed": false
        });
        let empty_commit = parse(&apply_editor_action(
            &json!({
                "schema_version": SCHEMA_VERSION,
                "input_interface": structured_input_interface(),
                "state": oversized_empty_state.clone(),
                "action": {"type": "commit"}
            })
            .to_string(),
        ));
        assert_size_limit(&empty_commit);
        let empty_select_slot = parse(&apply_editor_action(
            &json!({
                "schema_version": SCHEMA_VERSION,
                "input_interface": structured_input_interface(),
                "state": oversized_empty_state.clone(),
                "action": {"type": "select_slot", "path": [0], "cursor": 0}
            })
            .to_string(),
        ));
        assert_size_limit(&empty_select_slot);

        let cleared_empty = parse(&apply_editor_action(
            &json!({
                "schema_version": SCHEMA_VERSION,
                "input_interface": structured_input_interface(),
                "state": oversized_empty_state,
                "action": {"type": "clear"}
            })
            .to_string(),
        ));
        assert_eq!(cleared_empty["ok"], true);
        assert_eq!(
            cleared_empty["data"],
            json!({
                "answer": {"type": "empty"},
                "cursor": 0,
                "active_path": [],
                "committed": false
            })
        );

        let cleared = parse(&apply_editor_action(
            &json!({
                "schema_version": SCHEMA_VERSION,
                "input_interface": structured_input_interface(),
                "state": oversized_state,
                "action": {"type": "clear"}
            })
            .to_string(),
        ));
        assert_eq!(cleared["ok"], true);
        assert_eq!(cleared["data"]["answer"], json!({"type": "empty"}));

        let normalized = parse(&normalize_answer(
            &json!({"schema_version": SCHEMA_VERSION, "answer": oversized_answer.clone()})
                .to_string(),
        ));
        assert_size_limit(&normalized);
        let normalized_empty = parse(&normalize_answer(
            &json!({"schema_version": SCHEMA_VERSION, "answer": oversized_empty_answer.clone()})
                .to_string(),
        ));
        assert_size_limit(&normalized_empty);

        let graded_composite = parse(&grade_answer(
            &json!({
                "schema_version": SCHEMA_VERSION,
                "expected": oversized_answer,
                "actual": {"type": "integer", "value": "1"}
            })
            .to_string(),
        ));
        assert_size_limit(&graded_composite);
        let graded_empty_composite = parse(&grade_answer(
            &json!({
                "schema_version": SCHEMA_VERSION,
                "expected": oversized_empty_answer,
                "actual": {"type": "empty"}
            })
            .to_string(),
        ));
        assert_size_limit(&graded_empty_composite);

        let extreme_decimal = json!({
            "type": "exact_decimal",
            "value": {"coefficient": "0", "scale": u32::MAX}
        });
        let normalized_extreme = parse(&normalize_answer(
            &json!({"schema_version": SCHEMA_VERSION, "answer": extreme_decimal.clone()})
                .to_string(),
        ));
        assert_size_limit(&normalized_extreme);
        let graded_extreme = parse(&grade_answer(
            &json!({
                "schema_version": SCHEMA_VERSION,
                "expected": {"type": "integer", "value": "0"},
                "actual": extreme_decimal
            })
            .to_string(),
        ));
        assert_size_limit(&graded_extreme);
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
            let normalized = parse(&normalize_answer(
                &json!({"schema_version": SCHEMA_VERSION, "answer": answer.clone()}).to_string(),
            ));
            assert_eq!(normalized["ok"], true);
            assert_eq!(normalized["data"], answer);

            let graded = parse(&grade_answer(
                &json!({
                    "schema_version": SCHEMA_VERSION,
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
    fn answer_i64_payloads_cross_json_as_exact_decimal_strings() {
        let exact = "999999999999999999";
        let graded = parse(&grade_answer(&format!(
            r#"{{"schema_version":6,"expected":{{"type":"integer","value":"{exact}"}},"actual":{{"type":"integer","value":"{exact}"}}}}"#
        )));
        assert_eq!(graded["data"]["is_correct"], true);
        assert_eq!(graded["data"]["actual"]["value"], exact);
    }

    #[test]
    fn browser_clock_failures_latch_to_timeout() {
        let state = BrowserClockState::try_new(100.0).unwrap();
        let started = state.read(Some(101.0));
        let failed = state.read(None);
        assert_eq!(failed, Duration::MAX);
        assert!(failed.saturating_sub(started) >= drill_core::DEFAULT_TIMEOUT);
        assert_eq!(state.read(Some(102.0)), Duration::MAX);
    }
}
