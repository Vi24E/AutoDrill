import {
  DRILL_SCHEMA_VERSION,
  DrillEngineError,
  type AnswerNode,
  type DrillEngine,
  type DrillSettings,
  type EditorAction,
  type EditorState,
  type GradeRequest,
  type GradeResult,
  type GradeWarningCode,
  type ProblemDto,
  type ProblemSetIdentity,
  type WorksheetDto,
} from './drill-engine';

/** Generated wasm-pack exports. Every call accepts the schema-v2 JSON string. */
export type DrillWasmRuntime = {
  generate_problem?: (request: string) => unknown | Promise<unknown>;
  generate_worksheet?: (request: string) => unknown | Promise<unknown>;
  regenerate_problem_set?: (request: string) => unknown | Promise<unknown>;
  apply_editor_action?: (request: string) => unknown | Promise<unknown>;
  normalize_answer?: (request: string) => unknown | Promise<unknown>;
  grade_answer?: (request: string) => unknown | Promise<unknown>;
  calculate_effort?: (request: string) => unknown | Promise<unknown>;
};

declare global {
  interface Window {
    /** Set by the generated drill-wasm package at application bootstrap. */
    __AUTODRILL_WASM__?: DrillWasmRuntime;
  }
}

type RecordValue = Record<string, unknown>;

function isRecord(value: unknown): value is RecordValue {
  return Boolean(value) && typeof value === 'object' && !Array.isArray(value);
}

function resolveRuntime(runtime?: DrillWasmRuntime): DrillWasmRuntime {
  if (runtime) return runtime;
  if (typeof window !== 'undefined' && window.__AUTODRILL_WASM__) {
    return window.__AUTODRILL_WASM__;
  }
  throw new DrillEngineError(
    'wasm_unavailable',
    'drill-wasm is not loaded. The generated WASM package must be attached before using the drill.',
  );
}

function mapBoundaryError(error: unknown): DrillEngineError {
  if (error instanceof DrillEngineError) return error;
  const candidate = isRecord(error) ? error : null;
  const nested = isRecord(candidate?.error) ? candidate.error : null;
  const kind = candidate?.kind ?? candidate?.code ?? nested?.kind ?? nested?.code;
  if (kind === 'generation_timeout' || kind === 'timeout') {
    return new DrillEngineError('generation_timeout', 'Problem generation exceeded its time budget.', error);
  }
  if (kind === 'generation_attempt_limit' || kind === 'attempt_limit') {
    return new DrillEngineError(
      'generation_attempt_limit',
      'Problem generation exceeded its maximum number of attempts.',
      error,
    );
  }
  if (kind === 'answer_ast_size_limit') {
    return new DrillEngineError(
      'answer_ast_size_limit',
      'The answer AST exceeded its maximum size.',
      error,
    );
  }
  if (error instanceof Error) return new DrillEngineError('invalid_dto', error.message, error);
  return new DrillEngineError('invalid_dto', 'The drill-wasm response was not valid.', error);
}

async function invokeBoundary(call: (request: string) => unknown | Promise<unknown>, request: unknown): Promise<unknown> {
  return call(JSON.stringify(request));
}

function decodeWasmValue(value: unknown): unknown {
  if (typeof value !== 'string') return value;
  try {
    return JSON.parse(value) as unknown;
  } catch (error) {
    throw new DrillEngineError('invalid_dto', 'WASM returned malformed JSON.', error);
  }
}

function invalidDto(message: string, value?: unknown): never {
  throw new DrillEngineError('invalid_dto', message, value);
}

function assertFiniteNumber(value: unknown, label: string): asserts value is number {
  if (typeof value !== 'number' || !Number.isFinite(value)) invalidDto(`WASM returned an invalid ${label}.`, value);
}

function assertInteger(value: unknown, label: string): asserts value is number {
  assertFiniteNumber(value, label);
  if (!Number.isInteger(value)) invalidDto(`WASM returned a non-integer ${label}.`, value);
}

function assertU32(value: unknown, label: string): asserts value is number {
  assertInteger(value, label);
  if (value < 0 || value > 0xffff_ffff) invalidDto(`WASM returned an out-of-range ${label}.`, value);
}

function assertCanonicalI64String(value: unknown, label: string): asserts value is string {
  if (typeof value !== 'string' || !/^(?:0|-?[1-9]\d*)$/.test(value)) {
    invalidDto(`WASM returned an invalid ${label}.`, value);
  }
  try {
    const parsed = BigInt(value);
    if (parsed < -(1n << 63n) || parsed > (1n << 63n) - 1n) {
      invalidDto(`WASM returned an out-of-range ${label}.`, value);
    }
  } catch {
    invalidDto(`WASM returned an invalid ${label}.`, value);
  }
}

