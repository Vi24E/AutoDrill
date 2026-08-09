import { spawnSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const generatedPath = resolve(root, 'apps/web/src/generated/drill-core-contract.ts');
const writeMode = process.argv.includes('--write');

const cargo = process.env.CARGO || 'cargo';
const childEnv = { ...process.env };
if (!childEnv.HOME) childEnv.HOME = process.env.USERPROFILE || root;
const result = spawnSync(cargo, ['run', '-q', '-p', 'drill-core', '--bin', 'export_web_contract'], {
  cwd: root,
  encoding: 'utf8',
  stdio: ['ignore', 'pipe', 'inherit'],
  env: childEnv,
});
if (result.error) {
  console.error(`Failed to execute ${cargo}: ${result.error.message}`);
  process.exit(1);
}
if (result.status !== 0) process.exit(result.status ?? 1);

let contract;
try {
  contract = JSON.parse(result.stdout);
} catch (error) {
  console.error('Rust web contract exporter returned invalid JSON.');
  console.error(error);
  process.exit(1);
}

const generated = `// GENERATED FILE. DO NOT EDIT BY HAND.\n// Source: drill-core::web_contract(). Run \`pnpm contract:generate\` after changing the Rust contract.\n\nexport const DRILL_CORE_CONTRACT = ${JSON.stringify(contract, null, 2)} as const;\n\nexport type DrillCoreGradeWarningCode = typeof DRILL_CORE_CONTRACT.grade_warning_codes[number];\n`;

if (writeMode) {
  mkdirSync(dirname(generatedPath), { recursive: true });
  writeFileSync(generatedPath, generated, 'utf8');
  console.log(`Updated ${generatedPath.slice(root.length + 1)}`);
  process.exit(0);
}

let existing = '';
try {
  existing = readFileSync(generatedPath, 'utf8');
} catch {
  // Report the same actionable error as a stale generated file.
}
if (existing !== generated) {
  console.error('Rust/Web compatibility contract is stale.');
  console.error('Run: node scripts/sync-web-contract.mjs --write');
  process.exit(1);
}
