import { createHash, randomUUID } from 'node:crypto';
import { existsSync, readFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { CUSTOM_SAMPLING_PROFILE, scoreInformationCandidates } from './custom-sampling.mjs';

const CONTRACT_PATH = new URL('../generated/drill-core-contract.json', import.meta.url);
const EXCLUDED_QA_CURRICULUM_UNITS = new Set([
  'grade1-addition',
  'grade1-subtraction',
  'multiplication-table',
  'division-table',
]);
const SEED_ALPHABET = 'ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz23456789';
const CUSTOM_CANDIDATE_WORKSHEETS = 4;
const MAX_DIAGNOSTIC_CACHE_ENTRIES = 128;

function digestNumber(seed, label) {
  return createHash('sha256').update(`${label}:${seed}`).digest().readUInt32BE(0);
}

function generatorSeed(selectionSeed) {
  const bytes = createHash('sha256').update(`generator:${selectionSeed}`).digest();
  return Array.from(bytes.subarray(0, 4), (value) => SEED_ALPHABET[value % SEED_ALPHABET.length]).join('');
}

function exactDecimal(coefficient, scale) {
  const raw = String(coefficient);
  const negative = raw.startsWith('-');
  const digits = (negative ? raw.slice(1) : raw).padStart(scale + 1, '0');
  const value = scale ? `${digits.slice(0, -scale)}.${digits.slice(-scale)}` : digits;
  return negative ? `−${value}` : value;
}

function rational(value) {
  const numerator = String(value.numerator).replace(/^-/, '−');
  return value.denominator === 1 ? numerator : `${numerator}/${value.denominator}`;
}

function expression(node) {
  if (node.kind === 'integer') return String(node.value);
  if (node.kind === 'rational') return rational(node.value);
  if (node.kind === 'exact_decimal') return exactDecimal(node.coefficient, node.scale);
  if (node.kind === 'binary') {
    const operator = { add: '+', subtract: '−', multiply: '×', divide: '÷' }[node.operator] ?? node.operator;
    return `(${expression(node.left)} ${operator} ${expression(node.right)})`;
  }
  throw new Error(`Unsupported arithmetic expression: ${node.kind}`);
}

export function formatProblem(problem) {
  const prompt = problem.prompt;
  if (prompt.kind === 'addition') return `${prompt.left} + ${prompt.right} =`;
  if (prompt.kind === 'arithmetic') return `${expression(prompt.expression).replace(/^[(]|[)]$/g, '')} =`;
  if (prompt.kind === 'column_arithmetic') {
    const operator = { add: '+', subtract: '−', multiply: '×', divide: '÷' }[prompt.operator] ?? prompt.operator;
    return `${expression(prompt.left)} ${operator} ${expression(prompt.right)} =`;
  }
  if (prompt.kind === 'linear_equation') {
    return `${linearExpression(prompt.left)} = ${linearExpression(prompt.right)}`;
  }
  if (prompt.kind === 'quadratic_equation') return quadraticEquation(prompt);
  if (prompt.kind === 'simultaneous_equation') {
    return prompt.equations.map((equation) => `${linearExpression(equation.left)} = ${linearExpression(equation.right)}`).join('\n');
  }
  if (prompt.kind === 'liar_puzzle') {
    return prompt.statements.map((statement, index) => `${personLabel(index + 1)}さん「${liarStatement(statement)}」`).join('\n');
  }
  if (prompt.kind === 'mini_sudoku') return `4×4 数独\n${formatGrid(prompt.givens)}`;
  throw new Error(`Unsupported QA prompt kind: ${prompt.kind}`);
}

function formatAnswerNode(answer) {
  if (answer.type === 'empty') return '';
  if (answer.type === 'integer') return String(answer.value).replace(/^-/, '−');
  if (answer.type === 'exact_decimal') return exactDecimal(answer.value.coefficient, answer.value.scale);
  if (answer.type === 'nan_error') return answer.value;
  if (answer.type === 'negative') return `−${formatAnswerNode(answer.value)}`;
  if (answer.type === 'fraction') return `${formatAnswerNode(answer.value.numerator)}/${formatAnswerNode(answer.value.denominator)}`;
  if (answer.type === 'mixed_fraction') return `${formatAnswerNode(answer.value.whole)} ${formatAnswerNode(answer.value.numerator)}/${formatAnswerNode(answer.value.denominator)}`;
  if (answer.type === 'root') return answer.value.index ? `${formatAnswerNode(answer.value.index)}√(${formatAnswerNode(answer.value.radicand)})` : `√(${formatAnswerNode(answer.value.radicand)})`;
  if (answer.type === 'plus_minus') return `±${formatAnswerNode(answer.value)}`;
  if (answer.type === 'binary') {
    const operator = { add: '+', subtract: '−', multiply: '×' }[answer.value.operator] ?? answer.value.operator;
    return `(${formatAnswerNode(answer.value.left)} ${operator} ${formatAnswerNode(answer.value.right)})`;
  }
  if (answer.type === 'tuple') return answer.value.map(formatAnswerNode).join(', ');
  if (answer.type === 'variable') return answer.value;
  throw new Error(`Unsupported canonical answer type: ${answer.type}`);
}

export function formatCanonicalAnswer(answer, problem) {
  if (answer.type === 'tuple' && problem?.prompt.kind === 'simultaneous_equation') {
    return `x = ${formatAnswerNode(answer.value[0])}, y = ${formatAnswerNode(answer.value[1])}`;
  }
  if (answer.type === 'tuple' && problem?.prompt.kind === 'liar_puzzle') {
    return answer.value.map((value) => `${personLabel(Number(formatAnswerNode(value)))}さん`).join('、');
  }
  if (answer.type === 'tuple' && problem?.prompt.kind === 'mini_sudoku') {
    return formatGrid(answer.value.map((value) => Number(formatAnswerNode(value))));
  }
  if (answer.type === 'tuple' && problem?.prompt.kind === 'column_arithmetic' && problem.prompt.operator === 'divide') {
    return `${formatAnswerNode(answer.value[0])} あまり ${formatAnswerNode(answer.value[1])}`;
  }
  if (answer.type === 'tuple' && problem?.prompt.kind === 'quadratic_equation') {
    return `x = ${answer.value.map(formatAnswerNode).join(', ')}`;
  }
  return formatAnswerNode(answer);
}

function linearScalar(value, omitOne = false) {
  let text;
  if (value.kind === 'integer') text = String(value.value).replace(/^-/, '−');
  else if (value.kind === 'fraction') text = rational(value.value);
  else if (value.kind === 'exact_decimal') text = exactDecimal(value.coefficient, value.scale);
  else throw new Error(`Unsupported linear scalar: ${value.kind}`);
  if (omitOne && text === '1') return '';
  if (omitOne && text === '−1') return '−';
  return text;
}

function linearExpression(node) {
  if (node.kind === 'variable') return node.variable;
  if (node.kind === 'constant') return linearScalar(node.value);
  if (node.kind === 'add') return `${linearExpression(node.left)} + ${linearExpression(node.right)}`;
  if (node.kind === 'subtract') return `${linearExpression(node.left)} − ${linearExpression(node.right)}`;
  if (node.kind === 'scale') {
    const body = linearExpression(node.expression);
    const grouped = node.expression.kind === 'add' || node.expression.kind === 'subtract' ? `(${body})` : body;
    return `${linearScalar(node.factor, true)}${grouped}`;
  }
  throw new Error(`Unsupported linear expression: ${node.kind}`);
}

function quadraticExpression(node) {
  if (node.kind === 'linear') return linearExpression(node.expression);
  if (node.kind === 'square') {
    const body = linearExpression(node.expression);
    return node.expression.kind === 'variable' ? `${body}²` : `(${body})²`;
  }
  if (node.kind === 'add') return `${quadraticExpression(node.left)} + ${quadraticExpression(node.right)}`;
  if (node.kind === 'subtract') return `${quadraticExpression(node.left)} − ${quadraticExpression(node.right)}`;
  if (node.kind === 'scale') {
    const body = quadraticExpression(node.expression);
    const grouped = node.expression.kind === 'add' || node.expression.kind === 'subtract' ? `(${body})` : body;
    return `${linearScalar(node.factor, true)}${grouped}`;
  }
  throw new Error(`Unsupported quadratic expression: ${node.kind}`);
}

function quadraticEquation(prompt) {
  return `${quadraticExpression(prompt.equation.left)} = ${quadraticExpression(prompt.equation.right)}`;
}

function personLabel(person) { return String.fromCharCode('A'.charCodeAt(0) + person - 1); }
function liarStatement(statement) {
  if (statement.kind === 'says_liar') return `${personLabel(statement.person)}さんはうそつきだ。`;
  if (statement.kind === 'says_not_liar') return `${personLabel(statement.person)}さんはうそつきではない。`;
  if (statement.kind === 'exactly_one_liar') return `${personLabel(statement.first)}さんと${personLabel(statement.second)}さんのうち、うそつきは1人だけだ。`;
  if (statement.kind === 'exact_liar_count') return `このなかの${statement.count}人がうそつきだ。`;
  if (statement.kind === 'both_liar') return `${personLabel(statement.first)}さんと${personLabel(statement.second)}さんはうそつきだ。`;
  if (statement.kind === 'both_not_liar') return `${personLabel(statement.first)}さんと${personLabel(statement.second)}さんはうそつきではない。`;
  throw new Error(`Unsupported liar statement: ${statement.kind}`);
}

function formatGrid(values) {
  return Array.from({ length: 4 }, (_, row) => values.slice(row * 4, row * 4 + 4).map((value) => value ?? '・').join(' ')).join('\n');
}

function wasmDirectory() {
  const candidates = [
    process.env.AUTODRILL_QA_WASM_DIR,
    resolve(import.meta.dirname, '../wasm'),
    resolve(import.meta.dirname, '../../web/public/wasm/pkg'),
  ].filter(Boolean);
  const found = candidates.find((candidate) => existsSync(join(candidate, 'drill_wasm.js')) && existsSync(join(candidate, 'drill_wasm_bg.wasm')));
  if (!found) throw new Error('AutoDrill WASM runtimeが見つかりません。先に ./scripts/build-wasm.sh を実行してください。');
  return found;
}

export class AutoDrillRuntime {
  constructor({ contract = JSON.parse(readFileSync(CONTRACT_PATH, 'utf8')), selectionSeed = randomUUID } = {}) {
    this.contract = contract;
    this.selectionSeed = selectionSeed;
    this.runtimePromise = null;
    this.batches = new Map();
    this.diagnosticCache = new Map();
    const contractThemes = Object.values(contract.themes);
    this.excludedSkillIds = new Set(
      contractThemes
        .filter((theme) => EXCLUDED_QA_CURRICULUM_UNITS.has(theme.curriculum_unit.key))
        .map((theme) => theme.skill_id),
    );
    this.themes = contractThemes.filter((theme) => !this.excludedSkillIds.has(theme.skill_id));
    if (!this.themes.length) throw new Error('QAで評価可能なAutoDrill themeがありません。');
  }

  listUnits() {
    return this.themes.map((theme) => ({
      skill_id: theme.skill_id,
      numeric_theme_id: theme.numeric_theme_id,
      name: theme.curriculum_path.filter((part) => part !== 'root').at(-1) ?? theme.skill_id,
      curriculum_path: theme.curriculum_path,
    }));
  }

  async runtime() {
    if (!this.runtimePromise) {
      this.runtimePromise = (async () => {
        const directory = wasmDirectory();
        const glue = readFileSync(join(directory, 'drill_wasm.js'), 'utf8');
        const bindings = await import(`data:text/javascript;base64,${Buffer.from(glue).toString('base64')}`);
        bindings.initSync({ module: readFileSync(join(directory, 'drill_wasm_bg.wasm')) });
        return bindings;
      })();
    }
    return this.runtimePromise;
  }

  async diagnosticWorksheet(request, { cache = true } = {}) {
    const key = JSON.stringify(request);
    if (cache && this.diagnosticCache.has(key)) return this.diagnosticCache.get(key);
    const runtime = await this.runtime();
    if (typeof runtime.generate_qa_worksheet_with_effort !== 'function') {
      throw new Error('QA effort diagnosticsを含むWASMが必要です。pnpm --filter @autodrill/qa build:wasm を実行してください。');
    }
    const envelope = JSON.parse(runtime.generate_qa_worksheet_with_effort(JSON.stringify(request)));
    if (!envelope.ok) throw new Error(`AutoDrill QA diagnostic generation failed: ${envelope.error?.message ?? 'unknown error'}`);
    if (cache) {
      this.diagnosticCache.set(key, envelope.data);
      while (this.diagnosticCache.size > MAX_DIAGNOSTIC_CACHE_ENTRIES) this.diagnosticCache.delete(this.diagnosticCache.keys().next().value);
    }
    return envelope.data;
  }

  async generateProblem({ skillId, samplingMode = 'random', observations = [] } = {}) {
    if (samplingMode === 'random') return this.generateRandomProblem({ skillId });
    if (samplingMode !== 'custom') throw new Error(`Unknown QA sampling mode: ${samplingMode}`);
    return this.generateCustomProblem({ skillId, observations });
  }

  async generateCustomProblem({ skillId, observations = [] } = {}) {
    const selectionSeed = this.selectionSeed();
    const theme = this.themes.find((candidate) => candidate.skill_id === skillId);
    if (!theme) throw new Error(`QAで選択できない単元です: ${skillId}`);

    const candidates = [];
    const candidateRequests = [];
    const seen = new Set();
    let operationVectorBasis = null;
    for (let worksheetIndex = 0; worksheetIndex < CUSTOM_CANDIDATE_WORKSHEETS; worksheetIndex += 1) {
      const request = {
        schema_version: this.contract.schema_version,
        numeric_theme_id: theme.numeric_theme_id,
        seed: generatorSeed(`${selectionSeed}:custom:${worksheetIndex}`),
        difficulty: 4,
      };
      candidateRequests.push(request);
      const generated = await this.diagnosticWorksheet(request, { cache: false });
      const basis = generated.operation_vector_basis ?? [];
      if (operationVectorBasis == null) operationVectorBasis = basis;
      else if (JSON.stringify(operationVectorBasis) !== JSON.stringify(basis)) {
        throw new Error('QA effort diagnostic basis changed within one custom candidate pool.');
      }
      generated.worksheet.problems.forEach((problem, problemIndex) => {
        const signature = JSON.stringify(problem.prompt);
        if (seen.has(signature)) return;
        seen.add(signature);
        const diagnostics = generated.problems[problemIndex];
        const sourceIdentifier = `${generated.worksheet.identity.numeric_theme_id}:${generated.worksheet.identity.generator_revision}:${generated.worksheet.identity.seed}:${generated.worksheet.identity.difficulty}:${problemIndex}`;
        candidates.push({
          ...diagnostics,
          signature,
          source_identifier: sourceIdentifier,
          request,
          worksheet: generated.worksheet,
          problem_index: problemIndex,
          problem,
        });
      });
    }
    operationVectorBasis ??= [];

    const observed = [];
    const observedSignatures = new Set();
    for (const observation of observations) {
      const payload = observation?.original_source_payload;
      const request = payload?.generation_request;
      const problemIndex = payload?.problem_index;
      if (!request || !Number.isInteger(problemIndex) || payload?.theme?.skill_id !== skillId) continue;
      const signature = JSON.stringify(payload.problem?.prompt);
      observedSignatures.add(signature);
      const snapshot = payload.qa_sampling;
      const snapshotBasisMatches = snapshot?.mode === 'custom'
        && JSON.stringify(snapshot.operation_vector_basis ?? null) === JSON.stringify(operationVectorBasis);
      if (snapshotBasisMatches && Number.isFinite(snapshot.effort)) {
        observed.push({
          effort: snapshot.effort,
          effort_model: snapshot.effort_model,
          operation_vector: snapshot.operation_vector,
        });
        continue;
      }
      const generated = await this.diagnosticWorksheet(request);
      if (JSON.stringify(generated.operation_vector_basis ?? []) !== JSON.stringify(operationVectorBasis)) continue;
      const problem = generated.worksheet?.problems?.[problemIndex];
      const diagnostic = generated.problems?.[problemIndex];
      if (!problem || !diagnostic || JSON.stringify(problem.prompt) !== signature) continue;
      observed.push(diagnostic);
    }

    const unseen = candidates.filter((candidate) => !observedSignatures.has(candidate.signature));
    const scoringPool = unseen.length ? unseen : candidates;
    const scored = scoreInformationCandidates({ observed, candidates: scoringPool });
    scored.sort((left, right) => right.information_score - left.information_score
      || digestNumber(selectionSeed, left.source_identifier) - digestNumber(selectionSeed, right.source_identifier));
    const selected = scored[0];
    if (!selected) throw new Error('custom sampling candidateを生成できませんでした。');
    const unitName = theme.curriculum_path.filter((part) => part !== 'root').at(-1) ?? theme.skill_id;
    return {
      item: {
        source: 'autodrill',
        source_identifier: selected.source_identifier,
        unit_name: unitName,
        problem_representation: formatProblem(selected.problem),
        canonical_answer: formatCanonicalAnswer(selected.problem.canonical_answer, selected.problem),
        original_source_payload: {
          integration_version: 'autodrill_qa_wasm_v1',
          selection_seed: selectionSeed,
          theme,
          generation_request: selected.request,
          problem_index: selected.problem_index,
          problem: selected.problem,
          worksheet: selected.worksheet,
          qa_sampling: {
            mode: 'custom',
            profile: CUSTOM_SAMPLING_PROFILE,
            information_score: selected.information_score,
            effort: selected.effort,
            effort_model: selected.effort_model,
            operation_vector_basis: operationVectorBasis,
            operation_vector: selected.operation_vector,
          },
        },
      },
      selection: {
        selection_policy: 'autodrill_unit_custom_v1',
        candidate_source: 'drill_core_random_worksheet_effort_pool',
        filters: {
          selected_skill_id: skillId,
          requested_difficulty: 4,
          custom_profile: CUSTOM_SAMPLING_PROFILE,
          operation_vector_basis: operationVectorBasis,
          observed_count: observed.length,
          worksheet_seed: selected.worksheet.identity.seed,
          worksheet_problem_index: selected.problem_index,
          candidate_worksheet_requests: candidateRequests,
        },
        random_seed: selectionSeed,
        selection_probability: null,
        candidate_count: scoringPool.length,
        model_name: CUSTOM_SAMPLING_PROFILE.name,
        model_version: CUSTOM_SAMPLING_PROFILE.version,
        candidate_scores: scored.map((candidate) => ({
          source_identifier: candidate.source_identifier,
          score: candidate.information_score,
          effort: candidate.effort,
          effort_model: candidate.effort_model,
          operation_vector: candidate.operation_vector,
        })),
      },
    };
  }

  async generateRandomProblem({ skillId } = {}) {
    let selectionSeed = this.selectionSeed();
    const theme = skillId
      ? this.themes.find((candidate) => candidate.skill_id === skillId)
      : this.themes[digestNumber(selectionSeed, 'theme') % this.themes.length];
    if (!theme) throw new Error(`QAで選択できない単元です: ${skillId}`);
    let batch = skillId ? this.batches.get(skillId) : null;
    if (!batch?.remaining.length) {
      const seed = generatorSeed(selectionSeed);
      const request = {
        schema_version: this.contract.schema_version,
        numeric_theme_id: theme.numeric_theme_id,
        seed,
        difficulty: 4,
      };
      const runtime = await this.runtime();
      const envelope = JSON.parse(runtime.generate_worksheet(JSON.stringify(request)));
      if (!envelope.ok) throw new Error(`AutoDrill generation failed: ${envelope.error?.message ?? 'unknown error'}`);
      const worksheet = envelope.data;
      const remaining = worksheet.problems.map((_, index) => index).sort((left, right) => (
        digestNumber(selectionSeed, `problem:${left}`) - digestNumber(selectionSeed, `problem:${right}`)
      ));
      batch = { selectionSeed, request, worksheet, remaining };
      if (skillId) this.batches.set(skillId, batch);
    } else {
      selectionSeed = batch.selectionSeed;
    }
    const candidateCount = batch.remaining.length;
    const problemIndex = batch.remaining.shift();
    const { worksheet, request } = batch;
    const problem = worksheet.problems[problemIndex];
    const unitName = theme.curriculum_path.filter((part) => part !== 'root').at(-1) ?? theme.skill_id;
    return {
      item: {
        source: 'autodrill',
        source_identifier: `${worksheet.identity.numeric_theme_id}:${worksheet.identity.generator_revision}:${worksheet.identity.seed}:${worksheet.identity.difficulty}:${problemIndex}`,
        unit_name: unitName,
        problem_representation: formatProblem(problem),
        canonical_answer: formatCanonicalAnswer(problem.canonical_answer, problem),
        original_source_payload: {
          integration_version: 'autodrill_qa_wasm_v1',
          selection_seed: selectionSeed,
          theme,
          generation_request: request,
          problem_index: problemIndex,
          problem,
          worksheet,
        },
      },
      selection: {
        selection_policy: skillId ? 'autodrill_unit_random_v1' : 'autodrill_random_v1',
        candidate_source: 'drill_core_worksheet_without_replacement',
        filters: { selected_skill_id: skillId ?? null, worksheet_seed: worksheet.identity.seed, worksheet_problem_index: problemIndex, excluded_skill_ids: [...this.excludedSkillIds], requested_difficulty: 4 },
        random_seed: selectionSeed,
        selection_probability: 1 / (skillId ? 1 : this.themes.length) / candidateCount,
        candidate_count: candidateCount,
      },
    };
  }

  async gradeAnswer(sourcePayload, rawAnswer) {
    if (sourcePayload?.integration_version !== 'autodrill_qa_wasm_v1' || !sourcePayload.problem) return null;
    const runtime = await this.runtime();
    const problem = sourcePayload.problem;
    const parsed = JSON.parse(runtime.parse_mathlive_answer(JSON.stringify({
      schema_version: this.contract.schema_version,
      input_interface: problem.input_interface,
      latex: rawAnswer,
    })));
    if (!parsed.ok) throw new Error(`AutoDrill answer parse failed: ${parsed.error?.message ?? 'unknown error'}`);
    const graded = JSON.parse(runtime.grade_answer(JSON.stringify({
      schema_version: this.contract.schema_version,
      expected: problem.canonical_answer,
      actual: parsed.data,
      answer_schema: problem.answer_schema,
      input_interface: problem.input_interface,
    })));
    if (!graded.ok) throw new Error(`AutoDrill grading failed: ${graded.error?.message ?? 'unknown error'}`);
    return {
      correctness: graded.data.is_correct ? 'correct' : 'incorrect',
      normalized_user_answer: JSON.stringify(parsed.data),
      grading_method: 'autodrill_wasm_grade_v1',
      raw_result: { parsed: parsed.data, graded: graded.data },
    };
  }
}
