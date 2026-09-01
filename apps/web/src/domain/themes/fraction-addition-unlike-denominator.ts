import { arithmeticTheme, FRACTION_INSTRUCTION } from './theme-definition';

export const FRACTION_ADDITION_UNLIKE_DENOMINATOR_DEFINITION = arithmeticTheme({
  numeric_theme_id: 57, label: '異分母の分数の足し算',
  route: { themeSlug: 'fraction-addition-unlike-denominator' },
  search: { title: '異分母の分数の足し算 | AutoDrill', description: '小学5年生向けの異分母の分数の足し算ドリルです。' }, title: '異分母の分数の足し算', instruction: FRACTION_INSTRUCTION,
});
