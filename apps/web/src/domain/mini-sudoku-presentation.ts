import type { ProblemDto } from '@/domain/drill-engine';
import type { CellGeometry } from '@/domain/layout';
import {
  WORKSHEET_GRID_POINT,
  worksheetGridColumnAt,
  worksheetGridLineX,
  worksheetGridLineY,
  worksheetGridPointOffsetCqw,
  worksheetGridRowAt,
} from '@/domain/worksheet-grid-presentation';

export const MINI_SUDOKU_PAGE_GRID_CELL_SPAN = 2;

const MINI_SUDOKU_BOARD_TOP_CELLS = 2;
const MINI_SUDOKU_NUMBER_COLUMN_OFFSET_CELLS = -2;
const MINI_SUDOKU_NUMBER_ROW_OFFSET_CELLS = -1;

export function miniSudokuSide(problem: ProblemDto): number {
  if (problem.prompt.kind !== 'mini_sudoku' || problem.input_interface.type !== 'digit_grid') {
    throw new Error('Mini Sudoku presentation requires a digit-grid puzzle problem.');
  }
  const side = Math.sqrt(problem.input_interface.cell_count);
  if (!Number.isInteger(side)) throw new Error('Digit-grid cell count must form a square.');
  return side;
}

/**
 * Place the Mini Sudoku board and its problem number on integer page-grid lines.
 * The logical problem cell selects the region only; it is not a second coordinate system.
 */
export function miniSudokuGridVariables(problem: ProblemDto, cell: CellGeometry): Record<string, string> {
  const boardCells = miniSudokuSide(problem) * MINI_SUDOKU_PAGE_GRID_CELL_SPAN;
  const boardWidth = boardCells * WORKSHEET_GRID_POINT;
  const boardColumn = worksheetGridColumnAt(cell.x + (cell.width - boardWidth) / 2);
  const boardRow = worksheetGridRowAt(cell.y + MINI_SUDOKU_BOARD_TOP_CELLS * WORKSHEET_GRID_POINT);
  const boardX = worksheetGridLineX(boardColumn);
  const boardY = worksheetGridLineY(boardRow);
  const numberX = worksheetGridLineX(boardColumn + MINI_SUDOKU_NUMBER_COLUMN_OFFSET_CELLS);
  const numberY = worksheetGridLineY(boardRow + MINI_SUDOKU_NUMBER_ROW_OFFSET_CELLS);

  return {
    '--mini-sudoku-grid-left': worksheetGridPointOffsetCqw(boardX - cell.x),
    '--mini-sudoku-grid-top': worksheetGridPointOffsetCqw(boardY - cell.y),
    '--mini-sudoku-number-left': worksheetGridPointOffsetCqw(numberX - cell.x),
    '--mini-sudoku-number-top': worksheetGridPointOffsetCqw(numberY - cell.y),
  };
}
