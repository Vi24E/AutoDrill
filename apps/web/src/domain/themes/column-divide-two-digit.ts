import { COLUMN_DIVIDE_2DIGIT_CURRICULUM_PATH, COLUMN_DIVIDE_2DIGIT_GENERATOR_REVISION, COLUMN_DIVIDE_2DIGIT_LAYOUT, COLUMN_DIVIDE_2DIGIT_SKILL_ID, COLUMN_DIVIDE_2DIGIT_THEME_ID } from '../drill-engine';
import { columnArithmeticTheme } from './theme-definition';

const COLUMN_DIVISION_INPUT = { type: 'structured_math', allowed_structures: ['tuple'] } as const;

export const COLUMN_DIVIDE_2DIGIT_DEFINITION = columnArithmeticTheme({
  numeric_theme_id: COLUMN_DIVIDE_2DIGIT_THEME_ID, generator_revision: COLUMN_DIVIDE_2DIGIT_GENERATOR_REVISION,
  themeKey: 'jp.grade4.column.division.two_digit_divisor', label: '二桁で割る割り算の筆算', grade: { slug: 'grade-4', label: '小学4年生' },
  tags: ['division', 'column_arithmetic', 'print_recommended'],
  problemCount: COLUMN_DIVIDE_2DIGIT_LAYOUT.problem_count, layout: COLUMN_DIVIDE_2DIGIT_LAYOUT,
  route: { gradeSlug: 'grade-4', themeSlug: 'column-division-two-digit', pathname: '/drills/grade-4/column-division-two-digit' },
  search: { title: '二桁で割る割り算の筆算 | AutoDrill', description: '小学4年生向けの二桁の除数で割る筆算ドリルです。商とあまりを求めます。' },
  compatibility: { skillId: COLUMN_DIVIDE_2DIGIT_SKILL_ID, curriculumPath: COLUMN_DIVIDE_2DIGIT_CURRICULUM_PATH },
  inputInterface: COLUMN_DIVISION_INPUT, answerSchemaKind: 'ordered_pair', title: '二桁で割る割り算の筆算', instruction: '次の割り算を筆算でし、商とあまりを求めなさい。',
});
