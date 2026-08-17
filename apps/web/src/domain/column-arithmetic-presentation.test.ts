import { describe, expect, it } from 'vitest';

import { columnArithmeticGridVariables, columnArithmeticWorkingRows } from '@/domain/column-arithmetic-presentation';
import type { ArithmeticExpression, ProblemDto } from '@/domain/drill-engine';

function columnProblem(operator: 'add' | 'multiply' | 'divide', left: ArithmeticExpression, right: ArithmeticExpression): ProblemDto {
  return {
    prompt: { kind: 'column_arithmetic', operator, left, right },
    canonical_answer: { type: 'integer', value: '0' },
  } as unknown as ProblemDto;
}

describe('column arithmetic presentation grid', () => {
  it('sizes every arithmetic lane as an integer number of page-grid cells', () => {
    const problem = columnProblem('add', { kind: 'integer', value: 57 }, { kind: 'integer', value: 38 });
    expect(columnArithmeticGridVariables(problem)).toEqual({
      '--column-operator-width': 'calc(1 * var(--worksheet-grid-cell))',
      '--column-digit-width': 'calc(2 * var(--worksheet-grid-cell))',
      '--column-working-rows': '0',
    });
  });

  it('derives multiplication workspace only from the shared integer-grid policy', () => {
    const oneDigit = columnProblem('multiply', { kind: 'integer', value: 91 }, { kind: 'integer', value: 3 });
    const twoDigit = columnProblem('multiply', { kind: 'integer', value: 91 }, { kind: 'integer', value: 23 });
    expect(columnArithmeticWorkingRows(oneDigit)).toBe(0);
    expect(columnArithmeticWorkingRows(twoDigit)).toBe(1);
    expect(columnArithmeticGridVariables(oneDigit)['--column-working-rows']).toBe('0');
    expect(columnArithmeticGridVariables(twoDigit)['--column-working-rows']).toBe('1');
  });

  it('sizes integer long division to the actual divisor and dividend digits', () => {
    const problem = columnProblem('divide', { kind: 'integer', value: 744 }, { kind: 'integer', value: 8 });
    expect(columnArithmeticGridVariables(problem)).toEqual({
      '--column-operator-width': 'calc(1 * var(--worksheet-grid-cell))',
      '--column-digit-width': 'calc(3 * var(--worksheet-grid-cell))',
      '--column-working-rows': '0',
      '--column-division-active-width': 'calc(3 * var(--worksheet-grid-cell))',
      '--column-division-work-rows': '3',
      '--column-remainder-width': 'calc(2 * var(--worksheet-grid-cell))',
      '--column-division-quotient-trailing-width': 'calc(0 * var(--worksheet-grid-cell))',
    });
  });

  it('keeps decimal quotients compact instead of reserving a six-cell lane', () => {
    const problem = {
      ...columnProblem('divide', { kind: 'integer', value: 3 }, { kind: 'integer', value: 2 }),
      canonical_answer: { type: 'exact_decimal', value: { coefficient: '15', scale: 1 } },
    } as ProblemDto;
    expect(columnArithmeticGridVariables(problem)['--column-division-active-width']).toBe('calc(2 * var(--worksheet-grid-cell))');
  });

  it('reserves trailing quotient cells when the normalized dividend has more decimal places', () => {
    const problem = {
      ...columnProblem('divide', { kind: 'exact_decimal', coefficient: 1230, scale: 2 }, { kind: 'integer', value: 3 }),
      canonical_answer: { type: 'exact_decimal', value: { coefficient: '41', scale: 1 } },
    } as ProblemDto;
    expect(columnArithmeticGridVariables(problem)['--column-division-quotient-trailing-width']).toBe('calc(1 * var(--worksheet-grid-cell))');
  });

  it('snaps a problem lane to the page-wide grid and aligns its vertical start to a grid row', () => {
    const problem = columnProblem('add', { kind: 'integer', value: 90 }, { kind: 'integer', value: 74 });
    const variables = columnArithmeticGridVariables(problem, { x: 42, y: 90, width: 127.82 });
    expect(variables['--column-lane-right-offset']).toMatch(/cqw$/);
    expect(variables['--column-expression-top-offset']).toMatch(/cqw$/);
  });
});
