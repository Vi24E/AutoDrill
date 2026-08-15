import { COLUMN_SUBTRACT_3_4DIGIT_CURRICULUM_PATH, COLUMN_SUBTRACT_3_4DIGIT_GENERATOR_REVISION, COLUMN_SUBTRACT_3_4DIGIT_LAYOUT, COLUMN_SUBTRACT_3_4DIGIT_SKILL_ID, COLUMN_SUBTRACT_3_4DIGIT_THEME_ID } from '../drill-engine';
import { columnArithmeticTheme, SIMPLE_POSITIVE } from './theme-definition';

export const COLUMN_SUBTRACT_3_4DIGIT_DEFINITION = columnArithmeticTheme({
  numeric_theme_id: COLUMN_SUBTRACT_3_4DIGIT_THEME_ID, generator_revision: COLUMN_SUBTRACT_3_4DIGIT_GENERATOR_REVISION,
  themeKey: 'jp.grade3.column.subtraction.three_four_digit', label: '三・四桁の引き算の筆算', grade: { slug: 'grade-3', label: '小学3年生' },
  tags: ['subtraction', 'column_arithmetic', 'print_recommended'],
  problemCount: COLUMN_SUBTRACT_3_4DIGIT_LAYOUT.problem_count, layout: COLUMN_SUBTRACT_3_4DIGIT_LAYOUT,
  route: { gradeSlug: 'grade-3', themeSlug: 'column-subtraction-three-four-digit', pathname: '/drills/grade-3/column-subtraction-three-four-digit' },
  search: { title: '三・四桁の引き算の筆算 | AutoDrill', description: '小学3年生向けの三・四桁の引き算を練習する筆算ドリルです。' },
  compatibility: { skillId: COLUMN_SUBTRACT_3_4DIGIT_SKILL_ID, curriculumPath: COLUMN_SUBTRACT_3_4DIGIT_CURRICULUM_PATH },
  inputInterface: SIMPLE_POSITIVE, answerSchemaKind: 'integer', title: '三・四桁の引き算の筆算',
});
