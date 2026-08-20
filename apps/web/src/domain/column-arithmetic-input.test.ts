import { describe, expect, it } from 'vitest';

import {
  columnDigitSpec,
  columnDigitsFromAnswer,
  columnDigitsToAnswer,
  nextColumnDigitIndex,
  replaceColumnAnswerPart,
} from '@/domain/column-arithmetic-input';
import { DRILL_SCHEMA_VERSION, type ProblemDto } from '@/domain/drill-engine';

function problem(overrides: Partial<ProblemDto> = {}): ProblemDto {
  return {
    schema_version: DRILL_SCHEMA_VERSION,
    id: 1,
    problem_id: '1',
    numeric_theme_id: 25,
    prompt: {
      kind: 'column_arithmetic',
      operator: 'add',
      left: { kind: 'integer', value: 140 },
      right: { kind: 'integer', value: 5366 },
    },
    input_interface: { type: 'simple_numeric', allow_decimal: false, allow_negative: false },
    answer_schema: { kind: 'integer', min: '0', max: '99999' },
    canonical_answer: { type: 'integer', value: '5506' },
    worked_solution: null,
    ...overrides,
  };
}

describe('column arithmetic digit input', () => {
  it('starts add/subtract/multiply at the ones place and advances right-to-left', () => {
    const spec = columnDigitSpec(problem(), 'single');
    expect(spec.direction).toBe('right-to-left');
    expect(spec.initialIndex).toBe(spec.cellCount - 1);
    expect(nextColumnDigitIndex(spec, spec.initialIndex)).toBe(spec.initialIndex - 1);

    const draft = Array<string | null>(spec.cellCount).fill(null);
    draft[spec.initialIndex] = '4';
    draft[spec.initialIndex - 1] = '2';
    expect(columnDigitsToAnswer(draft, spec)).toEqual({ type: 'integer', value: '24' });
  });

  it('starts long-division quotient at the first Rust worked-solution quotient cell and advances left-to-right', () => {
    const division = problem({
      numeric_theme_id: 31,
      prompt: {
        kind: 'column_arithmetic',
        operator: 'divide',
        left: { kind: 'integer', value: 224 },
        right: { kind: 'integer', value: 7 },
      },
      canonical_answer: { type: 'tuple', value: [{ type: 'integer', value: '32' }, { type: 'integer', value: '0' }] },
      answer_schema: { kind: 'ordered_pair' },
      worked_solution: {
        kind: 'long_division',
        divisor: 7,
        dividend_coefficient: 224,
        dividend_scale: 0,
        quotient_trailing_cells: 0,
        steps: [
          { product: 21, after: 14, product_offset: 1, after_offset: 0 },
          { product: 14, after: 0, product_offset: 0, after_offset: 0 },
        ],
      },
    });
    const spec = columnDigitSpec(division, 'quotient');
    expect(spec.direction).toBe('left-to-right');
    expect(spec.initialIndex).toBe(1);
    expect(nextColumnDigitIndex(spec, spec.initialIndex)).toBe(2);
  });

  it('keeps each decimal digit in a fixed grid slot and reconstructs exact decimals', () => {
    const decimal = problem({
      numeric_theme_id: 33,
      prompt: {
        kind: 'column_arithmetic',
        operator: 'add',
        left: { kind: 'exact_decimal', coefficient: 123, scale: 1 },
        right: { kind: 'exact_decimal', coefficient: 45, scale: 1 },
      },
      canonical_answer: { type: 'exact_decimal', value: { coefficient: '168', scale: 1 } },
      answer_schema: { kind: 'decimal', max_scale: 3 },
      input_interface: { type: 'simple_numeric', allow_decimal: true, allow_negative: false },
    });
    const spec = columnDigitSpec(decimal, 'single');
    expect(spec.decimalBoundary).toBe(spec.cellCount - 1);
    const digits = columnDigitsFromAnswer(decimal.canonical_answer, spec);
    expect(columnDigitsToAnswer(digits, spec)).toEqual(decimal.canonical_answer);
  });

  it('preserves leading zero places for small exact decimals such as 0.05', () => {
    const decimal = problem({
      numeric_theme_id: 33,
      prompt: {
        kind: 'column_arithmetic',
        operator: 'add',
        left: { kind: 'exact_decimal', coefficient: 3, scale: 2 },
        right: { kind: 'exact_decimal', coefficient: 2, scale: 2 },
      },
      canonical_answer: { type: 'exact_decimal', value: { coefficient: '5', scale: 2 } },
      answer_schema: { kind: 'decimal', max_scale: 3 },
      input_interface: { type: 'simple_numeric', allow_decimal: true, allow_negative: false },
    });
    const spec = columnDigitSpec(decimal, 'single');
    const digits = columnDigitsFromAnswer(decimal.canonical_answer, spec);
    expect(digits.slice(-3)).toEqual(['0', '0', '5']);
    expect(columnDigitsToAnswer(digits, spec)).toEqual(decimal.canonical_answer);
  });

  it('updates quotient digits without taking ownership of the ordinary remainder field', () => {
    const answer = { type: 'tuple', value: [{ type: 'integer', value: '32' }, { type: 'integer', value: '5' }] } satisfies import('@/domain/drill-engine').AnswerNode;
    expect(replaceColumnAnswerPart(answer, 'quotient', { type: 'integer', value: '31' })).toEqual({
      type: 'tuple',
      value: [{ type: 'integer', value: '31' }, { type: 'integer', value: '5' }],
    });
  });
});
