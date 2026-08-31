import { createHash, randomUUID } from 'node:crypto';
import {
  APP_VERSION, ATTEMPT_OUTCOMES, EXPORT_SCHEMA_VERSION, INPUT_EVENT_TYPES,
  QA_SCHEMA_VERSION, RATING_SCALE, QaValidationError, assertRating,
} from './constants.mjs';
import { transaction } from './db.mjs';
import { captureGitState, normalizeGitState } from './git-state.mjs';

const MAX_TEXT = 100_000;

function now() { return new Date().toISOString(); }
function json(value) { return JSON.stringify(value ?? {}); }
function parseJson(value, fallback = null) {
  if (value == null) return fallback;
  try { return JSON.parse(value); } catch { return fallback; }
}
function requiredText(value, name, max = MAX_TEXT) {
  if (typeof value !== 'string' || !value.trim()) throw new QaValidationError(`${name} is required.`);
  if (value.length > max) throw new QaValidationError(`${name} is too long.`);
  return value.trim();
}
function requiredExactText(value, name, max = MAX_TEXT) {
  if (typeof value !== 'string' || !value.trim()) throw new QaValidationError(`${name} is required.`);
  if (value.length > max) throw new QaValidationError(`${name} is too long.`);
  return value;
}
function optionalText(value, name, max = MAX_TEXT) {
  if (value == null || value === '') return null;
  if (typeof value !== 'string' || value.length > max) throw new QaValidationError(`${name} is invalid.`);
  return value;
}
function normalizeManualAnswer(value) { return value.normalize('NFKC').trim().replace(/\s+/g, ' '); }
function contentHash({ unitName, problemRepresentation, canonicalAnswer, sourcePayload }) {
  return createHash('sha256').update(json({ unitName, problemRepresentation, canonicalAnswer, sourcePayload })).digest('hex');
}
export class QaRepository {
  constructor(database, { gitSha, gitState = gitSha ? normalizeGitState(null, gitSha) : captureGitState(), clock = now } = {}) {
    this.db = database;
    this.gitState = normalizeGitState(gitState, gitSha ?? 'unknown');
    this.gitSha = this.gitState.head_sha;
    this.clock = clock;
  }

  metadata() {
    return { appVersion: APP_VERSION, qaSchemaVersion: QA_SCHEMA_VERSION, exportSchemaVersion: EXPORT_SCHEMA_VERSION, gitSha: this.gitSha, gitState: this.gitState, ratingScale: RATING_SCALE };
  }

  createSession(input = {}) {
    if (this.currentSession()) throw new QaValidationError('End the current QA session before starting another one.', 409);
    const session = {
      id: randomUUID(), evaluator: optionalText(input.evaluator, 'evaluator', 200) ?? 'User',
      startedAt: this.clock(), timezone: requiredText(input.local_timezone ?? 'UTC', 'local_timezone', 200),
      note: optionalText(input.note, 'note', 10_000),
    };
    this.db.prepare(`INSERT INTO qa_sessions
      (id,evaluator,started_at,local_timezone,application_version,qa_schema_version,autodrill_git_sha,autodrill_git_state_json,note)
      VALUES (?,?,?,?,?,?,?,?,?)`).run(session.id, session.evaluator, session.startedAt, session.timezone, APP_VERSION, QA_SCHEMA_VERSION, this.gitSha, json(this.gitState), session.note);
    return this.getSession(session.id);
  }

  getSession(id) { return this.db.prepare('SELECT * FROM qa_sessions WHERE id = ?').get(id) ?? null; }
  currentSession() { return this.db.prepare('SELECT * FROM qa_sessions WHERE ended_at IS NULL AND invalidated_at IS NULL ORDER BY started_at DESC LIMIT 1').get() ?? null; }
  listSessions() {
    return this.db.prepare(`SELECT s.*, COUNT(a.id) AS attempt_count,
      SUM(CASE WHEN a.state='complete' THEN 1 ELSE 0 END) AS completed_count
      FROM qa_sessions s LEFT JOIN attempts a ON a.session_id=s.id
      GROUP BY s.id ORDER BY s.started_at DESC`).all();
  }
  endSession(id) {
    if (this.activeAttempt(id)) throw new QaValidationError('Finish or abandon the active attempt first.', 409);
    this.db.prepare('UPDATE qa_sessions SET ended_at = COALESCE(ended_at, ?) WHERE id = ?').run(this.clock(), id);
    return this.getSession(id);
  }

