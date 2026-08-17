import { cpSync, existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { basename, dirname, join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';

const root = resolve(import.meta.dirname, '..');
const generatedDir = resolve(root, 'apps/web/src/generated/wire');
const write = process.argv.includes('--write');
const check = process.argv.includes('--check') || !write;
const tempDir = mkdtempSync(join(tmpdir(), 'autodrill-wire-types-'));

function fail(message) {
  console.error(message);
  process.exitCode = 1;
}

try {
  const result = spawnSync('cargo', [
    'run', '-q', '-p', 'drill-core', '--features', 'wire-types', '--bin', 'export_web_wire_types', '--offline', '--', tempDir,
  ], {
    cwd: root,
    encoding: 'utf8',
  });
  if (result.status !== 0) {
    process.stderr.write(result.stderr ?? '');
    process.stdout.write(result.stdout ?? '');
    fail('Failed to generate Rust wire TypeScript bindings.');
    process.exit();
  }

  const bindingFiles = readdirSync(tempDir).filter((name) => name.endsWith('.ts')).sort();
  const barrel = [
    '// GENERATED FILE. DO NOT EDIT BY HAND.',
    '// Source: canonical drill-core Rust wire DTOs via ts-rs.',
    '',
    ...bindingFiles.map((name) => `export type { ${basename(name, '.ts')} } from './${basename(name, '.ts')}';`),
    '',
  ].join('\n');
  writeFileSync(join(tempDir, 'index.ts'), barrel, 'utf8');

  const expectedFiles = [...bindingFiles, 'index.ts'].sort();
  const existingFiles = existsSync(generatedDir)
    ? readdirSync(generatedDir).filter((name) => name.endsWith('.ts')).sort()
    : [];

  const sameNames = JSON.stringify(existingFiles) === JSON.stringify(expectedFiles);
  const sameContents = sameNames && expectedFiles.every((name) => (
    readFileSync(join(generatedDir, name), 'utf8') === readFileSync(join(tempDir, name), 'utf8')
  ));

  if (write) {
    rmSync(generatedDir, { recursive: true, force: true });
    mkdirSync(dirname(generatedDir), { recursive: true });
    cpSync(tempDir, generatedDir, { recursive: true });
    console.log(`Updated ${generatedDir.slice(root.length + 1)}`);
  } else if (check && !sameContents) {
    fail('Generated Rust wire TypeScript bindings are stale. Run `node scripts/sync-web-wire-types.mjs --write`.');
  }
} finally {
  rmSync(tempDir, { recursive: true, force: true });
}
