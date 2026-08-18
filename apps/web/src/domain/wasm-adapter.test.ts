import { describe, expect, it, vi } from 'vitest';

import { createWasmDrillEngine } from '@/domain/wasm-adapter';
import { DRILL_SCHEMA_VERSION, answerNodeText } from '@/domain/drill-engine';
import { ONE_DIGIT_ADDITION_DEFINITION } from '@/domain/themes/one-digit-addition';
import { DRILL_CORE_CONTRACT } from '@/generated/drill-core-contract';
import { fixtureSettings, fixtureWorksheet, liarFixtureWorksheet, linearFixtureWorksheet, miniSudokuFixtureWorksheet } from '@/test/fixtures';

const envelope = (data: unknown) => ({ schema_version: DRILL_SCHEMA_VERSION, ok: true, data, error: null });
const simpleInputInterface = { type: 'simple_numeric', allow_decimal: false, allow_negative: false } as const;
const structuredInputInterface = {
  type: 'structured_math',
  allowed_structures: ['fraction', 'mixed_fraction', 'decimal', 'root', 'negative', 'plus_minus', 'tuple'],
} as const;

describe('versioned WASM adapter', () => {
  it('rejects schema-v2 envelopes and malformed input interfaces', async () => {
    const oldEnvelope = createWasmDrillEngine({
      generate_worksheet: vi.fn().mockResolvedValue({ schema_version: 2, ok: true, data: fixtureWorksheet(), error: null }),
    });
    await expect(oldEnvelope.generateWorksheet(fixtureSettings())).rejects.toMatchObject({ kind: 'invalid_dto' });

    const malformed = fixtureWorksheet();
    malformed.problems = [{
      ...malformed.problems[0]!,
      input_interface: { type: 'structured_math', allowed_structures: ['fraction', 'fraction'] } as never,
    }, ...malformed.problems.slice(1)];
    const malformedEngine = createWasmDrillEngine({
      generate_worksheet: vi.fn().mockResolvedValue(envelope(malformed)),
    });
    await expect(malformedEngine.generateWorksheet(fixtureSettings())).rejects.toMatchObject({ kind: 'invalid_dto' });
  });

  it('rejects a non-digits-only interface for the current one-digit theme', async () => {
    const malformed = fixtureWorksheet();
    malformed.problems = [{
      ...malformed.problems[0]!,
      input_interface: {
        type: 'structured_math',
        allowed_structures: ['fraction'],
      },
    }, ...malformed.problems.slice(1)];
    const engine = createWasmDrillEngine({
      generate_worksheet: vi.fn().mockResolvedValue(envelope(malformed)),
    });
    await expect(engine.generateWorksheet(fixtureSettings())).rejects.toMatchObject({ kind: 'invalid_dto' });
  });

  it('rejects a response AST that exceeds the structural node budget even when display size is small', async () => {
    const malformed = fixtureWorksheet();
    malformed.problems = [{
      ...malformed.problems[0]!,
      canonical_answer: {
        type: 'tuple',
        value: Array.from({ length: DRILL_CORE_CONTRACT.max_answer_ast_size }, () => ({ type: 'empty' as const })),
      },
    }, ...malformed.problems.slice(1)];
    const engine = createWasmDrillEngine({
      generate_worksheet: vi.fn().mockResolvedValue(envelope(malformed)),
    });
    await expect(engine.generateWorksheet(fixtureSettings())).rejects.toMatchObject({ kind: 'invalid_dto' });
  });

  it('sends the exact current-schema request and preserves identity/problem-set metadata', async () => {
    const worksheet = fixtureWorksheet();
    const generate = vi.fn().mockResolvedValue(envelope(worksheet));
    const engine = createWasmDrillEngine({ generate_worksheet: generate });

    const result = await engine.generateWorksheet(fixtureSettings());
    expect(JSON.parse(generate.mock.calls[0]?.[0] as string)).toEqual({
      schema_version: DRILL_SCHEMA_VERSION,
      numeric_theme_id: 1,
      seed: 'fixtureSeed',
      difficulty: 3,
    });
    expect(result).toMatchObject({
      schema_version: DRILL_SCHEMA_VERSION,
      problem_set_id: `${DRILL_SCHEMA_VERSION}-1-${ONE_DIGIT_ADDITION_DEFINITION.generator_revision}-fixtureSeed-3`,
      identity: { numeric_theme_id: 1, generator_revision: ONE_DIGIT_ADDITION_DEFINITION.generator_revision, seed: 'fixtureSeed', difficulty: 3 },
    });
    expect(new Set(result.problems.map((problem) => problem.id)).size).toBe(20);
  });

  it('accepts registered linear-equation DTOs without addition-specific boundary branches', async () => {
    for (const themeId of [2, 3] as const) {
      const worksheet = linearFixtureWorksheet(themeId);
      const generate = vi.fn().mockResolvedValue(envelope(worksheet));
      const engine = createWasmDrillEngine({ generate_worksheet: generate });
      const result = await engine.generateWorksheet({
        schema_version: DRILL_SCHEMA_VERSION,
        numeric_theme_id: themeId,
        seed: 'fixtureSeed',
        difficulty: 3,
      });
      expect(result.layout).toEqual({ problem_count: 16, columns: 2, rows: 8 });
      expect(result.problems).toHaveLength(16);
      expect(result.problems.every((problem) => problem.prompt.kind === 'linear_equation')).toBe(true);
      expect(result.problems.every((problem) => problem.input_interface.type === 'structured_math')).toBe(true);
    }
  });


  it('accepts liar-puzzle DTOs with the current six statement variants', async () => {
    const worksheet = liarFixtureWorksheet();
    const generate = vi.fn().mockResolvedValue(envelope(worksheet));
    const engine = createWasmDrillEngine({ generate_worksheet: generate });
    const result = await engine.generateWorksheet({
      schema_version: DRILL_SCHEMA_VERSION,
      numeric_theme_id: 20,
      seed: 'fixtureSeed',
      difficulty: 2,
    });
    expect(result.problems).toHaveLength(6);
    const prompt = result.problems[0]!.prompt;
    expect(prompt.kind).toBe('liar_puzzle');
    if (prompt.kind !== 'liar_puzzle') throw new Error('liar-puzzle prompt expected');
    expect(prompt.statements.map((statement) => statement.kind)).toEqual([
      'says_liar', 'exact_liar_count', 'both_not_liar', 'says_not_liar',
    ]);
  });

  it('accepts fixed digit-grid DTOs and rejects malformed grid contracts', async () => {
    const worksheet = miniSudokuFixtureWorksheet();
    const engine = createWasmDrillEngine({ generate_worksheet: vi.fn().mockResolvedValue(envelope(worksheet)) });
    const result = await engine.generateWorksheet({
      schema_version: DRILL_SCHEMA_VERSION,
      numeric_theme_id: 38,
      seed: 'fixtureSeed',
      difficulty: 3,
    });
    expect(result.problems).toHaveLength(4);
    expect(result.problems.every((problem) => problem.input_interface.type === 'digit_grid')).toBe(true);
    expect(result.problems.every((problem) => problem.answer_schema.kind === 'ordered_tuple')).toBe(true);

    const malformed = miniSudokuFixtureWorksheet();
    malformed.problems = [{
      ...malformed.problems[0]!,
      prompt: { kind: 'mini_sudoku', givens: [1, null] },
    }, ...malformed.problems.slice(1)];
    const malformedEngine = createWasmDrillEngine({ generate_worksheet: vi.fn().mockResolvedValue(envelope(malformed)) });
    await expect(malformedEngine.generateWorksheet({
      schema_version: DRILL_SCHEMA_VERSION,
      numeric_theme_id: 38,
      seed: 'fixtureSeed',
      difficulty: 3,
    })).rejects.toMatchObject({ kind: 'invalid_dto' });

    const mixedEffort = miniSudokuFixtureWorksheet();
    mixedEffort.problems = [{
      ...mixedEffort.problems[0]!,
      effort: 7,
    }, ...mixedEffort.problems.slice(1)];
    const mixedEffortEngine = createWasmDrillEngine({ generate_worksheet: vi.fn().mockResolvedValue(envelope(mixedEffort)) });
    await expect(mixedEffortEngine.generateWorksheet({
      schema_version: DRILL_SCHEMA_VERSION,
      numeric_theme_id: 38,
      seed: 'fixtureSeed',
      difficulty: 3,
    })).rejects.toMatchObject({ kind: 'invalid_dto' });
  });

  it('rejects a prompt kind that disagrees with the registered linear theme', async () => {
    const worksheet = linearFixtureWorksheet(2);
    worksheet.problems = [{
      ...worksheet.problems[0]!,
      prompt: { kind: 'addition', left: 1, right: 2 },
    }, ...worksheet.problems.slice(1)];
    const engine = createWasmDrillEngine({ generate_worksheet: vi.fn().mockResolvedValue(envelope(worksheet)) });
    await expect(engine.generateWorksheet({
      schema_version: DRILL_SCHEMA_VERSION,
      numeric_theme_id: 2,
      seed: 'fixtureSeed',
      difficulty: 3,
    })).rejects.toMatchObject({ kind: 'invalid_dto' });
  });

  it('preserves distinct timeout and attempt-limit errors', async () => {
    const timeout = createWasmDrillEngine({
      generate_worksheet: vi.fn().mockResolvedValue({
        schema_version: DRILL_SCHEMA_VERSION,
        ok: false,
        data: null,
        error: { code: 'generation_timeout', message: 'timed out' },
      }),
    });
    await expect(timeout.generateWorksheet(fixtureSettings())).rejects.toMatchObject({ kind: 'generation_timeout' });

    const attempts = createWasmDrillEngine({
      generate_worksheet: vi.fn().mockResolvedValue({
        schema_version: DRILL_SCHEMA_VERSION,
        ok: false,
        data: null,
        error: { code: 'generation_attempt_limit', message: 'attempt limit' },
      }),
    });
    await expect(attempts.generateWorksheet(fixtureSettings())).rejects.toMatchObject({ kind: 'generation_attempt_limit' });
  });
  it('preserves generator configuration and aggregate contract errors across the WASM boundary', async () => {
    for (const kind of [
      'invalid_sampling_strategy',
      'invalid_registry',
      'invalid_generated_problem',
      'invalid_generated_worksheet',
    ] as const) {
      const engine = createWasmDrillEngine({
        generate_worksheet: vi.fn().mockResolvedValue({
          schema_version: DRILL_SCHEMA_VERSION,
          ok: false,
          data: null,
          error: { code: kind, message: kind },
        }),
      });
      await expect(engine.generateWorksheet(fixtureSettings())).rejects.toMatchObject({ kind });
    }
  });

  it('maps MathLive LaTeX through the explicit Rust AnswerNode adapter boundary', async () => {
    const parseMathLiveAnswer = vi.fn().mockResolvedValue(envelope({
      type: 'fraction',
      value: { numerator: { type: 'integer', value: '7' }, denominator: { type: 'integer', value: '2' } },
    }));
    const engine = createWasmDrillEngine({ parse_mathlive_answer: parseMathLiveAnswer });

    await expect(engine.parseMathLiveAnswer('\\frac{7}{2}', structuredInputInterface)).resolves.toEqual({
      type: 'fraction',
      value: { numerator: { type: 'integer', value: '7' }, denominator: { type: 'integer', value: '2' } },
    });
    expect(JSON.parse(parseMathLiveAnswer.mock.calls[0]?.[0] as string)).toEqual({
      schema_version: DRILL_SCHEMA_VERSION,
      input_interface: structuredInputInterface,
      latex: '\\frac{7}{2}',
    });
  });

  it('grades MathLive structural answers without requiring editor path state', async () => {
    const worksheet = linearFixtureWorksheet(3);
    const submitted = {
      type: 'fraction' as const,
      value: {
        numerator: { type: 'integer' as const, value: '11' },
        denominator: { type: 'integer' as const, value: '1' },
      },
    };
    const gradeAnswer = vi.fn().mockImplementation(async (requestJson: string) => {
      const request = JSON.parse(requestJson) as { expected: unknown; actual: { type: string } };
      return envelope({
        status: request.actual.type === 'empty' ? 'unanswered' : 'incorrect',
        is_correct: false,
        expected: request.expected,
        actual: request.actual,
        warnings: [],
      });
    });
    const engine = createWasmDrillEngine({ grade_answer: gradeAnswer });

    const result = await engine.gradeAnswer({
      schema_version: DRILL_SCHEMA_VERSION,
      worksheet,
      answers: [{ problem_id: worksheet.problems[0]!.problem_id, answer: submitted }],
    });

    expect(JSON.parse(gradeAnswer.mock.calls[0]?.[0] as string).actual).toEqual(submitted);
    expect(result.items[0]).toMatchObject({ answer: '11/1', correct: false });
  });
 it('rejects runtime grade requests that do not use the current schema', async () => {
    const gradeAnswer = vi.fn();
    const engine = createWasmDrillEngine({ grade_answer: gradeAnswer });
    await expect(engine.gradeAnswer({
      schema_version: 2,
      worksheet: fixtureWorksheet(),
      answers: [],
    } as never)).rejects.toMatchObject({ kind: 'invalid_dto' });
    expect(gradeAnswer).not.toHaveBeenCalled();
  });

  it.each([
    ['duplicates', ['fraction_not_reduced', 'fraction_not_reduced']],
    ['non-canonical order', ['redundant_decimal', 'fraction_not_reduced']],
    ['unknown identifiers', ['future_warning']],
  ])('rejects %s in grade warning arrays', async (_caseName, warnings) => {
    const worksheet = fixtureWorksheet();
    const gradeAnswer = vi.fn().mockResolvedValue(envelope({
      status: 'correct',
      is_correct: true,
      expected: { type: 'integer', value: '2' },
      actual: { type: 'integer', value: '2' },
      warnings,
    }));
    const engine = createWasmDrillEngine({ grade_answer: gradeAnswer });

    await expect(engine.gradeAnswer({
      schema_version: DRILL_SCHEMA_VERSION,
      worksheet,
      answers: worksheet.problems.map((problem) => ({
        problem_id: problem.problem_id,
        answer: { type: 'empty' },
      })),
    })).rejects.toMatchObject({ kind: 'invalid_dto' });
  });

  it.each([
    ['invalid_answer_schema', 'invalid_answer_schema'],
    ['expected_answer_outside_schema', 'expected_answer_outside_schema'],
  ] as const)('preserves Rust grading error code %s', async (code, expectedKind) => {
    const worksheet = fixtureWorksheet();
    const gradeAnswer = vi.fn().mockResolvedValue({
      schema_version: DRILL_SCHEMA_VERSION,
      ok: false,
      data: null,
      error: { code, message: code },
    });
    const engine = createWasmDrillEngine({ grade_answer: gradeAnswer });

    await expect(engine.gradeAnswer({
      schema_version: DRILL_SCHEMA_VERSION,
      worksheet,
      answers: [{
        problem_id: worksheet.problems[0]!.problem_id,
        answer: { type: 'integer', value: '2' },
      }],
    })).rejects.toMatchObject({ kind: expectedKind });
  });

  it('accepts the generated integer-form warning used for answers such as 2/1', async () => {
    const worksheet = linearFixtureWorksheet(2);
    const gradeAnswer = vi.fn().mockResolvedValue(envelope({
      status: 'correct',
      is_correct: true,
      expected: { type: 'integer', value: '2' },
      actual: { type: 'integer', value: '2' },
      warnings: ['integer_form_required'],
    }));
    const engine = createWasmDrillEngine({ grade_answer: gradeAnswer });

    const result = await engine.gradeAnswer({
      schema_version: DRILL_SCHEMA_VERSION,
      worksheet,
      answers: worksheet.problems.map((problem) => ({
        problem_id: problem.problem_id,
        answer: { type: 'empty' },
      })),
    });

    expect(result.items[0]).toMatchObject({ correct: true, warnings: ['integer_form_required'] });
  });

  it('preserves canonical warnings attached to an incorrect answer', async () => {
    const worksheet = fixtureWorksheet();
    const gradeAnswer = vi.fn().mockResolvedValue(envelope({
      status: 'incorrect',
      is_correct: false,
      expected: { type: 'integer', value: '2' },
      actual: { type: 'integer', value: '3' },
      warnings: ['redundant_decimal'],
    }));
    const engine = createWasmDrillEngine({ grade_answer: gradeAnswer });

    const result = await engine.gradeAnswer({
      schema_version: DRILL_SCHEMA_VERSION,
      worksheet,
      answers: worksheet.problems.map((problem) => ({
        problem_id: problem.problem_id,
        answer: { type: 'empty' },
      })),
    });
    expect(result.items[0]).toMatchObject({ correct: false, warnings: ['redundant_decimal'] });
  });

  it('accepts the unanswered grade status for an empty actual answer', async () => {
    const worksheet = fixtureWorksheet();
    const gradeAnswer = vi.fn().mockResolvedValue(envelope({
      status: 'unanswered',
      is_correct: false,
      expected: { type: 'integer', value: '2' },
      actual: { type: 'empty' },
      warnings: [],
    }));
    const engine = createWasmDrillEngine({ grade_answer: gradeAnswer });

    const result = await engine.gradeAnswer({
      schema_version: DRILL_SCHEMA_VERSION,
      worksheet,
      answers: worksheet.problems.map((problem) => ({
        problem_id: problem.problem_id,
        answer: { type: 'empty' },
      })),
    });

    expect(result.items[0]).toEqual({ problem_id: '1', answer: null, correct: false, warnings: [] });
  });

  it.each([
    ['unknown status', { status: 'partial', is_correct: false, actual: { type: 'integer', value: '3' } }],
    ['correct with false flag', { status: 'correct', is_correct: false, actual: { type: 'integer', value: '2' } }],
    ['correct with empty answer', { status: 'correct', is_correct: true, actual: { type: 'empty' } }],
    ['incorrect with true flag', { status: 'incorrect', is_correct: true, actual: { type: 'integer', value: '3' } }],
    ['incorrect with empty answer', { status: 'incorrect', is_correct: false, actual: { type: 'empty' } }],
    ['unanswered with true flag', { status: 'unanswered', is_correct: true, actual: { type: 'empty' } }],
    ['unanswered with non-empty answer', { status: 'unanswered', is_correct: false, actual: { type: 'integer', value: '3' } }],
  ])('rejects %s grade status payloads', async (_caseName, payload) => {
    const worksheet = fixtureWorksheet();
    const gradeAnswer = vi.fn().mockResolvedValue(envelope({
      ...payload,
      expected: { type: 'integer', value: '2' },
      warnings: [],
    }));
    const engine = createWasmDrillEngine({ grade_answer: gradeAnswer });

    await expect(engine.gradeAnswer({
      schema_version: DRILL_SCHEMA_VERSION,
      worksheet,
      answers: worksheet.problems.map((problem) => ({
        problem_id: problem.problem_id,
        answer: { type: 'empty' },
      })),
    })).rejects.toMatchObject({ kind: 'invalid_dto' });
  });

  it('rejects unknown prompt and AnswerNode variants at the boundary', async () => {
    const unknownPrompt = fixtureWorksheet();
    const problem = unknownPrompt.problems[0]!;
    unknownPrompt.problems = [{ ...problem, prompt: { kind: 'multiplication', left: 1, right: 2 } as never }, ...unknownPrompt.problems.slice(1)];
    const engine = createWasmDrillEngine({ generate_worksheet: vi.fn().mockResolvedValue(envelope(unknownPrompt)) });
    await expect(engine.generateWorksheet(fixtureSettings())).rejects.toMatchObject({ kind: 'invalid_dto' });

    const unknownAnswer = fixtureWorksheet();
    unknownAnswer.problems = [{ ...unknownAnswer.problems[0]!, canonical_answer: { type: 'future_kind', value: 1 } as never }, ...unknownAnswer.problems.slice(1)];
    const answerEngine = createWasmDrillEngine({ generate_worksheet: vi.fn().mockResolvedValue(envelope(unknownAnswer)) });
    await expect(answerEngine.generateWorksheet(fixtureSettings())).rejects.toMatchObject({ kind: 'invalid_dto' });
  });
 it('preserves exact decimal text in the grade projection without number coercion', async () => {
    const worksheet = fixtureWorksheet();
    worksheet.problems = worksheet.problems.map((problem) => ({
      ...problem,
      input_interface: { type: 'simple_numeric', allow_decimal: true, allow_negative: false },
    }));
    const exact = '123456789012345678';
    const gradeAnswer = vi.fn().mockResolvedValue(envelope({
      status: 'correct',
      is_correct: true,
      expected: { type: 'exact_decimal', value: { coefficient: exact, scale: 2 } },
      actual: { type: 'exact_decimal', value: { coefficient: exact, scale: 2 } },
      warnings: [],
    }));
    const engine = createWasmDrillEngine({ grade_answer: gradeAnswer });

    const result = await engine.gradeAnswer({
      schema_version: DRILL_SCHEMA_VERSION,
      worksheet,
      answers: worksheet.problems.map((problem) => ({
        problem_id: problem.problem_id,
        answer: { type: 'empty' },
      })),
    });

    expect(result.items[0]?.answer).toBe('1234567890123456.78');
    expect(gradeAnswer).toHaveBeenCalledTimes(20);
  });

  it('accepts nan_error as raw editable text without numeric coercion', async () => {
    const worksheet = fixtureWorksheet();
    worksheet.problems = [{
      ...worksheet.problems[0]!,
      canonical_answer: { type: 'nan_error', value: '1e+' },
    }, ...worksheet.problems.slice(1)];
    const generated = createWasmDrillEngine({ generate_worksheet: vi.fn().mockResolvedValue(envelope(worksheet)) });
    const result = await generated.generateWorksheet(fixtureSettings());
    expect(result.problems[0]?.canonical_answer).toEqual({ type: 'nan_error', value: '1e+' });
    expect(answerNodeText(result.problems[0]!.canonical_answer)).toBe('1e+');

    const gradeAnswer = vi.fn().mockResolvedValue(envelope({
      status: 'incorrect',
      is_correct: false,
      expected: { type: 'integer', value: '2' },
      actual: { type: 'nan_error', value: '1e+' },
      warnings: [],
    }));
    const engine = createWasmDrillEngine({ grade_answer: gradeAnswer });
    const graded = await engine.gradeAnswer({
      schema_version: DRILL_SCHEMA_VERSION,
      worksheet,
      answers: worksheet.problems.map((problem) => ({
        problem_id: problem.problem_id,
        answer: { type: 'empty' },
      })),
    });
    expect(graded.items[0]).toMatchObject({ answer: '1e+', correct: false, warnings: [] });
  });

  it('rejects JSON numbers and non-canonical strings for exact integer payloads', async () => {
    const numeric = fixtureWorksheet();
    numeric.problems = [{
      ...numeric.problems[0]!,
      canonical_answer: { type: 'integer', value: 999999999999999999 as never },
    }, ...numeric.problems.slice(1)];
    const numericEngine = createWasmDrillEngine({ generate_worksheet: vi.fn().mockResolvedValue(envelope(numeric)) });
    await expect(numericEngine.generateWorksheet(fixtureSettings())).rejects.toMatchObject({ kind: 'invalid_dto' });

    const nonCanonical = fixtureWorksheet();
    nonCanonical.problems = [{
      ...nonCanonical.problems[0]!,
      canonical_answer: { type: 'integer', value: '01' },
    }, ...nonCanonical.problems.slice(1)];
    const nonCanonicalEngine = createWasmDrillEngine({ generate_worksheet: vi.fn().mockResolvedValue(envelope(nonCanonical)) });
    await expect(nonCanonicalEngine.generateWorksheet(fixtureSettings())).rejects.toMatchObject({ kind: 'invalid_dto' });
  });
});
