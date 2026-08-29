import { arithmeticLeafText } from '@/domain/problem-format';
import { A4_PAGE } from '@/domain/layout';
import { WORKSHEET_GRID_POINT } from '@/domain/worksheet-grid-presentation';

const COLUMN_DIVISION_WORK_ROWS = 3;
const COLUMN_REMAINDER_CELLS = 2;
import { answerNodeText, type AnswerNode, type ArithmeticExpression, type ProblemDto } from '@/domain/drill-engine';

function expressionScale(expression: ArithmeticExpression): number {
  return expression.kind === 'exact_decimal' ? expression.scale : 0;
}

export function columnAnswerScale(answer: AnswerNode): number {
  if (answer.type === 'exact_decimal') return answer.value.scale;
  if (answer.type === 'tuple' && answer.value[0]?.type === 'exact_decimal') return answer.value[0].value.scale;
  return 0;
}

function answerScalar(answer: AnswerNode): AnswerNode {
  return answer.type === 'tuple' ? (answer.value[0] ?? { type: 'empty' }) : answer;
}

function gridCellCount(text: string): number {
  return [...text].filter((character) => character !== '.' && character !== '−' && character !== '-').length;
}

export function columnDivisionTargetScale(problem: ProblemDto): number {
  if (problem.prompt.kind !== 'column_arithmetic' || problem.prompt.operator !== 'divide') return 0;
  const leftScale = expressionScale(problem.prompt.left);
  const rightScale = expressionScale(problem.prompt.right);
  const normalizedDividendScale = rightScale <= leftScale ? leftScale - rightScale : 0;
  return Math.max(normalizedDividendScale, columnAnswerScale(problem.canonical_answer));
}

type CellGeometry = { x: number; y: number; width: number };

export function columnArithmeticLaneCells(problem: ProblemDto): { operatorCells: number; digitCells: number } {
  if (problem.prompt.kind !== 'column_arithmetic') return { operatorCells: 1, digitCells: 2 };
  const leftText = arithmeticLeafText(problem.prompt.left);
  const rightText = arithmeticLeafText(problem.prompt.right);
  const answerText = answerNodeText(answerScalar(problem.canonical_answer));

  if (problem.prompt.operator !== 'divide') {
    return {
      operatorCells: 1,
      digitCells: Math.max(2, gridCellCount(leftText), gridCellCount(rightText), gridCellCount(answerText)),
    };
  }

  const leftScale = expressionScale(problem.prompt.left);
  const rightScale = expressionScale(problem.prompt.right);
  const normalizedDividendScale = rightScale <= leftScale ? leftScale - rightScale : 0;
  const targetScale = columnDivisionTargetScale(problem);
  const appendedZeros = Math.max(0, targetScale - normalizedDividendScale);
  const normalizedDividendDigits = problem.prompt.left.kind === 'integer'
    ? Math.abs(problem.prompt.left.value).toString().length + appendedZeros
    : problem.prompt.left.kind === 'exact_decimal'
      ? Math.abs(problem.prompt.left.coefficient).toString().length + appendedZeros
      : gridCellCount(leftText) + appendedZeros;

  return {
    operatorCells: Math.max(1, gridCellCount(rightText)),
    digitCells: Math.max(2, gridCellCount(leftText), normalizedDividendDigits, gridCellCount(answerText)),
  };
}


export function columnArithmeticWorkingRows(problem: ProblemDto): number {
  if (problem.prompt.kind !== 'column_arithmetic') return 0;
  if (problem.prompt.operator !== 'multiply') return 0;
  // A one-digit multiplier has no partial-product row. Multi-digit multiplication
  // reserves one grid row for handwritten partial work before the final answer.
  return gridCellCount(arithmeticLeafText(problem.prompt.right)) > 1 ? 1 : 0;
}

export function columnArithmeticDigitCells(problem: ProblemDto): number {
  return columnArithmeticLaneCells(problem).digitCells;
}

export function columnArithmeticGridVariables(problem: ProblemDto, cell?: CellGeometry): Record<string, string> {
  if (problem.prompt.kind !== 'column_arithmetic') return {};
  const { operatorCells, digitCells } = columnArithmeticLaneCells(problem);
  const variables: Record<string, string> = {
    '--column-operator-width': `calc(${operatorCells} * var(--worksheet-grid-cell))`,
    '--column-digit-width': `calc(${digitCells} * var(--worksheet-grid-cell))`,
    '--column-working-rows': String(columnArithmeticWorkingRows(problem)),
  };

  if (cell) {
    const cellRight = cell.x + cell.width;
    // The page grid is the visual coordinate system. Equal-width logical problem
    // cells are not grid-aligned, so anchor each worksheet column to an evenly
    // spaced page-grid line instead of clamping a wide lane to the invisible cell
    // boundary. The signed margin lets the last columns extend slightly right
    // when that is what the visible grid requires.
    const columnIndex = Math.max(0, Math.round((cell.x - A4_PAGE.margin) / cell.width));
    const firstAnchor = Math.floor((A4_PAGE.margin + cell.width) / WORKSHEET_GRID_POINT);
    const anchorStride = Math.round(cell.width / WORKSHEET_GRID_POINT);
    const snappedRight = (firstAnchor + columnIndex * anchorStride) * WORKSHEET_GRID_POINT;
    const rightOffset = cellRight - snappedRight;
    variables['--column-lane-right-offset'] = `${(rightOffset / A4_PAGE.width) * 100}cqw`;

    const gridOriginY = A4_PAGE.margin + A4_PAGE.headerHeight;
    const desiredTop = cell.y + WORKSHEET_GRID_POINT;
    const snappedTop = gridOriginY + Math.ceil((desiredTop - gridOriginY) / WORKSHEET_GRID_POINT) * WORKSHEET_GRID_POINT;
    const topOffset = Math.max(0, snappedTop - cell.y);
    variables['--column-expression-top-offset'] = `${(topOffset / A4_PAGE.width) * 100}cqw`;
  }

  if (problem.prompt.operator === 'divide') {
    const quotientScale = columnAnswerScale(problem.canonical_answer);
    const targetScale = columnDivisionTargetScale(problem);
    variables['--column-division-active-width'] = `calc(${digitCells} * var(--worksheet-grid-cell))`;
    variables['--column-division-work-rows'] = String(COLUMN_DIVISION_WORK_ROWS);
    variables['--column-remainder-width'] = `calc(${COLUMN_REMAINDER_CELLS} * var(--worksheet-grid-cell))`;
    variables['--column-division-quotient-trailing-width'] = `calc(${Math.max(0, targetScale - quotientScale)} * var(--worksheet-grid-cell))`;
  }
  return variables;
}
