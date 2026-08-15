import { COLUMN_DECIMAL_ADD_SUBTRACT_CURRICULUM_PATH, COLUMN_DECIMAL_ADD_SUBTRACT_GENERATOR_REVISION, COLUMN_DECIMAL_ADD_SUBTRACT_LAYOUT, COLUMN_DECIMAL_ADD_SUBTRACT_SKILL_ID, COLUMN_DECIMAL_ADD_SUBTRACT_THEME_ID } from '../drill-engine';
import { columnArithmeticTheme, SIMPLE_DECIMAL } from './theme-definition';

export const COLUMN_DECIMAL_ADD_SUBTRACT_DEFINITION = columnArithmeticTheme({
  numeric_theme_id: COLUMN_DECIMAL_ADD_SUBTRACT_THEME_ID, generator_revision: COLUMN_DECIMAL_ADD_SUBTRACT_GENERATOR_REVISION,
  themeKey: 'jp.grade4.column.decimal.add_subtract', label: '小数の足し算と引き算の筆算', grade: { slug: 'grade-4', label: '小学4年生' },
  tags: ['decimals', 'addition', 'subtraction', 'column_arithmetic', 'print_recommended'],
  problemCount: COLUMN_DECIMAL_ADD_SUBTRACT_LAYOUT.problem_count, layout: COLUMN_DECIMAL_ADD_SUBTRACT_LAYOUT,
  route: { gradeSlug: 'grade-4', themeSlug: 'column-decimal-add-subtract', pathname: '/drills/grade-4/column-decimal-add-subtract' },
  search: { title: '小数の足し算と引き算の筆算 | AutoDrill', description: '小学4年生向けに、小数点をそろえて足し算・引き算をする筆算ドリルです。' },
  compatibility: { skillId: COLUMN_DECIMAL_ADD_SUBTRACT_SKILL_ID, curriculumPath: COLUMN_DECIMAL_ADD_SUBTRACT_CURRICULUM_PATH },
  inputInterface: SIMPLE_DECIMAL, answerSchemaKind: 'decimal', title: '小数の足し算と引き算の筆算',
});
