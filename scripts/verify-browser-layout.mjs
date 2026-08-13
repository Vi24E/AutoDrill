#!/usr/bin/env node
import { spawn, spawnSync } from 'node:child_process';
import { createServer } from 'node:http';
import { existsSync, mkdtempSync, readFileSync, rmSync, statSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { extname, join, normalize, resolve } from 'node:path';
import net from 'node:net';

const ROOT = resolve(import.meta.dirname, '..');
const OUT = join(ROOT, 'apps/web/out');
const BASE_PATH = '/AutoDrill';
const SEEDS = ['A1b2', 'M7x9'];
const EXTRA_SIGNED_SEEDS = ['Q4r6', 'Z8k3'];
const VIEWPORT = { width: 1600, height: 800, deviceScaleFactor: 1, mobile: false };
const PRINT_ONLY = process.env.AUTODRILL_PRINT_ONLY === 'true';

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

function worksheetProbe(seed) {
  return `(async () => {
    const seed = ${JSON.stringify(seed)};
    const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
    const waitFor = async (fn, label) => {
      for (let i = 0; i < 400; i += 1) { const value = fn(); if (value) return value; await sleep(25); }
      throw new Error('Timed out waiting for ' + label);
    };
    await waitFor(() => window.__AUTODRILL_WASM__, 'WASM');
    const difficulty = await waitFor(() => document.querySelector('button[aria-label=\"難易度\"]'), 'difficulty');
    difficulty.click();
    const hard = await waitFor(() => [...document.querySelectorAll('[role=\"option\"]')].find((option) => option.getAttribute('aria-label') === 'むずかしい'), 'hard difficulty');
    hard.click();
    await new Promise(requestAnimationFrame);
    await new Promise(requestAnimationFrame);
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
    await document.fonts.ready;
    await new Promise(requestAnimationFrame);
    await new Promise(requestAnimationFrame);
    const divider = paper.querySelector('.problem-divider')?.getBoundingClientRect() ?? null;
    const cells = [...paper.querySelectorAll('.problem-cell')];
    const crossings = [];
    for (const cell of cells) {
      const column = Number(cell.dataset.layoutColumn);
      // Measure the layout boxes that own visible worksheet content. MathLive's
      // internal shadow parts use their own coordinate space in some browser
      // versions, so measuring those directly creates false divider crossings.
      // The expression/answer hosts expand with the rendered MathLive content.
      const rects = [...cell.querySelectorAll('.problem-number, math-span.problem-math-expression, .liar-statements, .problem-answer-area')]
        .map((element) => element.getBoundingClientRect())
        .filter((rect) => rect.width > 0 && rect.height > 0);
      if (rects.length === 0) continue;
      const minLeft = Math.min(...rects.map((rect) => rect.left));
      const maxRight = Math.max(...rects.map((rect) => rect.right));
      const overflow = divider ? (column === 0 ? maxRight - divider.left : divider.right - minLeft) : Math.max(cell.getBoundingClientRect().left - minLeft, maxRight - cell.getBoundingClientRect().right);
      if (overflow > 1) {
        crossings.push({
          problem: Number(cell.dataset.problemIndex) + 1,
          column,
          overflow: Math.round(overflow * 10) / 10,
          expression: cell.querySelector('math-span')?.getAttribute('aria-label') ?? cell.querySelector('.expression')?.textContent ?? '',
        });
      }
    }
    const gradeClass = [...paper.classList].find((name) => name.startsWith('worksheet-grade-')) ?? null;
    const fontSize = getComputedStyle(paper.querySelector('.expression')).fontSize;
    return { crossings, count: cells.length, gradeClass, fontSize, alert: document.querySelector('[role="alert"]')?.getAttribute('aria-label') ?? null };
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
    const descriptions = [...dialog.querySelectorAll('.grading-setting-copy > span')].map((element) => element.textContent?.trim());
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

function algebraKeypadProbe() {
  return `(async () => {
    const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
    const waitFor = async (fn, label) => {
      for (let i = 0; i < 400; i += 1) { const value = fn(); if (value) return value; await sleep(25); }
      throw new Error('Timed out waiting for ' + label);
    };
    const field = await waitFor(() => document.querySelector('math-field.answer-mathfield'), 'quadratic answer field');
    field.click();
    const panel = await waitFor(() => document.querySelector('.input-panel'), 'quadratic input panel');
    const keys = [...panel.querySelectorAll('.formula-keypad-algebraic button')].map((button) => {
      const rect = button.getBoundingClientRect();
      return {
        label: button.getAttribute('aria-label'),
        text: button.textContent?.trim() ?? '',
        top: rect.top,
        left: rect.left,
        width: rect.width,
        height: rect.height,
      };
    });
    const groupTops = [...panel.querySelectorAll('.input-panel-inner-algebraic > *')].map((element) => element.getBoundingClientRect().top);
    const rowTops = [...new Set(keys.map((item) => Math.round(item.top * 10) / 10))].sort((a, b) => a - b);
    const rowCounts = rowTops.map((top) => keys.filter((item) => Math.abs(item.top - top) < 1).length);

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
    const containmentSample = () => {
      const frame = field.closest('.answer-box').getBoundingClientRect();
      const painted = paintedBounds();
      const containsPaint = painted
        ? painted.left >= frame.left - 1 && painted.right <= frame.right + 1
          && painted.top >= frame.top - 1 && painted.bottom <= frame.bottom + 1
        : true;
      return { rendered: Boolean(painted), containsPaint, frame: { width: frame.width, height: frame.height }, painted };
    };
    for (const latex of ['99999999', '\\sqrt{2}', '1,2', '\\pm\\sqrt{2}', '\\sqrt{57\\pm\\sqrt{99}}{42}']) {
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
    return {
      keys,
      rowTops,
      rowCounts,
      maxKeyHeightDelta: Math.max(0, ...keys.map((item) => item.height)) - Math.min(...keys.map((item) => item.height)),
      maxGroupTopDelta: Math.max(0, ...groupTops) - Math.min(...groupTops),
      inputResults,
    };
  })()`;
}

function printPreviewProbe(seed) {
  return `(async () => {
    const seed = ${JSON.stringify(seed)};
    const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
    const waitFor = async (fn, label, attempts = 800) => {
      for (let i = 0; i < attempts; i += 1) { const value = fn(); if (value) return value; await sleep(25); }
      throw new Error('Timed out waiting for ' + label);
    };
    await waitFor(() => window.__AUTODRILL_WASM__, 'WASM');
    await waitFor(() => document.querySelector('button[aria-label="難易度"]'), 'difficulty');
    await new Promise(requestAnimationFrame);
    await new Promise(requestAnimationFrame);
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
    const divider = problemPage.querySelector('.problem-divider').getBoundingClientRect();
    const crossings = [...problemPage.querySelectorAll('.problem-cell')].filter((cell) => {
      const index = Number(cell.dataset.printProblemIndex ?? 0);
      const column = index >= 10 ? 1 : 0;
      const rects = [...cell.querySelectorAll('.problem-number, math-span.problem-math-expression, .problem-answer-area')]
        .map((element) => element.getBoundingClientRect())
        .filter((rect) => rect.width > 0 && rect.height > 0);
      if (rects.length === 0) return false;
      const minLeft = Math.min(...rects.map((rect) => rect.left));
      const maxRight = Math.max(...rects.map((rect) => rect.right));
      return column === 0 ? maxRight > divider.left + 1 : minLeft < divider.right - 1;
    }).length;
    printButton.click();
    const result = await waitFor(() => window.__AUTODRILL_PRINT_PROBE__, 'native print callback', 320);
    return { ...result, initialDisabled, stacked: stacked.length, equalsCount, crossings };
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
      '例: 2/4 と 1/2 の表記を区別します。',
      '例: √16 と 4 の表記を区別します。',
      '例: 0.5 と 1/2 の表記を区別します。',
      '上の3項目以外の、数学的に同値だが未整理・冗長な表記の違いを区別します。',
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
  }

  const sitemap = readFileSync(join(OUT, 'sitemap.xml'), 'utf8');
  const routes = [...sitemap.matchAll(/<loc>[^<]+\/AutoDrill([^<]*)<\/loc>/g)]
    .map((match) => match[1] || '/')
    .filter((pathname) => pathname !== '/');
  const failures = [];
  let worksheetSampleCount = 0;
  if (!PRINT_ONLY) {
    for (const route of routes) {
      const routeSeeds = route.includes('/signed-arithmetic-') ? [...SEEDS, ...EXTRA_SIGNED_SEEDS] : SEEDS;
      for (const seed of routeSeeds) {
        worksheetSampleCount += 1;
        const localRoute = route.endsWith('/') ? route : `${route}/`;
        console.log(`[layout] checking ${route} seed=${seed}`);
        await navigate(cdp, `${origin}${BASE_PATH}${localRoute}`);
        let result;
        try {
          result = await cdp.evaluate(worksheetProbe(seed));
        } catch (error) {
          throw new Error(`Worksheet probe failed for ${route} seed=${seed}; console=${cdp.consoleErrors.join(' | ') || 'none'}`, { cause: error });
        }
        if (result.alert) failures.push({ route, seed, reason: `UI alert: ${result.alert}` });
        for (const crossing of result.crossings) {
          console.warn(`[layout] divider crossing: ${route} seed=${seed} problem=${crossing.problem} overflow=${crossing.overflow}px expression=${crossing.expression}`);
          failures.push({ route, seed, ...crossing });
        }
        if (cdp.consoleErrors.length > 0) failures.push({ route, seed, reason: `console errors: ${cdp.consoleErrors.join(' | ')}` });
        console.log(`[layout] ${route} seed=${seed}: ${result.count} problems, ${result.gradeClass}, expression ${result.fontSize}, crossings=${result.crossings.length}`);
        if (route.endsWith('/simultaneous-equation-1') && seed === SEEDS[0]) {
          const input = await cdp.evaluate(simultaneousInputProbe());
          if (
            input.fieldCount !== 24
            || JSON.stringify(input.coordinates.map((item) => item.prefix)) !== JSON.stringify(['x =', 'y ='])
            || JSON.stringify(input.labels) !== JSON.stringify(['マイナス'])
            || input.xValue !== '2'
            || input.yValue !== '3'
          ) {
            failures.push({ route, seed, reason: `simultaneous separate-coordinate input mismatch: ${JSON.stringify(input)}` });
          }
          if (input.notice === '式が大きすぎます！') {
            failures.push({ route, seed, reason: `simultaneous coordinate input was rejected as too large: ${JSON.stringify(input)}` });
          }
        }
        if (route.endsWith('/quadratic-equation-1') && seed === SEEDS[0]) {
          const keypad = await cdp.evaluate(algebraKeypadProbe());
          const expectedLabels = ['分数', '平方根', '複数解', 'プラスを挿入', 'マイナスを挿入', 'プラスマイナスを挿入'];
          const expectedTexts = ['+', '−', '±'];
          if (JSON.stringify(keypad.keys.map((item) => item.label)) !== JSON.stringify(expectedLabels)) {
            failures.push({ route, seed, reason: `quadratic key order mismatch: ${JSON.stringify(keypad.keys)}` });
          }
          if (JSON.stringify(keypad.keys.slice(-3).map((item) => item.text)) !== JSON.stringify(expectedTexts)) {
            failures.push({ route, seed, reason: `quadratic operators mismatch: ${JSON.stringify(keypad.keys)}` });
          }
          if (JSON.stringify(keypad.rowCounts) !== JSON.stringify([3, 3])) {
            failures.push({ route, seed, reason: `quadratic formula keys must be exactly 2 rows x 3 columns: rows=${JSON.stringify(keypad.rowCounts)}` });
          }
          if (keypad.maxKeyHeightDelta > 1) {
            failures.push({ route, seed, reason: `quadratic formula keys have inconsistent heights: delta=${keypad.maxKeyHeightDelta}px` });
          }
          if (keypad.maxGroupTopDelta > 1) {
            failures.push({ route, seed, reason: `quadratic keypad groups wrapped vertically: top delta=${keypad.maxGroupTopDelta}px` });
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
        }
      }
    }
  }
  await navigate(cdp, `${origin}${BASE_PATH}/drills/grade-7/signed-arithmetic-1/`);
  const printResult = await cdp.evaluate(printPreviewProbe('A1b2'));
  if (printResult.generationFailed) throw new Error(`Signed print probe could not generate worksheet: ${JSON.stringify(printResult)}`);
  if (printResult.initialDisabled) failures.push({ route: 'signed-arithmetic-1', seed: 'A1b2', reason: 'print button was disabled before the user requested printing' });
  if (printResult.stacked !== 20 || printResult.equalsCount !== 0) failures.push({ route: 'signed-arithmetic-1', seed: 'A1b2', reason: `signed layout mismatch: stacked=${printResult.stacked}, equals=${printResult.equalsCount}` });
  if (printResult.crossings !== 0) failures.push({ route: 'signed-arithmetic-1', seed: 'A1b2', reason: `print preview has ${printResult.crossings} center-divider crossing(s)` });
  if (printResult.ready !== printResult.total) failures.push({ route: 'signed-arithmetic-1', seed: 'A1b2', reason: `native print was called with ${printResult.ready}/${printResult.total} MathLive spans ready; missing=${printResult.missing.join(' | ')}` });
  console.log(`[print] signed-arithmetic-1 seed=A1b2: MathLive ready ${printResult.ready}/${printResult.total}, stacked=${printResult.stacked}, equals=${printResult.equalsCount}, crossings=${printResult.crossings}`);

  if (failures.length > 0) { console.error(JSON.stringify(failures, null, 2)); throw new Error(`Browser worksheet layout verification failed with ${failures.length} issue(s).`); }
  console.log(`Browser layout verified: dropdowns selectable, ${worksheetSampleCount} worksheet samples do not cross the center divider, and native print waits for stable MathLive rendering.`);
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
