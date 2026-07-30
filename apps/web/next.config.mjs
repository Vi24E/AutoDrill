const DEVELOPMENT_OUTPUT_DIR = '.next-dev';
const PRODUCTION_OUTPUT_DIR = '.next';

/**
 * Keep the development server's incremental output separate from the
 * production build. A `next build` must not replace chunks that an already
 * running `next dev` page is trying to load.
 */
export function resolveDistDir(nodeEnv = process.env.NODE_ENV) {
  return nodeEnv === 'development' ? DEVELOPMENT_OUTPUT_DIR : PRODUCTION_OUTPUT_DIR;
}

/** @type {import('next').NextConfig} */
const nextConfig = {
  reactStrictMode: true,
  distDir: resolveDistDir(),
};

export default nextConfig;
