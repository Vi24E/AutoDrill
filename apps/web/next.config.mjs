import { PHASE_DEVELOPMENT_SERVER } from 'next/constants.js';

const DEVELOPMENT_OUTPUT_DIR = '.next-dev';
const PRODUCTION_OUTPUT_DIR = '.next';
const GITHUB_PAGES_BASE_PATH = '/AutoDrill';

/** Keep development and production output isolated so overlapping commands cannot corrupt chunks. */
export function resolveDistDir(phase) {
  return phase === PHASE_DEVELOPMENT_SERVER ? DEVELOPMENT_OUTPUT_DIR : PRODUCTION_OUTPUT_DIR;
}

function contentSecurityPolicy(development) {
  const scriptSources = ["'self'", "'unsafe-inline'", "'wasm-unsafe-eval'"];
  if (development) scriptSources.push("'unsafe-eval'");
  return [
    "default-src 'self'",
    "base-uri 'self'",
    "frame-ancestors 'none'",
    "object-src 'none'",
    "form-action 'self'",
    "img-src 'self' data: blob:",
    "font-src 'self' data:",
    "style-src 'self' 'unsafe-inline'",
    `script-src ${scriptSources.join(' ')}`,
    `connect-src 'self'${development ? ' ws: wss:' : ''}`,
    "worker-src 'self' blob:",
  ].join('; ');
}

export function securityHeaders(development = false) {
  return [
    { key: 'Content-Security-Policy', value: contentSecurityPolicy(development) },
    { key: 'X-Content-Type-Options', value: 'nosniff' },
    { key: 'X-Frame-Options', value: 'DENY' },
    { key: 'Referrer-Policy', value: 'strict-origin-when-cross-origin' },
    { key: 'Permissions-Policy', value: 'camera=(), microphone=(), geolocation=(), payment=(), usb=()' },
  ];
}

/** Next calls this config with the current phase, which also lets development CSP allow HMR only in dev. */
const nextConfig = (phase) => {
  const githubPages = process.env.GITHUB_PAGES === 'true';
  const config = {
    reactStrictMode: true,
    distDir: resolveDistDir(phase),
    poweredByHeader: false,
    ...(githubPages ? {
      output: 'export',
      trailingSlash: true,
      basePath: GITHUB_PAGES_BASE_PATH,
    } : {}),
  };

  // Static hosts such as GitHub Pages cannot apply Next response headers.
  // Keep the stronger header policy for local/normal deployments and omit the
  // unsupported `headers()` feature only for the Pages export build.
  if (!githubPages) {
    config.headers = async () => [
      {
        source: '/wasm/pkg/:path*',
        headers: [{ key: 'Cache-Control', value: 'no-store, max-age=0' }],
      },
      { source: '/:path*', headers: securityHeaders(phase === PHASE_DEVELOPMENT_SERVER) },
    ];
  }
  return config;
};

export default nextConfig;