  createItem(input) {
    const source = ['manual', 'autodrill', 'imported', 'other'].includes(input.source) ? input.source : 'manual';
    const unitName = requiredText(input.unit_name, 'unit_name', 1_000);
    const problemRepresentation = requiredExactText(input.problem_representation, 'problem_representation');
    const canonicalAnswer = requiredExactText(input.canonical_answer, 'canonical_answer');
    let sourcePayload = null;
    if (input.original_source_payload != null && input.original_source_payload !== '') {
      if (typeof input.original_source_payload === 'string') {
        try { sourcePayload = JSON.parse(input.original_source_payload); }
        catch { throw new QaValidationError('original_source_payload must be valid JSON.'); }
      } else sourcePayload = input.original_source_payload;
    }
    const createdAt = this.clock();
    const itemId = randomUUID();
    const revisionId = randomUUID();
    const hash = contentHash({ unitName, problemRepresentation, canonicalAnswer, sourcePayload });
    return transaction(this.db, () => {
      this.db.prepare(`INSERT INTO items(id,source,source_identifier,content_hash,created_at)
        VALUES (?,?,?,?,?)`).run(itemId, source, optionalText(input.source_identifier, 'source_identifier', 2_000), hash, createdAt);
      this.db.prepare(`INSERT INTO item_revisions
        (id,item_id,revision_number,unit_name,problem_representation,canonical_answer,content_hash,original_source_payload_json,revision_reason,created_at)
        VALUES (?,?,?,?,?,?,?,?,?,?)`).run(revisionId, itemId, 1, unitName, problemRepresentation, canonicalAnswer, hash,
          sourcePayload == null ? null : json(sourcePayload), 'initial_import', createdAt);
      const position = this.db.prepare('SELECT COALESCE(MAX(position),0)+1 AS position FROM queue_entries').get().position;
      this.db.prepare('INSERT INTO queue_entries(id,item_id,added_at,position,state) VALUES (?,?,?,?,?)')
        .run(randomUUID(), itemId, createdAt, position, 'queued');
      this.audit('item', itemId, 'created', null, { source, unitName, problemRepresentation, canonicalAnswer, sourcePayload }, 'User');
      return { id: itemId, content_hash: hash, duplicate_count: this.db.prepare('SELECT COUNT(*) AS n FROM items WHERE content_hash=?').get(hash).n };
    });
  }

  reviseItem(itemId, input) {
    if (this.anyActiveAttempt()) throw new QaValidationError('Problem editing is locked during an active attempt.', 423);
    return transaction(this.db, () => {
      const old = this.itemDetail(itemId);
      if (!old) throw new QaValidationError('Problem not found.', 404);
      const unitName = requiredText(input.unit_name ?? old.unit_name, 'unit_name', 1_000);
      const problemRepresentation = requiredExactText(input.problem_representation ?? old.problem_representation, 'problem_representation');
      const canonicalAnswer = requiredExactText(input.canonical_answer ?? old.canonical_answer, 'canonical_answer');
      const sourcePayload = input.original_source_payload === undefined ? parseJson(old.original_source_payload_json) : input.original_source_payload;
      const revision = old.current_revision_number + 1;
      const changedAt = this.clock();
      const hash = contentHash({ unitName, problemRepresentation, canonicalAnswer, sourcePayload });
      this.db.prepare(`INSERT INTO item_revisions
        (id,item_id,revision_number,unit_name,problem_representation,canonical_answer,content_hash,original_source_payload_json,revision_reason,created_at)
        VALUES (?,?,?,?,?,?,?,?,?,?)`).run(randomUUID(), itemId, revision, unitName, problemRepresentation, canonicalAnswer, hash,
          sourcePayload == null ? null : json(sourcePayload), requiredText(input.reason, 'reason', 2_000), changedAt);
      this.db.prepare('UPDATE items SET current_revision_number=?, content_hash=? WHERE id=?').run(revision, hash, itemId);
      this.audit('item', itemId, 'revised', old, { unitName, problemRepresentation, canonicalAnswer, sourcePayload, revision }, 'User', input.reason);
      return this.itemDetail(itemId);
    });
  }

  listItems({ unit = '' } = {}) {
    const like = `%${unit}%`;
    return this.db.prepare(`SELECT i.id,i.source,i.source_identifier,i.content_hash,i.created_at,i.current_revision_number,
      r.unit_name,r.problem_representation,q.position,
      (SELECT COUNT(*) FROM attempts a WHERE a.item_id=i.id) AS exposure_count
      FROM items i JOIN item_revisions r ON r.item_id=i.id AND r.revision_number=i.current_revision_number
      JOIN queue_entries q ON q.item_id=i.id AND q.invalidated_at IS NULL
      WHERE i.invalidated_at IS NULL AND (?='' OR r.unit_name LIKE ?)
      ORDER BY q.position,r.created_at`).all(unit, like);
  }

  itemDetail(itemId) {
    return this.db.prepare(`SELECT i.*,r.content_hash AS revision_content_hash,r.unit_name,r.problem_representation,r.canonical_answer,r.original_source_payload_json,
      r.revision_reason,r.created_at AS revision_created_at
      FROM items i JOIN item_revisions r ON r.item_id=i.id AND r.revision_number=i.current_revision_number
      WHERE i.id=?`).get(itemId) ?? null;
  }

