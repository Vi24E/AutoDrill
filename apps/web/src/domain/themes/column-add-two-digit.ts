import { COLUMN_ADD_2DIGIT_CURRICULUM_PATH, COLUMN_ADD_2DIGIT_GENERATOR_REVISION, COLUMN_ADD_2DIGIT_LAYOUT, COLUMN_ADD_2DIGIT_SKILL_ID, COLUMN_ADD_2DIGIT_THEME_ID } from '../drill-engine';
import { columnArithmeticTheme, SIMPLE_POSITIVE } from './theme-definition';

export const COLUMN_ADD_2DIGIT_DEFINITION = columnArithmeticTheme({
  numeric_theme_id: COLUMN_ADD_2DIGIT_THEME_ID, generator_revision: COLUMN_ADD_2DIGIT_GENERATOR_REVISION,
  themeKey: 'jp.grade2.column.addition.two_digit', label: '二桁の足し算の筆算', grade: { slug: 'grade-2', label: '小学2年生' },
  tags: ['addition', 'column_arithmetic', 'print_recommended'],
  problemCount: COLUMN_ADD_2DIGIT_LAYOUT.problem_count, layout: COLUMN_ADD_2DIGIT_LAYOUT,
  route: { gradeSlug: 'grade-2', themeSlug: 'column-addition-two-digit', pathname: '/drills/grade-2/column-addition-two-digit' },
  search: { title: '二桁の足し算の筆算 | AutoDrill', description: '小学2年生向けの二桁どうしの足し算を、縦にそろえて練習する筆算ドリルです。' },
  compatibility: { skillId: COLUMN_ADD_2DIGIT_SKILL_ID, curriculumPath: COLUMN_ADD_2DIGIT_CURRICULUM_PATH },
  inputInterface: SIMPLE_POSITIVE, answerSchemaKind: 'integer', title: '二桁の足し算の筆算',
});