function assertCanonicalU64String(value: unknown, label: string): asserts value is string {
  if (typeof value !== 'string' || !/^(?:0|[1-9]\d*)$/.test(value)) {
    invalidDto(`WASM returned an invalid ${label}.`, value);
  }
  try {
    const parsed = BigInt(value);
    if (parsed > (1n << 64n) - 1n) invalidDto(`WASM returned an out-of-range ${label}.`, value);
  } catch {
    invalidDto(`WASM returned an invalid ${label}.`, value);
  }
}

function assertAnswerNode(value: unknown): asserts value is AnswerNode {
  if (!isRecord(value) || typeof value.type !== 'string') invalidDto('WASM returned an invalid tagged AnswerNode.', value);
  switch (value.type) {
    case 'empty':
      return;
    case 'integer':
      assertCanonicalI64String(value.value, 'integer answer');
      return;
    case 'exact_decimal':
      if (!isRecord(value.value)) invalidDto('WASM returned an invalid exact-decimal value.', value);
      assertCanonicalI64String(value.value.coefficient, 'exact-decimal coefficient');
      assertU32(value.value.scale, 'exact-decimal scale');
      return;
    case 'fraction':
      if (!isRecord(value.value)) invalidDto('WASM returned an invalid fraction value.', value);
      assertAnswerNode(value.value.numerator);
      assertAnswerNode(value.value.denominator);
      return;
    case 'mixed_fraction':
      if (!isRecord(value.value)) invalidDto('WASM returned an invalid mixed-fraction value.', value);
      assertAnswerNode(value.value.whole);
      assertAnswerNode(value.value.numerator);
      assertAnswerNode(value.value.denominator);
      return;
    case 'root':
      if (!isRecord(value.value)) invalidDto('WASM returned an invalid root value.', value);
      assertAnswerNode(value.value.radicand);
      if (value.value.index !== null) assertAnswerNode(value.value.index);
      return;
    case 'negative':
    case 'plus_minus':
      assertAnswerNode(value.value);
      return;
    case 'tuple':
      if (!Array.isArray(value.value)) invalidDto('WASM returned an invalid tuple value.', value);
      value.value.forEach(assertAnswerNode);
      return;
    case 'variable':
      if (typeof value.value !== 'string') invalidDto('WASM returned an invalid variable value.', value);
      return;
    default:
      invalidDto(`WASM returned an unsupported AnswerNode type: ${value.type}.`, value);
  }
}

function unwrapEnvelope(value: unknown): unknown {
  const decoded = decodeWasmValue(value);
  if (!isRecord(decoded) || typeof decoded.ok !== 'boolean') {
    invalidDto('WASM response did not contain the schema-v2 envelope.', value);
  }
  if (decoded.schema_version !== DRILL_SCHEMA_VERSION) {
    invalidDto('WASM response used an unsupported schema version.', value);
  }
  if (!decoded.ok) throw mapBoundaryError(decoded.error ?? decoded);
  return decoded.data;
}

function assertIdentity(value: unknown): asserts value is ProblemSetIdentity {
  if (!isRecord(value)) invalidDto('WASM returned an empty problem-set identity.', value);
  if (value.schema_version !== DRILL_SCHEMA_VERSION) invalidDto('WASM returned an unsupported identity schema.', value);
  assertInteger(value.numeric_theme_id, 'identity numeric_theme_id');
  assertInteger(value.generator_revision, 'identity generator_revision');
  if (typeof value.seed !== 'string') invalidDto('WASM returned an invalid identity seed.', value);
  assertInteger(value.difficulty, 'identity difficulty');
  if (value.difficulty < 1 || value.difficulty > 5) invalidDto('WASM returned an invalid identity difficulty.', value);
}

