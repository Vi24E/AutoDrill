import {
  DRILL_SCHEMA_VERSION,
  DrillEngineError,
  type AnswerNode,
  type DrillEngine,
  type DrillSettings,
  type GradeRequest,
  type GradeResult,
  type ProblemDto,
  type WorksheetDto,
} from '@/domain/drill-engine';
import { LIAR_PUZZLE_DEFINITION } from '@/domain/themes/liar-puzzle';
import { MINI_SUDOKU_DEFINITION } from '@/domain/themes/mini-sudoku';
import { ONE_DIGIT_ADDITION_DEFINITION } from '@/domain/themes/one-digit-addition';
import { LINEAR_EQUATION_1_DEFINITION } from '@/domain/themes/linear-equation-1';
import { LINEAR_EQUATION_2_DEFINITION } from '@/domain/themes/linear-equation-2';
import { SIMULTANEOUS_EQUATION_1_DEFINITION } from '@/domain/themes/simultaneous-equation-1';
import { COLUMN_DIVIDE_1DIGIT_DEFINITION } from '@/domain/themes/column-divide-one-digit';
import { COLUMN_DIVIDE_2DIGIT_DEFINITION } from '@/domain/themes/column-divide-two-digit';
import { COLUMN_DECIMAL_MULTIPLICATION_DEFINITION } from '@/domain/themes/column-decimal-multiplication';
import { DRILL_CORE_CONTRACT } from '@/generated/drill-core-contract';

const FIXTURE_SEED = 'fixtureSeed';
const FIXTURE_THEME_ID = ONE_DIGIT_ADDITION_DEFINITION.numeric_theme_id;
const FIXTURE_DIFFICULTY = 3 as const;

export function fixtureSettings(): DrillSettings {
  return {
    schema_version: DRILL_SCHEMA_VERSION,
    numeric_theme_id: FIXTURE_THEME_ID,
    difficulty: FIXTURE_DIFFICULTY,
    seed: FIXTURE_SEED,
  };
}

export function fixtureWorksheet(): WorksheetDto {
  const problems: ProblemDto[] = Array.from({ length: 20 }, (_, index) => {
    const left = (index % 9) + 1;
    const right = (Math.floor(index / 9) % 9) + 1;
    const answer: AnswerNode = { type: 'integer', value: String(left + right) };
    return {
      schema_version: DRILL_SCHEMA_VERSION,
      id: index + 1,
      problem_id: String(index + 1),
      numeric_theme_id: FIXTURE_THEME_ID,
      prompt: { kind: 'addition', left, right },
      input_interface: { type: 'simple_numeric', allow_decimal: false, allow_negative: false },
      column_input: null,
      answer_schema: { kind: 'integer', min: '1', max: '18' },
      canonical_answer: answer,
      worked_solution: null,
    };
  });
  return {
    schema_version: DRILL_SCHEMA_VERSION,
    identity: {
      schema_version: DRILL_SCHEMA_VERSION,
      numeric_theme_id: FIXTURE_THEME_ID,
      generator_revision: ONE_DIGIT_ADDITION_DEFINITION.generator_revision,
      seed: FIXTURE_SEED,
      difficulty: FIXTURE_DIFFICULTY,
    },
    layout: ONE_DIGIT_ADDITION_DEFINITION.layout,
    seed: FIXTURE_SEED,
    problems,
  };
}

export function linearFixtureWorksheet(themeId: 2 | 3 = 2): WorksheetDto {
  const definition = themeId === 2 ? LINEAR_EQUATION_1_DEFINITION : LINEAR_EQUATION_2_DEFINITION;
  const problems: ProblemDto[] = Array.from({ length: 16 }, (_, index) => {
    const solution = themeId === 2 ? (index % 11) - 5 : null;
    const canonicalAnswer: AnswerNode = themeId === 2
      ? { type: 'integer', value: String(solution) }
      : { type: 'fraction', value: { numerator: { type: 'integer', value: '1' }, denominator: { type: 'integer', value: '2' } } };
    const a = { numerator: 2, denominator: 1 };
    const b = { numerator: 0, denominator: 1 };
    const c = themeId === 2 ? { numerator: 1, denominator: 1 } : { numerator: 0, denominator: 1 };
    const d = themeId === 2
      ? { numerator: solution!, denominator: 1 }
      : { numerator: 1, denominator: 1 };
    return {
      schema_version: DRILL_SCHEMA_VERSION,
      id: index + 1,
      problem_id: String(index + 1),
      numeric_theme_id: themeId,
      prompt: {
        kind: 'linear_equation',
        a,
        b,
        c,
        d,
        left_negative_constant_as_subtraction: false,
        right_negative_constant_as_subtraction: false,
      },
      input_interface: definition.inputInterface,
      column_input: null,
      answer_schema: themeId === 2
        ? { kind: 'integer', min: '-15', max: '15' }
        : { kind: 'rational', max_abs_numerator: 20, max_denominator: 12, require_reduced_fraction_form: true },
      canonical_answer: canonicalAnswer,
      worked_solution: null,
    };
  });
  return {
    schema_version: DRILL_SCHEMA_VERSION,
    identity: {
      schema_version: DRILL_SCHEMA_VERSION,
      numeric_theme_id: themeId,
      generator_revision: definition.generator_revision,
      seed: FIXTURE_SEED,
      difficulty: FIXTURE_DIFFICULTY,
    },
    layout: definition.layout,
    seed: FIXTURE_SEED,
    problems,
  };
}

