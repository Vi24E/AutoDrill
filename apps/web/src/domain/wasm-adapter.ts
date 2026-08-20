import {
  DRILL_SCHEMA_VERSION,
  DrillEngineError,
  answerNodeText,
  type AnswerNode,
  type AnswerInputInterface,
  type DrillEngine,
  type DrillSettings,
  type GradeRequest,
  type GradeResult,
  type GradeWarningCode,
  type ProblemDto,
  type ProblemSetIdentity,
  type WorksheetDto,
} from './drill-engine';
import { DRILL_CORE_CONTRACT } from '@/generated/drill-core-contract';

/** Generated wasm-pack exports. New requests use the current generated schema. */
export type DrillWasmRuntime = {
  generate_problem?: (request: string) => unknown | Promise<unknown>;
  generate_worksheet?: (request: string) => unknown | Promise<unknown>;
  regenerate_problem_set?: (request: string) => unknown | Promise<unknown>;
  parse_mathlive_answer?: (request: string) => unknown | Promise<unknown>;
  normalize_answer?: (request: string) => unknown | Promise<unknown>;
  grade_answer?: (request: string) => unknown | Promise<unknown>;
};

declare global {
  interface Window {
    /** Set by the generated drill-wasm package at application bootstrap. */
    __AUTODRILL_WASM__?: DrillWasmRuntime;
    /** Current generated Rust/Web schema exposed for browser contract probes. */
    __AUTODRILL_SCHEMA_VERSION__?: number;
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
  if (kind === 'unsupported_schema_version') {
    return new DrillEngineError('unsupported_schema_version', 'The Rust boundary rejected the schema version.', error);
  }
  if (kind === 'unknown_theme') {
    return new DrillEngineError('unknown_theme', 'The Rust boundary does not recognize this theme.', error);
  }
  if (kind === 'unknown_generator_revision') {
    return new DrillEngineError('unknown_generator_revision', 'The Rust boundary does not recognize this generator revision.', error);
  }
  if (kind === 'invalid_problem_set_identity') {
    return new DrillEngineError('invalid_problem_set_identity', 'The problem-set identity is invalid.', error);
  }
  if (kind === 'input_structure_not_allowed') {
    return new DrillEngineError('input_structure_not_allowed', 'The requested input structure is not allowed.', error);
  }
  if (kind === 'input_interface_violation') {
    return new DrillEngineError('input_interface_violation', 'The answer violates the Rust input-interface contract.', error);
  }
  if (kind === 'answer_ast_size_limit') {
    return new DrillEngineError(
      'answer_ast_size_limit',
      'The answer AST exceeded its maximum size.',
      error,
    );
  }
  if (kind === 'invalid_sampling_strategy') {
    return new DrillEngineError(
      'invalid_sampling_strategy',
      'The registered generator violated its sampling strategy contract.',
      error,
    );
  }
  if (kind === 'invalid_registry') {
    return new DrillEngineError('invalid_registry', 'The Rust theme registry is invalid.', error);
  }
  if (kind === 'invalid_generated_problem') {
    return new DrillEngineError(
      'invalid_generated_problem',
      'A generator produced a problem that violated the Rust domain contract.',
      error,
    );
  }
  if (kind === 'invalid_generated_worksheet') {
    return new DrillEngineError(
      'invalid_generated_worksheet',
      'Worksheet assembly violated the Rust domain contract.',
      error,
    );
  }
  if (kind === 'invalid_answer_schema') {
    return new DrillEngineError('invalid_answer_schema', 'The answer schema is structurally invalid.', error);
  }
  if (kind === 'expected_answer_outside_schema') {
    return new DrillEngineError(
      'expected_answer_outside_schema',
      'The expected answer does not satisfy its Rust answer schema.',
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
  if (!Number.isSafeInteger(value)) invalidDto(`WASM returned a non-safe integer ${label}.`, value);
}

function assertU32(value: unknown, label: string): asserts value is number {
  assertInteger(value, label);
  if (value < 0 || value > 0xffff_ffff) invalidDto(`WASM returned an out-of-range ${label}.`, value);
}

function assertU8(value: unknown, label: string): asserts value is number {
  assertInteger(value, label);
  if (value < 0 || value > 0xff) invalidDto(`WASM returned an out-of-range ${label}.`, value);
}

function assertWorkedSolution(value: unknown): void {
  if (!isRecord(value) || typeof value.kind !== 'string') {
    invalidDto('WASM returned an invalid worked solution.', value);
  }
  if (value.kind === 'column_multiplication') {
    if (!Array.isArray(value.partial_products)) invalidDto('WASM returned invalid multiplication partial products.', value);
    value.partial_products.forEach((partial) => {
      if (!isRecord(partial)) invalidDto('WASM returned an invalid multiplication partial product.', partial);
      assertInteger(partial.value, 'multiplication partial product');
      assertU32(partial.place, 'multiplication partial place');
    });
    return;
  }
  if (value.kind === 'long_division') {
    assertInteger(value.divisor, 'long-division divisor');
    assertInteger(value.dividend_coefficient, 'long-division dividend coefficient');
    assertU32(value.dividend_scale, 'long-division dividend scale');
    assertU32(value.quotient_trailing_cells, 'long-division quotient trailing cells');
    if (!Array.isArray(value.steps)) invalidDto('WASM returned invalid long-division steps.', value);
    value.steps.forEach((step) => {
      if (!isRecord(step)) invalidDto('WASM returned an invalid long-division step.', step);
      assertInteger(step.product, 'long-division product');
      assertInteger(step.after, 'long-division next partial dividend');
      assertU32(step.product_offset, 'long-division product offset');
      assertU32(step.after_offset, 'long-division next-partial offset');
    });
    return;
  }
  invalidDto('WASM returned an unknown worked-solution kind.', value);
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

const MAX_ANSWER_AST_SIZE = DRILL_CORE_CONTRACT.max_answer_ast_size;

type AnswerValidationState = {
  visitedNodes: number;
};

function assertAnswerNode(value: unknown): asserts value is AnswerNode {
  validateAnswerNode(value, { visitedNodes: 0 });
}

function validateAnswerNode(value: unknown, validation: AnswerValidationState): void {
  if (!isRecord(value) || typeof value.type !== 'string') invalidDto('WASM returned an invalid tagged AnswerNode.', value);
  validation.visitedNodes += 1;
  if (validation.visitedNodes > MAX_ANSWER_AST_SIZE) {
    invalidDto('WASM returned an AnswerNode exceeding the structural node limit.', value);
  }
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
    case 'nan_error':
      if (typeof value.value !== 'string') invalidDto('WASM returned an invalid raw answer text.', value);
      return;
    case 'fraction':
      if (!isRecord(value.value)) invalidDto('WASM returned an invalid fraction value.', value);
      validateAnswerNode(value.value.numerator, validation);
      validateAnswerNode(value.value.denominator, validation);
      return;
    case 'mixed_fraction':
      if (!isRecord(value.value)) invalidDto('WASM returned an invalid mixed-fraction value.', value);
      validateAnswerNode(value.value.whole, validation);
      validateAnswerNode(value.value.numerator, validation);
      validateAnswerNode(value.value.denominator, validation);
      return;
    case 'root':
      if (!isRecord(value.value)) invalidDto('WASM returned an invalid root value.', value);
      validateAnswerNode(value.value.radicand, validation);
      if (value.value.index !== null) validateAnswerNode(value.value.index, validation);
      return;
    case 'negative':
    case 'plus_minus':
      validateAnswerNode(value.value, validation);
      return;
    case 'binary':
      if (!isRecord(value.value) || !['add', 'subtract', 'multiply'].includes(String(value.value.operator))) {
        invalidDto('WASM returned an invalid algebraic binary answer.', value);
      }
      validateAnswerNode(value.value.left, validation);
      validateAnswerNode(value.value.right, validation);
      return;
    case 'tuple':
      if (!Array.isArray(value.value)) invalidDto('WASM returned an invalid tuple value.', value);
      value.value.forEach((item) => validateAnswerNode(item, validation));
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
    invalidDto('WASM response did not contain the current-schema envelope.', value);
  }
  if (decoded.schema_version !== DRILL_SCHEMA_VERSION) {
    invalidDto('WASM response used an unsupported schema version.', value);
  }
  if (!decoded.ok) throw mapBoundaryError(decoded.error ?? decoded);
  return decoded.data;
}

const INPUT_STRUCTURES = DRILL_CORE_CONTRACT.editor_structures;

/** Validate only the generated Serde wire shape. Domain capability semantics stay in Rust. */
function assertInputInterface(value: unknown): asserts value is AnswerInputInterface {
  if (!isRecord(value) || typeof value.type !== 'string') {
    invalidDto('WASM returned an invalid input interface.', value);
  }
  if (value.type === 'simple_numeric') {
    if (typeof value.allow_decimal !== 'boolean' || typeof value.allow_negative !== 'boolean') {
      invalidDto('WASM returned an invalid simple-numeric input interface.', value);
    }
    return;
  }
  if (value.type === 'digit_grid') {
    assertU8(value.min_digit, 'digit-grid minimum digit');
    assertU8(value.max_digit, 'digit-grid maximum digit');
    assertU8(value.cell_count, 'digit-grid cell count');
    return;
  }
  if (value.type === 'structured_math') {
    if (!Array.isArray(value.allowed_structures)) {
      invalidDto('WASM returned an invalid structured-math input interface.', value);
    }
    for (const structure of value.allowed_structures) {
      if (typeof structure !== 'string' || !(INPUT_STRUCTURES as readonly string[]).includes(structure)) {
        invalidDto('WASM returned an unknown input-interface capability.', value);
      }
    }
    return;
  }
  invalidDto(`WASM returned an unsupported input interface: ${value.type}.`, value);
}

function assertRationalCoefficient(value: unknown, label: string): void {
  if (!isRecord(value)) invalidDto(`WASM returned an invalid ${label}.`, value);
  assertInteger(value.numerator, `${label} numerator`);
  assertInteger(value.denominator, `${label} denominator`);
}

function assertArithmeticExpression(value: unknown): void {
  if (!isRecord(value) || typeof value.kind !== 'string') invalidDto('WASM returned an invalid arithmetic expression.', value);
  if (value.kind === 'integer') {
    assertInteger(value.value, 'arithmetic integer');
    return;
  }
  if (value.kind === 'rational') {
    assertRationalCoefficient(value.value, 'arithmetic rational');
    return;
  }
  if (value.kind === 'exact_decimal') {
    assertInteger(value.coefficient, 'arithmetic decimal coefficient');
    assertU32(value.scale, 'arithmetic decimal scale');
    return;
  }
  if (value.kind === 'binary') {
    if (!['add', 'subtract', 'multiply', 'divide'].includes(String(value.operator))) invalidDto('WASM returned an invalid arithmetic operator.', value);
    assertArithmeticExpression(value.left);
    assertArithmeticExpression(value.right);
    return;
  }
  invalidDto('WASM returned an unsupported arithmetic expression variant.', value);
}

function assertPrompt(value: unknown): void {
  if (!isRecord(value) || typeof value.kind !== 'string') {
    invalidDto('WASM returned an invalid problem prompt.', value);
  }
  if (value.kind === 'addition') {
    assertU8(value.left, 'addition left operand');
    assertU8(value.right, 'addition right operand');
    return;
  }
  if (value.kind === 'arithmetic') {
    assertArithmeticExpression(value.expression);
    return;
  }
  if (value.kind === 'column_arithmetic') {
    if (!['add', 'subtract', 'multiply', 'divide'].includes(String(value.operator))) {
      invalidDto('WASM returned an invalid column-arithmetic operator.', value);
    }
    assertArithmeticExpression(value.left);
    assertArithmeticExpression(value.right);
    return;
  }
  if (value.kind === 'simultaneous_equation') {
    for (const name of ['a', 'b', 'c', 'd', 'e', 'f'] as const) {
      assertInteger(value[name], `simultaneous coefficient ${name}`);
    }
    return;
  }
  if (value.kind === 'liar_puzzle') {
    assertU8(value.people_count, 'liar-puzzle people count');
    if (!Array.isArray(value.statements)) invalidDto('WASM returned an invalid liar-puzzle statement list.', value);
    for (const statement of value.statements) {
      if (!isRecord(statement)) invalidDto('WASM returned an invalid liar-puzzle statement.', statement);
      if (statement.kind === 'says_liar' || statement.kind === 'says_not_liar') {
        assertU8(statement.person, 'liar-puzzle person');
      } else if (statement.kind === 'exactly_one_liar' || statement.kind === 'both_liar' || statement.kind === 'both_not_liar') {
        assertU8(statement.first, 'liar-puzzle first person');
        assertU8(statement.second, 'liar-puzzle second person');
      } else if (statement.kind === 'exact_liar_count') {
        assertU8(statement.count, 'liar-puzzle liar count');
      } else {
        invalidDto('WASM returned an unsupported liar-puzzle statement.', statement);
      }
    }
    return;
  }
  if (value.kind === 'mini_sudoku') {
    if (!Array.isArray(value.givens)) invalidDto('WASM returned an invalid mini-sudoku grid.', value);
    for (const cell of value.givens) {
      if (cell !== null) assertU8(cell, 'mini-sudoku given');
    }
    return;
  }
  if (value.kind === 'quadratic_equation') {
    if (!['square_equals_constant', 'square_plus_constant_zero', 'factored_scale', 'standard'].includes(String(value.form))) {
      invalidDto('WASM returned an invalid quadratic-equation form.', value);
    }
    assertRationalCoefficient(value.a, 'quadratic coefficient a');
    assertRationalCoefficient(value.b, 'quadratic coefficient b');
    assertRationalCoefficient(value.c, 'quadratic coefficient c');
    return;
  }
  if (value.kind === 'linear_equation') {
    assertRationalCoefficient(value.a, 'linear coefficient a');
    assertRationalCoefficient(value.b, 'linear coefficient b');
    assertRationalCoefficient(value.c, 'linear coefficient c');
    assertRationalCoefficient(value.d, 'linear coefficient d');
    if (typeof value.left_negative_constant_as_subtraction !== 'boolean'
        || typeof value.right_negative_constant_as_subtraction !== 'boolean') {
      invalidDto('WASM returned invalid linear-equation display metadata.', value);
    }
    return;
  }
  invalidDto(`WASM returned an unsupported problem prompt: ${value.kind}.`, value);
}

function assertAnswerSchema(value: unknown): void {
  if (!isRecord(value) || typeof value.kind !== 'string') {
    invalidDto('WASM returned an invalid answer schema.', value);
  }
  if (value.kind === 'integer') {
    assertCanonicalI64String(value.min, 'answer-schema minimum');
    assertCanonicalI64String(value.max, 'answer-schema maximum');
    return;
  }
  if (value.kind === 'algebraic' || value.kind === 'ordered_pair') return;
  if (value.kind === 'ordered_tuple') {
    assertU32(value.length, 'ordered-tuple length');
    return;
  }
  if (value.kind === 'decimal') {
    assertU32(value.max_scale, 'answer-schema maximum decimal scale');
    return;
  }
  if (value.kind === 'rational') {
    assertU32(value.max_abs_numerator, 'answer-schema maximum numerator');
    assertU32(value.max_denominator, 'answer-schema maximum denominator');
    if (typeof value.require_reduced_fraction_form !== 'boolean') {
      invalidDto('WASM returned an invalid rational answer schema.', value);
    }
    return;
  }
  invalidDto(`WASM returned an unsupported answer schema: ${value.kind}.`, value);
}

function assertIdentity(value: unknown): asserts value is ProblemSetIdentity {
  if (!isRecord(value)) invalidDto('WASM returned an empty problem-set identity.', value);
  if (value.schema_version !== DRILL_SCHEMA_VERSION) invalidDto('WASM returned an unsupported identity schema.', value);
  assertU32(value.numeric_theme_id, 'identity numeric_theme_id');
  assertU32(value.generator_revision, 'identity generator_revision');
  if (typeof value.seed !== 'string') invalidDto('WASM returned an invalid identity seed.', value);
  assertU8(value.difficulty, 'identity difficulty');
}

function assertWorksheet(value: unknown): WorksheetDto {
  const unwrapped = unwrapEnvelope(value);
  if (!isRecord(unwrapped)) invalidDto('WASM returned an empty worksheet DTO.', value);
  if (unwrapped.schema_version !== DRILL_SCHEMA_VERSION || typeof unwrapped.problem_set_id !== 'string') {
    invalidDto('WASM returned a worksheet with an unsupported schema.', value);
  }
  assertIdentity(unwrapped.identity);
  const identity = unwrapped.identity;
  if (typeof unwrapped.skill_id !== 'string' || !Array.isArray(unwrapped.curriculum_path)
      || !unwrapped.curriculum_path.every((segment) => typeof segment === 'string')) {
    invalidDto('WASM returned an invalid worksheet metadata projection.', value);
  }
  if (!isRecord(unwrapped.layout)) invalidDto('WASM returned an invalid worksheet layout.', value);
  assertU32(unwrapped.layout.problem_count, 'worksheet problem count');
  assertU32(unwrapped.layout.columns, 'worksheet column count');
  assertU32(unwrapped.layout.rows, 'worksheet row count');
  if (!Array.isArray(unwrapped.problems)) invalidDto('WASM returned an invalid worksheet problem list.', value);
  const problems = unwrapped.problems.map((problem, index) => {
    if (!isRecord(problem)) invalidDto(`WASM returned an invalid problem at index ${index}.`, problem);
    if (problem.schema_version !== DRILL_SCHEMA_VERSION) invalidDto('WASM returned a problem with an unsupported schema.', problem);
    assertU32(problem.id, 'problem id');
    assertU32(problem.numeric_theme_id, 'problem numeric_theme_id');
    assertPrompt(problem.prompt);
    assertInputInterface(problem.input_interface);
    assertAnswerSchema(problem.answer_schema);
    assertAnswerNode(problem.canonical_answer);
    if (problem.worked_solution !== null) {
      assertWorkedSolution(problem.worked_solution);
    }
    return {
      ...problem,
      problem_id: String(problem.id),
    } as ProblemDto;
  });
  return {
    ...unwrapped,
    identity,
    problems,
    seed: identity.seed,
  } as unknown as WorksheetDto;
}

function answerTextOrNull(value: AnswerNode): string | null {
  return value.type === 'empty' ? null : answerNodeText(value);
}

const GRADE_WARNING_CODES: readonly GradeWarningCode[] = DRILL_CORE_CONTRACT.grade_warning_codes;

function hasKnownGradeWarnings(value: unknown): value is GradeWarningCode[] {
  return Array.isArray(value)
    && value.every((warning) => typeof warning === 'string' && GRADE_WARNING_CODES.includes(warning as GradeWarningCode));
}

function gradeItemFromWasm(problemId: string, value: unknown): { problem_id: string; answer: string | null; correct: boolean; warnings: readonly GradeWarningCode[] } {
  const data = unwrapEnvelope(value);
  if (!isRecord(data) || typeof data.is_correct !== 'boolean') invalidDto('WASM returned an invalid grade DTO.', value);
  assertAnswerNode(data.expected);
  assertAnswerNode(data.actual);
  if (data.status !== 'correct' && data.status !== 'incorrect' && data.status !== 'unanswered') {
    invalidDto('WASM returned an unsupported grade status.', data.status);
  }
  if (!hasKnownGradeWarnings(data.warnings)) {
    invalidDto('WASM returned unknown grade warnings.', data.warnings);
  }
  return {
    problem_id: problemId,
    answer: answerTextOrNull(data.actual),
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

    async parseMathLiveAnswer(latex, inputInterface) {
      try {
        assertInputInterface(inputInterface);
        const parse = resolveRuntime(runtime).parse_mathlive_answer;
        if (!parse) throw new DrillEngineError('wasm_unavailable', 'drill-wasm does not expose parse_mathlive_answer.');
        const decoded = decodeWasmValue(await invokeBoundary(parse, {
          schema_version: DRILL_SCHEMA_VERSION,
          input_interface: {
            ...inputInterface,
            ...(inputInterface.type === 'structured_math'
              ? { allowed_structures: [...inputInterface.allowed_structures] }
              : {}),
          },
          latex,
        }));
        const data = unwrapEnvelope(decoded);
        assertAnswerNode(data);
        return data;
      } catch (error) {
        throw mapBoundaryError(error);
      }
    },

    async gradeAnswer(request: GradeRequest) {
      try {
        const gradeAnswer = resolveRuntime(runtime).grade_answer;
        if (!gradeAnswer) throw new DrillEngineError('wasm_unavailable', 'drill-wasm does not expose grade_answer.');
        const items = await Promise.all(request.worksheet.problems.map(async (problem) => {
          assertInputInterface(problem.input_interface);
          assertAnswerNode(problem.canonical_answer);
          const answer = request.answers.find((entry) => entry.problem_id === problem.problem_id)?.answer;
          if (answer) assertAnswerNode(answer);
          const value = await invokeBoundary(gradeAnswer, {
            schema_version: request.schema_version,
            expected: problem.canonical_answer,
            actual: answer ?? { type: 'empty' },
            answer_schema: problem.answer_schema,
            input_interface: problem.input_interface,
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