function assertWorksheet(value: unknown): WorksheetDto {
  const unwrapped = unwrapEnvelope(value);
  if (!isRecord(unwrapped)) invalidDto('WASM returned an empty worksheet DTO.', value);
  if (unwrapped.schema_version !== DRILL_SCHEMA_VERSION || typeof unwrapped.problem_set_id !== 'string') {
    invalidDto('WASM returned a worksheet with an unsupported schema.', value);
  }
  assertIdentity(unwrapped.identity);
  const identity = unwrapped.identity;
  if (identity.numeric_theme_id !== 1 || identity.generator_revision !== 2) {
    invalidDto('WASM returned an unregistered schema-v2 theme identity.', identity);
  }
  const expectedProblemSetId = `${DRILL_SCHEMA_VERSION}-${identity.numeric_theme_id}-${identity.generator_revision}-${identity.seed}-${identity.difficulty}`;
  if (unwrapped.problem_set_id !== expectedProblemSetId) {
    invalidDto('WASM returned a problem-set ID inconsistent with its identity.', unwrapped.problem_set_id);
  }
  if (typeof unwrapped.skill_id !== 'string' || !Array.isArray(unwrapped.curriculum_path) || !unwrapped.curriculum_path.every((item) => typeof item === 'string')) {
    invalidDto('WASM returned an invalid worksheet curriculum projection.', value);
  }
  if (!isRecord(unwrapped.layout) || unwrapped.layout.problem_count !== 20 || unwrapped.layout.columns !== 2 || unwrapped.layout.rows !== 10) {
    invalidDto('WASM returned an unsupported worksheet layout.', value);
  }
  if (!Array.isArray(unwrapped.problems) || unwrapped.problems.length !== 20) {
    invalidDto('WASM returned a worksheet without exactly 20 problems.', value);
  }
  const ids = new Set<number>();
  const problems = unwrapped.problems.map((problem, index) => {
    if (!isRecord(problem)) invalidDto(`WASM returned an invalid problem at index ${index}.`, problem);
    if (problem.schema_version !== DRILL_SCHEMA_VERSION) invalidDto('WASM returned a problem with an unsupported schema.', problem);
    assertInteger(problem.id, 'problem id');
    if (problem.id < 1 || problem.id > 20) invalidDto('WASM returned a problem id outside the registered layout.', problem.id);
    if (ids.has(problem.id)) invalidDto('WASM returned duplicate problem ids.', problem);
    ids.add(problem.id);
    assertInteger(problem.numeric_theme_id, 'problem numeric_theme_id');
    if (problem.numeric_theme_id !== identity.numeric_theme_id) {
      invalidDto('WASM returned a problem for a different numeric theme.', problem);
    }
    if (!isRecord(problem.prompt) || problem.prompt.kind !== 'addition') invalidDto('WASM returned an unsupported prompt variant.', problem);
    assertInteger(problem.prompt.left, 'addition left operand');
    assertInteger(problem.prompt.right, 'addition right operand');
    if (!isRecord(problem.answer_schema) || problem.answer_schema.kind !== 'integer') invalidDto('WASM returned an unsupported answer schema.', problem);
    assertCanonicalI64String(problem.answer_schema.min, 'answer-schema minimum');
    assertCanonicalI64String(problem.answer_schema.max, 'answer-schema maximum');
    assertAnswerNode(problem.canonical_answer);
    if (!isRecord(problem.solution_graph) || !Array.isArray(problem.solution_graph.steps)) invalidDto('WASM returned an invalid solution graph.', problem);
    problem.solution_graph.steps.forEach((step) => {
      if (!isRecord(step) || !('id' in step) || !('operation' in step) || !('depends_on' in step)) {
        invalidDto('WASM returned an invalid solution step.', step);
      }
      assertInteger(step.id, 'solution-step id');
      if (!isRecord(step.operation) || typeof step.operation.kind !== 'string') invalidDto('WASM returned an unknown solution operation.', step);
      if (step.operation.kind === 'big_num') {
        assertCanonicalU64String(step.operation.magnitude, 'BigNum magnitude');
      }
      if (!Array.isArray(step.depends_on)) invalidDto('WASM returned invalid solution dependencies.', step);
      step.depends_on.forEach((dependency) => assertInteger(dependency, 'solution dependency'));
    });
    if (!isRecord(problem.operation_vector) || !Array.isArray(problem.operation_vector.values) || problem.operation_vector.values.length !== 27) invalidDto('WASM returned an invalid operation vector.', problem);
    problem.operation_vector.values.forEach((item) => assertFiniteNumber(item, 'operation-vector value'));
    assertFiniteNumber(problem.effort, 'problem effort');
    return {
      ...problem,
      problem_id: String(problem.id),
      left: problem.prompt.left,
      right: problem.prompt.right,
    } as ProblemDto;
  });
  return {
    ...unwrapped,
    identity: unwrapped.identity,
    problems,
    seed: identity.seed,
  } as unknown as WorksheetDto;
}

