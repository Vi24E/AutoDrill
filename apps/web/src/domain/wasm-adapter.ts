import {
  DrillEngineError,
  type DrillEngine,
  type DrillSettings,
  type EditorAction,
  type EditorState,
  type GradeRequest,
  type GradeResult,
  type WorksheetDto,
} from './drill-engine';

/**
 * Minimal generated-module surface. The real package is supplied by the Rust
 * commander; this app intentionally has no TypeScript fallback implementation
 * of generation or grading.
 */
export type DrillWasmRuntime = {
  generate_problem?: (request: unknown) => unknown | Promise<unknown>;
  generate_worksheet?: (request: unknown) => unknown | Promise<unknown>;
  apply_editor_action?: (request: unknown) => unknown | Promise<unknown>;
  normalize_answer?: (request: unknown) => unknown | Promise<unknown>;
  grade_answer?: (request: unknown) => unknown | Promise<unknown>;
  calculate_effort?: (request: unknown) => unknown | Promise<unknown>;
};

declare global {
  interface Window {
    /** Set by the generated drill-wasm package at application bootstrap. */
    __AUTODRILL_WASM__?: DrillWasmRuntime;
  }
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
  const candidate = error as { kind?: unknown; code?: unknown; message?: unknown; error?: unknown } | null;
  const nested = candidate?.error as { kind?: unknown; code?: unknown } | undefined;
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

async function invokeBoundary(call: (request: unknown) => unknown | Promise<unknown>, request: unknown): Promise<unknown> {
  const payload = JSON.stringify(request);
  try {
    // wasm-bindgen exports in drill-wasm accept JSON strings. Keeping the
    // object retry makes the seam friendly to a thin test/runtime wrapper.
    return await call(payload);
  } catch (stringError) {
    try {
      return await call(request);
    } catch {
      throw stringError;
    }
  }
}

function decodeWasmValue(value: unknown): unknown {
  if (typeof value !== 'string') return value;
  try {
    return JSON.parse(value) as unknown;
  } catch (error) {
    throw new DrillEngineError('invalid_dto', 'WASM returned malformed JSON.', error);
  }
}

function assertWorksheet(value: unknown): WorksheetDto {
  const unwrapped = unwrapEnvelope(decodeWasmValue(value));
  if (!unwrapped || typeof unwrapped !== 'object') {
    throw new DrillEngineError('invalid_dto', 'WASM returned an empty worksheet DTO.', value);
  }
  const worksheet = unwrapped as Partial<WorksheetDto>;
  if (
    worksheet.schema_version !== 1 ||
    worksheet.skill_id !== 'jp.grade1.addition.one_digit.1' ||
    !Array.isArray(worksheet.problems) ||
    worksheet.problems.length !== 20
  ) {
    throw new DrillEngineError('invalid_dto', 'WASM returned a worksheet with an unsupported schema.', value);
  }
  return worksheet as WorksheetDto;
}

function assertEditorState(value: unknown): EditorState {
  const unwrapped = unwrapEnvelope(decodeWasmValue(value));
  if (!unwrapped || typeof unwrapped !== 'object') {
    throw new DrillEngineError('invalid_dto', 'WASM returned an empty editor state.', value);
  }
  const state = unwrapped as Partial<EditorState>;
  if (state.schema_version !== 1 || !state.node || state.node.kind !== 'integer') {
    throw new DrillEngineError('invalid_dto', 'WASM returned an unsupported editor state.', value);
  }
  return state as EditorState;
}

function unwrapEnvelope(value: unknown): unknown {
  value = decodeWasmValue(value);
  if (!value || typeof value !== 'object') return value;
  const candidate = value as { schema_version?: unknown; ok?: unknown; data?: unknown; error?: unknown };
  if (typeof candidate.ok !== 'boolean') return value;
  if (!candidate.ok) throw mapBoundaryError(candidate.error ?? value);
  return candidate.data;
}

function gradeItemFromWasm(problemId: string, value: unknown) {
  const data = unwrapEnvelope(value);
  if (!data || typeof data !== 'object') {
    throw new DrillEngineError('invalid_dto', 'WASM returned an empty grade DTO.', value);
  }
  const item = data as {
    is_correct?: unknown;
    correct?: unknown;
    actual?: unknown;
    answer?: unknown;
    submitted?: unknown;
  };
  const correct = typeof item.is_correct === 'boolean' ? item.is_correct : item.correct;
  if (typeof correct !== 'boolean') {
    throw new DrillEngineError('invalid_dto', 'WASM returned a grade without a correctness flag.', value);
  }
  const answerValue = item.actual ?? item.answer ?? item.submitted;
  const answer = typeof answerValue === 'number'
    ? answerValue
    : answerValue && typeof answerValue === 'object' && typeof (answerValue as { value?: unknown }).value === 'number'
      ? (answerValue as { value: number }).value
      : null;
  return { problem_id: problemId, answer, correct };
}

export function createWasmDrillEngine(runtime?: DrillWasmRuntime): DrillEngine {
  return {
    async generateWorksheet(settings) {
      try {
        const resolved = resolveRuntime(runtime);
        // A worksheet is a distinct Rust DTO. `generate_problem` is retained
        // as a required single-problem export, but cannot stand in for this
        // twenty-problem operation.
        const generate = resolved.generate_worksheet;
        if (!generate) {
          throw new DrillEngineError('wasm_unavailable', 'drill-wasm does not expose generate_worksheet.');
        }
        return assertWorksheet(await invokeBoundary(generate, settings));
      } catch (error) {
        throw mapBoundaryError(error);
      }
    },

    async applyEditorAction(state, action) {
      try {
        const resolved = resolveRuntime(runtime);
        if (!resolved.apply_editor_action) {
          throw new DrillEngineError('wasm_unavailable', 'drill-wasm does not expose apply_editor_action.');
        }
        return assertEditorState(await invokeBoundary(resolved.apply_editor_action, { schema_version: 1, state, action }));
      } catch (error) {
        throw mapBoundaryError(error);
      }
    },

    async gradeAnswer(request) {
      try {
        const resolved = resolveRuntime(runtime);
        const gradeAnswer = resolved.grade_answer;
        if (!gradeAnswer) {
          throw new DrillEngineError('wasm_unavailable', 'drill-wasm does not expose grade_answer.');
        }
        // The adapter only sequences one Rust grade_answer call per problem.
        // It never computes correctness or normalizes the editor state.
        const items = await Promise.all(request.worksheet.problems.map(async (problem) => {
          const editorState = request.answers.find((entry) => entry.problem_id === problem.problem_id)?.editor_state;
          const value = await invokeBoundary(gradeAnswer, {
            schema_version: 1,
            expected: problem.canonical_answer,
            actual: (editorState ?? { schema_version: 1, node: { kind: 'integer', digits: [] }, cursor: 0, committed: false }).node,
          });
          return gradeItemFromWasm(problem.problem_id, value);
        }));
        return {
          schema_version: 1,
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
