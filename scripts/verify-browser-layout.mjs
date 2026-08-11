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
const SEEDS = ['A1b2', 'M7x9', 'Q4r6', 'Z8k3'];
const VIEWPORT = { width: 1600, height: 800, deviceScaleFactor: 1, mobile: false };

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
      for (let i = 0; i < 200; i += 1) { const value = fn(); if (value) return value; await sleep(25); }
      throw new Error('Timed out waiting for ' + label);
    };
    await waitFor(() => window.__AUTODRILL_WASM__, 'WASM');
    const difficulty = await waitFor(() => document.querySelector('button[aria-label="難易度"]'), 'difficulty');
    difficulty.click();
    const hard = await waitFor(() => [...document.querySelectorAll('[role="option"]')].find((option) => option.getAttribute('aria-label') === '5: とてもむずかしい'), 'difficulty 5');
    hard.click();
    const seedInput = document.querySelector('input[aria-label="Seed"]');
    const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set;
    setter.call(seedInput, seed);
    seedInput.dispatchEvent(new Event('input', { bubbles: true }));
    await sleep(10);
    document.querySelector('button[aria-label="問題生成"]').click();
    const paper = await waitFor(() => document.querySelector('.worksheet-screen .paper'), 'worksheet');
    await document.fonts.ready;
    await new Promise(requestAnimationFrame);
    await new Promise(requestAnimationFrame);
    const divider = paper.querySelector('.problem-divider').getBoundingClientRect();
    const cells = [...paper.querySelectorAll('.problem-cell')];
    const crossings = [];
    for (const cell of cells) {
      const column = Number(cell.dataset.layoutColumn);
      // Measure the layout boxes that own visible worksheet content. MathLive's
      // internal shadow parts use their own coordinate space in some browser
      // versions, so measuring those directly creates false divider crossings.
      // The expression/answer hosts expand with the rendered MathLive content.
      const rects = [...cell.querySelectorAll('.problem-number, .expression, .problem-answer-area')]
        .map((element) => element.getBoundingClientRect())
        .filter((rect) => rect.width > 0 && rect.height > 0);
      if (rects.length === 0) continue;
      const minLeft = Math.min(...rects.map((rect) => rect.left));
      const maxRight = Math.max(...rects.map((rect) => rect.right));
      const overflow = column === 0 ? maxRight - divider.left : divider.right - minLeft;
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
    targets = await waitForJson(`http://127.0.0.1:${debugPort}/json/list`, 30_000);
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
  await navigate(cdp, `${origin}${BASE_PATH}/`);
  await verifySelectHitbox(cdp, '難易度');
  const dropdown = await cdp.evaluate(dropdownProbe);
  if (!dropdown.difficulty.visible || dropdown.difficulty.selected !== '5') {
    throw new Error(`Difficulty dropdown final option is clipped or not selectable: ${JSON.stringify(dropdown.difficulty)}`);
  }
  if (!dropdown.grade.visible || dropdown.grade.selected !== 'grade-9') {
    throw new Error(`Grade dropdown final option is clipped or not selectable: ${JSON.stringify(dropdown.grade)}`);
  }

  const sitemap = readFileSync(join(OUT, 'sitemap.xml'), 'utf8');
  const routes = [...sitemap.matchAll(/<loc>[^<]+\/AutoDrill([^<]*)<\/loc>/g)]
    .map((match) => match[1] || '/')
    .filter((pathname) => pathname !== '/');
  const failures = [];
  for (const route of routes) {
    for (const seed of SEEDS) {
      const localRoute = route.endsWith('/') ? route : `${route}/`;
      await navigate(cdp, `${origin}${BASE_PATH}${localRoute}`);
      const result = await cdp.evaluate(worksheetProbe(seed));
      if (result.alert) failures.push({ route, seed, reason: `UI alert: ${result.alert}` });
      for (const crossing of result.crossings) {
        console.warn(`[layout] divider crossing: ${route} seed=${seed} problem=${crossing.problem} overflow=${crossing.overflow}px expression=${crossing.expression}`);
        failures.push({ route, seed, ...crossing });
      }
      if (cdp.consoleErrors.length > 0) failures.push({ route, seed, reason: `console errors: ${cdp.consoleErrors.join(' | ')}` });
      console.log(`[layout] ${route} seed=${seed}: ${result.count} problems, ${result.gradeClass}, expression ${result.fontSize}, crossings=${result.crossings.length}`);
    }
  }
  if (failures.length > 0) throw new Error(`Browser worksheet layout verification failed with ${failures.length} issue(s).`);
  console.log(`Browser layout verified: dropdowns selectable and ${routes.length * SEEDS.length} worksheet samples do not cross the center divider.`);
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
