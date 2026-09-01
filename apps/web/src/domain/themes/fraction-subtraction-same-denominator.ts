import { arithmeticTheme, FRACTION_INSTRUCTION } from './theme-definition';

export const FRACTION_SUBTRACTION_SAME_DENOMINATOR_DEFINITION = arithmeticTheme({
  numeric_theme_id: 56, label: '同分母の分数の引き算',
  route: { themeSlug: 'fraction-subtraction-same-denominator' },
  search: { title: '同分母の分数の引き算 | AutoDrill', description: '小学4年生向けの同分母の分数の引き算ドリルです。' }, title: '同分母の分数の引き算', instruction: FRACTION_INSTRUCTION,
});
