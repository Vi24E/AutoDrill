import { DECIMAL_MULTIPLY_DIVIDE_CURRICULUM_PATH, DECIMAL_MULTIPLY_DIVIDE_GENERATOR_REVISION, DECIMAL_MULTIPLY_DIVIDE_LAYOUT, DECIMAL_MULTIPLY_DIVIDE_SKILL_ID, DECIMAL_MULTIPLY_DIVIDE_THEME_ID } from '../drill-engine';
import { arithmeticTheme, SIMPLE_DECIMAL } from './theme-definition';

export const DECIMAL_MULTIPLY_DIVIDE_DEFINITION = arithmeticTheme({
  numeric_theme_id: DECIMAL_MULTIPLY_DIVIDE_THEME_ID, generator_revision: DECIMAL_MULTIPLY_DIVIDE_GENERATOR_REVISION,
  themeKey: 'jp.grade5.decimal.multiply_divide', label: '小数の掛け算と割り算', grade: { slug: 'grade-5', label: '小学5年生' },
  gradeGenre: { genreKey: 'decimals', label: '小数' }, recommendedGenre: { genreKey: 'decimals', label: '小数' },
  problemCount: DECIMAL_MULTIPLY_DIVIDE_LAYOUT.problem_count, layout: DECIMAL_MULTIPLY_DIVIDE_LAYOUT,
  route: { gradeSlug: 'grade-5', themeSlug: 'decimal-multiply-divide', pathname: '/drills/grade-5/decimal-multiply-divide' },
  search: { title: '小数の掛け算と割り算 | AutoDrill', description: '小学5年生向けの小数の掛け算・割り算ドリルです。' },
  compatibility: { skillId: DECIMAL_MULTIPLY_DIVIDE_SKILL_ID, curriculumPath: DECIMAL_MULTIPLY_DIVIDE_CURRICULUM_PATH },
  inputInterface: SIMPLE_DECIMAL, answerSchemaKind: 'decimal', title: '小数の掛け算と割り算', instruction: '次の計算をしなさい。',
});
