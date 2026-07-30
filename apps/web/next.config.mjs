import { PHASE_DEVELOPMENT_SERVER } from 'next/constants.js';

const DEVELOPMENT_OUTPUT_DIR = '.next-dev';
const PRODUCTION_OUTPUT_DIR = '.next';

/**
 * Keep the development server's incremental output separate from the
 * production build. A `next build` must not replace chunks that an already
 * running `next dev` page is trying to load.
 */
export function resolveDistDir(phase) {
  return phase === PHASE_DEVELOPMENT_SERVER ? DEVELOPMENT_OUTPUT_DIR : PRODUCTION_OUTPUT_DIR;
}

/**
 * Next calls a function-valued config with the current phase. Using that
 * official phase signal avoids relying on a module-load NODE_ENV value that
 * can be stale when dev and production commands overlap.
 */
const nextConfig = (phase) => ({
  reactStrictMode: true,
  distDir: resolveDistDir(phase),
});

export default nextConfig;
