ALTER TABLE evaluations
ADD COLUMN difficulty_position REAL
CHECK (difficulty_position IS NULL OR difficulty_position BETWEEN 0.0 AND 1.0);

ALTER TABLE evaluations
ADD COLUMN singularity_position REAL
CHECK (singularity_position IS NULL OR singularity_position BETWEEN 0.0 AND 1.0);
