import { describe, expect, it, vi } from 'vitest';

import { createWasmDrillEngine } from '@/domain/wasm-adapter';
import { DRILL_SCHEMA_VERSION, emptyEditorState } from '@/domain/drill-engine';
import { fixtureSettings, fixtureWorksheet } from '@/test/fixtures';

const envelope = (data: unknown) => ({ schema_version: DRILL_SCHEMA_VERSION, ok: true, data, error: null });

describe('versioned WASM adapter', () => {
  it('sends the exact schema-v2 request and preserves identity/problem-set metadata', async () => {
    const worksheet = fixtureWorksheet();
    const generate = vi.fn().mockResolvedValue(envelope(worksheet));
    const engine = createWasmDrillEngine({ generate_worksheet: generate });

    const result = await engine.generateWorksheet(fixtureSettings());
    expect(JSON.parse(generate.mock.calls[0]?.[0] as string)).toEqual({
      schema_version: 2,
      numeric_theme_id: 1,
      seed: 'fixtureSeed',
      difficulty: 3,
    });
    expect(result).toMatchObject({
      schema_version: 2,
      problem_set_id: '2-1-2-fixtureSeed-3',
      identity: { numeric_theme_id: 1, generator_revision: 2, seed: 'fixtureSeed', difficulty: 3 },
    });
    expect(new Set(result.problems.map((problem) => problem.id)).size).toBe(20);
  });

  it('preserves distinct timeout and attempt-limit errors', async () => {
    const timeout = createWasmDrillEngine({
      generate_worksheet: vi.fn().mockResolvedValue({
        schema_version: 2,
        ok: false,
        data: null,
        error: { code: 'generation_timeout', message: 'timed out' },
      }),
    });
    await expect(timeout.generateWorksheet(fixtureSettings())).rejects.toMatchObject({ kind: 'generation_timeout' });

    const attempts = createWasmDrillEngine({
      generate_worksheet: vi.fn().mockResolvedValue({
        schema_version: 2,
        ok: false,
        data: null,
        error: { code: 'generation_attempt_limit', message: 'attempt limit' },
      }),
    });
    await expect(attempts.generateWorksheet(fixtureSettings())).rejects.toMatchObject({ kind: 'generation_attempt_limit' });
  });

  it('maps editor actions to Rust tags and grades every problem through the boundary', async () => {
    const worksheet = fixtureWorksheet();
    const gradeAnswer = vi.fn().mockResolvedValue(envelope({
      status: 'correct',
      is_correct: true,
      expected: { type: 'integer', value: '2' },
      actual: { type: 'integer', value: '2' },
      warnings: ['fraction_not_reduced'],
    }));
    const applyEditorAction = vi.fn().mockResolvedValue(envelope(emptyEditorState()));
    const engine = createWasmDrillEngine({ apply_editor_action: applyEditorAction, grade_answer: gradeAnswer });

    await engine.applyEditorAction(emptyEditorState(), { kind: 'clear' });
    const result = await engine.gradeAnswer({
      schema_version: 2,
      worksheet,
      answers: worksheet.problems.map((problem) => ({ problem_id: problem.problem_id, editor_state: emptyEditorState() })),
    });

    expect(JSON.parse(applyEditorAction.mock.calls[0]?.[0] as string)).toEqual({
      schema_version: 2,
      state: { answer: { type: 'empty' }, cursor: 0, committed: false },
      action: { type: 'clear' },
    });
    expect(gradeAnswer).toHaveBeenCalledTimes(20);
    expect(JSON.parse(gradeAnswer.mock.calls[0]?.[0] as string)).toEqual({
      schema_version: 2,
      expected: worksheet.problems[0]?.canonical_answer,
      actual: { type: 'empty' },
    });
    expect(result).toMatchObject({ schema_version: 2, correct_count: 20, total_count: 20 });
    expect(result.items[0]?.warnings).toEqual(['fraction_not_reduced']);
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
      schema_version: 2,
      worksheet,
      answers: worksheet.problems.map((problem) => ({
        problem_id: problem.problem_id,
        editor_state: emptyEditorState(),
      })),
    })).rejects.toMatchObject({ kind: 'invalid_dto' });
  });

  it('rejects grade warnings attached to an incorrect answer', async () => {
    const worksheet = fixtureWorksheet();
    const gradeAnswer = vi.fn().mockResolvedValue(envelope({
      status: 'incorrect',
      is_correct: false,
      expected: { type: 'integer', value: '2' },
      actual: { type: 'integer', value: '3' },
      warnings: ['redundant_decimal'],
    }));
    const engine = createWasmDrillEngine({ grade_answer: gradeAnswer });

    await expect(engine.gradeAnswer({
      schema_version: 2,
      worksheet,
      answers: worksheet.problems.map((problem) => ({
        problem_id: problem.problem_id,
        editor_state: emptyEditorState(),
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

  it('preserves the typed answer AST size error from the editor boundary', async () => {
    const runtime = {
      apply_editor_action: vi.fn().mockResolvedValue({
        schema_version: 2,
        ok: false,
        data: null,
        error: { code: 'answer_ast_size_limit', details: { max_size: 18 } },
      }),
    };
    const engine = createWasmDrillEngine(runtime);
    await expect(engine.applyEditorAction(emptyEditorState(), { kind: 'insert_digit', digit: 1 }))
      .rejects.toMatchObject({ kind: 'answer_ast_size_limit' });
  });

  it('preserves 18-digit integers as exact decimal strings', async () => {
    const exact = '999999999999999999';
    const runtime = {
      apply_editor_action: vi.fn().mockResolvedValue(envelope({
        answer: { type: 'integer', value: exact },
        cursor: 18,
        committed: false,
      })),
    };
    const engine = createWasmDrillEngine(runtime);
    const result = await engine.applyEditorAction(emptyEditorState(), { kind: 'insert_digit', digit: 9 });
    expect(result.answer).toEqual({ type: 'integer', value: exact });
    expect(JSON.parse(JSON.stringify(result.answer))).toEqual({ type: 'integer', value: exact });
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

  it('rejects exact-decimal scales outside the Rust u32 range', async () => {
    const runtime = {
      apply_editor_action: vi.fn().mockResolvedValue(envelope({
        answer: { type: 'exact_decimal', value: { coefficient: '3', scale: -1 } },
        cursor: 0,
        committed: false,
      })),
    };
    const engine = createWasmDrillEngine(runtime);
    await expect(engine.applyEditorAction(emptyEditorState(), { kind: 'clear' }))
      .rejects.toMatchObject({ kind: 'invalid_dto' });
  });
});