export function simultaneousFixtureWorksheet(): WorksheetDto {
  const definition = SIMULTANEOUS_EQUATION_1_DEFINITION;
  const problems: ProblemDto[] = Array.from({ length: definition.problemCount }, (_, index) => {
    const x = (index % 7) - 3;
    const y = (index % 5) - 2;
    const a = 1;
    const b = 1;
    const d = 1;
    const e = -1;
    return {
      schema_version: DRILL_SCHEMA_VERSION,
      id: index + 1,
      problem_id: String(index + 1),
      numeric_theme_id: definition.numeric_theme_id,
      prompt: { kind: 'simultaneous_equation', a, b, c: a * x + b * y, d, e, f: d * x + e * y },
      input_interface: definition.inputInterface,
      column_input: null,
      answer_schema: { kind: 'ordered_pair', min: '-15', max: '15' },
      canonical_answer: { type: 'tuple', value: [{ type: 'integer', value: String(x) }, { type: 'integer', value: String(y) }] },
      worked_solution: null,
    };
  });
  return {
    schema_version: DRILL_SCHEMA_VERSION,
    identity: {
      schema_version: DRILL_SCHEMA_VERSION,
      numeric_theme_id: definition.numeric_theme_id,
      generator_revision: definition.generator_revision,
      seed: FIXTURE_SEED,
      difficulty: FIXTURE_DIFFICULTY,
    },
    layout: definition.layout,
    seed: FIXTURE_SEED,
    problems,
  };
}

export function columnDivisionFixtureWorksheet(themeId: 31 | 32 = 31): WorksheetDto {
  const definition = themeId === 31 ? COLUMN_DIVIDE_1DIGIT_DEFINITION : COLUMN_DIVIDE_2DIGIT_DEFINITION;
  const divisor = themeId === 31 ? 7 : 23;
  const dividend = themeId === 31 ? 224 : 1245;
  const quotient = themeId === 31 ? 32 : 54;
  const remainder = themeId === 31 ? 0 : 3;
  const problems: ProblemDto[] = Array.from({ length: definition.problemCount }, (_, index) => ({
    schema_version: DRILL_SCHEMA_VERSION,
    id: index + 1,
    problem_id: String(index + 1),
    numeric_theme_id: definition.numeric_theme_id,
    prompt: {
      kind: 'column_arithmetic',
      operator: 'divide',
      left: { kind: 'integer', value: dividend },
      right: { kind: 'integer', value: divisor },
    },
    input_interface: definition.inputInterface,
    column_input: {
      single: null,
      quotient: { order: 'natural_division_flow', decimal_point: { type: 'none' } },
      remainder: { order: 'big_endian', decimal_point: { type: 'none' } },
    },
    answer_schema: { kind: 'ordered_pair' },
    canonical_answer: {
      type: 'tuple',
      value: [{ type: 'integer', value: String(quotient) }, { type: 'integer', value: String(remainder) }],
    },
    worked_solution: {
      kind: 'long_division',
      divisor,
      dividend_coefficient: dividend,
      dividend_scale: 0,
      quotient_trailing_cells: 0,
      steps: themeId === 31
        ? [
            { product: 21, after: 14, product_offset: 1, after_offset: 0 },
            { product: 14, after: 0, product_offset: 0, after_offset: 0 },
          ]
        : [
            { product: 115, after: 95, product_offset: 1, after_offset: 0 },
            { product: 92, after: 3, product_offset: 0, after_offset: 0 },
          ],
    },
  }));
  return {
    schema_version: DRILL_SCHEMA_VERSION,
    identity: {
      schema_version: DRILL_SCHEMA_VERSION,
      numeric_theme_id: definition.numeric_theme_id,
      generator_revision: definition.generator_revision,
      seed: FIXTURE_SEED,
      difficulty: FIXTURE_DIFFICULTY,
    },
    layout: definition.layout,
    seed: FIXTURE_SEED,
    problems,
  };
}

