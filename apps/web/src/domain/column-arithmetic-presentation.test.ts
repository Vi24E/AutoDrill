import { describe, expect, it } from 'vitest';

import { columnArithmeticGridVariables } from '@/domain/column-arithmetic-presentation';
import type { ArithmeticExpression, ProblemDto } from '@/domain/drill-engine';

function columnProblem(operator: 'add' | 'multiply' | 'divide', left: ArithmeticExpression, right: ArithmeticExpression): ProblemDto {
  return {
    prompt: { kind: 'column_arithmetic', operator, left, right },
    canonical_answer: { type: 'integer', value: '0' },
    worked_solution: null,
  } as unknown as ProblemDto;
}

function longDivisionProblem(
  left: ArithmeticExpression,
  right: ArithmeticExpression,
  canonicalAnswer: ProblemDto['canonical_answer'],
  worked: { divisor: number; dividend_coefficient: number; dividend_scale: number; quotient_trailing_cells: number },
): ProblemDto {
  return {
    ...columnProblem('divide', left, right),
    canonical_answer: canonicalAnswer,
    worked_solution: { kind: 'long_division', ...worked, steps: [] },
  } as ProblemDto;
}

describe('column arithmetic presentation grid', () => {
  it('sizes every arithmetic lane as an integer number of page-grid cells', () => {
    const problem = columnProblem('add', { kind: 'integer', value: 57 }, { kind: 'integer', value: 38 });
    expect(columnArithmeticGridVariables(problem)).toEqual({
      '--column-operator-width': 'calc(1 * var(--worksheet-grid-cell))',
      '--column-digit-width': 'calc(3 * var(--worksheet-grid-cell))',
      '--column-total-width': 'calc(3 * var(--worksheet-grid-cell))',
    });
  });

  it('reserves the maximum possible product width without adding a blank work row', () => {
    const oneDigit = columnProblem('multiply', { kind: 'integer', value: 91 }, { kind: 'integer', value: 3 });
    const twoDigit = columnProblem('multiply', { kind: 'integer', value: 91 }, { kind: 'integer', value: 23 });
    expect(columnArithmeticGridVariables(oneDigit)).toMatchObject({
      '--column-digit-width': 'calc(3 * var(--worksheet-grid-cell))',
      '--column-total-width': 'calc(3 * var(--worksheet-grid-cell))',
    });
    expect(columnArithmeticGridVariables(twoDigit)).toMatchObject({
      '--column-digit-width': 'calc(4 * var(--worksheet-grid-cell))',
      '--column-total-width': 'calc(4 * var(--worksheet-grid-cell))',
    });
    expect(columnArithmeticGridVariables(twoDigit)).not.toHaveProperty('--column-working-rows');
  });

  it('sizes integer long division to the actual divisor and dividend digits', () => {
    const problem = longDivisionProblem(
      { kind: 'integer', value: 744 },
      { kind: 'integer', value: 8 },
      { type: 'integer', value: '93' },
      { divisor: 8, dividend_coefficient: 744, dividend_scale: 0, quotient_trailing_cells: 0 },
    );
    expect(columnArithmeticGridVariables(problem)).toEqual({
      '--column-operator-width': 'calc(1 * var(--worksheet-grid-cell))',
      '--column-digit-width': 'calc(3 * var(--worksheet-grid-cell))',
      '--column-total-width': 'calc(4 * var(--worksheet-grid-cell))',
      '--column-division-active-width': 'calc(3 * var(--worksheet-grid-cell))',
      '--column-division-work-rows': '3',
      '--column-remainder-width': 'calc(2 * var(--worksheet-grid-cell))',
      '--column-division-quotient-trailing-width': 'calc(0 * var(--worksheet-grid-cell))',
    });
  });

  it('keeps decimal quotients compact instead of reserving a six-cell lane', () => {
    const problem = longDivisionProblem(
      { kind: 'integer', value: 3 },
      { kind: 'integer', value: 2 },
      { type: 'exact_decimal', value: { coefficient: '15', scale: 1 } },
      { divisor: 2, dividend_coefficient: 30, dividend_scale: 1, quotient_trailing_cells: 0 },
    );
    expect(columnArithmeticGridVariables(problem)['--column-division-active-width']).toBe('calc(2 * var(--worksheet-grid-cell))');
  });

  it('reserves trailing quotient cells when the normalized dividend has more decimal places', () => {
    const problem = longDivisionProblem(
      { kind: 'exact_decimal', coefficient: 1230, scale: 2 },
      { kind: 'integer', value: 3 },
      { type: 'exact_decimal', value: { coefficient: '41', scale: 1 } },
      { divisor: 3, dividend_coefficient: 1230, dividend_scale: 2, quotient_trailing_cells: 1 },
    );
    expect(columnArithmeticGridVariables(problem)['--column-division-quotient-trailing-width']).toBe('calc(1 * var(--worksheet-grid-cell))');
  });

  it('uses Rust normalized dividend metadata instead of recomputing decimal normalization from operands', () => {
    const problem = longDivisionProblem(
      { kind: 'exact_decimal', coefficient: 12, scale: 1 },
      { kind: 'exact_decimal', coefficient: 3, scale: 2 },
      { type: 'integer', value: '40' },
      { divisor: 3, dividend_coefficient: 120, dividend_scale: 0, quotient_trailing_cells: 0 },
    );
    expect(columnArithmeticGridVariables(problem)['--column-division-active-width']).toBe('calc(3 * var(--worksheet-grid-cell))');
  });

  it('snaps a problem lane to the page-wide grid and aligns its vertical start to a grid row', () => {
    const problem = columnProblem('add', { kind: 'integer', value: 90 }, { kind: 'integer', value: 74 });
    const variables = columnArithmeticGridVariables(problem, { x: 42, y: 90, width: 127.82 });
    expect(variables['--column-lane-right-offset']).toMatch(/cqw$/);
    expect(variables['--column-expression-top-offset']).toMatch(/cqw$/);
  });

  it('allows a signed lane offset when the visible page grid lies beyond a logical four-column cell', () => {
    const problem = columnProblem('multiply', { kind: 'integer', value: 504 }, { kind: 'integer', value: 81 });
    const variables = columnArithmeticGridVariables(problem, { x: 297.64, y: 90, width: 127.82 });
    expect(Number.parseFloat(variables['--column-lane-right-offset']!)).toBeLessThan(0);
  });
});
