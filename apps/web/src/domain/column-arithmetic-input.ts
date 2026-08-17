import type { AnswerNode, ProblemDto } from '@/domain/drill-engine';
import { columnAnswerScale, columnArithmeticDigitCells } from '@/domain/column-arithmetic-presentation';

export type ColumnAnswerSlot = 'single' | 'quotient';
export type ColumnInputDirection = 'left-to-right' | 'right-to-left';

export type ColumnDigitSpec = {
  cellCount: number;
  scale: number;
  direction: ColumnInputDirection;
  activeStart: number;
  activeEnd: number;
  initialIndex: number;
  decimalBoundary: number | null;
};

export function columnAnswerPart(answer: AnswerNode, slot: ColumnAnswerSlot): AnswerNode {
  if (slot === 'single') return answer;
  if (answer.type !== 'tuple') return { type: 'empty' };
  return answer.value[0] ?? { type: 'empty' };
}

function canonicalPart(problem: ProblemDto, slot: ColumnAnswerSlot): AnswerNode {
  return columnAnswerPart(problem.canonical_answer, slot);
}

function firstQuotientCell(problem: ProblemDto, digitCells: number, activeEnd: number): number {
  if (problem.worked_solution?.kind !== 'long_division') return 0;
  const first = problem.worked_solution.steps[0];
  if (!first) return 0;
  const index = digitCells - first.product_offset - 1;
  return Math.max(0, Math.min(activeEnd, index));
}

export function columnDigitSpec(problem: ProblemDto, slot: ColumnAnswerSlot): ColumnDigitSpec {
  if (problem.prompt.kind !== 'column_arithmetic') {
    throw new Error('Column digit input requires a column arithmetic problem.');
  }



  const digitCells = columnArithmeticDigitCells(problem);
  const part = canonicalPart(problem, slot);
  const scale = columnAnswerScale(part);
  if (problem.prompt.operator === 'divide') {
    const trailingCells = problem.worked_solution?.kind === 'long_division'
      ? problem.worked_solution.quotient_trailing_cells
      : 0;
    const activeEnd = Math.max(0, digitCells - trailingCells - 1);
    const activeStart = firstQuotientCell(problem, digitCells, activeEnd);
    return {
      cellCount: digitCells,
      scale,
      direction: 'left-to-right',
      activeStart,
      activeEnd,
      initialIndex: activeStart,
      decimalBoundary: scale > 0 ? activeEnd + 1 - scale : null,
    };
  }

  return {
    cellCount: digitCells,
    scale,
    direction: 'right-to-left',
    activeStart: 0,
    activeEnd: digitCells - 1,
    initialIndex: digitCells - 1,
    decimalBoundary: scale > 0 ? digitCells - scale : null,
  };
}

function coefficientDigits(answer: AnswerNode): string | null {
  if (answer.type === 'integer') return answer.value.replace(/^[-−]/, '');
  if (answer.type === 'exact_decimal') {
    const digits = answer.value.coefficient.replace(/^[-−]/, '');
    return digits.padStart(answer.value.scale + 1, '0');
  }
  return null;
}

export function columnDigitsFromAnswer(answer: AnswerNode, spec: ColumnDigitSpec): Array<string | null> {
  const digits = Array.from({ length: spec.cellCount }, () => null as string | null);
  const coefficient = coefficientDigits(answer);
  if (!coefficient) return digits;
  const normalized = answer.type === 'exact_decimal'
    ? coefficient
    : coefficient.replace(/^0+(?=\d)/, '');
  const end = spec.activeEnd;
  const start = Math.max(spec.activeStart, end - normalized.length + 1);
  const visible = normalized.slice(-(end - start + 1));
  for (let offset = 0; offset < visible.length; offset += 1) digits[start + offset] = visible[offset]!;
  return digits;
}

function incompleteAnswer(raw: readonly (string | null)[]): AnswerNode {
  return { type: 'nan_error', value: `column-draft:${raw.map((digit) => digit ?? '_').join('')}` };
}

export function columnDigitsToAnswer(digits: readonly (string | null)[], spec: ColumnDigitSpec): AnswerNode {
  const active = digits.slice(spec.activeStart, spec.activeEnd + 1);
  const firstFilled = active.findIndex((digit) => digit !== null);
  if (firstFilled < 0) return { type: 'empty' };

  if (spec.scale === 0) {
    const significant = active.slice(firstFilled);
    if (significant.some((digit) => digit === null)) return incompleteAnswer(active);
    const value = significant.join('').replace(/^0+(?=\d)/, '') || '0';
    return { type: 'integer', value };
  }

  const globalBoundary = spec.decimalBoundary ?? (spec.activeEnd + 1 - spec.scale);
  const localBoundary = globalBoundary - spec.activeStart;
  const integerDigits = active.slice(0, Math.max(0, localBoundary));
  const fractionDigits = active.slice(Math.max(0, localBoundary));
  if (fractionDigits.length !== spec.scale || fractionDigits.some((digit) => digit === null)) {
    return incompleteAnswer(active);
  }

  const firstInteger = integerDigits.findIndex((digit) => digit !== null);
  const integerPart = firstInteger < 0 ? [] : integerDigits.slice(firstInteger);
  if (integerPart.some((digit) => digit === null)) return incompleteAnswer(active);
  const rawCoefficient = `${integerPart.join('') || '0'}${fractionDigits.join('')}`;
  const coefficient = BigInt(rawCoefficient || '0').toString();
  return { type: 'exact_decimal', value: { coefficient, scale: spec.scale } };
}

export function replaceColumnAnswerPart(answer: AnswerNode, slot: ColumnAnswerSlot, part: AnswerNode): AnswerNode {
  if (slot === 'single') return part;
  const remainder = answer.type === 'tuple' ? (answer.value[1] ?? { type: 'empty' }) : { type: 'empty' as const };
  return { type: 'tuple', value: [part, remainder] };
}

export function nextColumnDigitIndex(spec: ColumnDigitSpec, current: number): number {
  const delta = spec.direction === 'left-to-right' ? 1 : -1;
  return Math.max(spec.activeStart, Math.min(spec.activeEnd, current + delta));
}

export function previousColumnDigitIndex(spec: ColumnDigitSpec, current: number): number {
  const delta = spec.direction === 'left-to-right' ? -1 : 1;
  return Math.max(spec.activeStart, Math.min(spec.activeEnd, current + delta));
}
