import { describe, expect, it } from 'vitest';

import { digitGridValues, emptyDigitGridAnswer, replaceDigitGridCell } from './digit-grid-input';

const input = { type: 'digit_grid', min_digit: 1, max_digit: 4, cell_count: 16 } as const;

describe('digit-grid input projection', () => {
  it('keeps a fixed-length tuple and replaces only the selected cell', () => {
    const empty = emptyDigitGridAnswer(input);
    expect(empty.type).toBe('tuple');
    expect(empty.type === 'tuple' ? empty.value : []).toHaveLength(16);

    const filled = replaceDigitGridCell(empty, input, 5, 3);
    expect(digitGridValues(filled, input)[5]).toBe(3);
    expect(digitGridValues(filled, input).filter((value) => value !== null)).toEqual([3]);

    const cleared = replaceDigitGridCell(filled, input, 5, null);
    expect(digitGridValues(cleared, input).every((value) => value === null)).toBe(true);
  });

  it('fails closed for out-of-range edits', () => {
    const empty = emptyDigitGridAnswer(input);
    expect(replaceDigitGridCell(empty, input, -1, 2)).toEqual(empty);
    expect(replaceDigitGridCell(empty, input, 16, 2)).toEqual(empty);
    expect(replaceDigitGridCell(empty, input, 0, 5)).toEqual(empty);
  });
});
