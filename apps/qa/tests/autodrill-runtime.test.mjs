import assert from 'node:assert/strict';
import test from 'node:test';
import { AutoDrillRuntime } from '../src/autodrill-runtime.mjs';

test('AutoDrill runtime generates a provenance-complete random item and grades with Rust WASM', async () => {
  const runtime = new AutoDrillRuntime({ selectionSeed: () => 'runtime-test-selection' });
  const generated = await runtime.generateRandomProblem();
  const payload = generated.item.original_source_payload;

  assert.equal(generated.item.source, 'autodrill');
  assert.equal(payload.integration_version, 'autodrill_qa_wasm_v1');
  assert.equal(payload.selection_seed, 'runtime-test-selection');
  assert.equal(payload.generation_request.difficulty, 4);
  assert.equal(payload.problem, payload.worksheet.problems[payload.problem_index]);
  assert.equal(generated.selection.selection_policy, 'autodrill_random_v1');
  assert.ok(generated.selection.selection_probability > 0);

  const grading = await runtime.gradeAnswer(payload, generated.item.canonical_answer.replaceAll('−', '-'));
  assert.equal(grading.correctness, 'correct');
  assert.equal(grading.grading_method, 'autodrill_wasm_grade_v1');
  assert.ok(grading.raw_result.parsed);
  assert.ok(grading.raw_result.graded);
});
