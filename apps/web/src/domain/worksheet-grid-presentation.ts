import { A4_PAGE } from '@/domain/layout';

/**
 * Canonical A4 worksheet grid geometry. Column arithmetic and grid-based puzzles
 * share this page coordinate system; neither feature owns a second copy.
 */
export const WORKSHEET_GRID_POINT = 19.5;

export function worksheetPageGridVariables(): Record<string, string> {
  return {
    '--worksheet-grid-cell': `${(WORKSHEET_GRID_POINT / A4_PAGE.width) * 100}cqw`,
    '--worksheet-grid-top': `${((A4_PAGE.margin + A4_PAGE.headerHeight) / A4_PAGE.height) * 100}%`,
  };
}
