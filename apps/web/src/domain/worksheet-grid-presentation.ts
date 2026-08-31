import { A4_PAGE } from '@/domain/layout';

/**
 * Canonical A4 worksheet grid geometry. Column arithmetic and grid-based puzzles
 * share this page coordinate system; neither feature owns a second copy.
 */
export const WORKSHEET_GRID_POINT = 19.5;
export const WORKSHEET_GRID_ORIGIN = {
  x: 0,
  y: A4_PAGE.margin + A4_PAGE.headerHeight,
} as const;

type GridRounding = 'nearest' | 'floor' | 'ceil';

function roundGrid(value: number, rounding: GridRounding): number {
  if (rounding === 'floor') return Math.floor(value);
  if (rounding === 'ceil') return Math.ceil(value);
  return Math.round(value);
}

export function worksheetGridColumnAt(pageX: number, rounding: GridRounding = 'nearest'): number {
  return roundGrid((pageX - WORKSHEET_GRID_ORIGIN.x) / WORKSHEET_GRID_POINT, rounding);
}

export function worksheetGridRowAt(pageY: number, rounding: GridRounding = 'nearest'): number {
  return roundGrid((pageY - WORKSHEET_GRID_ORIGIN.y) / WORKSHEET_GRID_POINT, rounding);
}

export function worksheetGridLineX(column: number): number {
  return WORKSHEET_GRID_ORIGIN.x + column * WORKSHEET_GRID_POINT;
}

export function worksheetGridLineY(row: number): number {
  return WORKSHEET_GRID_ORIGIN.y + row * WORKSHEET_GRID_POINT;
}

export function worksheetGridPointOffsetCqw(points: number): string {
  return `${(points / A4_PAGE.width) * 100}cqw`;
}

export function worksheetPageGridVariables(): Record<string, string> {
  return {
    '--worksheet-grid-cell': worksheetGridPointOffsetCqw(WORKSHEET_GRID_POINT),
    '--worksheet-grid-top': `${(WORKSHEET_GRID_ORIGIN.y / A4_PAGE.height) * 100}%`,
  };
}
