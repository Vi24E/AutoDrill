import {
  ADDITION_CURRICULUM_PATH,
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
      left,
      right,
      prompt: { kind: 'addition', left, right },
      answer_schema: { kind: 'integer', min: '1', max: '18' },
      canonical_answer: answer,
      solution_graph: { steps: [] },
      operation_vector: { values: Array.from({ length: 27 }, () => 0) },
      effort: 0,
    };
  });
  return {
    schema_version: DRILL_SCHEMA_VERSION,
    problem_set_id: '2-1-2-fixtureSeed-3',
    identity: {
      schema_version: DRILL_SCHEMA_VERSION,
      numeric_theme_id: FIXTURE_THEME_ID,
      generator_revision: 2,
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

function answerDigits(answer: AnswerNode): string {
  return answer.type === 'integer' ? String(answer.value) : '';
}

function applyFixtureEditor(state: EditorState, action: EditorAction): EditorState {
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
    committed: action.kind === 'commit',
  };
}

export function fixtureEngine(worksheet = fixtureWorksheet()): DrillEngine {
  return {
    async generateWorksheet() {
      return worksheet;
    },
    async applyEditorAction(state, action) {
      return applyFixtureEditor(state, action);
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
