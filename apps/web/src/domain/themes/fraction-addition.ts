import { arithmeticTheme, FRACTION_INSTRUCTION } from './theme-definition';

export const FRACTION_ADDITION_DEFINITION = arithmeticTheme({
  numeric_theme_id: 9, label: '分数の足し算',
  route: { themeSlug: 'fraction-addition' },
  search: { title: '分数の足し算 | AutoDrill', description: '小学5年生向けの分数の足し算ドリルです。' }, title: '分数の足し算', instruction: FRACTION_INSTRUCTION,
});
