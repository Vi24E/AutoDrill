import { describe, expect, it } from 'vitest';

import { A4_PAGE, buildSharedWorksheetLayout, getCellTopPosition } from '@/domain/layout';
import {
  MINI_SUDOKU_PAGE_GRID_CELL_SPAN,
  miniSudokuGridVariables,
  miniSudokuSide,
} from '@/domain/mini-sudoku-presentation';
import {
  WORKSHEET_GRID_ORIGIN,
  WORKSHEET_GRID_POINT,
} from '@/domain/worksheet-grid-presentation';
import { miniSudokuFixtureWorksheet } from '@/test/fixtures';

function cqwToPagePoints(value: string): number {
  const numeric = Number.parseFloat(value.replace('cqw', ''));
  return (numeric / 100) * A4_PAGE.width;
}

function expectOnGridLine(value: number, origin: number): void {
  const gridUnits = (value - origin) / WORKSHEET_GRID_POINT;
  expect(gridUnits).toBeCloseTo(Math.round(gridUnits), 8);
}

describe('mini Sudoku worksheet-grid placement', () => {
  it('anchors every board and problem number to page-grid lines with one shared relative policy', () => {
    const worksheet = miniSudokuFixtureWorksheet();
    const layout = buildSharedWorksheetLayout(worksheet);

    for (const cell of layout.cells) {
      const geometry = getCellTopPosition(layout, cell);
      const variables = miniSudokuGridVariables(cell.problem, geometry);
      const boardX = geometry.x + cqwToPagePoints(variables['--mini-sudoku-grid-left']!);
      const boardY = geometry.y + cqwToPagePoints(variables['--mini-sudoku-grid-top']!);
      const numberX = geometry.x + cqwToPagePoints(variables['--mini-sudoku-number-left']!);
      const numberY = geometry.y + cqwToPagePoints(variables['--mini-sudoku-number-top']!);

      expectOnGridLine(boardX, WORKSHEET_GRID_ORIGIN.x);
      expectOnGridLine(boardY, WORKSHEET_GRID_ORIGIN.y);
      expectOnGridLine(numberX, WORKSHEET_GRID_ORIGIN.x);
      expectOnGridLine(numberY, WORKSHEET_GRID_ORIGIN.y);
      expect((boardX - numberX) / WORKSHEET_GRID_POINT).toBeCloseTo(2, 8);
      expect((boardY - numberY) / WORKSHEET_GRID_POINT).toBeCloseTo(1, 8);
      expect(miniSudokuSide(cell.problem) * MINI_SUDOKU_PAGE_GRID_CELL_SPAN).toBe(8);
    }
  });
});
