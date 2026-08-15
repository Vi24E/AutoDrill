import { COLUMN_DECIMAL_MULTIPLICATION_CURRICULUM_PATH, COLUMN_DECIMAL_MULTIPLICATION_GENERATOR_REVISION, COLUMN_DECIMAL_MULTIPLICATION_LAYOUT, COLUMN_DECIMAL_MULTIPLICATION_SKILL_ID, COLUMN_DECIMAL_MULTIPLICATION_THEME_ID } from '../drill-engine';
import { columnArithmeticTheme, SIMPLE_DECIMAL } from './theme-definition';

export const COLUMN_DECIMAL_MULTIPLICATION_DEFINITION = columnArithmeticTheme({
  numeric_theme_id: COLUMN_DECIMAL_MULTIPLICATION_THEME_ID, generator_revision: COLUMN_DECIMAL_MULTIPLICATION_GENERATOR_REVISION,
  themeKey: 'jp.grade5.column.decimal.multiplication', label: '小数の掛け算の筆算', grade: { slug: 'grade-5', label: '小学5年生' },
  tags: ['decimals', 'multiplication', 'column_arithmetic', 'print_recommended'],
  problemCount: COLUMN_DECIMAL_MULTIPLICATION_LAYOUT.problem_count, layout: COLUMN_DECIMAL_MULTIPLICATION_LAYOUT,
  route: { gradeSlug: 'grade-5', themeSlug: 'column-decimal-multiplication', pathname: '/drills/grade-5/column-decimal-multiplication' },
  search: { title: '小数の掛け算の筆算 | AutoDrill', description: '小学5年生向けの小数どうしの掛け算を縦式で練習する筆算ドリルです。' },
  compatibility: { skillId: COLUMN_DECIMAL_MULTIPLICATION_SKILL_ID, curriculumPath: COLUMN_DECIMAL_MULTIPLICATION_CURRICULUM_PATH },
  inputInterface: SIMPLE_DECIMAL, answerSchemaKind: 'decimal', title: '小数の掛け算の筆算',
});
