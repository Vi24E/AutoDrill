import { LINEAR_EQUATION_1_CURRICULUM_PATH, LINEAR_EQUATION_1_GENERATOR_REVISION, LINEAR_EQUATION_1_LAYOUT, LINEAR_EQUATION_1_SKILL_ID, LINEAR_EQUATION_1_THEME_ID } from '../drill-engine';
import { defineTheme, LINEAR_INPUT_INTERFACE, LINEAR_INSTRUCTION } from './theme-definition';

export const LINEAR_EQUATION_1_DEFINITION = defineTheme({
  numeric_theme_id: LINEAR_EQUATION_1_THEME_ID, generator_revision: LINEAR_EQUATION_1_GENERATOR_REVISION,
  themeKey: 'jp.grade7.equation.linear.1', label: '一次方程式(1)', grade: { slug: 'grade-7', label: '中学1年生' },
  tags: ['equations', 'linear_equation'],
  problemCount: LINEAR_EQUATION_1_LAYOUT.problem_count, layout: LINEAR_EQUATION_1_LAYOUT,
  route: { gradeSlug: 'grade-7', themeSlug: 'linear-equation-1', pathname: '/drills/grade-7/linear-equation-1' },
  search: { title: '一次方程式(1) | AutoDrill', description: '中学1年生向けの整数解をもつ一次方程式ドリルです。' },
  compatibility: { skillId: LINEAR_EQUATION_1_SKILL_ID, curriculumPath: LINEAR_EQUATION_1_CURRICULUM_PATH },
  promptKind: 'linear_equation', answerSchemaKind: 'integer', inputInterface: LINEAR_INPUT_INTERFACE,
  worksheet: { title: '一次方程式(1)', instruction: LINEAR_INSTRUCTION, answerPrefix: 'x =' },
});
