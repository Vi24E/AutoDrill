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
      '/drills/grade-3/division-1',
      '/drills/grade-4/decimal-add-subtract',
      '/drills/grade-5/fraction-addition',
      '/drills/grade-5/decimal-multiply-divide',
      '/drills/grade-5/fraction-subtraction',
      '/drills/grade-6/fraction-multiplication',
      '/drills/grade-6/fraction-division',
      '/drills/grade-7/signed-arithmetic-1',
      '/drills/grade-7/signed-arithmetic-2',
      '/drills/grade-7/linear-equation-1',
      '/drills/grade-7/linear-equation-2',
      '/drills/grade-8/simultaneous-equation-1',
      '/drills/grade-9/quadratic-equation-1',
      '/drills/grade-9/quadratic-equation-2',
      '/drills/grade-9/quadratic-equation-3',
      '/drills/bonus/liar-puzzle',
    ]);
  });

  it('includes the GitHub Pages project base path in alpha deployment URLs', () => {
    const previousSite = process.env.NEXT_PUBLIC_SITE_URL;
    const previousBase = process.env.NEXT_PUBLIC_BASE_PATH;
    process.env.NEXT_PUBLIC_SITE_URL = 'https://vi24e.github.io';
    process.env.NEXT_PUBLIC_BASE_PATH = '/AutoDrill';
    try {
      const urls = sitemap().map((entry) => entry.url);
      expect(urls[0]).toBe('https://vi24e.github.io/AutoDrill/');
      expect(urls[1]).toBe('https://vi24e.github.io/AutoDrill/drills/grade-1/one-digit-addition');
    } finally {
      if (previousSite === undefined) delete process.env.NEXT_PUBLIC_SITE_URL;
      else process.env.NEXT_PUBLIC_SITE_URL = previousSite;
      if (previousBase === undefined) delete process.env.NEXT_PUBLIC_BASE_PATH;
      else process.env.NEXT_PUBLIC_BASE_PATH = previousBase;
    }
  });

});