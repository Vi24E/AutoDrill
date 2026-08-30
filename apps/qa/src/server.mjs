#!/usr/bin/env node
import { createReadStream, existsSync, statSync } from 'node:fs';
import { createServer } from 'node:http';
import { extname, join, normalize } from 'node:path';
import { fileURLToPath } from 'node:url';
import { openDatabase } from './db.mjs';
import { QaRepository } from './repository.mjs';
import { QaValidationError } from './constants.mjs';

const PUBLIC_DIR = fileURLToPath(new URL('../public/', import.meta.url));
const MAX_BODY_BYTES = 1_200_000;
const MIME = { '.html': 'text/html; charset=utf-8', '.js': 'application/javascript; charset=utf-8', '.css': 'text/css; charset=utf-8', '.svg': 'image/svg+xml' };

function securityHeaders() {
  return {
    'Cache-Control': 'no-store',
    'Content-Security-Policy': "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self'; base-uri 'none'; form-action 'self'; frame-ancestors 'none'",
    'X-Content-Type-Options': 'nosniff',
    'X-Frame-Options': 'DENY',
    'Referrer-Policy': 'no-referrer',
  };
}

async function readJson(req) {
  const chunks = [];
  let size = 0;
  for await (const chunk of req) {
    size += chunk.length;
    if (size > MAX_BODY_BYTES) throw new QaValidationError('Request body is too large.', 413);
    chunks.push(chunk);
  }
  if (!chunks.length) return {};
  try { return JSON.parse(Buffer.concat(chunks).toString('utf8')); }
  catch { throw new QaValidationError('Request body must be valid JSON.'); }
}

function sendJson(res, status, value, headers = {}) {
  res.writeHead(status, { ...securityHeaders(), 'Content-Type': 'application/json; charset=utf-8', ...headers });
  res.end(JSON.stringify(value));
}

function routeMatch(pathname, pattern) {
  const names = [];
  const expression = new RegExp(`^${pattern.replace(/:([a-z_]+)/g, (_, name) => { names.push(name); return '([^/]+)'; })}$`);
  const match = pathname.match(expression);
  return match ? Object.fromEntries(names.map((name, index) => [name, decodeURIComponent(match[index + 1])])) : null;
}

function serveStatic(pathname, res) {
  const requested = pathname === '/' ? 'index.html' : pathname.slice(1);
  const safe = normalize(requested).replace(/^([.][.][/\\])+/, '');
  const file = join(PUBLIC_DIR, safe);
  if (!file.startsWith(PUBLIC_DIR) || !existsSync(file) || !statSync(file).isFile()) return false;
  res.writeHead(200, { ...securityHeaders(), 'Content-Type': MIME[extname(file)] ?? 'application/octet-stream' });
  createReadStream(file).pipe(res);
  return true;
}

