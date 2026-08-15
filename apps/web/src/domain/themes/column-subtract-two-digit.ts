import { COLUMN_SUBTRACT_2DIGIT_CURRICULUM_PATH, COLUMN_SUBTRACT_2DIGIT_GENERATOR_REVISION, COLUMN_SUBTRACT_2DIGIT_LAYOUT, COLUMN_SUBTRACT_2DIGIT_SKILL_ID, COLUMN_SUBTRACT_2DIGIT_THEME_ID } from '../drill-engine';
import { columnArithmeticTheme, SIMPLE_POSITIVE } from './theme-definition';

export const COLUMN_SUBTRACT_2DIGIT_DEFINITION = columnArithmeticTheme({
  numeric_theme_id: COLUMN_SUBTRACT_2DIGIT_THEME_ID, generator_revision: COLUMN_SUBTRACT_2DIGIT_GENERATOR_REVISION,
  themeKey: 'jp.grade2.column.subtraction.two_digit', label: '二桁の引き算の筆算', grade: { slug: 'grade-2', label: '小学2年生' },
  tags: ['subtraction', 'column_arithmetic', 'print_recommended'],
  problemCount: COLUMN_SUBTRACT_2DIGIT_LAYOUT.problem_count, layout: COLUMN_SUBTRACT_2DIGIT_LAYOUT,
  route: { gradeSlug: 'grade-2', themeSlug: 'column-subtraction-two-digit', pathname: '/drills/grade-2/column-subtraction-two-digit' },
  search: { title: '二桁の引き算の筆算 | AutoDrill', description: '小学2年生向けの二桁どうしの引き算を、縦にそろえて練習する筆算ドリルです。' },
  compatibility: { skillId: COLUMN_SUBTRACT_2DIGIT_SKILL_ID, curriculumPath: COLUMN_SUBTRACT_2DIGIT_CURRICULUM_PATH },
  inputInterface: SIMPLE_POSITIVE, answerSchemaKind: 'integer', title: '二桁の引き算の筆算',
});