export function columnDecimalMultiplicationFixtureWorksheet(): WorksheetDto {
  const definition = COLUMN_DECIMAL_MULTIPLICATION_DEFINITION;
  const problems: ProblemDto[] = Array.from({ length: definition.problemCount }, (_, index) => ({
    schema_version: DRILL_SCHEMA_VERSION,
    id: index + 1,
    problem_id: String(index + 1),
    numeric_theme_id: definition.numeric_theme_id,
    prompt: {
      kind: 'column_arithmetic',
      operator: 'multiply',
      left: { kind: 'exact_decimal', coefficient: 12, scale: 1 },
      right: { kind: 'exact_decimal', coefficient: 3, scale: 1 },
    },
    input_interface: definition.inputInterface,
    column_input: {
      single: { order: 'least_significant_first', decimal_point: { type: 'editable' } },
      quotient: null,
      remainder: null,
    },
    answer_schema: { kind: 'decimal', max_scale: 6 },
    canonical_answer: { type: 'exact_decimal', value: { coefficient: '36', scale: 2 } },
    worked_solution: { kind: 'column_multiplication', partial_products: [{ value: 36, place: 0 }] },
  }));
  return {
    schema_version: DRILL_SCHEMA_VERSION,
    identity: {
      schema_version: DRILL_SCHEMA_VERSION,
      numeric_theme_id: definition.numeric_theme_id,
      generator_revision: definition.generator_revision,
      seed: FIXTURE_SEED,
      difficulty: FIXTURE_DIFFICULTY,
    },
    layout: definition.layout,
    seed: FIXTURE_SEED,
    problems,
  };
}

export function liarFixtureWorksheet(): WorksheetDto {
  const definition = LIAR_PUZZLE_DEFINITION;
  const statements = [
    { kind: 'says_liar' as const, person: 2 },
    { kind: 'exact_liar_count' as const, count: 2 },
    { kind: 'both_not_liar' as const, first: 2, second: 4 },
    { kind: 'says_not_liar' as const, person: 3 },
  ];
  const problems: ProblemDto[] = Array.from({ length: definition.problemCount }, (_, index) => ({
    schema_version: DRILL_SCHEMA_VERSION,
    id: index + 1,
    problem_id: String(index + 1),
    numeric_theme_id: definition.numeric_theme_id,
    prompt: { kind: 'liar_puzzle', people_count: 4, statements },
    input_interface: definition.inputInterface,
    column_input: null,
    answer_schema: { kind: 'algebraic' },
    canonical_answer: { type: 'tuple', value: [{ type: 'integer', value: '1' }, { type: 'integer', value: '3' }] },
    worked_solution: null,
  }));
  return {
    schema_version: DRILL_SCHEMA_VERSION,
    identity: { schema_version: DRILL_SCHEMA_VERSION, numeric_theme_id: definition.numeric_theme_id, generator_revision: definition.generator_revision, seed: FIXTURE_SEED, difficulty: 2 },
    layout: definition.layout,
    seed: FIXTURE_SEED,
    problems,
  };
}

export function miniSudokuFixtureWorksheet(): WorksheetDto {
  const definition = MINI_SUDOKU_DEFINITION;
  const solution = [1, 2, 3, 4, 3, 4, 1, 2, 2, 1, 4, 3, 4, 3, 2, 1];
  const givens = [1, null, null, 4, null, 4, 1, null, null, 1, 4, null, 4, null, null, 1];
  const problems: ProblemDto[] = Array.from({ length: definition.problemCount }, (_, index) => ({
    schema_version: DRILL_SCHEMA_VERSION,
    id: index + 1,
    problem_id: String(index + 1),
    numeric_theme_id: definition.numeric_theme_id,
    prompt: { kind: 'mini_sudoku', givens },
    input_interface: definition.inputInterface,
    column_input: null,
    answer_schema: { kind: 'ordered_tuple', length: 16 },
    canonical_answer: {
      type: 'tuple',
      value: solution.map((value) => ({ type: 'integer' as const, value: String(value) })),
    },
    worked_solution: null,
  }));
  return {
    schema_version: DRILL_SCHEMA_VERSION,
    identity: {
      schema_version: DRILL_SCHEMA_VERSION,
      numeric_theme_id: definition.numeric_theme_id,
      generator_revision: definition.generator_revision,
      seed: FIXTURE_SEED,
      difficulty: FIXTURE_DIFFICULTY,
    },
    layout: definition.layout,
    seed: FIXTURE_SEED,
    problems,
  };
}

export function fixtureEngine(worksheet = fixtureWorksheet()): DrillEngine {
  return {
    async generateWorksheet() {
      return worksheet;
    },
    async parseMathLiveAnswer(latex) {
      if (latex === '') return { type: 'empty' };
      if (/^\d+$/.test(latex)) {
        if (latex.length > DRILL_CORE_CONTRACT.max_answer_ast_size) throw new DrillEngineError('answer_ast_size_limit', 'Answer is too large.');
        return { type: 'integer', value: String(BigInt(latex)) };
      }
      return { type: 'nan_error', value: latex };
    },
    async gradeAnswer(request: GradeRequest): Promise<GradeResult> {
      const items = request.worksheet.problems.map((problem) => {
        const answer = request.answers.find((entry) => entry.problem_id === problem.problem_id)?.answer ?? { type: 'empty' };
        const value = answer.type === 'integer' ? answer.value : null;
        const expected = problem.canonical_answer.type === 'integer' ? problem.canonical_answer.value : null;
        return { problem_id: problem.problem_id, answer: value, correct: value !== null && value === expected, warnings: [] };
      });
      return {
        schema_version: DRILL_SCHEMA_VERSION,
        items,
        correct_count: items.filter((item) => item.correct).length,
        total_count: items.length,
      };
    },
  };
}
