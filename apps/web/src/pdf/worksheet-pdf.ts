import { degrees, PDFDocument, StandardFonts, rgb } from 'pdf-lib';

import { A4_PAGE, buildSharedWorksheetLayout, getCellPosition } from '@/domain/layout';
import { integerAnswerValue, type ProblemDto, type WorksheetDto } from '@/domain/drill-engine';
import { formatWorksheetFooter, type WorksheetMetadata } from '@/domain/worksheet-metadata';

export type PdfPageModel = {
  kind: 'problems' | 'answers';
  rotated: boolean;
  footer?: {
    text: string;
    physical_corner: 'bottom-right';
  };
  cells: readonly {
    number: string;
    problem_id: string;
    expression: string;
    answer?: string;
  }[];
};

export type FooterPageGeometry = {
  width: number;
  height: number;
  margin: number;
};

export type FooterPosition = {
  x: number;
  y: number;
  rotated: boolean;
  /** Counter-rotation keeps metadata readable after the answer page rotates. */
  text_rotation: 0 | 180;
};

export type PhysicalBounds = {
  left: number;
  right: number;
  bottom: number;
  top: number;
};

export type PdfProblemLineGeometry = {
  expressionX: number;
  answerBoxX: number;
  answerBoxWidth: number;
  answerGap: number;
};

/** Keep the printable answer box immediately after the rendered equals sign. */
export function getPdfProblemLineGeometry(
  cell: { x: number; width: number },
  expressionWidth: number,
): PdfProblemLineGeometry {
  const expressionX = cell.x + 32;
  const answerBoxWidth = 25;
  const answerGap = 6;
  const rightInset = 8;
  return {
    expressionX,
    answerBoxX: Math.min(expressionX + expressionWidth + answerGap, cell.x + cell.width - answerBoxWidth - rightInset),
    answerBoxWidth,
    answerGap,
  };
}

/**
 * Place a small footer so it is physically bottom-right on both pages. A
 * 180-degree page rotation mirrors the unrotated coordinates. The answer page
 * draws from the unrotated top edge and counter-rotates the text so it remains
 * readable while landing at the physical bottom-right.
 */
export function getFooterPosition(
  page: FooterPageGeometry,
  textWidth: number,
  fontSize: number,
  rotated: boolean,
): FooterPosition {
  return rotated
    ? { x: page.margin + textWidth, y: page.height - page.margin, rotated, text_rotation: 180 }
    : { x: page.width - page.margin - textWidth, y: page.margin + 4, rotated, text_rotation: 0 };
}

export function getFooterPhysicalBounds(
  page: FooterPageGeometry,
  position: FooterPosition,
  textWidth: number,
  fontSize: number,
): PhysicalBounds {
  const user = position.text_rotation === 180
    ? {
        left: position.x - textWidth,
        right: position.x,
        bottom: position.y - fontSize,
        top: position.y,
      }
    : {
        left: position.x,
        right: position.x + textWidth,
        bottom: position.y,
        top: position.y + fontSize,
      };
  if (!position.rotated) return user;
  return {
    left: page.width - user.right,
    right: page.width - user.left,
    bottom: page.height - user.top,
    top: page.height - user.bottom,
  };
}

/**
 * A serializable description used by both the PDF renderer and focused tests.
 * Geometry is always derived from the same layout model as the web worksheet.
 */
export function buildPdfPageModel(worksheet: WorksheetDto, metadata?: WorksheetMetadata): readonly PdfPageModel[] {
  const layout = buildSharedWorksheetLayout(worksheet);
  const cells = layout.cells.map(({ problem }, index) => ({
    number: `${index + 1}.`,
    problem_id: problem.problem_id,
    expression: `${problem.prompt.left} + ${problem.prompt.right} =`,
    answer: integerAnswerValue(problem.canonical_answer) ?? undefined,
  }));
  return [
    {
      kind: 'problems',
      rotated: false,
      ...(metadata ? { footer: { text: formatWorksheetFooter(metadata), physical_corner: 'bottom-right' as const } } : {}),
      cells: cells.map(({ number, problem_id, expression }) => ({ number, problem_id, expression })),
    },
    {
      kind: 'answers',
      rotated: true,
      ...(metadata ? { footer: { text: formatWorksheetFooter(metadata), physical_corner: 'bottom-right' as const } } : {}),
      cells,
    },
  ];
}

function drawFooter(
  page: ReturnType<PDFDocument['addPage']>,
  font: Awaited<ReturnType<PDFDocument['embedFont']>>,
  metadata: WorksheetMetadata,
  rotated: boolean,
) {
  const fontSize = 7;
  const text = formatWorksheetFooter(metadata);
  const textWidth = font.widthOfTextAtSize(text, fontSize);
  const position = getFooterPosition(A4_PAGE, textWidth, fontSize, rotated);
  page.drawText(text, {
    x: position.x,
    y: position.y,
    size: fontSize,
    font,
    rotate: degrees(position.text_rotation),
    color: rgb(0.4, 0.4, 0.4),
  });
}

