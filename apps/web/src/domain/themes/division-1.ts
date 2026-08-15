import { DIVISION_1_CURRICULUM_PATH, DIVISION_1_GENERATOR_REVISION, DIVISION_1_LAYOUT, DIVISION_1_SKILL_ID, DIVISION_1_THEME_ID } from '../drill-engine';
import { arithmeticTheme, SIMPLE_POSITIVE } from './theme-definition';

export const DIVISION_1_DEFINITION = arithmeticTheme({
  numeric_theme_id: DIVISION_1_THEME_ID, generator_revision: DIVISION_1_GENERATOR_REVISION,
  themeKey: 'jp.grade3.division.table.1', label: '割り算(1)', grade: { slug: 'grade-3', label: '小学3年生' },
  tags: ['division'],
  problemCount: DIVISION_1_LAYOUT.problem_count, layout: DIVISION_1_LAYOUT,
  route: { gradeSlug: 'grade-3', themeSlug: 'division-1', pathname: '/drills/grade-3/division-1' },
  search: { title: '割り算(1) | AutoDrill', description: '小学3年生向けの九九の範囲の割り算ドリルです。' },
  compatibility: { skillId: DIVISION_1_SKILL_ID, curriculumPath: DIVISION_1_CURRICULUM_PATH },
  inputInterface: SIMPLE_POSITIVE, title: '割り算(1)', instruction: '次の割り算をしなさい。',
});
