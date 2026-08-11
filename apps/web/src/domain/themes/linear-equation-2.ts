import { LINEAR_EQUATION_2_CURRICULUM_PATH, LINEAR_EQUATION_2_GENERATOR_REVISION, LINEAR_EQUATION_2_LAYOUT, LINEAR_EQUATION_2_SKILL_ID, LINEAR_EQUATION_2_THEME_ID } from '../drill-engine';
import { LINEAR_INPUT_INTERFACE, LINEAR_INSTRUCTION, type ThemeDefinition } from './theme-definition';

export const LINEAR_EQUATION_2_DEFINITION: ThemeDefinition = {
  numeric_theme_id: LINEAR_EQUATION_2_THEME_ID, generator_revision: LINEAR_EQUATION_2_GENERATOR_REVISION,
  themeKey: 'jp.grade7.equation.linear.2', label: '一次方程式(2)', grade: { slug: 'grade-7', label: '中学1年生' },
  gradeGenre: { genreKey: 'linear-equation', label: '一次方程式' }, recommendedGenre: { genreKey: 'equation', label: '方程式' },
  problemCount: LINEAR_EQUATION_2_LAYOUT.problem_count, layout: LINEAR_EQUATION_2_LAYOUT,
  route: { gradeSlug: 'grade-7', themeSlug: 'linear-equation-2', pathname: '/drills/grade-7/linear-equation-2' },
  search: { title: '一次方程式(2) | AutoDrill', description: '中学1年生向けの分数係数・分数解を含む一次方程式ドリルです。' },
  compatibility: { skillId: LINEAR_EQUATION_2_SKILL_ID, curriculumPath: LINEAR_EQUATION_2_CURRICULUM_PATH },
  promptKind: 'linear_equation', answerSchemaKind: 'rational', inputInterface: LINEAR_INPUT_INTERFACE,
  worksheet: { title: '一次方程式(2)', instruction: LINEAR_INSTRUCTION, answerPrefix: 'x =' },
};
