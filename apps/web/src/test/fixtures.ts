import {
  ADDITION_CURRICULUM_PATH,
  ADDITION_GENERATOR_VERSION,
  ADDITION_LAYOUT,
  DRILL_SCHEMA_VERSION,
  emptyEditorState,
  type DrillEngine,
  type DrillSettings,
  type EditorAction,
  type EditorState,
  type GradeRequest,
  type GradeResult,
  type ProblemDto,
  type WorksheetDto,
} from '@/domain/drill-engine';

export function fixtureSettings(): DrillSettings {
  return {
    schema_version: DRILL_SCHEMA_VERSION,
    skill_id: 'jp.grade1.addition.one_digit.1',
    curriculum_path: ADDITION_CURRICULUM_PATH,
    generator_version: ADDITION_GENERATOR_VERSION,
    layout: ADDITION_LAYOUT,
    seed: 'fixtureSeed',
  };
}

export function fixtureWorksheet(): WorksheetDto {
  const problems: ProblemDto[] = Array.from({ length: 20 }, (_, index) => {
    const left = (index % 9) + 1;
    const right = (Math.floor(index / 9) % 9) + 1;
    return {
      schema_version: DRILL_SCHEMA_VERSION,
      problem_id: `fixture-${index + 1}`,
      skill_id: 'jp.grade1.addition.one_digit.1',
      left,
      right,
      prompt: { kind: 'addition', left, right },
      answer_schema: { kind: 'integer', min: 1, max: 18 },
      canonical_answer: { kind: 'integer', value: left + right },
      operation_counts: { additions: 1, carries: left + right >= 10 ? 1 : 0 },
    };
  });
  return {
    schema_version: DRILL_SCHEMA_VERSION,
    generator_version: ADDITION_GENERATOR_VERSION,
    skill_id: 'jp.grade1.addition.one_digit.1',
    curriculum_path: ADDITION_CURRICULUM_PATH,
    seed: 'fixtureSeed',
    layout: ADDITION_LAYOUT,
    problems,
  };
}

function applyFixtureEditor(state: EditorState, action: EditorAction): EditorState {
  const digits = [...state.node.digits];
  let cursor = state.cursor;
  if (action.kind === 'insert_digit' && digits.length < 2) {
    digits.splice(cursor, 0, action.digit);
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
  return { schema_version: 1, node: { kind: 'integer', digits }, cursor, committed: action.kind === 'commit' };
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
        const digits = answer.node.digits.join('');
        const value = digits.length > 0 ? Number(digits) : null;
        return { problem_id: problem.problem_id, answer: value, correct: value === problem.canonical_answer.value };
      });
      return { schema_version: 1, items, correct_count: items.filter((item) => item.correct).length, total_count: items.length };
    },
  };
}
