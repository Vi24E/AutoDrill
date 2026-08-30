export const APP_VERSION = '0.4.0';
export const QA_SCHEMA_VERSION = 4;
export const EXPORT_SCHEMA_VERSION = 1;

export const RATING_SCALE = Object.freeze({
  version: 1,
  min: 1,
  max: 7,
  axes: Object.freeze({
    difficulty: Object.freeze({
      label: '難しさ',
      anchors: Object.freeze({ 1: '非常に易しい', 4: '中程度', 7: '非常に難しい' }),
    }),
    singularity: Object.freeze({
      label: '特異性',
      anchors: Object.freeze({ 1: '非常に典型的', 4: 'やや特徴的', 7: '非常に珍しい・特異' }),
    }),
  }),
});

export const ATTEMPT_OUTCOMES = Object.freeze([
  'answered',
  'unable_to_solve',
  'broken_unrateable',
  'skipped',
]);

export const INPUT_EVENT_TYPES = new Set([
  'shown', 'answer_focused', 'answer_blurred', 'first_input', 'answer_started',
  'answer_changed', 'answer_cleared', 'submit', 'unable_to_solve',
  'broken_unrateable', 'skip', 'rating_started', 'rating_selected',
  'rating_revised', 'rating_submitted', 'answer_revealed', 'visibility_hidden',
  'visibility_visible', 'window_blurred', 'window_focused', 'resumed', 'abandoned',
]);

export function assertRating(value, axis) {
  if (!Number.isInteger(value) || value < RATING_SCALE.min || value > RATING_SCALE.max) {
    throw new QaValidationError(`${axis} must be an integer from ${RATING_SCALE.min} to ${RATING_SCALE.max}.`);
  }
}

export class QaValidationError extends Error {
  constructor(message, status = 400) {
    super(message);
    this.name = 'QaValidationError';
    this.status = status;
  }
}
