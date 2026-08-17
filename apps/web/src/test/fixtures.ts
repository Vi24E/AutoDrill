import {
  DRILL_OPERATION_KIND_COUNT,
  DRILL_SCHEMA_VERSION,
  DrillEngineError,
  type AnswerNode,
  type DrillEngine,
  type DrillSettings,
  type EditorAction,
  type EditorState,
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
      answer_schema: { kind: 'integer', min: '1', max: '18' },
      canonical_answer: answer,
      solution_graph: { steps: [] },
      operation_vector: { values: Array.from({ length: DRILL_OPERATION_KIND_COUNT }, () => 0) },
      effort: 0,
    };
  });
  return {
    schema_version: DRILL_SCHEMA_VERSION,
    problem_set_id: `${DRILL_SCHEMA_VERSION}-1-${ONE_DIGIT_ADDITION_DEFINITION.generator_revision}-fixtureSeed-3`,
    identity: {
      schema_version: DRILL_SCHEMA_VERSION,
      numeric_theme_id: FIXTURE_THEME_ID,
      generator_revision: ONE_DIGIT_ADDITION_DEFINITION.generator_revision,
      seed: FIXTURE_SEED,
      difficulty: FIXTURE_DIFFICULTY,
    },
    skill_id: ONE_DIGIT_ADDITION_DEFINITION.compatibility.skillId,
    curriculum_path: ONE_DIGIT_ADDITION_DEFINITION.compatibility.curriculumPath.map((segment) => segment.label),
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
      answer_schema: themeId === 2
        ? { kind: 'integer', min: '-15', max: '15' }
        : { kind: 'rational', max_abs_numerator: 20, max_denominator: 12, require_reduced_fraction_form: true },
      canonical_answer: canonicalAnswer,
      solution_graph: { steps: [] },
      operation_vector: { values: Array.from({ length: DRILL_OPERATION_KIND_COUNT }, () => 0) },
      effort: 0,
    };
  });
  return {
    schema_version: DRILL_SCHEMA_VERSION,
    problem_set_id: `${DRILL_SCHEMA_VERSION}-${themeId}-${definition.generator_revision}-fixtureSeed-3`,
    identity: {
      schema_version: DRILL_SCHEMA_VERSION,
      numeric_theme_id: themeId,
      generator_revision: definition.generator_revision,
      seed: FIXTURE_SEED,
      difficulty: FIXTURE_DIFFICULTY,
    },
    skill_id: definition.compatibility.skillId,
    curriculum_path: definition.compatibility.curriculumPath.map((segment) => segment.label),
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
      answer_schema: { kind: 'ordered_pair', min: '-15', max: '15' },
      canonical_answer: { type: 'tuple', value: [{ type: 'integer', value: String(x) }, { type: 'integer', value: String(y) }] },
      solution_graph: { steps: [] },
      operation_vector: { values: Array.from({ length: DRILL_OPERATION_KIND_COUNT }, () => 0) },
      effort: 0,
    };
  });
  return {
    schema_version: DRILL_SCHEMA_VERSION,
    problem_set_id: `${DRILL_SCHEMA_VERSION}-${definition.numeric_theme_id}-${definition.generator_revision}-fixtureSeed-3`,
    identity: {
      schema_version: DRILL_SCHEMA_VERSION,
      numeric_theme_id: definition.numeric_theme_id,
      generator_revision: definition.generator_revision,
      seed: FIXTURE_SEED,
      difficulty: FIXTURE_DIFFICULTY,
    },
    skill_id: definition.compatibility.skillId,
    curriculum_path: definition.compatibility.curriculumPath.map((segment) => segment.label),
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
    { kind: 'implication' as const, antecedent_person: 1, antecedent_is_liar: true, consequent_person: 3, consequent_is_liar: false },
  ];
  const problems: ProblemDto[] = Array.from({ length: definition.problemCount }, (_, index) => ({
    schema_version: DRILL_SCHEMA_VERSION,
    id: index + 1,
    problem_id: String(index + 1),
    numeric_theme_id: definition.numeric_theme_id,
    prompt: { kind: 'liar_puzzle', people_count: 4, statements },
    input_interface: definition.inputInterface,
    answer_schema: { kind: 'algebraic' },
    canonical_answer: { type: 'tuple', value: [{ type: 'integer', value: '1' }, { type: 'integer', value: '3' }] },
    solution_graph: { steps: [] },
    operation_vector: { values: Array.from({ length: DRILL_OPERATION_KIND_COUNT }, () => 0) },
    effort: 0,
  }));
  return {
    schema_version: DRILL_SCHEMA_VERSION,
    problem_set_id: `${DRILL_SCHEMA_VERSION}-${definition.numeric_theme_id}-${definition.generator_revision}-fixtureSeed-2`,
    identity: { schema_version: DRILL_SCHEMA_VERSION, numeric_theme_id: definition.numeric_theme_id, generator_revision: definition.generator_revision, seed: FIXTURE_SEED, difficulty: 2 },
    skill_id: definition.compatibility.skillId,
    curriculum_path: definition.compatibility.curriculumPath.map((segment) => segment.label),
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
    answer_schema: { kind: 'ordered_tuple', length: 16 },
    canonical_answer: {
      type: 'tuple',
      value: solution.map((value) => ({ type: 'integer' as const, value: String(value) })),
    },
    solution_graph: { steps: [] },
    operation_vector: { values: Array.from({ length: DRILL_OPERATION_KIND_COUNT }, () => 0) },
    theme_specific_effort: 4.2,
    effort: 4.2,
  }));
  return {
    schema_version: DRILL_SCHEMA_VERSION,
    problem_set_id: `${DRILL_SCHEMA_VERSION}-${definition.numeric_theme_id}-${definition.generator_revision}-fixtureSeed-3`,
    identity: {
      schema_version: DRILL_SCHEMA_VERSION,
      numeric_theme_id: definition.numeric_theme_id,
      generator_revision: definition.generator_revision,
      seed: FIXTURE_SEED,
      difficulty: FIXTURE_DIFFICULTY,
    },
    skill_id: definition.compatibility.skillId,
    curriculum_path: definition.compatibility.curriculumPath.map((segment) => segment.label),
    layout: definition.layout,
    seed: FIXTURE_SEED,
    problems,
  };
}

