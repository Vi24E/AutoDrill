import { DRILL_CORE_CONTRACT, type DrillCoreGradeWarningCode } from '@/generated/drill-core-contract';

/**
 * The public Web/WASM boundary mirrors the generated Rust compatibility contract.
 * React-only fields remain Web-owned; cross-language identity/layout/warning values
 * are never duplicated by hand in TypeScript.
 */
const ADDITION_CORE_CONTRACT = DRILL_CORE_CONTRACT.themes['1'];
const LINEAR_EQUATION_1_CORE_CONTRACT = DRILL_CORE_CONTRACT.themes['2'];
const LINEAR_EQUATION_2_CORE_CONTRACT = DRILL_CORE_CONTRACT.themes['3'];

export const DRILL_SCHEMA_VERSION = DRILL_CORE_CONTRACT.schema_version;
export const ADDITION_THEME_ID = ADDITION_CORE_CONTRACT.numeric_theme_id;
export const LINEAR_EQUATION_1_THEME_ID = LINEAR_EQUATION_1_CORE_CONTRACT.numeric_theme_id;
export const LINEAR_EQUATION_2_THEME_ID = LINEAR_EQUATION_2_CORE_CONTRACT.numeric_theme_id;
export const ADDITION_SKILL_ID = ADDITION_CORE_CONTRACT.skill_id;
export const ADDITION_GENERATOR_REVISION = ADDITION_CORE_CONTRACT.generator_revision;
export const LINEAR_EQUATION_1_SKILL_ID = LINEAR_EQUATION_1_CORE_CONTRACT.skill_id;
export const LINEAR_EQUATION_2_SKILL_ID = LINEAR_EQUATION_2_CORE_CONTRACT.skill_id;
export const LINEAR_EQUATION_1_GENERATOR_REVISION = LINEAR_EQUATION_1_CORE_CONTRACT.generator_revision;
export const LINEAR_EQUATION_2_GENERATOR_REVISION = LINEAR_EQUATION_2_CORE_CONTRACT.generator_revision;
/** Kept as a presentation label for existing callers; never sent to Rust. */
export const ADDITION_GENERATOR_VERSION = 'addition-one-digit-r2' as const;

export type DifficultyLevel = 1 | 2 | 3 | 4 | 5;

export type CurriculumPathSegment = {
  id: string;
  label: string;
};

export const ADDITION_CURRICULUM_PATH: readonly CurriculumPathSegment[] = [
  { id: 'root', label: ADDITION_CORE_CONTRACT.curriculum_path[0] },
  { id: 'jp-grade-1', label: ADDITION_CORE_CONTRACT.curriculum_path[1] },
  { id: ADDITION_SKILL_ID, label: ADDITION_CORE_CONTRACT.curriculum_path[2] },
];

export const LINEAR_EQUATION_1_CURRICULUM_PATH: readonly CurriculumPathSegment[] = [
  { id: 'root', label: LINEAR_EQUATION_1_CORE_CONTRACT.curriculum_path[0] },
  { id: 'jp-grade-7', label: LINEAR_EQUATION_1_CORE_CONTRACT.curriculum_path[1] },
  { id: 'jp-grade-7-linear-equation', label: LINEAR_EQUATION_1_CORE_CONTRACT.curriculum_path[2] },
  { id: LINEAR_EQUATION_1_SKILL_ID, label: LINEAR_EQUATION_1_CORE_CONTRACT.curriculum_path[3] },
];

export const LINEAR_EQUATION_2_CURRICULUM_PATH: readonly CurriculumPathSegment[] = [
  { id: 'root', label: LINEAR_EQUATION_2_CORE_CONTRACT.curriculum_path[0] },
  { id: 'jp-grade-7', label: LINEAR_EQUATION_2_CORE_CONTRACT.curriculum_path[1] },
  { id: 'jp-grade-7-linear-equation', label: LINEAR_EQUATION_2_CORE_CONTRACT.curriculum_path[2] },
  { id: LINEAR_EQUATION_2_SKILL_ID, label: LINEAR_EQUATION_2_CORE_CONTRACT.curriculum_path[3] },
];

export type WorksheetLayout = {
  problem_count: number;
  columns: number;
  rows: number;
};

export const ADDITION_LAYOUT: WorksheetLayout = { ...ADDITION_CORE_CONTRACT.layout };
export const LINEAR_EQUATION_1_LAYOUT: WorksheetLayout = { ...LINEAR_EQUATION_1_CORE_CONTRACT.layout };
export const LINEAR_EQUATION_2_LAYOUT: WorksheetLayout = { ...LINEAR_EQUATION_2_CORE_CONTRACT.layout };

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
  /** Raw text that the core could not parse as a numeric answer. */
  | { type: 'nan_error'; value: string }
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
  active_path: readonly number[];
  committed: boolean;
};

export type AnswerInputStructure =
  | 'fraction'
  | 'mixed_fraction'
  | 'decimal'
  | 'root'
  | 'negative'
  | 'plus_minus'
  | 'tuple';

export type AnswerInputInterface =
  | { type: 'simple_numeric'; allow_decimal: boolean; allow_negative: boolean }
  | { type: 'structured_math'; allowed_structures: readonly AnswerInputStructure[] };

