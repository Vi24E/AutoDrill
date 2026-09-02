import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { AutoDrillRuntime, formatProblem } from '../src/autodrill-runtime.mjs';

const contract = JSON.parse(readFileSync(new URL('../generated/drill-core-contract.json', import.meta.url), 'utf8'));
const EXCLUDED_QA_CURRICULUM_UNITS = new Set([
  'grade1-addition',
  'grade1-subtraction',
  'multiplication-table',
  'division-table',
]);


test('QA problem formatting renders typed equation surfaces without legacy coefficient inference', () => {
  const shiftedQuadratic = {
    prompt: {
      kind: 'quadratic_equation',
      equation: {
        left: {
          kind: 'subtract',
          left: {
            kind: 'scale',
            factor: { kind: 'integer', value: 2 },
            expression: {
              kind: 'square',
              expression: {
                kind: 'add',
                left: { kind: 'variable', variable: 'x' },
                right: { kind: 'constant', value: { kind: 'integer', value: 3 } },
              },
            },
          },
          right: { kind: 'linear', expression: { kind: 'constant', value: { kind: 'integer', value: 8 } } },
        },
        right: { kind: 'linear', expression: { kind: 'constant', value: { kind: 'integer', value: 0 } } },
      },
      solve_method: 'square_root',
    },
  };
  assert.equal(formatProblem(shiftedQuadratic), '2(x + 3)² − 8 = 0');

  const simultaneous = {
    prompt: {
      kind: 'simultaneous_equation',
      equations: [
        {
          left: { kind: 'add', left: { kind: 'variable', variable: 'x' }, right: { kind: 'variable', variable: 'y' } },
          right: { kind: 'constant', value: { kind: 'integer', value: 5 } },
        },
        {
          left: { kind: 'subtract', left: { kind: 'variable', variable: 'x' }, right: { kind: 'variable', variable: 'y' } },
          right: { kind: 'constant', value: { kind: 'integer', value: 1 } },
        },
      ],
      solve_method: 'elimination',
    },
  };
  assert.equal(formatProblem(simultaneous), 'x + y = 5\nx − y = 1');
});

test('AutoDrill runtime exposes every non-excluded unit through the original WASM generator', async () => {
  const runtime = new AutoDrillRuntime({ selectionSeed: () => 'runtime-test-selection' });
  const units = runtime.listUnits();
  const expectedThemes = Object.values(contract.themes)
    .filter((theme) => !EXCLUDED_QA_CURRICULUM_UNITS.has(theme.curriculum_unit.key));
  assert.deepEqual(
    units.map((unit) => unit.skill_id).sort(),
    expectedThemes.map((theme) => theme.skill_id).sort(),
  );
  assert.ok(units.some((unit) => unit.skill_id === 'jp.grade5.fraction.addition.summary'));
  assert.ok(units.some((unit) => unit.skill_id === 'jp.grade6.fraction.division'));
  assert.ok(units.some((unit) => unit.skill_id === 'jp.grade7.equation.linear.1'));
  assert.ok(units.some((unit) => unit.skill_id === 'jp.grade9.equation.quadratic.3'));
  assert.ok(units.some((unit) => unit.skill_id === 'jp.grade9.equation.quadratic.4'));
  assert.ok(units.some((unit) => unit.skill_id === 'bonus.logic.mini_sudoku'));

  const generatedBySkill = new Map();
  for (const unit of units) {
    const unitProblem = await runtime.generateRandomProblem({ skillId: unit.skill_id });
    assert.equal(unitProblem.item.unit_name, unit.name);
    assert.ok(unitProblem.item.problem_representation.length > 0, unit.name);
    assert.ok(unitProblem.item.canonical_answer.length > 0, unit.name);
    assert.equal(unitProblem.item.original_source_payload.theme.skill_id, unit.skill_id);
    generatedBySkill.set(unit.skill_id, unitProblem);
  }

  const generated = generatedBySkill.get('jp.grade5.fraction.addition.summary');
  const payload = generated.item.original_source_payload;

  assert.equal(generated.item.source, 'autodrill');
  assert.equal(generated.item.unit_name, '分数の足し算（まとめ）');
  assert.match(generated.item.problem_representation, /\//);
  assert.match(generated.item.canonical_answer, /\//);
  assert.equal(payload.integration_version, 'autodrill_qa_wasm_v1');
  assert.equal(payload.selection_seed, 'runtime-test-selection');
  assert.equal(payload.generation_request.difficulty, 4);
  assert.equal(payload.problem, payload.worksheet.problems[payload.problem_index]);
  assert.equal(generated.selection.selection_policy, 'autodrill_unit_random_v1');
  assert.equal(generated.selection.filters.selected_skill_id, 'jp.grade5.fraction.addition.summary');
  const expectedExcludedSkillIds = Object.values(contract.themes)
    .filter((theme) => EXCLUDED_QA_CURRICULUM_UNITS.has(theme.curriculum_unit.key))
    .map((theme) => theme.skill_id)
    .sort();
  assert.deepEqual([...generated.selection.filters.excluded_skill_ids].sort(), expectedExcludedSkillIds);
  assert.equal(generated.selection.candidate_count, payload.worksheet.problems.length);
  assert.ok(generated.selection.selection_probability > 0);

  const nextFraction = await runtime.generateRandomProblem({ skillId: 'jp.grade5.fraction.addition.summary' });
  assert.equal(nextFraction.item.original_source_payload.worksheet.identity.seed, payload.worksheet.identity.seed);
  assert.notEqual(nextFraction.item.original_source_payload.problem_index, payload.problem_index);
  assert.equal(nextFraction.selection.candidate_count, payload.worksheet.problems.length - 1);

  const linear = generatedBySkill.get('jp.grade7.equation.linear.1');
  const custom = await runtime.generateProblem({
    skillId: 'jp.grade7.equation.linear.1',
    samplingMode: 'custom',
    observations: [{
      original_source_payload: linear.item.original_source_payload,
      difficulty_position: 0.5,
      singularity_position: 0.5,
    }],
  });
  assert.equal(custom.selection.selection_policy, 'autodrill_unit_custom_v1');
  assert.equal(custom.selection.model_name, 'operation_vector_information');
  assert.equal(custom.selection.model_version, '1');
  assert.equal(custom.selection.selection_probability, null);
  assert.ok(custom.selection.candidate_count > 0);
  assert.ok(custom.selection.candidate_scores.length > 0);
  assert.equal(custom.item.original_source_payload.qa_sampling.mode, 'custom');
  assert.ok(Number.isFinite(custom.item.original_source_payload.qa_sampling.effort));
  assert.ok(Array.isArray(custom.item.original_source_payload.qa_sampling.operation_vector_basis));
  assert.ok(custom.item.original_source_payload.qa_sampling.operation_vector_basis.length > 0);
  assert.ok(Array.isArray(custom.item.original_source_payload.qa_sampling.operation_vector));
  assert.deepEqual(custom.selection.filters.operation_vector_basis, custom.item.original_source_payload.qa_sampling.operation_vector_basis);
  assert.ok(custom.selection.candidate_scores.every((candidate) => Array.isArray(candidate.operation_vector)));

  const numeric = generatedBySkill.get('jp.grade2.addition.two_digit');
  const grading = await runtime.gradeAnswer(numeric.item.original_source_payload, numeric.item.canonical_answer.replaceAll('−', '-'));
  assert.equal(grading.correctness, 'correct');
  assert.equal(grading.grading_method, 'autodrill_wasm_grade_v1');
  assert.ok(grading.raw_result.parsed);
  assert.ok(grading.raw_result.graded);
});
