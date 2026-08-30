import { describe, expect, it } from 'vitest';

import { columnAddSubtractValueLayout, columnArithmeticGridVariables } from '@/domain/column-arithmetic-presentation';
import { A4_PAGE } from '@/domain/layout';
import { WORKSHEET_GRID_POINT } from '@/domain/worksheet-grid-presentation';
import type { ArithmeticExpression, ProblemDto } from '@/domain/drill-engine';

function columnProblem(operator: 'add' | 'subtract' | 'multiply' | 'divide', left: ArithmeticExpression, right: ArithmeticExpression): ProblemDto {
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
      '--column-operand-width': 'calc(2 * var(--worksheet-grid-cell))',
      '--column-digit-width': 'calc(3 * var(--worksheet-grid-cell))',
      '--column-total-width': 'calc(3 * var(--worksheet-grid-cell))',
    });
  });

  it('uses the aligned decimal display width when whole and fractional maxima come from different operands', () => {
    const layout = columnAddSubtractValueLayout(['12', '3.45']);
    expect(layout).toEqual({
      texts: ['12  ', ' 3.45'],
      cellCount: 4,
      usesDecimalAlignment: true,
    });

    const problem = columnProblem(
      'add',
      { kind: 'integer', value: 12 },
      { kind: 'exact_decimal', coefficient: 345, scale: 2 },
    );
    expect(columnArithmeticGridVariables(problem)).toMatchObject({
      '--column-operator-width': 'calc(1 * var(--worksheet-grid-cell))',
      '--column-operand-width': 'calc(4 * var(--worksheet-grid-cell))',
      '--column-digit-width': 'calc(5 * var(--worksheet-grid-cell))',
      '--column-total-width': 'calc(5 * var(--worksheet-grid-cell))',
    });
  });

  it('places add/subtract/multiply operators one cell left of the widest operand', () => {
    const cases = [
      columnProblem('add', { kind: 'integer', value: 404 }, { kind: 'integer', value: 43 }),
      columnProblem('subtract', { kind: 'integer', value: 404 }, { kind: 'integer', value: 43 }),
      columnProblem('multiply', { kind: 'integer', value: 404 }, { kind: 'integer', value: 43 }),
    ];
    for (const problem of cases) {
      expect(columnArithmeticGridVariables(problem)).toMatchObject({
        '--column-operator-width': 'calc(1 * var(--worksheet-grid-cell))',
        '--column-operand-width': 'calc(3 * var(--worksheet-grid-cell))',
      });
    }
  });

  it('reserves the maximum possible product width without adding a blank work row', () => {
    const oneDigit = columnProblem('multiply', { kind: 'integer', value: 91 }, { kind: 'integer', value: 3 });
    const twoDigit = columnProblem('multiply', { kind: 'integer', value: 91 }, { kind: 'integer', value: 23 });
    expect(columnArithmeticGridVariables(oneDigit)).toMatchObject({
      '--column-operand-width': 'calc(2 * var(--worksheet-grid-cell))',
      '--column-digit-width': 'calc(3 * var(--worksheet-grid-cell))',
      '--column-total-width': 'calc(3 * var(--worksheet-grid-cell))',
    });
    expect(columnArithmeticGridVariables(twoDigit)).toMatchObject({
      '--column-operand-width': 'calc(2 * var(--worksheet-grid-cell))',
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
      '--column-operand-width': 'calc(3 * var(--worksheet-grid-cell))',
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

  it('keeps every four-column lane anchor on an integer page-grid stride after the chosen two-cell shift', () => {
    const problem = columnProblem('multiply', { kind: 'integer', value: 504 }, { kind: 'integer', value: 81 });
    const width = (A4_PAGE.width - A4_PAGE.margin * 2) / 4;
    const anchors = Array.from({ length: 4 }, (_, column) => {
      const x = A4_PAGE.margin + width * column;
      const variables = columnArithmeticGridVariables(problem, { x, y: 90, width });
      const rightOffsetPoints = Number.parseFloat(variables['--column-lane-right-offset']!) / 100 * A4_PAGE.width;
      return x + width - rightOffsetPoints;
    });

    expect(anchors[0] / WORKSHEET_GRID_POINT).toBeCloseTo(6, 8);
    for (let index = 1; index < anchors.length; index += 1) {
      expect((anchors[index] - anchors[index - 1]) / WORKSHEET_GRID_POINT).toBeCloseTo(7, 8);
    }
  });
});
