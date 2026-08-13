import {
  SIMULTANEOUS_EQUATION_1_CURRICULUM_PATH,
  SIMULTANEOUS_EQUATION_1_GENERATOR_REVISION,
  SIMULTANEOUS_EQUATION_1_LAYOUT,
  SIMULTANEOUS_EQUATION_1_SKILL_ID,
  SIMULTANEOUS_EQUATION_1_THEME_ID,
} from '../drill-engine';
import { type ThemeDefinition } from './theme-definition';

const SIMULTANEOUS_INPUT = {
  type: 'structured_math',
  allowed_structures: ['negative', 'tuple'],
} as const;

export const SIMULTANEOUS_EQUATION_1_DEFINITION: ThemeDefinition = {
  numeric_theme_id: SIMULTANEOUS_EQUATION_1_THEME_ID,
  generator_revision: SIMULTANEOUS_EQUATION_1_GENERATOR_REVISION,
  themeKey: 'jp.grade8.equation.simultaneous.1',
  label: '連立方程式(1)',
  grade: { slug: 'grade-8', label: '中学2年生' },
  gradeGenre: { genreKey: 'simultaneous-equation', label: '連立方程式' },
  recommendedGenre: { genreKey: 'equation', label: '方程式' },
  problemCount: SIMULTANEOUS_EQUATION_1_LAYOUT.problem_count,
  layout: SIMULTANEOUS_EQUATION_1_LAYOUT,
  route: { gradeSlug: 'grade-8', themeSlug: 'simultaneous-equation-1', pathname: '/drills/grade-8/simultaneous-equation-1' },
  search: { title: '連立方程式(1) | AutoDrill', description: '中学2年生向けの整数解をもつ連立方程式ドリルです。' },
  compatibility: { skillId: SIMULTANEOUS_EQUATION_1_SKILL_ID, curriculumPath: SIMULTANEOUS_EQUATION_1_CURRICULUM_PATH },
  promptKind: 'simultaneous_equation',
  answerSchemaKind: 'ordered_pair',
  inputInterface: SIMULTANEOUS_INPUT,
  worksheet: { title: '連立方程式(1)', instruction: '次の連立方程式を解きなさい。', answerPrefix: null },
};