function drawProblemPage(
  page: ReturnType<PDFDocument['addPage']>,
  worksheet: WorksheetDto,
  font: Awaited<ReturnType<PDFDocument['embedFont']>>,
  metadata?: WorksheetMetadata,
) {
  const layout = buildSharedWorksheetLayout(worksheet);
  page.drawLine({
    start: { x: layout.dividerX, y: layout.page.margin + layout.page.footerHeight },
    end: { x: layout.dividerX, y: layout.page.height - layout.page.margin - layout.page.headerHeight },
    thickness: 0.7,
    color: rgb(0.4, 0.4, 0.4),
  });

  for (const cell of layout.cells) {
    const position = getCellPosition(layout, cell);
    const baseline = position.y + position.height / 2 + 5;
    const expression = `${cell.problem.prompt.left} + ${cell.problem.prompt.right} =`;
    const line = getPdfProblemLineGeometry(position, font.widthOfTextAtSize(expression, 18));
    page.drawText(`${cell.index + 1}.`, {
      x: position.x + 8,
      y: baseline + 1,
      size: 10,
      font,
      color: rgb(0.25, 0.25, 0.25),
    });
    page.drawText(expression, {
      x: line.expressionX,
      y: baseline,
      size: 18,
      font,
      color: rgb(0, 0, 0),
    });
    page.drawRectangle({
      x: line.answerBoxX,
      y: baseline - 4,
      width: line.answerBoxWidth,
      height: 25,
      borderWidth: 1,
      borderColor: rgb(0, 0, 0),
      color: rgb(1, 1, 1),
    });
  }
  if (metadata) drawFooter(page, font, metadata, false);
}

function drawAnswerPage(
  page: ReturnType<PDFDocument['addPage']>,
  worksheet: WorksheetDto,
  font: Awaited<ReturnType<PDFDocument['embedFont']>>,
  metadata?: WorksheetMetadata,
) {
  const layout = buildSharedWorksheetLayout(worksheet);
  page.setRotation(degrees(180));
  page.drawLine({
    start: { x: layout.dividerX, y: layout.page.margin + layout.page.footerHeight },
    end: { x: layout.dividerX, y: layout.page.height - layout.page.margin - layout.page.headerHeight },
    thickness: 0.7,
    color: rgb(0.4, 0.4, 0.4),
  });

  for (const cell of layout.cells) {
    const position = getCellPosition(layout, cell);
    const baseline = position.y + position.height / 2 + 5;
    page.drawText(`${cell.index + 1}.`, {
      x: position.x + 8,
      y: baseline + 1,
      size: 10,
      font,
      color: rgb(0.25, 0.25, 0.25),
    });
    page.drawText(`${cell.problem.prompt.left} + ${cell.problem.prompt.right} = ${integerAnswerValue(cell.problem.canonical_answer) ?? ''}`, {
      x: position.x + 24,
      y: baseline,
      size: 17,
      font,
      color: rgb(0, 0, 0),
    });
  }
  if (metadata) drawFooter(page, font, metadata, true);
}

/** Create actual PDF bytes entirely in the browser. */
export async function generateWorksheetPdfBytes(worksheet: WorksheetDto, metadata?: WorksheetMetadata): Promise<Uint8Array> {
  const pdf = await PDFDocument.create();
  // Standard Helvetica keeps this path self-contained and avoids any runtime
  // network/font fetch. The PDF content is intentionally numeric/ASCII; the
  // web ribbon retains the Japanese curriculum labels.
  const font = await pdf.embedFont(StandardFonts.Helvetica);
  const problemPage = pdf.addPage([595.28, 841.89]);
  drawProblemPage(problemPage, worksheet, font, metadata);
  const answerPage = pdf.addPage([595.28, 841.89]);
  drawAnswerPage(answerPage, worksheet, font, metadata);
  return pdf.save();
}

export async function worksheetPdfBlob(worksheet: WorksheetDto, metadata?: WorksheetMetadata): Promise<Blob> {
  const bytes = await generateWorksheetPdfBytes(worksheet, metadata);
  const copy = new Uint8Array(bytes.byteLength);
  copy.set(bytes);
  return new Blob([copy.buffer], { type: 'application/pdf' });
}

export async function openWorksheetPdf(
  worksheet: WorksheetDto,
  targetWindow?: Window | null,
  metadata?: WorksheetMetadata,
): Promise<void> {
  if (typeof window === 'undefined') return;
  // Open the tab before the first await so a direct button click is not
  // mistaken for a delayed popup. q1 can pass a tab opened before WASM
  // generation; q2 uses this function directly.
  const opened = targetWindow ?? window.open('about:blank', '_blank');
  if (!opened) {
    throw new Error('The browser blocked the PDF tab. Please allow pop-ups for AutoDrill.');
  }
  try {
    const blob = await worksheetPdfBlob(worksheet, metadata);
    const url = URL.createObjectURL(blob);
    opened.location.href = url;
    window.setTimeout(() => URL.revokeObjectURL(url), 60_000);
  } catch (error) {
    opened.close();
    throw error;
  }
}

export function problemExpression(problem: ProblemDto): string {
  return `${problem.prompt.left} + ${problem.prompt.right} =`;
}
