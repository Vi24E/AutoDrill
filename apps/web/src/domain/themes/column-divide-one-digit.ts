import { COLUMN_DIVIDE_1DIGIT_CURRICULUM_PATH, COLUMN_DIVIDE_1DIGIT_GENERATOR_REVISION, COLUMN_DIVIDE_1DIGIT_LAYOUT, COLUMN_DIVIDE_1DIGIT_SKILL_ID, COLUMN_DIVIDE_1DIGIT_THEME_ID } from '../drill-engine';
import { columnArithmeticTheme } from './theme-definition';

const COLUMN_DIVISION_INPUT = { type: 'structured_math', allowed_structures: ['tuple'] } as const;

export const COLUMN_DIVIDE_1DIGIT_DEFINITION = columnArithmeticTheme({
  numeric_theme_id: COLUMN_DIVIDE_1DIGIT_THEME_ID, generator_revision: COLUMN_DIVIDE_1DIGIT_GENERATOR_REVISION,
  themeKey: 'jp.grade3.column.division.one_digit_divisor', label: '一桁で割る割り算の筆算', grade: { slug: 'grade-3', label: '小学3年生' },
  tags: ['division', 'column_arithmetic', 'print_recommended'],
  problemCount: COLUMN_DIVIDE_1DIGIT_LAYOUT.problem_count, layout: COLUMN_DIVIDE_1DIGIT_LAYOUT,
  route: { gradeSlug: 'grade-3', themeSlug: 'column-division-one-digit', pathname: '/drills/grade-3/column-division-one-digit' },
  search: { title: '一桁で割る割り算の筆算 | AutoDrill', description: '小学3年生向けの一桁の除数で割る筆算ドリルです。商とあまりを求めます。' },
  compatibility: { skillId: COLUMN_DIVIDE_1DIGIT_SKILL_ID, curriculumPath: COLUMN_DIVIDE_1DIGIT_CURRICULUM_PATH },
  inputInterface: COLUMN_DIVISION_INPUT, answerSchemaKind: 'ordered_pair', title: '一桁で割る割り算の筆算', instruction: '次の割り算を筆算でし、商とあまりを求めなさい。',
});
