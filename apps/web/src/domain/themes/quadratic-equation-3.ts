import { QUADRATIC_EQUATION_3_CURRICULUM_PATH, QUADRATIC_EQUATION_3_GENERATOR_REVISION, QUADRATIC_EQUATION_3_LAYOUT, QUADRATIC_EQUATION_3_SKILL_ID, QUADRATIC_EQUATION_3_THEME_ID } from '../drill-engine';
import { defineTheme, QUADRATIC_INPUT_INTERFACE } from './theme-definition';

export const QUADRATIC_EQUATION_3_DEFINITION = defineTheme({
  numeric_theme_id: QUADRATIC_EQUATION_3_THEME_ID, generator_revision: QUADRATIC_EQUATION_3_GENERATOR_REVISION,
  themeKey: 'jp.grade9.equation.quadratic.3', label: '二次方程式(3)', grade: { slug: 'grade-9', label: '中学3年生' },
  tags: ['equations', 'quadratic_equation'],
  problemCount: QUADRATIC_EQUATION_3_LAYOUT.problem_count, layout: QUADRATIC_EQUATION_3_LAYOUT,
  route: { gradeSlug: 'grade-9', themeSlug: 'quadratic-equation-3', pathname: '/drills/grade-9/quadratic-equation-3' },
  search: { title: '二次方程式(3) | AutoDrill', description: '中学3年生向けの解の公式を使う二次方程式ドリルです。' },
  compatibility: { skillId: QUADRATIC_EQUATION_3_SKILL_ID, curriculumPath: QUADRATIC_EQUATION_3_CURRICULUM_PATH },
  promptKind: 'quadratic_equation', answerSchemaKind: 'algebraic', inputInterface: QUADRATIC_INPUT_INTERFACE,
  worksheet: { title: '二次方程式(3)', instruction: '次の二次方程式を解きなさい。必要なら解の公式を使いなさい。', answerPrefix: 'x =' },
});
