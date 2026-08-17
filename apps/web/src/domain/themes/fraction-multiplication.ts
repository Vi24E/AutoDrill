import { arithmeticTheme, FRACTION_INSTRUCTION } from './theme-definition';

export const FRACTION_MULTIPLICATION_DEFINITION = arithmeticTheme({
  numeric_theme_id: 10, label: '分数の掛け算',
  route: { themeSlug: 'fraction-multiplication' },
  search: { title: '分数の掛け算 | AutoDrill', description: '小学6年生向けの分数の掛け算ドリルです。' }, title: '分数の掛け算', instruction: FRACTION_INSTRUCTION,
});
