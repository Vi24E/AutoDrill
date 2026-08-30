import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { existsSync, mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { findAppBrowser, launchDesktop } from '../src/desktop.mjs';
import { buildMacApp } from '../scripts/build-macos-app.mjs';

test('configured app browser has priority', () => {
  assert.equal(findAppBrowser({ AUTODRILL_QA_BROWSER_PATH: process.execPath }, 'test'), process.execPath);
});

test('desktop window uses an ephemeral port and owns server/profile lifecycle', async () => {
  const directory = mkdtempSync(join(tmpdir(), 'autodrill-qa-desktop-test-'));
  const databasePath = join(directory, 'qa.sqlite3');
  let observedUrl = null;
  const browserLauncher = ({ url }) => {
    observedUrl = url;
    const check = `const response=await fetch(${JSON.stringify(`${url}/api/state`)}); if(!response.ok) process.exit(2); const state=await response.json(); if(state.metadata.appVersion!=='0.8.0'||state.metadata.qaSchemaVersion!==5) process.exit(3);`;
    return spawn(process.execPath, ['--input-type=module', '-e', check], { stdio: 'ignore' });
  };
  try {
    const desktop = await launchDesktop({ databasePath, browserBinary: process.execPath, browserLauncher, logger: { log() {} } });
    assert.match(observedUrl, /^http:\/\/127[.]0[.]0[.]1:\d+$/);
    assert.ok(Number(new URL(observedUrl).port) > 0);
    const profilePath = desktop.profilePath;
    assert.equal(existsSync(profilePath), true);
    await desktop.finished;
    assert.equal(existsSync(profilePath), false);
    assert.equal(existsSync(databasePath), true);
    await assert.rejects(fetch(observedUrl));
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});

test('macOS app build contains a self-contained QA runtime', () => {
  const directory = mkdtempSync(join(tmpdir(), 'autodrill-qa-app-build-test-'));
  const destination = join(directory, 'AutoDrill Problem QA.app');
  try {
    const built = buildMacApp({ destination, gitSha: 'packaged-test-sha' });
    assert.equal(existsSync(join(built.runtime, 'src', 'desktop.mjs')), true);
    assert.equal(existsSync(join(built.runtime, 'public', 'app.js')), true);
    assert.equal(existsSync(join(built.runtime, 'generated', 'drill-core-contract.json')), true);
    assert.equal(existsSync(join(built.runtime, 'wasm', 'drill_wasm_bg.wasm')), true);
    assert.equal(existsSync(join(destination, 'Contents', 'Resources', 'git-sha')), true);
    assert.equal(existsSync(join(destination, 'Contents', 'Resources', 'git-state.json')), true);
    assert.equal(built.gitState.head_sha, 'packaged-test-sha');
    assert.equal(existsSync(built.executable), true);
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});
