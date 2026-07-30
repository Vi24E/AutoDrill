import { describe, expect, it, vi } from 'vitest';

import { createWasmDrillEngine } from '@/domain/wasm-adapter';
import { emptyEditorState } from '@/domain/drill-engine';
import { fixtureSettings, fixtureWorksheet } from '@/test/fixtures';

describe('versioned WASM adapter', () => {
  it('unwraps the success envelope and preserves distinct generation errors', async () => {
    const runtime = { generate_worksheet: vi.fn().mockResolvedValue({ schema_version: 1, ok: true, data: fixtureWorksheet() }) };
    const engine = createWasmDrillEngine(runtime);
    await expect(engine.generateWorksheet(fixtureSettings())).resolves.toMatchObject({ schema_version: 1, layout: { rows: 10 } });

    const timeout = createWasmDrillEngine({ generate_worksheet: vi.fn().mockResolvedValue({ schema_version: 1, ok: false, error: { kind: 'generation_timeout' } }) });
    await expect(timeout.generateWorksheet(fixtureSettings())).rejects.toMatchObject({ kind: 'generation_timeout' });
    const attempts = createWasmDrillEngine({ generate_worksheet: vi.fn().mockResolvedValue({ schema_version: 1, ok: false, error: { kind: 'generation_attempt_limit' } }) });
    await expect(attempts.generateWorksheet(fixtureSettings())).rejects.toMatchObject({ kind: 'generation_attempt_limit' });
  });

  it('delegates editor actions and one grade_answer call per worksheet problem', async () => {
    const worksheet = fixtureWorksheet();
    const gradeAnswer = vi.fn().mockResolvedValue({ schema_version: 1, ok: true, data: { schema_version: 1, status: 'ok', is_correct: true, expected: { kind: 'integer', value: 2 }, actual: { kind: 'integer', value: 2 } } });
    const runtime = {
      apply_editor_action: vi.fn().mockResolvedValue({ schema_version: 1, ok: true, data: emptyEditorState() }),
      grade_answer: gradeAnswer,
    };
    const engine = createWasmDrillEngine(runtime);
    await engine.applyEditorAction(emptyEditorState(), { kind: 'clear' });
    const result = await engine.gradeAnswer({ schema_version: 1, worksheet, answers: worksheet.problems.map((problem) => ({ problem_id: problem.problem_id, editor_state: emptyEditorState() })) });
    expect(JSON.parse(runtime.apply_editor_action.mock.calls[0]?.[0] as string)).toEqual(expect.objectContaining({ schema_version: 1 }));
    expect(gradeAnswer).toHaveBeenCalledTimes(20);
    expect(JSON.parse(gradeAnswer.mock.calls[0]?.[0] as string)).toEqual(expect.objectContaining({ expected: worksheet.problems[0]?.canonical_answer, actual: { kind: 'integer', digits: [] } }));
    expect(result.correct_count).toBe(20);
  });
});