  findItemBySourceIdentifier(source, sourceIdentifier) {
    if (!sourceIdentifier) return null;
    return this.db.prepare(`SELECT id FROM items WHERE source=? AND source_identifier=? AND invalidated_at IS NULL ORDER BY created_at LIMIT 1`)
      .get(source, sourceIdentifier) ?? null;
  }

  activeAttempt(sessionId = null) {
    const sql = `SELECT id FROM attempts WHERE state IN ('solving','rating') ${sessionId ? 'AND session_id=?' : ''}
      ORDER BY shown_at DESC LIMIT 1`;
    const row = sessionId ? this.db.prepare(sql).get(sessionId) : this.db.prepare(sql).get();
    return row ? this.safeAttempt(row.id) : null;
  }
  anyActiveAttempt() { return Boolean(this.activeAttempt()); }

  startAttempt(input) {
    const session = this.getSession(requiredText(input.session_id, 'session_id', 100));
    if (!session || session.ended_at || session.invalidated_at) throw new QaValidationError('An active QA session is required.', 409);
    if (this.activeAttempt(session.id)) throw new QaValidationError('This session already has an active attempt.', 409);
    const selectionContext = input.selection_context ?? null;
    const policy = input.selection_policy === 'random' ? 'random' : 'manual_order';
    const unit = typeof input.unit_filter === 'string' ? input.unit_filter : '';
    let candidates;
    let chosen;
    let randomSeed = null;
    if (input.item_id) {
      chosen = this.db.prepare(`SELECT i.id,i.source,i.source_identifier,i.content_hash,i.created_at,i.current_revision_number,
        r.unit_name,r.problem_representation,q.position,
        (SELECT COUNT(*) FROM attempts a WHERE a.item_id=i.id) AS exposure_count
        FROM items i JOIN item_revisions r ON r.item_id=i.id AND r.revision_number=i.current_revision_number
        JOIN queue_entries q ON q.item_id=i.id AND q.invalidated_at IS NULL
        WHERE i.id=? AND i.invalidated_at IS NULL AND (?='' OR r.unit_name LIKE ?)`)
        .get(input.item_id, unit, `%${unit}%`);
      if (!chosen) throw new QaValidationError('The selected problem is not in the current candidate set.', 409);
      candidates = [chosen];
    } else {
      candidates = this.listItems({ unit });
      if (!candidates.length) throw new QaValidationError('No queued problems match the current filter.', 409);
    }
    if (!chosen && policy === 'random') {
      randomSeed = optionalText(input.random_seed, 'random_seed', 500) ?? randomUUID();
      const digest = createHash('sha256').update(randomSeed).digest();
      chosen = candidates[digest.readUInt32BE(0) % candidates.length];
    } else if (!chosen) chosen = candidates[0];
    const detail = this.itemDetail(chosen.id);
    const attemptId = randomUUID();
    const shownAt = this.clock();
    const ratingOnly = input.rating_only === true;
    return transaction(this.db, () => {
      const exposure = this.db.prepare('SELECT COUNT(*)+1 AS n FROM attempts WHERE item_id=?').get(chosen.id).n;
      this.db.prepare(`INSERT INTO attempts
        (id,session_id,item_id,item_revision_number,exposure_count,state,shown_at,rating_started_at,answer_revealed_at,
         correctness,grading_method,observation_mode,application_version,qa_schema_version,autodrill_git_sha,autodrill_git_state_json,browser_version)
        VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)`).run(attemptId, session.id, chosen.id, detail.current_revision_number, exposure,
          ratingOnly ? 'rating' : 'solving', shownAt, ratingOnly ? shownAt : null, ratingOnly ? shownAt : null,
          ratingOnly ? 'ungraded' : null, ratingOnly ? 'not_collected_assumed_solved_v1' : null,
          ratingOnly ? 'rating_only_answer_shown' : 'answer_then_rating', APP_VERSION, QA_SCHEMA_VERSION, this.gitSha, json(this.gitState),
          optionalText(input.browser_version, 'browser_version', 2_000));
      const probability = selectionContext && Object.hasOwn(selectionContext, 'selection_probability')
        ? selectionContext.selection_probability
        : (input.item_id || policy === 'manual_order' ? 1 : 1 / candidates.length);
      this.db.prepare(`INSERT INTO selection_events
        (id,attempt_id,selected_at,selection_policy,candidate_source,filters_json,random_seed,candidate_item_ids_json,
         model_name,model_version,candidate_scores_json,selection_probability)
        VALUES (?,?,?,?,?,?,?,?,?,?,?,?)`).run(randomUUID(), attemptId, shownAt,
          selectionContext?.selection_policy ?? (input.item_id ? 'explicit_item' : policy),
          selectionContext?.candidate_source ?? 'local_queue',
          json(selectionContext?.filters ?? { unit }), selectionContext?.random_seed ?? randomSeed,
          json(selectionContext ? [chosen.id] : candidates.map((item) => item.id)),
          selectionContext?.model_name ?? null, selectionContext?.model_version ?? null,
          selectionContext?.candidate_scores ? json(selectionContext.candidate_scores) : null, probability);
      this.appendEvent(attemptId, 'shown', { exposure_count: exposure, selection: selectionContext }, input.client_wall_at, input.client_monotonic_ms, shownAt);
      if (ratingOnly) {
        this.appendEvent(attemptId, 'answer_revealed', { reason: 'rating_only_answer_shown' }, input.client_wall_at, input.client_monotonic_ms, shownAt);
        this.appendEvent(attemptId, 'rating_started', { observation_mode: 'rating_only_answer_shown' }, input.client_wall_at, input.client_monotonic_ms, shownAt);
      }
      return this.safeAttempt(attemptId);
    });
  }

