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
const SEEDS = ['A1b2', 'M7x9'];
const EXTRA_SIGNED_SEEDS = ['Q4r6', 'Z8k3'];
const VIEWPORT = { width: 1600, height: 800, deviceScaleFactor: 1, mobile: false };
const PRINT_ONLY = process.env.AUTODRILL_PRINT_ONLY === 'true';
const ROUTE_FILTER = process.env.AUTODRILL_ROUTE_FILTER ?? '';


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
    const cells = [...paper.querySelectorAll('.problem-cell')];
    const crossings = [];
    const columnGridMismatches = [];
    for (const cell of cells) {
      const column = Number(cell.dataset.layoutColumn);
      // Check every cell boundary rather than a single center divider so 4-column
      // printable themes get the same clipping guarantees as legacy 2-column themes.
      const rects = [...cell.querySelectorAll('.problem-number, math-span.problem-math-expression, .column-arithmetic, .liar-statements, .problem-answer-area')]
        .map((element) => element.getBoundingClientRect())
        .filter((rect) => rect.width > 0 && rect.height > 0);
      if (rects.length === 0) continue;
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
          promptAria: cell.querySelector('.column-arithmetic')?.getAttribute('aria-label') ?? null,
          cellWidth: Math.round(cellRect.width * 10) / 10,
          expressionWidth: Math.round((cell.querySelector('.column-arithmetic')?.getBoundingClientRect().width ?? 0) * 10) / 10,
          operatorWidth: getComputedStyle(cell).getPropertyValue('--column-operator-width').trim(),
          digitWidth: getComputedStyle(cell).getPropertyValue('--column-digit-width').trim(),
          laneRightOffset: getComputedStyle(cell).getPropertyValue('--column-lane-right-offset').trim(),
        });
      }
      if (cell.classList.contains('problem-cell-column-arithmetic')) {
        const divide = cell.classList.contains('problem-cell-column-arithmetic-divide');
        const reference = divide
          ? cell.querySelector('.column-division-bracket')?.getBoundingClientRect()
          : (cell.querySelector('.column-arithmetic-final-rule') ?? cell.querySelector('.column-arithmetic-rule'))?.getBoundingClientRect();
        const answer = divide
          ? cell.querySelector('.column-division-answer-coordinate-quotient .answer-box')?.getBoundingClientRect()
          : cell.querySelector('.problem-answer-area .answer-box')?.getBoundingClientRect();
        if (reference && answer) {
          const leftDelta = Math.abs(reference.left - answer.left);
          const rightDelta = Math.abs(reference.right - answer.right);
          if (leftDelta > 1 || rightDelta > 1) {
            columnGridMismatches.push({
              problem: Number(cell.dataset.problemIndex) + 1,
              leftDelta: Math.round(leftDelta * 10) / 10,
              rightDelta: Math.round(rightDelta * 10) / 10,
            });
          }
        }
      }
    }
    const gradeClass = [...paper.classList].find((name) => name.startsWith('worksheet-grade-')) ?? null;
    const fontSize = getComputedStyle(paper.querySelector('.expression')).fontSize;
    return { crossings, columnGridMismatches, count: cells.length, gradeClass, fontSize, alert: document.querySelector('[role="alert"]')?.getAttribute('aria-label') ?? null };
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


function columnAdditionInputProbe() {
  return `(async () => {
    const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
    const waitFor = async (fn, label) => {
      for (let i = 0; i < 300; i += 1) { const value = fn(); if (value) return value; await sleep(25); }
      throw new Error('Timed out waiting for ' + label);
    };
    const fields = await waitFor(() => {
      const values = [...document.querySelectorAll('math-field.answer-mathfield')];
      return values.length === 16 ? values : null;
    }, '16 column addition answer fields');
    const field = fields[0];
    field.click();
    const panel = await waitFor(() => document.querySelector('.input-panel'), 'column addition input panel');
    for (const digit of ['1', '6', '4']) {
      [...panel.querySelectorAll('.keypad-numbers button')].find((button) => button.textContent?.trim() === digit)?.click();
      await sleep(60);
    }
    const cell = field.closest('.problem-cell-column-arithmetic');
    const rule = cell.querySelector('.column-arithmetic-rule').getBoundingClientRect();
    const frame = field.closest('.answer-box').getBoundingClientRect();
    const expression = cell.querySelector('.column-arithmetic');
    const content = field.shadowRoot?.querySelector('[part~="content"]');
    const numericGlyph = field.shadowRoot?.querySelector('.ML__cmr');
    return {
      fieldCount: fields.length,
      value: field.value,
      gap: Math.round((frame.top - rule.bottom) * 10) / 10,
      leftDelta: Math.round((frame.left - rule.left) * 10) / 10,
      rightDelta: Math.round((frame.right - rule.right) * 10) / 10,
      expressionFont: getComputedStyle(expression).fontFamily,
      fieldFont: getComputedStyle(field).fontFamily,
      contentFont: content ? getComputedStyle(content).fontFamily : null,
      glyphFont: numericGlyph ? getComputedStyle(numericGlyph).fontFamily : null,
      expressionFontSize: getComputedStyle(expression).fontSize,
      fieldFontSize: getComputedStyle(field).fontSize,
    };
  })()`;
}

