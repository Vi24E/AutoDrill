import { arithmeticTheme, FRACTION_INSTRUCTION } from './theme-definition';

export const FRACTION_ADDITION_DEFINITION = arithmeticTheme({
  numeric_theme_id: 9, label: '分数の足し算（まとめ）',
  route: { themeSlug: 'fraction-addition' },
  search: { title: '分数の足し算（まとめ） | AutoDrill', description: '小学5年生向けの分数の足し算まとめドリルです。' }, title: '分数の足し算（まとめ）', instruction: FRACTION_INSTRUCTION,
});
