import { describe, expect, it } from 'vitest';

import { worksheetGradeBand, worksheetGradeBandClass } from '@/domain/grade-band';

describe('worksheet grade typography bands', () => {
  it('uses three generic bands for grades 1-3, 4-6, and junior high', () => {
    for (const grade of [1, 2, 3]) expect(worksheetGradeBand(`grade-${grade}`)).toBe('early-elementary');
    for (const grade of [4, 5, 6]) expect(worksheetGradeBand(`grade-${grade}`)).toBe('late-elementary');
    for (const grade of [7, 8, 9]) expect(worksheetGradeBand(`grade-${grade}`)).toBe('junior-high');
    expect(worksheetGradeBandClass('grade-7')).toBe('worksheet-grade-junior-high');
  });
});
