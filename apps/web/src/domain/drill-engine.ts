/**
 * The only public boundary between the React client and drill-wasm.
 *
 * The client owns presentation and interaction state, while generation,
 * normalization, effort calculation, and grading remain in Rust/WASM.
 */

export const DRILL_SCHEMA_VERSION = 1 as const;
export const ADDITION_SKILL_ID = 'jp.grade1.addition.one_digit.1' as const;
export const ADDITION_GENERATOR_VERSION = 'addition-one-digit-v1' as const;

export type CurriculumPathSegment = {
  id: string;
  label: string;
};

export const ADDITION_CURRICULUM_PATH: readonly CurriculumPathSegment[] = [
  { id: 'root', label: 'root' },
  { id: 'jp-grade-1', label: '小学1年生' },
  { id: ADDITION_SKILL_ID, label: '1けたのたしざん(1)' },
];

export type WorksheetLayout = {
  problem_count: 20;
  columns: 2;
  rows: 10;
};

export const ADDITION_LAYOUT: WorksheetLayout = {
  problem_count: 20,
  columns: 2,
  rows: 10,
};

export type DrillSettings = {
  schema_version: typeof DRILL_SCHEMA_VERSION;
  skill_id: typeof ADDITION_SKILL_ID;
  curriculum_path: readonly CurriculumPathSegment[];
  generator_version: typeof ADDITION_GENERATOR_VERSION;
  layout: WorksheetLayout;
  seed: string;
};

export type IntegerAnswerNode = {
  kind: 'integer';
  digits: readonly number[];
};

export type EditorState = {
  schema_version: typeof DRILL_SCHEMA_VERSION;
  node: IntegerAnswerNode;
  cursor: number;
  committed: boolean;
};

export type ProblemDto = {
  schema_version: typeof DRILL_SCHEMA_VERSION;
  problem_id: string;
  skill_id: typeof ADDITION_SKILL_ID;
  left: number;
  right: number;
  prompt: {
    kind: 'addition';
    left: number;
    right: number;
  };
  answer_schema: {
    kind: 'integer';
    min: 1;
    max: 18;
  };
  canonical_answer: {
    kind: 'integer';
    value: number;
  };
  operation_counts: {
    additions: number;
    carries: number;
  };
};

export type WorksheetDto = {
  schema_version: typeof DRILL_SCHEMA_VERSION;
  generator_version: typeof ADDITION_GENERATOR_VERSION;
  skill_id: typeof ADDITION_SKILL_ID;
  curriculum_path: readonly CurriculumPathSegment[];
  seed: string;
  layout: WorksheetLayout;
  problems: readonly ProblemDto[];
};

export type AnswerEntry = {
  problem_id: string;
  editor_state: EditorState;
};

export type GradeRequest = {
  schema_version: typeof DRILL_SCHEMA_VERSION;
  worksheet: WorksheetDto;
  answers: readonly AnswerEntry[];
};

export type GradeItem = {
  problem_id: string;
  answer: number | null;
  correct: boolean;
};

export type GradeResult = {
  schema_version: typeof DRILL_SCHEMA_VERSION;
  items: readonly GradeItem[];
  correct_count: number;
  total_count: number;
};

export type EditorAction =
  | { kind: 'insert_digit'; digit: number }
  | { kind: 'delete_backward' }
  | { kind: 'delete_forward' }
  | { kind: 'move_left' }
  | { kind: 'move_right' }
  | { kind: 'clear' }
  | { kind: 'commit' };

export type GenerationErrorKind =
  | 'generation_timeout'
  | 'generation_attempt_limit'
  | 'wasm_unavailable'
  | 'invalid_dto';

export class DrillEngineError extends Error {
  readonly kind: GenerationErrorKind;
  readonly details?: unknown;

  constructor(kind: GenerationErrorKind, message: string, details?: unknown) {
    super(message);
    this.name = 'DrillEngineError';
    this.kind = kind;
    this.details = details;
  }
}

export interface DrillEngine {
  generateWorksheet(settings: DrillSettings): Promise<WorksheetDto>;
  applyEditorAction(state: EditorState, action: EditorAction): Promise<EditorState>;
  gradeAnswer(request: GradeRequest): Promise<GradeResult>;
}

export const DEFAULT_ADDITION_SETTINGS: DrillSettings = {
  schema_version: DRILL_SCHEMA_VERSION,
  skill_id: ADDITION_SKILL_ID,
  curriculum_path: ADDITION_CURRICULUM_PATH,
  generator_version: ADDITION_GENERATOR_VERSION,
  layout: ADDITION_LAYOUT,
  // q1 resolves a blank value to a fresh automatic seed per click. Keeping
  // the default blank makes that policy visible instead of silently reusing a
  // fixed worksheet on every generation.
  seed: '',
};

export function emptyEditorState(): EditorState {
  return {
    schema_version: DRILL_SCHEMA_VERSION,
    node: { kind: 'integer', digits: [] },
    cursor: 0,
    committed: false,
  };
}

/** Presentation-only conversion; parsing/normalization remains in WASM. */
export function editorDigits(state: EditorState): string {
  return state.node.digits.join('');
}

/** The selected digits are only used for restrained rendering. */
export function editorValue(state: EditorState): string | null {
  const digits = editorDigits(state);
  return digits.length > 0 ? digits : null;
}

export function formatCurriculumPath(path: readonly CurriculumPathSegment[]): string {
  return path.map((segment) => segment.label).join(' > ');
}