function answerDigits(answer: AnswerNode): string {
  return answer.type === 'integer' ? String(answer.value) : '';
}

function applyFixtureEditor(state: EditorState, action: EditorAction, _inputInterface: ProblemDto['input_interface']): EditorState {
  const digits = [...answerDigits(state.answer)];
  let cursor = Math.min(state.cursor, digits.length);
  if (action.type === 'insert_digit' && digits.length >= 18) {
    throw new DrillEngineError('answer_ast_size_limit', 'fixture answer AST size limit', { max_size: 18 });
  }
  if (action.type === 'insert_digit') {
    digits.splice(cursor, 0, String(action.digit));
    cursor += 1;
  } else if (action.type === 'backspace' && cursor > 0) {
    digits.splice(cursor - 1, 1);
    cursor -= 1;
  } else if (action.type === 'delete') {
    digits.splice(cursor, 1);
  } else if (action.type === 'move_left') {
    cursor = Math.max(0, cursor - 1);
  } else if (action.type === 'move_right') {
    cursor = Math.min(digits.length, cursor + 1);
  } else if (action.type === 'clear') {
    digits.length = 0;
    cursor = 0;
  }
  const normalized = digits.join('').replace(/^0+(?=\d)/, '');
  return {
    answer: normalized.length > 0
      ? { type: 'integer', value: normalized }
      : { type: 'empty' },
    cursor: Math.min(cursor, normalized.length),
    active_path: state.active_path,
    committed: action.type === 'commit',
  };
}

export function fixtureEngine(worksheet = fixtureWorksheet()): DrillEngine {
  return {
    async generateWorksheet() {
      return worksheet;
    },
    async applyEditorAction(state, action, inputInterface) {
      return applyFixtureEditor(state, action, inputInterface);
    },
    async parseMathLiveAnswer(latex) {
      if (latex === '') return { type: 'empty' };
      if (/^\d+$/.test(latex)) {
        if (latex.length > 18) throw new DrillEngineError('answer_ast_size_limit', 'Answer is too large.');
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
