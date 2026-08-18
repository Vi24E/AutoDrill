import { DRILL_CORE_CONTRACT, type DrillCoreGradeWarningCode } from '@/generated/drill-core-contract';
import type {
  AnswerInputInterface as RustAnswerInputInterface,
  AnswerNode as RustAnswerNode,
  AnswerSchema as RustAnswerSchema,
  ArithmeticExpression as RustArithmeticExpression,
  ArithmeticOperator as RustArithmeticOperator,
  ColumnMultiplicationPartial as RustColumnMultiplicationPartial,
  EditorStructure as RustEditorStructure,
  GenerateWorksheetRequest as RustGenerateWorksheetRequest,
  LiarStatement as RustLiarStatement,
  LongDivisionStep as RustLongDivisionStep,
  OperationVector as RustOperationVector,
  Problem as RustProblem,
  ProblemPrompt as RustProblemPrompt,
  ProblemSetIdentity as RustProblemSetIdentity,
  RationalCoefficient as RustRationalCoefficient,
  OperationPlan as RustOperationPlan,
  WorkedSolution as RustWorkedSolution,
  Worksheet as RustWorksheet,
} from '@/generated/wire';

/**
 * The public Web/WASM boundary mirrors generated Rust contracts. Theme-specific
 * metadata is resolved by ThemeDefinition from the generated contract; this
 * module only owns generic Web/WASM boundary types and behavior.
 */
export const DRILL_SCHEMA_VERSION = DRILL_CORE_CONTRACT.schema_version;
export const DRILL_OPERATION_KIND_COUNT = DRILL_CORE_CONTRACT.operation_kind_count;

export function drillOperationKindCountForSchema(schemaVersion: number): number | undefined {
  return schemaVersion === DRILL_SCHEMA_VERSION ? DRILL_OPERATION_KIND_COUNT : undefined;
}


export type DifficultyLevel = 1 | 2 | 3 | 4;

export type CurriculumPathSegment = {
  id: string;
  label: string;
};

export type WorksheetLayout = {
  problem_count: number;
  columns: number;
  rows: number;
};

/** Exact generate_worksheet request; registry owns layout and generator revision. */
export type DrillSettings = Pick<RustGenerateWorksheetRequest, 'numeric_theme_id' | 'seed'> & {
  schema_version: typeof DRILL_SCHEMA_VERSION;
  difficulty: DifficultyLevel;
};

export type ProblemSetIdentity = Omit<RustProblemSetIdentity, 'schema_version' | 'difficulty'> & {
  schema_version: typeof DRILL_SCHEMA_VERSION;
  difficulty: DifficultyLevel;
};

/** Canonical Rust AnswerNode generated from the Serde wire type. */
export type AnswerNode = RustAnswerNode;

export type IntegerAnswerNode = Extract<AnswerNode, { type: 'integer' }>;

export type AnswerInputStructure = RustEditorStructure;
export type AnswerInputInterface =
  | Extract<RustAnswerInputInterface, { type: 'simple_numeric' }>
  | (Omit<Extract<RustAnswerInputInterface, { type: 'structured_math' }>, 'allowed_structures'> & {
      allowed_structures: readonly AnswerInputStructure[];
    })
  | Extract<RustAnswerInputInterface, { type: 'digit_grid' }>;

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
  if (inputInterface.type === 'digit_grid') {
    return { allow_decimal: false, allow_negative: false, allowed_structures: [] };
  }
  return {
    allow_decimal: inputInterface.allowed_structures.includes('decimal'),
    allow_negative: inputInterface.allowed_structures.includes('negative'),
    allowed_structures: inputInterface.allowed_structures,
  };
}

export type RationalCoefficient = RustRationalCoefficient;
export type ArithmeticOperator = RustArithmeticOperator;
export type ArithmeticExpression = RustArithmeticExpression;
export type LiarStatement = RustLiarStatement;
export type ProblemPrompt = RustProblemPrompt;
export type AnswerSchema = RustAnswerSchema;
export type OperationVector = RustOperationVector;
export type OperationPlan = RustOperationPlan;
export type ColumnMultiplicationPartial = RustColumnMultiplicationPartial;
export type LongDivisionStep = RustLongDivisionStep;
export type WorkedSolution = RustWorkedSolution;

/** Rust Problem plus the stable UI problem-id projection. */
export type ProblemDto = Omit<RustProblem, 'schema_version' | 'worked_solution' | 'input_interface'> & {
  schema_version: typeof DRILL_SCHEMA_VERSION;
  /** Stable UI key derived from the Rust numeric problem id. */
  problem_id: string;
  input_interface: AnswerInputInterface;
  worked_solution?: WorkedSolution;
};

/** Rust Worksheet plus Web convenience projections. */
export type WorksheetDto = Omit<RustWorksheet, 'schema_version' | 'identity' | 'problems'> & {
  schema_version: typeof DRILL_SCHEMA_VERSION;
  identity: ProblemSetIdentity;
  problems: readonly ProblemDto[];
  seed: string;
};

export type AnswerEntry = {
  problem_id: string;
  answer: AnswerNode;
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
  | 'invalid_sampling_strategy'
  | 'invalid_registry'
  | 'invalid_generated_problem'
  | 'invalid_generated_worksheet'
  | 'invalid_answer_schema'
  | 'expected_answer_outside_schema'
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
  parseMathLiveAnswer(latex: string, inputInterface: AnswerInputInterface): Promise<AnswerNode>;
  gradeAnswer(request: GradeRequest): Promise<GradeResult>;
}

export const DEFAULT_DRILL_SETTINGS: DrillSettings = {
  schema_version: DRILL_SCHEMA_VERSION,
  // The Web curriculum registry replaces this with the selected theme ID.
  numeric_theme_id: 2,
  difficulty: 2,
  // q1 resolves a blank value to a fresh automatic seed per click.
  seed: '',
};

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
    case 'binary': {
      if (answer.value.operator === 'add' && answer.value.right.type === 'plus_minus') {
        return `${answerNodeText(answer.value.left)} ${answerNodeText(answer.value.right)}`;
      }
      const operator = answer.value.operator === 'add' ? ' + ' : answer.value.operator === 'subtract' ? ' − ' : ' × ';
      return `${answerNodeText(answer.value.left)}${operator}${answerNodeText(answer.value.right)}`;
    }
    case 'tuple': return answer.value.map(answerNodeText).join(', ');
    case 'variable': return answer.value;
  }
}

export function integerAnswerValue(answer: AnswerNode): string | null {
  return answer.type === 'integer' ? answer.value : null;
}

export function formatCurriculumPath(path: readonly (CurriculumPathSegment | string)[]): string {
  return path.map((segment) => typeof segment === 'string' ? segment : segment.label).join(' > ');
}
