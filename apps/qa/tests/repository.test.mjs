import assert from 'node:assert/strict';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { openDatabase } from '../src/db.mjs';
import { QaRepository } from '../src/repository.mjs';

function fixture(maxMigration) {
  const directory = mkdtempSync(join(tmpdir(), 'autodrill-qa-test-'));
  const path = join(directory, 'qa.sqlite3');
  const opened = openDatabase({ path, maxMigration });
  const repository = new QaRepository(opened.database, { gitSha: 'test-git-sha' });
  return { directory, path, database: opened.database, repository, cleanup() { opened.database.close(); rmSync(directory, { recursive: true, force: true }); } };
}

function seed(repository) {
  const session = repository.createSession({ evaluator: 'User', local_timezone: 'Asia/Tokyo', note: 'test' });
  const item = repository.createItem({ unit_name: 'たし算', problem_representation: '1 + 2 =', canonical_answer: '3', source: 'manual' });
  return { session, item };
}

function start(repository, session, item) {
  return repository.startAttempt({ session_id: session.id, item_id: item.id, browser_version: 'Test Browser', client_wall_at: '2026-08-30T00:00:00.000Z', client_monotonic_ms: 10 });
}

test('completed raw observation survives database restart', () => {
  const first = fixture();
  try {
    const { session, item } = seed(first.repository);
    const attempt = start(first.repository, session, item);
    first.repository.recordEvent(attempt.id, { event_type: 'answer_focused', client_monotonic_ms: 11 });
    first.repository.saveDraft(attempt.id, { raw_user_answer: '3', client_monotonic_ms: 20 });
    const submitted = first.repository.submitAttempt(attempt.id, { outcome: 'answered', raw_user_answer: '3', elapsed_since_shown_ms: 1250, client_monotonic_ms: 1260 });
    assert.equal(submitted.state, 'rating');
    assert.equal(Object.hasOwn(submitted, 'canonical_answer'), false, 'submit response must not reveal the answer');
    const rated = first.repository.rateAttempt(attempt.id, { difficulty_rating: 2, singularity_rating: 1, rating_duration_ms: 800, note: '典型的', client_monotonic_ms: 2060 });
    assert.equal(rated.canonical_answer, '3');
    first.database.close();

    const reopened = openDatabase({ path: first.path });
    const repository = new QaRepository(reopened.database, { gitSha: 'next-git-sha' });
    const detail = repository.attemptDetail(attempt.id);
    assert.equal(detail.raw_user_answer, '3');
    assert.equal(detail.correctness, 'correct');
    assert.equal(detail.answer_elapsed_ms, 1250);
    assert.equal(detail.evaluations[0].difficulty_rating, 2);
    assert.deepEqual(detail.events.map((event) => event.event_type), [
      'shown', 'answer_focused', 'answer_changed', 'submit', 'rating_started', 'rating_submitted', 'answer_revealed',
    ]);
    reopened.database.close();
    rmSync(first.directory, { recursive: true, force: true });
  } catch (error) {
    try { first.database.close(); } catch {}
    rmSync(first.directory, { recursive: true, force: true });
    throw error;
  }
});

test('rating correction appends a revision instead of overwriting', () => {
  const data = fixture();
  try {
    const { session, item } = seed(data.repository);
    const attempt = start(data.repository, session, item);
    data.repository.submitAttempt(attempt.id, { outcome: 'answered', raw_user_answer: '4', elapsed_since_shown_ms: 1000 });
    data.repository.rateAttempt(attempt.id, { difficulty_rating: 2, singularity_rating: 3, rating_duration_ms: 500 });
    const revised = data.repository.rateAttempt(attempt.id, { difficulty_rating: 4, singularity_rating: 5, note: '再考' });
    assert.deepEqual(revised.evaluations.map(({ revision_number, difficulty_rating, singularity_rating, pre_answer_reveal }) => ({ revision_number, difficulty_rating, singularity_rating, pre_answer_reveal })), [
      { revision_number: 1, difficulty_rating: 2, singularity_rating: 3, pre_answer_reveal: 1 },
      { revision_number: 2, difficulty_rating: 4, singularity_rating: 5, pre_answer_reveal: 0 },
    ]);
    assert.equal(revised.events.at(-1).event_type, 'rating_revised');
  } finally { data.cleanup(); }
});

