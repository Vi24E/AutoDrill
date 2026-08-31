#!/usr/bin/env node
import { spawn, spawnSync } from 'node:child_process';
import { existsSync, mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createQaServer } from './server.mjs';

export function findAppBrowser(env = process.env, platform = process.platform) {
  const configured = env.AUTODRILL_QA_BROWSER_PATH;
  const candidates = platform === 'darwin'
    ? [
        configured,
        '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
        '/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge',
        '/Applications/Brave Browser.app/Contents/MacOS/Brave Browser',
        '/Applications/Chromium.app/Contents/MacOS/Chromium',
      ]
    : [configured];
  for (const candidate of candidates.filter(Boolean)) if (existsSync(candidate)) return candidate;
  for (const name of ['google-chrome', 'microsoft-edge', 'brave-browser', 'chromium', 'chromium-browser']) {
    const found = spawnSync('which', [name], { encoding: 'utf8' }).stdout.trim();
    if (found) return found;
  }
  throw new Error('専用windowを開けるChrome / Edge / Brave / Chromiumが見つかりません。AUTODRILL_QA_BROWSER_PATHで実行fileを指定してください。');
}

function defaultBrowserLauncher({ browserBinary, url, profilePath }) {
  return spawn(browserBinary, [
    `--app=${url}`,
    `--user-data-dir=${profilePath}`,
    '--no-first-run',
    '--no-default-browser-check',
    '--disable-background-mode',
    '--disable-background-networking',
    '--disable-component-update',
    '--disable-default-apps',
    '--disable-extensions',
    '--disable-sync',
    '--noerrdialogs',
    '--window-size=1440,1000',
  ], { stdio: 'ignore' });
}

export async function launchDesktop({
  databasePath,
  browserBinary = findAppBrowser(),
  browserLauncher = defaultBrowserLauncher,
  logger = console,
} = {}) {
  const profilePath = mkdtempSync(join(tmpdir(), 'autodrill-qa-window-'));
  const qa = createQaServer({ databasePath, port: 0, host: '127.0.0.1', quiet: true });
  let browser = null;
  let closed = false;

  const close = async ({ terminateBrowser = true } = {}) => {
    if (closed) return;
    closed = true;
    if (terminateBrowser && browser && browser.exitCode == null && !browser.killed) browser.kill('SIGTERM');
    if (qa.server.listening) await qa.close();
    else qa.repository.db.close();
    rmSync(profilePath, { recursive: true, force: true });
  };

  try {
    const address = await qa.listen();
    const url = `http://127.0.0.1:${address.port}`;
    browser = browserLauncher({ browserBinary, url, profilePath });
    logger.log('AutoDrill Problem QAを専用windowで起動しました。windowを閉じると自動終了します。');
    const finished = new Promise((resolveFinished, rejectFinished) => {
      browser.once('error', async (error) => {
        await close({ terminateBrowser: false });
        rejectFinished(error);
      });
      browser.once('exit', async (code, signal) => {
        await close({ terminateBrowser: false });
        if (code === 0 || signal === 'SIGTERM') resolveFinished();
        else rejectFinished(new Error(`専用windowが異常終了しました（code=${code}, signal=${signal ?? 'none'}）。`));
      });
    });
    return { browser, databasePath: qa.databasePath, finished, profilePath, url, close };
  } catch (error) {
    await close();
    throw error;
  }
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const desktop = await launchDesktop();
  for (const signal of ['SIGINT', 'SIGTERM']) {
    process.once(signal, async () => { await desktop.close(); process.exit(0); });
  }
  await desktop.finished;
}
