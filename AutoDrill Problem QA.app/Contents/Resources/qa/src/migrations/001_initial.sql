CREATE TABLE qa_sessions (
  id TEXT PRIMARY KEY,
  evaluator TEXT NOT NULL,
  started_at TEXT NOT NULL,
  ended_at TEXT,
  local_timezone TEXT NOT NULL,
  application_version TEXT NOT NULL,
  qa_schema_version INTEGER NOT NULL,
  autodrill_git_sha TEXT NOT NULL,
  note TEXT,
  invalidated_at TEXT,
  invalidation_reason TEXT
) STRICT;

CREATE TABLE items (
  id TEXT PRIMARY KEY,
  source TEXT NOT NULL CHECK (source IN ('manual', 'autodrill', 'imported', 'other')),
  source_identifier TEXT,
  content_hash TEXT NOT NULL,
  created_at TEXT NOT NULL,
  current_revision_number INTEGER NOT NULL DEFAULT 1,
  invalidated_at TEXT,
  invalidation_reason TEXT
) STRICT;

CREATE INDEX items_content_hash_idx ON items(content_hash);

CREATE TABLE item_revisions (
  id TEXT PRIMARY KEY,
  item_id TEXT NOT NULL REFERENCES items(id),
  revision_number INTEGER NOT NULL,
  unit_name TEXT NOT NULL,
  problem_representation TEXT NOT NULL,
  canonical_answer TEXT NOT NULL,
  content_hash TEXT NOT NULL,
  original_source_payload_json TEXT,
  revision_reason TEXT NOT NULL,
  created_at TEXT NOT NULL,
  UNIQUE(item_id, revision_number)
) STRICT;

CREATE TABLE queue_entries (
  id TEXT PRIMARY KEY,
  item_id TEXT NOT NULL REFERENCES items(id),
  added_at TEXT NOT NULL,
  position INTEGER NOT NULL,
  state TEXT NOT NULL DEFAULT 'queued' CHECK (state IN ('queued', 'disabled')),
  invalidated_at TEXT
) STRICT;

CREATE INDEX queue_entries_order_idx ON queue_entries(state, position, added_at);

CREATE TABLE attempts (
  id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL REFERENCES qa_sessions(id),
  item_id TEXT NOT NULL REFERENCES items(id),
  item_revision_number INTEGER NOT NULL,
  exposure_count INTEGER NOT NULL,
  state TEXT NOT NULL CHECK (state IN ('solving', 'rating', 'complete', 'abandoned')),
  outcome TEXT CHECK (outcome IS NULL OR outcome IN ('answered', 'unable_to_solve', 'broken_unrateable', 'skipped')),
  shown_at TEXT NOT NULL,
  first_interaction_at TEXT,
  answer_started_at TEXT,
  submitted_at TEXT,
  rating_started_at TEXT,
  rating_submitted_at TEXT,
  answer_revealed_at TEXT,
  completed_at TEXT,
  raw_user_answer TEXT,
  normalized_user_answer TEXT,
  correctness TEXT CHECK (correctness IS NULL OR correctness IN ('correct', 'incorrect', 'ungraded')),
  grading_method TEXT,
  answer_elapsed_ms REAL,
  rating_elapsed_ms REAL,
  browser_version TEXT,
  application_version TEXT NOT NULL,
  qa_schema_version INTEGER NOT NULL,
  autodrill_git_sha TEXT NOT NULL,
  invalidated_at TEXT,
  invalidation_reason TEXT,
  FOREIGN KEY(item_id, item_revision_number) REFERENCES item_revisions(item_id, revision_number)
) STRICT;

CREATE INDEX attempts_session_idx ON attempts(session_id, shown_at DESC);
CREATE INDEX attempts_item_idx ON attempts(item_id, exposure_count);
CREATE UNIQUE INDEX one_open_attempt_per_session_idx
  ON attempts(session_id) WHERE state IN ('solving', 'rating');

CREATE TABLE selection_events (
  id TEXT PRIMARY KEY,
  attempt_id TEXT NOT NULL UNIQUE REFERENCES attempts(id),
  selected_at TEXT NOT NULL,
  selection_policy TEXT NOT NULL,
  candidate_source TEXT NOT NULL,
  filters_json TEXT NOT NULL,
  random_seed TEXT,
  candidate_item_ids_json TEXT NOT NULL,
  model_name TEXT,
  model_version TEXT,
  candidate_scores_json TEXT,
  selection_probability REAL
) STRICT;

CREATE TABLE input_events (
  id TEXT PRIMARY KEY,
  attempt_id TEXT NOT NULL REFERENCES attempts(id),
  sequence_number INTEGER NOT NULL,
  event_type TEXT NOT NULL,
  occurred_at TEXT NOT NULL,
  client_wall_at TEXT,
  client_monotonic_ms REAL,
  payload_json TEXT NOT NULL,
  UNIQUE(attempt_id, sequence_number)
) STRICT;

CREATE INDEX input_events_attempt_idx ON input_events(attempt_id, sequence_number);

CREATE TABLE evaluations (
  id TEXT PRIMARY KEY,
  attempt_id TEXT NOT NULL REFERENCES attempts(id),
  revision_number INTEGER NOT NULL,
  difficulty_rating INTEGER NOT NULL CHECK (difficulty_rating BETWEEN 1 AND 7),
  singularity_rating INTEGER NOT NULL CHECK (singularity_rating BETWEEN 1 AND 7),
  rating_scale_version INTEGER NOT NULL,
  rated_at TEXT NOT NULL,
  rating_duration_ms REAL,
  confidence INTEGER CHECK (confidence IS NULL OR confidence BETWEEN 1 AND 5),
  note TEXT,
  pre_answer_reveal INTEGER NOT NULL CHECK (pre_answer_reveal IN (0, 1)),
  supersedes_evaluation_id TEXT REFERENCES evaluations(id),
  invalidated_at TEXT,
  invalidation_reason TEXT,
  UNIQUE(attempt_id, revision_number)
) STRICT;

CREATE INDEX evaluations_attempt_idx ON evaluations(attempt_id, revision_number);

CREATE TABLE change_audit (
  id TEXT PRIMARY KEY,
  entity_type TEXT NOT NULL,
  entity_id TEXT NOT NULL,
  action TEXT NOT NULL,
  changed_at TEXT NOT NULL,
  actor TEXT NOT NULL,
  before_json TEXT,
  after_json TEXT NOT NULL,
  reason TEXT
) STRICT;
