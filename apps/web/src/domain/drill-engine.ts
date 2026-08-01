/**
 * The public Web/WASM boundary mirrors the Rust schema-v2 JSON contract.
 * React-only fields (problem_id, left/right, and seed) are typed as
 * presentation projections on the values returned by the adapter; requests
 * sent to WASM contain only the Rust DTO fields.
 */

export const DRILL_SCHEMA_VERSION = 2 as const;
export const ADDITION_SKILL_ID = 'jp.grade1.addition.one_digit' as const;
export const ADDITION_GENERATOR_REVISION = 2 as const;
/** Kept as a presentation label for existing callers; never sent to Rust. */
export const ADDITION_GENERATOR_VERSION = 'addition-one-digit-v2' as const;

export type DifficultyLevel = 1 | 2 | 3 | 4 | 5;

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

/** Exact generate_worksheet request; registry owns layout and generator revision. */
export type DrillSettings = {
  schema_version: typeof DRILL_SCHEMA_VERSION;
  numeric_theme_id: number;
  seed: string;
  difficulty: DifficultyLevel;
};

export type ProblemSetIdentity = {
  schema_version: typeof DRILL_SCHEMA_VERSION;
  numeric_theme_id: number;
  generator_revision: number;
  seed: string;
  difficulty: DifficultyLevel;
};

/** Tagged Rust AnswerNode union. i64 payloads are canonical decimal strings. */
export type AnswerNode =
  | { type: 'empty' }
  | { type: 'integer'; value: string }
  | { type: 'exact_decimal'; value: { coefficient: string; scale: number } }
  | { type: 'fraction'; value: { numerator: AnswerNode; denominator: AnswerNode } }
  | { type: 'mixed_fraction'; value: { whole: AnswerNode; numerator: AnswerNode; denominator: AnswerNode } }
  | { type: 'root'; value: { radicand: AnswerNode; index: AnswerNode | null } }
  | { type: 'negative'; value: AnswerNode }
  | { type: 'plus_minus'; value: AnswerNode }
  | { type: 'tuple'; value: AnswerNode[] }
  | { type: 'variable'; value: string };

export type IntegerAnswerNode = Extract<AnswerNode, { type: 'integer' }>;

/** Rust EditorState (schema_version belongs to the request envelope). */
export type EditorState = {
  answer: AnswerNode;
  cursor: number;
  committed: boolean;
};

export type EditorAction =
  | { kind: 'insert_digit'; digit: number }
  | { kind: 'delete_backward' }
  | { kind: 'delete_forward' }
  | { kind: 'move_left' }
  | { kind: 'move_right' }
  | { kind: 'clear' }
  | { kind: 'commit' };

export type ProblemPrompt = {
  kind: 'addition';
  left: number;
  right: number;
};

export type AnswerSchema = {
  kind: 'integer';
  min: string;
  max: string;
};

export type OperationVector = {
  values: readonly number[];
};

export type SolutionStep = {
  id: number;
  operation: { kind: string; [key: string]: unknown };
  depends_on: readonly number[];
};

export type SolutionGraph = {
  steps: readonly SolutionStep[];
};

/** Rust Problem plus a typed addition presentation projection. */
export type ProblemDto = {
  schema_version: typeof DRILL_SCHEMA_VERSION;
  id: number;
  /** Stable UI key derived from the Rust numeric problem id. */
  problem_id: string;
  numeric_theme_id: number;
  prompt: ProblemPrompt;
  answer_schema: AnswerSchema;
  canonical_answer: AnswerNode;
  solution_graph: SolutionGraph;
  operation_vector: OperationVector;
  effort: number;
  /** Addition-only projection used by the existing worksheet renderer. */
  left: number;
  right: number;
};

/** Rust Worksheet plus the UI seed convenience projection. */
export type WorksheetDto = {
  schema_version: typeof DRILL_SCHEMA_VERSION;
  problem_set_id: string;
  identity: ProblemSetIdentity;
  skill_id: string;
  curriculum_path: readonly string[];
  layout: WorksheetLayout;
  problems: readonly ProblemDto[];
  seed: string;
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

export type GradeWarningCode =
  | 'fraction_not_reduced'
  | 'redundant_negative'
  | 'redundant_decimal';

export type GradeItem = {
  problem_id: string;
  answer: string | null;
  correct: boolean;
  warnings: readonly GradeWarningCode[];
};

export type GradeResult = {
  schema_version: typeof DRILL_SCHEMA_VERSION;
  items: readonly GradeItem[];
  correct_count: number;
  total_count: number;
};

export type DrillEngineErrorKind =
  | 'generation_timeout'
  | 'generation_attempt_limit'
  | 'answer_ast_size_limit'
  | 'wasm_unavailable'
  | 'invalid_dto';

export class DrillEngineError extends Error {
  readonly kind: DrillEngineErrorKind;
  readonly details?: unknown;

  constructor(kind: DrillEngineErrorKind, message: string, details?: unknown) {
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
  numeric_theme_id: 1,
  difficulty: 3,
  // q1 resolves a blank value to a fresh automatic seed per click.
  seed: '',
};

export function emptyEditorState(): EditorState {
  return {
    answer: { type: 'empty' },
    cursor: 0,
    committed: false,
  };
}

/** Presentation-only conversion; exact non-integer nodes are not coerced. */
export function editorDigits(state: EditorState): string {
  return state.answer.type === 'integer' ? String(state.answer.value) : '';
}

export function editorValue(state: EditorState): string | null {
  const digits = editorDigits(state);
  return digits.length > 0 ? digits : null;
}

export function integerAnswerValue(answer: AnswerNode): string | null {
  return answer.type === 'integer' ? answer.value : null;
}

export function formatCurriculumPath(path: readonly (CurriculumPathSegment | string)[]): string {
  return path.map((segment) => typeof segment === 'string' ? segment : segment.label).join(' > ');
}
