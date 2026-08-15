import { arithmeticLeafText } from '@/domain/problem-format';
import { A4_PAGE } from '@/domain/layout';
import { answerNodeText, type AnswerNode, type ArithmeticExpression, type ProblemDto } from '@/domain/drill-engine';

/**
 * Printed A4 is ~793.7 CSS px wide, so 19.5pt corresponds to ~26px.
 * The same unit is expressed in cqw so Web preview and native print scale together.
 */
export const COLUMN_ARITHMETIC_GRID_POINT = 19.5;
export const COLUMN_ARITHMETIC_GRID_CQW = (COLUMN_ARITHMETIC_GRID_POINT / A4_PAGE.width) * 100;

function expressionScale(expression: ArithmeticExpression): number {
  return expression.kind === 'exact_decimal' ? expression.scale : 0;
}

function answerScale(answer: AnswerNode): number {
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
  return Math.max(normalizedDividendScale, answerScale(problem.canonical_answer));
}

type CellGeometry = { x: number; y: number; width: number };

function columnArithmeticLaneCells(problem: ProblemDto): { operatorCells: number; digitCells: number } {
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

export function columnArithmeticGridVariables(problem: ProblemDto, cell?: CellGeometry): Record<string, string> {
  if (problem.prompt.kind !== 'column_arithmetic') return {};
  const { operatorCells, digitCells } = columnArithmeticLaneCells(problem);
  const variables: Record<string, string> = {
    '--column-operator-width': `calc(${operatorCells} * var(--worksheet-grid-cell))`,
    '--column-digit-width': `calc(${digitCells} * var(--worksheet-grid-cell))`,
  };

  if (cell) {
    const laneWidth = (operatorCells + digitCells) * COLUMN_ARITHMETIC_GRID_POINT;
    const cellRight = cell.x + cell.width;
    const desiredRight = cellRight - COLUMN_ARITHMETIC_GRID_POINT * 0.35;
    const snappedRight = Math.max(
      cell.x + laneWidth,
      Math.min(
        cellRight,
        Math.round(desiredRight / COLUMN_ARITHMETIC_GRID_POINT) * COLUMN_ARITHMETIC_GRID_POINT,
      ),
    );
    // Keep a small physical inset so 1px rules/SVG antialiasing never cross a cell boundary.
    const rightOffset = Math.max(0, cellRight - snappedRight + 1.5);
    variables['--column-lane-right-offset'] = `${(rightOffset / A4_PAGE.width) * 100}cqw`;

    const gridOriginY = A4_PAGE.margin + A4_PAGE.headerHeight;
    const desiredTop = cell.y + COLUMN_ARITHMETIC_GRID_POINT;
    const snappedTop = gridOriginY + Math.ceil((desiredTop - gridOriginY) / COLUMN_ARITHMETIC_GRID_POINT) * COLUMN_ARITHMETIC_GRID_POINT;
    const topOffset = Math.max(0, snappedTop - cell.y);
    variables['--column-expression-top-offset'] = `${(topOffset / A4_PAGE.width) * 100}cqw`;
  }

  if (problem.prompt.operator === 'divide') {
    const quotientScale = answerScale(problem.canonical_answer);
    const targetScale = columnDivisionTargetScale(problem);
    variables['--column-division-active-width'] = `calc(${digitCells} * var(--worksheet-grid-cell))`;
    variables['--column-division-quotient-trailing-width'] = `calc(${Math.max(0, targetScale - quotientScale)} * var(--worksheet-grid-cell))`;
  }
  return variables;
}
