import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';

const pkg = JSON.parse(readFileSync(new URL('../package.json', import.meta.url), 'utf8'));
const pinnedNode = readFileSync(new URL('../.nvmrc', import.meta.url), 'utf8').trim();
const runtime = pkg.devEngines?.runtime;
const packageManager = pkg.packageManager ?? '';
const match = /^pnpm@(.+)$/.exec(packageManager);

if (!runtime || runtime.name !== 'node' || runtime.version !== pinnedNode) {
  throw new Error(`Node toolchain drift: .nvmrc=${pinnedNode}, devEngines.runtime=${JSON.stringify(runtime)}`);
}

const major = Number.parseInt(pinnedNode.split('.')[0], 10);
const expectedEngine = `>=${pinnedNode} <${major + 1}`;
if (pkg.engines?.node !== expectedEngine) {
  throw new Error(`Node engines drift: expected ${expectedEngine}, got ${pkg.engines?.node ?? '<missing>'}`);
}

if (process.versions.node !== pinnedNode) {
  throw new Error(`Wrong Node.js runtime: expected ${pinnedNode}, got ${process.versions.node}`);
}

if (!match) {
  throw new Error(`packageManager must pin pnpm exactly, got ${packageManager || '<missing>'}`);
}

const expectedPnpm = match[1];
const actualPnpm = execFileSync('pnpm', ['--version'], { encoding: 'utf8' }).trim();
if (actualPnpm !== expectedPnpm) {
  throw new Error(`Wrong pnpm runtime: expected ${expectedPnpm}, got ${actualPnpm}`);
}

console.log(`Toolchain verified: Node.js ${pinnedNode}, pnpm ${expectedPnpm}.`);