export type InputCapabilities = {
  allow_decimal: boolean;
  allow_negative: boolean;
  allowed_structures: readonly AnswerInputStructure[];
};

export function inputCapabilities(inputInterface: AnswerInputInterface): InputCapabilities {
  if (inputInterface.type === 'simple_numeric') {
    return {
      allow_decimal: inputInterface.allow_decimal,
      allow_negative: inputInterface.allow_negative,
      allowed_structures: [],
    };
  }
  return {
    allow_decimal: inputInterface.allowed_structures.includes('decimal'),
    allow_negative: inputInterface.allowed_structures.includes('negative'),
    allowed_structures: inputInterface.allowed_structures,
  };
}

export function isEditorActionAllowed(inputInterface: AnswerInputInterface, action: EditorAction): boolean {
  if (action.kind !== 'insert_structure') return true;
  const capabilities = inputCapabilities(inputInterface);
  return capabilities.allowed_structures.includes(action.structure)
    || (action.structure === 'decimal' && capabilities.allow_decimal)
    || (action.structure === 'negative' && capabilities.allow_negative);
}

export type EditorAction =
  | { kind: 'insert_digit'; digit: number }
  | { kind: 'delete_backward' }
  | { kind: 'delete_forward' }
  | { kind: 'move_left' }
  | { kind: 'move_right' }
  | { kind: 'insert_structure'; structure: AnswerInputStructure }
  | { kind: 'select_slot'; path: readonly number[]; cursor: number }
  | { kind: 'clear' }
  | { kind: 'commit' };

export type RationalCoefficient = {
  numerator: number;
  denominator: number;
};

export type ProblemPrompt =
  | {
      kind: 'addition';
      left: number;
      right: number;
    }
  | {
      kind: 'linear_equation';
      a: RationalCoefficient;
      b: RationalCoefficient;
      c: RationalCoefficient;
      d: RationalCoefficient;
      left_negative_constant_as_subtraction: boolean;
      right_negative_constant_as_subtraction: boolean;
    };

export type AnswerSchema =
  | {
      kind: 'integer';
      min: string;
      max: string;
    }
  | {
      kind: 'rational';
      max_abs_numerator: number;
      max_denominator: number;
      require_reduced_fraction_form: boolean;
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
  input_interface: AnswerInputInterface;
  answer_schema: AnswerSchema;
  canonical_answer: AnswerNode;
  solution_graph: SolutionGraph;
  operation_vector: OperationVector;
  effort: number;
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

export type GradeWarningCode = DrillCoreGradeWarningCode;

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
  applyEditorAction(state: EditorState, action: EditorAction, inputInterface: AnswerInputInterface): Promise<EditorState>;
  gradeAnswer(request: GradeRequest): Promise<GradeResult>;
}

export const DEFAULT_DRILL_SETTINGS: DrillSettings = {
  schema_version: DRILL_SCHEMA_VERSION,
  // The Web curriculum registry replaces this with the selected theme ID.
  numeric_theme_id: 2,
  difficulty: 3,
  // q1 resolves a blank value to a fresh automatic seed per click.
  seed: '',
};

export function emptyEditorState(): EditorState {
  return {
    answer: { type: 'empty' },
    cursor: 0,
    active_path: [],
    committed: false,
  };
}

function exactDecimalText(coefficient: string, scale: number): string {
  const negative = coefficient.startsWith('-');
  const digits = negative ? coefficient.slice(1) : coefficient;
  const sign = negative ? '-' : '';
  if (scale === 0) return `${sign}${digits}.`;
  if (digits.length <= scale) return `${sign}0.${'0'.repeat(scale - digits.length)}${digits}`;
  const split = digits.length - scale;
  return `${sign}${digits.slice(0, split)}.${digits.slice(split)}`;
}

/** Plain-text projection for labels, grading summaries, and fallback output. */
export function answerNodeText(answer: AnswerNode): string {
  switch (answer.type) {
    case 'empty': return '';
    case 'integer': return answer.value;
    case 'exact_decimal': return exactDecimalText(answer.value.coefficient, answer.value.scale);
    case 'nan_error': return answer.value;
    case 'fraction': return `${answerNodeText(answer.value.numerator)}/${answerNodeText(answer.value.denominator)}`;
    case 'mixed_fraction': return `${answerNodeText(answer.value.whole)} ${answerNodeText(answer.value.numerator)}/${answerNodeText(answer.value.denominator)}`;
    case 'root': return `${answer.value.index ? answerNodeText(answer.value.index) : ''}√${answerNodeText(answer.value.radicand)}`;
    case 'negative': return `−${answerNodeText(answer.value)}`;
    case 'plus_minus': return `±${answerNodeText(answer.value)}`;
    case 'tuple': return answer.value.map(answerNodeText).join(', ');
    case 'variable': return answer.value;
  }
}

export function editorValue(state: EditorState): string | null {
  const text = answerNodeText(state.answer);
  return text.length > 0 ? text : null;
}

export function integerAnswerValue(answer: AnswerNode): string | null {
  return answer.type === 'integer' ? answer.value : null;
}

export function formatCurriculumPath(path: readonly (CurriculumPathSegment | string)[]): string {
  return path.map((segment) => typeof segment === 'string' ? segment : segment.label).join(' > ');
}
