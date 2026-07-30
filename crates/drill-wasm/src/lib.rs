#![forbid(unsafe_code)]

//! Thin JSON/WASM boundary.  All generation, editing, normalization, grading,
//! and effort behavior lives in `drill-core`; this crate only validates DTOs
//! and translates typed results/errors to JSON strings for JavaScript.

#[cfg(any(target_arch = "wasm32", test))]
use std::cell::Cell;
#[cfg(any(target_arch = "wasm32", test))]
use std::time::Duration;

#[cfg(not(target_arch = "wasm32"))]
use drill_core::generate_worksheet_with_config;
use drill_core::{
    apply_editor_action as core_apply_editor_action, calculate_effort as core_calculate_effort,
    generate_problem as core_generate_problem, grade_answer as core_grade_answer,
    normalize_answer as core_normalize_answer, AnswerNode, EditorAction, EditorError, EditorState,
    EffortWeights, GenerateProblemRequest, GenerateWorksheetRequest, GenerationConfig,
    GenerationError, GradeResult, Problem, Worksheet, SCHEMA_VERSION,
};
#[cfg(target_arch = "wasm32")]
use drill_core::{generate_worksheet_with_clock, MonotonicClock};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Value};
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    /// The `catch` wrapper converts a missing or throwing host clock into a
    /// regular Result rather than a WebAssembly trap.
    #[wasm_bindgen(catch, js_namespace = performance, js_name = now)]
    fn performance_now() -> Result<f64, JsValue>;
}

/// Browser-only monotonic clock. Native callers continue to use drill-core's
/// `SystemClock`; the WASM boundary avoids `std::time::Instant`, which traps on
/// wasm32-unknown-unknown.
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
            Some(now_ms) if now_ms.is_finite() && now_ms >= self.origin_ms => now_ms,
            _ => return self.failure_duration(read_index),
        };
        let elapsed_ms = now_ms - self.origin_ms;
        if elapsed_ms < previous {
            return self.failure_duration(read_index);
        }
        self.last_elapsed_ms.set(elapsed_ms);

        // performance.now() is normally small, but clamp the conversion so a
        // malformed host value can never panic Duration construction.
        let nanos = (elapsed_ms * 1_000_000.0).min(u64::MAX as f64) as u64;
        Duration::from_nanos(nanos)
    }

    fn failure_duration(&self, read_index: u64) -> Duration {
        self.failed.set(true);
        // drill-core records its starting timestamp from the first `now()`
        // call. If that read itself fails, return zero once and MAX on the
        // following read; otherwise MAX immediately. Either sequence makes
        // saturating elapsed time cross every finite configured timeout.
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
#[serde(tag = "kind", rename_all = "snake_case")]
enum BoundaryEditorAction {
    InsertDigit { digit: u8 },
    DeleteBackward,
    DeleteForward,
    MoveLeft,
    MoveRight,
    Clear,
    Commit,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
struct EditorActionRequest {
    schema_version: u16,
    state: Value,
    action: Value,
}

impl Default for EditorActionRequest {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            state: Value::Null,
            action: Value::Null,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(default)]
struct IntegerDraftDto {
    kind: String,
    #[serde(deserialize_with = "deserialize_digits")]
    digits: Vec<u8>,
}

impl Default for IntegerDraftDto {
    fn default() -> Self {
        Self {
            kind: "integer".to_owned(),
            digits: Vec::new(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(default)]
struct EditorStateDto {
    schema_version: u16,
    node: IntegerDraftDto,
    cursor: usize,
    committed: bool,
}

impl Default for EditorStateDto {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            node: IntegerDraftDto::default(),
            cursor: 0,
            committed: false,
        }
    }
}

#[derive(Debug, Serialize)]
struct ProblemDto {
    schema_version: u16,
    problem_id: String,
    skill_id: &'static str,
    left: u8,
    right: u8,
    prompt: PromptDto,
    answer_schema: AnswerSchemaDto,
    canonical_answer: CanonicalAnswerDto,
    operation_counts: drill_core::OperationCounts,
}

#[derive(Debug, Serialize)]
struct PromptDto {
    kind: &'static str,
    left: u8,
    right: u8,
}

#[derive(Debug, Serialize)]
struct AnswerSchemaDto {
    kind: &'static str,
    min: u8,
    max: u8,
}

#[derive(Debug, Serialize, Deserialize)]
struct CanonicalAnswerDto {
    kind: String,
    value: Option<i64>,
}

#[derive(Debug, Serialize)]
struct WorksheetDto {
    schema_version: u16,
    generator_version: String,
    skill_id: String,
    curriculum_path: Vec<CurriculumPathSegmentDto>,
    seed: String,
    layout: drill_core::LayoutMetadata,
    problems: Vec<ProblemDto>,
}

#[derive(Debug, Serialize)]
struct CurriculumPathSegmentDto {
    id: String,
    label: String,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
struct NormalizeAnswerRequest {
    schema_version: u16,
    answer: Value,
}

impl Default for NormalizeAnswerRequest {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            answer: Value::Null,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default)]
struct GradeAnswerRequest {
    schema_version: u16,
    expected: Value,
    actual: Value,
}

impl Default for GradeAnswerRequest {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            expected: Value::Null,
            actual: Value::Null,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default)]
struct CalculateEffortRequest {
    schema_version: u16,
    problem: Value,
    weights: Option<EffortWeights>,
}

impl Default for CalculateEffortRequest {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            problem: Value::Null,
            weights: None,
        }
    }
}

