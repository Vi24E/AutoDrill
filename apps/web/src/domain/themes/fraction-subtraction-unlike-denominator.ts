import { arithmeticTheme, FRACTION_INSTRUCTION } from './theme-definition';

export const FRACTION_SUBTRACTION_UNLIKE_DENOMINATOR_DEFINITION = arithmeticTheme({
  numeric_theme_id: 58, label: '異分母の分数の引き算',
  route: { themeSlug: 'fraction-subtraction-unlike-denominator' },
  search: { title: '異分母の分数の引き算 | AutoDrill', description: '小学5年生向けの異分母の分数の引き算ドリルです。' }, title: '異分母の分数の引き算', instruction: FRACTION_INSTRUCTION,
});
