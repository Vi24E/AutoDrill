import {
  ADDITION_CURRICULUM_PATH,
  ADDITION_GENERATOR_REVISION,
  ADDITION_LAYOUT,
  DRILL_SCHEMA_VERSION,
  DrillEngineError,
  emptyEditorState,
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
import { ALL_MATH_STRUCTURES, LINEAR_EQUATION_1_DEFINITION, LINEAR_EQUATION_2_DEFINITION } from '@/domain/theme-registry';

const FIXTURE_SEED = 'fixtureSeed';
const FIXTURE_THEME_ID = 1;
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
      operation_vector: { values: Array.from({ length: 27 }, () => 0) },
      effort: 0,
    };
  });
  return {
    schema_version: DRILL_SCHEMA_VERSION,
    problem_set_id: `3-1-${ADDITION_GENERATOR_REVISION}-fixtureSeed-3`,
    identity: {
      schema_version: DRILL_SCHEMA_VERSION,
      numeric_theme_id: FIXTURE_THEME_ID,
      generator_revision: ADDITION_GENERATOR_REVISION,
      seed: FIXTURE_SEED,
      difficulty: FIXTURE_DIFFICULTY,
    },
    skill_id: 'jp.grade1.addition.one_digit',
    curriculum_path: ADDITION_CURRICULUM_PATH.map((segment) => segment.label),
    layout: ADDITION_LAYOUT,
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
      input_interface: {
        type: 'structured_math',
        allowed_structures: ALL_MATH_STRUCTURES,
      },
      answer_schema: themeId === 2
        ? { kind: 'integer', min: '-15', max: '15' }
        : { kind: 'rational', max_abs_numerator: 20, max_denominator: 12, require_reduced_fraction_form: true },
      canonical_answer: canonicalAnswer,
      solution_graph: { steps: [] },
      operation_vector: { values: Array.from({ length: 27 }, () => 0) },
      effort: 0,
    };
  });
  return {
    schema_version: DRILL_SCHEMA_VERSION,
    problem_set_id: `3-${themeId}-${definition.generator_revision}-fixtureSeed-3`,
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

function answerDigits(answer: AnswerNode): string {
  return answer.type === 'integer' ? String(answer.value) : '';
}

function applyFixtureEditor(state: EditorState, action: EditorAction, _inputInterface: ProblemDto['input_interface']): EditorState {
  const digits = [...answerDigits(state.answer)];
  let cursor = Math.min(state.cursor, digits.length);
  if (action.kind === 'insert_digit' && digits.length >= 18) {
    throw new DrillEngineError('answer_ast_size_limit', 'fixture answer AST size limit', { max_size: 18 });
  }
  if (action.kind === 'insert_digit') {
    digits.splice(cursor, 0, String(action.digit));
    cursor += 1;
  } else if (action.kind === 'delete_backward' && cursor > 0) {
    digits.splice(cursor - 1, 1);
    cursor -= 1;
  } else if (action.kind === 'delete_forward') {
    digits.splice(cursor, 1);
  } else if (action.kind === 'move_left') {
    cursor = Math.max(0, cursor - 1);
  } else if (action.kind === 'move_right') {
    cursor = Math.min(digits.length, cursor + 1);
  } else if (action.kind === 'clear') {
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
    committed: action.kind === 'commit',
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
    async gradeAnswer(request: GradeRequest): Promise<GradeResult> {
      const items = request.worksheet.problems.map((problem) => {
        const answer = request.answers.find((entry) => entry.problem_id === problem.problem_id)?.editor_state ?? emptyEditorState();
        const value = answer.answer.type === 'integer' ? answer.answer.value : null;
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