#[derive(Debug, Serialize)]
struct GradeResultDto {
    schema_version: u16,
    status: drill_core::GradeStatus,
    is_correct: bool,
    expected: CanonicalAnswerDto,
    actual: CanonicalAnswerDto,
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
    let request = match parse_seed_request::<GenerateProblemRequest>(input_json) {
        Ok(request) => request,
        Err(error) => return error_response(error),
    };
    if let Err(error) = validate_schema(request.schema_version) {
        return error_response(error);
    }
    match core_generate_problem(&request.seed) {
        Ok(problem) => success_response(problem_dto(&problem)),
        Err(error) => error_response(generation_error(error)),
    }
}

#[wasm_bindgen]
pub fn generate_worksheet(input_json: &str) -> String {
    let request = match parse_seed_request::<GenerateWorksheetRequest>(input_json) {
        Ok(request) => request,
        Err(error) => return error_response(error),
    };
    if let Err(error) = validate_schema(request.schema_version) {
        return error_response(error);
    }
    let mut config = GenerationConfig::default();
    if let Some(problem_count) = request.problem_count {
        config.problem_count = problem_count;
    }
    if let Some(timeout_ms) = request.timeout_ms {
        config.timeout = std::time::Duration::from_millis(timeout_ms);
    }
    if let Some(max_attempts) = request.max_attempts {
        config.max_attempts = max_attempts;
    }
    match generate_worksheet_for_platform(&request.seed, &config) {
        Ok(worksheet) => success_response(worksheet_dto(&worksheet)),
        Err(error) => error_response(generation_error(error)),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn generate_worksheet_for_platform(
    seed: &str,
    config: &GenerationConfig,
) -> Result<Worksheet, GenerationError> {
    generate_worksheet_with_config(seed, config)
}

#[cfg(target_arch = "wasm32")]
fn generate_worksheet_for_platform(
    seed: &str,
    config: &GenerationConfig,
) -> Result<Worksheet, GenerationError> {
    let clock = BrowserClock::try_new().ok_or_else(|| GenerationError::timeout(config.timeout))?;
    generate_worksheet_with_clock(seed, config, &clock)
}

#[wasm_bindgen]
pub fn apply_editor_action(input_json: &str) -> String {
    let request: EditorActionRequest = match parse_json(input_json) {
        Ok(request) => request,
        Err(error) => return error_response(error),
    };
    if let Err(error) = validate_schema(request.schema_version) {
        return error_response(error);
    }
    let state = match parse_editor_state(&request.state) {
        Ok(state) => state,
        Err(error) => return error_response(error),
    };
    let action = match parse_editor_action(&request.action) {
        Ok(action) => action,
        Err(error) => return error_response(error),
    };
    match core_apply_editor_action(&state, &action) {
        Ok(state) => success_response(editor_state_dto(&state)),
        Err(error) => error_response(editor_error(error)),
    }
}

#[wasm_bindgen]
pub fn normalize_answer(input_json: &str) -> String {
    let request: NormalizeAnswerRequest = match parse_json(input_json) {
        Ok(request) => request,
        Err(_) => match serde_json::from_str::<AnswerNode>(input_json) {
            Ok(answer) => return success_response(core_normalize_answer(&answer)),
            Err(error) => return error_response(invalid_request(&error.to_string())),
        },
    };
    // Unknown fields are intentionally ignored by serde so a raw AnswerNode
    // object (`{"type":"integer","value":4}`) can deserialize as the
    // request default.  Retry it as a node before reporting a missing field.
    let answer = if request.answer.is_null() {
        let value: Value = match serde_json::from_str(input_json) {
            Ok(value) => value,
            Err(error) => return error_response(invalid_request(&error.to_string())),
        };
        match parse_answer_node(&value) {
            Ok(answer) => answer,
            Err(error) => return error_response(error),
        }
    } else {
        match parse_answer_node(&request.answer) {
            Ok(answer) => answer,
            Err(error) => return error_response(error),
        }
    };
    if let Err(error) = validate_schema(request.schema_version) {
        return error_response(error);
    }
    success_response(canonical_answer_dto(&core_normalize_answer(&answer)))
}

#[wasm_bindgen]
pub fn grade_answer(input_json: &str) -> String {
    let request: GradeAnswerRequest = match parse_json(input_json) {
        Ok(request) => request,
        Err(error) => return error_response(error),
    };
    if let Err(error) = validate_schema(request.schema_version) {
        return error_response(error);
    }
    let expected = match parse_answer_node(&request.expected) {
        Ok(answer) => answer,
        Err(error) => return error_response(error),
    };
    let actual = match parse_answer_node(&request.actual) {
        Ok(answer) => answer,
        Err(error) => return error_response(error),
    };
    success_response(grade_result_dto(&core_grade_answer(&expected, &actual)))
}

#[wasm_bindgen]
pub fn calculate_effort(input_json: &str) -> String {
    let request: CalculateEffortRequest = match parse_json(input_json) {
        Ok(request) => request,
        Err(error) => return error_response(error),
    };
    if let Err(error) = validate_schema(request.schema_version) {
        return error_response(error);
    }
    let problem = match parse_problem(&request.problem) {
        Ok(problem) => problem,
        Err(error) => return error_response(error),
    };
    success_response(core_calculate_effort(
        &problem,
        &request.weights.unwrap_or_default(),
    ))
}

fn problem_dto(problem: &Problem) -> ProblemDto {
    ProblemDto {
        schema_version: SCHEMA_VERSION,
        problem_id: problem.id.to_string(),
        skill_id: drill_core::SKILL_ID,
        left: problem.left,
        right: problem.right,
        prompt: PromptDto {
            kind: "addition",
            left: problem.left,
            right: problem.right,
        },
        answer_schema: AnswerSchemaDto {
            kind: "integer",
            min: drill_core::MIN_ANSWER,
            max: drill_core::MAX_ANSWER,
        },
        canonical_answer: CanonicalAnswerDto {
            kind: "integer".to_owned(),
            value: Some(i64::from(problem.answer)),
        },
        operation_counts: problem.operation_counts.clone(),
    }
}

fn worksheet_dto(worksheet: &Worksheet) -> WorksheetDto {
    WorksheetDto {
        schema_version: worksheet.schema_version,
        generator_version: worksheet.generator_version.clone(),
        skill_id: worksheet.skill_id.clone(),
        curriculum_path: worksheet
            .curriculum_path
            .iter()
            .enumerate()
            .map(|(index, label)| CurriculumPathSegmentDto {
                id: match index {
                    0 => "root".to_owned(),
                    1 => "jp-grade-1".to_owned(),
                    _ => drill_core::SKILL_ID.to_owned(),
                },
                label: label.clone(),
            })
            .collect(),
        seed: worksheet.seed.clone(),
        layout: worksheet.layout.clone(),
        problems: worksheet.problems.iter().map(problem_dto).collect(),
    }
}

fn editor_state_dto(state: &EditorState) -> EditorStateDto {
    let digits = match &state.answer {
        AnswerNode::Empty => Vec::new(),
        AnswerNode::Integer(value) if *value >= 0 => value
            .to_string()
            .bytes()
            .filter_map(|byte| byte.checked_sub(b'0'))
            .collect(),
        AnswerNode::Integer(value) => value
            .unsigned_abs()
            .to_string()
            .bytes()
            .filter_map(|byte| byte.checked_sub(b'0'))
            .collect(),
    };
    EditorStateDto {
        schema_version: SCHEMA_VERSION,
        node: IntegerDraftDto {
            kind: "integer".to_owned(),
            digits,
        },
        cursor: state.cursor,
        committed: state.committed,
    }
}

fn canonical_answer_dto(answer: &AnswerNode) -> CanonicalAnswerDto {
    CanonicalAnswerDto {
        kind: "integer".to_owned(),
        value: answer.as_integer(),
    }
}

fn grade_result_dto(result: &GradeResult) -> GradeResultDto {
    GradeResultDto {
        schema_version: SCHEMA_VERSION,
        status: result.status.clone(),
        is_correct: result.is_correct,
        expected: canonical_answer_dto(&result.expected),
        actual: canonical_answer_dto(&result.actual),
    }
}

fn parse_editor_state(value: &Value) -> Result<EditorState, ApiError> {
    if value.is_null() {
        return Err(invalid_request("state is required"));
    }
    if value.get("node").is_some() {
        if let Ok(dto) = serde_json::from_value::<EditorStateDto>(value.clone()) {
            validate_schema(dto.schema_version)?;
            if dto.node.kind != "integer" {
                return Err(invalid_request("editor state node kind must be integer"));
            }
            let answer = answer_from_digits(&dto.node.digits)?;
            return Ok(EditorState {
                answer,
                cursor: dto.cursor,
                committed: dto.committed,
            });
        }
    }
    let state = serde_json::from_value::<EditorState>(value.clone())
        .map_err(|error| invalid_request(&format!("invalid editor state: {error}")))?;
    validate_answer_size(state.answer.clone())?;
    Ok(state)
}

fn parse_editor_action(value: &Value) -> Result<EditorAction, ApiError> {
    if value.is_null() {
        return Err(invalid_request("action is required"));
    }
    if let Ok(action) = serde_json::from_value::<BoundaryEditorAction>(value.clone()) {
        return Ok(match action {
            BoundaryEditorAction::InsertDigit { digit } => EditorAction::InsertDigit { digit },
            BoundaryEditorAction::DeleteBackward => EditorAction::Backspace,
            BoundaryEditorAction::DeleteForward => EditorAction::Delete,
            BoundaryEditorAction::MoveLeft => EditorAction::MoveLeft,
            BoundaryEditorAction::MoveRight => EditorAction::MoveRight,
            BoundaryEditorAction::Clear => EditorAction::Clear,
            BoundaryEditorAction::Commit => EditorAction::Commit,
        });
    }
    serde_json::from_value::<EditorAction>(value.clone())
        .map_err(|error| invalid_request(&format!("invalid editor action: {error}")))
}

fn parse_answer_node(value: &Value) -> Result<AnswerNode, ApiError> {
    if let Ok(answer) = serde_json::from_value::<AnswerNode>(value.clone()) {
        return validate_answer_size(answer);
    }
    let object = value
        .as_object()
        .ok_or_else(|| invalid_request("answer must be an object"))?;
    let kind = object
        .get("kind")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid_request("answer kind is required"))?;
    if kind != "integer" {
        return Err(invalid_request("answer kind must be integer"));
    }
    if let Some(value) = object.get("value") {
        if value.is_null() {
            return Ok(AnswerNode::Empty);
        }
        let value = value
            .as_i64()
            .ok_or_else(|| invalid_request("integer answer value must be an integer"))?;
        return validate_answer_size(AnswerNode::Integer(value));
    }
    let digits = object
        .get("digits")
        .ok_or_else(|| invalid_request("integer answer digits or value is required"))?;
    let digits = digits_from_value(digits)?;
    answer_from_digits(&digits)
}

fn parse_problem(value: &Value) -> Result<Problem, ApiError> {
    if value.is_null() {
        return Err(invalid_request("problem is required"));
    }
    if let Ok(problem) = serde_json::from_value::<Problem>(value.clone()) {
        return Ok(problem);
    }
    let object = value
        .as_object()
        .ok_or_else(|| invalid_request("problem must be an object"))?;
    let problem_id = object
        .get("problem_id")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid_request("problem_id is required"))?;
    let id = problem_id
        .parse::<u32>()
        .map_err(|_| invalid_request("problem_id must contain a numeric id"))?;
    let left = object
        .get("left")
        .and_then(Value::as_u64)
        .and_then(|value| u8::try_from(value).ok())
        .ok_or_else(|| invalid_request("problem left operand is required"))?;
    let right = object
        .get("right")
        .and_then(Value::as_u64)
        .and_then(|value| u8::try_from(value).ok())
        .ok_or_else(|| invalid_request("problem right operand is required"))?;
    let canonical_answer = object
        .get("canonical_answer")
        .ok_or_else(|| invalid_request("canonical_answer is required"))?;
    let answer = parse_answer_node(canonical_answer)?
        .as_integer()
        .ok_or_else(|| invalid_request("canonical_answer must not be empty"))?;
    let answer = u8::try_from(answer)
        .map_err(|_| invalid_request("canonical_answer must fit an unsigned byte"))?;
    let operation_counts = object
        .get("operation_counts")
        .cloned()
        .map(|value| {
            serde_json::from_value(value).map_err(|error| invalid_request(&error.to_string()))
        })
        .transpose()?
        .unwrap_or_else(|| drill_core::operation_counts_for(left, right));
    Ok(Problem {
        schema_version: SCHEMA_VERSION,
        id,
        left,
        right,
        answer,
        operation_counts,
    })
}

