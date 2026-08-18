import {
  drillOperationKindCountForSchema,
  DRILL_SCHEMA_VERSION,
  DrillEngineError,
  answerNodeText,
  type AnswerNode,
  type AnswerInputInterface,
  type AnswerInputStructure,
  type DrillEngine,
  type DrillSettings,
  type GradeRequest,
  type GradeResult,
  type GradeWarningCode,
  type ProblemDto,
  type ProblemSetIdentity,
  type WorksheetDto,
} from './drill-engine';
import { findThemeDefinitionByNumericId, sameInputInterface } from './theme-registry';
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
    if (value.divisor <= 0) invalidDto('WASM returned a non-positive long-division divisor.', value.divisor);
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

const MAX_ANSWER_AST_SIZE = DRILL_CORE_CONTRACT.max_answer_ast_size;

type AnswerValidationState = {
  visitedNodes: number;
};

function assertAnswerNode(value: unknown): asserts value is AnswerNode {
  validateAnswerNode(value, { visitedNodes: 0 }, MAX_ANSWER_AST_SIZE);
}

function validateAnswerNode(
  value: unknown,
  validation: AnswerValidationState,
  displayRemaining: number,
): number {
  if (!isRecord(value) || typeof value.type !== 'string') invalidDto('WASM returned an invalid tagged AnswerNode.', value);
  validation.visitedNodes += 1;
  if (validation.visitedNodes > MAX_ANSWER_AST_SIZE) {
    invalidDto('WASM returned an AnswerNode exceeding the structural node limit.', value);
  }
  let size: number;
  switch (value.type) {
    case 'empty':
      size = 0;
      break;
    case 'integer':
      assertCanonicalI64String(value.value, 'integer answer');
      size = value.value.startsWith('-') ? value.value.length - 1 : value.value.length;
      break;
    case 'exact_decimal':
      if (!isRecord(value.value)) invalidDto('WASM returned an invalid exact-decimal value.', value);
      assertCanonicalI64String(value.value.coefficient, 'exact-decimal coefficient');
      assertU32(value.value.scale, 'exact-decimal scale');
      size = Math.max(
        value.value.coefficient.startsWith('-') ? value.value.coefficient.length - 1 : value.value.coefficient.length,
        value.value.scale + 1,
      );
      break;
    case 'nan_error':
      if (typeof value.value !== 'string') invalidDto('WASM returned an invalid raw answer text.', value);
      size = [...value.value].slice(0, displayRemaining + 1).length;
      break;
    case 'fraction':
      if (!isRecord(value.value)) invalidDto('WASM returned an invalid fraction value.', value);
      if (displayRemaining < 1) invalidDto('WASM returned an AnswerNode exceeding the display-size limit.', value);
      {
        let remaining = displayRemaining - 1;
        const numeratorSize = validateAnswerNode(value.value.numerator, validation, remaining);
        remaining -= numeratorSize;
        const denominatorSize = validateAnswerNode(value.value.denominator, validation, remaining);
        remaining -= denominatorSize;
        size = displayRemaining - remaining;
      }
      break;
    case 'mixed_fraction':
      if (!isRecord(value.value)) invalidDto('WASM returned an invalid mixed-fraction value.', value);
      if (displayRemaining < 1) invalidDto('WASM returned an AnswerNode exceeding the display-size limit.', value);
      {
        let remaining = displayRemaining - 1;
        const wholeSize = validateAnswerNode(value.value.whole, validation, remaining);
        remaining -= wholeSize;
        const numeratorSize = validateAnswerNode(value.value.numerator, validation, remaining);
        remaining -= numeratorSize;
        const denominatorSize = validateAnswerNode(value.value.denominator, validation, remaining);
        remaining -= denominatorSize;
        size = displayRemaining - remaining;
      }
      break;
    case 'root':
      if (!isRecord(value.value)) invalidDto('WASM returned an invalid root value.', value);
      if (displayRemaining < 1) invalidDto('WASM returned an AnswerNode exceeding the display-size limit.', value);
      {
        let remaining = displayRemaining - 1;
        const radicandSize = validateAnswerNode(value.value.radicand, validation, remaining);
        remaining -= radicandSize;
        if (value.value.index !== null) {
          const indexSize = validateAnswerNode(value.value.index, validation, remaining);
          remaining -= indexSize;
        }
        size = displayRemaining - remaining;
      }
      break;
    case 'negative':
    case 'plus_minus':
      if (displayRemaining < 1) invalidDto('WASM returned an AnswerNode exceeding the display-size limit.', value);
      size = 1 + validateAnswerNode(value.value, validation, displayRemaining - 1);
      break;
    case 'binary':
      if (!isRecord(value.value) || !['add', 'subtract', 'multiply'].includes(String(value.value.operator))) {
        invalidDto('WASM returned an invalid algebraic binary answer.', value);
      }
      if (displayRemaining < 1) invalidDto('WASM returned an AnswerNode exceeding the display-size limit.', value);
      {
        let remaining = displayRemaining - 1;
        const leftSize = validateAnswerNode(value.value.left, validation, remaining);
        remaining -= leftSize;
        const rightSize = validateAnswerNode(value.value.right, validation, remaining);
        remaining -= rightSize;
        size = displayRemaining - remaining;
      }
      break;
    case 'tuple':
      if (!Array.isArray(value.value)) invalidDto('WASM returned an invalid tuple value.', value);
      if (displayRemaining < 1) invalidDto('WASM returned an AnswerNode exceeding the display-size limit.', value);
      {
        let remaining = displayRemaining - 1;
        size = 1;
        for (const item of value.value) {
          const itemSize = validateAnswerNode(item, validation, remaining);
          remaining -= itemSize;
          size += itemSize;
        }
      }
      break;
    case 'variable':
      if (typeof value.value !== 'string') invalidDto('WASM returned an invalid variable value.', value);
      size = Math.max(1, [...value.value].length);
      break;
    default:
      invalidDto(`WASM returned an unsupported AnswerNode type: ${value.type}.`, value);
  }
  if (size > displayRemaining || size > MAX_ANSWER_AST_SIZE) {
    invalidDto('WASM returned an AnswerNode exceeding the size limit.', value);
  }
  return size;
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

const INPUT_STRUCTURES: readonly AnswerInputStructure[] = DRILL_CORE_CONTRACT.editor_structures;

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
    assertInteger(value.min_digit, 'digit-grid minimum digit');
    assertInteger(value.max_digit, 'digit-grid maximum digit');
    assertInteger(value.cell_count, 'digit-grid cell count');
    if (value.min_digit < 0 || value.max_digit < value.min_digit || value.max_digit > 9 || value.cell_count <= 0) {
      invalidDto('WASM returned an invalid digit-grid input interface.', value);
    }
    return;
  }
  if (value.type === 'structured_math') {
    if (!Array.isArray(value.allowed_structures)) {
      invalidDto('WASM returned an invalid structured-math input interface.', value);
    }
    const seen = new Set<string>();
    for (const structure of value.allowed_structures) {
      if (typeof structure !== 'string' || !INPUT_STRUCTURES.includes(structure as AnswerInputStructure) || seen.has(structure)) {
        invalidDto('WASM returned invalid or duplicate input-interface capabilities.', value);
      }
      seen.add(structure);
    }
    return;
  }
  invalidDto(`WASM returned an unsupported input interface: ${value.type}.`, value);
}

