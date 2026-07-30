import { describe, expect, it } from 'vitest';
import {
  PHASE_DEVELOPMENT_SERVER,
  PHASE_PRODUCTION_BUILD,
  PHASE_PRODUCTION_SERVER,
} from 'next/constants.js';

import nextConfig, { resolveDistDir } from './next.config.mjs';

describe('Next output directories', () => {
  it('keeps development output isolated from production output', () => {
    expect(resolveDistDir(PHASE_DEVELOPMENT_SERVER)).toBe('.next-dev');
    expect(resolveDistDir(PHASE_PRODUCTION_BUILD)).toBe('.next');
    expect(resolveDistDir(PHASE_PRODUCTION_SERVER)).toBe('.next');
  });

  it('passes the phase-derived directory through the Next config function', () => {
    expect(nextConfig(PHASE_DEVELOPMENT_SERVER).distDir).toBe('.next-dev');
    expect(nextConfig(PHASE_PRODUCTION_BUILD).distDir).toBe('.next');
  });
});
