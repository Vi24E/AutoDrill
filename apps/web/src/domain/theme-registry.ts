export { ALL_MATH_STRUCTURES, derivedGradeTag, hasThemeTag, taxonomyTags } from './themes/theme-definition';
export type { ThemeDefinition, ThemePromptKind } from './themes/theme-definition';

import { ONE_DIGIT_ADDITION_DEFINITION } from './themes/one-digit-addition';
import { ONE_DIGIT_SUBTRACTION_DEFINITION } from './themes/one-digit-subtraction';
import { TWO_DIGIT_ADDITION_DEFINITION } from './themes/two-digit-addition';
import { MULTIPLICATION_TABLE_DEFINITION } from './themes/multiplication-table';
import { ADDITION_UP_TO_10_DEFINITION } from './themes/addition-up-to-10';
import { SUBTRACTION_UP_TO_10_DEFINITION } from './themes/subtraction-up-to-10';
import { ADDITION_WITH_CARRY_DEFINITION } from './themes/addition-with-carry';
import { SUBTRACTION_WITH_BORROW_DEFINITION } from './themes/subtraction-with-borrow';
import { MULTIPLICATION_TABLE_1_DEFINITION } from './themes/multiplication-table-1';
import { MULTIPLICATION_TABLE_2_DEFINITION } from './themes/multiplication-table-2';
import { MULTIPLICATION_TABLE_3_DEFINITION } from './themes/multiplication-table-3';
import { MULTIPLICATION_TABLE_4_DEFINITION } from './themes/multiplication-table-4';
import { MULTIPLICATION_TABLE_5_DEFINITION } from './themes/multiplication-table-5';
import { MULTIPLICATION_TABLE_6_DEFINITION } from './themes/multiplication-table-6';
import { MULTIPLICATION_TABLE_7_DEFINITION } from './themes/multiplication-table-7';
import { MULTIPLICATION_TABLE_8_DEFINITION } from './themes/multiplication-table-8';
import { MULTIPLICATION_TABLE_9_DEFINITION } from './themes/multiplication-table-9';
import { DIVISION_WITH_REMAINDER_DEFINITION } from './themes/division-with-remainder';
import { SIMPLE_TWO_DIGIT_DIVISION_DEFINITION } from './themes/simple-two-digit-division';
import { FRACTION_ADDITION_DEFINITION } from './themes/fraction-addition';
import { FRACTION_SUBTRACTION_DEFINITION } from './themes/fraction-subtraction';
import { FRACTION_MULTIPLICATION_DEFINITION } from './themes/fraction-multiplication';
import { FRACTION_DIVISION_DEFINITION } from './themes/fraction-division';
import { FRACTION_INTEGER_MULTIPLICATION_DEFINITION } from './themes/fraction-integer-multiplication';
import { FRACTION_INTEGER_DIVISION_DEFINITION } from './themes/fraction-integer-division';
import { FRACTION_SUMMARY_IMPROPER_DEFINITION } from './themes/fraction-summary-improper';
import { DIVISION_1_DEFINITION } from './themes/division-1';
import { SIGNED_ARITHMETIC_1_DEFINITION } from './themes/signed-arithmetic-1';
import { SIGNED_ARITHMETIC_2_DEFINITION } from './themes/signed-arithmetic-2';
import { LINEAR_EQUATION_1_DEFINITION } from './themes/linear-equation-1';
import { LINEAR_EQUATION_2_DEFINITION } from './themes/linear-equation-2';
import { QUADRATIC_EQUATION_1_DEFINITION } from './themes/quadratic-equation-1';
import { QUADRATIC_EQUATION_2_DEFINITION } from './themes/quadratic-equation-2';
import { QUADRATIC_EQUATION_3_DEFINITION } from './themes/quadratic-equation-3';
import { DECIMAL_ADD_SUBTRACT_DEFINITION } from './themes/decimal-add-subtract';
import { DECIMAL_MULTIPLICATION_DEFINITION } from './themes/decimal-multiplication';
import { DECIMAL_DIVISION_DEFINITION } from './themes/decimal-division';
import { SIMULTANEOUS_EQUATION_1_DEFINITION } from './themes/simultaneous-equation-1';
import { LIAR_PUZZLE_DEFINITION } from './themes/liar-puzzle';
import { MINI_SUDOKU_DEFINITION } from './themes/mini-sudoku';
import { COLUMN_ADD_2DIGIT_DEFINITION } from './themes/column-add-two-digit';
import { COLUMN_SUBTRACT_2DIGIT_DEFINITION } from './themes/column-subtract-two-digit';
import { COLUMN_ADD_3_4DIGIT_DEFINITION } from './themes/column-add-three-four-digit';
import { COLUMN_SUBTRACT_3_4DIGIT_DEFINITION } from './themes/column-subtract-three-four-digit';
import { COLUMN_MULTIPLY_1DIGIT_DEFINITION } from './themes/column-multiply-one-digit';
import { COLUMN_MULTIPLY_2DIGIT_DEFINITION } from './themes/column-multiply-two-digit';
import { COLUMN_DIVIDE_1DIGIT_DEFINITION } from './themes/column-divide-one-digit';
import { COLUMN_DIVIDE_2DIGIT_DEFINITION } from './themes/column-divide-two-digit';
import { COLUMN_DECIMAL_ADD_SUBTRACT_DEFINITION } from './themes/column-decimal-add-subtract';
import { COLUMN_DECIMAL_MULTIPLY_INTEGER_DEFINITION } from './themes/column-decimal-multiply-integer';
import { COLUMN_DECIMAL_DIVIDE_INTEGER_DEFINITION } from './themes/column-decimal-divide-integer';
import { COLUMN_DECIMAL_MULTIPLICATION_DEFINITION } from './themes/column-decimal-multiplication';
import { COLUMN_DECIMAL_DIVISION_DEFINITION } from './themes/column-decimal-division';
import type { ThemeDefinition } from './themes/theme-definition';

