import assert from 'node:assert/strict';
import test from 'node:test';
import { AutoDrillRuntime } from '../src/autodrill-runtime.mjs';

test('AutoDrill runtime exposes every non-excluded unit through the original WASM generator', async () => {
  const runtime = new AutoDrillRuntime({ selectionSeed: () => 'runtime-test-selection' });
  const units = runtime.listUnits();
  assert.equal(units.length, 34);
  assert.ok(units.some((unit) => unit.skill_id === 'jp.grade5.fraction.addition'));
  assert.ok(units.some((unit) => unit.skill_id === 'jp.grade6.fraction.division'));
  assert.ok(units.some((unit) => unit.skill_id === 'jp.grade7.equation.linear.1'));
  assert.ok(units.some((unit) => unit.skill_id === 'jp.grade9.equation.quadratic.3'));
  assert.ok(units.some((unit) => unit.skill_id === 'bonus.logic.mini_sudoku'));
  assert.ok(!units.some((unit) => unit.skill_id === 'jp.grade1.addition.one_digit'));
  assert.ok(!units.some((unit) => unit.skill_id === 'jp.grade1.subtraction.one_digit'));
  assert.ok(!units.some((unit) => unit.skill_id === 'jp.grade2.multiplication.table'));
  assert.ok(!units.some((unit) => unit.skill_id === 'jp.grade3.division.table.1'));

  const generatedBySkill = new Map();
  for (const unit of units) {
    const unitProblem = await runtime.generateRandomProblem({ skillId: unit.skill_id });
    assert.equal(unitProblem.item.unit_name, unit.name);
    assert.ok(unitProblem.item.problem_representation.length > 0, unit.name);
    assert.ok(unitProblem.item.canonical_answer.length > 0, unit.name);
    assert.equal(unitProblem.item.original_source_payload.theme.skill_id, unit.skill_id);
    generatedBySkill.set(unit.skill_id, unitProblem);
  }

  const generated = generatedBySkill.get('jp.grade5.fraction.addition');
  const payload = generated.item.original_source_payload;

  assert.equal(generated.item.source, 'autodrill');
  assert.equal(generated.item.unit_name, '分数の足し算');
  assert.match(generated.item.problem_representation, /\//);
  assert.match(generated.item.canonical_answer, /\//);
  assert.equal(payload.integration_version, 'autodrill_qa_wasm_v1');
  assert.equal(payload.selection_seed, 'runtime-test-selection');
  assert.equal(payload.generation_request.difficulty, 4);
  assert.equal(payload.problem, payload.worksheet.problems[payload.problem_index]);
  assert.equal(generated.selection.selection_policy, 'autodrill_unit_random_v1');
  assert.equal(generated.selection.filters.selected_skill_id, 'jp.grade5.fraction.addition');
  assert.equal(generated.selection.candidate_count, payload.worksheet.problems.length);
  assert.ok(generated.selection.selection_probability > 0);

  const nextFraction = await runtime.generateRandomProblem({ skillId: 'jp.grade5.fraction.addition' });
  assert.equal(nextFraction.item.original_source_payload.worksheet.identity.seed, payload.worksheet.identity.seed);
  assert.notEqual(nextFraction.item.original_source_payload.problem_index, payload.problem_index);
  assert.equal(nextFraction.selection.candidate_count, payload.worksheet.problems.length - 1);

  const numeric = generatedBySkill.get('jp.grade2.addition.two_digit');
  const grading = await runtime.gradeAnswer(numeric.item.original_source_payload, numeric.item.canonical_answer.replaceAll('−', '-'));
  assert.equal(grading.correctness, 'correct');
  assert.equal(grading.grading_method, 'autodrill_wasm_grade_v1');
  assert.ok(grading.raw_result.parsed);
  assert.ok(grading.raw_result.graded);
});
