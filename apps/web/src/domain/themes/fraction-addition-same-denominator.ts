import { arithmeticTheme, FRACTION_INSTRUCTION } from './theme-definition';

export const FRACTION_ADDITION_SAME_DENOMINATOR_DEFINITION = arithmeticTheme({
  numeric_theme_id: 55, label: '同分母の分数の足し算',
  route: { themeSlug: 'fraction-addition-same-denominator' },
  search: { title: '同分母の分数の足し算 | AutoDrill', description: '小学4年生向けの同分母の分数の足し算ドリルです。' }, title: '同分母の分数の足し算', instruction: FRACTION_INSTRUCTION,
});
