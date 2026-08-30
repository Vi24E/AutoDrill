import { execFileSync } from 'node:child_process';
import { chmodSync, cpSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { basename, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { captureGitState, normalizeGitState } from '../src/git-state.mjs';

const SCRIPT_DIRECTORY = fileURLToPath(new URL('.', import.meta.url));
const QA_ROOT = resolve(SCRIPT_DIRECTORY, '..');
const REPOSITORY_ROOT = resolve(QA_ROOT, '../..');
const DEFAULT_DESTINATION = join(REPOSITORY_ROOT, 'AutoDrill Problem QA.app');

function repositoryGitSha() {
  return execFileSync('git', ['rev-parse', 'HEAD'], { cwd: REPOSITORY_ROOT, encoding: 'utf8' }).trim();
}

export function buildMacApp({ destination = DEFAULT_DESTINATION, gitSha, gitState } = {}) {
  if (basename(destination) !== 'AutoDrill Problem QA.app') {
    throw new Error('Refusing to replace a destination that is not the QA app bundle.');
  }
  const contents = join(destination, 'Contents');
  const executable = join(contents, 'MacOS', 'AutoDrill Problem QA');
  const resources = join(contents, 'Resources');
  const runtime = join(resources, 'qa');
  const nodeVersion = readFileSync(join(REPOSITORY_ROOT, '.nvmrc'), 'utf8').trim();
  const resolvedGitState = gitState
    ? normalizeGitState(gitState, gitSha ?? 'unknown')
    : gitSha
      ? normalizeGitState(null, gitSha)
      : captureGitState({ repositoryRoot: REPOSITORY_ROOT });
  const resolvedGitSha = gitSha ?? resolvedGitState.head_sha ?? repositoryGitSha();

  rmSync(destination, { recursive: true, force: true });
  mkdirSync(join(contents, 'MacOS'), { recursive: true });
  mkdirSync(runtime, { recursive: true });

  cpSync(join(QA_ROOT, 'macos', 'Info.plist'), join(contents, 'Info.plist'));
  cpSync(join(QA_ROOT, 'macos', 'launcher.zsh'), executable);
  cpSync(join(QA_ROOT, 'src'), join(runtime, 'src'), { recursive: true });
  cpSync(join(QA_ROOT, 'public'), join(runtime, 'public'), { recursive: true });
  cpSync(join(QA_ROOT, 'generated'), join(runtime, 'generated'), { recursive: true });
  mkdirSync(join(runtime, 'wasm'), { recursive: true });
  for (const file of ['drill_wasm.js', 'drill_wasm_bg.wasm']) {
    cpSync(join(REPOSITORY_ROOT, 'apps', 'qa', 'wasm', file), join(runtime, 'wasm', file));
  }
  cpSync(join(QA_ROOT, 'package.json'), join(runtime, 'package.json'));
  writeFileSync(join(resources, 'node-version'), `${nodeVersion}\n`);
  writeFileSync(join(resources, 'git-sha'), `${resolvedGitSha}\n`);
  writeFileSync(join(resources, 'git-state.json'), `${JSON.stringify(resolvedGitState, null, 2)}\n`);
  writeFileSync(join(resources, 'manifest.json'), `${JSON.stringify({
    application: 'AutoDrill Problem QA',
    appVersion: JSON.parse(readFileSync(join(QA_ROOT, 'package.json'), 'utf8')).version,
    gitSha: resolvedGitSha,
    gitState: resolvedGitState,
    nodeVersion,
  }, null, 2)}\n`);
  chmodSync(executable, 0o755);
  return { destination, executable, gitSha: resolvedGitSha, gitState: resolvedGitState, nodeVersion, runtime };
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const built = buildMacApp();
  console.log(`Built ${built.destination}`);
}
