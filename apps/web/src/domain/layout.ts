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
    column: index % columns,
    row: Math.floor(index / columns),
  }));

  return {
    page: A4_PAGE,
    layout: worksheet.layout,
    cells,
    dividerX: A4_PAGE.margin + columnWidth,
  };
}

export function getCellPosition(
  model: SharedWorksheetLayout,
  cell: ProblemCell,
): { x: number; y: number; width: number; height: number } {
  const top = getCellTopPosition(model, cell);
  return {
    ...top,
    // pdf-lib uses a bottom-origin y coordinate while the shared row model
    // (and the Web CSS grid) counts from the top. Flip the row's top-origin
    // offset so row 0 is physically above row 9 on the PDF page.
    y: model.page.height - top.y - top.height,
  };
}

/**
 * Return a cell in the top-origin coordinate system used by CSS layout.
 * Keeping this beside the PDF bottom-origin conversion makes the A4 margins,
 * reserved top/bottom areas, row order, and cell sizes one shared contract.
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
