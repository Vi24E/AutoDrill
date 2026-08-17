import { arithmeticTheme } from './theme-definition';

export const DECIMAL_DIVISION_DEFINITION = arithmeticTheme({
  numeric_theme_id: 24, label: '小数の割り算',
  route: { themeSlug: 'decimal-division' },
  search: { title: '小数の割り算 | AutoDrill', description: '小学5年生向けの小数の割り算ドリルです。割り切れる問題を生成します。' }, title: '小数の割り算', instruction: '次の計算をしなさい。',
});