  safeAttempt(attemptId) {
    const row = this.db.prepare(`SELECT a.id,a.session_id,a.item_id,a.item_revision_number,a.exposure_count,a.state,a.outcome,a.observation_mode,
      a.shown_at,a.first_interaction_at,a.answer_started_at,a.submitted_at,a.rating_started_at,a.raw_user_answer,
      r.unit_name,r.problem_representation,r.canonical_answer,i.source,i.source_identifier,
      json_extract(r.original_source_payload_json,'$.theme.skill_id') AS source_skill_id
      FROM attempts a JOIN items i ON i.id=a.item_id
      JOIN item_revisions r ON r.item_id=a.item_id AND r.revision_number=a.item_revision_number WHERE a.id=?`).get(attemptId) ?? null;
    if (row) {
      if (row.observation_mode !== 'rating_only_answer_shown') delete row.canonical_answer;
      const draft = this.db.prepare(`SELECT payload_json FROM input_events
        WHERE attempt_id=? AND event_type='rating_selected' ORDER BY sequence_number DESC LIMIT 1`).get(attemptId);
      row.rating_draft = parseJson(draft?.payload_json);
      if (row.rating_draft && row.rating_draft.difficulty_position == null) {
        const span = RATING_SCALE.max - RATING_SCALE.min;
        row.rating_draft.difficulty_position = (row.rating_draft.difficulty - RATING_SCALE.min) / span;
        row.rating_draft.singularity_position = (row.rating_draft.singularity - RATING_SCALE.min) / span;
      }
    }
    return row;
  }

  beginRatingOnly(attemptId, input = {}) {
    const attempt = this.db.prepare('SELECT * FROM attempts WHERE id=?').get(attemptId);
    if (!attempt) throw new QaValidationError('Attempt not found.', 404);
    if (!['solving', 'rating'].includes(attempt.state)) throw new QaValidationError('Attempt is not active.', 409);
    if (attempt.observation_mode === 'rating_only_answer_shown') return this.safeAttempt(attemptId);
    const at = this.clock();
    return transaction(this.db, () => {
      if (attempt.state === 'solving') {
        this.db.prepare(`UPDATE attempts SET state='rating',observation_mode='rating_only_answer_shown',
          correctness='ungraded',grading_method='not_collected_assumed_solved_v1',rating_started_at=?,answer_revealed_at=? WHERE id=?`)
          .run(at, at, attemptId);
      } else {
        this.db.prepare(`UPDATE attempts SET observation_mode='rating_only_answer_shown',answer_revealed_at=COALESCE(answer_revealed_at,?) WHERE id=?`)
          .run(at, attemptId);
      }
      this.appendEvent(attemptId, 'answer_revealed', { reason: 'flow_simplified_to_rating_only' }, input.client_wall_at, input.client_monotonic_ms, at);
      if (attempt.state === 'solving') this.appendEvent(attemptId, 'rating_started', { observation_mode: 'rating_only_answer_shown' }, input.client_wall_at, input.client_monotonic_ms, at);
      return this.safeAttempt(attemptId);
    });
  }

  appendEvent(attemptId, eventType, payload = {}, clientWallAt = null, clientMonotonicMs = null, occurredAt = this.clock()) {
    if (!INPUT_EVENT_TYPES.has(eventType)) throw new QaValidationError(`Unsupported event_type: ${eventType}`);
    const attempt = this.db.prepare('SELECT id FROM attempts WHERE id=?').get(attemptId);
    if (!attempt) throw new QaValidationError('Attempt not found.', 404);
    const sequence = this.db.prepare('SELECT COALESCE(MAX(sequence_number),0)+1 AS n FROM input_events WHERE attempt_id=?').get(attemptId).n;
    this.db.prepare(`INSERT INTO input_events
      (id,attempt_id,sequence_number,event_type,occurred_at,client_wall_at,client_monotonic_ms,payload_json)
      VALUES (?,?,?,?,?,?,?,?)`).run(randomUUID(), attemptId, sequence, eventType, occurredAt,
        optionalText(clientWallAt, 'client_wall_at', 100), Number.isFinite(clientMonotonicMs) ? clientMonotonicMs : null, json(payload));
    if (!['shown', 'answer_revealed', 'rating_started', 'resumed'].includes(eventType)) {
      this.db.prepare('UPDATE attempts SET first_interaction_at=COALESCE(first_interaction_at,?) WHERE id=?').run(occurredAt, attemptId);
    }
    if (eventType === 'answer_started' || eventType === 'first_input') {
      this.db.prepare('UPDATE attempts SET answer_started_at=COALESCE(answer_started_at,?) WHERE id=?').run(occurredAt, attemptId);
    }
    return { sequence_number: sequence, occurred_at: occurredAt };
  }

