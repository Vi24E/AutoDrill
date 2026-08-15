import { QUADRATIC_EQUATION_1_CURRICULUM_PATH, QUADRATIC_EQUATION_1_GENERATOR_REVISION, QUADRATIC_EQUATION_1_LAYOUT, QUADRATIC_EQUATION_1_SKILL_ID, QUADRATIC_EQUATION_1_THEME_ID } from '../drill-engine';
import { defineTheme, QUADRATIC_INPUT_INTERFACE } from './theme-definition';

export const QUADRATIC_EQUATION_1_DEFINITION = defineTheme({
  numeric_theme_id: QUADRATIC_EQUATION_1_THEME_ID, generator_revision: QUADRATIC_EQUATION_1_GENERATOR_REVISION,
  themeKey: 'jp.grade9.equation.quadratic.1', label: '二次方程式(1)', grade: { slug: 'grade-9', label: '中学3年生' },
  tags: ['equations', 'quadratic_equation'],
  problemCount: QUADRATIC_EQUATION_1_LAYOUT.problem_count, layout: QUADRATIC_EQUATION_1_LAYOUT,
  route: { gradeSlug: 'grade-9', themeSlug: 'quadratic-equation-1', pathname: '/drills/grade-9/quadratic-equation-1' },
  search: { title: '二次方程式(1) | AutoDrill', description: '中学3年生向けの平方根に帰着できる二次方程式ドリルです。' },
  compatibility: { skillId: QUADRATIC_EQUATION_1_SKILL_ID, curriculumPath: QUADRATIC_EQUATION_1_CURRICULUM_PATH },
  promptKind: 'quadratic_equation', answerSchemaKind: 'algebraic', inputInterface: QUADRATIC_INPUT_INTERFACE,
  worksheet: { title: '二次方程式(1)', instruction: '次の二次方程式を解きなさい。', answerPrefix: 'x =' },
});
