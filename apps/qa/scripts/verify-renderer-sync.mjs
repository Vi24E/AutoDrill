import { build } from 'vite';
import { mkdtempSync, readFileSync, readdirSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const SCRIPT_DIRECTORY = fileURLToPath(new URL('.', import.meta.url));
const QA_ROOT = resolve(SCRIPT_DIRECTORY, '..');
const TRACKED_RENDERER_DIRECTORY = join(QA_ROOT, 'public', 'renderer');
const RENDERER_CONFIG = join(QA_ROOT, 'vite.renderer.config.mjs');

function listFiles(directory, root = directory) {
  return readdirSync(directory, { withFileTypes: true })
    .flatMap((entry) => {
      const path = join(directory, entry.name);
      if (entry.isDirectory()) return listFiles(path, root);
      if (!entry.isFile()) return [];
      return [relative(root, path)];
    })
    .sort();
}

function compareRendererDirectories(expectedDirectory, actualDirectory) {
  const expectedFiles = listFiles(expectedDirectory);
  const actualFiles = listFiles(actualDirectory);
  const expectedSet = new Set(expectedFiles);
  const actualSet = new Set(actualFiles);
  const missingFiles = expectedFiles.filter((file) => !actualSet.has(file));
  const unexpectedFiles = actualFiles.filter((file) => !expectedSet.has(file));
  const changedFiles = expectedFiles.filter((file) => (
    actualSet.has(file)
    && !readFileSync(join(expectedDirectory, file)).equals(readFileSync(join(actualDirectory, file)))
  ));

  return { changedFiles, missingFiles, unexpectedFiles };
}

const temporaryRoot = mkdtempSync(join(tmpdir(), 'autodrill-qa-renderer-check-'));
const generatedRendererDirectory = join(temporaryRoot, 'renderer');

try {
  await build({
    configFile: RENDERER_CONFIG,
    build: {
      emptyOutDir: true,
      outDir: generatedRendererDirectory,
    },
  });

  const mismatch = compareRendererDirectories(TRACKED_RENDERER_DIRECTORY, generatedRendererDirectory);
  if (mismatch.changedFiles.length || mismatch.missingFiles.length || mismatch.unexpectedFiles.length) {
    const details = [
      mismatch.changedFiles.length ? `changed: ${mismatch.changedFiles.join(', ')}` : null,
      mismatch.missingFiles.length ? `missing from generated output: ${mismatch.missingFiles.join(', ')}` : null,
      mismatch.unexpectedFiles.length ? `missing from tracked output: ${mismatch.unexpectedFiles.join(', ')}` : null,
    ].filter(Boolean).join('\n');
    throw new Error(
      `Tracked QA renderer is stale relative to the current production Web source.\n${details}\n`
      + 'Run `pnpm --filter @autodrill/qa build:renderer` and commit the generated renderer files.',
    );
  }

  console.log('QA renderer is synchronized with the current production Web source.');
} finally {
  rmSync(temporaryRoot, { recursive: true, force: true, maxRetries: 5, retryDelay: 100 });
}
