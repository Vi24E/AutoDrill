import { describe, expect, it } from 'vitest';

import sitemap from '@/app/sitemap';

describe('sitemap discovery', () => {
  it('includes only TOP and implemented unit routes', () => {
    const paths = sitemap().map((entry) => new URL(entry.url).pathname);
    expect(paths).toEqual([
      '/',
      '/drills/grade-1/one-digit-addition',
      '/drills/grade-1/one-digit-subtraction',
      '/drills/grade-2/two-digit-addition',
      '/drills/grade-2/multiplication-table',
      '/drills/grade-5/fraction-addition',
      '/drills/grade-5/fraction-subtraction',
      '/drills/grade-6/fraction-multiplication',
      '/drills/grade-7/signed-arithmetic-1',
      '/drills/grade-7/signed-arithmetic-2',
      '/drills/grade-7/linear-equation-1',
      '/drills/grade-7/linear-equation-2',
    ]);
    expect(paths.some((path) => path.includes('Dummy'))).toBe(false);
  });
});
