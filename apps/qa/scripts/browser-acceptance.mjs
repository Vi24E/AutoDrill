#!/usr/bin/env node
import { spawn, spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
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

async function capture(connection, name) {
  if (!process.env.AUTODRILL_QA_BROWSER_SCREENSHOT_DIR) return;
  mkdirSync(process.env.AUTODRILL_QA_BROWSER_SCREENSHOT_DIR, { recursive: true });
  const screenshot = await connection.send('Page.captureScreenshot', { format: 'png', captureBeyondViewport: false });
  writeFileSync(join(process.env.AUTODRILL_QA_BROWSER_SCREENSHOT_DIR, `${name}.png`), Buffer.from(screenshot.data, 'base64'));
}

async function dragRating(connection, evaluate, prefix, xPosition, singularityPosition) {
  await evaluate(`document.querySelector('[data-rating-surface="${prefix}"]').scrollIntoView({block:'center'})`);
  const bounds = await evaluate(`(()=>{const r=document.querySelector('[data-rating-surface="${prefix}"]').getBoundingClientRect();return {left:r.left,top:r.top,width:r.width,height:r.height}})()`);
  const startX = bounds.left + bounds.width * 0.5;
  const startY = bounds.top + bounds.height * 0.5;
  const endX = bounds.left + bounds.width * xPosition;
  const endY = bounds.top + bounds.height * (1 - singularityPosition);
  await connection.send('Input.dispatchMouseEvent', { type: 'mousePressed', x: startX, y: startY, button: 'left', buttons: 1, clickCount: 1 });
  await connection.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: endX, y: endY, button: 'left', buttons: 1 });
  await connection.send('Input.dispatchMouseEvent', { type: 'mouseReleased', x: endX, y: endY, button: 'left', buttons: 0, clickCount: 1 });
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
  await waitFor(evaluate, `document.querySelector('#unit-choice') !== null`, 'unit chooser');
  const unitChooser = await evaluate(`({
    labels: [...document.querySelector('#unit-choice').options].map(option=>option.textContent),
    horizontalOverflow: document.documentElement.scrollWidth > innerWidth,
  })`);
  const hasUnit = (name) => unitChooser.labels.some((label) => label.startsWith(`${name}（`) && label.endsWith('件）'));
  if (unitChooser.labels.length !== 35 || !hasUnit('分数の足し算') || !hasUnit('一次方程式(1)') || !hasUnit('二次方程式(3)') || !hasUnit('すうじはひとりぼっち') || hasUnit('一桁の足し算') || hasUnit('一桁の引き算') || hasUnit('九九') || unitChooser.horizontalOverflow) {
    throw new Error(`Unit chooser contract is broken: ${JSON.stringify(unitChooser)}`);
  }
  await capture(cdp, '00-unit-choice');
  await evaluate(setAndInput('#unit-choice', 'jp.grade7.equation.linear.1'));
  await evaluate(`document.querySelector('#start-unit').click()`);
  await waitFor(evaluate, `document.querySelectorAll('[data-prefix="rate"]').length === 49`, 'automatic rating-only problem');
  try {
    await waitFor(evaluate, `document.querySelector('.print-render-shell.ready') !== null`, 'exact print renderer');
  } catch (error) {
    const rendererState = await evaluate(`(()=>{const f=document.querySelector('.print-render-frame');const d=f?.contentDocument;const cell=d?.querySelector('[data-print-page="answers"] [data-print-problem-index]');return {src:f?.src,body:d?.body?.innerText,ready:d?.body?.dataset?.ready,stage:d?.body?.dataset?.renderStage,error:d?.body?.dataset?.renderError,math:[...(cell?.querySelectorAll('math-span')??[])].map(m=>({host:m.getBoundingClientRect().toJSON(),defined:Boolean(m.shadowRoot),render:m.shadowRoot?.querySelector('[part~="render"]')?.getBoundingClientRect().toJSON()})),html:d?.documentElement?.outerHTML?.slice(0,500)}})()`);
    throw new Error(`${error.message}: ${JSON.stringify(rendererState)}`);
  }
  try {
    await waitFor(evaluate, `document.querySelector('.print-prefetch-frame[data-prefetch-ready="true"]') !== null`, 'next print layout prefetched');
  } catch (error) {
    const state = await evaluate(`(()=>{const f=document.querySelector('.print-prefetch-frame');return {bin:document.querySelector('#print-prefetch-bin')?.innerHTML,src:f?.src,body:f?.contentDocument?.body?.innerText,ready:f?.contentDocument?.body?.dataset?.ready}})()`);
    throw new Error(`${error.message}: ${JSON.stringify(state)}`);
  }
  const firstAttempt = qa.repository.activeAttempt();
  const firstDetail = qa.repository.attemptDetail(firstAttempt.id);
  if (firstDetail.source !== 'autodrill') throw new Error('Quick flow did not create an AutoDrill item.');
  if (firstDetail.unit_name !== '一次方程式(1)' || !firstDetail.problem_representation.includes('x') || firstDetail.original_source_payload.theme.skill_id !== 'jp.grade7.equation.linear.1') throw new Error('Selected linear-equation unit was not generated.');
  const initialUi = await evaluate(`({
    answerVisible: Boolean(document.querySelector('.print-render-frame')?.contentDocument?.querySelector('[data-print-page="answers"] [data-print-problem-index]')),
    hasAnswerInput: Boolean(document.querySelector('#answer, #submit-answer')),
    cells: document.querySelectorAll('[data-prefix="rate"]').length,
    horizontalOverflow: document.documentElement.scrollWidth > innerWidth,
    matrixBottom: Math.round(document.querySelector('.rating-matrix').getBoundingClientRect().bottom),
    confirmBottom: Math.round(document.querySelector('#confirm-rating').getBoundingClientRect().bottom),
    horizontalGap: (()=>{const a=document.querySelectorAll('[data-prefix="rate"]')[0].getBoundingClientRect();const b=document.querySelectorAll('[data-prefix="rate"]')[1].getBoundingClientRect();return Math.round((b.left-a.right)*10)/10})(),
    verticalGap: (()=>{const a=document.querySelectorAll('[data-prefix="rate"]')[0].getBoundingClientRect();const b=document.querySelectorAll('[data-prefix="rate"]')[7].getBoundingClientRect();return Math.round((b.top-a.bottom)*10)/10})(),
    originOffset: (()=>{const s=document.querySelector('.rating-surface').getBoundingClientRect();const o=document.querySelectorAll('[data-prefix="rate"]')[24].getBoundingClientRect();return {x:Math.round((o.left+o.width/2-(s.left+s.width/2))*10)/10,y:Math.round((o.top+o.height/2-(s.top+s.height/2))*10)/10}})(),
    axisText: document.querySelector('.axis-explainer').innerText,
    selectedUnit: document.querySelector('#unit-select').value,
    printRenderer: (()=>{const frame=document.querySelector('.print-render-overlay iframe, .print-render-shell .print-render-frame');const cell=frame.contentDocument.querySelector('[data-print-page="answers"][data-qa-selected-page="true"] [data-qa-mathlive-ready="true"]');const math=[...(cell?.querySelectorAll('math-span')??[])];return {ready:document.querySelector('.print-render-shell').classList.contains('ready'),selectedCell:Boolean(cell),math:math.length,renderedMath:math.length>0&&Number(frame.contentDocument.body.dataset.mathliveReadyCount)===math.length}})(),
    viewportHeight: innerHeight,
  })`);
  if (!initialUi.answerVisible || initialUi.hasAnswerInput || initialUi.cells !== 49 || initialUi.horizontalOverflow || initialUi.confirmBottom > initialUi.viewportHeight || initialUi.horizontalGap !== 0 || initialUi.verticalGap !== 0 || Math.abs(initialUi.originOffset.x) > 0.5 || Math.abs(initialUi.originOffset.y) > 0.5 || !initialUi.axisText.includes('横軸：難しさ') || !initialUi.axisText.includes('縦軸：特異性') || initialUi.selectedUnit !== 'jp.grade7.equation.linear.1' || !initialUi.printRenderer.ready || !initialUi.printRenderer.selectedCell || !initialUi.printRenderer.renderedMath) {
    throw new Error(`Initial rating UI is broken: ${JSON.stringify(initialUi)}`);
  }
  if (firstDetail.observation_mode !== 'rating_only_answer_shown' || firstDetail.raw_user_answer !== null || firstDetail.correctness !== 'ungraded') {
    throw new Error('Rating-only persistence contract was not used.');
  }
  await dragRating(cdp, evaluate, 'rate', 0.713, 0.284);
  await waitFor(evaluate, `!document.querySelector('#confirm-rating').disabled`, 'continuous pointer selection');
  const continuousDraft = qa.repository.activeAttempt().rating_draft;
  if (Math.abs(continuousDraft.difficulty_position - 0.713) > 0.01 || Math.abs(continuousDraft.singularity_position - 0.284) > 0.01) throw new Error(`Continuous position was snapped or lost: ${JSON.stringify(continuousDraft)}`);
  await capture(cdp, '01-rating');

  await cdp.send('Page.reload', { ignoreCache: true });
  await waitFor(evaluate, `document.querySelectorAll('[data-prefix="rate"]').length === 49`, 'reload resume');
  await waitFor(evaluate, `!document.querySelector('[data-rating-cursor]').hidden`, 'continuous cursor reload resume');
  if (await evaluate(`document.querySelector('#confirm-rating').disabled`)) throw new Error('Reloaded continuous rating could not be confirmed.');
  if (qa.repository.activeAttempt().id !== firstAttempt.id) throw new Error('Reload created a duplicate attempt.');
  if (await evaluate(`document.querySelector('[data-view="history"]').disabled`)) throw new Error('History navigation cannot explicitly abandon the current problem.');

  await stopChrome(chrome, cdp);
  chrome = null; cdp = null;
  ({ chrome, cdp } = await launchChrome(qaUrl, directory, 'second'));
  ({ evaluate } = cdp);
  await waitFor(evaluate, `document.querySelectorAll('[data-prefix="rate"]').length === 49`, 'browser restart resume');
  await waitFor(evaluate, `!document.querySelector('[data-rating-cursor]').hidden`, 'continuous cursor browser restart resume');
  if (await evaluate(`document.querySelector('#confirm-rating').disabled`)) throw new Error('Browser-restarted continuous rating could not be confirmed.');
  if (qa.repository.activeAttempt().id !== firstAttempt.id) throw new Error('Browser restart created a duplicate attempt.');

  await evaluate(`document.querySelector('[data-rating-surface="rate"]').focus()`);
  await evaluate(`document.activeElement.dispatchEvent(new KeyboardEvent('keydown',{key:'ArrowRight',bubbles:true}))`);
  await waitFor(evaluate, `document.querySelector('#selection-status').innerText.includes('難しさ')`, 'continuous keyboard movement');
  await evaluate(`document.dispatchEvent(new KeyboardEvent('keydown',{key:'Enter',bubbles:true}))`);
  await waitFor(evaluate, `(async()=>{const s=await fetch('/api/state').then(r=>r.json());return s.activeAttempt?.id!==${JSON.stringify(firstAttempt.id)}&&document.querySelector('.print-render-shell')?.dataset.renderAttempt===s.activeAttempt?.id&&document.querySelectorAll('[data-prefix="rate"]').length===49})()`, 'immediate next problem after keyboard save');
  const prefetchedSwap = await evaluate(`(()=>{const frame=document.querySelector('.print-render-overlay iframe');const cell=frame?.contentDocument?.querySelector('[data-print-page="answers"][data-qa-selected-page="true"] [data-qa-mathlive-ready="true"]');const math=[...(cell?.querySelectorAll('math-span')??[])];return {ready:document.querySelector('.print-render-shell')?.classList.contains('ready'),prefetchSource:frame?.src.includes('prefetch='),fallbackHidden:getComputedStyle(document.querySelector('.print-render-fallback')).visibility==='hidden',renderedMath:math.length>0&&Number(frame.contentDocument.body.dataset.mathliveReadyCount)===math.length}})()`);
  if (!prefetchedSwap.ready || !prefetchedSwap.prefetchSource || !prefetchedSwap.fallbackHidden || !prefetchedSwap.renderedMath) throw new Error(`Prefetched render was not swapped immediately: ${JSON.stringify(prefetchedSwap)}`);
  if (await evaluate(`Boolean(document.querySelector('#next-problem, .saved-panel'))`)) throw new Error('A blocking saved screen remains in the flow.');
  if (qa.repository.activeAttempt().source_skill_id !== 'jp.grade7.equation.linear.1') throw new Error('Auto-next changed the selected unit.');
  if (qa.repository.attemptDetail(qa.repository.activeAttempt().id).original_source_payload.worksheet.identity.seed !== firstDetail.original_source_payload.worksheet.identity.seed) throw new Error('Auto-next did not consume the prefetched worksheet batch.');
  await waitFor(evaluate, `document.querySelector('.print-render-shell.ready') !== null`, 'next exact print render');
  await capture(cdp, '02-next-problem');

  const secondAttempt = qa.repository.activeAttempt();
  await dragRating(cdp, evaluate, 'rate', 0.82, 0.23);
  await waitFor(evaluate, `!document.querySelector('#confirm-rating').disabled`, 'second selection');
  await evaluate(`document.querySelector('#confirm-rating').click()`);
  await waitFor(evaluate, `(async()=>{const s=await fetch('/api/state').then(r=>r.json());return s.activeAttempt?.id!==${JSON.stringify(secondAttempt.id)}&&document.querySelector('.print-render-shell')?.dataset.renderAttempt===s.activeAttempt?.id&&document.querySelectorAll('[data-prefix="rate"]').length===49})()`, 'immediate next problem after second save');
  await waitFor(evaluate, `document.querySelector('.print-render-shell.ready') !== null`, 'third prefetched problem ready');
  await waitFor(evaluate, `document.querySelector('#unit-select option:checked')?.textContent.includes('2件')`, 'selected unit observation count update');
  await evaluate(`window.confirm=()=>true; document.querySelector('[data-view="history"]').click()`);
  try {
    await waitFor(evaluate, `document.querySelector('tbody tr[data-id]') !== null`, 'history rows');
  } catch (error) {
    const state = await evaluate(`({body:document.body.innerText.slice(0,1500),historyActive:document.querySelector('[data-view="history"]')?.classList.contains('active'),historyDisabled:document.querySelector('[data-view="history"]')?.disabled,html:document.querySelector('#app')?.innerHTML.slice(0,1000)})`);
    throw new Error(`${error.message}: ${JSON.stringify(state)}`);
  }
  if (await evaluate(`document.querySelectorAll('tbody tr[data-id]').length`) !== 3) throw new Error('Completed and explicitly abandoned attempts were not all visible in history.');
  if (await evaluate(`document.querySelectorAll('tbody tr[data-state="complete"]').length`) !== 2) throw new Error('The two saved ratings were not both visible in history.');
  await capture(cdp, '03-history');
  await evaluate(`document.querySelector('tbody tr[data-state="complete"]').click()`);
  await waitFor(evaluate, `document.querySelector('[data-prefix="revise"]') !== null`, 'history detail');
  await evaluate(`document.querySelector('#revision-grid').closest('details').open=true`);
  await dragRating(cdp, evaluate, 'revise', 0.91, 0.41);
  await waitFor(evaluate, `!document.querySelector('#save-revision').disabled`, 'continuous revision selection');
  await evaluate(setAndInput('#revision-note', 'browser correction'));
  await evaluate(`document.querySelector('#save-revision').click()`);
  await waitFor(evaluate, `document.querySelectorAll('#history-detail tbody tr').length >= 2`, 'rating revision history');

  const exportCheck = await evaluate(`(async()=>{ const full=await fetch('/api/export/full').then(r=>r.json()); const csv=await fetch('/api/export/analysis.csv').then(r=>r.text()); return {attempts:full.data.attempts.length,completed:full.data.attempts.filter(attempt=>attempt.state==='complete').length,abandoned:full.data.attempts.filter(attempt=>attempt.state==='abandoned').length,events:full.data.input_events.length,autodrill:full.data.items.every(item=>item.source==='autodrill'),ratingOnly:full.data.attempts.every(attempt=>attempt.observation_mode==='rating_only_answer_shown'&&attempt.raw_user_answer===null&&attempt.correctness==='ungraded'),continuous:full.data.evaluations.every(evaluation=>Number.isFinite(evaluation.difficulty_position)&&Number.isFinite(evaluation.singularity_position)),preReveal:full.data.evaluations.every(evaluation=>evaluation.pre_answer_reveal===0),sourcePayload:full.data.item_revisions.every(item=>item.original_source_payload_json.includes('autodrill_qa_wasm_v1')&&item.original_source_payload_json.includes('jp.grade7.equation.linear.1')&&item.original_source_payload_json.includes('generation_git_state')),unitSelection:full.data.selection_events.every(selection=>selection.selection_policy==='autodrill_unit_random_v1'&&JSON.parse(selection.filters_json).selected_skill_id==='jp.grade7.equation.linear.1'&&Number.isInteger(JSON.parse(selection.filters_json).worksheet_problem_index)),gitState:full.data.attempts.every(attempt=>JSON.parse(attempt.autodrill_git_state_json).head_sha==='browser-acceptance-sha'),csv:csv.includes('difficulty_position')&&csv.includes('singularity_position')&&csv.includes('browser-acceptance-sha')&&csv.includes('git_worktree_state')}; })()`);
  if (exportCheck.attempts !== 3 || exportCheck.completed !== 2 || exportCheck.abandoned !== 1 || exportCheck.events < 9 || !exportCheck.autodrill || !exportCheck.ratingOnly || !exportCheck.continuous || !exportCheck.preReveal || !exportCheck.sourcePayload || !exportCheck.unitSelection || !exportCheck.gitState || !exportCheck.csv) throw new Error(`Export verification failed: ${JSON.stringify(exportCheck)}`);
  if (cdp.errors.length) throw new Error(`Browser console errors: ${cdp.errors.join(' | ')}`);
  const result = { status: 'passed', operations: ['unit observation counts', 'all current unit chooser', 'equation unit availability', 'fraction unit availability', 'basic repetition unit exclusion', 'original Rust/WASM selected-unit generation', 'prefetched worksheet same-unit auto-next', 'prefetched print render immediate swap', 'exact production print renderer', 'automatic session start', 'answer shown before rating', 'no answer input', 'continuous 2D pointer drag', 'continuous position persistence', 'clear horizontal difficulty axis', 'clear vertical singularity axis', 'fine-grained arrow-key movement', 'keyboard save', 'immediate next problem without saved screen', 'explicit abandon before history', 'history', 'continuous rating correction', 'reload/resume', 'browser restart/resume', 'viewport overflow check', 'commit state export', 'full JSON export', 'analysis CSV export'], completedAttempts: 2, abandonedAttempts: 1, localServerRestartCoveredBy: 'server.test.mjs' };
  if (process.env.AUTODRILL_QA_BROWSER_RESULT_PATH) writeFileSync(process.env.AUTODRILL_QA_BROWSER_RESULT_PATH, JSON.stringify(result, null, 2));
  console.log(JSON.stringify(result, null, 2));
} catch (error) {
  if (cdp?.errors.length) error.message = `${error.message}\nBrowser errors: ${cdp.errors.join(' | ')}`;
  if (process.env.AUTODRILL_QA_BROWSER_RESULT_PATH) writeFileSync(process.env.AUTODRILL_QA_BROWSER_RESULT_PATH, JSON.stringify({ status: 'failed', error: error.stack ?? String(error) }, null, 2));
  throw error;
} finally {
  await stopChrome(chrome, cdp);
  await qa.close().catch(() => {});
  rmSync(directory, { recursive: true, force: true });
}
