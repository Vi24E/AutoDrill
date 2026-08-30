import assert from 'node:assert/strict';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { createQaServer } from '../src/server.mjs';

async function request(base, path, body, method = 'POST') {
  const response = await fetch(`${base}${path}`, { method, headers: body ? { 'Content-Type': 'application/json', 'X-AutoDrill-QA': '1' } : {}, body: body ? JSON.stringify(body) : undefined });
  return { response, payload: await response.json() };
}

test('HTTP boundary withholds canonical answer and history until rating', async () => {
  const directory = mkdtempSync(join(tmpdir(), 'autodrill-qa-server-'));
  const qa = createQaServer({ databasePath: join(directory, 'qa.sqlite3'), port: 0, gitSha: 'http-test', quiet: true });
  const address = await qa.listen();
  const base = `http://127.0.0.1:${address.port}`;
  try {
    const { payload: session } = await request(base, '/api/sessions', { evaluator: 'User', local_timezone: 'Asia/Tokyo' });
    const { payload: item } = await request(base, '/api/items', { source: 'manual', unit_name: '九九', problem_representation: '2 × 3 =', canonical_answer: '6' });
    const { payload: attempt } = await request(base, '/api/attempts', { session_id: session.id, item_id: item.id });
    assert.equal(JSON.stringify(attempt).includes('canonical'), false);
    const state = await fetch(`${base}/api/state`).then((response) => response.json());
    assert.equal(JSON.stringify(state).includes('canonical_answer'), false);
    const historyLocked = await fetch(`${base}/api/history`);
    assert.equal(historyLocked.status, 423);
    const { payload: submitted } = await request(base, `/api/attempts/${attempt.id}/submit`, { outcome: 'answered', raw_user_answer: '6' });
    assert.equal(JSON.stringify(submitted).includes('canonical'), false);
    const ratingState = await fetch(`${base}/api/state`).then((response) => response.json());
    assert.equal(JSON.stringify(ratingState.activeAttempt).includes('correctness'), false);
    assert.equal(JSON.stringify(ratingState.activeAttempt).includes('canonical_answer'), false);
    const { payload: revealed } = await request(base, `/api/attempts/${attempt.id}/ratings`, { difficulty_rating: 1, singularity_rating: 1 });
    assert.equal(revealed.canonical_answer, '6');
    assert.equal(revealed.correctness, 'correct');
  } finally { await qa.close(); rmSync(directory, { recursive: true, force: true }); }
});

test('active attempt and draft survive a local server restart', async () => {
  const directory = mkdtempSync(join(tmpdir(), 'autodrill-qa-restart-'));
  const databasePath = join(directory, 'qa.sqlite3');
  let qa = createQaServer({ databasePath, port: 0, gitSha: 'before-restart', quiet: true });
  let address = await qa.listen();
  let base = `http://127.0.0.1:${address.port}`;
  try {
    const { payload: session } = await request(base, '/api/sessions', { evaluator: 'User', local_timezone: 'Asia/Tokyo' });
    const { payload: item } = await request(base, '/api/items', { source: 'manual', unit_name: 'restart', problem_representation: '9 + 9 =', canonical_answer: '18' });
    const { payload: attempt } = await request(base, '/api/attempts', { session_id: session.id, item_id: item.id });
    await request(base, `/api/attempts/${attempt.id}/draft`, { raw_user_answer: '1' }, 'PATCH');
    await qa.close();

    qa = createQaServer({ databasePath, port: 0, gitSha: 'after-restart', quiet: true });
    address = await qa.listen();
    base = `http://127.0.0.1:${address.port}`;
    const state = await fetch(`${base}/api/state`).then((response) => response.json());
    assert.equal(state.activeAttempt.id, attempt.id);
    assert.equal(state.activeAttempt.raw_user_answer, '1');
    assert.equal(state.activeAttempt.autodrill_git_sha, undefined, 'safe active response does not leak unrelated provenance fields');
  } finally {
    if (qa.server.listening) await qa.close();
    rmSync(directory, { recursive: true, force: true });
  }
});

test('quick flow creates a session and preserves random-generation and WASM-grading provenance', async () => {
  const directory = mkdtempSync(join(tmpdir(), 'autodrill-qa-quick-'));
  const autodrillRuntime = {
    async generateRandomProblem() {
      return {
        item: {
          source: 'autodrill',
          source_identifier: '1:5:QaA1:4:0',
          unit_name: '一桁の足し算',
          problem_representation: '2 + 3 =',
          canonical_answer: '5',
          original_source_payload: { integration_version: 'autodrill_qa_wasm_v1', problem: { id: 1 }, worksheet: { problems: [{ id: 1 }] } },
        },
        selection: {
          selection_policy: 'autodrill_random_v1',
          candidate_source: 'test_theme_registry',
          filters: { requested_difficulty: 4 },
          random_seed: 'selection-seed',
          selection_probability: 0.01,
        },
      };
    },
    async gradeAnswer() {
      return {
        correctness: 'correct',
        normalized_user_answer: '{"type":"integer","value":"5"}',
        grading_method: 'autodrill_wasm_grade_v1',
        raw_result: { parsed: { type: 'integer', value: '5' }, graded: { is_correct: true, warnings: [] } },
      };
    },
  };
  const qa = createQaServer({ databasePath: join(directory, 'qa.sqlite3'), port: 0, gitSha: 'quick-test', quiet: true, autodrillRuntime });
  const address = await qa.listen();
  const base = `http://127.0.0.1:${address.port}`;
  try {
    const { payload: attempt } = await request(base, '/api/quick/next', { local_timezone: 'Asia/Tokyo', browser_version: 'test' });
    assert.equal(attempt.problem_representation, '2 + 3 =');
    assert.equal(JSON.stringify(attempt).includes('canonical_answer'), false);
    const { payload: submitted } = await request(base, `/api/attempts/${attempt.id}/submit`, { outcome: 'answered', raw_user_answer: '5' });
    assert.equal(JSON.stringify(submitted).includes('correctness'), false);
    const { payload: revealed } = await request(base, `/api/attempts/${attempt.id}/ratings`, { difficulty_rating: 2, singularity_rating: 3 });
    assert.equal(revealed.correctness, 'correct');
    assert.equal(revealed.grading_method, 'autodrill_wasm_grade_v1');
    assert.equal(revealed.selection.selection_policy, 'autodrill_random_v1');
    assert.equal(revealed.selection.random_seed, 'selection-seed');
    assert.deepEqual(revealed.events.find((event) => event.event_type === 'submit').payload.grading.graded.warnings, []);
    const { payload: repeated } = await request(base, '/api/quick/next', { local_timezone: 'Asia/Tokyo', browser_version: 'test' });
    assert.equal(repeated.item_id, attempt.item_id);
    assert.equal(repeated.exposure_count, 2);
  } finally {
    await qa.close();
    rmSync(directory, { recursive: true, force: true });
  }
});