  saveDraft(attemptId, input) {
    const attempt = this.db.prepare('SELECT * FROM attempts WHERE id=?').get(attemptId);
    if (!attempt) throw new QaValidationError('Attempt not found.', 404);
    if (attempt.state !== 'solving') throw new QaValidationError('Answer draft is no longer editable.', 409);
    const answer = typeof input.raw_user_answer === 'string' ? input.raw_user_answer : '';
    if (answer.length > MAX_TEXT) throw new QaValidationError('Answer is too long.');
    return transaction(this.db, () => {
      this.db.prepare('UPDATE attempts SET raw_user_answer=? WHERE id=?').run(answer, attemptId);
      const eventType = answer ? 'answer_changed' : 'answer_cleared';
      const event = this.appendEvent(attemptId, eventType, { raw_user_answer: answer }, input.client_wall_at, input.client_monotonic_ms);
      return { saved_at: event.occurred_at, sequence_number: event.sequence_number };
    });
  }

  submitAttempt(attemptId, input, grading = null) {
    const attempt = this.db.prepare('SELECT * FROM attempts WHERE id=?').get(attemptId);
    if (!attempt) throw new QaValidationError('Attempt not found.', 404);
    if (attempt.state !== 'solving') throw new QaValidationError('Attempt has already been submitted.', 409);
    const outcome = input.outcome;
    if (!ATTEMPT_OUTCOMES.includes(outcome)) throw new QaValidationError('A supported outcome is required.');
    const raw = typeof input.raw_user_answer === 'string' ? input.raw_user_answer : (attempt.raw_user_answer ?? '');
    if (outcome === 'answered' && !raw.trim()) throw new QaValidationError('Enter an answer or choose another outcome.');
    const item = this.db.prepare(`SELECT r.canonical_answer FROM item_revisions r
      WHERE r.item_id=? AND r.revision_number=?`).get(attempt.item_id, attempt.item_revision_number);
    let normalized = raw ? normalizeManualAnswer(raw) : null;
    let correctness = 'ungraded';
    let gradingMethod = 'not_graded';
    if (outcome === 'answered') {
      if (grading) {
        correctness = grading.correctness;
        normalized = grading.normalized_user_answer;
        gradingMethod = grading.grading_method;
      } else {
        correctness = normalized === normalizeManualAnswer(item.canonical_answer) ? 'correct' : 'incorrect';
        gradingMethod = 'manual_exact_text_nfkc_v1';
      }
    } else if (outcome === 'unable_to_solve') {
      correctness = 'incorrect'; gradingMethod = 'explicit_unable_to_solve_v1';
    }
    const submittedAt = this.clock();
    const needsRating = outcome === 'answered' || outcome === 'unable_to_solve';
    return transaction(this.db, () => {
      this.db.prepare(`UPDATE attempts SET raw_user_answer=?,normalized_user_answer=?,correctness=?,grading_method=?,outcome=?,
        submitted_at=?,rating_started_at=?,state=?,answer_elapsed_ms=? WHERE id=?`).run(raw, normalized, correctness, gradingMethod,
          outcome, submittedAt, needsRating ? submittedAt : null, needsRating ? 'rating' : 'complete',
          Number.isFinite(input.elapsed_since_shown_ms) ? input.elapsed_since_shown_ms : null, attemptId);
      this.appendEvent(attemptId, outcome === 'answered' ? 'submit' : outcome.replace('skipped', 'skip'), { outcome, grading: grading?.raw_result ?? null }, input.client_wall_at, input.client_monotonic_ms, submittedAt);
      if (needsRating) {
        this.appendEvent(attemptId, 'rating_started', {}, input.client_wall_at, input.client_monotonic_ms, submittedAt);
        return { attempt_id: attemptId, state: 'rating', rating_started_at: submittedAt };
      }
      const revealedAt = this.clock();
      this.db.prepare('UPDATE attempts SET answer_revealed_at=?,completed_at=? WHERE id=?').run(revealedAt, revealedAt, attemptId);
      this.appendEvent(attemptId, 'answer_revealed', { reason: outcome }, input.client_wall_at, input.client_monotonic_ms, revealedAt);
      return this.attemptDetail(attemptId);
    });
  }

