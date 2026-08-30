#!/usr/bin/env node
import { spawn, spawnSync } from 'node:child_process';
import { createServer } from 'node:http';
import { existsSync, mkdtempSync, readFileSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { extname, join, normalize, resolve } from 'node:path';
import net from 'node:net';

const ROOT = resolve(import.meta.dirname, '..');
const OUT = join(ROOT, 'apps/web/out');
const BASE_PATH = '/AutoDrill';
const SEEDS = (process.env.AUTODRILL_SEEDS ?? 'A1b2,M7x9').split(',').filter(Boolean);
const EXTRA_SIGNED_SEEDS = ['Q4r6', 'Z8k3'];
const VIEWPORT = { width: 1600, height: 800, deviceScaleFactor: 1, mobile: false };
const PRINT_ONLY = process.env.AUTODRILL_PRINT_ONLY === 'true';
const ROUTE_FILTER = process.env.AUTODRILL_ROUTE_FILTER ?? '';
const CPU_THROTTLE_RATE = Number(process.env.AUTODRILL_CPU_THROTTLE_RATE ?? '1');
const CPU_THROTTLE_THEME_ID = Number(process.env.AUTODRILL_CPU_THROTTLE_THEME_ID ?? '23');
const GENERATION_PROBE = process.env.AUTODRILL_GENERATION_PROBE === 'true' || CPU_THROTTLE_RATE > 1;
const SKIP_PRINT_PROBES = process.env.AUTODRILL_SKIP_PRINT_PROBES === 'true';


if (!existsSync(join(OUT, 'index.html'))) {
  throw new Error('apps/web/out is missing. Run the GitHub Pages build before browser layout verification.');
}

function mimeType(file) {
  return ({ '.html': 'text/html; charset=utf-8', '.js': 'application/javascript; charset=utf-8', '.css': 'text/css; charset=utf-8', '.wasm': 'application/wasm', '.svg': 'image/svg+xml', '.woff2': 'font/woff2', '.json': 'application/json; charset=utf-8', '.xml': 'application/xml; charset=utf-8' })[extname(file)] ?? 'application/octet-stream';
}

function resolveStaticFile(urlPath) {
  let pathname = decodeURIComponent(urlPath.split('?')[0] ?? '/');
  if (!pathname.startsWith(BASE_PATH)) return null;
  pathname = pathname.slice(BASE_PATH.length) || '/';
  const safe = normalize(pathname).replace(/^([.][.][/\\])+/, '').replace(/^[/\\]+/, '');
  let candidate = join(OUT, safe);
  if (existsSync(candidate) && statSync(candidate).isDirectory()) candidate = join(candidate, 'index.html');
  if (!existsSync(candidate) && existsSync(`${candidate}.html`)) candidate = `${candidate}.html`;
  return existsSync(candidate) ? candidate : null;
}

const server = createServer((req, res) => {
  const file = resolveStaticFile(req.url ?? '/');
  if (!file) {
    res.writeHead(404, { 'Content-Type': 'text/plain; charset=utf-8' });
    res.end('not found');
    return;
  }
  res.writeHead(200, { 'Content-Type': mimeType(file), 'Cache-Control': 'no-store' });
  res.end(readFileSync(file));
});

function chromeBinary() {
  const candidates = [
    process.env.CHROME_PATH,
    '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
    '/usr/bin/google-chrome',
    '/usr/bin/google-chrome-stable',
    '/usr/bin/chromium',
    '/usr/bin/chromium-browser',
  ].filter(Boolean);
  for (const candidate of candidates) if (existsSync(candidate)) return candidate;
  for (const name of ['google-chrome', 'google-chrome-stable', 'chromium', 'chromium-browser']) {
    const found = spawnSync('which', [name], { encoding: 'utf8' }).stdout.trim();
    if (found) return found;
  }
  throw new Error('Chrome/Chromium was not found. Set CHROME_PATH for browser layout verification.');
}

function freePort() {
  return new Promise((resolvePort, reject) => {
    const probe = net.createServer();
    probe.once('error', reject);
    probe.listen(0, '127.0.0.1', () => {
      const address = probe.address();
      const port = typeof address === 'object' && address ? address.port : null;
      probe.close(() => port ? resolvePort(port) : reject(new Error('Could not allocate a Chrome debugging port.')));
    });
  });
}

async function waitForJson(url, timeoutMs = 15_000) {
  const deadline = Date.now() + timeoutMs;
  let last;
  while (Date.now() < deadline) {
    try {
      const response = await fetch(url);
      if (response.ok) return response.json();
      last = new Error(`HTTP ${response.status}`);
    } catch (error) { last = error; }
    await new Promise((resolveWait) => setTimeout(resolveWait, 100));
  }
  throw last ?? new Error(`Timed out waiting for ${url}`);
}

async function connectCdp(webSocketDebuggerUrl) {
  const ws = new WebSocket(webSocketDebuggerUrl);
  await new Promise((resolveOpen, reject) => {
    ws.addEventListener('open', resolveOpen, { once: true });
    ws.addEventListener('error', reject, { once: true });
  });
  let id = 1;
  const pending = new Map();
  const consoleErrors = [];
  ws.addEventListener('message', (event) => {
    const message = JSON.parse(event.data);
    if (message.id && pending.has(message.id)) {
      const item = pending.get(message.id);
      pending.delete(message.id);
      if (message.error) item.reject(new Error(message.error.message));
      else item.resolve(message.result);
      return;
    }
    if (message.method === 'Runtime.consoleAPICalled' && message.params.type === 'error') {
      consoleErrors.push(message.params.args.map((arg) => arg.value ?? arg.description ?? '').join(' '));
    }
    if (message.method === 'Runtime.exceptionThrown') {
      consoleErrors.push(message.params.exceptionDetails?.exception?.description ?? message.params.exceptionDetails?.text ?? 'Runtime exception');
    }
  });
  const send = (method, params = {}) => new Promise((resolveSend, reject) => {
    const messageId = id++;
    pending.set(messageId, { resolve: resolveSend, reject });
    ws.send(JSON.stringify({ id: messageId, method, params }));
  });
  const evaluate = async (expression) => {
    const result = await send('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true });
    if (result.exceptionDetails) throw new Error(result.exceptionDetails.exception?.description ?? result.exceptionDetails.text);
    return result.result?.value;
  };
  return { ws, send, evaluate, consoleErrors };
}

async function navigate(cdp, url) {
  cdp.consoleErrors.length = 0;
  await cdp.send('Page.navigate', { url });
  const deadline = Date.now() + 12_000;
  while (Date.now() < deadline) {
    if (await cdp.evaluate('document.readyState === "complete"')) return;
    await new Promise((resolveWait) => setTimeout(resolveWait, 50));
  }
  throw new Error(`Timed out loading ${url}`);
}

async function mouseClick(cdp, x, y) {
  await cdp.send('Input.dispatchMouseEvent', { type: 'mousePressed', x, y, button: 'left', clickCount: 1 });
  await cdp.send('Input.dispatchMouseEvent', { type: 'mouseReleased', x, y, button: 'left', clickCount: 1 });
}

async function typeKeyboardText(cdp, text) {
  for (const character of text) {
    await cdp.send('Input.dispatchKeyEvent', { type: 'keyDown', text: character, key: character, code: `Digit${character}` });
    await cdp.send('Input.dispatchKeyEvent', { type: 'keyUp', key: character, code: `Digit${character}` });
    await new Promise((resolveWait) => setTimeout(resolveWait, 80));
  }
}

async function verifySelectHitbox(cdp, ariaLabel) {
  const geometry = await cdp.evaluate(`(async () => {
    const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
    let trigger = null;
    for (let i = 0; i < 120 && !trigger; i += 1) {
      trigger = document.querySelector('button[aria-label=${JSON.stringify(ariaLabel)}]');
      if (!trigger) await sleep(25);
    }
    if (!trigger) throw new Error('Missing select trigger: ${ariaLabel}');
    const field = trigger.closest('.field-group');
    const label = field?.querySelector('.field-label');
    const t = trigger.getBoundingClientRect();
    const l = label?.getBoundingClientRect();
    return {
      trigger: { left: t.left, right: t.right, top: t.top, bottom: t.bottom },
      label: l ? { left: l.left, right: l.right, top: l.top, bottom: l.bottom } : null,
    };
  })()`);
  const t = geometry.trigger;
  await mouseClick(cdp, t.left + Math.min(12, (t.right - t.left) / 2), t.top - 2);
  let expanded = await cdp.evaluate(`document.querySelector('button[aria-label=${JSON.stringify(ariaLabel)}]').getAttribute('aria-expanded')`);
  if (expanded !== 'false') throw new Error(`${ariaLabel} select opened from a click just outside its visible border.`);
  if (geometry.label) {
    const l = geometry.label;
    await mouseClick(cdp, l.left + Math.min(12, (l.right - l.left) / 2), (l.top + l.bottom) / 2);
    expanded = await cdp.evaluate(`document.querySelector('button[aria-label=${JSON.stringify(ariaLabel)}]').getAttribute('aria-expanded')`);
    if (expanded !== 'false') throw new Error(`${ariaLabel} select opened from a click on the visual label above it.`);
  }
}

const dropdownProbe = `(async () => {
  const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
  const waitFor = async (fn, label) => {
    for (let i = 0; i < 120; i += 1) { const value = fn(); if (value) return value; await sleep(25); }
    throw new Error('Timed out waiting for ' + label);
  };
  const hitTestLastOption = async (ariaLabel) => {
    const trigger = await waitFor(() => document.querySelector('button[aria-label="' + ariaLabel + '"]'), ariaLabel);
    trigger.click();
    const listbox = await waitFor(() => document.querySelector('[role="listbox"][aria-label="' + ariaLabel + 'の選択肢"]'), ariaLabel + ' listbox');
    listbox.scrollTop = listbox.scrollHeight;
    const options = [...listbox.querySelectorAll('[role="option"]')];
    const last = options.at(-1);
    last.scrollIntoView({ block: 'nearest' });
    await new Promise(requestAnimationFrame);
    const rect = last.getBoundingClientRect();
    const listRect = listbox.getBoundingClientRect();
    const x = Math.min(rect.right - 2, Math.max(rect.left + 2, (rect.left + rect.right) / 2));
    const y = Math.min(rect.bottom - 2, Math.max(rect.top + 2, (rect.top + rect.bottom) / 2));
    const hit = document.elementFromPoint(x, y)?.closest('[role="option"]');
    const visible = rect.top >= Math.max(0, listRect.top) - 1
      && rect.bottom <= Math.min(innerHeight, listRect.bottom) + 1
      && hit === last;
    const lastLabel = last.getAttribute('aria-label');
    if (visible) {
      last.click();
      await sleep(20);
      await new Promise(requestAnimationFrame);
    }
    return { visible, lastLabel, selected: trigger.getAttribute('data-value'), listRect: { top: listRect.top, bottom: listRect.bottom }, rect: { top: rect.top, bottom: rect.bottom } };
  };
  await waitFor(() => document.querySelector('button[aria-label="難易度"]'), 'hydration');
  const difficulty = await hitTestLastOption('難易度');
  document.querySelector('button[aria-label="学年から選ぶ"]').click();
  await sleep(20);
  const grade = await hitTestLastOption('学年');
  return { difficulty, grade };
})()`;

function settingsOptionCoverageProbe() {
  return `(async () => {
    const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
    const waitFor = async (fn, label) => {
      for (let i = 0; i < 400; i += 1) { const value = fn(); if (value) return value; await sleep(25); }
      throw new Error('Timed out waiting for ' + label);
    };
    await waitFor(() => document.querySelector('button[aria-label="難易度"]'), 'settings hydration');
    const choose = async (ariaLabel, optionLabel) => {
      const trigger = await waitFor(() => document.querySelector('button[aria-label="' + ariaLabel + '"]'), ariaLabel);
      if (trigger.getAttribute('aria-expanded') !== 'true') trigger.click();
      const listbox = await waitFor(() => document.querySelector('[role="listbox"][aria-label="' + ariaLabel + 'の選択肢"]'), ariaLabel + ' listbox');
      const option = [...listbox.querySelectorAll('[role="option"]')].find((item) => item.getAttribute('aria-label') === optionLabel);
      if (!option) throw new Error('Missing ' + ariaLabel + ' option ' + optionLabel);
      option.click();
      await waitFor(() => document.querySelector('button[aria-label="' + ariaLabel + '"]')?.getAttribute('data-selected-label') === optionLabel, ariaLabel + '=' + optionLabel);
    };
    const labels = async (ariaLabel) => {
      const trigger = await waitFor(() => document.querySelector('button[aria-label="' + ariaLabel + '"]'), ariaLabel);
      if (trigger.getAttribute('aria-expanded') !== 'true') trigger.click();
      const listbox = await waitFor(() => document.querySelector('[role="listbox"][aria-label="' + ariaLabel + 'の選択肢"]'), ariaLabel + ' listbox');
      const result = [...listbox.querySelectorAll('[role="option"]')].map((item) => item.getAttribute('aria-label')).filter(Boolean);
      trigger.click();
      await sleep(10);
      return result;
    };

    const visitedThemes = new Set();
    const difficultyLabels = await labels('難易度');
    for (const label of difficultyLabels) await choose('難易度', label);

    const recommendedButton = [...document.querySelectorAll('.selection-mode-tabs button')].find((button) => button.textContent?.trim() === 'おすすめ');
    recommendedButton?.click();
    await sleep(20);
    const recommendedGenres = await labels('ジャンル');
    for (const genre of recommendedGenres) {
      await choose('ジャンル', genre);
      for (const theme of await labels('テーマ')) {
        await choose('テーマ', theme);
        visitedThemes.add(theme);
      }
    }

    document.querySelector('button[aria-label="学年から選ぶ"]')?.click();
    await waitFor(() => document.querySelector('button[aria-label="学年"]'), 'grade select');
    const gradeLabels = await labels('学年');
    for (const grade of gradeLabels) {
      await choose('学年', grade);
      for (const genre of await labels('ジャンル')) {
        await choose('ジャンル', genre);
        for (const theme of await labels('テーマ')) {
          await choose('テーマ', theme);
          visitedThemes.add(theme);
        }
      }
    }
    return {
      difficulties: difficultyLabels.length,
      grades: gradeLabels.length,
      recommendedGenres: recommendedGenres.length,
      themes: visitedThemes.size,
      alert: document.querySelector('[role="alert"]')?.getAttribute('aria-label') ?? null,
    };
  })()`;
}

function uiStateGraphProbe() {
  return `(async () => {
    const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
    const waitFor = async (fn, label) => {
      for (let i = 0; i < 500; i += 1) { const value = fn(); if (value) return value; await sleep(25); }
      throw new Error('Timed out waiting for ' + label);
    };
    const visible = (element) => {
      const details = element.closest('details');
      if (details && !details.open && !element.matches('summary')) return false;
      if (element.matches('math-field.answer-mathfield') && element.getAttribute('aria-readonly') === 'true') return false;
      const style = getComputedStyle(element);
      const rect = element.matches('math-field.answer-mathfield')
        ? (element.closest('.answer-box')?.getBoundingClientRect() ?? element.getBoundingClientRect())
        : element.getBoundingClientRect();
      return style.display !== 'none' && style.visibility !== 'hidden' && rect.width > 0 && rect.height > 0 && !element.disabled;
    };
    const text = (element) => element.getAttribute('aria-label') || element.textContent?.trim() || '';
    const signature = (element) => {
      if (element.closest('.worksheet-print-preview')) {
        if (element.matches('input[type="checkbox"]')) return 'preview:解答を逆さにする';
        return 'preview:' + text(element);
      }
      if (element.closest('.grading-settings-modal')) return 'modal:' + text(element);
      if (element.closest('.input-panel')) return 'input:' + text(element);
      if (element.matches('math-field.answer-mathfield')) return 'problem:math-field';
      if (element.matches('button[data-column-digit-index]')) return 'problem:column-digit';
      if (element.matches('button[data-digit-grid-cell]')) return 'problem:digit-grid-cell';
      if (element.matches('button.liar-person-choice')) return 'problem:liar-choice';
      if (element.closest('.grade-actions')) return 'graded:' + text(element);
      if (element.closest('.worksheet-screen') && ['採点', '印刷', 'TOPに戻る'].includes(text(element))) return 'worksheet:' + text(element);
      if (element.matches('.furigana-toggle input')) return 'settings:furigana';
      if (element.matches('button[role="combobox"]')) return 'settings:select:' + text(element);
      if (element.matches('.selection-mode-tabs button')) return 'settings:mode:' + text(element);
      if (element.matches('.advanced-settings > summary')) return 'settings:advanced';
      if (element.matches('input[aria-label="Seed"]')) return 'settings:seed';
      if (element.matches('.grading-settings-open-button')) return 'settings:grading-settings';
      if (element.matches('.primary-button')) return 'settings:generate';
      if (element.matches('.secondary-button')) return 'settings:print';
      return 'unknown:' + element.tagName.toLowerCase() + ':' + text(element);
    };
    const census = (root) => [...new Set([...root.querySelectorAll('button, input, summary, math-field.answer-mathfield')]
      .filter(visible).map(signature))].sort();
    const states = {};
    const edges = [];
    const edge = (name, ok, detail = null) => edges.push({ name, ok: Boolean(ok), detail });
    const settings = await waitFor(() => document.querySelector('.settings-screen'), 'settings');
    await waitFor(() => document.querySelector('button[aria-label="難易度"]'), 'settings hydration');
    states.settings = census(settings);

    const furigana = settings.querySelector('.furigana-toggle input');
    const furiganaBefore = furigana.checked;
    furigana.click(); await sleep(20);
    edge('settings.furigana.toggle', furigana.checked !== furiganaBefore);
    furigana.click(); await sleep(20);

    document.querySelector('button[aria-label="学年から選ぶ"]')?.click(); await sleep(20);
    states.settingsGrade = census(settings);
    edge('settings.mode.grade', Boolean(document.querySelector('button[aria-label="学年"]')));
    [...document.querySelectorAll('.selection-mode-tabs button')].find((button) => button.textContent?.trim() === 'おすすめ')?.click(); await sleep(20);
    edge('settings.mode.recommended', !document.querySelector('button[aria-label="学年"]'));

    const advanced = settings.querySelector('.advanced-settings');
    const summary = advanced?.querySelector('summary');
    summary?.click(); await sleep(20);
    states.settingsAdvanced = census(settings);
    edge('settings.advanced.open', advanced?.open === true);
    const seed = document.querySelector('input[aria-label="Seed"]');
    const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set;
    setter.call(seed, 'UiGraph');
    seed.dispatchEvent(new Event('input', { bubbles: true }));
    seed.dispatchEvent(new Event('change', { bubbles: true }));
    await sleep(20);
    edge('settings.seed.edit', seed.value === 'UiGraph');
    summary?.click(); await sleep(20);
    edge('settings.advanced.close', advanced?.open === false);
    summary?.click(); await sleep(20);

    const openModal = async () => {
      [...document.querySelectorAll('button')].find((button) => button.textContent?.includes('採点設定'))?.click();
      return waitFor(() => document.querySelector('.grading-settings-modal'), 'grading settings modal');
    };
    let modal = await openModal();
    states.gradingModal = census(modal);
    for (const button of [...modal.querySelectorAll('.grading-setting-toggle button')]) {
      button.click(); await sleep(10);
      edge('grading-modal.toggle.' + text(button), button.getAttribute('aria-pressed') === 'true');
    }
    modal.querySelector('button[aria-label="採点設定を閉じる"]')?.click();
    await waitFor(() => !document.querySelector('.grading-settings-modal') && true, 'modal close button');
    edge('grading-modal.close-button', true);
    modal = await openModal();
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    await waitFor(() => !document.querySelector('.grading-settings-modal') && true, 'modal Escape close');
    edge('grading-modal.escape', true);
    modal = await openModal();
    const backdrop = modal.closest('.grading-settings-modal-backdrop');
    backdrop?.dispatchEvent(new MouseEvent('mousedown', { bubbles: true }));
    await waitFor(() => !document.querySelector('.grading-settings-modal') && true, 'modal backdrop close');
    edge('grading-modal.backdrop', true);

    window.__AUTODRILL_STATE_GRAPH_PRINTS__ = 0;
    window.print = () => { window.__AUTODRILL_STATE_GRAPH_PRINTS__ += 1; };
    const previewFromSettings = async (closeWithEscape = false) => {
      document.querySelector('button[aria-label="印刷 (pdfで出力)"]')?.click();
      const preview = await waitFor(() => document.querySelector('.worksheet-print-preview'), 'settings print preview');
      states.printPreview = states.printPreview ?? census(preview);
      if (!closeWithEscape) {
        const rotate = preview.querySelector('input[type="checkbox"]');
        const before = rotate.checked; rotate.click(); await sleep(20);
        edge('print-preview.rotate', rotate.checked !== before);
        preview.querySelector('.worksheet-print-preview-print')?.click();
        await waitFor(() => window.__AUTODRILL_STATE_GRAPH_PRINTS__ > 0, 'native print call');
        edge('print-preview.print', window.__AUTODRILL_STATE_GRAPH_PRINTS__ > 0);
        preview.querySelector('.worksheet-print-preview-back')?.click();
      } else {
        window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
      }
      await waitFor(() => !document.querySelector('.worksheet-print-preview') && true, 'preview close');
      await waitFor(() => {
        const generateButton = document.querySelector('button.primary-button');
        return generateButton && !generateButton.disabled ? generateButton : null;
      }, 'settings ready after preview close');
      return Boolean(document.querySelector('.settings-screen'));
    };
    edge('settings.print.preview.back', await previewFromSettings(false));
    edge('settings.print.preview.escape', await previewFromSettings(true));

    const generate = async () => {
      document.querySelector('button[aria-label="問題生成"]')?.click();
      return waitFor(() => {
        const worksheet = document.querySelector('.worksheet-screen');
        if (worksheet) return worksheet;
        const error = document.querySelector('.error-message');
        if (error) throw new Error('Worksheet generation failed: ' + (error.getAttribute('aria-label') || error.textContent || 'unknown error'));
        return null;
      }, 'worksheet');
    };
    let worksheet = await generate();
    edge('settings.generate.worksheet', Boolean(worksheet));
    let firstField = await waitFor(() => worksheet.querySelector('math-field.answer-mathfield[aria-readonly="false"]'), 'first answer field');
    states.worksheetEditing = census(worksheet);
    firstField.click();
    let panel = await waitFor(() => document.querySelector('.input-panel'), 'input panel');
    states.inputPanel = census(worksheet);
    edge('worksheet.answer.select', Boolean(panel));
    panel.querySelector('button[aria-label="入力パネルを閉じる"]')?.click();
    await waitFor(() => !document.querySelector('.input-panel') && true, 'input panel close');
    edge('input-panel.close-button', true);

    firstField.click();
    await waitFor(() => document.querySelector('.input-panel'), 'input panel before Escape');
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    await waitFor(() => !document.querySelector('.input-panel') && true, 'input panel Escape close');
    edge('input-panel.escape', true);

    firstField.click();
    await waitFor(() => document.querySelector('.input-panel'), 'input panel before print');
    document.querySelector('button[aria-label="印刷"]')?.click();
    let inputPreview = await waitFor(() => document.querySelector('.worksheet-print-preview'), 'input-state print preview');
    inputPreview.querySelector('.worksheet-print-preview-back')?.click();
    await waitFor(() => !document.querySelector('.worksheet-print-preview') && true, 'input-state preview close');
    edge('input-panel.print.back', Boolean(document.querySelector('.input-panel')) && firstField.closest('.answer-box')?.classList.contains('answer-box-selected'));

    document.querySelector('button[aria-label="採点"]')?.click();
    await waitFor(() => document.querySelector('.grade-result-panel'), 'graded from input panel');
    edge('input-panel.grade', !document.querySelector('.input-panel'));
    document.querySelector('button[aria-label="問題に戻る"]')?.click();
    await waitFor(() => document.querySelector('button[aria-label="採点"]')?.getAttribute('aria-pressed') === 'false', 'editing after input-panel grade');

    firstField = await waitFor(() => document.querySelector('math-field.answer-mathfield[aria-readonly="false"]'), 'field before input-panel TOP');
    firstField.click();
    await waitFor(() => document.querySelector('.input-panel'), 'input panel before TOP');
    document.querySelector('button[aria-label="TOPに戻る"]')?.click();
    await waitFor(() => document.querySelector('.settings-screen'), 'TOP from input panel');
    edge('input-panel.top', !document.querySelector('.input-panel'));

    worksheet = await generate();
    firstField = await waitFor(() => worksheet.querySelector('math-field.answer-mathfield[aria-readonly="false"]'), 'field after input-panel transitions');

    const worksheetPreview = async (closeWithEscape = false) => {
      document.querySelector('button[aria-label="印刷"]')?.click();
      const preview = await waitFor(() => document.querySelector('.worksheet-print-preview'), 'worksheet print preview');
      if (closeWithEscape) window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
      else preview.querySelector('.worksheet-print-preview-back')?.click();
      await waitFor(() => !document.querySelector('.worksheet-print-preview') && true, 'worksheet preview close');
      return Boolean(document.querySelector('.worksheet-screen'));
    };
    edge('worksheet.editing.print.back', await worksheetPreview(false));
    edge('worksheet.editing.print.escape', await worksheetPreview(true));
    document.querySelector('button[aria-label="TOPに戻る"]')?.click();
    await waitFor(() => document.querySelector('.settings-screen'), 'TOP after editing');
    edge('worksheet.editing.top', true);

    worksheet = await generate();
    const answer = await waitFor(() => worksheet.querySelector('math-field.answer-mathfield[aria-readonly="false"]'), 'answer for preserve edge');
    answer.click();
    const answerPanel = await waitFor(() => document.querySelector('.input-panel'), 'answer panel');
    [...answerPanel.querySelectorAll('.keypad-numbers button')].find((button) => button.textContent?.trim() === '7')?.click();
    await waitFor(() => answer.value === '7', 'MathLive value 7');
    await waitFor(() => [...document.querySelectorAll('math-field.answer-mathfield')].some((field) => field.getAttribute('aria-label')?.endsWith('答え 7')), 'accepted answer 7');
    document.querySelector('button[aria-label="採点"]')?.click();
    await waitFor(() => document.querySelector('.grade-result-panel'), 'graded worksheet');
    await waitFor(() => {
      const fields = [...document.querySelectorAll('math-field.answer-mathfield')];
      return fields.length > 0 && fields.every((field) => field.getAttribute('aria-readonly') === 'true');
    }, 'graded read-only answer fields');
    states.worksheetGraded = census(document.querySelector('.worksheet-screen'));
    edge('worksheet.editing.grade', true);
    edge('worksheet.graded.print.back', await worksheetPreview(false));
    edge('worksheet.graded.print.escape', await worksheetPreview(true));
    document.querySelector('button[aria-label="問題に戻る"]')?.click();
    await waitFor(() => document.querySelector('button[aria-label="採点"]')?.getAttribute('aria-pressed') === 'false', 'return to editing');
    const returnedValues = [...document.querySelectorAll('math-field.answer-mathfield')].map((field) => field.value);
    edge('worksheet.graded.return', returnedValues.includes('7'), returnedValues.slice(0, 4));

    document.querySelector('button[aria-label="採点"]')?.click();
    await waitFor(() => document.querySelector('.grade-result-panel'), 'graded before retry');
    document.querySelector('button[aria-label="もう一回問題を解く"]')?.click();
    await waitFor(() => document.querySelector('button[aria-label="採点"]')?.getAttribute('aria-pressed') === 'false', 'retry editing');
    const retryValues = [...document.querySelectorAll('math-field.answer-mathfield')].map((field) => field.value);
    edge('worksheet.graded.retry', retryValues.length > 0 && retryValues.every((value) => value === ''), retryValues.slice(0, 4));

    document.querySelector('button[aria-label="採点"]')?.click();
    await waitFor(() => document.querySelector('.grade-result-panel'), 'graded before TOP');
    document.querySelector('button[aria-label="TOPに戻る"]')?.click();
    await waitFor(() => document.querySelector('.settings-screen'), 'TOP after graded');
    edge('worksheet.graded.top', true);

    return {
      states,
      edges,
      obsoleteDifferentWorksheetPresent: Boolean([...document.querySelectorAll('button')].find((button) => text(button) === '別の問題を解く')),
      alert: document.querySelector('[role="alert"]')?.getAttribute('aria-label') ?? null,
    };
  })()`;
}

function worksheetAffordanceCoverageProbe() {
  return `(async () => {
    const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
    const waitFor = async (fn, label) => {
      for (let i = 0; i < 300; i += 1) { const value = fn(); if (value) return value; await sleep(20); }
      throw new Error('Timed out waiting for ' + label);
    };
    const failures = [];
    const cells = [...document.querySelectorAll('.worksheet-screen .problem-cell')];
    let attempted = 0;
    const noFatal = (label) => {
      const alert = document.querySelector('[role="alert"]')?.getAttribute('aria-label') ?? null;
      const notice = document.querySelector('.worksheet-toast')?.getAttribute('aria-label') ?? null;
      if (alert || notice === '式が大きすぎます！') failures.push({ label, alert, notice });
    };
    for (const [problemIndex, cell] of cells.entries()) {
      const controls = [
        ...cell.querySelectorAll('button[data-column-digit-index]:not(:disabled)'),
        ...cell.querySelectorAll('button[data-digit-grid-cell]:not(:disabled)'),
        ...cell.querySelectorAll('button.liar-person-choice:not(:disabled)'),
        ...cell.querySelectorAll('math-field.answer-mathfield[aria-readonly="false"]'),
      ];
      for (const control of controls) {
        attempted += 1;
        const label = control.getAttribute('aria-label') ?? ('problem ' + (problemIndex + 1));
        if (control.matches('button.liar-person-choice')) {
          const before = control.getAttribute('aria-pressed');
          control.click(); await sleep(20);
          if (control.getAttribute('aria-pressed') === before) failures.push({ label, reason: 'liar choice did not toggle' });
          noFatal(label);
          continue;
        }
        control.click();
        const panel = await waitFor(() => document.querySelector('.input-panel'), label + ' input panel');
        const numberButtons = [...panel.querySelectorAll('.keypad-numbers button:not(:disabled)')];
        const digit = control.matches('button[data-digit-grid-cell]') ? (numberButtons[0]?.textContent?.trim() ?? '1') : '1';
        const digitButton = numberButtons.find((button) => button.textContent?.trim() === digit) ?? numberButtons[0];
        if (!digitButton) { failures.push({ label, reason: 'no enabled number key' }); continue; }
        if (control.matches('math-field.answer-mathfield')) {
          const two = numberButtons.find((button) => button.textContent?.trim() === '2') ?? digitButton;
          two.click();
          await waitFor(() => String(control.value ?? '').endsWith(two.textContent?.trim() ?? ''), label + ' first digit');
          digitButton.click();
          const expected = (two.textContent?.trim() ?? '') + (digitButton.textContent?.trim() ?? '');
          await waitFor(() => String(control.value ?? '') === expected, label + ' multi-digit input');
        } else {
          digitButton.click(); await sleep(20);
          const rendered = control.textContent?.trim() ?? '';
          if (!rendered.includes(digitButton.textContent?.trim() ?? '')) failures.push({ label, reason: 'digit did not render', rendered });
        }
        noFatal(label);
      }
    }
    return { attempted, cells: cells.length, failures };
  })()`;
}

function openInputPanelCoverageProbe() {
  return `(async () => {
    const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
    const waitFor = async (fn, label) => {
      for (let i = 0; i < 200; i += 1) { const value = fn(); if (value) return value; await sleep(20); }
      throw new Error('Timed out waiting for ' + label);
    };
    const target = document.querySelector('math-field.answer-mathfield[aria-readonly="false"], button[data-column-digit-index]:not(:disabled), button[data-digit-grid-cell]:not(:disabled)');
    if (!target) return { applicable: false };
    const kind = target.matches('math-field.answer-mathfield') ? 'math-field' : target.matches('button[data-column-digit-index]') ? 'column-digit' : 'digit-grid';
    target.click();
    const panel = await waitFor(() => document.querySelector('.input-panel'), 'input panel');
    const actions = [...panel.querySelectorAll('button:not(:disabled)')].map((button) => button.getAttribute('aria-label') || button.textContent?.trim() || '').filter(Boolean);
    return { applicable: true, kind, actions, signature: kind + '::' + actions.join('|') };
  })()`;
}

function exerciseInputPanelActionsProbe() {
  return `(async () => {
    const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
    const waitFor = async (fn, label) => {
      for (let i = 0; i < 240; i += 1) { const value = fn(); if (value) return value; await sleep(20); }
      throw new Error('Timed out waiting for ' + label);
    };
    const targetSelector = 'math-field.answer-mathfield[aria-readonly="false"], button[data-column-digit-index]:not(:disabled), button[data-digit-grid-cell]:not(:disabled)';
    const targets = () => [...document.querySelectorAll(targetSelector)];
    const open = async (targetIndex) => {
      const candidates = targets();
      const target = candidates[targetIndex % candidates.length];
      if (!target) throw new Error('No editable target for input-panel action coverage');
      target.click();
      const selectedTarget = await waitFor(() => {
        if (target.matches('math-field.answer-mathfield')) {
          return target.closest('.answer-box')?.classList.contains('answer-box-selected') ? target : null;
        }
        if (target.matches('button[data-column-digit-index]')) {
          return target.classList.contains('column-digit-slot-selected') ? target : null;
        }
        if (target.matches('button[data-digit-grid-cell]')) {
          return target.classList.contains('digit-grid-cell-selected') ? target : null;
        }
        return null;
      }, 'clicked editable target selection');
      const panel = await waitFor(() => document.querySelector('.input-panel'), 'input panel');
      await sleep(50);
      return { target: selectedTarget, panel };
    };
    const actionName = (button) => button.getAttribute('aria-label') || button.textContent?.trim() || '';
    const initial = await open(0);
    const descriptors = [...initial.panel.querySelectorAll('button:not(:disabled)')].map(actionName).filter(Boolean);
    const targetCount = targets().length;
    initial.panel.querySelector('button[aria-label="入力パネルを閉じる"]')?.click();
    await sleep(30);
    const failures = [];
    let executed = 0;
    const seededActions = new Set(['カーソルを左へ', 'カーソルを右へ', '一文字戻す', 'クリア', '確定']);
    const isTargetSelected = (target) => {
      if (target.matches('math-field.answer-mathfield')) return target.closest('.answer-box')?.classList.contains('answer-box-selected') === true;
      if (target.matches('button[data-column-digit-index]')) return target.classList.contains('column-digit-slot-selected');
      if (target.matches('button[data-digit-grid-cell]')) return target.classList.contains('digit-grid-cell-selected');
      return false;
    };

    const movementTargetIndex = (descriptor) => {
      if (!['カーソルを左へ', 'カーソルを右へ'].includes(descriptor)) return -1;
      const delta = descriptor === 'カーソルを右へ' ? 1 : -1;
      return targets().findIndex((candidate) => {
        if (!candidate.matches('button[data-column-digit-index]')) return false;
        const current = Number(candidate.getAttribute('data-column-digit-index'));
        const container = candidate.closest('[data-column-answer-slot]');
        const siblings = [...(container?.querySelectorAll('button[data-column-digit-index]:not(:disabled)') ?? [])];
        return siblings.some((sibling) => Number(sibling.getAttribute('data-column-digit-index')) === current + delta);
      });
    };

    const resetReusedTarget = async (targetIndex) => {
      const { target, panel } = await open(targetIndex);
      if (target.matches('math-field.answer-mathfield')) {
        target.setValue('', { silenceNotifications: false });
        target.dispatchEvent(new InputEvent('input', { bubbles: true, composed: true, inputType: 'deleteContentBackward' }));
        await waitFor(() => String(target.value ?? '') === '', 'MathLive harness reset');
        await sleep(120);
      } else {
        panel.querySelector('.keypad-clear')?.click();
        await sleep(120);
      }
      await waitFor(() => document.querySelector('.worksheet-toast')?.getAttribute('aria-label') !== '式が大きすぎます！', 'notice harness reset');
      return open(targetIndex);
    };

    for (const [actionIndex, descriptor] of descriptors.entries()) {
      const movableTargetIndex = movementTargetIndex(descriptor);
      const targetIndex = movableTargetIndex >= 0 ? movableTargetIndex : (descriptor === '確定' ? 0 : actionIndex);
      const reusedTarget = targetIndex < actionIndex || targetIndex >= targetCount;
      let current = reusedTarget
        ? await resetReusedTarget(targetIndex)
        : await open(targetIndex);
      let { target, panel } = current;
      if (seededActions.has(descriptor)) {
        const seedButton = [...panel.querySelectorAll('.keypad-numbers button:not(:disabled)')].find((button) => button.textContent?.trim() === '8')
          ?? panel.querySelector('.keypad-numbers button:not(:disabled)');
        if (seedButton) {
          seedButton.click();
          await sleep(120);
          if (target.matches('math-field.answer-mathfield')) {
            await waitFor(() => String(target.value ?? '').includes(seedButton.textContent?.trim() ?? ''), 'MathLive seeded baseline');
          }
          current = await open(targetIndex);
          target = current.target;
          panel = current.panel;
        }
      }
      const buttons = [...panel.querySelectorAll('button:not(:disabled)')];
      const button = buttons.find((candidate) => actionName(candidate) === descriptor);
      if (!button) { failures.push({ descriptor, reason: 'action disappeared before execution' }); continue; }
      if (target.matches('math-field.answer-mathfield')) {
        if (descriptor === 'カーソルを右へ') target.position = 0;
        if (descriptor === 'カーソルを左へ') target.position = Math.max(1, String(target.value ?? '').length);
      }
      const before = target.matches('math-field.answer-mathfield') ? String(target.value ?? '') : target.textContent?.trim() ?? '';
      const beforePosition = target.matches('math-field.answer-mathfield') ? Number(target.position) : null;
      button.click();
      executed += 1;
      await sleep(180);
      const alert = document.querySelector('[role="alert"]')?.getAttribute('aria-label') ?? null;
      const notice = document.querySelector('.worksheet-toast')?.getAttribute('aria-label') ?? null;
      if (alert || notice === '式が大きすぎます！') failures.push({ descriptor, alert, notice });
      const after = target.matches('math-field.answer-mathfield') ? String(target.value ?? '') : target.textContent?.trim() ?? '';
      if (descriptor === '入力パネルを閉じる' && document.querySelector('.input-panel')) failures.push({ descriptor, reason: 'panel stayed open' });
      if (/^[0-9]$/.test(descriptor) && after === before) failures.push({ descriptor, reason: 'digit action did not change selected input', before, after });
      if (descriptor === '小数点' && target.matches('math-field.answer-mathfield') && !after.includes('.')) failures.push({ descriptor, reason: 'decimal point was not inserted', before, after });
      if (['分数', '帯分数', '平方根', '複数解', 'x, y', 'プラスを挿入', 'マイナスを挿入', 'プラスマイナスを挿入'].includes(descriptor)
        && target.matches('math-field.answer-mathfield') && after === before) {
        failures.push({ descriptor, reason: 'structure/operator action did not change MathLive value', before, after });
      }
      if (['カーソルを左へ', 'カーソルを右へ'].includes(descriptor) && target.matches('math-field.answer-mathfield')) {
        const afterPosition = Number(target.position);
        if (after !== before) failures.push({ descriptor, reason: 'cursor action changed the mathematical value', before, after });
        if (descriptor === 'カーソルを左へ' && !(afterPosition < beforePosition)) failures.push({ descriptor, reason: 'cursor-left did not move the caret', beforePosition, afterPosition });
        if (descriptor === 'カーソルを右へ' && !(afterPosition > beforePosition)) failures.push({ descriptor, reason: 'cursor-right did not move the caret', beforePosition, afterPosition });
      }
      if (['カーソルを左へ', 'カーソルを右へ'].includes(descriptor) && !target.matches('math-field.answer-mathfield') && isTargetSelected(target)) {
        failures.push({ descriptor, reason: 'grid cursor action did not move selection' });
      }
      if (descriptor === '一文字戻す' && target.matches('math-field.answer-mathfield') && after === before) failures.push({ descriptor, reason: 'backspace did not change MathLive value', before, after });
      if (descriptor === 'クリア') {
        if (target.matches('math-field.answer-mathfield') && after !== '') failures.push({ descriptor, reason: 'clear did not empty MathLive value', before, after });
        if (!target.matches('math-field.answer-mathfield') && after === before) failures.push({ descriptor, reason: 'clear did not change selected grid input', before, after });
      }
      if (descriptor === '確定' && document.querySelector('.input-panel') && isTargetSelected(target)) {
        failures.push({ descriptor, reason: 'commit did not advance or close the selected input' });
      }
    }
    return { descriptors, targetCount, executed, failures };
  })()`;
}

function browserWorksheetGridAlignment(root) {
  if (!root) return { applicable: false, reason: 'missing root', maxError: 0, sampleCount: 0, worst: null };
  const grid = root.matches?.('.problem-grid-worksheet-grid') ? root : root.querySelector?.('.problem-grid-worksheet-grid');
  if (!grid) return { applicable: false, reason: 'missing worksheet grid', maxError: 0, sampleCount: 0, worst: null };
  const columnCells = [...root.querySelectorAll('.problem-cell-column-arithmetic')];
  const miniSudokuCells = [...root.querySelectorAll('.problem-cell-mini-sudoku')];
  if (columnCells.length === 0 && miniSudokuCells.length === 0) {
    return { applicable: false, reason: 'no worksheet-grid presentation cells', maxError: 0, sampleCount: 0, worst: null };
  }

  const gridRect = grid.getBoundingClientRect();
  const gridStyle = getComputedStyle(grid);
  const gridPaint = getComputedStyle(grid, '::before');
  const paintedSizes = gridPaint.backgroundSize.split(',')[0]?.trim().split(/\s+/) ?? [];
  const cssPitchX = Number.parseFloat(paintedSizes[0] ?? '');
  const cssPitchY = Number.parseFloat(paintedSizes[1] ?? paintedSizes[0] ?? '');
  const cssWidth = Number.parseFloat(gridStyle.width);
  const cssHeight = Number.parseFloat(gridStyle.height);
  const scaleX = Number.isFinite(cssWidth) && cssWidth > 0 ? gridRect.width / cssWidth : 1;
  const scaleY = Number.isFinite(cssHeight) && cssHeight > 0 ? gridRect.height / cssHeight : scaleX;
  const pitchX = cssPitchX * scaleX;
  const pitchY = cssPitchY * scaleY;
  const top = Number.parseFloat(gridPaint.top);
  const left = Number.parseFloat(gridPaint.left);
  if (![pitchX, pitchY, top, left].every(Number.isFinite) || pitchX <= 0 || pitchY <= 0) {
    return { applicable: true, reason: `invalid grid geometry: size=${gridPaint.backgroundSize} top=${gridPaint.top} left=${gridPaint.left}`, maxError: Number.POSITIVE_INFINITY, sampleCount: 0, worst: null };
  }
  const originX = gridRect.left + left * scaleX;
  const originY = gridRect.top + top * scaleY;
  const lineError = (value, origin, pitch) => {
    const units = (value - origin) / pitch;
    return Math.abs(units - Math.round(units)) * pitch;
  };

  let maxError = 0;
  let sampleCount = 0;
  let worst = null;
  const record = (problem, feature, axis, value, origin, pitch) => {
    const error = lineError(value, origin, pitch);
    sampleCount += 1;
    if (error > maxError) {
      maxError = error;
      worst = { problem, feature, axis, error, value };
    }
  };
  const recordRect = (problem, feature, rect, axes = 'xy') => {
    if (axes.includes('x')) {
      record(problem, feature, 'left', rect.left, originX, pitchX);
      record(problem, feature, 'right', rect.right, originX, pitchX);
    }
    if (axes.includes('y')) {
      record(problem, feature, 'top', rect.top, originY, pitchY);
      record(problem, feature, 'bottom', rect.bottom, originY, pitchY);
    }
  };

  for (const cell of columnCells) {
    const problem = Number(cell.dataset.problemIndex ?? cell.dataset.printProblemIndex ?? -1) + 1;
    const lane = cell.querySelector('.column-arithmetic');
    if (lane) recordRect(problem, 'lane', lane.getBoundingClientRect(), 'x');
    const problemNumberCell = cell.querySelector('.problem-number-stack');
    if (problemNumberCell) recordRect(problem, 'problem-number-cell', problemNumberCell.getBoundingClientRect());
    for (const digit of cell.querySelectorAll('.column-arithmetic-digit-cell')) {
      recordRect(problem, 'digit-cell', digit.getBoundingClientRect());
    }
    for (const slot of cell.querySelectorAll('.column-digit-slot')) {
      recordRect(problem, 'answer-slot', slot.getBoundingClientRect());
    }
    for (const blank of cell.querySelectorAll('.worksheet-print-empty-answer')) {
      recordRect(problem, 'print-answer-box', blank.getBoundingClientRect());
    }
    for (const answerBox of cell.querySelectorAll('.column-division-answer-coordinate-remainder .answer-box')) {
      recordRect(problem, 'remainder-box', answerBox.getBoundingClientRect());
    }
    for (const rule of cell.querySelectorAll('.column-arithmetic-rule, .column-arithmetic-final-rule, .column-division-solution-rule')) {
      const rect = rule.getBoundingClientRect();
      record(problem, 'rule', 'y', rect.top, originY, pitchY);
      record(problem, 'rule', 'left', rect.left, originX, pitchX);
      record(problem, 'rule', 'right', rect.right, originX, pitchX);
    }
    for (const bracket of cell.querySelectorAll('.column-division-bracket-mark')) {
      const rect = bracket.getBoundingClientRect();
      record(problem, 'division-bracket', 'top', rect.top, originY, pitchY);
      record(problem, 'division-bracket', 'bottom', rect.bottom, originY, pitchY);
    }
  }

  for (const cell of miniSudokuCells) {
    const problem = Number(cell.dataset.problemIndex ?? cell.dataset.printProblemIndex ?? -1) + 1;
    const problemNumberCell = cell.querySelector('.problem-number-stack');
    if (problemNumberCell) recordRect(problem, 'problem-number-cell', problemNumberCell.getBoundingClientRect());
    const board = cell.querySelector('.mini-sudoku-grid');
    if (board) recordRect(problem, 'mini-sudoku-grid', board.getBoundingClientRect());
    for (const digitCell of cell.querySelectorAll('.mini-sudoku-grid [data-digit-grid-cell]')) {
      recordRect(problem, 'mini-sudoku-cell', digitCell.getBoundingClientRect());
    }
  }

  const roundedWorst = worst ? { ...worst, error: Math.round(worst.error * 1000) / 1000, value: Math.round(worst.value * 1000) / 1000 } : null;
  return {
    applicable: true,
    cellPx: Math.round(Math.max(pitchX, pitchY) * 1000) / 1000,
    maxError: Math.round(maxError * 1000) / 1000,
    sampleCount,
    worst: roundedWorst,
  };
}

function browserColumnLaneSafety(root) {
  if (!root) return [];
  const page = root.matches?.('.paper, .worksheet-print-page') ? root : root.closest?.('.paper, .worksheet-print-page') ?? root.querySelector?.('.paper, .worksheet-print-page') ?? root;
  const pageRect = page.getBoundingClientRect();
  const items = [...root.querySelectorAll('.problem-cell-column-arithmetic')].flatMap((cell) => {
    const rects = [...cell.querySelectorAll('.problem-number-stack, .column-arithmetic, .column-answer-user, .column-answer-correction, .column-division-answer-coordinate, .column-division-correction')]
      .map((element) => element.getBoundingClientRect())
      .filter((rect) => rect.width > 0 && rect.height > 0);
    if (rects.length === 0) return [];
    const cellRect = cell.getBoundingClientRect();
    return [{
      problem: Number(cell.dataset.problemIndex ?? cell.dataset.printProblemIndex ?? -1) + 1,
      column: Number(cell.dataset.layoutColumn ?? -1),
      rowTop: cellRect.top,
      cellLeft: cellRect.left,
      left: Math.min(...rects.map((rect) => rect.left)),
      right: Math.max(...rects.map((rect) => rect.right)),
    }];
  });
  const issues = [];
  for (const item of items) {
    const overflow = Math.max(pageRect.left - item.left, item.right - pageRect.right, 0);
    if (overflow > 1) {
      issues.push({ ...item, kind: 'page-overflow', overflow: Math.round(overflow * 10) / 10 });
    }
  }
  const rows = [];
  for (const item of items.sort((a, b) => a.rowTop - b.rowTop || a.cellLeft - b.cellLeft)) {
    let row = rows.find((candidate) => Math.abs(candidate.top - item.rowTop) <= 1);
    if (!row) { row = { top: item.rowTop, items: [] }; rows.push(row); }
    row.items.push(item);
  }
  for (const row of rows) {
    row.items.sort((a, b) => a.left - b.left);
    for (let index = 1; index < row.items.length; index += 1) {
      const previous = row.items[index - 1];
      const current = row.items[index];
      const overlap = previous.right - current.left;
      if (overlap > 1) {
        issues.push({
          problem: current.problem,
          column: current.column,
          kind: 'lane-overlap',
          overflow: Math.round(overlap * 10) / 10,
          previousProblem: previous.problem,
        });
      }
    }
  }
  return issues;
}

function worksheetGridPrintAlignmentProbe() {
  return `(async () => {
    const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
    const waitFor = async (fn, label) => {
      for (let i = 0; i < 400; i += 1) { const value = fn(); if (value) return value; await sleep(25); }
      throw new Error('Timed out waiting for ' + label);
    };
    const measureWorksheetGridAlignment = ${browserWorksheetGridAlignment.toString()};
    const print = document.querySelector('button[aria-label="印刷"]');
    if (!print) throw new Error('Missing print button for worksheet-grid alignment probe');
    print.click();
    const preview = await waitFor(() => document.querySelector('.worksheet-print-preview'), 'print preview');
    await document.fonts.ready;
    await new Promise(requestAnimationFrame);
    await new Promise(requestAnimationFrame);
    const rotateAnswer = preview.querySelector('.worksheet-print-preview-rotate input[type="checkbox"]');
    if (rotateAnswer?.checked) {
      rotateAnswer.click();
      await new Promise(requestAnimationFrame);
      await new Promise(requestAnimationFrame);
    }
    const problemPage = preview.querySelector('[data-print-page="problems"]');
    const answerPage = preview.querySelector('[data-print-page="answers"]');
    const problems = measureWorksheetGridAlignment(problemPage);
    const answers = measureWorksheetGridAlignment(answerPage);
    const back = preview.querySelector('.worksheet-print-preview-back')
      ?? [...preview.querySelectorAll('button')].find((button) => button.textContent?.trim() === '戻る');
    back?.click();
    await waitFor(() => document.querySelector('.worksheet-screen .paper'), 'worksheet after print preview');
    return { problems, answers };
  })()`;
}

function worksheetProbe(seed, difficultyLabel = 'むずかしい') {
  return `(async () => {
    const seed = ${JSON.stringify(seed)};
    const difficultyLabel = ${JSON.stringify(difficultyLabel)};
    const measureWorksheetGridAlignment = ${browserWorksheetGridAlignment.toString()};
    const measureColumnLaneSafety = ${browserColumnLaneSafety.toString()};
    const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
    const waitFor = async (fn, label) => {
      for (let i = 0; i < 400; i += 1) { const value = fn(); if (value) return value; await sleep(25); }
      throw new Error('Timed out waiting for ' + label);
    };
    await waitFor(() => window.__AUTODRILL_WASM__, 'WASM');
    const difficulty = await waitFor(() => document.querySelector('button[aria-label=\"難易度\"]'), 'difficulty');
    difficulty.click();
    const difficultyOption = await waitFor(() => [...document.querySelectorAll('[role=\"option\"]')].find((option) => option.getAttribute('aria-label') === difficultyLabel), difficultyLabel + ' difficulty');
    difficultyOption.click();
    await new Promise(requestAnimationFrame);
    await new Promise(requestAnimationFrame);
    const advanced = await waitFor(() => document.querySelector('.advanced-settings'), 'advanced settings');
    if (!advanced.open) {
      advanced.querySelector('summary')?.click();
      await waitFor(() => advanced.open, 'advanced settings open');
      await new Promise(requestAnimationFrame);
      await new Promise(requestAnimationFrame);
    }
    const seedInput = document.querySelector('input[aria-label="Seed"]');
    const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set;
    setter.call(seedInput, seed);
    seedInput.dispatchEvent(new Event('input', { bubbles: true }));
    seedInput.dispatchEvent(new Event('change', { bubbles: true }));
    await new Promise(requestAnimationFrame);
    await new Promise(requestAnimationFrame);
    document.querySelector('button[aria-label="問題生成"]').click();
    const outcome = await waitFor(() => document.querySelector('.worksheet-screen .paper') || document.querySelector('[role="alert"]'), 'worksheet or generation error');
    if (outcome.getAttribute('role') === 'alert') throw new Error('Generation failed: ' + (outcome.getAttribute('aria-label') || outcome.textContent || 'unknown error'));
    const paper = outcome;
    const footerText = document.querySelector('[data-testid="worksheet-footer"]')?.textContent ?? '';
    if (!footerText.includes(seed)) throw new Error('Generated worksheet does not use requested Seed ' + seed + ': ' + footerText);
    await document.fonts.ready;
    await new Promise(requestAnimationFrame);
    await new Promise(requestAnimationFrame);
    const cells = [...paper.querySelectorAll('.problem-cell')];
    const crossings = [];
    const columnGridMismatches = [];
    for (const cell of cells) {
      const column = Number(cell.dataset.layoutColumn);
      const isColumnArithmetic = cell.classList.contains('problem-cell-column-arithmetic');
      // Ordinary worksheet content is owned by its logical A4 cell, so crossing
      // that cell is a clipping/layout failure. Column arithmetic is different:
      // its canonical owner is the page-wide worksheet grid, and a correctly
      // snapped lane may cross an invisible logical cell edge. For column lanes
      // we check actual page overflow and sibling-lane overlap below instead.
      if (!isColumnArithmetic) {
        const boundarySelector = '.problem-number, math-span.problem-math-expression, .liar-statements, .mini-sudoku-grid, .problem-answer-area';
        const rects = [...cell.querySelectorAll(boundarySelector)]
          .map((element) => element.getBoundingClientRect())
          .filter((rect) => rect.width > 0 && rect.height > 0);
        if (rects.length > 0) {
          const cellRect = cell.getBoundingClientRect();
          const minLeft = Math.min(...rects.map((rect) => rect.left));
          const maxRight = Math.max(...rects.map((rect) => rect.right));
          const overflow = Math.max(cellRect.left - minLeft, maxRight - cellRect.right);
          if (overflow > 1) {
            crossings.push({
              problem: Number(cell.dataset.problemIndex) + 1,
              column,
              overflow: Math.round(overflow * 10) / 10,
              expression: cell.querySelector('math-span')?.getAttribute('aria-label') ?? cell.querySelector('.expression')?.textContent ?? '',
            });
          }
        }
      }
      if (isColumnArithmetic) {
        const divide = cell.classList.contains('problem-cell-column-arithmetic-divide');
        const reference = divide
          ? cell.querySelector('.column-division-bracket')?.getBoundingClientRect()
          : (cell.querySelector('.column-arithmetic-final-rule') ?? cell.querySelector('.column-arithmetic-rule'))?.getBoundingClientRect();
        const answer = divide
          ? cell.querySelector('.column-division-answer-coordinate-quotient .column-digit-answer')?.getBoundingClientRect()
          : cell.querySelector('.column-answer-user .column-digit-answer')?.getBoundingClientRect();
        if (reference && answer) {
          const leftDelta = Math.abs(reference.right - answer.right);
          const laneLeft = cell.querySelector('.column-arithmetic')?.getBoundingClientRect().left ?? reference.left;
          const answerLaneLeft = answer.right - answer.width;
          let operatorDelta = 0;
          let problemNumberDelta = 0;
          const problemNumberCell = cell.querySelector('.problem-number-stack')?.getBoundingClientRect();
          const expressionLane = cell.querySelector('.column-arithmetic')?.getBoundingClientRect();
          const digitCell = cell.querySelector('.column-arithmetic-digit-cell')?.getBoundingClientRect();
          if (!divide) {
            const row = cell.querySelector('.column-arithmetic-row-bottom')?.getBoundingClientRect();
            const operator = cell.querySelector('.column-arithmetic-row-bottom .column-arithmetic-operator')?.getBoundingClientRect();
            const topDigits = cell.querySelectorAll('.column-arithmetic-row-top .column-arithmetic-digit-cell').length;
            const bottomDigits = cell.querySelectorAll('.column-arithmetic-row-bottom .column-arithmetic-digit-cell').length;
            const operandDigits = Math.max(topDigits, bottomDigits);
            if (row && operator && problemNumberCell && expressionLane && digitCell && operandDigits > 0) {
              const expectedOperatorRight = row.right - operandDigits * digitCell.width;
              const expectedOperatorLeft = expectedOperatorRight - operator.width;
              operatorDelta = Math.max(
                Math.abs(operator.left - expectedOperatorLeft),
                Math.abs(operator.right - expectedOperatorRight),
              );
              problemNumberDelta = Math.max(
                Math.abs(problemNumberCell.left - expectedOperatorLeft),
                Math.abs(problemNumberCell.right - expectedOperatorRight),
                Math.abs(problemNumberCell.bottom - expressionLane.top),
                Math.abs(problemNumberCell.width - digitCell.width),
                Math.abs(problemNumberCell.height - digitCell.height),
              );
            }
          } else if (problemNumberCell && expressionLane && digitCell) {
            problemNumberDelta = Math.max(
              Math.abs(problemNumberCell.left - expressionLane.left),
              Math.abs(problemNumberCell.right - (expressionLane.left + digitCell.width)),
              Math.abs(problemNumberCell.bottom - expressionLane.top),
              Math.abs(problemNumberCell.width - digitCell.width),
              Math.abs(problemNumberCell.height - digitCell.height),
            );
          }
          const horizontalDelta = Math.max(
            leftDelta,
            Math.max(0, laneLeft - answerLaneLeft),
            operatorDelta,
            problemNumberDelta,
          );
          let verticalDelta = 0;
          let expectedTop = null;
          if (!divide) {
            expectedTop = reference.bottom;
            verticalDelta = Math.abs(answer.top - expectedTop);
          }
          if (horizontalDelta > 1 || verticalDelta > 1) {
            columnGridMismatches.push({
              problem: Number(cell.dataset.problemIndex) + 1,
              horizontalDelta: Math.round(horizontalDelta * 10) / 10,
              operatorDelta: Math.round(operatorDelta * 10) / 10,
              problemNumberDelta: Math.round(problemNumberDelta * 10) / 10,
              verticalDelta: Math.round(verticalDelta * 10) / 10,
              expectedTop: expectedTop === null ? null : Math.round(expectedTop * 10) / 10,
              actualTop: Math.round(answer.top * 10) / 10,
            });
          }
        }
      }
    }
    for (const issue of measureColumnLaneSafety(paper)) {
      crossings.push({ ...issue, expression: 'column lane ' + issue.kind });
    }
    const gradeClass = [...paper.classList].find((name) => name.startsWith('worksheet-grade-')) ?? null;
    const expression = paper.querySelector('.expression');
    const fontSize = expression ? getComputedStyle(expression).fontSize : null;
    const worksheetGridAlignment = measureWorksheetGridAlignment(paper);
    return { crossings, columnGridMismatches, worksheetGridAlignment, count: cells.length, gradeClass, fontSize, alert: document.querySelector('[role="alert"]')?.getAttribute('aria-label') ?? null };
  })()`;
}


function gradingSettingsProbe() {
  return `(async () => {
    const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
    const waitFor = async (fn, label) => {
      for (let i = 0; i < 200; i += 1) { const value = fn(); if (value) return value; await sleep(25); }
      throw new Error('Timed out waiting for ' + label);
    };
    const advanced = await waitFor(() => document.querySelector('.advanced-settings > summary'), 'advanced settings');
    advanced.click();
    const advancedPanel = advanced.closest('.advanced-settings');
    const advancedRubyCount = advancedPanel?.querySelectorAll('ruby, rt').length ?? -1;
    const grading = await waitFor(() => document.querySelector('.grading-settings-open-button'), 'grading settings button');
    grading.click();
    const dialog = await waitFor(() => document.querySelector('[role=\"dialog\"][aria-labelledby=\"grading-settings-title\"]'), 'grading settings dialog');
    const groups = [...dialog.querySelectorAll('.grading-setting-toggle')].map((group) => ({
      label: group.getAttribute('aria-label'),
      pressed: [...group.querySelectorAll('button')].filter((button) => button.getAttribute('aria-pressed') === 'true').map((button) => button.textContent?.trim()),
    }));
    await waitFor(() => dialog.querySelectorAll('.grading-setting-example math-span').length === 6, 'grading math examples');
    const descriptions = [...dialog.querySelectorAll('.grading-setting-copy > span')].map((element) => ({
      label: element.getAttribute('aria-label') ?? element.textContent?.trim() ?? '',
      math: [...element.querySelectorAll('math-span')].map((math) => ({
        ariaLabel: math.getAttribute('aria-label'),
        mode: math.getAttribute('mode'),
      })),
    }));
    const dialogRubyCount = dialog.querySelectorAll('ruby, rt').length;
    const fractionCorrect = [...dialog.querySelectorAll('.grading-setting-toggle button')].find((button) => button.getAttribute('aria-label') === '約分しましょうを丸にする');
    fractionCorrect?.click();
    await new Promise(requestAnimationFrame);
    const fractionCorrectPressed = fractionCorrect?.getAttribute('aria-pressed') === 'true';
    dialog.querySelector('button[aria-label=\"採点設定を閉じる\"]')?.click();
    await new Promise(requestAnimationFrame);
    return { groups, descriptions, advancedRubyCount, dialogRubyCount, fractionCorrectPressed, dialogClosed: !document.querySelector('[role=\"dialog\"]') };
  })()`;
}

function recommendedGradeTagProbe() {
  return `(async () => {
    const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
    const waitFor = async (fn, label) => {
      for (let i = 0; i < 200; i += 1) { const value = fn(); if (value) return value; await sleep(25); }
      throw new Error('Timed out waiting for ' + label);
    };
    const genre = await waitFor(() => document.querySelector('#genre-select'), 'genre select');
    genre.click();
    const equationOption = await waitFor(() => [...document.querySelectorAll('[role="option"]')].find((option) => option.getAttribute('aria-label') === '方程式'), 'equation genre');
    equationOption.click();
    await new Promise(requestAnimationFrame);
    const theme = await waitFor(() => document.querySelector('#theme-select'), 'theme select');
    theme.click();
    const rows = await waitFor(() => {
      const options = [...document.querySelectorAll('[role="option"]')];
      return options.some((option) => option.getAttribute('aria-label') === '連立方程式(1)') ? options : null;
    }, 'equation theme options');
    const labels = ['一次方程式(1)', '連立方程式(1)', '二次方程式(1)'];
    const result = labels.map((label) => {
      const row = rows.find((option) => option.getAttribute('aria-label') === label);
      const tag = row?.querySelector('.grade-tag');
      const check = row?.querySelector('.custom-select-check, .custom-select-check-placeholder');
      const tagRect = tag?.getBoundingClientRect();
      const checkRect = check?.getBoundingClientRect();
      return {
        label,
        tag: tag?.textContent?.trim() ?? null,
        className: tag?.className ?? null,
        tagRight: tagRect?.right ?? null,
        checkLeft: checkRect?.left ?? null,
        sameRow: tagRect && checkRect ? Math.abs((tagRect.top + tagRect.bottom) / 2 - (checkRect.top + checkRect.bottom) / 2) < 1 : false,
      };
    });
    theme.click();
    return result;
  })()`;
}

function simultaneousInputProbe() {
  return `(async () => {
    const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
    const waitFor = async (fn, label) => {
      for (let i = 0; i < 300; i += 1) { const value = fn(); if (value) return value; await sleep(25); }
      throw new Error('Timed out waiting for ' + label);
    };
    const fields = await waitFor(() => {
      const values = [...document.querySelectorAll('math-field.answer-mathfield')];
      return values.length === 24 ? values : null;
    }, '24 simultaneous coordinate answer fields');
    const xField = fields[0];
    const yField = fields[1];
    const coordinates = [xField, yField].map((field) => ({
      aria: field.getAttribute('aria-label'),
      prefix: field.closest('.simultaneous-answer-coordinate')?.querySelector('.answer-prefix-label')?.getAttribute('aria-label') ?? null,
      prefixLatex: field.closest('.simultaneous-answer-coordinate')?.querySelector('.answer-prefix-label')?.textContent ?? null,
    }));
    xField.click();
    const panel = await waitFor(() => document.querySelector('.input-panel'), 'simultaneous input panel');
    const labels = [...panel.querySelectorAll('.formula-keypad button')].map((button) => button.getAttribute('aria-label'));
    const digit2 = [...panel.querySelectorAll('.keypad-numbers button')].find((button) => button.textContent?.trim() === '2');
    digit2?.click();
    await sleep(80);
    yField.click();
    const digit3 = [...panel.querySelectorAll('.keypad-numbers button')].find((button) => button.textContent?.trim() === '3');
    digit3?.click();
    await sleep(80);
    return {
      fieldCount: fields.length,
      coordinates,
      labels,
      xValue: xField.value,
      yValue: yField.value,
      notice: document.querySelector('.worksheet-toast')?.textContent?.trim() ?? null,
    };
  })()`;
}


function miniSudokuInputProbe() {
  return `(async () => {
    const measureWorksheetGridAlignment = ${browserWorksheetGridAlignment.toString()};
    const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
    const waitFor = async (fn, label) => {
      for (let i = 0; i < 400; i += 1) { const value = fn(); if (value) return value; await sleep(25); }
      throw new Error('Timed out waiting for ' + label);
    };
    const grids = await waitFor(() => {
      const values = [...document.querySelectorAll('.worksheet-screen .mini-sudoku-grid')];
      return values.length === 4 ? values : null;
    }, 'four mini sudoku grids');
    const counts = grids.map((grid) => ({
      cells: grid.querySelectorAll('[data-digit-grid-cell]').length,
      editable: grid.querySelectorAll('button[data-digit-grid-cell]').length,
      givens: grid.querySelectorAll('.digit-grid-cell-given').length,
    }));
    const first = grids[0];
    const firstButton = first.querySelector('button[data-digit-grid-cell]');
    const selectedIndex = Number(firstButton?.getAttribute('data-digit-grid-cell'));
    firstButton?.click();
    const panel = await waitFor(() => document.querySelector('.input-panel'), 'mini sudoku input panel');
    const numberKeys = [...panel.querySelectorAll('.keypad-numbers button')].map((button) => button.textContent?.trim()).filter(Boolean);
    const digit = numberKeys[0];
    [...panel.querySelectorAll('.keypad-numbers button')].find((button) => button.textContent?.trim() === digit)?.click();
    await sleep(80);
    const written = first.querySelector('[data-digit-grid-cell="' + selectedIndex + '"] .digit-grid-cell-value')?.textContent?.trim() ?? '';
    const cellRects = [...first.querySelectorAll('[data-digit-grid-cell]')].map((cell) => cell.getBoundingClientRect());
    const maxSquareDelta = Math.max(...cellRects.map((rect) => Math.abs(rect.width - rect.height)));
    const gridRect = first.getBoundingClientRect();
    const cellSize = cellRects[0]?.width ?? 0;
    const overlayStyle = getComputedStyle(first, '::after');
    const overlay = {
      content: overlayStyle.content,
      pointerEvents: overlayStyle.pointerEvents,
      boxShadow: overlayStyle.boxShadow,
      backgroundImage: overlayStyle.backgroundImage,
    };
    document.querySelector('button[aria-label="採点"]')?.click();
    await waitFor(() => document.querySelector('.grade-result-panel'), 'mini sudoku grading');
    await new Promise(requestAnimationFrame);
    await new Promise(requestAnimationFrame);
    const gradedGridAlignment = measureWorksheetGridAlignment(document.querySelector('.worksheet-screen .paper'));
    return {
      gridCount: grids.length,
      counts,
      numberKeys,
      selectedIndex,
      written,
      maxSquareDelta,
      gridWidthDelta: Math.abs(gridRect.width - 4 * cellSize),
      overlay,
      panelDigitGrid: panel.classList.contains('input-panel-digit-grid'),
      gradedGridAlignment,
    };
  })()`;
}

function columnAdditionInputProbe() {
  return `(async () => {
    const measureWorksheetGridAlignment = ${browserWorksheetGridAlignment.toString()};
    const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
    const waitFor = async (fn, label) => {
      for (let i = 0; i < 400; i += 1) { const value = fn(); if (value) return value; await sleep(25); }
      throw new Error('Timed out waiting for ' + label);
    };
    const editors = await waitFor(() => {
      const values = [...document.querySelectorAll('.column-digit-answer-single')];
      return values.length === 16 ? values : null;
    }, '16 column addition digit editors');
    const editor = editors[0];
    const initialButtons = [...editor.querySelectorAll('button.column-digit-slot-active')];
    initialButtons.at(-1)?.click();
    await waitFor(() => document.querySelector('.input-panel'), 'column addition input panel');
    // Exercise the physical-keyboard path that previously left a stale focus ring.
    for (const digit of ['4', '6', '1']) {
      window.dispatchEvent(new KeyboardEvent('keydown', { key: digit, bubbles: true }));
      await sleep(60);
    }
    const cell = editor.closest('.problem-cell-column-arithmetic');
    const rule = cell.querySelector('.column-arithmetic-rule').getBoundingClientRect();
    const editorRect = editor.getBoundingClientRect();
    const slots = [...editor.querySelectorAll('.column-digit-slot-active')];
    const slotRects = slots.map((slot) => slot.getBoundingClientRect());
    const value = slots.map((slot) => slot.textContent?.trim() ?? '').join('').replace(/^0+(?=\\d)/, '');
    const selectedElement = editor.querySelector('.column-digit-slot-selected');
    const selected = slots.findIndex((slot) => slot === selectedElement);
    const domFocusFollowsSelection = document.activeElement === selectedElement;
    const adjacentGaps = slotRects.slice(1).map((rect, index) => Math.abs(rect.left - slotRects[index].right));
    const sizeDeltas = slotRects.map((rect) => Math.abs(rect.width - rect.height));
    const slotFontPx = parseFloat(getComputedStyle(slots[0]).fontSize || '0');
    const slotHeight = slotRects[0]?.height ?? 0;
    const slotOverflow = getComputedStyle(slots[0]).overflow;
    const expression = cell.querySelector('.column-arithmetic');
    const firstSlot = slots[0];
    const expressionFont = getComputedStyle(expression).fontFamily;
    const slotFont = getComputedStyle(firstSlot).fontFamily;
    const selectedGridAlignment = measureWorksheetGridAlignment(document.querySelector('.worksheet-screen .paper'));

    document.querySelector('button[aria-label="採点"]')?.click();
    const feedback = await waitFor(() => {
      const values = [...document.querySelectorAll('.problem-grade-mark')];
      return values.length === 16 ? values : null;
    }, 'column grading feedback');
    const gradedEditor = document.querySelector('.problem-cell-column-arithmetic .column-answer-user .column-digit-answer-single');
    const gradedValue = [...gradedEditor.querySelectorAll('.column-digit-slot-active')].map((slot) => slot.textContent?.trim() ?? '').join('').replace(/^0+(?=\\d)/, '');
    const correctionEditor = document.querySelector('.problem-cell-column-arithmetic .column-answer-correction .column-digit-answer-correction');
    const correctionValue = correctionEditor
      ? [...correctionEditor.querySelectorAll('.column-digit-slot-active')].map((slot) => slot.textContent?.trim() ?? '').join('').replace(/^0+(?=\\d)/, '')
      : null;
    const correctionInGrid = Boolean(correctionEditor);
    const separateCorrectAnswerCount = document.querySelectorAll('.column-grade-correct-answer').length;
    const correctionGlyph = correctionEditor?.querySelector('.column-digit-glyph');
    const correctionColor = correctionGlyph ? getComputedStyle(correctionGlyph).color : null;
    const firstProblemNumber = document.querySelector('.problem-cell-column-arithmetic .problem-number');
    const firstMarkRect = feedback[0]?.getBoundingClientRect();
    const firstNumberRect = firstProblemNumber?.getBoundingClientRect();
    const firstMarkAnchoredAboveProblemNumber = Boolean(
      firstMarkRect
      && firstNumberRect
      && firstMarkRect.top < firstNumberRect.top
      && Math.abs((firstMarkRect.left + firstMarkRect.right) / 2 - (firstNumberRect.left + firstNumberRect.right) / 2) <= 1
    );
    const firstMarkColor = feedback[0] ? getComputedStyle(feedback[0]).color : null;
    const allMarksRed = feedback.every((mark) => getComputedStyle(mark).color === 'rgb(210, 11, 11)');
    const minMarkFontPx = Math.min(...feedback.map((mark) => parseFloat(getComputedStyle(mark).fontSize || '0')));
    const problemNumberFontPx = firstProblemNumber ? parseFloat(getComputedStyle(firstProblemNumber).fontSize || '0') : 0;
    const markToNumberFontRatio = problemNumberFontPx > 0 ? minMarkFontPx / problemNumberFontPx : null;
    const gradedGridAlignment = measureWorksheetGridAlignment(document.querySelector('.worksheet-screen .paper'));
    return {
      fieldCount: editors.length,
      value,
      gradedValue,
      direction: editor.getAttribute('data-column-direction'),
      selected,
      domFocusFollowsSelection,
      topDelta: Math.round((editorRect.top - rule.bottom) * 10) / 10,
      rightDelta: Math.round((editorRect.right - rule.right) * 10) / 10,
      maxAdjacentGap: Math.max(0, ...adjacentGaps),
      maxCellSizeDelta: Math.max(0, ...sizeDeltas),
      slotFontScale: slotHeight > 0 ? slotFontPx / slotHeight : null,
      slotOverflow,
      expressionFont,
      slotFont,
      feedbackCount: feedback.length,
      firstMark: feedback[0]?.textContent?.trim() ?? null,
      firstMarkAnchoredAboveProblemNumber,
      firstMarkColor,
      allMarksRed,
      markToNumberFontRatio,
      correctionInGrid,
      correctionValue,
      separateCorrectAnswerCount,
      correctionColor,
      selectedGridAlignment,
      gradedGridAlignment,
    };
  })()`;
}

function columnDecimalInputProbe() {
  return `(async () => {
    const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
    const waitFor = async (fn, label) => {
      for (let i = 0; i < 400; i += 1) { const value = fn(); if (value) return value; await sleep(25); }
      throw new Error('Timed out waiting for ' + label);
    };
    const editors = await waitFor(() => {
      const values = [...document.querySelectorAll('.column-digit-answer-single')];
      return values.length === 16 ? values : null;
    }, '16 decimal column digit editors');
    const editor = editors.find((candidate) => candidate.querySelector('.column-digit-decimal-marker'));
    if (!editor) return { fieldCount: editors.length, markerFound: false };
    const marker = editor.querySelector('.column-digit-decimal-marker');
    const activeSlots = [...editor.querySelectorAll('button.column-digit-slot-active')];
    activeSlots.at(-1)?.click();
    const panel = await waitFor(() => document.querySelector('.input-panel'), 'decimal column input panel');
    const decimalKeyCount = panel.querySelectorAll('.keypad-decimal').length;
    [...panel.querySelectorAll('.keypad-numbers button')].find((button) => button.textContent?.trim() === '5')?.click();
    await sleep(80);
    const markerRect = marker.getBoundingClientRect();
    const slotRects = [...editor.querySelectorAll('.column-digit-slot')].map((slot) => slot.getBoundingClientRect());
    const boundaries = slotRects.flatMap((rect) => [rect.left, rect.right]);
    const boundaryDelta = Math.min(...boundaries.map((value) => Math.abs(value - (markerRect.left + markerRect.width / 2))));
    const selected = editor.querySelector('.column-digit-slot-selected');
    const rightmostValue = editor.querySelector('.column-digit-slot-active:last-of-type')?.textContent?.trim() ?? '';
    return {
      fieldCount: editors.length,
      markerFound: true,
      decimalKeyCount,
      boundaryDelta,
      rightmostValue,
      direction: editor.getAttribute('data-column-direction'),
      selectedLabel: selected?.getAttribute('aria-label') ?? null,
    };
  })()`;
}

function columnDivisionKeyboardTargetProbe() {
  return `(async () => {
    const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
    const waitFor = async (fn, label) => {
      for (let i = 0; i < 400; i += 1) { const value = fn(); if (value) return value; await sleep(25); }
      throw new Error('Timed out waiting for ' + label);
    };
    const remainders = await waitFor(() => {
      const values = [...document.querySelectorAll('.problem-cell-column-arithmetic-divide .column-division-answer-coordinate-remainder math-field.answer-mathfield')];
      return values.length === 12 ? values : null;
    }, '12 remainder fields for physical keyboard probe');
    const field = remainders[7];
    if (!field) throw new Error('Missing remainder field for physical keyboard probe');
    const rect = field.getBoundingClientRect();
    return {
      x: (rect.left + rect.right) / 2,
      y: (rect.top + rect.bottom) / 2,
      ariaLabel: field.getAttribute('aria-label'),
    };
  })()`;
}

function columnDivisionKeyboardResultProbe() {
  return `(() => {
    const fields = [...document.querySelectorAll('.problem-cell-column-arithmetic-divide .column-division-answer-coordinate-remainder math-field.answer-mathfield')];
    const field = fields[7];
    return {
      value: field?.value ?? null,
      selected: field?.classList.contains('answer-mathfield-selected') || field?.closest('.answer-box')?.classList.contains('answer-box-selected') === true,
      notice: document.querySelector('.worksheet-toast')?.getAttribute('aria-label') ?? null,
    };
  })()`;
}

function columnDivisionKeyboardFocusProbe() {
  return `(async () => {
    const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
    for (let i = 0; i < 80; i += 1) {
      const fields = [...document.querySelectorAll('.problem-cell-column-arithmetic-divide .column-division-answer-coordinate-remainder math-field.answer-mathfield')];
      const field = fields[7];
      const selected = field?.classList.contains('answer-mathfield-selected') || field?.closest('.answer-box')?.classList.contains('answer-box-selected') === true;
      if (field && selected && document.activeElement === field) return true;
      await sleep(25);
    }
    return false;
  })()`;
}

function columnDivisionInputProbe() {
  return `(async () => {
    const measureWorksheetGridAlignment = ${browserWorksheetGridAlignment.toString()};
    const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
    const waitFor = async (fn, label) => {
      for (let i = 0; i < 400; i += 1) { const value = fn(); if (value) return value; await sleep(25); }
      throw new Error('Timed out waiting for ' + label);
    };
    const quotients = await waitFor(() => {
      const values = [...document.querySelectorAll('.problem-cell-column-arithmetic-divide .column-digit-answer-quotient')];
      return values.length === 12 ? values : null;
    }, '12 quotient digit editors');
    const remainders = await waitFor(() => {
      const values = [...document.querySelectorAll('.problem-cell-column-arithmetic-divide .column-division-answer-coordinate-remainder math-field.answer-mathfield')];
      return values.length === 12 ? values : null;
    }, '12 ordinary remainder fields');
    const quotient = quotients[3];
    const remainder = remainders[3];
    const firstCell = remainder.closest('.problem-cell-column-arithmetic-divide');
    const quotientLabel = quotient.closest('.column-division-answer-coordinate')?.querySelector('.column-division-answer-label')?.textContent?.trim() ?? null;
    const remainderLabel = remainder.closest('.column-division-answer-coordinate')?.querySelector('.column-division-answer-label')?.textContent?.trim() ?? null;
    const quotientButtons = [...quotient.querySelectorAll('button.column-digit-slot-active')];
    quotientButtons[0]?.click();
    const panel = await waitFor(() => document.querySelector('.input-panel'), 'column division input panel');
    const formulaLabels = [...panel.querySelectorAll('.formula-keypad button')].map((button) => button.getAttribute('aria-label'));
    // Fill every quotient place. The final digit must automatically hand focus to remainder.
    for (let i = 0; i < quotientButtons.length; i += 1) {
      [...panel.querySelectorAll('.keypad-numbers button')].find((button) => button.textContent?.trim() === '1')?.click();
      await sleep(60);
    }
    const autoMovedToRemainder = remainder.classList.contains('answer-mathfield-selected')
      || remainder.closest('.answer-box')?.classList.contains('answer-box-selected') === true;
    // Remainder is an ordinary big-endian field: 2 then 1 must be 21, not 12.
    for (const digit of ['2', '1']) {
      [...panel.querySelectorAll('.keypad-numbers button')].find((button) => button.textContent?.trim() === digit)?.click();
      await sleep(60);
    }
    const quotientValue = [...quotient.querySelectorAll('.column-digit-slot-active')].map((slot) => slot.textContent?.trim() ?? '').join('').replace(/^0+(?=[0-9])/, '');
    const remainderValue = remainder.value;
    const noticeAfterAutoRemainder = document.querySelector('.worksheet-toast')?.getAttribute('aria-label') ?? null;

    // Exercise a separate remainder field through direct click as well as the
    // automatic quotient-to-remainder transition. Logical problem-cell crossing
    // is presentation-dependent and is not an input semantic invariant.
    const directRemainder = remainders.at(-1);
    directRemainder?.click();
    await sleep(80);
    const directPanel = await waitFor(() => document.querySelector('.input-panel'), 'direct remainder input panel');
    for (const digit of ['2', '1']) {
      [...directPanel.querySelectorAll('.keypad-numbers button')].find((button) => button.textContent?.trim() === digit)?.click();
      await sleep(80);
    }
    const directRemainderValue = directRemainder?.value ?? null;
    const noticeAfterDirectRemainder = document.querySelector('.worksheet-toast')?.getAttribute('aria-label') ?? null;

    const bracket = firstCell.querySelector('.column-division-bracket');
    const bracketPath = bracket?.querySelector('.column-division-bracket-mark path')?.getAttribute('d') ?? null;
    const selectedGridAlignment = measureWorksheetGridAlignment(document.querySelector('.worksheet-screen .paper'));
    document.querySelector('button[aria-label="採点"]')?.click();
    await waitFor(() => document.querySelectorAll('.problem-cell-column-arithmetic-divide .problem-grade-mark').length === 12, 'division grading feedback');
    const gradedFirstCell = [...document.querySelectorAll('.problem-cell-column-arithmetic-divide')][3];
    const gradedQuotient = gradedFirstCell.querySelector('.column-division-answer-coordinate-quotient .column-digit-answer-quotient');
    const gradedRemainder = gradedFirstCell.querySelector('.column-division-answer-coordinate-remainder math-field.answer-mathfield');
    const gradedDirectRemainder = [...document.querySelectorAll('.problem-cell-column-arithmetic-divide .column-division-answer-coordinate-remainder math-field.answer-mathfield')].at(-1);
    const quotientCorrection = gradedFirstCell.querySelector('.column-division-correction-quotient .column-digit-answer-correction');
    const remainderCorrection = gradedFirstCell.querySelector('.column-division-correction-remainder .column-remainder-correction-value');
    const gradedQuotientValue = gradedQuotient
      ? [...gradedQuotient.querySelectorAll('.column-digit-slot-active')].map((slot) => slot.textContent?.trim() ?? '').join('').replace(/^0+(?=[0-9])/, '')
      : null;
    const quotientCorrectionValue = quotientCorrection
      ? [...quotientCorrection.querySelectorAll('.column-digit-slot-active')].map((slot) => slot.textContent?.trim() ?? '').join('').replace(/^0+(?=[0-9])/, '')
      : null;
    const quotientCorrectionGlyph = quotientCorrection?.querySelector('.column-digit-glyph');
    const quotientCorrectionColor = quotientCorrectionGlyph ? getComputedStyle(quotientCorrectionGlyph).color : null;
    const remainderCorrectionValue = remainderCorrection?.textContent?.trim() ?? null;
    const remainderCorrectionColor = remainderCorrection ? getComputedStyle(remainderCorrection).color : null;
    const gradedGridAlignment = measureWorksheetGridAlignment(document.querySelector('.worksheet-screen .paper'));
    const returnButton = document.querySelector('button[aria-label="問題に戻る"]');
    returnButton?.click();
    await waitFor(() => document.querySelector('button[aria-label="採点"]')?.getAttribute('aria-pressed') === 'false', 'editing after grading');
    const returnedCells = [...document.querySelectorAll('.problem-cell-column-arithmetic-divide')];
    const returnedFirstCell = returnedCells[3];
    const returnedQuotient = returnedFirstCell?.querySelector('.column-division-answer-coordinate-quotient .column-digit-answer-quotient');
    const returnedRemainder = returnedFirstCell?.querySelector('.column-division-answer-coordinate-remainder math-field.answer-mathfield');
    const returnedDirectRemainder = returnedCells.at(-1)?.querySelector('.column-division-answer-coordinate-remainder math-field.answer-mathfield');
    const returnedQuotientValue = returnedQuotient
      ? [...returnedQuotient.querySelectorAll('.column-digit-slot-active')].map((slot) => slot.textContent?.trim() ?? '').join('').replace(/^0+(?=[0-9])/, '')
      : null;
    return {
      quotientCount: quotients.length,
      remainderCount: remainders.length,
      quotientLabel,
      remainderLabel,
      formulaLabels,
      quotientValue,
      remainderValue,
      noticeAfterAutoRemainder,
      directRemainderValue,
      noticeAfterDirectRemainder,
      autoMovedToRemainder,
      quotientDirection: quotient.getAttribute('data-column-direction'),
      remainderUsesDigitSlots: Boolean(firstCell.querySelector('.column-digit-answer-remainder')),
      bracketBorderTop: bracket ? getComputedStyle(bracket).borderTopWidth : null,
      bracketPath,
      gradedQuotientValue,
      gradedRemainderValue: gradedRemainder?.value ?? null,
      gradedDirectRemainderValue: gradedDirectRemainder?.value ?? null,
      returnedQuotientValue,
      returnedRemainderValue: returnedRemainder?.value ?? null,
      returnedDirectRemainderValue: returnedDirectRemainder?.value ?? null,
      returnedEditable: returnedRemainder?.getAttribute('aria-readonly') === 'false',
      quotientCorrectionInGrid: Boolean(quotientCorrection),
      quotientCorrectionValue,
      quotientCorrectionColor,
      remainderCorrectionInGrid: Boolean(remainderCorrection),
      remainderCorrectionValue,
      remainderCorrectionColor,
      separateCorrectAnswerCount: document.querySelectorAll('.column-grade-correct-answer').length,
      notice: document.querySelector('.worksheet-toast')?.getAttribute('aria-label') ?? null,
      selectedGridAlignment,
      gradedGridAlignment,
    };
  })()`;
}

function algebraKeypadProbe(closePanel = true) {
  return `(async () => {
    const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
    const waitFor = async (fn, label) => {
      for (let i = 0; i < 400; i += 1) { const value = fn(); if (value) return value; await sleep(25); }
      throw new Error('Timed out waiting for ' + label);
    };
    const field = await waitFor(() => document.querySelector('math-field.answer-mathfield'), 'quadratic answer field');
    field.click();
    const panel = await waitFor(() => document.querySelector('.input-panel'), 'quadratic input panel');
    const structures = [...panel.querySelectorAll('.formula-keypad-junior-high .formula-structure-key')].map((button) => {
      const rect = button.getBoundingClientRect();
      return { label: button.getAttribute('aria-label'), text: button.textContent?.trim() ?? '', disabled: button.disabled, top: rect.top, left: rect.left, right: rect.right, height: rect.height };
    });
    const operators = [...panel.querySelectorAll('.keypad-operators button')].map((button) => {
      const rect = button.getBoundingClientRect();
      return { label: button.getAttribute('aria-label'), text: button.textContent?.trim() ?? '', disabled: button.disabled, top: rect.top, left: rect.left, right: rect.right, height: rect.height };
    });
    const digit7 = panel.querySelector('.keypad-numbers-junior-high .keypad-digit');
    const numberGrid = panel.querySelector('.keypad-numbers-junior-high');
    const operatorGrid = panel.querySelector('.keypad-operators');
    const controls = panel.querySelector('.keypad-controls');
    const digitHeight = digit7?.getBoundingClientRect().height ?? 0;
    const rowGap = numberGrid ? parseFloat(getComputedStyle(numberGrid).rowGap || '0') : 0;
    const targetStructureHeight = digitHeight * 3 + rowGap * 2;
    const structureHeightDelta = Math.max(0, ...structures.map((item) => item.height)) - Math.min(...structures.map((item) => item.height));
    const operatorHeightDelta = Math.max(0, ...operators.map((item) => item.height)) - Math.min(...operators.map((item) => item.height));
    const structureHeight = structures[0]?.height ?? 0;
    const structureTop = structures.length ? Math.min(...structures.map((item) => item.top)) : 0;
    const structureBottom = structures.length ? Math.max(...structures.map((item) => item.top + item.height)) : 0;
    const structureStackHeight = structureBottom - structureTop;
    const operatorHeight = operators[0]?.height ?? 0;
    const numberRect = numberGrid?.getBoundingClientRect();
    const operatorRect = operatorGrid?.getBoundingClientRect();
    const controlRect = controls?.getBoundingClientRect();
    const operatorsBetweenNumbersAndControls = Boolean(numberRect && operatorRect && controlRect)
      && operatorRect.left >= numberRect.right - 1
      && operatorRect.right <= controlRect.left + 1;
    const groupTops = [...panel.querySelectorAll('.input-panel-inner-junior-high > *')].map((element) => element.getBoundingClientRect().top);

    const paintedBounds = () => {
      const content = field.shadowRoot?.querySelector('[part~=\"content\"]');
      const rects = content
        ? [content, ...content.querySelectorAll('*')]
            .map((element) => element.getBoundingClientRect())
            .filter((rect) => rect.width > 0 && rect.height > 0)
        : [];
      if (rects.length === 0) return null;
      return {
        left: Math.min(...rects.map((rect) => rect.left)),
        right: Math.max(...rects.map((rect) => rect.right)),
        top: Math.min(...rects.map((rect) => rect.top)),
        bottom: Math.max(...rects.map((rect) => rect.bottom)),
      };
    };
    const inputResults = [];
    const cellContainmentSample = () => {
      const cell = field.closest('.problem-cell')?.getBoundingClientRect();
      const painted = paintedBounds();
      const containsPaint = cell && painted
        ? painted.left >= cell.left + 3 && painted.right <= cell.right - 3
          && painted.top >= cell.top + 3 && painted.bottom <= cell.bottom - 3
        : true;
      return { rendered: Boolean(painted), containsPaint, cell, painted };
    };
    const containmentSample = () => {
      const frame = field.closest('.answer-box').getBoundingClientRect();
      const painted = paintedBounds();
      const containsPaint = painted
        ? painted.left >= frame.left - 1 && painted.right <= frame.right + 1
          && painted.top >= frame.top - 1 && painted.bottom <= frame.bottom + 1
        : true;
      return { rendered: Boolean(painted), containsPaint, frame: { width: frame.width, height: frame.height }, painted };
    };
    for (const latex of ['999999999999999999', '-1', '\\frac{1}{2}', '\\sqrt{2}', '1,2', '\\pm\\sqrt{2}', '\\sqrt{57\\pm\\sqrt{99}}{42}']) {
      field.setValue(latex, { silenceNotifications: false });
      const containmentSamples = [containmentSample()];
      field.dispatchEvent(new InputEvent('input', { bubbles: true, composed: true, inputType: 'insertText' }));
      containmentSamples.push(containmentSample());
      await new Promise(requestAnimationFrame);
      containmentSamples.push(containmentSample());
      await new Promise(requestAnimationFrame);
      containmentSamples.push(containmentSample());
      await sleep(120);
      containmentSamples.push(containmentSample());
      const notice = document.querySelector('.worksheet-toast')?.getAttribute('aria-label') ?? null;
      const retained = field.value;
      inputResults.push({
        latex,
        retained,
        notice,
        containsPaint: containmentSamples.every((sample) => sample.containsPaint),
        containmentSamples,
      });
    }
    // C-001 / M-001: parser-valid value with temporarily enlarged paint geometry.
    // This isolates visual overflow rollback from the independent AST-size limit.
    field.style.removeProperty('font-size');
    field.setValue('2', { silenceNotifications: false });
    field.dispatchEvent(new InputEvent('input', { bubbles: true, composed: true, inputType: 'insertText' }));
    await sleep(120);
    const schemaVersion = window.__AUTODRILL_SCHEMA_VERSION__;
    const parseRaw = await window.__AUTODRILL_WASM__.parse_mathlive_answer(JSON.stringify({
      schema_version: schemaVersion,
      input_interface: { type: 'structured_math', allowed_structures: ['fraction', 'root', 'negative', 'plus_minus', 'tuple', 'arithmetic'] },
      latex: '1',
    }));
    const parseResponse = typeof parseRaw === 'string' ? JSON.parse(parseRaw) : parseRaw;
    field.style.fontSize = '400px';
    field.setValue('1', { silenceNotifications: false });
    await new Promise(requestAnimationFrame);
    await new Promise(requestAnimationFrame);
    const overflowPaintBefore = cellContainmentSample();
    field.dispatchEvent(new InputEvent('input', { bubbles: true, composed: true, inputType: 'insertText' }));
    await new Promise(requestAnimationFrame);
    await new Promise(requestAnimationFrame);
    await sleep(120);
    const retainedAfterGuard = field.value;
    const noticeAfterGuard = document.querySelector('.worksheet-toast')?.getAttribute('aria-label') ?? null;
    field.style.removeProperty('font-size');
    await new Promise(requestAnimationFrame);
    await new Promise(requestAnimationFrame);
    const overflowResult = {
      parserAccepted: parseResponse?.ok === true,
      paintOverflowedBeforeGuard: overflowPaintBefore.rendered && !overflowPaintBefore.containsPaint,
      retained: retainedAfterGuard,
      notice: noticeAfterGuard,
      containsAfterRollback: cellContainmentSample().containsPaint,
    };

    const inner = panel.querySelector('.input-panel-inner').getBoundingClientRect();
    const numbers = panel.querySelector('.keypad-numbers').getBoundingClientRect();
    const numericCenterDelta = Math.abs((numbers.left + numbers.right) / 2 - (inner.left + inner.right) / 2);
    const closeButton = panel.querySelector('.input-panel-close');
    const closeButtonLabel = closeButton?.getAttribute('aria-label') ?? null;
    const closeButtonHasIcon = Boolean(closeButton?.querySelector('svg'));
    const closeButtonText = closeButton?.textContent?.trim() ?? '';
    const closePanel = ${JSON.stringify(closePanel)};
    if (closePanel) {
      window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
      await sleep(50);
    }
    const closedByEscape = closePanel ? !document.querySelector('.input-panel') : null;
    return {
      structures,
      operators,
      structureHeightDelta,
      operatorHeightDelta,
      structureHeight,
      structureStackHeight,
      operatorHeight,
      digitHeight,
      rowGap,
      targetStructureHeight,
      operatorsBetweenNumbersAndControls,
      maxGroupTopDelta: Math.max(0, ...groupTops) - Math.min(...groupTops),
      numericCenterDelta,
      closeButtonLabel,
      closeButtonHasIcon,
      closeButtonText,
      closedByEscape,
      inputResults,
      overflowResult,
    };
  })()`;
}

function signedClearProbe() {
  return `(async () => {
    const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
    const waitFor = async (fn, label) => {
      for (let i = 0; i < 400; i += 1) { const value = fn(); if (value) return value; await sleep(25); }
      throw new Error('Timed out waiting for ' + label);
    };
    const field = await waitFor(() => document.querySelector('math-field.answer-mathfield'), 'signed arithmetic answer field');
    field.click();
    const panel = await waitFor(() => document.querySelector('.input-panel'), 'signed arithmetic input panel');
    [...panel.querySelectorAll('.keypad-numbers button')].find((button) => button.textContent?.trim() === '1')?.click();
    await sleep(100);
    const before = field.value;
    panel.querySelector('.keypad-clear')?.click();
    await sleep(160);
    return {
      before,
      after: field.value,
      notice: document.querySelector('.worksheet-toast')?.getAttribute('aria-label') ?? null,
    };
  })()`;
}

function juniorHighKeypadShapeProbe() {
  return `(async () => {
    const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
    const waitFor = async (fn, label) => {
      for (let i = 0; i < 400; i += 1) { const value = fn(); if (value) return value; await sleep(25); }
      throw new Error('Timed out waiting for ' + label);
    };
    const field = await waitFor(() => document.querySelector('math-field.answer-mathfield'), 'junior-high answer field');
    field.click();
    const panel = await waitFor(() => document.querySelector('.input-panel'), 'junior-high input panel');
    const inner = panel.querySelector('.input-panel-inner-junior-high');
    const structureButtons = [...panel.querySelectorAll('.formula-keypad-junior-high .formula-structure-key')];
    const structureRects = structureButtons.map((button) => button.getBoundingClientRect());
    const operatorButtons = [...panel.querySelectorAll('.keypad-operators button')];
    const numberGrid = panel.querySelector('.keypad-numbers-junior-high');
    const operatorGrid = panel.querySelector('.keypad-operators');
    const controls = panel.querySelector('.keypad-controls');
    const numberRect = numberGrid?.getBoundingClientRect();
    const operatorRect = operatorGrid?.getBoundingClientRect();
    const controlRect = controls?.getBoundingClientRect();
    return {
      fullLayout: Boolean(inner),
      structures: structureButtons.map((button) => ({ label: button.getAttribute('aria-label'), disabled: button.disabled })),
      structure2x2: structureRects.length === 4
        && Math.abs(structureRects[0].top - structureRects[1].top) <= 1
        && Math.abs(structureRects[2].top - structureRects[3].top) <= 1
        && structureRects[2].top > structureRects[0].top
        && Math.abs(structureRects[0].left - structureRects[2].left) <= 1
        && Math.abs(structureRects[1].left - structureRects[3].left) <= 1,
      operators: operatorButtons.map((button) => ({ label: button.getAttribute('aria-label'), text: button.textContent?.trim() ?? '', disabled: button.disabled })),
      hasFixedDecimalKey: Boolean(numberGrid?.querySelector('.keypad-decimal')),
      operatorsBetweenNumbersAndControls: Boolean(numberRect && operatorRect && controlRect)
        && operatorRect.left >= numberRect.right - 1
        && operatorRect.right <= controlRect.left + 1,
    };
  })()`;
}

function directGenerationProbe(themeId, seed) {
  return `(async () => {
    const started = performance.now();
    const schemaVersion = window.__AUTODRILL_SCHEMA_VERSION__;
    if (!Number.isInteger(schemaVersion)) throw new Error('Current Rust/Web schema version is not exposed to the browser probe.');
    const raw = await window.__AUTODRILL_WASM__.generate_worksheet(JSON.stringify({
      schema_version: schemaVersion,
      numeric_theme_id: ${themeId},
      seed: ${JSON.stringify(seed)},
      difficulty: 3
    }));
    const response = typeof raw === 'string' ? JSON.parse(raw) : raw;
    return { elapsedMs: performance.now() - started, schemaVersion, ok: response?.ok === true, error: response?.error ?? null };
  })()`;
}

function printPreviewProbe(seed) {
  return `(async () => {
    const seed = ${JSON.stringify(seed)};
    const measureColumnLaneSafety = ${browserColumnLaneSafety.toString()};
    const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
    const waitFor = async (fn, label, attempts = 800) => {
      for (let i = 0; i < attempts; i += 1) { const value = fn(); if (value) return value; await sleep(25); }
      throw new Error('Timed out waiting for ' + label);
    };
    await waitFor(() => window.__AUTODRILL_WASM__, 'WASM');
    await waitFor(() => document.querySelector('button[aria-label="難易度"]'), 'difficulty');
    await new Promise(requestAnimationFrame);
    await new Promise(requestAnimationFrame);
    const advanced = await waitFor(() => document.querySelector('.advanced-settings'), 'advanced settings');
    if (!advanced.open) {
      advanced.querySelector('summary')?.click();
      await waitFor(() => advanced.open, 'advanced settings open');
      await new Promise(requestAnimationFrame);
      await new Promise(requestAnimationFrame);
    }
    const seedInput = document.querySelector('input[aria-label="Seed"]');
    const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set;
    setter.call(seedInput, seed);
    seedInput.dispatchEvent(new Event('input', { bubbles: true }));
    seedInput.dispatchEvent(new Event('change', { bubbles: true }));
    await new Promise(requestAnimationFrame);
    await new Promise(requestAnimationFrame);
    const generateButton = document.querySelector('button[aria-label="問題生成"]');
    const beforeGenerate = { disabled: generateButton?.disabled ?? null, label: generateButton?.getAttribute('aria-label') ?? null };
    generateButton?.click();
    let worksheetPaper = null;
    for (let i = 0; i < 800 && !worksheetPaper; i += 1) {
      worksheetPaper = document.querySelector('.worksheet-screen .paper');
      if (!worksheetPaper) await sleep(25);
    }
    if (!worksheetPaper) {
      return {
        generationFailed: true,
        beforeGenerate,
        currentGenerate: document.querySelector('button[aria-label^="問題"]')?.getAttribute('aria-label') ?? null,
        alert: document.querySelector('[role="alert"]')?.getAttribute('aria-label') ?? null,
        heading: document.querySelector('h1,h2')?.textContent ?? null,
        bodyBusy: document.querySelector('[aria-busy="true"]')?.className ?? null,
      };
    }
    const worksheetFooter = document.querySelector('[data-testid="worksheet-footer"]')?.textContent ?? '';
    if (!worksheetFooter.includes(seed)) throw new Error('Print worksheet does not use requested Seed ' + seed + ': ' + worksheetFooter);

    window.__AUTODRILL_PRINT_PROBE__ = null;
    window.print = () => {
      const spans = [...document.querySelectorAll('.worksheet-print-preview math-span')];
      const details = spans.map((element) => {
        const content = element.shadowRoot?.querySelector('[part~="render"]');
        const rect = content?.getBoundingClientRect();
        return {
          label: element.getAttribute('aria-label') ?? '',
          ready: Boolean(content && rect && rect.width > 0 && rect.height > 0),
        };
      });
      window.__AUTODRILL_PRINT_PROBE__ = {
        total: details.length,
        ready: details.filter((item) => item.ready).length,
        missing: details.filter((item) => !item.ready).map((item) => item.label),
      };
    };

    document.querySelector('button[aria-label="印刷"]').click();
    const preview = await waitFor(() => document.querySelector('.worksheet-print-preview'), 'print preview');
    const printButton = await waitFor(() => [...preview.querySelectorAll('button')].find((button) => button.textContent?.trim() === '印刷する'), 'print button');
    const initialDisabled = printButton.disabled;
    const problemPage = preview.querySelector('[data-print-page="problems"]');
    const stacked = [...problemPage.querySelectorAll('.problem-cell-answer-below')];
    const equalsCount = stacked.filter((cell) => cell.querySelector('math-span.problem-math-expression')?.getAttribute('aria-label')?.includes('=')).length;
    const cells = [...problemPage.querySelectorAll('.problem-cell')];
    const crossingDetails = cells.flatMap((cell) => {
      // Column arithmetic is positioned by the page-wide worksheet grid, not by
      // the invisible logical problem-cell edge. Its real page/sibling safety is
      // measured below by measureColumnLaneSafety, matching the Web probe.
      if (cell.classList.contains('problem-cell-column-arithmetic')) return [];
      const cellRect = cell.getBoundingClientRect();
      const entries = [...cell.querySelectorAll('.problem-number, math-span.problem-math-expression, .mini-sudoku-grid, .problem-answer-area')]
        .map((element) => ({ element, rect: element.getBoundingClientRect() }))
        .filter(({ rect }) => rect.width > 0 && rect.height > 0);
      if (entries.length === 0) return [];
      const minLeft = Math.min(...entries.map(({ rect }) => rect.left));
      const maxRight = Math.max(...entries.map(({ rect }) => rect.right));
      if (minLeft >= cellRect.left - 1 && maxRight <= cellRect.right + 1) return [];
      return [{
        problem: Number(cell.dataset.printProblemIndex ?? 0) + 1,
        leftOverflow: Math.max(0, Math.round((cellRect.left - minLeft) * 10) / 10),
        rightOverflow: Math.max(0, Math.round((maxRight - cellRect.right) * 10) / 10),
        offenders: entries
          .filter(({ rect }) => rect.left < cellRect.left - 1 || rect.right > cellRect.right + 1)
          .map(({ element, rect }) => ({
            className: typeof element.className === 'string' ? element.className : element.tagName,
            left: Math.round(rect.left * 10) / 10,
            right: Math.round(rect.right * 10) / 10,
            width: Math.round(rect.width * 10) / 10,
          })),
        cell: { left: Math.round(cellRect.left * 10) / 10, right: Math.round(cellRect.right * 10) / 10, width: Math.round(cellRect.width * 10) / 10 },
      }];
    });
    crossingDetails.push(...measureColumnLaneSafety(problemPage).map((issue) => ({
      problem: issue.problem,
      leftOverflow: issue.kind === 'page-overflow' ? issue.overflow : 0,
      rightOverflow: issue.kind === 'page-overflow' ? issue.overflow : 0,
      kind: issue.kind,
      overlap: issue.kind === 'lane-overlap' ? issue.overflow : 0,
    })));
    const crossings = crossingDetails.length;
    const dividerCount = problemPage.querySelectorAll('.problem-divider').length;
    const columnCells = problemPage.querySelectorAll('.problem-cell-column-arithmetic').length;
    const columnExpressions = problemPage.querySelectorAll('[data-column-arithmetic]').length;
    const emptyAnswers = problemPage.querySelectorAll('.worksheet-print-empty-answer').length;
    const answerPage = preview.querySelector('[data-print-page=\"answers\"]');
    const answerSolutions = [...(answerPage?.querySelectorAll('[data-column-solution=\"true\"]') ?? [])];
    const completedSolutions = answerSolutions.length;
    const visibleCompletedSolutions = answerSolutions.filter((solution) => {
      const rect = solution.getBoundingClientRect();
      return rect.width > 0 && rect.height > 0;
    }).length;
    const divisionSolutionSteps = answerPage?.querySelectorAll('.column-division-solution-step').length ?? 0;
    const divisionMinusSigns = answerPage?.querySelectorAll('.column-division-solution-minus').length ?? 0;
    const multiplicationPartials = answerPage?.querySelectorAll('.column-multiply-partial').length ?? 0;
    const answerPageEmptyAnswers = answerPage?.querySelectorAll('.worksheet-print-empty-answer').length ?? 0;
    const miniSudokuProblemGrids = problemPage.querySelectorAll('.mini-sudoku-grid').length;
    const miniSudokuAnswerGrids = answerPage?.querySelectorAll('.mini-sudoku-grid').length ?? 0;
    const miniSudokuProblemCells = problemPage.querySelectorAll('.mini-sudoku-grid [data-digit-grid-cell]').length;
    const miniSudokuAnswerCells = answerPage?.querySelectorAll('.mini-sudoku-grid [data-digit-grid-cell]').length ?? 0;
    const miniSudokuPrintButtons = preview.querySelectorAll('.mini-sudoku-grid button').length;
    const answerCrossingDetails = measureColumnLaneSafety(answerPage).map((issue) => ({
      problem: issue.problem,
      leftOverflow: issue.kind === 'page-overflow' ? issue.overflow : 0,
      rightOverflow: issue.kind === 'page-overflow' ? issue.overflow : 0,
      topOverflow: 0,
      bottomOverflow: 0,
      kind: issue.kind,
      overlap: issue.kind === 'lane-overlap' ? issue.overflow : 0,
    }));
    const answerCrossings = answerCrossingDetails.length;
    const divisionProblemFontSizes = [...new Set([...problemPage.querySelectorAll('.problem-cell-column-arithmetic-divide .expression')].map((expression) => getComputedStyle(expression).fontSize))];
    const divisionAnswerFontSizes = [...new Set([...(answerPage?.querySelectorAll('.column-division-solution') ?? [])].map((solution) => getComputedStyle(solution).fontSize))];
    printButton.click();
    const result = await waitFor(() => window.__AUTODRILL_PRINT_PROBE__, 'native print callback', 320);
    return { ...result, initialDisabled, stacked: stacked.length, equalsCount, crossings, crossingDetails, answerCrossings, answerCrossingDetails, dividerCount, columnCells, columnExpressions, emptyAnswers, completedSolutions, visibleCompletedSolutions, divisionSolutionSteps, divisionMinusSigns, divisionProblemFontSizes, divisionAnswerFontSizes, multiplicationPartials, answerPageEmptyAnswers, miniSudokuProblemGrids, miniSudokuAnswerGrids, miniSudokuProblemCells, miniSudokuAnswerCells, miniSudokuPrintButtons };
  })()`;
}

const httpPort = await new Promise((resolveListen, reject) => {
  server.once('error', reject);
  server.listen(0, '127.0.0.1', () => {
    const address = server.address();
    resolveListen(typeof address === 'object' && address ? address.port : 0);
  });
});
const debugPort = await freePort();
const userDataDir = mkdtempSync(join(tmpdir(), 'autodrill-layout-'));
const chromeArgs = [
  '--headless=new',
  `--remote-debugging-port=${debugPort}`,
  `--user-data-dir=${userDataDir}`,
  '--disable-gpu',
  '--no-first-run',
  '--no-default-browser-check',
  '--disable-background-networking',
  '--disable-extensions',
];
if (process.env.CI === 'true' && process.platform === 'linux') {
  chromeArgs.push('--no-sandbox', '--disable-dev-shm-usage');
}
chromeArgs.push('about:blank');
const chrome = spawn(chromeBinary(), chromeArgs, { stdio: ['ignore', 'ignore', 'pipe'] });
let chromeStderr = '';
chrome.stderr.setEncoding('utf8');
chrome.stderr.on('data', (chunk) => { chromeStderr += chunk; });

let cdp;
try {
  let targets;
  try {
    targets = await waitForJson(`http://127.0.0.1:${debugPort}/json/list`, 60_000);
  } catch (error) {
    const diagnostics = chromeStderr.trim();
    throw new Error(`Chrome DevTools did not become reachable on port ${debugPort}.${diagnostics ? `\nChrome stderr:\n${diagnostics}` : ''}`, { cause: error });
  }
  const page = targets.find((target) => target.type === 'page');
  if (!page) throw new Error('Chrome page target not found.');
  cdp = await connectCdp(page.webSocketDebuggerUrl);
  await cdp.send('Runtime.enable');
  await cdp.send('Page.enable');
  await cdp.send('Emulation.setDeviceMetricsOverride', VIEWPORT);
  const origin = `http://127.0.0.1:${httpPort}`;
  if (!PRINT_ONLY) {
    await navigate(cdp, `${origin}${BASE_PATH}/`);
    const gradingSettings = await cdp.evaluate(gradingSettingsProbe());
    const expectedGradingSettings = [
      { label: '約分しましょうの採点', pressed: ['×'] },
      { label: '整数でこたえましょうの採点', pressed: ['×'] },
      { label: '分数でこたえましょうの採点', pressed: ['○'] },
      { label: '最後まで計算しましょうの採点', pressed: ['×'] },
    ];
    const expectedGradingDescriptions = [
      { label: '例: 2/4 と 1/2 の表記を区別します。', math: [{ ariaLabel: '2/4', mode: 'displaystyle' }, { ariaLabel: '1/2', mode: 'displaystyle' }] },
      { label: '例: √16 と 4 の表記を区別します。', math: [{ ariaLabel: '√16', mode: 'displaystyle' }, { ariaLabel: '4', mode: 'displaystyle' }] },
      { label: '例: 0.5 と 1/2 の表記を区別します。', math: [{ ariaLabel: '0.5', mode: 'displaystyle' }, { ariaLabel: '1/2', mode: 'displaystyle' }] },
      { label: '上の3項目以外の、数学的に同値だが未整理・冗長な表記の違いを区別します。', math: [] },
    ];
    if (
      JSON.stringify(gradingSettings.groups) !== JSON.stringify(expectedGradingSettings)
      || JSON.stringify(gradingSettings.descriptions) !== JSON.stringify(expectedGradingDescriptions)
      || gradingSettings.advancedRubyCount !== 0
      || gradingSettings.dialogRubyCount !== 0
      || !gradingSettings.fractionCorrectPressed
      || !gradingSettings.dialogClosed
    ) {
      throw new Error(`Detailed settings/grading modal has ruby, wrong defaults/examples, cannot toggle, or cannot close: ${JSON.stringify(gradingSettings)}`);
    }
    const gradeTags = await cdp.evaluate(recommendedGradeTagProbe());
    const expectedTags = [['一次方程式(1)', '中1', 'grade-tag-grade-7'], ['連立方程式(1)', '中2', 'grade-tag-grade-8'], ['二次方程式(1)', '中3', 'grade-tag-grade-9']];
    for (const [label, tag, className] of expectedTags) {
      const row = gradeTags.find((item) => item.label === label);
      if (!row || row.tag !== tag || !String(row.className).includes(className) || !row.sameRow || !(row.tagRight <= row.checkLeft)) {
        throw new Error(`Recommended grade tag is missing or not immediately left of the check slot: ${JSON.stringify(gradeTags)}`);
      }
    }
    await verifySelectHitbox(cdp, '難易度');
    const dropdown = await cdp.evaluate(dropdownProbe);
    if (!dropdown.difficulty.visible || dropdown.difficulty.selected !== '4') {
      throw new Error(`Difficulty dropdown final option is clipped or not selectable: ${JSON.stringify(dropdown.difficulty)}`);
    }
    if (!dropdown.grade.visible || dropdown.grade.selected !== 'grade-9') {
      throw new Error(`Grade dropdown final option is clipped or not selectable: ${JSON.stringify(dropdown.grade)}`);
    }


    await navigate(cdp, `${origin}${BASE_PATH}/`);
    const optionCoverage = await cdp.evaluate(settingsOptionCoverageProbe());
    const sitemapThemeCount = [...readFileSync(join(OUT, 'sitemap.xml'), 'utf8').matchAll(/<loc>([^<]+)<\/loc>/g)]
      .map((match) => new URL(match[1]).pathname)
      .filter((pathname) => pathname.includes('/drills/')).length;
    if (optionCoverage.alert || optionCoverage.difficulties === 0 || optionCoverage.grades === 0 || optionCoverage.recommendedGenres === 0 || optionCoverage.themes !== sitemapThemeCount) {
      throw new Error(`Settings option coverage did not exercise the complete current option surface: ${JSON.stringify({ ...optionCoverage, sitemapThemeCount })}`);
    }
    console.log(`[interaction] settings options: difficulties=${optionCoverage.difficulties}, grades=${optionCoverage.grades}, themes=${optionCoverage.themes}`);

    await navigate(cdp, `${origin}${BASE_PATH}/drills/grade-1/one-digit-addition/`);
    const stateGraph = await cdp.evaluate(uiStateGraphProbe());
    const stateManifest = {
      settings: ['settings:advanced', 'settings:furigana', 'settings:generate', 'settings:mode:おすすめ', 'settings:mode:学年から選ぶ', 'settings:print', 'settings:select:ジャンル', 'settings:select:テーマ', 'settings:select:難易度'],
      settingsGrade: ['settings:advanced', 'settings:furigana', 'settings:generate', 'settings:mode:おすすめ', 'settings:mode:学年から選ぶ', 'settings:print', 'settings:select:ジャンル', 'settings:select:テーマ', 'settings:select:学年', 'settings:select:難易度'],
      settingsAdvanced: ['settings:advanced', 'settings:furigana', 'settings:generate', 'settings:grading-settings', 'settings:mode:おすすめ', 'settings:mode:学年から選ぶ', 'settings:print', 'settings:seed', 'settings:select:ジャンル', 'settings:select:テーマ', 'settings:select:難易度'],
      gradingModal: [
        'modal:採点設定を閉じる',
        'modal:約分しましょうを丸にする', 'modal:約分しましょうをバツにする',
        'modal:整数でこたえましょうを丸にする', 'modal:整数でこたえましょうをバツにする',
        'modal:分数でこたえましょうを丸にする', 'modal:分数でこたえましょうをバツにする',
        'modal:最後まで計算しましょうを丸にする', 'modal:最後まで計算しましょうをバツにする',
      ],
      printPreview: ['preview:戻る', 'preview:解答を逆さにする', 'preview:印刷する'],
      worksheetEditing: ['problem:math-field', 'worksheet:TOPに戻る', 'worksheet:印刷', 'worksheet:採点'],
      inputPanel: [
        'problem:math-field',
        'worksheet:TOPに戻る', 'worksheet:印刷', 'worksheet:採点',
        'input:入力パネルを閉じる',
        'input:7', 'input:8', 'input:9', 'input:4', 'input:5', 'input:6', 'input:1', 'input:2', 'input:3', 'input:0',
        'input:カーソルを左へ', 'input:カーソルを右へ', 'input:一文字戻す', 'input:クリア', 'input:確定',
      ],
      worksheetGraded: ['graded:もう一回問題を解く', 'graded:問題に戻る', 'worksheet:TOPに戻る', 'worksheet:印刷'],
    };
    const stateCensusFailures = Object.entries(stateManifest).flatMap(([state, expected]) => {
      const actual = stateGraph.states[state] ?? [];
      return JSON.stringify([...actual].sort()) === JSON.stringify([...expected].sort()) ? [] : [{ state, expected: [...expected].sort(), actual }];
    });
    const failedEdges = stateGraph.edges.filter((edge) => !edge.ok);
    if (stateGraph.alert || stateGraph.obsoleteDifferentWorksheetPresent || stateCensusFailures.length > 0 || failedEdges.length > 0) {
      throw new Error(`UI state graph coverage failed: ${JSON.stringify({ alert: stateGraph.alert, obsoleteDifferentWorksheetPresent: stateGraph.obsoleteDifferentWorksheetPresent, stateCensusFailures, failedEdges })}`);
    }
    console.log(`[interaction] UI state graph: ${stateGraph.edges.length} one-step edges verified across ${Object.keys(stateManifest).length} user-visible states`);
  }

  if (!PRINT_ONLY && GENERATION_PROBE) {
    await navigate(cdp, `${origin}${BASE_PATH}/`);
    const runtimeReady = await cdp.evaluate(`(async () => { for (let i = 0; i < 400; i += 1) { if (window.__AUTODRILL_WASM__ && Number.isInteger(window.__AUTODRILL_SCHEMA_VERSION__)) return true; await new Promise((resolve) => setTimeout(resolve, 25)); } return false; })()`);
    if (!runtimeReady) throw new Error('WASM was not ready before the CPU-throttled generation probe.');
    if (CPU_THROTTLE_RATE > 1) await cdp.send('Emulation.setCPUThrottlingRate', { rate: CPU_THROTTLE_RATE });
    let generation;
    try { generation = await cdp.evaluate(directGenerationProbe(CPU_THROTTLE_THEME_ID, SEEDS[0] ?? 'A1b2')); }
    finally { if (CPU_THROTTLE_RATE > 1) await cdp.send('Emulation.setCPUThrottlingRate', { rate: 1 }); }
    if (!generation.ok) throw new Error(`CPU-throttled generation failed: ${JSON.stringify(generation)}`);
    console.log(`[generation] theme=${CPU_THROTTLE_THEME_ID} cpu=${CPU_THROTTLE_RATE}x elapsed=${Math.round(generation.elapsedMs)}ms`);
  }

  const sitemap = readFileSync(join(OUT, 'sitemap.xml'), 'utf8');
  const routes = [...sitemap.matchAll(/<loc>([^<]+)<\/loc>/g)]
    .map((match) => new URL(match[1]).pathname)
    .map((pathname) => pathname.startsWith(BASE_PATH) ? (pathname.slice(BASE_PATH.length) || '/') : pathname)
    .filter((pathname) => pathname !== '/')
    .filter((pathname) => !ROUTE_FILTER || pathname.includes(ROUTE_FILTER));
  console.log(`[layout] selected ${routes.length} worksheet route(s)${ROUTE_FILTER ? ` for filter ${ROUTE_FILTER}` : ''}`);
  const failures = [];
  let worksheetSampleCount = 0;
  let answerAffordanceActionCount = 0;
  let inputPanelActionCount = 0;
  const inputPanelSignatures = new Set();
  if (!PRINT_ONLY) {
    for (const route of routes) {
      const routeSeeds = route.includes('/signed-arithmetic-') ? [...SEEDS, ...EXTRA_SIGNED_SEEDS] : SEEDS;
      for (const seed of routeSeeds) {
        worksheetSampleCount += 1;
        const localRoute = route.endsWith('/') ? route : `${route}/`;
        console.log(`[layout] checking ${route} seed=${seed}`);
        await navigate(cdp, `${origin}${BASE_PATH}${localRoute}`);
        let result;
        const inputDifficulty = route.endsWith('/column-division-one-digit') || route.endsWith('/column-division-two-digit') ? 'ふつう' : 'むずかしい';
        try {
          result = await cdp.evaluate(worksheetProbe(seed, inputDifficulty));
        } catch (error) {
          throw new Error(`Worksheet probe failed for ${route} seed=${seed}; console=${cdp.consoleErrors.join(' | ') || 'none'}`, { cause: error });
        }
        if (result.alert) failures.push({ route, seed, reason: `UI alert: ${result.alert}` });
        for (const crossing of result.crossings) {
          console.warn(`[layout] divider crossing: ${route} seed=${seed} problem=${crossing.problem} overflow=${crossing.overflow}px expression=${crossing.expression}`);
          failures.push({ route, seed, ...crossing });
        }
        for (const mismatch of result.columnGridMismatches ?? []) {
          console.warn(`[layout] column grid mismatch: ${route} seed=${seed} problem=${mismatch.problem} horizontal=${mismatch.horizontalDelta}px vertical=${mismatch.verticalDelta}px`);
          failures.push({ route, seed, reason: `column arithmetic answer is not on the shared digit grid`, ...mismatch });
        }
        if (result.worksheetGridAlignment?.applicable && result.worksheetGridAlignment.maxError > 0.25) {
          failures.push({ route, seed, reason: `worksheet-grid paint is not aligned to the worksheet background grid`, alignment: result.worksheetGridAlignment });
        }
        if (result.worksheetGridAlignment?.applicable && seed === SEEDS[0]) {
          const printAlignment = await cdp.evaluate(worksheetGridPrintAlignmentProbe());
          if (!printAlignment.problems?.applicable || !printAlignment.answers?.applicable
            || printAlignment.problems.maxError > 0.5 || printAlignment.answers.maxError > 0.5) {
            failures.push({ route, seed, reason: `worksheet-grid print problem/answer paint is not aligned to the worksheet background grid`, printAlignment });
          }
          console.log(`[grid] ${route} seed=${seed}: web=${result.worksheetGridAlignment.maxError}px print=${printAlignment.problems.maxError}px answer=${printAlignment.answers.maxError}px`);
        }
        if (cdp.consoleErrors.length > 0) failures.push({ route, seed, reason: `console errors: ${cdp.consoleErrors.join(' | ')}` });
        console.log(`[layout] ${route} seed=${seed}: ${result.count} problems, ${result.gradeClass}, expression ${result.fontSize}, crossings=${result.crossings.length}, gridMismatches=${result.columnGridMismatches?.length ?? 0}`);
        if (
          seed === SEEDS[0]
          && ['/signed-arithmetic-1', '/linear-equation-1', '/simultaneous-equation-1'].some((suffix) => route.endsWith(suffix))
        ) {
          const keypad = await cdp.evaluate(juniorHighKeypadShapeProbe());
          const labels = keypad.structures.map((item) => item.label);
          if (
            !keypad.fullLayout
            || keypad.structures.length !== 4
            || JSON.stringify(labels.slice(0, 3)) !== JSON.stringify(['分数', '帯分数', '平方根'])
            || !['複数解', 'x, y'].includes(labels[3])
            || keypad.structures.some((item) => item.disabled)
            || !keypad.structure2x2
            || JSON.stringify(keypad.operators.map((item) => item.text)) !== JSON.stringify(['+', '−', '±'])
            || keypad.operators.some((item) => item.disabled)
            || !keypad.hasFixedDecimalKey
            || !keypad.operatorsBetweenNumbersAndControls
          ) {
            failures.push({ route, seed, reason: `junior-high fixed keypad shape mismatch: ${JSON.stringify(keypad)}` });
          }
        }
        if (route.endsWith('/signed-arithmetic-1') && seed === SEEDS[0]) {
          const clear = await cdp.evaluate(signedClearProbe());
          if (clear.before !== '1' || clear.after !== '' || clear.notice === '式が大きすぎます！') {
            failures.push({ route, seed, reason: `shared MathLive clear regression: ${JSON.stringify(clear)}` });
          }
        }
        if (route.endsWith('/simultaneous-equation-1') && seed === SEEDS[0]) {
          const input = await cdp.evaluate(simultaneousInputProbe());
          if (
            input.fieldCount !== 24
            || JSON.stringify(input.coordinates.map((item) => item.prefix)) !== JSON.stringify(['x =', 'y ='])
            || JSON.stringify(input.coordinates.map((item) => item.prefixLatex)) !== JSON.stringify(['x=', 'y='])
            || input.xValue !== '2'
            || input.yValue !== '3'
          ) {
            failures.push({ route, seed, reason: `simultaneous separate-coordinate input mismatch: ${JSON.stringify(input)}` });
          }
          if (input.notice === '式が大きすぎます！') {
            failures.push({ route, seed, reason: `simultaneous coordinate input was rejected as too large: ${JSON.stringify(input)}` });
          }
        }
        if (route.endsWith('/mini-sudoku') && seed === SEEDS[0]) {
          const input = await cdp.evaluate(miniSudokuInputProbe());
          if (
            input.gridCount !== 4
            || input.counts.some((count) => count.cells !== 16 || count.editable < 5 || count.editable > 10 || count.givens + count.editable !== 16)
            || JSON.stringify(input.numberKeys) !== JSON.stringify(['1', '2', '3', '4'])
            || !Number.isInteger(input.selectedIndex)
            || input.written !== input.numberKeys[0]
            || input.maxSquareDelta > 1
            || input.gridWidthDelta > 1
            || input.overlay.content === 'none'
            || input.overlay.pointerEvents !== 'none'
            || input.overlay.boxShadow === 'none'
            || (input.overlay.backgroundImage.match(/linear-gradient/g) ?? []).length < 4
            || !input.panelDigitGrid
            || !input.gradedGridAlignment?.applicable || input.gradedGridAlignment.maxError > 0.25
          ) {
            failures.push({ route, seed, reason: `mini sudoku grid/input mismatch: ${JSON.stringify(input)}` });
          }
          console.log(`[input] mini-sudoku: ${JSON.stringify(input)}`);
        }
        if (route.endsWith('/column-addition-two-digit') && seed === SEEDS[0]) {
          const input = await cdp.evaluate(columnAdditionInputProbe());
          if (
            input.value !== '164'
            || input.fieldCount !== 16
            || input.direction !== 'right-to-left'
            || !input.domFocusFollowsSelection
            || Math.abs(input.topDelta) > 2
            || Math.abs(input.rightDelta) > 1
            || input.maxAdjacentGap > 1
            || input.maxCellSizeDelta > 1
            || input.slotFontScale === null || input.slotFontScale > 0.8
            || input.slotOverflow !== 'hidden'
            || !input.slotFont?.includes('Noto Sans JP')
            || input.feedbackCount !== 16
            || !['○', '✓'].includes(input.firstMark)
            || !input.firstMarkAnchoredAboveProblemNumber
            || input.firstMarkColor !== 'rgb(210, 11, 11)'
            || !input.allMarksRed
            || input.markToNumberFontRatio === null || input.markToNumberFontRatio < 1.5
            || input.gradedValue !== input.value
            || input.separateCorrectAnswerCount !== 0
            || (input.firstMark === '✓' && (!input.correctionInGrid || !input.correctionValue || input.correctionColor !== 'rgb(210, 11, 11)'))
            || !input.selectedGridAlignment?.applicable || input.selectedGridAlignment.maxError > 0.25
            || !input.gradedGridAlignment?.applicable || input.gradedGridAlignment.maxError > 0.25
          ) {
            failures.push({ route, seed, reason: `column addition digit-grid/grading mismatch: ${JSON.stringify(input)}` });
          }
          if (process.env.AUTODRILL_CAPTURE_COLUMN_INPUT_SCREENSHOT) {
            const screenshot = await cdp.send('Page.captureScreenshot', { format: 'png', captureBeyondViewport: true });
            writeFileSync(process.env.AUTODRILL_CAPTURE_COLUMN_INPUT_SCREENSHOT, Buffer.from(screenshot.data, 'base64'));
          }
          console.log(`[input] column-addition-two-digit: ${JSON.stringify(input)}`);
        }
        if (route.endsWith('/column-decimal-add-subtract') && seed === SEEDS[0]) {
          const decimalInput = await cdp.evaluate(columnDecimalInputProbe());
          if (
            decimalInput.fieldCount !== 16
            || !decimalInput.markerFound
            || decimalInput.decimalKeyCount !== 0
            || decimalInput.boundaryDelta > 1
            || decimalInput.rightmostValue !== '5'
            || decimalInput.direction !== 'right-to-left'
          ) {
            failures.push({ route, seed, reason: `decimal column fixed-point digit input mismatch: ${JSON.stringify(decimalInput)}` });
          }
        }
        if ((route.endsWith('/column-division-one-digit') || route.endsWith('/column-division-two-digit')) && seed === SEEDS[0]) {
          const keyboardTarget = await cdp.evaluate(columnDivisionKeyboardTargetProbe());
          await mouseClick(cdp, keyboardTarget.x, keyboardTarget.y);
          const keyboardFocused = await cdp.evaluate(columnDivisionKeyboardFocusProbe());
          await typeKeyboardText(cdp, '2');
          const keyboardAfterFirstDigit = await cdp.evaluate(columnDivisionKeyboardResultProbe());
          await typeKeyboardText(cdp, '1');
          const keyboardInput = await cdp.evaluate(columnDivisionKeyboardResultProbe());
          if (
            !keyboardFocused
            || keyboardAfterFirstDigit.value !== '2'
            || !keyboardInput.selected
            || keyboardInput.value !== '21'
            || keyboardInput.notice === '式が大きすぎます！'
          ) {
            failures.push({ route, seed, reason: `column division physical-keyboard remainder input mismatch: ${JSON.stringify({ keyboardTarget, keyboardFocused, keyboardAfterFirstDigit, keyboardInput })}` });
          }

          const input = await cdp.evaluate(columnDivisionInputProbe());
          if (process.env.AUTODRILL_CAPTURE_DIVISION_INPUT_SCREENSHOT) {
            const screenshot = await cdp.send('Page.captureScreenshot', { format: 'png', captureBeyondViewport: true });
            writeFileSync(process.env.AUTODRILL_CAPTURE_DIVISION_INPUT_SCREENSHOT, Buffer.from(screenshot.data, 'base64'));
          }
          if (
            input.quotientCount !== 12
            || input.remainderCount !== 12
            || input.quotientLabel !== '商'
            || input.remainderLabel !== 'あまり'
            || input.remainderValue !== '21'
            || input.noticeAfterAutoRemainder === '式が大きすぎます！'
            || input.directRemainderValue !== '21'
            || input.noticeAfterDirectRemainder === '式が大きすぎます！'
            || input.gradedDirectRemainderValue !== input.directRemainderValue
            || input.returnedQuotientValue !== input.quotientValue
            || input.returnedRemainderValue !== input.remainderValue
            || input.returnedDirectRemainderValue !== input.directRemainderValue
            || !input.returnedEditable
            || !input.autoMovedToRemainder
            || input.quotientDirection !== 'left-to-right'
            || input.remainderUsesDigitSlots
            || input.formulaLabels.length !== 0
            || input.bracketBorderTop !== '0px'
            || input.bracketPath !== 'M 0 28 C 7 21 7 7 0 0 L 100 0'
            || input.gradedQuotientValue !== input.quotientValue
            || input.gradedRemainderValue !== input.remainderValue
            || !input.quotientCorrectionInGrid
            || !input.quotientCorrectionValue
            || input.quotientCorrectionColor !== 'rgb(210, 11, 11)'
            || !input.remainderCorrectionInGrid
            || !input.remainderCorrectionValue
            || input.remainderCorrectionColor !== 'rgb(210, 11, 11)'
            || input.separateCorrectAnswerCount !== 0
            || !input.selectedGridAlignment?.applicable || input.selectedGridAlignment.maxError > 0.25
            || !input.gradedGridAlignment?.applicable || input.gradedGridAlignment.maxError > 0.25
          ) {
            failures.push({ route, seed, reason: `column division final-answer input mismatch: ${JSON.stringify(input)}` });
          }
          if (input.notice === '式が大きすぎます！') {
            failures.push({ route, seed, reason: `column division coordinate input was rejected as too large: ${JSON.stringify(input)}` });
          }
        }
        if (route.endsWith('/quadratic-equation-1') && seed === SEEDS[0]) {
          const captureKeypad = Boolean(process.env.AUTODRILL_CAPTURE_KEYPAD_SCREENSHOT);
          const keypad = await cdp.evaluate(algebraKeypadProbe(!captureKeypad));
          if (captureKeypad) {
            const screenshot = await cdp.send('Page.captureScreenshot', { format: 'png', captureBeyondViewport: true });
            writeFileSync(process.env.AUTODRILL_CAPTURE_KEYPAD_SCREENSHOT, Buffer.from(screenshot.data, 'base64'));
            keypad.closedByEscape = await cdp.evaluate(`(async () => { window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true })); await new Promise((resolve) => setTimeout(resolve, 50)); return !document.querySelector('.input-panel'); })()`);
          }
          const expectedStructureLabels = ['分数', '帯分数', '平方根', '複数解'];
          const expectedOperatorLabels = ['プラスを挿入', 'マイナスを挿入', 'プラスマイナスを挿入'];
          const expectedOperatorTexts = ['+', '−', '±'];
          if (JSON.stringify(keypad.structures.map((item) => item.label)) !== JSON.stringify(expectedStructureLabels)) {
            failures.push({ route, seed, reason: `quadratic fixed structure-key order mismatch: ${JSON.stringify(keypad.structures)}` });
          }
          if (JSON.stringify(keypad.operators.map((item) => item.label)) !== JSON.stringify(expectedOperatorLabels)
            || JSON.stringify(keypad.operators.map((item) => item.text)) !== JSON.stringify(expectedOperatorTexts)) {
            failures.push({ route, seed, reason: `quadratic compact operator-strip mismatch: ${JSON.stringify(keypad.operators)}` });
          }
          if (keypad.structureHeightDelta > 1 || Math.abs(keypad.structureStackHeight - keypad.targetStructureHeight) > 2) {
            failures.push({ route, seed, reason: `quadratic 2x2 structure grid does not match the combined 7/4/1 row height: ${JSON.stringify({ structureStackHeight: keypad.structureStackHeight, target: keypad.targetStructureHeight, buttonHeight: keypad.structureHeight, delta: keypad.structureHeightDelta })}` });
          }
          if (keypad.operatorHeightDelta > 1 || Math.abs(keypad.operatorHeight - keypad.digitHeight) > 1 || !keypad.operatorsBetweenNumbersAndControls) {
            failures.push({ route, seed, reason: `quadratic +/−/± strip is not compactly placed between numbers and controls: ${JSON.stringify({ digitHeight: keypad.digitHeight, operatorHeight: keypad.operatorHeight, between: keypad.operatorsBetweenNumbersAndControls })}` });
          }
          if (keypad.maxGroupTopDelta > 1) {
            failures.push({ route, seed, reason: `quadratic keypad groups wrapped vertically: top delta=${keypad.maxGroupTopDelta}px` });
          }
          if (keypad.closeButtonLabel !== '入力パネルを閉じる' || !keypad.closeButtonHasIcon || keypad.closeButtonText !== '' || !keypad.closedByEscape) {
            failures.push({ route, seed, reason: `input panel close controls are incomplete or still text-based: ${JSON.stringify({ closeButtonLabel: keypad.closeButtonLabel, closeButtonHasIcon: keypad.closeButtonHasIcon, closeButtonText: keypad.closeButtonText, closedByEscape: keypad.closedByEscape })}` });
          }
          for (const input of keypad.inputResults) {
            if (input.notice === '式が大きすぎます！') {
              failures.push({ route, seed, reason: `valid structured input was rejected as too large: ${input.latex}` });
            }
            if (!input.containmentSamples.some((sample) => sample.rendered)) {
              failures.push({ route, seed, reason: `MathLive input never rendered during containment probe: ${input.latex}; ${JSON.stringify(input)}` });
            } else if (!input.containsPaint) {
              failures.push({ route, seed, reason: `answer frame does not contain rendered MathLive input on every painted frame: ${input.latex}; ${JSON.stringify(input)}` });
            }
          }
          if (
            !keypad.overflowResult.parserAccepted
            || !keypad.overflowResult.paintOverflowedBeforeGuard
            || keypad.overflowResult.retained !== '2'
            || keypad.overflowResult.notice !== '式が大きすぎます！'
            || !keypad.overflowResult.containsAfterRollback
          ) {
            failures.push({ route, seed, reason: `visual overflow did not parse-successfully roll back to the last accepted MathLive value: ${JSON.stringify(keypad.overflowResult)}` });
          }
        }
        if (seed === SEEDS[0]) {
          await navigate(cdp, `${origin}${BASE_PATH}${localRoute}`);
          await cdp.evaluate(worksheetProbe(seed, inputDifficulty));
          const affordanceCoverage = await cdp.evaluate(worksheetAffordanceCoverageProbe());
          answerAffordanceActionCount += affordanceCoverage.attempted;
          if (affordanceCoverage.attempted === 0 || affordanceCoverage.failures.length > 0) {
            failures.push({ route, seed, reason: `worksheet editable-affordance coverage failed: ${JSON.stringify(affordanceCoverage)}` });
          }
          // Input-panel actions are one-step edges from a canonical fresh worksheet,
          // not a continuation of the preceding all-affordances mutation sweep.
          await navigate(cdp, `${origin}${BASE_PATH}${localRoute}`);
          await cdp.evaluate(worksheetProbe(seed, inputDifficulty));
          const panelCoverage = await cdp.evaluate(openInputPanelCoverageProbe());
          if (panelCoverage.applicable && !inputPanelSignatures.has(panelCoverage.signature)) {
            inputPanelSignatures.add(panelCoverage.signature);
            const actions = await cdp.evaluate(exerciseInputPanelActionsProbe());
            inputPanelActionCount += actions.executed;
            if (actions.executed !== actions.descriptors.length || actions.failures.length > 0) {
              failures.push({ route, seed, reason: `input-panel one-step action coverage failed: ${JSON.stringify(actions)}` });
            }
          }
        }
      }
    }
  }
  if (!PRINT_ONLY && (worksheetSampleCount === 0 || answerAffordanceActionCount === 0)) {
    throw new Error(`Browser interaction verification selected zero required samples: worksheets=${worksheetSampleCount}, answerAffordances=${answerAffordanceActionCount}${ROUTE_FILTER ? ` for filter ${ROUTE_FILTER}` : ''}.`);
  }

  if (!SKIP_PRINT_PROBES) {
    await navigate(cdp, `${origin}${BASE_PATH}/drills/bonus/mini-sudoku/`);
    const miniSudokuPrint = await cdp.evaluate(printPreviewProbe('A1b2'));
    if (miniSudokuPrint.generationFailed) throw new Error(`Mini sudoku print probe could not generate worksheet: ${JSON.stringify(miniSudokuPrint)}`);
    if (
      miniSudokuPrint.miniSudokuProblemGrids !== 4
      || miniSudokuPrint.miniSudokuAnswerGrids !== 4
      || miniSudokuPrint.miniSudokuProblemCells !== 64
      || miniSudokuPrint.miniSudokuAnswerCells !== 64
      || miniSudokuPrint.miniSudokuPrintButtons !== 0
      || miniSudokuPrint.dividerCount !== 0
      || miniSudokuPrint.crossings !== 0
    ) {
      failures.push({ route: 'mini-sudoku', seed: 'A1b2', reason: `mini sudoku print structure mismatch: ${JSON.stringify(miniSudokuPrint)}` });
    }
    const miniSudokuPrinted = await cdp.send('Page.printToPDF', { printBackground: true, preferCSSPageSize: true });
    const miniSudokuPdf = Buffer.from(miniSudokuPrinted.data, 'base64');
    const miniSudokuPageCount = (miniSudokuPdf.toString('latin1').match(/\/Type\s*\/Page\b/g) ?? []).length;
    if (miniSudokuPdf.length < 10_000 || miniSudokuPdf.subarray(0, 4).toString('ascii') !== '%PDF' || miniSudokuPageCount !== 2) {
      failures.push({ route: 'mini-sudoku', seed: 'A1b2', reason: `actual mini sudoku Chrome PDF invalid: bytes=${miniSudokuPdf.length}, pages=${miniSudokuPageCount}` });
    }
    console.log(`[print] mini-sudoku seed=A1b2: grids=${miniSudokuPrint.miniSudokuProblemGrids}+${miniSudokuPrint.miniSudokuAnswerGrids}, cells=${miniSudokuPrint.miniSudokuProblemCells}+${miniSudokuPrint.miniSudokuAnswerCells}, crossings=${miniSudokuPrint.crossings}, actual PDF bytes=${miniSudokuPdf.length}, pages=${miniSudokuPageCount}`);

    await navigate(cdp, `${origin}${BASE_PATH}/drills/grade-7/signed-arithmetic-1/`);
    const printResult = await cdp.evaluate(printPreviewProbe('A1b2'));
    if (printResult.generationFailed) throw new Error(`Signed print probe could not generate worksheet: ${JSON.stringify(printResult)}`);
    if (printResult.initialDisabled) failures.push({ route: 'signed-arithmetic-1', seed: 'A1b2', reason: 'print button was disabled before the user requested printing' });
    if (printResult.stacked !== 20 || printResult.equalsCount !== 0) failures.push({ route: 'signed-arithmetic-1', seed: 'A1b2', reason: `signed layout mismatch: stacked=${printResult.stacked}, equals=${printResult.equalsCount}` });
    if (printResult.crossings !== 0) failures.push({ route: 'signed-arithmetic-1', seed: 'A1b2', reason: `print preview has ${printResult.crossings} center-divider crossing(s)` });
    if (printResult.ready !== printResult.total) failures.push({ route: 'signed-arithmetic-1', seed: 'A1b2', reason: `native print was called with ${printResult.ready}/${printResult.total} MathLive spans ready; missing=${printResult.missing.join(' | ')}` });
    console.log(`[print] signed-arithmetic-1 seed=A1b2: MathLive ready ${printResult.ready}/${printResult.total}, stacked=${printResult.stacked}, equals=${printResult.equalsCount}, crossings=${printResult.crossings}`);

    await navigate(cdp, `${origin}${BASE_PATH}/drills/grade-4/column-decimal-add-subtract/`);
    const columnPrint = await cdp.evaluate(printPreviewProbe('A1b2'));
    if (columnPrint.generationFailed) throw new Error(`Column print probe could not generate worksheet: ${JSON.stringify(columnPrint)}`);
    if (columnPrint.columnCells !== 16 || columnPrint.columnExpressions !== 16 || columnPrint.dividerCount !== 0 || columnPrint.emptyAnswers !== 16 || columnPrint.completedSolutions !== 16 || columnPrint.visibleCompletedSolutions !== 16 || columnPrint.answerPageEmptyAnswers !== 0) {
      failures.push({ route: 'column-decimal-add-subtract', seed: 'A1b2', reason: `4x4 print structure mismatch: ${JSON.stringify(columnPrint)}` });
    }
    if (columnPrint.crossings !== 0) failures.push({ route: 'column-decimal-add-subtract', seed: 'A1b2', reason: `column print preview has ${columnPrint.crossings} cell crossing(s)` });
    if (columnPrint.ready !== columnPrint.total) failures.push({ route: 'column-decimal-add-subtract', seed: 'A1b2', reason: `column native print was called with ${columnPrint.ready}/${columnPrint.total} MathLive spans ready` });
    if (process.env.AUTODRILL_CAPTURE_SCREENSHOT) {
      const screenshot = await cdp.send('Page.captureScreenshot', { format: 'png', captureBeyondViewport: true });
      writeFileSync(process.env.AUTODRILL_CAPTURE_SCREENSHOT, Buffer.from(screenshot.data, 'base64'));
    }
    if (process.env.AUTODRILL_CAPTURE_ANSWER_SCREENSHOT) {
      await cdp.evaluate(`(() => { document.querySelector('[data-print-page=\"answers\"]')?.scrollIntoView({ block: 'start' }); return true; })()`);
      await new Promise((resolve) => setTimeout(resolve, 150));
      const screenshot = await cdp.send('Page.captureScreenshot', { format: 'png' });
      writeFileSync(process.env.AUTODRILL_CAPTURE_ANSWER_SCREENSHOT, Buffer.from(screenshot.data, 'base64'));
    }
    const printed = await cdp.send('Page.printToPDF', { printBackground: true, preferCSSPageSize: true });
    const pdf = Buffer.from(printed.data, 'base64');
    const pageCount = (pdf.toString('latin1').match(/\/Type\s*\/Page\b/g) ?? []).length;
    if (pdf.length < 10_000 || pdf.subarray(0, 4).toString('ascii') !== '%PDF' || pageCount !== 2) {
      failures.push({ route: 'column-decimal-add-subtract', seed: 'A1b2', reason: `actual Chrome PDF invalid: bytes=${pdf.length}, pages=${pageCount}` });
    }
    console.log(`[print] column-decimal-add-subtract seed=A1b2: 4x4 cells=${columnPrint.columnCells}, dividers=${columnPrint.dividerCount}, crossings=${columnPrint.crossings}, actual PDF bytes=${pdf.length}, pages=${pageCount}`);

    await navigate(cdp, `${origin}${BASE_PATH}/drills/grade-3/column-multiplication-two-digit/`);
    const multiplicationPrint = await cdp.evaluate(printPreviewProbe('A1b2'));
    if (multiplicationPrint.generationFailed) throw new Error(`Column multiplication print probe could not generate worksheet: ${JSON.stringify(multiplicationPrint)}`);
    if (multiplicationPrint.columnCells !== 16 || multiplicationPrint.columnExpressions !== 16 || multiplicationPrint.dividerCount !== 0 || multiplicationPrint.emptyAnswers !== 16 || multiplicationPrint.completedSolutions !== 16 || multiplicationPrint.visibleCompletedSolutions !== 16 || multiplicationPrint.multiplicationPartials !== 32 || multiplicationPrint.answerPageEmptyAnswers !== 0) {
      failures.push({ route: 'column-multiplication-two-digit', seed: 'A1b2', reason: `4x4 multiplication print structure mismatch: ${JSON.stringify(multiplicationPrint)}` });
    }
    if (multiplicationPrint.crossings !== 0) failures.push({ route: 'column-multiplication-two-digit', seed: 'A1b2', reason: `multiplication print preview has ${multiplicationPrint.crossings} cell crossing(s): ${JSON.stringify(multiplicationPrint.crossingDetails)}` });
    console.log(`[print] column-multiplication-two-digit seed=A1b2: partials=${multiplicationPrint.multiplicationPartials}, completed=${multiplicationPrint.completedSolutions}, crossings=${multiplicationPrint.crossings}`);

    await navigate(cdp, `${origin}${BASE_PATH}/drills/grade-4/column-division-two-digit/`);
    const divisionPrint = await cdp.evaluate(printPreviewProbe('A1b2'));
    if (divisionPrint.generationFailed) throw new Error(`Column division print probe could not generate worksheet: ${JSON.stringify(divisionPrint)}`);
    if (divisionPrint.columnCells !== 12 || divisionPrint.columnExpressions !== 12 || divisionPrint.dividerCount !== 0 || divisionPrint.emptyAnswers !== 12 || divisionPrint.completedSolutions !== 12 || divisionPrint.visibleCompletedSolutions !== 12 || divisionPrint.divisionSolutionSteps < 12 || divisionPrint.divisionMinusSigns !== 0 || divisionPrint.answerPageEmptyAnswers !== 0 || divisionPrint.answerCrossings !== 0 || JSON.stringify(divisionPrint.divisionAnswerFontSizes) !== JSON.stringify(divisionPrint.divisionProblemFontSizes)) {
      failures.push({ route: 'column-division-two-digit', seed: 'A1b2', reason: `4x3 division print structure mismatch: ${JSON.stringify(divisionPrint)}` });
    }
    if (divisionPrint.crossings !== 0) failures.push({ route: 'column-division-two-digit', seed: 'A1b2', reason: `division print preview has ${divisionPrint.crossings} cell crossing(s)` });
    if (divisionPrint.ready !== divisionPrint.total) failures.push({ route: 'column-division-two-digit', seed: 'A1b2', reason: `division native print was called with ${divisionPrint.ready}/${divisionPrint.total} MathLive spans ready` });
    if (process.env.AUTODRILL_CAPTURE_DIVISION_ANSWER_SCREENSHOT) {
      await cdp.evaluate(`(() => { document.querySelector('[data-print-page=\"answers\"]')?.scrollIntoView({ block: 'start' }); return true; })()`);
      await new Promise((resolve) => setTimeout(resolve, 150));
      const screenshot = await cdp.send('Page.captureScreenshot', { format: 'png' });
      writeFileSync(process.env.AUTODRILL_CAPTURE_DIVISION_ANSWER_SCREENSHOT, Buffer.from(screenshot.data, 'base64'));
    }
    const divisionPrinted = await cdp.send('Page.printToPDF', { printBackground: true, preferCSSPageSize: true });
    const divisionPdf = Buffer.from(divisionPrinted.data, 'base64');
    const divisionPageCount = (divisionPdf.toString('latin1').match(/\/Type\s*\/Page\b/g) ?? []).length;
    if (divisionPdf.length < 10_000 || divisionPdf.subarray(0, 4).toString('ascii') !== '%PDF' || divisionPageCount !== 2) {
      failures.push({ route: 'column-division-two-digit', seed: 'A1b2', reason: `actual division Chrome PDF invalid: bytes=${divisionPdf.length}, pages=${divisionPageCount}` });
    }
    console.log(`[print] column-division-two-digit seed=A1b2: 4x3 cells=${divisionPrint.columnCells}, dividers=${divisionPrint.dividerCount}, blankAnswerSlots=${divisionPrint.emptyAnswers}, solutionSteps=${divisionPrint.divisionSolutionSteps}, crossings=${divisionPrint.crossings}, actual PDF bytes=${divisionPdf.length}, pages=${divisionPageCount}`);

  }

  if (failures.length > 0) { console.error(JSON.stringify(failures, null, 2)); throw new Error(`Browser worksheet layout verification failed with ${failures.length} issue(s).`); }
  console.log(`Browser layout/interaction verified: ${worksheetSampleCount} worksheet samples, ${answerAffordanceActionCount} editable affordance actions, ${inputPanelActionCount} unique input-panel actions, and native print readiness all passed.`);
} finally {
  try { cdp?.ws.close(); } catch {}
  if (chrome.exitCode === null) {
    const exited = new Promise((resolveExit) => chrome.once('exit', resolveExit));
    chrome.kill('SIGTERM');
    await Promise.race([exited, new Promise((resolveWait) => setTimeout(resolveWait, 1000))]);
  }
  server.close();
  rmSync(userDataDir, { recursive: true, force: true, maxRetries: 5, retryDelay: 100 });
}
