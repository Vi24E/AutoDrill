import type { AnswerNode, ProblemDto } from './drill-engine';

export type AnswerPresentationPlan =
  | { kind: 'liar_puzzle'; peopleCount: number }
  | { kind: 'column_division'; hasRemainder: boolean; quotientSlot: 'single' | 'quotient' }
  | { kind: 'column_arithmetic' }
  | { kind: 'simultaneous_equation' }
  | { kind: 'digit_grid' }
  | { kind: 'standard' };

export function answerCoordinate(answer: AnswerNode, coordinate: 0 | 1): AnswerNode {
  return answer.type === 'tuple' && answer.value[coordinate]
    ? answer.value[coordinate]
    : ({ type: 'empty' } satisfies AnswerNode);
}

/**
 * Canonical semantic decomposition of a Problem answer. Renderers remain free to
 * choose interactive/print components, but they must not independently infer
 * whether the answer is a liar selection, column quotient/remainder, coordinate
 * pair, or ordinary scalar.
 */
export function answerPresentationPlan(problem: ProblemDto): AnswerPresentationPlan {
  if (problem.input_interface.type === 'digit_grid') return { kind: 'digit_grid' };
  if (problem.prompt.kind === 'liar_puzzle') {
    return { kind: 'liar_puzzle', peopleCount: problem.prompt.people_count };
  }
  if (problem.prompt.kind === 'column_arithmetic') {
    if (problem.prompt.operator === 'divide') {
      const hasRemainder = problem.answer_schema.kind === 'ordered_pair';
      return { kind: 'column_division', hasRemainder, quotientSlot: hasRemainder ? 'quotient' : 'single' };
    }
    return { kind: 'column_arithmetic' };
  }
  if (problem.prompt.kind === 'simultaneous_equation') {
    return { kind: 'simultaneous_equation' };
  }
  return { kind: 'standard' };
}