fn answer_from_digits(digits: &[u8]) -> Result<AnswerNode, ApiError> {
    if digits.is_empty() {
        return Ok(AnswerNode::Empty);
    }
    if digits.iter().any(|digit| *digit > 9) {
        return Err(invalid_request(
            "integer answer digits must be between 0 and 9",
        ));
    }
    let text: String = digits
        .iter()
        .map(|digit| char::from(b'0' + *digit))
        .collect();
    let value = text
        .parse::<i64>()
        .map_err(|_| invalid_request("integer answer is outside the supported range"))?;
    validate_answer_size(AnswerNode::Integer(value))
}

fn validate_answer_size(answer: AnswerNode) -> Result<AnswerNode, ApiError> {
    if answer.is_within_size_limit() {
        Ok(answer)
    } else {
        Err(invalid_request(&format!(
            "answer AST size must not exceed {}",
            drill_core::MAX_ANSWER_AST_SIZE
        )))
    }
}

fn deserialize_digits<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    digits_from_value(&value).map_err(|error| serde::de::Error::custom(error.message))
}

fn digits_from_value(value: &Value) -> Result<Vec<u8>, ApiError> {
    if let Some(text) = value.as_str() {
        if text.is_empty() {
            return Ok(Vec::new());
        }
        return text
            .bytes()
            .map(|byte| {
                byte.checked_sub(b'0')
                    .filter(|digit| *digit <= 9)
                    .ok_or_else(|| invalid_request("integer answer digits must be decimal"))
            })
            .collect();
    }
    let array = value
        .as_array()
        .ok_or_else(|| invalid_request("integer answer digits must be an array or string"))?;
    array
        .iter()
        .map(|digit| {
            digit
                .as_u64()
                .and_then(|digit| u8::try_from(digit).ok())
                .filter(|digit| *digit <= 9)
                .ok_or_else(|| invalid_request("integer answer digits must be between 0 and 9"))
        })
        .collect()
}

