import { describe, expect, it } from 'vitest';

import { problemExpression, problemExpressionTokens } from './problem-format';
import { linearFixtureWorksheet } from '@/test/fixtures';
import type { ProblemDto, RationalCoefficient } from './drill-engine';

function linearProblem(
  a: RationalCoefficient,
  b: RationalCoefficient,
  c: RationalCoefficient,
  d: RationalCoefficient,
  leftNegativeAsSubtraction = false,
  rightNegativeAsSubtraction = false,
): ProblemDto {
  const base = linearFixtureWorksheet(2).problems[0]!;
  return {
    ...base,
    prompt: {
      kind: 'linear_equation',
      a,
      b,
      c,
      d,
      left_negative_constant_as_subtraction: leftNegativeAsSubtraction,
      right_negative_constant_as_subtraction: rightNegativeAsSubtraction,
    },
  };
}

const q = (numerator: number, denominator = 1): RationalCoefficient => ({ numerator, denominator });

describe('problemExpression', () => {
  it('omits zero terms and coefficient 1 while retaining coefficient -1', () => {
    expect(problemExpression(linearProblem(q(0), q(3), q(1), q(0)))).toBe('3 = x');
    expect(problemExpression(linearProblem(q(-1), q(0), q(2), q(0)))).toBe('−x = 2x');
    expect(problemExpression(linearProblem(q(0), q(0), q(1), q(0)))).not.toContain('0x');
    expect(problemExpression(linearProblem(q(1), q(0), q(2), q(0)))).not.toContain('1x');
    expect(problemExpression(linearProblem(q(3), q(0), q(0), q(0)))).toBe('3x = 0');
    expect(problemExpression(linearProblem(q(3), q(0), q(0), q(5)))).toBe('3x = 5');
    expect(problemExpression(linearProblem(q(3), q(0), q(2), q(5)))).toBe('3x = 2x + 5');
  });

  it('uses the sampled negative-constant display form exactly', () => {
    const subtraction = linearProblem(q(2), q(-3), q(1), q(-4), true, true);
    const signedAddition = linearProblem(q(2), q(-3), q(1), q(-4), false, false);
    expect(problemExpression(subtraction)).toBe('2x − 3 = x − 4');
    expect(problemExpression(signedAddition)).toBe('2x + (−3) = x + (−4)');
  });

  it('formats reduced fractional coefficients and constants deterministically', () => {
    expect(problemExpression(linearProblem(q(1, 2), q(2, 3), q(-3, 4), q(-1, 5), false, true)))
      .toBe('1/2x + 2/3 = −3/4x − 1/5');
    expect(problemExpressionTokens(linearProblem(q(1, 2), q(2, 3), q(-3, 4), q(-1, 5), false, true))).toEqual([
      { kind: 'fraction', numerator: 1, denominator: 2 },
      { kind: 'text', text: 'x + ' },
      { kind: 'fraction', numerator: 2, denominator: 3 },
      { kind: 'text', text: ' = ' },
      { kind: 'minus' },
      { kind: 'fraction', numerator: 3, denominator: 4 },
      { kind: 'text', text: 'x ' },
      { kind: 'minus' },
      { kind: 'text', text: ' ' },
      { kind: 'fraction', numerator: 1, denominator: 5 },
    ]);
  });
});