function assertEditorState(value: unknown): EditorState {
  const unwrapped = unwrapEnvelope(value);
  if (!isRecord(unwrapped)) invalidDto('WASM returned an empty editor state.', value);
  assertAnswerNode(unwrapped.answer);
  assertInteger(unwrapped.cursor, 'editor cursor');
  if (typeof unwrapped.committed !== 'boolean') invalidDto('WASM returned an invalid editor committed flag.', value);
  return unwrapped as EditorState;
}

function mapEditorAction(action: EditorAction): RecordValue {
  switch (action.kind) {
    case 'insert_digit': return { type: 'insert_digit', digit: action.digit };
    case 'delete_backward': return { type: 'backspace' };
    case 'delete_forward': return { type: 'delete' };
    case 'move_left': return { type: 'move_left' };
    case 'move_right': return { type: 'move_right' };
    case 'clear': return { type: 'clear' };
    case 'commit': return { type: 'commit' };
  }
}

function integerOrNull(value: AnswerNode): string | null {
  return value.type === 'integer' ? value.value : null;
}

const GRADE_WARNING_CODES: readonly GradeWarningCode[] = [
  'fraction_not_reduced',
  'redundant_negative',
  'redundant_decimal',
];

function hasCanonicalGradeWarnings(value: unknown): value is GradeWarningCode[] {
  if (!Array.isArray(value)) return false;
  let previousIndex = -1;
  for (const warning of value) {
    if (typeof warning !== 'string') return false;
    const index = GRADE_WARNING_CODES.indexOf(warning as GradeWarningCode);
    if (index <= previousIndex) return false;
    previousIndex = index;
  }
  return true;
}

function gradeItemFromWasm(problemId: string, value: unknown): { problem_id: string; answer: string | null; correct: boolean; warnings: readonly GradeWarningCode[] } {
  const data = unwrapEnvelope(value);
  if (!isRecord(data) || typeof data.is_correct !== 'boolean') invalidDto('WASM returned an invalid grade DTO.', value);
  assertAnswerNode(data.expected);
  assertAnswerNode(data.actual);
  if (!hasCanonicalGradeWarnings(data.warnings)) {
    invalidDto('WASM returned invalid grade warnings.', data.warnings);
  }
  if (!data.is_correct && data.warnings.length > 0) {
    invalidDto('WASM returned grade warnings for an incorrect answer.', data.warnings);
  }
  return {
    problem_id: problemId,
    answer: integerOrNull(data.actual),
    correct: data.is_correct,
    warnings: data.warnings,
  };
}

export function createWasmDrillEngine(runtime?: DrillWasmRuntime): DrillEngine {
  return {
    async generateWorksheet(settings) {
      try {
        const generate = resolveRuntime(runtime).generate_worksheet;
        if (!generate) throw new DrillEngineError('wasm_unavailable', 'drill-wasm does not expose generate_worksheet.');
        return assertWorksheet(await invokeBoundary(generate, settings));
      } catch (error) {
        throw mapBoundaryError(error);
      }
    },

    async applyEditorAction(state, action) {
      try {
        const apply = resolveRuntime(runtime).apply_editor_action;
        if (!apply) throw new DrillEngineError('wasm_unavailable', 'drill-wasm does not expose apply_editor_action.');
        return assertEditorState(await invokeBoundary(apply, {
          schema_version: DRILL_SCHEMA_VERSION,
          state,
          action: mapEditorAction(action),
        }));
      } catch (error) {
        throw mapBoundaryError(error);
      }
    },

    async gradeAnswer(request: GradeRequest) {
      try {
        const gradeAnswer = resolveRuntime(runtime).grade_answer;
        if (!gradeAnswer) throw new DrillEngineError('wasm_unavailable', 'drill-wasm does not expose grade_answer.');
        const items = await Promise.all(request.worksheet.problems.map(async (problem) => {
          const editorState = request.answers.find((entry) => entry.problem_id === problem.problem_id)?.editor_state;
          const value = await invokeBoundary(gradeAnswer, {
            schema_version: DRILL_SCHEMA_VERSION,
            expected: problem.canonical_answer,
            actual: editorState?.answer ?? { type: 'empty' },
          });
          return gradeItemFromWasm(problem.problem_id, value);
        }));
        return {
          schema_version: DRILL_SCHEMA_VERSION,
          items,
          correct_count: items.filter((item) => item.correct).length,
          total_count: items.length,
        } satisfies GradeResult;
      } catch (error) {
        throw mapBoundaryError(error);
      }
    },
  };
}
