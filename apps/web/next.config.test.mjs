import { describe, expect, it } from 'vitest';

import { resolveDistDir } from './next.config.mjs';

describe('Next output directories', () => {
  it('keeps development output isolated from production output', () => {
    expect(resolveDistDir('development')).toBe('.next-dev');
    expect(resolveDistDir('production')).toBe('.next');
    expect(resolveDistDir('test')).toBe('.next');
  });
});