fn parse_json<T: DeserializeOwned>(input_json: &str) -> Result<T, ApiError> {
    serde_json::from_str(input_json).map_err(|error| invalid_request(&error.to_string()))
}

/// Seed-only strings are accepted as a convenience for small callers while
/// object DTOs remain the canonical, versioned boundary representation.
fn parse_seed_request<T>(input_json: &str) -> Result<T, ApiError>
where
    T: DeserializeOwned + FromSeedString,
{
    match serde_json::from_str(input_json) {
        Ok(request) => Ok(request),
        Err(object_error) => match serde_json::from_str::<String>(input_json) {
            Ok(seed) => Ok(T::from_seed(seed)),
            Err(_) => Err(invalid_request(&object_error.to_string())),
        },
    }
}

trait FromSeedString {
    fn from_seed(seed: String) -> Self;
}

impl FromSeedString for GenerateProblemRequest {
    fn from_seed(seed: String) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            seed,
        }
    }
}

impl FromSeedString for GenerateWorksheetRequest {
    fn from_seed(seed: String) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            seed,
            ..Self::default()
        }
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
    let code = error.code().to_owned();
    let details = match &error {
        GenerationError::Timeout { timeout_ms } => Some(json!({ "timeout_ms": timeout_ms })),
        GenerationError::AttemptLimit {
            attempts,
            max_attempts,
        } => Some(json!({ "attempts": attempts, "max_attempts": max_attempts })),
        GenerationError::InvalidProblemCount { requested } => {
            Some(json!({ "requested": requested }))
        }
    };
    ApiError {
        code,
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
    .unwrap_or_else(|_| "{\"schema_version\":1,\"ok\":false,\"data\":null,\"error\":{\"code\":\"serialization_error\",\"message\":\"serialization failed\"}}".to_owned())
}