test('problem correction preserves exact historical snapshots and hashes', () => {
  const data = fixture();
  try {
    const item = data.repository.createItem({ unit_name: '分数', problem_representation: '  1/2 + 1/2 =  ', canonical_answer: ' 1 ', source: 'manual' });
    const before = data.repository.itemDetail(item.id);
    assert.equal(before.problem_representation, '  1/2 + 1/2 =  ');
    const after = data.repository.reviseItem(item.id, { unit_name: '分数の加法', problem_representation: '1/2 + 1/2 =', canonical_answer: '1', reason: '表記訂正' });
    assert.equal(after.current_revision_number, 2);
    assert.notEqual(after.revision_content_hash, before.revision_content_hash);
    const revisions = data.database.prepare('SELECT revision_number,problem_representation,content_hash FROM item_revisions WHERE item_id=? ORDER BY revision_number').all(item.id);
    assert.equal(revisions.length, 2);
    assert.equal(revisions[0].problem_representation, '  1/2 + 1/2 =  ');
    assert.notEqual(revisions[0].content_hash, revisions[1].content_hash);
    assert.equal(data.database.prepare("SELECT COUNT(*) AS n FROM change_audit WHERE entity_id=? AND action='revised'").get(item.id).n, 1);
  } finally { data.cleanup(); }
});

test('repeated exposure creates distinct attempts', () => {
  const data = fixture();
  try {
    const { session, item } = seed(data.repository);
    for (const answer of ['3', '2']) {
      const attempt = start(data.repository, session, item);
      data.repository.submitAttempt(attempt.id, { outcome: 'answered', raw_user_answer: answer });
      data.repository.rateAttempt(attempt.id, { difficulty_rating: 2, singularity_rating: 2 });
    }
    const attempts = data.database.prepare('SELECT id,exposure_count FROM attempts ORDER BY exposure_count').all();
    assert.equal(attempts.length, 2);
    assert.deepEqual(attempts.map((row) => row.exposure_count), [1, 2]);
    assert.notEqual(attempts[0].id, attempts[1].id);
  } finally { data.cleanup(); }
});

test('incomplete draft resumes intact and can be explicitly abandoned', () => {
  const first = fixture();
  try {
    const { session, item } = seed(first.repository);
    const attempt = start(first.repository, session, item);
    first.repository.saveDraft(attempt.id, { raw_user_answer: '途中', client_monotonic_ms: 50 });
    first.database.close();
    const reopened = openDatabase({ path: first.path });
    const repository = new QaRepository(reopened.database, { gitSha: 'test-git-sha' });
    assert.equal(repository.activeAttempt(session.id).raw_user_answer, '途中');
    repository.abandonAttempt(attempt.id, { reason: 'reload test cleanup' });
    assert.equal(repository.activeAttempt(session.id), null);
    assert.equal(repository.attemptDetail(attempt.id).state, 'abandoned');
    reopened.database.close();
    rmSync(first.directory, { recursive: true, force: true });
  } catch (error) {
    try { first.database.close(); } catch {}
    rmSync(first.directory, { recursive: true, force: true });
    throw error;
  }
});

test('exports contain provenance, raw events, revisions, and analysis columns', () => {
  const data = fixture();
  try {
    const { session, item } = seed(data.repository);
    const attempt = start(data.repository, session, item);
    data.repository.saveDraft(attempt.id, { raw_user_answer: '3' });
    data.repository.submitAttempt(attempt.id, { outcome: 'answered', raw_user_answer: '3', elapsed_since_shown_ms: 900 });
    data.repository.rateAttempt(attempt.id, { difficulty_rating: 1, singularity_rating: 1, note: 'easy' });
    const full = data.repository.fullExport();
    assert.equal(full.manifest.qa_schema_version, 2);
    assert.equal(full.data.attempts[0].autodrill_git_sha, 'test-git-sha');
    assert.ok(full.data.input_events.length >= 6);
    assert.equal(full.data.evaluations[0].note, 'easy');
    assert.equal(full.data.selection_events[0].candidate_source, 'local_queue');
    assert.equal(full.data.selection_events[0].selection_probability, 1);
    const csv = data.repository.analysisCsv();
    for (const header of ['export_schema_version', 'raw_user_answer', 'canonical_answer', 'difficulty_rating', 'singularity_rating', 'autodrill_git_sha']) assert.match(csv.split('\n')[0], new RegExp(header));
    assert.match(csv, /test-git-sha/);
    assert.equal(data.database.prepare('PRAGMA integrity_check').get().integrity_check, 'ok');
    assert.deepEqual(data.database.prepare('PRAGMA foreign_key_check').all(), []);
  } finally { data.cleanup(); }
});

test('ordered migration preserves observations from schema v1', () => {
  const data = fixture(1);
  const { session, item } = seed(data.repository);
  const attempt = start(data.repository, session, item);
  data.repository.saveDraft(attempt.id, { raw_user_answer: 'migration-data' });
  data.database.close();
  const reopened = openDatabase({ path: data.path });
  try {
    assert.equal(reopened.schemaVersion, 2);
    assert.equal(reopened.database.prepare('SELECT raw_user_answer FROM attempts WHERE id=?').get(attempt.id).raw_user_answer, 'migration-data');
    assert.doesNotThrow(() => reopened.database.prepare('SELECT * FROM model_runs').all());
  } finally { reopened.database.close(); rmSync(data.directory, { recursive: true, force: true }); }
});
