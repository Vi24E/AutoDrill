import { render } from '@testing-library/react';
import { describe, expect, it } from 'vitest';

import { ProblemExpression } from '@/components/ProblemExpression';
import { DRILL_SCHEMA_VERSION, type ProblemDto } from '@/domain/drill-engine';

function columnProblem(prompt: ProblemDto['prompt'], canonical_answer: ProblemDto['canonical_answer'], worked_solution: ProblemDto['worked_solution'] = null): ProblemDto {
  return {
    schema_version: DRILL_SCHEMA_VERSION,
    id: 1,
    problem_id: '1',
    numeric_theme_id: 25,
    prompt,
    input_interface: { type: 'simple_numeric', allow_decimal: true, allow_negative: false },
    column_input: {
      single: {
        order: prompt.kind === 'column_arithmetic' && prompt.operator === 'divide' ? 'natural_division_flow' : 'least_significant_first',
        decimal_point: canonical_answer.type === 'exact_decimal'
          ? { type: 'fixed', scale: canonical_answer.value.scale }
          : { type: 'none' },
      },
      quotient: null,
      remainder: null,
    },
    answer_schema: { kind: 'decimal', max_scale: 3 },
    canonical_answer,
    worked_solution,
  };
}

describe('column arithmetic digit grid', () => {
  it('places ordinary operands into fixed digit cells', () => {
    const problem = columnProblem(
      { kind: 'column_arithmetic', operator: 'add', left: { kind: 'integer', value: 90 }, right: { kind: 'integer', value: 74 } },
      { type: 'integer', value: '164' },
    );
    const { container } = render(<ProblemExpression problem={problem} />);
    expect(container.querySelectorAll('.column-arithmetic-row-top .column-arithmetic-digit-cell')).toHaveLength(2);
    expect(container.querySelectorAll('.column-arithmetic-row-bottom .column-arithmetic-digit-cell')).toHaveLength(2);
  });

  it('does not consume a digit cell for decimal points in aligned addition', () => {
    const problem = columnProblem(
      { kind: 'column_arithmetic', operator: 'add', left: { kind: 'exact_decimal', coefficient: 123, scale: 1 }, right: { kind: 'exact_decimal', coefficient: 45, scale: 1 } },
      { type: 'exact_decimal', value: { coefficient: '168', scale: 1 } },
    );
    const { container } = render(<ProblemExpression problem={problem} />);
    const top = container.querySelector('.column-arithmetic-row-top')!;
    expect(top.querySelectorAll('.column-arithmetic-digit-cell')).toHaveLength(3);
    expect(top.querySelectorAll('.column-arithmetic-decimal-marker')).toHaveLength(1);
  });

  it('renders the long-division decimal point as a zero-width grid intersection marker', () => {
    const problem = columnProblem(
      { kind: 'column_arithmetic', operator: 'divide', left: { kind: 'exact_decimal', coefficient: 135, scale: 1 }, right: { kind: 'integer', value: 5 } },
      { type: 'exact_decimal', value: { coefficient: '27', scale: 1 } },
      {
        kind: 'long_division',
        divisor: 5,
        dividend_coefficient: 135,
        dividend_scale: 1,
        quotient_trailing_cells: 0,
        steps: [
          { product: 10, after: 35, product_offset: 1, after_offset: 0 },
          { product: 35, after: 0, product_offset: 0, after_offset: 0 },
        ],
      },
    );
    const { container } = render(<ProblemExpression problem={problem} solution />);
    const products = container.querySelectorAll('.column-division-solution-product-value');
    expect(products.length).toBeGreaterThanOrEqual(2);
    const finalCells = products[products.length - 1]!.querySelectorAll('.column-arithmetic-digit-cell');
    expect(finalCells).toHaveLength(2);
    expect(products[products.length - 1]!.querySelectorAll('.column-arithmetic-decimal-marker')).toHaveLength(1);
    expect(container.querySelectorAll('.column-division-solution-minus')).toHaveLength(0);
    expect(container.textContent).not.toContain('−');
    const bracketPath = container.querySelector('.column-division-bracket-mark path');
    expect(bracketPath).toHaveAttribute('d', expect.stringContaining('L 100 0'));
  });
  it('keeps only the primary rule on unsolved multiplication problems', () => {
    const problem = columnProblem(
      { kind: 'column_arithmetic', operator: 'multiply', left: { kind: 'integer', value: 304 }, right: { kind: 'integer', value: 3 } },
      { type: 'integer', value: '912' },
    );
    const unsolved = render(<ProblemExpression problem={problem} />);
    expect(unsolved.container.querySelectorAll('.column-arithmetic-rule')).toHaveLength(1);
    expect(unsolved.container.querySelectorAll('.column-arithmetic-final-rule')).toHaveLength(0);
    unsolved.unmount();

    const solved = render(<ProblemExpression problem={problem} solution />);
    expect(solved.container.querySelectorAll('.column-arithmetic-rule')).toHaveLength(1);
  });

});
