import { columnArithmeticTheme } from './theme-definition';

export const COLUMN_DECIMAL_DIVISION_ROUNDED_DEFINITION = columnArithmeticTheme({
  numeric_theme_id: 66, label: '商を四捨五入する小数の割り算の筆算',
  route: { themeSlug: 'column-decimal-division-rounded' },
  search: { title: '商を四捨五入する小数の割り算の筆算 | AutoDrill', description: '小学5年生向けに、小数の割り算の商を四捨五入して小数第1位までの概数で答える筆算ドリルです。' },
  title: '商を四捨五入する小数の割り算の筆算',
  instruction: '次の割り算を筆算でし、商は四捨五入して小数第1位までの概数で答えなさい。',
});
