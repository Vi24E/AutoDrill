import { arithmeticLeafText } from '@/domain/problem-format';
import { A4_PAGE } from '@/domain/layout';
import { WORKSHEET_GRID_POINT } from '@/domain/worksheet-grid-presentation';

const COLUMN_DIVISION_WORK_ROWS = 3;
const COLUMN_REMAINDER_CELLS = 2;
const COLUMN_LANE_RIGHT_SHIFT_CELLS = -2;
import { answerNodeText, type AnswerNode, type ArithmeticExpression, type ProblemDto } from '@/domain/drill-engine';

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

type ColumnOperandShape = { coefficientDigits: number; scale: number; wholeDigits: number };

function columnOperandShape(expression: ArithmeticExpression): ColumnOperandShape {
  if (expression.kind === 'integer') {
    const coefficientDigits = Math.abs(expression.value).toString().length;
    return { coefficientDigits, scale: 0, wholeDigits: coefficientDigits };
  }
  if (expression.kind === 'exact_decimal') {
    const coefficientDigits = Math.abs(expression.coefficient).toString().length;
    return {
      coefficientDigits,
      scale: expression.scale,
      wholeDigits: Math.max(0, coefficientDigits - expression.scale),
    };
  }
  throw new Error('Column arithmetic presentation requires scalar integer or exact-decimal operands.');
}

function maximumNonDivisionAnswerCells(
  operator: 'add' | 'subtract' | 'multiply',
  left: ArithmeticExpression,
  right: ArithmeticExpression,
): number {
  const leftShape = columnOperandShape(left);
  const rightShape = columnOperandShape(right);
  if (operator === 'multiply') {
    const coefficientDigits = leftShape.coefficientDigits + rightShape.coefficientDigits;
    const scale = leftShape.scale + rightShape.scale;
    return Math.max(coefficientDigits, scale + 1);
  }

  const scale = Math.max(leftShape.scale, rightShape.scale);
  const widestWholePart = Math.max(leftShape.wholeDigits, rightShape.wholeDigits);
  const wholeDigits = operator === 'add'
    ? widestWholePart + 1
    : Math.max(1, widestWholePart);
  return wholeDigits + scale;
}

type LongDivisionWorkedSolution = Extract<NonNullable<ProblemDto['worked_solution']>, { kind: 'long_division' }>;

function longDivisionWorkedSolution(problem: ProblemDto): LongDivisionWorkedSolution {
  const worked = problem.worked_solution;
  if (!worked || worked.kind !== 'long_division') {
    throw new Error('Column division presentation requires Rust long-division worked solution metadata.');
  }
  return worked;
}

type CellGeometry = { x: number; y: number; width: number };

export function columnArithmeticLaneCells(problem: ProblemDto): { operatorCells: number; operandCells: number; digitCells: number; totalCells: number } {
  if (problem.prompt.kind !== 'column_arithmetic') return { operatorCells: 1, operandCells: 2, digitCells: 2, totalCells: 3 };
  const leftText = arithmeticLeafText(problem.prompt.left);
  const rightText = arithmeticLeafText(problem.prompt.right);
  const leftCells = gridCellCount(leftText);
  const rightCells = gridCellCount(rightText);

  if (problem.prompt.operator !== 'divide') {
    const maximumAnswerCells = maximumNonDivisionAnswerCells(
      problem.prompt.operator,
      problem.prompt.left,
      problem.prompt.right,
    );
    const operatorCells = 1;
    const operandCells = Math.max(leftCells, rightCells);
    const digitCells = Math.max(2, maximumAnswerCells);
    return {
      operatorCells,
      operandCells,
      digitCells,
      totalCells: Math.max(digitCells, operatorCells + operandCells),
    };
  }

  const answerText = answerNodeText(answerScalar(problem.canonical_answer));

  const worked = longDivisionWorkedSolution(problem);
  const normalizedDividendDigits = Math.max(
    Math.abs(worked.dividend_coefficient).toString().length,
    worked.dividend_scale + 1,
  );
  const quotientDigits = gridCellCount(answerText) + worked.quotient_trailing_cells;

  const operatorCells = Math.max(1, gridCellCount(rightText));
  const operandCells = Math.max(leftCells, rightCells);
  const digitCells = Math.max(2, gridCellCount(leftText), normalizedDividendDigits, quotientDigits);
  return {
    operatorCells,
    operandCells,
    digitCells,
    totalCells: operatorCells + digitCells,
  };
}

export function columnArithmeticDigitCells(problem: ProblemDto): number {
  return columnArithmeticLaneCells(problem).digitCells;
}

export function columnArithmeticGridVariables(problem: ProblemDto, cell?: CellGeometry): Record<string, string> {
  if (problem.prompt.kind !== 'column_arithmetic') return {};
  const { operatorCells, operandCells, digitCells, totalCells } = columnArithmeticLaneCells(problem);
  const variables: Record<string, string> = {
    '--column-operator-width': `calc(${operatorCells} * var(--worksheet-grid-cell))`,
    '--column-operand-width': `calc(${operandCells} * var(--worksheet-grid-cell))`,
    '--column-digit-width': `calc(${digitCells} * var(--worksheet-grid-cell))`,
    '--column-total-width': `calc(${totalCells} * var(--worksheet-grid-cell))`,
  };

  if (cell) {
    const cellRight = cell.x + cell.width;
    // The page grid is the visual coordinate system. Equal-width logical problem
    // cells are not grid-aligned, so anchor each worksheet column to an evenly
    // spaced page-grid line instead of clamping a wide lane to the invisible cell
    // boundary. The user-selected lane shift moves every written-arithmetic body
    // by the same integer number of grid cells, preserving page-wide alignment.
    const columnIndex = Math.max(0, Math.round((cell.x - A4_PAGE.margin) / cell.width));
    const firstAnchor = Math.floor((A4_PAGE.margin + cell.width) / WORKSHEET_GRID_POINT);
    const anchorStride = Math.round(cell.width / WORKSHEET_GRID_POINT);
    const snappedRight = (
      firstAnchor
      + COLUMN_LANE_RIGHT_SHIFT_CELLS
      + columnIndex * anchorStride
    ) * WORKSHEET_GRID_POINT;
    const rightOffset = cellRight - snappedRight;
    variables['--column-lane-right-offset'] = `${(rightOffset / A4_PAGE.width) * 100}cqw`;

    const gridOriginY = A4_PAGE.margin + A4_PAGE.headerHeight;
    const desiredTop = cell.y + WORKSHEET_GRID_POINT;
    const snappedTop = gridOriginY + Math.ceil((desiredTop - gridOriginY) / WORKSHEET_GRID_POINT) * WORKSHEET_GRID_POINT;
    const topOffset = Math.max(0, snappedTop - cell.y);
    variables['--column-expression-top-offset'] = `${(topOffset / A4_PAGE.width) * 100}cqw`;
  }

  if (problem.prompt.operator === 'divide') {
    const worked = longDivisionWorkedSolution(problem);
    variables['--column-division-active-width'] = `calc(${digitCells} * var(--worksheet-grid-cell))`;
    variables['--column-division-work-rows'] = String(COLUMN_DIVISION_WORK_ROWS);
    variables['--column-remainder-width'] = `calc(${COLUMN_REMAINDER_CELLS} * var(--worksheet-grid-cell))`;
    variables['--column-division-quotient-trailing-width'] = `calc(${worked.quotient_trailing_cells} * var(--worksheet-grid-cell))`;
  }
  return variables;
}
