ALTER TABLE qa_sessions
ADD COLUMN autodrill_git_state_json TEXT NOT NULL
DEFAULT '{"source":"legacy","head_sha":"unknown","worktree_state":"unknown","worktree_dirty":null}'
CHECK (json_valid(autodrill_git_state_json));

ALTER TABLE attempts
ADD COLUMN autodrill_git_state_json TEXT NOT NULL
DEFAULT '{"source":"legacy","head_sha":"unknown","worktree_state":"unknown","worktree_dirty":null}'
CHECK (json_valid(autodrill_git_state_json));
