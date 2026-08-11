import type { AnswerInputInterface } from './drill-engine';

export { ONE_DIGIT_ADDITION_DEFINITION } from './themes/one-digit-addition';
export { ONE_DIGIT_SUBTRACTION_DEFINITION } from './themes/one-digit-subtraction';
export { TWO_DIGIT_ADDITION_DEFINITION } from './themes/two-digit-addition';
export { MULTIPLICATION_TABLE_DEFINITION } from './themes/multiplication-table';
export { FRACTION_ADDITION_DEFINITION } from './themes/fraction-addition';
export { FRACTION_SUBTRACTION_DEFINITION } from './themes/fraction-subtraction';
export { FRACTION_MULTIPLICATION_DEFINITION } from './themes/fraction-multiplication';
export { SIGNED_ARITHMETIC_1_DEFINITION } from './themes/signed-arithmetic-1';
export { SIGNED_ARITHMETIC_2_DEFINITION } from './themes/signed-arithmetic-2';
export { LINEAR_EQUATION_1_DEFINITION } from './themes/linear-equation-1';
export { LINEAR_EQUATION_2_DEFINITION } from './themes/linear-equation-2';
export { ALL_MATH_STRUCTURES } from './themes/theme-definition';
export type { ThemeDefinition, ThemePromptKind } from './themes/theme-definition';

import { ONE_DIGIT_ADDITION_DEFINITION } from './themes/one-digit-addition';
import { ONE_DIGIT_SUBTRACTION_DEFINITION } from './themes/one-digit-subtraction';
import { TWO_DIGIT_ADDITION_DEFINITION } from './themes/two-digit-addition';
import { MULTIPLICATION_TABLE_DEFINITION } from './themes/multiplication-table';
import { FRACTION_ADDITION_DEFINITION } from './themes/fraction-addition';
import { FRACTION_SUBTRACTION_DEFINITION } from './themes/fraction-subtraction';
import { FRACTION_MULTIPLICATION_DEFINITION } from './themes/fraction-multiplication';
import { SIGNED_ARITHMETIC_1_DEFINITION } from './themes/signed-arithmetic-1';
import { SIGNED_ARITHMETIC_2_DEFINITION } from './themes/signed-arithmetic-2';
import { LINEAR_EQUATION_1_DEFINITION } from './themes/linear-equation-1';
import { LINEAR_EQUATION_2_DEFINITION } from './themes/linear-equation-2';
import type { ThemeDefinition } from './themes/theme-definition';

/** Central registry is intentionally only enumeration + lookup; each theme owns one definition file. */
export const THEME_DEFINITIONS: readonly ThemeDefinition[] = [
  ONE_DIGIT_ADDITION_DEFINITION,
  ONE_DIGIT_SUBTRACTION_DEFINITION,
  TWO_DIGIT_ADDITION_DEFINITION,
  MULTIPLICATION_TABLE_DEFINITION,
  FRACTION_ADDITION_DEFINITION,
  FRACTION_SUBTRACTION_DEFINITION,
  FRACTION_MULTIPLICATION_DEFINITION,
  SIGNED_ARITHMETIC_1_DEFINITION,
  SIGNED_ARITHMETIC_2_DEFINITION,
  LINEAR_EQUATION_1_DEFINITION,
  LINEAR_EQUATION_2_DEFINITION,
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
  return false;
}
