import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const REPOSITORY_ROOT = resolve(import.meta.dirname, '../../..');

function fingerprint(value) {
  return value ? createHash('sha256').update(value).digest('hex') : null;
}

function readPackagedState(env) {
  const path = env.AUTODRILL_QA_GIT_STATE_PATH;
  if (!path || !existsSync(path)) return null;
  try { return JSON.parse(readFileSync(path, 'utf8')); }
  catch { return null; }
}

export function normalizeGitState(value, fallbackSha = 'unknown') {
  const state = value && typeof value === 'object' ? value : {};
  const headSha = typeof state.head_sha === 'string' && state.head_sha ? state.head_sha : fallbackSha;
  const worktreeState = ['clean', 'dirty', 'unknown'].includes(state.worktree_state) ? state.worktree_state : 'unknown';
  return {
    source: typeof state.source === 'string' ? state.source : 'explicit_sha',
    head_sha: headSha,
    worktree_state: worktreeState,
    worktree_dirty: worktreeState === 'unknown' ? null : worktreeState === 'dirty',
    status_porcelain: typeof state.status_porcelain === 'string' ? state.status_porcelain : null,
    status_sha256: typeof state.status_sha256 === 'string' ? state.status_sha256 : null,
    tracked_diff_sha256: typeof state.tracked_diff_sha256 === 'string' ? state.tracked_diff_sha256 : null,
  };
}

export function captureGitState({ env = process.env, repositoryRoot = REPOSITORY_ROOT } = {}) {
  const packaged = readPackagedState(env);
  if (packaged) return normalizeGitState(packaged, env.AUTODRILL_QA_GIT_SHA ?? 'unknown');
  const explicitSha = env.AUTODRILL_QA_GIT_SHA;
  try {
    const headSha = explicitSha ?? execFileSync('git', ['rev-parse', 'HEAD'], { cwd: repositoryRoot, encoding: 'utf8' }).trim();
    const status = execFileSync('git', ['status', '--porcelain=v1', '--untracked-files=all'], { cwd: repositoryRoot, encoding: 'utf8', maxBuffer: 10_000_000 }).trim();
    const trackedDiff = execFileSync('git', ['diff', '--binary', 'HEAD'], { cwd: repositoryRoot, encoding: 'utf8', maxBuffer: 100_000_000 });
    return normalizeGitState({
      source: 'repository_worktree',
      head_sha: headSha,
      worktree_state: status ? 'dirty' : 'clean',
      status_porcelain: status || null,
      status_sha256: fingerprint(status),
      tracked_diff_sha256: fingerprint(trackedDiff),
    }, headSha);
  } catch {
    return normalizeGitState({ source: explicitSha ? 'explicit_sha' : 'unavailable', head_sha: explicitSha ?? 'unknown' }, explicitSha ?? 'unknown');
  }
}
