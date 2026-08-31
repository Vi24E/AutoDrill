ALTER TABLE attempts
ADD COLUMN observation_mode TEXT NOT NULL DEFAULT 'answer_then_rating'
CHECK (observation_mode IN ('answer_then_rating', 'rating_only_answer_shown'));