  rateAttempt(attemptId, input) {
    assertRating(input.difficulty_rating, 'difficulty_rating');
    assertRating(input.singularity_rating, 'singularity_rating');
    const ratingSpan = RATING_SCALE.max - RATING_SCALE.min;
    const difficultyPosition = input.difficulty_position == null
      ? (input.difficulty_rating - RATING_SCALE.min) / ratingSpan : Number(input.difficulty_position);
    const singularityPosition = input.singularity_position == null
      ? (input.singularity_rating - RATING_SCALE.min) / ratingSpan : Number(input.singularity_position);
    for (const [name, position, rating] of [
      ['difficulty_position', difficultyPosition, input.difficulty_rating],
      ['singularity_position', singularityPosition, input.singularity_rating],
    ]) {
      if (!Number.isFinite(position) || position < 0 || position > 1) throw new QaValidationError(`${name} must be between 0 and 1.`);
      if (Math.round(RATING_SCALE.min + position * ratingSpan) !== rating) throw new QaValidationError(`${name} does not match its derived rating.`);
    }
    const confidence = input.confidence == null || input.confidence === '' ? null : Number(input.confidence);
    if (confidence != null && (!Number.isInteger(confidence) || confidence < 1 || confidence > 5)) {
      throw new QaValidationError('confidence must be an integer from 1 to 5.');
    }
    const attempt = this.db.prepare('SELECT * FROM attempts WHERE id=?').get(attemptId);
    if (!attempt) throw new QaValidationError('Attempt not found.', 404);
    if (!['rating', 'complete'].includes(attempt.state)) throw new QaValidationError('Attempt is not ready for rating.', 409);
    const existing = this.db.prepare('SELECT * FROM evaluations WHERE attempt_id=? ORDER BY revision_number DESC LIMIT 1').get(attemptId);
    const revision = (existing?.revision_number ?? 0) + 1;
    const ratedAt = this.clock();
    const preReveal = attempt.answer_revealed_at == null;
    return transaction(this.db, () => {
      const evaluationId = randomUUID();
      this.db.prepare(`INSERT INTO evaluations
        (id,attempt_id,revision_number,difficulty_rating,singularity_rating,difficulty_position,singularity_position,
         rating_scale_version,rated_at,rating_duration_ms,confidence,note,pre_answer_reveal,supersedes_evaluation_id)
        VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?)`).run(evaluationId, attemptId, revision, input.difficulty_rating, input.singularity_rating,
          difficultyPosition, singularityPosition, RATING_SCALE.version, ratedAt, Number.isFinite(input.rating_duration_ms) ? input.rating_duration_ms : null,
          confidence, optionalText(input.note, 'note', 10_000), preReveal ? 1 : 0, existing?.id ?? null);
      const ratingPayload = {
        difficulty: input.difficulty_rating, singularity: input.singularity_rating,
        difficulty_position: difficultyPosition, singularity_position: singularityPosition, revision,
      };
      if (attempt.state === 'rating') {
        this.appendEvent(attemptId, 'rating_submitted', ratingPayload, input.client_wall_at, input.client_monotonic_ms, ratedAt);
        const revealedAt = this.clock();
        this.db.prepare(`UPDATE attempts SET state='complete',rating_submitted_at=?,answer_revealed_at=COALESCE(answer_revealed_at,?),completed_at=?,rating_elapsed_ms=? WHERE id=?`)
          .run(ratedAt, revealedAt, revealedAt, Number.isFinite(input.rating_duration_ms) ? input.rating_duration_ms : null, attemptId);
        if (preReveal) this.appendEvent(attemptId, 'answer_revealed', {}, input.client_wall_at, input.client_monotonic_ms, revealedAt);
      } else {
        this.appendEvent(attemptId, 'rating_revised', ratingPayload, input.client_wall_at, input.client_monotonic_ms, ratedAt);
      }
      return this.attemptDetail(attemptId);
    });
  }

  abandonAttempt(attemptId, input = {}) {
    const attempt = this.db.prepare('SELECT * FROM attempts WHERE id=?').get(attemptId);
    if (!attempt || !['solving', 'rating'].includes(attempt.state)) throw new QaValidationError('No resumable attempt was found.', 409);
    const at = this.clock();
    return transaction(this.db, () => {
      this.db.prepare(`UPDATE attempts SET state='abandoned',completed_at=?,invalidated_at=?,invalidation_reason=? WHERE id=?`)
        .run(at, at, optionalText(input.reason, 'reason', 2_000) ?? 'explicit_abandon', attemptId);
      this.appendEvent(attemptId, 'abandoned', { reason: input.reason ?? 'explicit_abandon' }, input.client_wall_at, input.client_monotonic_ms, at);
      return this.attemptDetail(attemptId);
    });
  }

  recordEvent(attemptId, input) {
    return transaction(this.db, () => this.appendEvent(attemptId, input.event_type, input.payload, input.client_wall_at, input.client_monotonic_ms));
  }

