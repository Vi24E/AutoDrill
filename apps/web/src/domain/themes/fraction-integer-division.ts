import { arithmeticTheme, FRACTION_INSTRUCTION } from './theme-definition';

export const FRACTION_INTEGER_DIVISION_DEFINITION = arithmeticTheme({
  numeric_theme_id: 22, label: '分数と整数の割り算',
  route: { themeSlug: 'fraction-integer-division' },
  search: { title: '分数と整数の割り算 | AutoDrill', description: '小学6年生向けの分数と整数の割り算ドリルです。' }, title: '分数と整数の割り算', instruction: FRACTION_INSTRUCTION,
});
