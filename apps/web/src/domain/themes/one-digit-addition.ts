import { ADDITION_CURRICULUM_PATH, ADDITION_GENERATOR_REVISION, ADDITION_LAYOUT, ADDITION_SKILL_ID, ADDITION_THEME_ID } from '../drill-engine';
import { defineTheme, SIMPLE_POSITIVE } from './theme-definition';

export const ONE_DIGIT_ADDITION_DEFINITION = defineTheme({
  numeric_theme_id: ADDITION_THEME_ID,
  generator_revision: ADDITION_GENERATOR_REVISION,
  themeKey: 'jp.grade1.addition.one_digit',
  label: '一桁の足し算',
  grade: { slug: 'grade-1', label: '小学1年生' },
  tags: ['addition'],
  problemCount: ADDITION_LAYOUT.problem_count,
  layout: ADDITION_LAYOUT,
  route: { gradeSlug: 'grade-1', themeSlug: 'one-digit-addition', pathname: '/drills/grade-1/one-digit-addition' },
  search: { title: '一桁の足し算 | AutoDrill', description: '小学1年生向けの一桁の足し算ドリルです。' },
  compatibility: { skillId: ADDITION_SKILL_ID, curriculumPath: ADDITION_CURRICULUM_PATH },
  promptKind: 'addition',
  answerSchemaKind: 'integer',
  inputInterface: SIMPLE_POSITIVE,
  worksheet: { title: '1けたのたしざん(1)', instruction: '', answerPrefix: null },
});