/** Central registry is intentionally only enumeration + lookup; each theme owns one definition file. */
export const THEME_DEFINITIONS: readonly ThemeDefinition[] = [
  ONE_DIGIT_ADDITION_DEFINITION,
  ONE_DIGIT_SUBTRACTION_DEFINITION,
  TWO_DIGIT_ADDITION_DEFINITION,
  COLUMN_ADD_2DIGIT_DEFINITION,
  COLUMN_SUBTRACT_2DIGIT_DEFINITION,
  COLUMN_ADD_3_4DIGIT_DEFINITION,
  COLUMN_SUBTRACT_3_4DIGIT_DEFINITION,
  MULTIPLICATION_TABLE_DEFINITION,
  ADDITION_UP_TO_10_DEFINITION,
  SUBTRACTION_UP_TO_10_DEFINITION,
  ADDITION_WITH_CARRY_DEFINITION,
  SUBTRACTION_WITH_BORROW_DEFINITION,
  MULTIPLICATION_TABLE_1_DEFINITION,
  MULTIPLICATION_TABLE_2_DEFINITION,
  MULTIPLICATION_TABLE_3_DEFINITION,
  MULTIPLICATION_TABLE_4_DEFINITION,
  MULTIPLICATION_TABLE_5_DEFINITION,
  MULTIPLICATION_TABLE_6_DEFINITION,
  MULTIPLICATION_TABLE_7_DEFINITION,
  MULTIPLICATION_TABLE_8_DEFINITION,
  MULTIPLICATION_TABLE_9_DEFINITION,
  DIVISION_WITH_REMAINDER_DEFINITION,
  SIMPLE_TWO_DIGIT_DIVISION_DEFINITION,
  DIVISION_1_DEFINITION,
  COLUMN_MULTIPLY_1DIGIT_DEFINITION,
  COLUMN_MULTIPLY_2DIGIT_DEFINITION,
  COLUMN_DIVIDE_1DIGIT_DEFINITION,
  COLUMN_DIVIDE_2DIGIT_DEFINITION,
  DECIMAL_ADD_SUBTRACT_DEFINITION,
  COLUMN_DECIMAL_ADD_SUBTRACT_DEFINITION,
  COLUMN_DECIMAL_MULTIPLY_INTEGER_DEFINITION,
  COLUMN_DECIMAL_DIVIDE_INTEGER_DEFINITION,
  FRACTION_ADDITION_DEFINITION,
  DECIMAL_MULTIPLICATION_DEFINITION,
  DECIMAL_DIVISION_DEFINITION,
  COLUMN_DECIMAL_MULTIPLICATION_DEFINITION,
  COLUMN_DECIMAL_DIVISION_DEFINITION,
  FRACTION_SUBTRACTION_DEFINITION,
  FRACTION_MULTIPLICATION_DEFINITION,
  FRACTION_DIVISION_DEFINITION,
  FRACTION_INTEGER_MULTIPLICATION_DEFINITION,
  FRACTION_INTEGER_DIVISION_DEFINITION,
  FRACTION_SUMMARY_IMPROPER_DEFINITION,
  SIGNED_ARITHMETIC_1_DEFINITION,
  SIGNED_ARITHMETIC_2_DEFINITION,
  LINEAR_EQUATION_1_DEFINITION,
  LINEAR_EQUATION_2_DEFINITION,
  SIMULTANEOUS_EQUATION_1_DEFINITION,
  QUADRATIC_EQUATION_1_DEFINITION,
  QUADRATIC_EQUATION_2_DEFINITION,
  QUADRATIC_EQUATION_3_DEFINITION,
  LIAR_PUZZLE_DEFINITION,
  MINI_SUDOKU_DEFINITION,
];

export function findThemeDefinitionByNumericId(numericThemeId: number): ThemeDefinition | undefined {
  return THEME_DEFINITIONS.find((theme) => theme.numeric_theme_id === numericThemeId);
}
