import { COLUMN_MULTIPLY_1DIGIT_CURRICULUM_PATH, COLUMN_MULTIPLY_1DIGIT_GENERATOR_REVISION, COLUMN_MULTIPLY_1DIGIT_LAYOUT, COLUMN_MULTIPLY_1DIGIT_SKILL_ID, COLUMN_MULTIPLY_1DIGIT_THEME_ID } from '../drill-engine';
import { columnArithmeticTheme, SIMPLE_POSITIVE } from './theme-definition';

export const COLUMN_MULTIPLY_1DIGIT_DEFINITION = columnArithmeticTheme({
  numeric_theme_id: COLUMN_MULTIPLY_1DIGIT_THEME_ID, generator_revision: COLUMN_MULTIPLY_1DIGIT_GENERATOR_REVISION,
  themeKey: 'jp.grade3.column.multiplication.one_digit_multiplier', label: '一桁をかける掛け算の筆算', grade: { slug: 'grade-3', label: '小学3年生' },
  tags: ['multiplication', 'column_arithmetic', 'print_recommended'],
  problemCount: COLUMN_MULTIPLY_1DIGIT_LAYOUT.problem_count, layout: COLUMN_MULTIPLY_1DIGIT_LAYOUT,
  route: { gradeSlug: 'grade-3', themeSlug: 'column-multiplication-one-digit', pathname: '/drills/grade-3/column-multiplication-one-digit' },
  search: { title: '一桁をかける掛け算の筆算 | AutoDrill', description: '小学3年生向けの二・三桁の数に一桁の数をかける筆算ドリルです。' },
  compatibility: { skillId: COLUMN_MULTIPLY_1DIGIT_SKILL_ID, curriculumPath: COLUMN_MULTIPLY_1DIGIT_CURRICULUM_PATH },
  inputInterface: SIMPLE_POSITIVE, answerSchemaKind: 'integer', title: '一桁をかける掛け算の筆算',
});
