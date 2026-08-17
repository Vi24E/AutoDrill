import type { ProblemDto, WorksheetDto, WorksheetLayout } from './drill-engine';
import { findThemeDefinitionByNumericId } from './theme-registry';

export const A4_PAGE = {
  width: 595.28,
  height: 841.89,
  margin: 42,
  headerHeight: 48,
  footerHeight: 24,
} as const;

export type ProblemCell = {
  index: number;
  problem: ProblemDto;
  column: number;
  row: number;
};

export type SharedWorksheetLayout = {
  page: typeof A4_PAGE;
  layout: WorksheetLayout;
  cells: readonly ProblemCell[];
  /** All internal vertical boundaries between worksheet columns. */
  dividerXs: readonly number[];
};

export function buildSharedWorksheetLayout(worksheet: WorksheetDto): SharedWorksheetLayout {
  const { columns, rows } = worksheet.layout;
  const contentWidth = A4_PAGE.width - A4_PAGE.margin * 2;
  const columnWidth = contentWidth / columns;
  const theme = findThemeDefinitionByNumericId(worksheet.identity.numeric_theme_id);
  if (!theme) throw new Error(`Unknown worksheet theme ${worksheet.identity.numeric_theme_id}.`);
  const rowMajor = theme.presentation.column_arithmetic;
  const cells = worksheet.problems.map((problem, index) => ({
    index,
    problem,
    // Ordinary worksheets keep the historical vertical reading order.
    // Printable column arithmetic follows the conventional worksheet order:
    // 1–4 across the first row, then 5–8, 9–12, and 13–16.
    column: rowMajor ? index % columns : Math.floor(index / rows),
    row: rowMajor ? Math.floor(index / columns) : index % rows,
  }));

  return {
    page: A4_PAGE,
    layout: worksheet.layout,
    cells,
    dividerXs: Array.from({ length: Math.max(0, columns - 1) }, (_, index) => (
      A4_PAGE.margin + columnWidth * (index + 1)
    )),
  };
}

/**
 * Return a cell in the top-origin coordinate system used by CSS layout.
 * The interactive worksheet and browser-native print DOM both consume this
 * top-origin A4 geometry, so row order, margins, and cell sizes stay in one contract.
 */
export function getCellTopPosition(
  model: SharedWorksheetLayout,
  cell: ProblemCell,
): { x: number; y: number; width: number; height: number } {
  const contentWidth = model.page.width - model.page.margin * 2;
  const contentHeight =
    model.page.height - model.page.margin * 2 - model.page.headerHeight - model.page.footerHeight;
  const width = contentWidth / model.layout.columns;
  const height = contentHeight / model.layout.rows;
  return {
    x: model.page.margin + width * cell.column,
    y: model.page.margin + model.page.headerHeight + height * cell.row,
    width,
    height,
  };
}