export function createQaServer({ databasePath, port = 4179, host = '127.0.0.1', gitSha, quiet = false } = {}) {
  const opened = openDatabase({ path: databasePath });
  const repository = new QaRepository(opened.database, { gitSha });
  const server = createServer(async (req, res) => {
    try {
      const url = new URL(req.url ?? '/', `http://${req.headers.host ?? '127.0.0.1'}`);
      if (!url.pathname.startsWith('/api/')) {
        if (!serveStatic(url.pathname, res)) sendJson(res, 404, { error: 'Not found.' });
        return;
      }
      if (req.method !== 'GET' && req.headers['x-autodrill-qa'] !== '1') {
        throw new QaValidationError('Missing local QA request header.', 403);
      }

      if (req.method === 'GET' && url.pathname === '/api/state') {
        const session = repository.currentSession();
        const activeAttempt = session ? repository.activeAttempt(session.id) : null;
        sendJson(res, 200, { metadata: repository.metadata(), databasePath: opened.path, session, activeAttempt, items: activeAttempt ? [] : repository.listItems({ unit: url.searchParams.get('unit') ?? '' }) });
        return;
      }
      if (req.method === 'POST' && url.pathname === '/api/sessions') { sendJson(res, 201, repository.createSession(await readJson(req))); return; }
      let params = routeMatch(url.pathname, '/api/sessions/:id/end');
      if (req.method === 'POST' && params) { sendJson(res, 200, repository.endSession(params.id)); return; }
      if (req.method === 'GET' && url.pathname === '/api/sessions') { sendJson(res, 200, repository.listSessions()); return; }

      if (req.method === 'POST' && url.pathname === '/api/items') { sendJson(res, 201, repository.createItem(await readJson(req))); return; }
      if (req.method === 'GET' && url.pathname === '/api/items') {
        if (repository.anyActiveAttempt()) throw new QaValidationError('Problem list is locked during an active attempt.', 423);
        sendJson(res, 200, repository.listItems({ unit: url.searchParams.get('unit') ?? '' })); return;
      }
      params = routeMatch(url.pathname, '/api/items/:id');
      if (req.method === 'GET' && params) {
        if (repository.anyActiveAttempt()) throw new QaValidationError('Problem details are locked during an active attempt.', 423);
        const item = repository.itemDetail(params.id); if (!item) throw new QaValidationError('Problem not found.', 404);
        sendJson(res, 200, item); return;
      }
      if (req.method === 'PATCH' && params) { sendJson(res, 200, repository.reviseItem(params.id, await readJson(req))); return; }

      if (req.method === 'POST' && url.pathname === '/api/attempts') { sendJson(res, 201, repository.startAttempt(await readJson(req))); return; }
      params = routeMatch(url.pathname, '/api/attempts/:id/draft');
      if (req.method === 'PATCH' && params) { sendJson(res, 200, repository.saveDraft(params.id, await readJson(req))); return; }
      params = routeMatch(url.pathname, '/api/attempts/:id/events');
      if (req.method === 'POST' && params) { sendJson(res, 201, repository.recordEvent(params.id, await readJson(req))); return; }
      params = routeMatch(url.pathname, '/api/attempts/:id/submit');
      if (req.method === 'POST' && params) { sendJson(res, 200, repository.submitAttempt(params.id, await readJson(req))); return; }
      params = routeMatch(url.pathname, '/api/attempts/:id/ratings');
      if (req.method === 'POST' && params) { sendJson(res, 201, repository.rateAttempt(params.id, await readJson(req))); return; }
      params = routeMatch(url.pathname, '/api/attempts/:id/abandon');
      if (req.method === 'POST' && params) { sendJson(res, 200, repository.abandonAttempt(params.id, await readJson(req))); return; }
      params = routeMatch(url.pathname, '/api/attempts/:id');
      if (req.method === 'GET' && params) {
        const detail = repository.attemptDetail(params.id); if (!detail) throw new QaValidationError('Attempt not found.', 404);
        if (['solving', 'rating'].includes(detail.state)) throw new QaValidationError('Answer details remain hidden until rating is complete.', 423);
        sendJson(res, 200, detail); return;
      }

      if (req.method === 'GET' && url.pathname === '/api/history') {
        sendJson(res, 200, repository.history(Object.fromEntries(url.searchParams))); return;
      }
      if (req.method === 'GET' && url.pathname === '/api/export/full') {
        const payload = JSON.stringify(repository.fullExport(), null, 2);
        res.writeHead(200, { ...securityHeaders(), 'Content-Type': 'application/json; charset=utf-8', 'Content-Disposition': `attachment; filename="autodrill-qa-full-${new Date().toISOString().slice(0, 10)}.json"` });
        res.end(payload); return;
      }
      if (req.method === 'GET' && url.pathname === '/api/export/analysis.csv') {
        res.writeHead(200, { ...securityHeaders(), 'Content-Type': 'text/csv; charset=utf-8', 'Content-Disposition': `attachment; filename="autodrill-qa-analysis-${new Date().toISOString().slice(0, 10)}.csv"` });
        res.end(`\uFEFF${repository.analysisCsv()}`); return;
      }
      sendJson(res, 404, { error: 'API route not found.' });
    } catch (error) {
      const status = error instanceof QaValidationError ? error.status : 500;
      if (status === 500) console.error(error);
      sendJson(res, status, { error: status === 500 ? 'Internal QA application error.' : error.message });
    }
  });

  return {
    repository, databasePath: opened.path, server,
    listen: () => new Promise((resolveListen, reject) => {
      server.once('error', reject);
      server.listen(port, host, () => {
        const address = server.address();
        if (!quiet) console.log(`AutoDrill QA: http://${host}:${address.port}\nDatabase: ${opened.path}`);
        resolveListen(address);
      });
    }),
    close: () => new Promise((resolveClose, reject) => server.close((error) => {
      if (error) reject(error); else { opened.database.close(); resolveClose(); }
    })),
  };
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const qa = createQaServer({ port: Number(process.env.AUTODRILL_QA_PORT ?? 4179) });
  await qa.listen();
  for (const signal of ['SIGINT', 'SIGTERM']) process.once(signal, async () => { await qa.close(); process.exit(0); });
}
