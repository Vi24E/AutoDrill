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
    column_input: {
      single: { order: 'least_significant_first', decimal_point: { type: 'none' } },
      quotient: null,
      remainder: null,
    },
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

  it('offers every possible long-division quotient cell without revealing the canonical start position', () => {
    const division = problem({
      numeric_theme_id: 31,
      prompt: {
        kind: 'column_arithmetic',
        operator: 'divide',
        left: { kind: 'integer', value: 224 },
        right: { kind: 'integer', value: 7 },
      },
      canonical_answer: { type: 'tuple', value: [{ type: 'integer', value: '32' }, { type: 'integer', value: '0' }] },
      column_input: {
        single: null,
        quotient: { order: 'natural_division_flow', decimal_point: { type: 'none' } },
        remainder: { order: 'big_endian', decimal_point: { type: 'none' } },
      },
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
    expect(spec.order).toBe('natural_division_flow');
    expect(spec.direction).toBe('left-to-right');
    expect(spec.activeStart).toBe(0);
    expect(spec.initialIndex).toBe(0);
    expect(spec.activeEnd).toBe(2);
    expect(nextColumnDigitIndex(spec, spec.initialIndex)).toBe(1);
  });

  it('keeps decimal long-division geometry while leaving the quotient width for the learner to determine', () => {
    const division = problem({
      numeric_theme_id: 37,
      prompt: {
        kind: 'column_arithmetic',
        operator: 'divide',
        left: { kind: 'exact_decimal', coefficient: 21, scale: 2 },
        right: { kind: 'exact_decimal', coefficient: 21, scale: 1 },
      },
      canonical_answer: { type: 'exact_decimal', value: { coefficient: '1', scale: 1 } },
      column_input: {
        single: { order: 'natural_division_flow', decimal_point: { type: 'fixed', scale: 1 } },
        quotient: null,
        remainder: null,
      },
      answer_schema: { kind: 'decimal', max_scale: 6 },
      input_interface: { type: 'simple_numeric', allow_decimal: true, allow_negative: false },
      worked_solution: {
        kind: 'long_division',
        divisor: 21,
        dividend_coefficient: 21,
        dividend_scale: 1,
        quotient_trailing_cells: 0,
        steps: [{ product: 21, after: 0, product_offset: 0, after_offset: 0 }],
      },
    });
    const spec = columnDigitSpec(division, 'single');
    expect(spec.order).toBe('natural_division_flow');
    expect(spec.activeStart).toBe(0);
    expect(spec.initialIndex).toBe(0);
    expect(spec.activeEnd).toBeGreaterThan(spec.activeStart);
    expect(spec.fixedDecimalBoundary).toBe(spec.activeEnd);
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
      column_input: {
        single: { order: 'least_significant_first', decimal_point: { type: 'fixed', scale: 1 } },
        quotient: null,
        remainder: null,
      },
    });
    const spec = columnDigitSpec(decimal, 'single');
    expect(spec.fixedDecimalBoundary).toBe(spec.cellCount - 1);
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
      column_input: {
        single: { order: 'least_significant_first', decimal_point: { type: 'fixed', scale: 2 } },
        quotient: null,
        remainder: null,
      },
    });
    const spec = columnDigitSpec(decimal, 'single');
    const digits = columnDigitsFromAnswer(decimal.canonical_answer, spec);
    expect(digits.slice(-3)).toEqual(['0', '0', '5']);
    expect(columnDigitsToAnswer(digits, spec)).toEqual(decimal.canonical_answer);
  });


  it('keeps big-endian as a distinct selectable typed policy', () => {
    const division = problem({
      prompt: {
        kind: 'column_arithmetic',
        operator: 'divide',
        left: { kind: 'integer', value: 224 },
        right: { kind: 'integer', value: 7 },
      },
      column_input: {
        single: null,
        quotient: { order: 'big_endian', decimal_point: { type: 'none' } },
        remainder: { order: 'big_endian', decimal_point: { type: 'none' } },
      },
      canonical_answer: { type: 'tuple', value: [{ type: 'integer', value: '32' }, { type: 'integer', value: '0' }] },
      answer_schema: { kind: 'ordered_pair' },
      worked_solution: {
        kind: 'long_division',
        divisor: 7,
        dividend_coefficient: 224,
        dividend_scale: 0,
        quotient_trailing_cells: 0,
        steps: [{ product: 21, after: 14, product_offset: 1, after_offset: 0 }],
      },
    });
    const spec = columnDigitSpec(division, 'quotient');
    expect(spec.order).toBe('big_endian');
    expect(spec.direction).toBe('left-to-right');
    expect(spec.initialIndex).toBe(spec.activeStart);
  });

  it('keeps decimal placement editable without exposing the canonical multiplication scale', () => {
    const decimal = problem({
      numeric_theme_id: 36,
      prompt: {
        kind: 'column_arithmetic',
        operator: 'multiply',
        left: { kind: 'exact_decimal', coefficient: 12, scale: 1 },
        right: { kind: 'exact_decimal', coefficient: 3, scale: 1 },
      },
      canonical_answer: { type: 'exact_decimal', value: { coefficient: '36', scale: 2 } },
      answer_schema: { kind: 'decimal', max_scale: 6 },
      input_interface: { type: 'simple_numeric', allow_decimal: true, allow_negative: false },
      column_input: {
        single: { order: 'least_significant_first', decimal_point: { type: 'editable' } },
        quotient: null,
        remainder: null,
      },
      worked_solution: { kind: 'column_multiplication', partial_products: [{ value: 36, place: 0 }] },
    });
    const spec = columnDigitSpec(decimal, 'single');
    expect(spec.decimalPoint).toEqual({ type: 'editable' });
    expect(spec.fixedDecimalBoundary).toBeNull();

    const draft = Array<string | null>(spec.cellCount).fill(null);
    draft[spec.activeEnd - 1] = '3';
    draft[spec.activeEnd] = '6';
    expect(columnDigitsToAnswer(draft, spec)).toEqual({ type: 'integer', value: '36' });
    expect(columnDigitsToAnswer(draft, spec, spec.activeEnd - 1)).toEqual({
      type: 'exact_decimal',
      value: { coefficient: '36', scale: 2 },
    });
  });

  it('updates quotient digits without taking ownership of the ordinary remainder field', () => {
    const answer = { type: 'tuple', value: [{ type: 'integer', value: '32' }, { type: 'integer', value: '5' }] } satisfies import('@/domain/drill-engine').AnswerNode;
    expect(replaceColumnAnswerPart(answer, 'quotient', { type: 'integer', value: '31' })).toEqual({
      type: 'tuple',
      value: [{ type: 'integer', value: '31' }, { type: 'integer', value: '5' }],
    });
  });
});
