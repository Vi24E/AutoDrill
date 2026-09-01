import { arithmeticTheme, FRACTION_INSTRUCTION } from './theme-definition';

export const FRACTION_SUBTRACTION_DEFINITION = arithmeticTheme({
  numeric_theme_id: 11, label: '分数の引き算（まとめ）',
  route: { themeSlug: 'fraction-subtraction' },
  search: { title: '分数の引き算（まとめ） | AutoDrill', description: '小学5年生向けの分数の引き算まとめドリルです。' }, title: '分数の引き算（まとめ）', instruction: FRACTION_INSTRUCTION,
});
