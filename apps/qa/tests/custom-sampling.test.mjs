import assert from 'node:assert/strict';
import test from 'node:test';
import { scoreInformationCandidates } from '../src/custom-sampling.mjs';

test('custom information sampler prioritizes an operation-vector direction absent from observations', () => {
  const observed = [
    { effort: 10, operation_vector: [2, 0] },
    { effort: 12, operation_vector: [3, 0] },
  ];
  const scored = scoreInformationCandidates({
    observed,
    candidates: [
      { id: 'covered', effort: 11, operation_vector: [2, 0] },
      { id: 'uncovered', effort: 11, operation_vector: [0, 2] },
    ],
  });
  const byId = new Map(scored.map((candidate) => [candidate.id, candidate.information_score]));
  assert.ok(byId.get('uncovered') > byId.get('covered'));
});

test('custom information sampler detects an under-covered correlated direction', () => {
  const observed = Array.from({ length: 8 }, (_, index) => ({
    effort: index + 1,
    operation_vector: [index + 1, index + 1],
  }));
  const scored = scoreInformationCandidates({
    observed,
    candidates: [
      { id: 'same-line', effort: 9, operation_vector: [2, 2] },
      { id: 'off-line', effort: 9, operation_vector: [4, 0] },
    ],
  });
  const byId = new Map(scored.map((candidate) => [candidate.id, candidate.information_score]));
  assert.ok(byId.get('off-line') > byId.get('same-line'));
});

test('custom information sampler has a scalar-effort fallback for theme-specific models', () => {
  const scored = scoreInformationCandidates({
    observed: [{ effort: 1, operation_vector: null }],
    candidates: [
      { id: 'near', effort: 1, operation_vector: null },
      { id: 'far', effort: 4, operation_vector: null },
    ],
  });
  assert.equal(scored.length, 2);
  assert.ok(scored.every((candidate) => Number.isFinite(candidate.information_score)));
});
