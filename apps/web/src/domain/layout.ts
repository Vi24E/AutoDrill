import type { ProblemDto, WorksheetDto, WorksheetLayout } from './drill-engine';

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
  dividerX: number;
};

export function buildSharedWorksheetLayout(worksheet: WorksheetDto): SharedWorksheetLayout {
  const { columns, rows } = worksheet.layout;
  const contentWidth = A4_PAGE.width - A4_PAGE.margin * 2;
  const columnWidth = contentWidth / columns;
  const cells = worksheet.problems.map((problem, index) => ({
    index,
    problem,
    // Worksheets are read vertically: 1–10 down the left column, then
    // 11–20 down the right. Web and PDF consume this same placement model.
    column: Math.floor(index / rows),
    row: index % rows,
  }));

  return {
    page: A4_PAGE,
    layout: worksheet.layout,
    cells,
    dividerX: A4_PAGE.margin + columnWidth,
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
