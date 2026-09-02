import { answerDecimalScale, columnArithmeticTheme, decimalPlaceLabel } from './theme-definition';

const NUMERIC_THEME_ID = 65 as const;
const ANSWER_PLACE = decimalPlaceLabel(answerDecimalScale(NUMERIC_THEME_ID));

export const COLUMN_DECIMAL_DIVISION_REMAINDER_DEFINITION = columnArithmeticTheme({
  numeric_theme_id: NUMERIC_THEME_ID, label: '余りを答える小数の割り算の筆算',
  route: { themeSlug: 'column-decimal-division-remainder' },
  search: { title: '余りを答える小数の割り算の筆算 | AutoDrill', description: `小学5年生向けに、小数の割り算を${ANSWER_PLACE}まで計算し、商と余りを答える筆算ドリルです。` },
  title: '余りを答える小数の割り算の筆算',
  instruction: `次の割り算を筆算でし、商は${ANSWER_PLACE}まで求め、あまりも答えなさい。`,
});
