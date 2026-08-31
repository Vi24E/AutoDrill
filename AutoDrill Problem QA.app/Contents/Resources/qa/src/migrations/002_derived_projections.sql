CREATE TABLE model_runs (
  id TEXT PRIMARY KEY,
  model_name TEXT NOT NULL,
  model_version TEXT NOT NULL,
  code_git_sha TEXT NOT NULL,
  generated_at TEXT NOT NULL,
  training_data_cutoff TEXT NOT NULL,
  configuration_json TEXT NOT NULL,
  posterior_summary_json TEXT NOT NULL,
  diagnostics_json TEXT NOT NULL,
  invalidated_at TEXT
) STRICT;

CREATE TABLE derived_results (
  id TEXT PRIMARY KEY,
  model_run_id TEXT NOT NULL REFERENCES model_runs(id),
  entity_type TEXT NOT NULL,
  entity_id TEXT NOT NULL,
  result_json TEXT NOT NULL,
  generated_at TEXT NOT NULL,
  UNIQUE(model_run_id, entity_type, entity_id)
) STRICT;
