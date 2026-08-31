import type {
  AnswerNode,
  ColumnAnswerPartInput,
  ColumnDecimalPointInput,
  ColumnInputOrder,
  ProblemDto,
} from '@/domain/drill-engine';
import { columnArithmeticDigitCells } from '@/domain/column-arithmetic-presentation';

export type ColumnAnswerSlot = 'single' | 'quotient';
export type ColumnInputDirection = 'left-to-right' | 'right-to-left';

export type ColumnDigitSpec = {
  cellCount: number;
  order: ColumnInputOrder;
  direction: ColumnInputDirection;
  activeStart: number;
  activeEnd: number;
  initialIndex: number;
  decimalPoint: ColumnDecimalPointInput;
  fixedDecimalBoundary: number | null;
};

export function columnAnswerPart(answer: AnswerNode, slot: ColumnAnswerSlot): AnswerNode {
  if (slot === 'single') return answer;
  if (answer.type !== 'tuple') return { type: 'empty' };
  return answer.value[0] ?? { type: 'empty' };
}

function partInput(problem: ProblemDto, slot: ColumnAnswerSlot): ColumnAnswerPartInput {
  const metadata = problem.column_input;
  const part = slot === 'single' ? metadata?.single : metadata?.quotient;
  if (!part) throw new Error(`Column input metadata is missing for ${slot}.`);
  return part;
}

function directionFor(order: ColumnInputOrder): ColumnInputDirection {
  return order === 'least_significant_first' ? 'right-to-left' : 'left-to-right';
}

function fixedDecimalBoundary(
  decimalPoint: ColumnDecimalPointInput,
  activeEnd: number,
): number | null {
  if (decimalPoint.type !== 'fixed' || decimalPoint.scale === 0) return null;
  return activeEnd + 1 - decimalPoint.scale;
}

export function columnDigitSpec(problem: ProblemDto, slot: ColumnAnswerSlot): ColumnDigitSpec {
  if (problem.prompt.kind !== 'column_arithmetic') {
    throw new Error('Column digit input requires a column arithmetic problem.');
  }

  const digitCells = columnArithmeticDigitCells(problem);
  const input = partInput(problem, slot);
  const direction = directionFor(input.order);
  if (problem.worked_solution?.kind === 'long_division') {
    const trailingCells = problem.worked_solution.quotient_trailing_cells;
    const activeEnd = Math.max(0, digitCells - trailingCells - 1);
    const activeStart = 0;
    return {
      cellCount: digitCells,
      order: input.order,
      direction,
      activeStart,
      activeEnd,
      initialIndex: direction === 'left-to-right' ? activeStart : activeEnd,
      decimalPoint: input.decimal_point,
      fixedDecimalBoundary: fixedDecimalBoundary(input.decimal_point, activeEnd),
    };
  }

  const activeStart = 0;
  const activeEnd = digitCells - 1;
  return {
    cellCount: digitCells,
    order: input.order,
    direction,
    activeStart,
    activeEnd,
    initialIndex: direction === 'left-to-right' ? activeStart : activeEnd,
    decimalPoint: input.decimal_point,
    fixedDecimalBoundary: fixedDecimalBoundary(input.decimal_point, activeEnd),
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

function decimalScale(spec: ColumnDigitSpec, decimalBoundary: number | null): number | null {
  if (spec.decimalPoint.type === 'none') return 0;
  if (spec.decimalPoint.type === 'fixed') return spec.decimalPoint.scale;
  if (decimalBoundary === null) return 0;
  return Math.max(0, spec.activeEnd + 1 - decimalBoundary);
}

export function columnDecimalBoundaryFromAnswer(answer: AnswerNode, spec: ColumnDigitSpec): number | null {
  if (spec.decimalPoint.type === 'none') return null;
  if (spec.decimalPoint.type === 'fixed') return spec.fixedDecimalBoundary;
  if (answer.type !== 'exact_decimal' || answer.value.scale === 0) return null;
  return Math.max(spec.activeStart, spec.activeEnd + 1 - answer.value.scale);
}

export function columnDigitsToAnswer(
  digits: readonly (string | null)[],
  spec: ColumnDigitSpec,
  decimalBoundary: number | null = spec.fixedDecimalBoundary,
): AnswerNode {
  const active = digits.slice(spec.activeStart, spec.activeEnd + 1);
  const firstFilled = active.findIndex((digit) => digit !== null);
  if (firstFilled < 0) return { type: 'empty' };

  const scale = decimalScale(spec, decimalBoundary);
  if (scale === null || scale === 0) {
    const significant = active.slice(firstFilled);
    if (significant.some((digit) => digit === null)) return incompleteAnswer(active);
    const value = significant.join('').replace(/^0+(?=\d)/, '') || '0';
    return { type: 'integer', value };
  }

  const globalBoundary = spec.activeEnd + 1 - scale;
  const localBoundary = globalBoundary - spec.activeStart;
  const integerDigits = active.slice(0, Math.max(0, localBoundary));
  const fractionDigits = active.slice(Math.max(0, localBoundary));
  if (fractionDigits.length !== scale || fractionDigits.some((digit) => digit === null)) {
    return incompleteAnswer(active);
  }

  const firstInteger = integerDigits.findIndex((digit) => digit !== null);
  const integerPart = firstInteger < 0 ? [] : integerDigits.slice(firstInteger);
  if (integerPart.some((digit) => digit === null)) return incompleteAnswer(active);
  const rawCoefficient = `${integerPart.join('') || '0'}${fractionDigits.join('')}`;
  const coefficient = BigInt(rawCoefficient || '0').toString();
  return { type: 'exact_decimal', value: { coefficient, scale } };
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
