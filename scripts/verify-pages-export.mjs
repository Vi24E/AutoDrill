import fs from 'node:fs';
import path from 'node:path';

const repoRoot = path.resolve(import.meta.dirname, '..');
const outDir = path.join(repoRoot, 'apps/web/out');
const basePath = '/AutoDrill';
const origin = 'https://vi24e.github.io';

function fail(message) {
  console.error(`Pages export verification failed: ${message}`);
  process.exit(1);
}
function read(relative) {
  const file = path.join(outDir, relative);
  if (!fs.existsSync(file)) fail(`missing ${relative}`);
  return fs.readFileSync(file, 'utf8');
}

const root = read('index.html');
const sitemap = read('sitemap.xml');
if (!root.includes('name="robots" content="noindex, nofollow"')) fail('alpha noindex metadata is missing');
if (!root.includes(`${basePath}/_next/`)) fail('Next assets do not use the project base path');
if (root.includes('href="/_next/') || root.includes('src="/_next/')) fail('root-relative Next asset escaped the project base path');

const locations = [...sitemap.matchAll(/<loc>([^<]+)<\/loc>/g)].map((match) => match[1]);
if (locations.length !== 38) fail(`expected 38 sitemap URLs, found ${locations.length}`);
for (const url of locations) {
  if (!url.startsWith(`${origin}${basePath}/`)) fail(`sitemap URL escaped project site: ${url}`);
  const pathname = new URL(url).pathname.slice(`${basePath}/`.length);
  const htmlPath = pathname === '' ? 'index.html' : path.join(pathname, 'index.html');
  if (!fs.existsSync(path.join(outDir, htmlPath))) fail(`sitemap route has no exported HTML: ${htmlPath}`);
}

const staticDir = path.join(outDir, '_next/static');
const stack = [staticDir];
let sawWasmPath = false;
while (stack.length) {
  const current = stack.pop();
  for (const entry of fs.readdirSync(current, { withFileTypes: true })) {
    const absolute = path.join(current, entry.name);
    if (entry.isDirectory()) { stack.push(absolute); continue; }
    if (!entry.name.endsWith('.js')) continue;
    const source = fs.readFileSync(absolute, 'utf8');
    if (source.includes(`${basePath}/wasm/pkg/drill_wasm.js`)) sawWasmPath = true;
    if (source.includes('PDF Japanese font shard') || source.includes('pdf-lib')) {
      fail(`obsolete PDF implementation leaked into ${path.relative(outDir, absolute)}`);
    }
  }
}
if (!sawWasmPath) fail('browser bundle does not reference base-path-aware WASM glue');
for (const file of ['wasm/pkg/drill_wasm.js', 'wasm/pkg/drill_wasm_bg.wasm']) {
  if (!fs.existsSync(path.join(outDir, file))) fail(`missing generated ${file}`);
}
console.log(`GitHub Pages export verified: ${locations.length} routes, base path ${basePath}.`);
