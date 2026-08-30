import { createHash, randomUUID } from 'node:crypto';
import { existsSync, readFileSync } from 'node:fs';
import { join, resolve } from 'node:path';

const CONTRACT_PATH = new URL('../generated/drill-core-contract.json', import.meta.url);
const SIMPLE_PROMPT_KINDS = new Set(['addition', 'arithmetic', 'column_arithmetic']);
const EXCLUDED_QA_SKILLS = new Set([
  'jp.grade1.addition.one_digit',
  'jp.grade1.subtraction.one_digit',
  'jp.grade2.multiplication.table',
  'jp.grade3.division.table.1',
]);
const SEED_ALPHABET = 'ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz23456789';

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
  return value.denominator === 1 ? String(value.numerator) : `${value.numerator}/${value.denominator}`;
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
  throw new Error(`Unsupported QA prompt kind: ${prompt.kind}`);
}

export function formatCanonicalAnswer(answer) {
  if (answer.type === 'integer') return String(answer.value).replace(/^-/, '−');
  if (answer.type === 'exact_decimal') return exactDecimal(answer.value.coefficient, answer.value.scale);
  if (answer.type === 'negative') return `−${formatCanonicalAnswer(answer.value)}`;
  if (answer.type === 'fraction') return `${formatCanonicalAnswer(answer.value.numerator)}/${formatCanonicalAnswer(answer.value.denominator)}`;
  if (answer.type === 'mixed_fraction') return `${formatCanonicalAnswer(answer.value.whole)} ${formatCanonicalAnswer(answer.value.numerator)}/${formatCanonicalAnswer(answer.value.denominator)}`;
  if (answer.type === 'tuple') {
    const values = answer.value.map(formatCanonicalAnswer);
    return values.length === 2 ? `${values[0]} あまり ${values[1]}` : values.join(', ');
  }
  return JSON.stringify(answer);
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
    this.themes = Object.values(contract.themes).filter((theme) => (
      SIMPLE_PROMPT_KINDS.has(theme.answer_contract?.prompt_kind)
      && !EXCLUDED_QA_SKILLS.has(theme.skill_id)
    ));
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

  async generateRandomProblem({ skillId } = {}) {
    const selectionSeed = this.selectionSeed();
    const theme = skillId
      ? this.themes.find((candidate) => candidate.skill_id === skillId)
      : this.themes[digestNumber(selectionSeed, 'theme') % this.themes.length];
    if (!theme) throw new Error(`QAで選択できない単元です: ${skillId}`);
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
    const problemIndex = digestNumber(selectionSeed, 'problem') % worksheet.problems.length;
    const problem = worksheet.problems[problemIndex];
    const unitName = theme.curriculum_path.filter((part) => part !== 'root').at(-1) ?? theme.skill_id;
    return {
      item: {
        source: 'autodrill',
        source_identifier: `${worksheet.identity.numeric_theme_id}:${worksheet.identity.generator_revision}:${worksheet.identity.seed}:${worksheet.identity.difficulty}:${problemIndex}`,
        unit_name: unitName,
        problem_representation: formatProblem(problem),
        canonical_answer: formatCanonicalAnswer(problem.canonical_answer),
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
        candidate_source: 'drill_core_web_contract_qa_themes',
        filters: { selected_skill_id: skillId ?? null, prompt_kinds: [...SIMPLE_PROMPT_KINDS], excluded_skill_ids: [...EXCLUDED_QA_SKILLS], requested_difficulty: 4 },
        random_seed: selectionSeed,
        selection_probability: 1 / (skillId ? 1 : this.themes.length) / worksheet.problems.length,
        candidate_count: skillId ? 1 : this.themes.length,
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