function inputAllowsStructure(inputInterface: AnswerInputInterface, structure: AnswerInputStructure): boolean {
  if (inputInterface.type === 'digit_grid') return false;
  if (inputInterface.type === 'simple_numeric') {
    return structure === 'decimal'
      ? inputInterface.allow_decimal
      : structure === 'negative'
        ? inputInterface.allow_negative
        : false;
  }
  return inputInterface.allowed_structures.includes(structure);
}

/**
 * Validate the complete answer tree, not just the active editor leaf. A
 * nan_error is intentionally an interface-neutral raw-text sentinel: it is
 * never a typed capability and editor recovery must not coerce it into a
 * forbidden typed node.
 */
function assertAnswerSupportsInputInterface(answer: AnswerNode, inputInterface: AnswerInputInterface): void {
  if (inputInterface.type === 'digit_grid') {
    if (answer.type === 'empty') return;
    if (answer.type !== 'tuple' || answer.value.length !== inputInterface.cell_count) {
      invalidDto('WASM returned a digit-grid answer with the wrong shape.', answer);
    }
    for (const cell of answer.value) {
      if (cell.type === 'empty') continue;
      if (cell.type !== 'integer') invalidDto('WASM returned a non-integer digit-grid cell.', cell);
      const digit = Number(cell.value);
      if (!Number.isSafeInteger(digit) || digit < inputInterface.min_digit || digit > inputInterface.max_digit) {
        invalidDto('WASM returned an out-of-range digit-grid cell.', cell);
      }
    }
    return;
  }
  switch (answer.type) {
    case 'empty':
    case 'nan_error':
      return;
    case 'integer':
      if (answer.value.startsWith('-') && !inputAllowsStructure(inputInterface, 'negative')) {
        invalidDto('WASM returned a negative integer outside the input interface.', answer);
      }
      return;
    case 'exact_decimal':
      if (!inputAllowsStructure(inputInterface, 'decimal')) {
        invalidDto('WASM returned a decimal outside the input interface.', answer);
      }
      if (answer.value.coefficient.startsWith('-') && !inputAllowsStructure(inputInterface, 'negative')) {
        invalidDto('WASM returned a negative decimal outside the input interface.', answer);
      }
      return;
    case 'fraction':
      if (!inputAllowsStructure(inputInterface, 'fraction')) invalidDto('WASM returned a disallowed fraction.', answer);
      assertAnswerSupportsInputInterface(answer.value.numerator, inputInterface);
      assertAnswerSupportsInputInterface(answer.value.denominator, inputInterface);
      return;
    case 'mixed_fraction':
      if (!inputAllowsStructure(inputInterface, 'mixed_fraction')) invalidDto('WASM returned a disallowed mixed fraction.', answer);
      assertAnswerSupportsInputInterface(answer.value.whole, inputInterface);
      assertAnswerSupportsInputInterface(answer.value.numerator, inputInterface);
      assertAnswerSupportsInputInterface(answer.value.denominator, inputInterface);
      return;
    case 'root':
      if (!inputAllowsStructure(inputInterface, 'root')) invalidDto('WASM returned a disallowed root.', answer);
      assertAnswerSupportsInputInterface(answer.value.radicand, inputInterface);
      if (answer.value.index !== null) assertAnswerSupportsInputInterface(answer.value.index, inputInterface);
      return;
    case 'negative':
      if (!inputAllowsStructure(inputInterface, 'negative')) invalidDto('WASM returned a disallowed negative node.', answer);
      assertAnswerSupportsInputInterface(answer.value, inputInterface);
      return;
    case 'plus_minus':
      if (!inputAllowsStructure(inputInterface, 'plus_minus')) invalidDto('WASM returned a disallowed plus-minus node.', answer);
      assertAnswerSupportsInputInterface(answer.value, inputInterface);
      return;
    case 'binary':
      if (!inputAllowsStructure(inputInterface, 'arithmetic')) invalidDto('WASM returned a disallowed algebraic operator.', answer);
      assertAnswerSupportsInputInterface(answer.value.left, inputInterface);
      assertAnswerSupportsInputInterface(answer.value.right, inputInterface);
      return;
    case 'tuple':
      if (!inputAllowsStructure(inputInterface, 'tuple')) invalidDto('WASM returned a disallowed tuple.', answer);
      answer.value.forEach((item) => assertAnswerSupportsInputInterface(item, inputInterface));
      return;
    case 'variable':
      invalidDto('WASM returned a variable outside the input interface.', answer);
  }
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

function assertPrompt(
  value: unknown,
  expectedKind: 'addition' | 'arithmetic' | 'column_arithmetic' | 'linear_equation' | 'quadratic_equation' | 'simultaneous_equation' | 'liar_puzzle' | 'mini_sudoku',
  expectedInputInterface: AnswerInputInterface,
): void {
  if (!isRecord(value) || value.kind !== expectedKind) {
    invalidDto(`WASM returned an unsupported prompt variant; expected ${expectedKind}.`, value);
  }
  if (expectedKind === 'addition') {
    assertInteger(value.left, 'addition left operand');
    assertInteger(value.right, 'addition right operand');
    return;
  }
  if (expectedKind === 'arithmetic') {
    assertArithmeticExpression(value.expression);
    return;
  }
  if (expectedKind === 'column_arithmetic') {
    if (!['add', 'subtract', 'multiply', 'divide'].includes(String(value.operator))) invalidDto('WASM returned an invalid column-arithmetic operator.', value);
    const left = value.left;
    const right = value.right;
    assertArithmeticExpression(left);
    assertArithmeticExpression(right);
    if (!isRecord(left) || !isRecord(right)) invalidDto('WASM returned invalid column-arithmetic operands.', value);
    if (left.kind === 'binary' || left.kind === 'rational' || right.kind === 'binary' || right.kind === 'rational') {
      invalidDto('WASM returned an unsupported column-arithmetic operand.', value);
    }
    return;
  }
  if (expectedKind === 'simultaneous_equation') {
    for (const name of ['a', 'b', 'c', 'd', 'e', 'f'] as const) assertInteger(value[name], `simultaneous coefficient ${name}`);
    return;
  }
  if (expectedKind === 'liar_puzzle') {
    assertInteger(value.people_count, 'liar-puzzle people count');
    if (!Array.isArray(value.statements) || value.statements.length !== value.people_count) {
      invalidDto('WASM returned an invalid liar-puzzle shape.', value);
    }
    for (const statement of value.statements) {
      if (!isRecord(statement)) invalidDto('WASM returned an invalid liar-puzzle statement.', statement);
      if (statement.kind === 'says_liar' || statement.kind === 'says_not_liar') {
        assertInteger(statement.person, 'liar-puzzle person');
      } else if (statement.kind === 'exactly_one_liar' || statement.kind === 'both_liar' || statement.kind === 'both_not_liar') {
        assertInteger(statement.first, 'liar-puzzle first person');
        assertInteger(statement.second, 'liar-puzzle second person');
      } else if (statement.kind === 'exact_liar_count') {
        assertInteger(statement.count, 'liar-puzzle liar count');
      } else {
        invalidDto('WASM returned an unsupported liar-puzzle statement.', statement);
      }
    }
    return;
  }
  if (expectedKind === 'mini_sudoku') {
    if (expectedInputInterface.type !== 'digit_grid') {
      invalidDto('Registered mini-sudoku input contract is not a digit grid.', expectedInputInterface);
    }
    if (!Array.isArray(value.givens) || value.givens.length !== expectedInputInterface.cell_count) {
      invalidDto('WASM returned an invalid mini-sudoku grid.', value);
    }
    for (const cell of value.givens) {
      if (cell === null) continue;
      assertInteger(cell, 'mini-sudoku given');
      if (cell < expectedInputInterface.min_digit || cell > expectedInputInterface.max_digit) {
        invalidDto('WASM returned an out-of-range mini-sudoku given.', cell);
      }
    }
    return;
  }
  if (expectedKind === 'quadratic_equation') {
    if (!['square_equals_constant', 'square_plus_constant_zero', 'factored_scale', 'standard'].includes(String(value.form))) {
      invalidDto('WASM returned an invalid quadratic-equation form.', value);
    }
    assertRationalCoefficient(value.a, 'quadratic coefficient a');
    assertRationalCoefficient(value.b, 'quadratic coefficient b');
    assertRationalCoefficient(value.c, 'quadratic coefficient c');
    return;
  }
  assertRationalCoefficient(value.a, 'linear coefficient a');
  assertRationalCoefficient(value.b, 'linear coefficient b');
  assertRationalCoefficient(value.c, 'linear coefficient c');
  assertRationalCoefficient(value.d, 'linear coefficient d');
  if (typeof value.left_negative_constant_as_subtraction !== 'boolean'
      || typeof value.right_negative_constant_as_subtraction !== 'boolean') {
    invalidDto('WASM returned invalid linear-equation display metadata.', value);
  }
}

function assertAnswerSchema(value: unknown, expectedKind: 'integer' | 'rational' | 'decimal' | 'ordered_pair' | 'ordered_tuple' | 'algebraic'): void {
  if (!isRecord(value) || value.kind !== expectedKind) {
    invalidDto(`WASM returned an unsupported answer schema; expected ${expectedKind}.`, value);
  }
  if (expectedKind === 'integer') {
    assertCanonicalI64String(value.min, 'answer-schema minimum');
    assertCanonicalI64String(value.max, 'answer-schema maximum');
    return;
  }
  if (expectedKind === 'algebraic' || expectedKind === 'ordered_pair') return;
  if (expectedKind === 'ordered_tuple') {
    assertInteger(value.length, 'ordered-tuple length');
    if (value.length <= 0) invalidDto('WASM returned an invalid ordered-tuple answer schema.', value);
    return;
  }
  if (expectedKind === 'decimal') {
    assertU32(value.max_scale, 'answer-schema maximum decimal scale');
    if (value.max_scale === 0) invalidDto('WASM returned an invalid decimal answer schema.', value);
    return;
  }
  assertU32(value.max_abs_numerator, 'answer-schema maximum numerator');
  assertU32(value.max_denominator, 'answer-schema maximum denominator');
  if (value.max_abs_numerator === 0 || value.max_denominator === 0
      || typeof value.require_reduced_fraction_form !== 'boolean') {
    invalidDto('WASM returned an invalid rational answer schema.', value);
  }
}

function assertIdentity(value: unknown): asserts value is ProblemSetIdentity {
  if (!isRecord(value)) invalidDto('WASM returned an empty problem-set identity.', value);
  if (value.schema_version !== DRILL_SCHEMA_VERSION) invalidDto('WASM returned an unsupported identity schema.', value);
  assertInteger(value.numeric_theme_id, 'identity numeric_theme_id');
  assertInteger(value.generator_revision, 'identity generator_revision');
  if (typeof value.seed !== 'string') invalidDto('WASM returned an invalid identity seed.', value);
  if (!/^[1-9a-km-zA-HJ-NP-Z]{1,16}$/.test(value.seed)) {
    invalidDto('WASM returned an invalid identity seed.', value.seed);
  }
  assertInteger(value.difficulty, 'identity difficulty');
  if (value.difficulty < 1 || value.difficulty > 4) invalidDto('WASM returned an invalid identity difficulty.', value);
}

function assertWorksheet(value: unknown): WorksheetDto {
  const unwrapped = unwrapEnvelope(value);
  if (!isRecord(unwrapped)) invalidDto('WASM returned an empty worksheet DTO.', value);
  if (unwrapped.schema_version !== DRILL_SCHEMA_VERSION || typeof unwrapped.problem_set_id !== 'string') {
    invalidDto('WASM returned a worksheet with an unsupported schema.', value);
  }
  assertIdentity(unwrapped.identity);
  const identity = unwrapped.identity;
  const definition = findThemeDefinitionByNumericId(identity.numeric_theme_id);
  if (!definition || identity.generator_revision !== definition.generator_revision) {
    invalidDto('WASM returned an unregistered current-schema theme identity.', identity);
  }
  const expectedProblemSetId = `${DRILL_SCHEMA_VERSION}-${identity.numeric_theme_id}-${identity.generator_revision}-${identity.seed}-${identity.difficulty}`;
  if (unwrapped.problem_set_id !== expectedProblemSetId) {
    invalidDto('WASM returned a problem-set ID inconsistent with its identity.', unwrapped.problem_set_id);
  }
  const expectedPath = definition.compatibility.curriculumPath.map((segment) => segment.label);
  if (unwrapped.skill_id !== definition.compatibility.skillId
      || !Array.isArray(unwrapped.curriculum_path)
      || unwrapped.curriculum_path.length !== expectedPath.length
      || !unwrapped.curriculum_path.every((item, index) => item === expectedPath[index])) {
    invalidDto('WASM returned an invalid worksheet curriculum projection.', value);
  }
  if (!isRecord(unwrapped.layout)
      || unwrapped.layout.problem_count !== definition.layout.problem_count
      || unwrapped.layout.columns !== definition.layout.columns
      || unwrapped.layout.rows !== definition.layout.rows) {
    invalidDto('WASM returned a worksheet layout inconsistent with the theme registry.', value);
  }
  if (!Array.isArray(unwrapped.problems) || unwrapped.problems.length !== definition.problemCount) {
    invalidDto('WASM returned a worksheet with the wrong registered problem count.', value);
  }
  const ids = new Set<number>();
  const problems = unwrapped.problems.map((problem, index) => {
    if (!isRecord(problem)) invalidDto(`WASM returned an invalid problem at index ${index}.`, problem);
    if (problem.schema_version !== DRILL_SCHEMA_VERSION) invalidDto('WASM returned a problem with an unsupported schema.', problem);
    assertInteger(problem.id, 'problem id');
    if (problem.id < 1 || problem.id > definition.problemCount) invalidDto('WASM returned a problem id outside the registered layout.', problem.id);
    if (ids.has(problem.id)) invalidDto('WASM returned duplicate problem ids.', problem);
    ids.add(problem.id);
    assertInteger(problem.numeric_theme_id, 'problem numeric_theme_id');
    if (problem.numeric_theme_id !== identity.numeric_theme_id) {
      invalidDto('WASM returned a problem for a different numeric theme.', problem);
    }
    assertPrompt(problem.prompt, definition.promptKind, definition.inputInterface);
    assertInputInterface(problem.input_interface);
    if (!sameInputInterface(problem.input_interface, definition.inputInterface)) {
      invalidDto('WASM returned input capabilities inconsistent with the theme registry.', problem.input_interface);
    }
    assertAnswerSchema(problem.answer_schema, definition.answerSchemaKind);
    assertAnswerNode(problem.canonical_answer);
    assertAnswerSupportsInputInterface(problem.canonical_answer, problem.input_interface);
    if (problem.worked_solution !== undefined) assertWorkedSolution(problem.worked_solution);
    if (!isRecord(problem.operation_plan) || !Array.isArray(problem.operation_plan.operations)) {
      invalidDto('WASM returned an invalid operation plan.', problem);
    }
    problem.operation_plan.operations.forEach((operation) => {
      if (!isRecord(operation) || typeof operation.kind !== 'string') {
        invalidDto('WASM returned an unknown effort operation.', operation);
      }
      if (operation.kind === 'big_num') {
        assertCanonicalU64String(operation.magnitude, 'BigNum magnitude');
      }
    });
    const expectedOperationKindCount = drillOperationKindCountForSchema(problem.schema_version);
    if (expectedOperationKindCount === undefined || !isRecord(problem.operation_vector) || !Array.isArray(problem.operation_vector.values) || problem.operation_vector.values.length !== expectedOperationKindCount) invalidDto('WASM returned an invalid operation vector.', problem);
    problem.operation_vector.values.forEach((item) => assertFiniteNumber(item, 'operation-vector value'));
    if (problem.theme_specific_effort !== undefined && problem.theme_specific_effort !== null) {
      assertFiniteNumber(problem.theme_specific_effort, 'theme-specific problem effort');
      if (problem.theme_specific_effort < 0) invalidDto('WASM returned a negative theme-specific effort.', problem.theme_specific_effort);
      if (problem.operation_plan.operations.length !== 0
          || problem.operation_vector.values.some((value) => value !== 0)
          || problem.effort !== problem.theme_specific_effort) {
        invalidDto('WASM returned a theme-specific effort mixed with the ordinary operation-vector model.', problem);
      }
    }
    assertFiniteNumber(problem.effort, 'problem effort');
    return {
      ...problem,
      problem_id: String(problem.id),
    } as ProblemDto;
  });
  return {
    ...unwrapped,
    identity: unwrapped.identity,
    problems,
    seed: identity.seed,
  } as unknown as WorksheetDto;
}

function answerTextOrNull(value: AnswerNode): string | null {
  return value.type === 'empty' ? null : answerNodeText(value);
}

function containsNanError(value: AnswerNode): boolean {
  switch (value.type) {
    case 'nan_error': return true;
    case 'fraction': return containsNanError(value.value.numerator) || containsNanError(value.value.denominator);
    case 'mixed_fraction': return containsNanError(value.value.whole)
      || containsNanError(value.value.numerator)
      || containsNanError(value.value.denominator);
    case 'root': return containsNanError(value.value.radicand)
      || (value.value.index !== null && containsNanError(value.value.index));
    case 'negative':
    case 'plus_minus': return containsNanError(value.value);
    case 'binary': return containsNanError(value.value.left) || containsNanError(value.value.right);
    case 'tuple': return value.value.some(containsNanError);
    case 'empty':
    case 'integer':
    case 'exact_decimal':
    case 'variable': return false;
  }
}

const GRADE_WARNING_CODES: readonly GradeWarningCode[] = DRILL_CORE_CONTRACT.grade_warning_codes;

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

function gradeItemFromWasm(problemId: string, inputInterface: AnswerInputInterface, value: unknown): { problem_id: string; answer: string | null; correct: boolean; warnings: readonly GradeWarningCode[] } {
  const data = unwrapEnvelope(value);
  if (!isRecord(data) || typeof data.is_correct !== 'boolean') invalidDto('WASM returned an invalid grade DTO.', value);
  assertAnswerNode(data.expected);
  assertAnswerNode(data.actual);
  assertAnswerSupportsInputInterface(data.expected, inputInterface);
  assertAnswerSupportsInputInterface(data.actual, inputInterface);
  if (data.status !== 'correct' && data.status !== 'incorrect' && data.status !== 'unanswered') {
    invalidDto('WASM returned an unsupported grade status.', data.status);
  }
  const actualIsEmpty = data.actual.type === 'empty';
  const expectedStatus = actualIsEmpty ? (data.is_correct ? null : 'unanswered') : data.is_correct ? 'correct' : 'incorrect';
  const statusConsistent = expectedStatus !== null && data.status === expectedStatus;
  if (!statusConsistent) {
    invalidDto('WASM returned an inconsistent grade status.', data);
  }
  if (data.is_correct && (containsNanError(data.expected) || containsNanError(data.actual))) {
    invalidDto('WASM marked a nan_error answer as correct.', data);
  }
  if (!hasCanonicalGradeWarnings(data.warnings)) {
    invalidDto('WASM returned invalid grade warnings.', data.warnings);
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
        if (settings.schema_version !== DRILL_SCHEMA_VERSION) {
          invalidDto('The worksheet request used an unsupported schema version.', settings);
        }
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
        assertAnswerSupportsInputInterface(data, inputInterface);
        return data;
      } catch (error) {
        throw mapBoundaryError(error);
      }
    },

    async gradeAnswer(request: GradeRequest) {
      try {
        if (request.schema_version !== DRILL_SCHEMA_VERSION || request.worksheet.schema_version !== DRILL_SCHEMA_VERSION) {
          invalidDto('The grade request used an unsupported schema version.', request);
        }
        const gradeAnswer = resolveRuntime(runtime).grade_answer;
        if (!gradeAnswer) throw new DrillEngineError('wasm_unavailable', 'drill-wasm does not expose grade_answer.');
        const items = await Promise.all(request.worksheet.problems.map(async (problem) => {
          assertInputInterface(problem.input_interface);
          assertAnswerNode(problem.canonical_answer);
          assertAnswerSupportsInputInterface(problem.canonical_answer, problem.input_interface);
          const answer = request.answers.find((entry) => entry.problem_id === problem.problem_id)?.answer;
          if (answer) {
            assertAnswerNode(answer);
            assertAnswerSupportsInputInterface(answer, problem.input_interface);
          }
          const value = await invokeBoundary(gradeAnswer, {
            schema_version: DRILL_SCHEMA_VERSION,
            expected: problem.canonical_answer,
            actual: answer ?? { type: 'empty' },
            answer_schema: problem.answer_schema,
          });
          return gradeItemFromWasm(problem.problem_id, problem.input_interface, value);
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
