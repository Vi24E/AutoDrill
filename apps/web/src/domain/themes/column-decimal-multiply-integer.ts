import { COLUMN_DECIMAL_MULTIPLY_INTEGER_CURRICULUM_PATH, COLUMN_DECIMAL_MULTIPLY_INTEGER_GENERATOR_REVISION, COLUMN_DECIMAL_MULTIPLY_INTEGER_LAYOUT, COLUMN_DECIMAL_MULTIPLY_INTEGER_SKILL_ID, COLUMN_DECIMAL_MULTIPLY_INTEGER_THEME_ID } from '../drill-engine';
import { columnArithmeticTheme, SIMPLE_DECIMAL } from './theme-definition';

export const COLUMN_DECIMAL_MULTIPLY_INTEGER_DEFINITION = columnArithmeticTheme({
  numeric_theme_id: COLUMN_DECIMAL_MULTIPLY_INTEGER_THEME_ID, generator_revision: COLUMN_DECIMAL_MULTIPLY_INTEGER_GENERATOR_REVISION,
  themeKey: 'jp.grade4.column.decimal.multiply_integer', label: '小数と整数の掛け算の筆算', grade: { slug: 'grade-4', label: '小学4年生' },
  tags: ['decimals', 'multiplication', 'column_arithmetic', 'print_recommended'],
  problemCount: COLUMN_DECIMAL_MULTIPLY_INTEGER_LAYOUT.problem_count, layout: COLUMN_DECIMAL_MULTIPLY_INTEGER_LAYOUT,
  route: { gradeSlug: 'grade-4', themeSlug: 'column-decimal-multiply-integer', pathname: '/drills/grade-4/column-decimal-multiply-integer' },
  search: { title: '小数と整数の掛け算の筆算 | AutoDrill', description: '小学4年生向けの小数に整数をかける筆算ドリルです。' },
  compatibility: { skillId: COLUMN_DECIMAL_MULTIPLY_INTEGER_SKILL_ID, curriculumPath: COLUMN_DECIMAL_MULTIPLY_INTEGER_CURRICULUM_PATH },
  inputInterface: SIMPLE_DECIMAL, answerSchemaKind: 'decimal', title: '小数と整数の掛け算の筆算',
});
