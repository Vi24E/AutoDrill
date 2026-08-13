import { QUADRATIC_EQUATION_2_CURRICULUM_PATH, QUADRATIC_EQUATION_2_GENERATOR_REVISION, QUADRATIC_EQUATION_2_LAYOUT, QUADRATIC_EQUATION_2_SKILL_ID, QUADRATIC_EQUATION_2_THEME_ID } from '../drill-engine';
import { QUADRATIC_INPUT_INTERFACE, type ThemeDefinition } from './theme-definition';

export const QUADRATIC_EQUATION_2_DEFINITION: ThemeDefinition = {
  numeric_theme_id: QUADRATIC_EQUATION_2_THEME_ID, generator_revision: QUADRATIC_EQUATION_2_GENERATOR_REVISION,
  themeKey: 'jp.grade9.equation.quadratic.2', label: '二次方程式(2)', grade: { slug: 'grade-9', label: '中学3年生' },
  gradeGenre: { genreKey: 'quadratic-equation', label: '二次方程式' }, recommendedGenre: { genreKey: 'equation', label: '方程式' },
  problemCount: QUADRATIC_EQUATION_2_LAYOUT.problem_count, layout: QUADRATIC_EQUATION_2_LAYOUT,
  route: { gradeSlug: 'grade-9', themeSlug: 'quadratic-equation-2', pathname: '/drills/grade-9/quadratic-equation-2' },
  search: { title: '二次方程式(2) | AutoDrill', description: '中学3年生向けの因数分解で解く二次方程式ドリルです。' },
  compatibility: { skillId: QUADRATIC_EQUATION_2_SKILL_ID, curriculumPath: QUADRATIC_EQUATION_2_CURRICULUM_PATH },
  promptKind: 'quadratic_equation', answerSchemaKind: 'algebraic', inputInterface: QUADRATIC_INPUT_INTERFACE,
  worksheet: { title: '二次方程式(2)', instruction: '次の二次方程式を解きなさい。', answerPrefix: 'x =' },
};