  attemptDetail(attemptId) {
    const attempt = this.db.prepare(`SELECT a.*,i.source,i.source_identifier,r.content_hash AS item_content_hash,
      r.unit_name,r.problem_representation,r.canonical_answer,r.original_source_payload_json
      FROM attempts a JOIN items i ON i.id=a.item_id
      JOIN item_revisions r ON r.item_id=a.item_id AND r.revision_number=a.item_revision_number WHERE a.id=?`).get(attemptId);
    if (!attempt) return null;
    return {
      ...attempt,
      original_source_payload: parseJson(attempt.original_source_payload_json),
      selection: this.db.prepare('SELECT * FROM selection_events WHERE attempt_id=?').get(attemptId) ?? null,
      events: this.db.prepare('SELECT * FROM input_events WHERE attempt_id=? ORDER BY sequence_number').all(attemptId).map((event) => ({ ...event, payload: parseJson(event.payload_json, {}) })),
      evaluations: this.db.prepare('SELECT * FROM evaluations WHERE attempt_id=? ORDER BY revision_number').all(attemptId),
      item_revisions: this.db.prepare('SELECT * FROM item_revisions WHERE item_id=? ORDER BY revision_number').all(attempt.item_id),
      unit_stats: this.unitStats(attempt.unit_name),
    };
  }

  history(filters = {}) {
    if (this.anyActiveAttempt()) throw new QaValidationError('History is locked until the active rating is completed or abandoned.', 423);
    const clauses = [`a.state IN ('complete','abandoned')`];
    const values = [];
    if (filters.unit) { clauses.push('r.unit_name LIKE ?'); values.push(`%${filters.unit}%`); }
    if (filters.date_from) { clauses.push('a.shown_at >= ?'); values.push(filters.date_from); }
    if (filters.date_to) { clauses.push('a.shown_at <= ?'); values.push(`${filters.date_to}T23:59:59.999Z`); }
    if (filters.correctness) { clauses.push('a.correctness = ?'); values.push(filters.correctness); }
    if (filters.difficulty) { clauses.push('e.difficulty_rating = ?'); values.push(Number(filters.difficulty)); }
    if (filters.singularity) { clauses.push('e.singularity_rating = ?'); values.push(Number(filters.singularity)); }
    const rows = this.db.prepare(`SELECT a.*,i.source,i.source_identifier,r.unit_name,r.problem_representation,r.canonical_answer,
      r.content_hash AS item_content_hash,e.difficulty_rating,e.singularity_rating,e.difficulty_position,e.singularity_position,
      e.note,e.confidence,e.revision_number AS evaluation_revision
      FROM attempts a JOIN items i ON i.id=a.item_id
      JOIN item_revisions r ON r.item_id=a.item_id AND r.revision_number=a.item_revision_number
      LEFT JOIN evaluations e ON e.attempt_id=a.id AND e.revision_number=(SELECT MAX(e2.revision_number) FROM evaluations e2 WHERE e2.attempt_id=a.id)
      WHERE ${clauses.join(' AND ')} ORDER BY a.shown_at DESC LIMIT 1000`).all(...values);
    return { rows, summary: summarize(rows) };
  }

  unitObservationCounts() {
    return new Map(this.db.prepare(`SELECT
      json_extract(r.original_source_payload_json,'$.theme.skill_id') AS skill_id,
      COUNT(DISTINCT a.id) AS observation_count
      FROM attempts a
      JOIN item_revisions r ON r.item_id=a.item_id AND r.revision_number=a.item_revision_number
      WHERE a.state='complete' AND a.invalidated_at IS NULL
        AND EXISTS (SELECT 1 FROM evaluations e WHERE e.attempt_id=a.id AND e.invalidated_at IS NULL)
        AND json_extract(r.original_source_payload_json,'$.theme.skill_id') IS NOT NULL
      GROUP BY skill_id`).all().map((row) => [row.skill_id, row.observation_count]));
  }

  unitSamplingObservations(skillId) {
    return this.db.prepare(`SELECT r.original_source_payload_json,
      e.difficulty_rating,e.singularity_rating,e.difficulty_position,e.singularity_position
      FROM attempts a
      JOIN item_revisions r ON r.item_id=a.item_id AND r.revision_number=a.item_revision_number
      JOIN evaluations e ON e.attempt_id=a.id
        AND e.revision_number=(SELECT MAX(e2.revision_number) FROM evaluations e2 WHERE e2.attempt_id=a.id AND e2.invalidated_at IS NULL)
      WHERE a.state='complete' AND a.invalidated_at IS NULL AND e.invalidated_at IS NULL
        AND json_extract(r.original_source_payload_json,'$.theme.skill_id')=?
      ORDER BY a.shown_at`).all(skillId).map((row) => ({
        original_source_payload: parseJson(row.original_source_payload_json),
        difficulty_rating: row.difficulty_rating,
        singularity_rating: row.singularity_rating,
        difficulty_position: row.difficulty_position,
        singularity_position: row.singularity_position,
      }));
  }

