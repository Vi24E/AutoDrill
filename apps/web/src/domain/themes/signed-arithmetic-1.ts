import { SIGNED_ARITHMETIC_1_CURRICULUM_PATH, SIGNED_ARITHMETIC_1_GENERATOR_REVISION, SIGNED_ARITHMETIC_1_LAYOUT, SIGNED_ARITHMETIC_1_SKILL_ID, SIGNED_ARITHMETIC_1_THEME_ID } from '../drill-engine';
import { arithmeticTheme, SIMPLE_SIGNED } from './theme-definition';

export const SIGNED_ARITHMETIC_1_DEFINITION = arithmeticTheme({
  numeric_theme_id: SIGNED_ARITHMETIC_1_THEME_ID, generator_revision: SIGNED_ARITHMETIC_1_GENERATOR_REVISION,
  themeKey: 'jp.grade7.signed.arithmetic.1', label: '負の数の計算(1)', grade: { slug: 'grade-7', label: '中学1年生' },
  gradeGenre: { genreKey: 'signed-numbers', label: '正の数・負の数' }, recommendedGenre: { genreKey: 'negative-numbers', label: '負の数' },
  problemCount: SIGNED_ARITHMETIC_1_LAYOUT.problem_count, layout: SIGNED_ARITHMETIC_1_LAYOUT,
  route: { gradeSlug: 'grade-7', themeSlug: 'signed-arithmetic-1', pathname: '/drills/grade-7/signed-arithmetic-1' },
  search: { title: '負の数の計算(1) | AutoDrill', description: '中学1年生向けの正負の整数の加減ドリルです。' },
  compatibility: { skillId: SIGNED_ARITHMETIC_1_SKILL_ID, curriculumPath: SIGNED_ARITHMETIC_1_CURRICULUM_PATH },
  inputInterface: SIMPLE_SIGNED, title: '負の数の計算(1)', instruction: '次の計算をしなさい。',
});
