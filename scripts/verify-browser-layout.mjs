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
      const rects = [...cell.querySelectorAll('.problem-number, math-span.problem-math-expression, .column-arithmetic, .liar-statements, .mini-sudoku-grid, .problem-answer-area')]
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
          ? cell.querySelector('.column-division-answer-coordinate-quotient .column-digit-answer')?.getBoundingClientRect()
          : cell.querySelector('.column-answer-user .column-digit-answer')?.getBoundingClientRect();
        if (reference && answer) {
          const leftDelta = Math.abs(reference.right - answer.right);
          const laneLeft = cell.querySelector('.column-arithmetic')?.getBoundingClientRect().left ?? reference.left;
          const answerLaneLeft = answer.right - answer.width;
          const horizontalDelta = Math.max(leftDelta, Math.max(0, laneLeft - answerLaneLeft));
          let verticalDelta = 0;
          let expectedTop = null;
          if (!divide) {
            const gridCell = cell.querySelector('.column-arithmetic-digit-cell')?.getBoundingClientRect().width ?? 0;
            const workingRows = Number.parseInt(getComputedStyle(cell).getPropertyValue('--column-working-rows').trim() || '0', 10);
            expectedTop = reference.bottom + workingRows * gridCell;
            verticalDelta = Math.abs(answer.top - expectedTop);
          }
          if (horizontalDelta > 1 || verticalDelta > 1) {
            columnGridMismatches.push({
              problem: Number(cell.dataset.problemIndex) + 1,
              horizontalDelta: Math.round(horizontalDelta * 10) / 10,
              verticalDelta: Math.round(verticalDelta * 10) / 10,
              expectedTop: expectedTop === null ? null : Math.round(expectedTop * 10) / 10,
              actualTop: Math.round(answer.top * 10) / 10,
            });
          }
        }
      }
    }
    const gradeClass = [...paper.classList].find((name) => name.startsWith('worksheet-grade-')) ?? null;
    const expression = paper.querySelector('.expression');
    const fontSize = expression ? getComputedStyle(expression).fontSize : null;
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
    const blockRight = first.querySelector('.digit-grid-cell-block-right');
    const blockBottom = first.querySelector('.digit-grid-cell-block-bottom');
    const blockRightWidth = blockRight ? parseFloat(getComputedStyle(blockRight).borderRightWidth) : 0;
    const blockBottomWidth = blockBottom ? parseFloat(getComputedStyle(blockBottom).borderBottomWidth) : 0;
    return {
      gridCount: grids.length,
      counts,
      numberKeys,
      selectedIndex,
      written,
      maxSquareDelta,
      gridWidthDelta: Math.abs(gridRect.width - 4 * cellSize),
      blockRightWidth,
      blockBottomWidth,
      panelDigitGrid: panel.classList.contains('input-panel-digit-grid'),
    };
  })()`;
}

function columnAdditionInputProbe() {
  return `(async () => {
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

    document.querySelector('button[aria-label="採点"]')?.click();
    const feedback = await waitFor(() => {
      const values = [...document.querySelectorAll('.column-grade-feedback')];
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
      firstMark: feedback[0]?.querySelector('.column-grade-mark')?.textContent?.trim() ?? null,
      correctionInGrid,
      correctionValue,
      separateCorrectAnswerCount,
      correctionColor,
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

function columnDivisionInputProbe() {
  return `(async () => {
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
    const firstCell = document.querySelector('.problem-cell-column-arithmetic-divide');
    const quotient = quotients[0];
    const remainder = remainders[0];
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
    const bracket = firstCell.querySelector('.column-division-bracket');
    const bracketPath = bracket?.querySelector('.column-division-bracket-mark path')?.getAttribute('d') ?? null;
    document.querySelector('button[aria-label="採点"]')?.click();
    await waitFor(() => document.querySelectorAll('.column-grade-feedback').length === 12, 'division grading feedback');
    const gradedFirstCell = document.querySelector('.problem-cell-column-arithmetic-divide');
    const gradedQuotient = gradedFirstCell.querySelector('.column-division-answer-coordinate-quotient .column-digit-answer-quotient');
    const gradedRemainder = gradedFirstCell.querySelector('.column-division-answer-coordinate-remainder math-field.answer-mathfield');
    const quotientCorrection = gradedFirstCell.querySelector('.column-division-correction-quotient .column-digit-answer-correction');
    const remainderCorrection = gradedFirstCell.querySelector('.column-division-correction-remainder .column-remainder-correction-value');
    const gradedQuotientValue = gradedQuotient
      ? [...gradedQuotient.querySelectorAll('.column-digit-slot-active')].map((slot) => slot.textContent?.trim() ?? '').join('').replace(/^0+(?=[0-9])/, '')
      : null;
    const quotientCorrectionValue = quotientCorrection
      ? [...quotientCorrection.querySelectorAll('.column-digit-slot-active')].map((slot) => slot.textContent?.trim() ?? '').join('').replace(/^0+(?=[0-9])/, '')
      : null;
    return {
      quotientCount: quotients.length,
      remainderCount: remainders.length,
      quotientLabel,
      remainderLabel,
      formulaLabels,
      quotientValue,
      remainderValue,
      autoMovedToRemainder,
      quotientDirection: quotient.getAttribute('data-column-direction'),
      remainderUsesDigitSlots: Boolean(firstCell.querySelector('.column-digit-answer-remainder')),
      bracketBorderTop: bracket ? getComputedStyle(bracket).borderTopWidth : null,
      bracketPath,
      gradedQuotientValue,
      gradedRemainderValue: gradedRemainder?.value ?? null,
      quotientCorrectionInGrid: Boolean(quotientCorrection),
      quotientCorrectionValue,
      quotientCorrectionColor: quotientCorrection?.querySelector('.column-digit-glyph') ? getComputedStyle(quotientCorrection.querySelector('.column-digit-glyph')).color : null,
      remainderCorrectionInGrid: Boolean(remainderCorrection),
      remainderCorrectionValue: remainderCorrection?.textContent?.trim() ?? null,
      remainderCorrectionColor: remainderCorrection ? getComputedStyle(remainderCorrection).color : null,
      separateCorrectAnswerCount: document.querySelectorAll('.column-grade-correct-answer').length,
      notice: document.querySelector('.worksheet-toast')?.getAttribute('aria-label') ?? null,
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
      const entries = [...cell.querySelectorAll('.problem-number, math-span.problem-math-expression, .column-arithmetic, .mini-sudoku-grid, .problem-answer-area')]
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
    const divisionMinusSigns = answerPage?.querySelectorAll('.column-division-solution-minus').length ?? 0;
    const multiplicationPartials = answerPage?.querySelectorAll('.column-multiply-partial').length ?? 0;
    const answerPageEmptyAnswers = answerPage?.querySelectorAll('.worksheet-print-empty-answer').length ?? 0;
    const miniSudokuProblemGrids = problemPage.querySelectorAll('.mini-sudoku-grid').length;
    const miniSudokuAnswerGrids = answerPage?.querySelectorAll('.mini-sudoku-grid').length ?? 0;
    const miniSudokuProblemCells = problemPage.querySelectorAll('.mini-sudoku-grid [data-digit-grid-cell]').length;
    const miniSudokuAnswerCells = answerPage?.querySelectorAll('.mini-sudoku-grid [data-digit-grid-cell]').length ?? 0;
    const miniSudokuPrintButtons = preview.querySelectorAll('.mini-sudoku-grid button').length;
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
          console.warn(`[layout] column grid mismatch: ${route} seed=${seed} problem=${mismatch.problem} horizontal=${mismatch.horizontalDelta}px vertical=${mismatch.verticalDelta}px`);
          failures.push({ route, seed, reason: `column arithmetic answer is not on the shared digit grid`, ...mismatch });
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
            || input.blockRightWidth < 1.5
            || input.blockBottomWidth < 1.5
            || !input.panelDigitGrid
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
            || !['○', '×'].includes(input.firstMark)
            || input.gradedValue !== input.value
            || input.separateCorrectAnswerCount !== 0
            || (input.firstMark === '×' && (!input.correctionInGrid || !input.correctionValue || input.correctionColor !== 'rgb(210, 11, 11)'))
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
        if (route.endsWith('/column-division-one-digit') && seed === SEEDS[0]) {
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
      }
    }
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