  unitStats(unitName) {
    const rows = this.db.prepare(`SELECT a.correctness,a.answer_elapsed_ms,e.difficulty_rating,e.singularity_rating
      FROM attempts a JOIN item_revisions r ON r.item_id=a.item_id AND r.revision_number=a.item_revision_number
      JOIN evaluations e ON e.attempt_id=a.id AND e.revision_number=(SELECT MAX(e2.revision_number) FROM evaluations e2 WHERE e2.attempt_id=a.id)
      WHERE a.state='complete' AND a.invalidated_at IS NULL AND r.unit_name=?`).all(unitName);
    return { unit_name: unitName, ...summarize(rows), points: rows.map((row) => ({ difficulty: row.difficulty_rating, singularity: row.singularity_rating })) };
  }

  fullExport() {
    if (this.anyActiveAttempt()) throw new QaValidationError('Export is locked until the active rating is completed or abandoned.', 423);
    const tables = ['schema_migrations','qa_sessions','items','item_revisions','queue_entries','attempts','selection_events','input_events','evaluations','change_audit','model_runs','derived_results'];
    return {
      manifest: { export_schema_version: EXPORT_SCHEMA_VERSION, qa_schema_version: QA_SCHEMA_VERSION, exported_at: this.clock(), application_version: APP_VERSION, autodrill_git_sha: this.gitSha, autodrill_git_state: this.gitState, format: 'autodrill-qa-full-json-v1' },
      data: Object.fromEntries(tables.map((table) => [table, this.db.prepare(`SELECT * FROM ${table}`).all()])),
    };
  }

  analysisCsv() {
    const exportedAt = this.clock();
    const rows = this.history({}).rows;
    const columns = ['export_schema_version','qa_schema_version','exported_at','attempt_id','session_id','evaluator','item_id','item_revision_number','source','source_identifier','unit_name','problem_representation','canonical_answer','observation_mode','raw_user_answer','normalized_user_answer','correctness','outcome','exposure_count','shown_at','first_interaction_at','answer_started_at','submitted_at','rating_started_at','rating_submitted_at','answer_revealed_at','completed_at','answer_elapsed_ms','rating_elapsed_ms','difficulty_rating','singularity_rating','difficulty_position','singularity_position','evaluation_revision','note','browser_version','application_version','autodrill_git_sha','autodrill_git_state_json','git_worktree_state','git_worktree_dirty','git_status_sha256','git_tracked_diff_sha256'];
    const sessionById = new Map(this.listSessions().map((session) => [session.id, session]));
    const lines = [columns.join(',')];
    for (const row of rows) {
      const gitState = parseJson(row.autodrill_git_state_json, {});
      const flat = {
        export_schema_version: EXPORT_SCHEMA_VERSION, qa_schema_version: QA_SCHEMA_VERSION, exported_at: exportedAt,
        attempt_id: row.id, evaluator: sessionById.get(row.session_id)?.evaluator, ...row,
        git_worktree_state: gitState.worktree_state,
        git_worktree_dirty: gitState.worktree_dirty,
        git_status_sha256: gitState.status_sha256,
        git_tracked_diff_sha256: gitState.tracked_diff_sha256,
      };
      lines.push(columns.map((column) => csv(flat[column])).join(','));
    }
    return lines.join('\n');
  }

  audit(entityType, entityId, action, before, after, actor = 'User', reason = null) {
    this.db.prepare(`INSERT INTO change_audit(id,entity_type,entity_id,action,changed_at,actor,before_json,after_json,reason)
      VALUES (?,?,?,?,?,?,?,?,?)`).run(randomUUID(), entityType, entityId, action, this.clock(), actor, before == null ? null : json(before), json(after), reason);
  }
}

function median(values) {
  const sorted = values.filter(Number.isFinite).sort((a, b) => a - b);
  if (!sorted.length) return null;
  const middle = Math.floor(sorted.length / 2);
  return sorted.length % 2 ? sorted[middle] : (sorted[middle - 1] + sorted[middle]) / 2;
}
function summarize(rows) {
  const rated = rows.filter((row) => row.difficulty_rating != null);
  const graded = rows.filter((row) => ['correct', 'incorrect'].includes(row.correctness));
  const counts = {};
  for (const row of rated) counts[`${row.difficulty_rating},${row.singularity_rating}`] = (counts[`${row.difficulty_rating},${row.singularity_rating}`] ?? 0) + 1;
  return {
    sample_count: rows.length,
    rated_count: rated.length,
    correctness_rate: graded.length ? graded.filter((row) => row.correctness === 'correct').length / graded.length : null,
    median_response_ms: median(rows.map((row) => row.answer_elapsed_ms)),
    median_difficulty: median(rated.map((row) => row.difficulty_rating)),
    median_singularity: median(rated.map((row) => row.singularity_rating)),
    cell_counts: counts,
  };
}
function csv(value) {
  if (value == null) return '';
  const string = String(value);
  return /[",\n\r]/.test(string) ? `"${string.replaceAll('"', '""')}"` : string;
}
