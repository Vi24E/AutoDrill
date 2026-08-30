#!/usr/bin/env node
import { spawn, spawnSync } from 'node:child_process';
import { existsSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import net from 'node:net';
import { createQaServer } from '../src/server.mjs';

function chromeBinary() {
  const candidates = [process.env.CHROME_PATH, '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', '/usr/bin/google-chrome', '/usr/bin/chromium'].filter(Boolean);
  for (const candidate of candidates) if (existsSync(candidate)) return candidate;
  for (const name of ['google-chrome', 'chromium']) {
    const found = spawnSync('which', [name], { encoding: 'utf8' }).stdout.trim();
    if (found) return found;
  }
  throw new Error('Chrome/Chromium was not found. Set CHROME_PATH.');
}

function freePort() {
  return new Promise((resolvePort, reject) => {
    const server = net.createServer();
    server.once('error', reject);
    server.listen(0, '127.0.0.1', () => { const port = server.address().port; server.close(() => resolvePort(port)); });
  });
}

async function waitForJson(url, timeout = 15_000) {
  const deadline = Date.now() + timeout;
  let last;
  while (Date.now() < deadline) {
    try { const response = await fetch(url); if (response.ok) return response.json(); last = new Error(`HTTP ${response.status}`); }
    catch (error) { last = error; }
    await new Promise((resolveWait) => setTimeout(resolveWait, 100));
  }
  throw last ?? new Error(`Timed out waiting for ${url}`);
}

async function connect(webSocketDebuggerUrl) {
  const socket = new WebSocket(webSocketDebuggerUrl);
  await new Promise((resolveOpen, reject) => { socket.addEventListener('open', resolveOpen, { once: true }); socket.addEventListener('error', reject, { once: true }); });
  let id = 1;
  const pending = new Map();
  const errors = [];
  socket.addEventListener('message', (event) => {
    const message = JSON.parse(event.data);
    if (message.id && pending.has(message.id)) {
      const { resolve, reject } = pending.get(message.id); pending.delete(message.id);
      if (message.error) reject(new Error(message.error.message)); else resolve(message.result);
    }
    if (message.method === 'Runtime.exceptionThrown') errors.push(message.params.exceptionDetails?.text ?? 'Runtime exception');
    if (message.method === 'Runtime.consoleAPICalled' && message.params.type === 'error') errors.push(message.params.args.map((arg) => arg.value ?? arg.description).join(' '));
  });
  const send = (method, params = {}) => new Promise((resolve, reject) => { const next = id++; pending.set(next, { resolve, reject }); socket.send(JSON.stringify({ id: next, method, params })); });
  const evaluate = async (expression) => {
    const result = await send('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true });
    if (result.exceptionDetails) throw new Error(result.exceptionDetails.exception?.description ?? result.exceptionDetails.text);
    return result.result.value;
  };
  return { socket, send, evaluate, errors };
}

async function launchChrome(url, directory, suffix) {
  const debugPort = await freePort();
  const processHandle = spawn(chromeBinary(), ['--headless=new', '--no-first-run', '--no-default-browser-check', '--disable-background-networking', '--disable-component-update', '--disable-sync', '--disable-extensions', `--remote-debugging-port=${debugPort}`, `--user-data-dir=${join(directory, `chrome-profile-${suffix}`)}`, '--window-size=1440,1000', url], { stdio: 'ignore' });
  const pages = await waitForJson(`http://127.0.0.1:${debugPort}/json/list`);
  const connection = await connect(pages.find((page) => page.type === 'page').webSocketDebuggerUrl);
  await connection.send('Runtime.enable');
  await connection.send('Page.enable');
  return { chrome: processHandle, cdp: connection };
}

async function stopChrome(processHandle, connection) {
  connection?.socket.close();
  if (!processHandle || processHandle.exitCode != null) return;
  processHandle.kill('SIGTERM');
  await new Promise((resolveStop) => {
    const timeout = setTimeout(() => { processHandle.kill('SIGKILL'); resolveStop(); }, 3_000);
    processHandle.once('exit', () => { clearTimeout(timeout); resolveStop(); });
  });
}

async function waitFor(evaluate, expression, label, timeout = 12_000) {
  const deadline = Date.now() + timeout;
  while (Date.now() < deadline) {
    if (await evaluate(expression)) return;
    await new Promise((resolveWait) => setTimeout(resolveWait, 80));
  }
  throw new Error(`Timed out: ${label}`);
}

function setAndInput(selector, value) {
  return `(() => { const e=document.querySelector(${JSON.stringify(selector)}); if(!e) return false; e.value=${JSON.stringify(value)}; e.dispatchEvent(new Event('input',{bubbles:true})); e.dispatchEvent(new Event('change',{bubbles:true})); return true; })()`;
}

const directory = mkdtempSync(join(tmpdir(), 'autodrill-qa-browser-'));
const qa = createQaServer({ databasePath: join(directory, 'qa.sqlite3'), port: 0, gitSha: 'browser-acceptance-sha', quiet: true });
let chrome;
let cdp;
try {
  const address = await qa.listen();
  const qaUrl = `http://127.0.0.1:${address.port}`;
  ({ chrome, cdp } = await launchChrome(qaUrl, directory, 'first'));
  let { evaluate } = cdp;
  await waitFor(evaluate, `document.querySelector('#answer') !== null`, 'automatic random problem');
  const firstAttempt = qa.repository.activeAttempt();
  const firstDetail = qa.repository.attemptDetail(firstAttempt.id);
  if (firstDetail.source !== 'autodrill') throw new Error('Quick flow did not create an AutoDrill item.');
  const leakageBefore = await evaluate(`document.body.innerText.includes('正答') || document.body.innerText.includes('正解') || document.body.innerText.includes('不正解')`);
  if (leakageBefore) throw new Error('Canonical answer or correctness leaked before rating.');

  await evaluate(setAndInput('#answer', 'draft answer'));
  await waitFor(evaluate, `document.querySelector('#save-indicator').innerText.startsWith('保存済み')`, 'draft autosave');
  await cdp.send('Page.reload', { ignoreCache: true });
  await waitFor(evaluate, `document.querySelector('#answer')?.value === 'draft answer'`, 'reload resume');
  if (!await evaluate(`document.querySelector('[data-view="history"]').disabled`)) throw new Error('History navigation was not locked during attempt.');

  await stopChrome(chrome, cdp);
  chrome = null; cdp = null;
  ({ chrome, cdp } = await launchChrome(qaUrl, directory, 'second'));
  ({ evaluate } = cdp);
  await waitFor(evaluate, `document.querySelector('#answer')?.value === 'draft answer'`, 'browser restart resume');

  await evaluate(setAndInput('#answer', firstDetail.canonical_answer.replaceAll('−', '-')));
  await evaluate(`document.dispatchEvent(new KeyboardEvent('keydown',{key:'Enter',ctrlKey:true,bubbles:true}))`);
  await waitFor(evaluate, `document.querySelector('[data-rating-cell]') !== null`, 'rating screen via keyboard submit');
  if (await evaluate(`document.body.innerText.includes('正答') || document.body.innerText.includes('正解')`)) throw new Error('Answer leaked on rating screen.');
  await evaluate(`document.querySelector('[data-prefix="rate"][data-d="3"][data-s="4"]').focus()`);
  await evaluate(`document.activeElement.dispatchEvent(new KeyboardEvent('keydown',{key:'ArrowRight',bubbles:true}))`);
  if (!await evaluate(`document.activeElement.matches('[data-d="4"][data-s="4"]')`)) throw new Error('Rating arrow-key navigation failed.');
  await evaluate(`document.activeElement.click()`);
  await waitFor(evaluate, `!document.querySelector('#confirm-rating').disabled`, 'rating selection');
  await evaluate(`document.querySelector('#rating-note').value='browser annotation'; document.querySelector('#confirm-rating').click()`);
  await waitFor(evaluate, `document.body.innerText.includes('正答') && document.querySelector('#next-problem') !== null`, 'answer reveal');
  if (process.env.AUTODRILL_QA_BROWSER_SCREENSHOT_PATH) {
    const screenshot = await cdp.send('Page.captureScreenshot', { format: 'png', captureBeyondViewport: false });
    writeFileSync(process.env.AUTODRILL_QA_BROWSER_SCREENSHOT_PATH, Buffer.from(screenshot.data, 'base64'));
  }

  await evaluate(`document.querySelector('#next-problem').click()`);
  await waitFor(evaluate, `document.querySelector('#answer') !== null`, 'next random problem');
  const secondAttempt = qa.repository.activeAttempt();
  const secondDetail = qa.repository.attemptDetail(secondAttempt.id);
  await evaluate(setAndInput('#answer', secondDetail.canonical_answer.replaceAll('−', '-')));
  await evaluate(`document.querySelector('#submit-answer').click()`);
  await waitFor(evaluate, `document.querySelector('[data-prefix="rate"]') !== null`, 'second rating');
  await evaluate(`document.querySelector('[data-prefix="rate"][data-d="5"][data-s="2"]').click()`);
  await waitFor(evaluate, `!document.querySelector('#confirm-rating').disabled`, 'second selection');
  await evaluate(`document.querySelector('#confirm-rating').click()`);
  await waitFor(evaluate, `document.querySelector('#next-problem') !== null`, 'second reveal');
  await evaluate(`document.querySelector('#show-history').click()`);
  await waitFor(evaluate, `document.querySelector('tbody tr[data-id]') !== null`, 'history rows');
  if (await evaluate(`document.querySelectorAll('tbody tr[data-id]').length`) !== 2) throw new Error('Random attempts were not both visible in history.');
  await evaluate(`document.querySelector('tbody tr[data-id]').click()`);
  await waitFor(evaluate, `document.querySelector('[data-prefix="revise"]') !== null`, 'history detail');
  await evaluate(`document.querySelector('[data-prefix="revise"][data-d="6"][data-s="3"]').click()`);
  await evaluate(setAndInput('#revision-note', 'browser correction'));
  await evaluate(`document.querySelector('#save-revision').click()`);
  await waitFor(evaluate, `document.querySelectorAll('#history-detail tbody tr').length >= 2`, 'rating revision history');

  const exportCheck = await evaluate(`(async()=>{ const full=await fetch('/api/export/full').then(r=>r.json()); const csv=await fetch('/api/export/analysis.csv').then(r=>r.text()); return {attempts:full.data.attempts.length,events:full.data.input_events.length,autodrill:full.data.items.every(item=>item.source==='autodrill'),sourcePayload:full.data.item_revisions.every(item=>item.original_source_payload_json.includes('autodrill_qa_wasm_v1')),csv:csv.includes('raw_user_answer')&&csv.includes('browser-acceptance-sha')}; })()`);
  if (exportCheck.attempts !== 2 || exportCheck.events < 12 || !exportCheck.autodrill || !exportCheck.sourcePayload || !exportCheck.csv) throw new Error(`Export verification failed: ${JSON.stringify(exportCheck)}`);
  if (cdp.errors.length) throw new Error(`Browser console errors: ${cdp.errors.join(' | ')}`);
  const result = { status: 'passed', operations: ['automatic session start', 'AutoDrill random generation', 'answer typing', 'keyboard submit', 'WASM grading', '7x7 rating', 'blind answer reveal', 'automatic next problem', 'history', 'rating correction', 'reload/resume', 'browser restart/resume', 'full JSON export', 'analysis CSV export'], attempts: 2, localServerRestartCoveredBy: 'server.test.mjs' };
  if (process.env.AUTODRILL_QA_BROWSER_RESULT_PATH) writeFileSync(process.env.AUTODRILL_QA_BROWSER_RESULT_PATH, JSON.stringify(result, null, 2));
  console.log(JSON.stringify(result, null, 2));
} catch (error) {
  if (process.env.AUTODRILL_QA_BROWSER_RESULT_PATH) writeFileSync(process.env.AUTODRILL_QA_BROWSER_RESULT_PATH, JSON.stringify({ status: 'failed', error: error.stack ?? String(error) }, null, 2));
  throw error;
} finally {
  await stopChrome(chrome, cdp);
  await qa.close().catch(() => {});
  rmSync(directory, { recursive: true, force: true });
}