fn error_response(error: ApiError) -> String {
    serde_json::to_string(&ApiResponse::<Value> {
        schema_version: SCHEMA_VERSION,
        ok: false,
        data: None,
        error: Some(error),
    })
    .unwrap_or_else(|_| "{\"schema_version\":1,\"ok\":false,\"data\":null,\"error\":{\"code\":\"serialization_error\",\"message\":\"serialization failed\"}}".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn worksheet_boundary_is_versioned_and_seed_is_a_string() {
        let output = generate_worksheet(r#"{"schema_version":1,"seed":"9007199254740993"}"#);
        let value: Value = serde_json::from_str(&output).unwrap();
        assert_eq!(value["schema_version"], 1);
        assert_eq!(value["ok"], true);
        assert_eq!(value["data"]["seed"], "9007199254740993");
        assert_eq!(value["data"]["layout"]["problem_count"], 20);
        assert_eq!(value["data"]["problems"][0]["prompt"]["kind"], "addition");
        assert_eq!(
            value["data"]["problems"][0]["answer_schema"]["kind"],
            "integer"
        );
    }

    #[test]
    fn generation_errors_have_stable_codes() {
        let timeout = generate_worksheet(
            r#"{"schema_version":1,"seed":"x","timeout_ms":0,"problem_count":20}"#,
        );
        let timeout: Value = serde_json::from_str(&timeout).unwrap();
        assert_eq!(timeout["error"]["code"], "generation_timeout");

        let attempts = generate_worksheet(
            r#"{"schema_version":1,"seed":"x","max_attempts":0,"problem_count":20,"timeout_ms":1000}"#,
        );
        let attempts: Value = serde_json::from_str(&attempts).unwrap();
        assert_eq!(attempts["error"]["code"], "generation_attempt_limit");
    }

    #[test]
    fn wasm_clock_latches_throwing_invalid_or_backward_reads_as_timeout() {
        let throwing = BrowserClockState::try_new(100.0).unwrap();
        let started = throwing.read(Some(101.0));
        let failed = throwing.read(None);
        assert_eq!(failed, Duration::MAX);
        assert!(failed.saturating_sub(started) >= drill_core::DEFAULT_TIMEOUT);
        assert_eq!(throwing.read(Some(102.0)), Duration::MAX);

        let invalid = BrowserClockState::try_new(100.0).unwrap();
        let started = invalid.read(Some(101.0));
        let failed = invalid.read(Some(f64::NAN));
        assert_eq!(failed, Duration::MAX);
        assert!(failed.saturating_sub(started) >= drill_core::DEFAULT_TIMEOUT);

        let backward = BrowserClockState::try_new(100.0).unwrap();
        let started = backward.read(Some(101.0));
        let failed = backward.read(Some(100.5));
        assert_eq!(failed, Duration::MAX);
        assert!(failed.saturating_sub(started) >= drill_core::DEFAULT_TIMEOUT);

        // If the first core read fails, it becomes the zero start instant and
        // the next latched read still guarantees a timeout.
        let first_read_failure = BrowserClockState::try_new(100.0).unwrap();
        assert_eq!(first_read_failure.read(None), Duration::ZERO);
        assert_eq!(first_read_failure.read(Some(101.0)), Duration::MAX);
    }

    #[test]
    fn editor_boundary_reports_answer_ast_size_limit() {
        let request = json!({
            "schema_version": 1,
            "state": {
                "schema_version": 1,
                "node": {
                    "kind": "integer",
                    "digits": vec![1_u8; drill_core::MAX_ANSWER_AST_SIZE]
                },
                "cursor": drill_core::MAX_ANSWER_AST_SIZE,
                "committed": false
            },
            "action": { "kind": "insert_digit", "digit": 2 }
        });
        let response: Value =
            serde_json::from_str(&apply_editor_action(&request.to_string())).unwrap();
        assert_eq!(response["ok"], false);
        assert_eq!(response["error"]["code"], "answer_ast_size_limit");
        assert_eq!(
            response["error"]["details"]["max_size"],
            drill_core::MAX_ANSWER_AST_SIZE
        );

        let oversized = json!({
            "schema_version": 1,
            "state": {
                "schema_version": 1,
                "node": {
                    "kind": "integer",
                    "digits": vec![1_u8; drill_core::MAX_ANSWER_AST_SIZE + 1]
                },
                "cursor": drill_core::MAX_ANSWER_AST_SIZE + 1,
                "committed": false
            },
            "action": { "kind": "clear" }
        });
        let response: Value =
            serde_json::from_str(&apply_editor_action(&oversized.to_string())).unwrap();
        assert_eq!(response["ok"], false);
        assert_eq!(response["error"]["code"], "invalid_request");
    }

    #[test]
    fn editor_grade_and_effort_boundaries_are_json_only() {
        let edited = apply_editor_action(
            r#"{"schema_version":1,"state":{"node":{"kind":"integer","digits":[]},"cursor":0,"committed":false},"action":{"kind":"insert_digit","digit":4}}"#,
        );
        let edited: Value = serde_json::from_str(&edited).unwrap();
        assert_eq!(
            edited["data"]["node"],
            json!({ "kind": "integer", "digits": [4] })
        );

        let graded = grade_answer(
            r#"{"schema_version":1,"expected":{"type":"integer","value":4},"actual":{"type":"integer","value":4}}"#,
        );
        let graded: Value = serde_json::from_str(&graded).unwrap();
        assert_eq!(graded["data"]["is_correct"], true);
        assert_eq!(
            graded["data"]["expected"],
            json!({ "kind": "integer", "value": 4 })
        );

        let normalized =
            normalize_answer(r#"{"schema_version":1,"answer":{"kind":"integer","digits":[0,4]}}"#);
        let normalized: Value = serde_json::from_str(&normalized).unwrap();
        assert_eq!(normalized["data"], json!({ "kind": "integer", "value": 4 }));
    }
}
