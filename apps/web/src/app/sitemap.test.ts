import { describe, expect, it } from 'vitest';

import sitemap from '@/app/sitemap';
import { IMPLEMENTED_THEMES } from '@/domain/curriculum';

describe('sitemap discovery', () => {
  it('includes only TOP and implemented unit routes', () => {
    const paths = sitemap().map((entry) => new URL(entry.url).pathname);
    expect(paths).toEqual(['/', ...IMPLEMENTED_THEMES.map((theme) => theme.route.pathname)]);
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