function columnDivisionInputProbe() {
  return `(async () => {
    const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
    const waitFor = async (fn, label) => {
      for (let i = 0; i < 300; i += 1) { const value = fn(); if (value) return value; await sleep(25); }
      throw new Error('Timed out waiting for ' + label);
    };
    const fields = await waitFor(() => {
      const values = [...document.querySelectorAll('math-field.answer-mathfield')];
      return values.length === 24 ? values : null;
    }, '24 quotient/remainder fields');
    const quotient = fields[0];
    const remainder = fields[1];
    const labels = [quotient, remainder].map((field) => ({
      aria: field.getAttribute('aria-label'),
      label: field.closest('.column-division-answer-coordinate')?.querySelector('.column-division-answer-label')?.textContent?.trim() ?? null,
    }));
    quotient.click();
    const panel = await waitFor(() => document.querySelector('.input-panel'), 'column division input panel');
    const formulaLabels = [...panel.querySelectorAll('.formula-keypad button')].map((button) => button.getAttribute('aria-label'));
    const digit2 = [...panel.querySelectorAll('.keypad-numbers button')].find((button) => button.textContent?.trim() === '2');
    digit2?.click();
    await sleep(80);
    remainder.click();
    const digit3 = [...panel.querySelectorAll('.keypad-numbers button')].find((button) => button.textContent?.trim() === '3');
    digit3?.click();
    await sleep(80);
    return {
      fieldCount: fields.length,
      labels,
      formulaLabels,
      quotientValue: quotient.value,
      remainderValue: remainder.value,
      notice: document.querySelector('.worksheet-toast')?.getAttribute('aria-label') ?? null,
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
    const cells = [...problemPage.querySelectorAll('.problem-cell')];
    const crossingDetails = cells.flatMap((cell) => {
      const cellRect = cell.getBoundingClientRect();
      const entries = [...cell.querySelectorAll('.problem-number, math-span.problem-math-expression, .column-arithmetic, .problem-answer-area')]
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
        laneRightOffset: getComputedStyle(cell).getPropertyValue('--column-lane-right-offset').trim(),
      }];
    });
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
    const multiplicationPartials = answerPage?.querySelectorAll('.column-multiply-partial').length ?? 0;
    const answerPageEmptyAnswers = answerPage?.querySelectorAll('.worksheet-print-empty-answer').length ?? 0;
    const answerCrossingDetails = [...(answerPage?.querySelectorAll('.problem-cell-column-arithmetic') ?? [])].flatMap((cell) => {
      const cellRect = cell.getBoundingClientRect();
      const solution = cell.querySelector('[data-column-solution=\"true\"]');
      if (!solution) return [];
      const rect = solution.getBoundingClientRect();
      const leftOverflow = Math.max(0, cellRect.left - rect.left);
      const rightOverflow = Math.max(0, rect.right - cellRect.right);
      const topOverflow = Math.max(0, cellRect.top - rect.top);
      const bottomOverflow = Math.max(0, rect.bottom - cellRect.bottom);
      if (Math.max(leftOverflow, rightOverflow, topOverflow, bottomOverflow) <= 1) return [];
      return [{
        problem: Number(cell.dataset.printProblemIndex ?? 0) + 1,
        leftOverflow: Math.round(leftOverflow * 10) / 10,
        rightOverflow: Math.round(rightOverflow * 10) / 10,
        topOverflow: Math.round(topOverflow * 10) / 10,
        bottomOverflow: Math.round(bottomOverflow * 10) / 10,
      }];
    });
    const answerCrossings = answerCrossingDetails.length;
    const divisionProblemFontSizes = [...new Set([...problemPage.querySelectorAll('.problem-cell-column-arithmetic-divide .expression')].map((expression) => getComputedStyle(expression).fontSize))];
    const divisionAnswerFontSizes = [...new Set([...(answerPage?.querySelectorAll('.column-division-solution') ?? [])].map((solution) => getComputedStyle(solution).fontSize))];
    printButton.click();
    const result = await waitFor(() => window.__AUTODRILL_PRINT_PROBE__, 'native print callback', 320);
    return { ...result, initialDisabled, stacked: stacked.length, equalsCount, crossings, crossingDetails, answerCrossings, answerCrossingDetails, dividerCount, columnCells, columnExpressions, emptyAnswers, completedSolutions, visibleCompletedSolutions, divisionSolutionSteps, divisionProblemFontSizes, divisionAnswerFontSizes, multiplicationPartials, answerPageEmptyAnswers };
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
    .filter((pathname) => pathname !== '/')
    .filter((pathname) => !ROUTE_FILTER || pathname.includes(ROUTE_FILTER));
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
        for (const mismatch of result.columnGridMismatches ?? []) {
          console.warn(`[layout] column grid mismatch: ${route} seed=${seed} problem=${mismatch.problem} left=${mismatch.leftDelta}px right=${mismatch.rightDelta}px`);
          failures.push({ route, seed, reason: `column arithmetic answer is not on the shared digit grid`, ...mismatch });
        }
        if (cdp.consoleErrors.length > 0) failures.push({ route, seed, reason: `console errors: ${cdp.consoleErrors.join(' | ')}` });
        console.log(`[layout] ${route} seed=${seed}: ${result.count} problems, ${result.gradeClass}, expression ${result.fontSize}, crossings=${result.crossings.length}, gridMismatches=${result.columnGridMismatches?.length ?? 0}`);
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
        if (route.endsWith('/column-addition-two-digit') && seed === SEEDS[0]) {
          const input = await cdp.evaluate(columnAdditionInputProbe());
          if (input.value !== '164' || input.fieldCount !== 16 || input.gap < 2 || input.gap > 12 || Math.abs(input.leftDelta) > 1 || Math.abs(input.rightDelta) > 1 || input.expressionFontSize !== input.fieldFontSize || !input.glyphFont?.includes('Noto Sans JP')) {
            failures.push({ route, seed, reason: `column addition answer alignment mismatch: ${JSON.stringify(input)}` });
          }
          if (process.env.AUTODRILL_CAPTURE_COLUMN_INPUT_SCREENSHOT) {
            const screenshot = await cdp.send('Page.captureScreenshot', { format: 'png', captureBeyondViewport: true });
            writeFileSync(process.env.AUTODRILL_CAPTURE_COLUMN_INPUT_SCREENSHOT, Buffer.from(screenshot.data, 'base64'));
          }
          console.log(`[input] column-addition-two-digit: ${JSON.stringify(input)}`);
        }
        if (route.endsWith('/column-division-one-digit') && seed === SEEDS[0]) {
          const input = await cdp.evaluate(columnDivisionInputProbe());
          if (process.env.AUTODRILL_CAPTURE_DIVISION_INPUT_SCREENSHOT) {
            const screenshot = await cdp.send('Page.captureScreenshot', { format: 'png', captureBeyondViewport: true });
            writeFileSync(process.env.AUTODRILL_CAPTURE_DIVISION_INPUT_SCREENSHOT, Buffer.from(screenshot.data, 'base64'));
          }
          if (
            input.fieldCount !== 24
            || JSON.stringify(input.labels.map((item) => item.label)) !== JSON.stringify(['商', 'あまり'])
            || input.quotientValue !== '2'
            || input.remainderValue !== '3'
            || input.formulaLabels.length !== 0
          ) {
            failures.push({ route, seed, reason: `column division final-answer input mismatch: ${JSON.stringify(input)}` });
          }
          if (input.notice === '式が大きすぎます！') {
            failures.push({ route, seed, reason: `column division coordinate input was rejected as too large: ${JSON.stringify(input)}` });
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
  if (divisionPrint.columnCells !== 12 || divisionPrint.columnExpressions !== 12 || divisionPrint.dividerCount !== 0 || divisionPrint.emptyAnswers !== 12 || divisionPrint.completedSolutions !== 12 || divisionPrint.visibleCompletedSolutions !== 12 || divisionPrint.divisionSolutionSteps < 12 || divisionPrint.answerPageEmptyAnswers !== 0 || divisionPrint.answerCrossings !== 0 || JSON.stringify(divisionPrint.divisionAnswerFontSizes) !== JSON.stringify(divisionPrint.divisionProblemFontSizes)) {
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

  if (failures.length > 0) { console.error(JSON.stringify(failures, null, 2)); throw new Error(`Browser worksheet layout verification failed with ${failures.length} issue(s).`); }
  console.log(`Browser layout verified: dropdowns selectable, ${worksheetSampleCount} worksheet samples stay within their cell boundaries, and native print waits for stable MathLive rendering.`);
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
