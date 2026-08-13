import { LIAR_PUZZLE_CURRICULUM_PATH, LIAR_PUZZLE_GENERATOR_REVISION, LIAR_PUZZLE_LAYOUT, LIAR_PUZZLE_SKILL_ID, LIAR_PUZZLE_THEME_ID, type AnswerInputInterface } from '../drill-engine';
import type { ThemeDefinition } from './theme-definition';

const LIAR_PUZZLE_INPUT: AnswerInputInterface = { type: 'structured_math', allowed_structures: ['tuple'] };

export const LIAR_PUZZLE_DEFINITION: ThemeDefinition = {
  numeric_theme_id: LIAR_PUZZLE_THEME_ID,
  generator_revision: LIAR_PUZZLE_GENERATOR_REVISION,
  themeKey: 'bonus.logic.liar-puzzle',
  label: 'うそつきだれだ',
  grade: null,
  gradeGenre: null,
  recommendedGenre: { genreKey: 'bonus', label: 'おまけ' },
  problemCount: LIAR_PUZZLE_LAYOUT.problem_count,
  layout: LIAR_PUZZLE_LAYOUT,
  route: { gradeSlug: 'bonus', themeSlug: 'liar-puzzle', pathname: '/drills/bonus/liar-puzzle' },
  search: { title: 'うそつきだれだ | AutoDrill', description: '正直者とうそつきの発言から、うそつきを全員見つける論理クイズです。' },
  compatibility: { skillId: LIAR_PUZZLE_SKILL_ID, curriculumPath: LIAR_PUZZLE_CURRICULUM_PATH },
  promptKind: 'liar_puzzle',
  answerSchemaKind: 'algebraic',
  inputInterface: LIAR_PUZZLE_INPUT,
  worksheet: { title: 'うそつきだれだ', instruction: '正直な人は本当のことを、うそつきはうそのことを言います。うそつきを全員選びなさい。', answerPrefix: null },
};
