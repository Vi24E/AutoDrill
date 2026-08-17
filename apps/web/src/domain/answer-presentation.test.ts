import { describe, expect, it } from 'vitest';

import { answerCoordinate, answerPresentationPlan } from './answer-presentation';
import type { ProblemDto } from './drill-engine';
import { fixtureWorksheet } from '@/test/fixtures';

function problem(overrides: Partial<ProblemDto>): ProblemDto {
  return { ...fixtureWorksheet().problems[0]!, ...overrides };
}

describe('answerPresentationPlan', () => {
  it('projects answer semantics once for both Web and PDF renderers', () => {
    expect(answerPresentationPlan(problem({}))).toEqual({ kind: 'standard' });
    expect(answerPresentationPlan(problem({
      prompt: { kind: 'liar_puzzle', people_count: 4, statements: [] },
    }))).toEqual({ kind: 'liar_puzzle', peopleCount: 4 });
    expect(answerPresentationPlan(problem({
      prompt: {
        kind: 'column_arithmetic',
        operator: 'divide',
        left: { kind: 'integer', value: 12 },
        right: { kind: 'integer', value: 5 },
      },
      answer_schema: { kind: 'ordered_pair' },
    }))).toEqual({ kind: 'column_division', hasRemainder: true, quotientSlot: 'quotient' });
    expect(answerPresentationPlan(problem({
      prompt: {
        kind: 'column_arithmetic',
        operator: 'divide',
        left: { kind: 'integer', value: 12 },
        right: { kind: 'integer', value: 4 },
      },
      answer_schema: { kind: 'integer', min: '0', max: '99' },
    }))).toEqual({ kind: 'column_division', hasRemainder: false, quotientSlot: 'single' });
    expect(answerPresentationPlan(problem({
      prompt: { kind: 'simultaneous_equation', a: 1, b: 2, c: 3, d: 4, e: 5, f: 6 },
    }))).toEqual({ kind: 'simultaneous_equation' });
  });

  it('extracts tuple coordinates without duplicating renderer-specific fallback logic', () => {
    const tuple = { type: 'tuple', value: [{ type: 'integer', value: '3' }, { type: 'integer', value: '4' }] } satisfies import('./drill-engine').AnswerNode;
    expect(answerCoordinate(tuple, 0)).toEqual({ type: 'integer', value: '3' });
    expect(answerCoordinate(tuple, 1)).toEqual({ type: 'integer', value: '4' });
    expect(answerCoordinate({ type: 'integer', value: '3' }, 0)).toEqual({ type: 'empty' });
  });
});
