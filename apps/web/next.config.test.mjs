import { describe, expect, it } from 'vitest';
import {
  PHASE_DEVELOPMENT_SERVER,
  PHASE_PRODUCTION_BUILD,
  PHASE_PRODUCTION_SERVER,
} from 'next/constants.js';

import nextConfig, { resolveDistDir, securityHeaders } from './next.config.mjs';

describe('Next runtime config', () => {
  it('keeps development output isolated from production output', () => {
    expect(resolveDistDir(PHASE_DEVELOPMENT_SERVER)).toBe('.next-dev');
    expect(resolveDistDir(PHASE_PRODUCTION_BUILD)).toBe('.next');
    expect(resolveDistDir(PHASE_PRODUCTION_SERVER)).toBe('.next');
  });

  it('passes the phase-derived directory through the Next config function', () => {
    expect(nextConfig(PHASE_DEVELOPMENT_SERVER).distDir).toBe('.next-dev');
    expect(nextConfig(PHASE_PRODUCTION_BUILD).distDir).toBe('.next');
    expect(nextConfig(PHASE_PRODUCTION_BUILD).poweredByHeader).toBe(false);
  });


  it('exports a GitHub Pages project site without unsupported response headers', () => {
    const previous = process.env.GITHUB_PAGES;
    process.env.GITHUB_PAGES = 'true';
    try {
      const config = nextConfig(PHASE_PRODUCTION_BUILD);
      expect(config.output).toBe('export');
      expect(config.trailingSlash).toBe(true);
      expect(config.basePath).toBe('/AutoDrill');
      expect(config.headers).toBeUndefined();
    } finally {
      if (previous === undefined) delete process.env.GITHUB_PAGES;
      else process.env.GITHUB_PAGES = previous;
    }
  });

  it('serves generated WASM without persistent browser caching during active development', async () => {
    const routes = await nextConfig(PHASE_PRODUCTION_SERVER).headers();
    const wasm = routes.find((route) => route.source === '/wasm/pkg/:path*');
    expect(wasm).toBeDefined();
    expect(Object.fromEntries(wasm.headers.map(({ key, value }) => [key, value]))['Cache-Control']).toBe('no-store, max-age=0');
  });

  it('applies fail-closed browser security headers while keeping dev-only HMR allowances out of production', () => {
    const production = Object.fromEntries(securityHeaders(false).map(({ key, value }) => [key, value]));
    const development = Object.fromEntries(securityHeaders(true).map(({ key, value }) => [key, value]));
    expect(production['X-Content-Type-Options']).toBe('nosniff');
    expect(production['X-Frame-Options']).toBe('DENY');
    expect(production['Content-Security-Policy']).toContain("frame-ancestors 'none'");
    expect(production['Content-Security-Policy']).toContain("object-src 'none'");
    expect(production['Content-Security-Policy']).toContain("script-src 'self' 'unsafe-inline' 'wasm-unsafe-eval'");
    expect(production['Content-Security-Policy']).not.toContain("'unsafe-eval'");
    expect(development['Content-Security-Policy']).toContain("'unsafe-eval'");
  });
});
