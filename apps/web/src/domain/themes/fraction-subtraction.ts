import { arithmeticTheme, FRACTION_INSTRUCTION } from './theme-definition';

export const FRACTION_SUBTRACTION_DEFINITION = arithmeticTheme({
  numeric_theme_id: 11, label: '分数の引き算',
  route: { themeSlug: 'fraction-subtraction' },
  search: { title: '分数の引き算 | AutoDrill', description: '小学5年生向けの正の分数の引き算ドリルです。' }, title: '分数の引き算', instruction: FRACTION_INSTRUCTION,
});
