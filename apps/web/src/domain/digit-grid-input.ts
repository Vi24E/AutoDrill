import type { AnswerInputInterface, AnswerNode, ProblemDto } from '@/domain/drill-engine';

export type DigitGridInputInterface = Extract<AnswerInputInterface, { type: 'digit_grid' }>;

export function emptyDigitGridAnswer(input: DigitGridInputInterface): AnswerNode {
  return {
    type: 'tuple',
    value: Array.from({ length: input.cell_count }, () => ({ type: 'empty' } as const)),
  };
}

export function initialDigitGridAnswer(problem: ProblemDto): AnswerNode {
  if (problem.input_interface.type !== 'digit_grid') return { type: 'empty' };
  if (problem.prompt.kind !== 'mini_sudoku') return emptyDigitGridAnswer(problem.input_interface);
  return {
    type: 'tuple',
    value: problem.prompt.givens.map((given) => given === null
      ? ({ type: 'empty' } as const)
      : ({ type: 'integer', value: String(given) } as const)),
  };
}

export function digitGridValues(answer: AnswerNode, input: DigitGridInputInterface): Array<number | null> {
  const values = answer.type === 'tuple' ? answer.value : [];
  return Array.from({ length: input.cell_count }, (_, index) => {
    const value = values[index];
    if (value?.type !== 'integer') return null;
    const parsed = Number(value.value);
    return Number.isSafeInteger(parsed) && parsed >= input.min_digit && parsed <= input.max_digit
      ? parsed
      : null;
  });
}

export function replaceDigitGridCell(
  answer: AnswerNode,
  input: DigitGridInputInterface,
  cellIndex: number,
  digit: number | null,
): AnswerNode {
  if (!Number.isInteger(cellIndex) || cellIndex < 0 || cellIndex >= input.cell_count) return answer;
  if (digit !== null && (!Number.isInteger(digit) || digit < input.min_digit || digit > input.max_digit)) return answer;
  const values = answer.type === 'tuple'
    ? Array.from({ length: input.cell_count }, (_, index) => answer.value[index] ?? ({ type: 'empty' } as const))
    : Array.from({ length: input.cell_count }, () => ({ type: 'empty' } as const));
  values[cellIndex] = digit === null
    ? ({ type: 'empty' } as const)
    : ({ type: 'integer', value: String(digit) } as const);
  return { type: 'tuple', value: values };
}
