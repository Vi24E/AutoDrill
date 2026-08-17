import type { AnswerInputInterface } from './drill-engine';

export { ALL_MATH_STRUCTURES, derivedGradeTag, hasThemeTag, taxonomyTags } from './themes/theme-definition';
export type { ThemeDefinition, ThemePromptKind } from './themes/theme-definition';

import { ONE_DIGIT_ADDITION_DEFINITION } from './themes/one-digit-addition';
import { ONE_DIGIT_SUBTRACTION_DEFINITION } from './themes/one-digit-subtraction';
import { TWO_DIGIT_ADDITION_DEFINITION } from './themes/two-digit-addition';
import { MULTIPLICATION_TABLE_DEFINITION } from './themes/multiplication-table';
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

export function sameInputInterface(left: AnswerInputInterface, right: AnswerInputInterface): boolean {
  if (left.type !== right.type) return false;
  if (left.type === 'simple_numeric' && right.type === 'simple_numeric') {
    return left.allow_decimal === right.allow_decimal && left.allow_negative === right.allow_negative;
  }
  if (left.type === 'structured_math' && right.type === 'structured_math') {
    return left.allowed_structures.length === right.allowed_structures.length
      && left.allowed_structures.every((structure, index) => structure === right.allowed_structures[index]);
  }
  if (left.type === 'digit_grid' && right.type === 'digit_grid') {
    return left.min_digit === right.min_digit
      && left.max_digit === right.max_digit
      && left.cell_count === right.cell_count;
  }
  return false;
}
