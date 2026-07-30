import { PDFDocument } from 'pdf-lib';
import { describe, expect, it } from 'vitest';

import { A4_PAGE, buildSharedWorksheetLayout, getCellPosition } from '@/domain/layout';
import {
  buildPdfPageModel,
  generateWorksheetPdfBytes,
  getFooterPhysicalBounds,
  getFooterPosition,
  getPdfProblemLineGeometry,
} from '@/pdf/worksheet-pdf';
import { fixtureWorksheet } from '@/test/fixtures';
import type { WorksheetMetadata } from '@/domain/worksheet-metadata';

describe('shared worksheet layout and PDF', () => {
  const metadata: WorksheetMetadata = { generated_date: '2026-07-30', seed: 'repeatMe' };

  it('uses the same 2 x 10 model for web geometry and PDF pages', () => {
    const worksheet = fixtureWorksheet();
    const layout = buildSharedWorksheetLayout(worksheet);
    const pages = buildPdfPageModel(worksheet);
    expect(layout.cells).toHaveLength(20);
    expect(layout.cells[0]).toMatchObject({ column: 0, row: 0 });
    expect(layout.cells[19]).toMatchObject({ column: 1, row: 9 });
    const firstPosition = getCellPosition(layout, layout.cells[0]!);
    const lastPosition = getCellPosition(layout, layout.cells[19]!);
    expect(firstPosition.y).toBeGreaterThan(lastPosition.y);
    expect(firstPosition.y + firstPosition.height).toBeCloseTo(A4_PAGE.height - A4_PAGE.margin - A4_PAGE.headerHeight, 5);
    expect(lastPosition.y).toBeCloseTo(A4_PAGE.margin + A4_PAGE.footerHeight, 5);
    expect(pages).toHaveLength(2);
    expect(pages[0]).toMatchObject({ kind: 'problems', rotated: false });
    expect(pages[1]).toMatchObject({ kind: 'answers', rotated: true });
    expect(pages[0]?.cells[0]?.problem_id).toBe(layout.cells[0]?.problem.problem_id);
    expect(pages[0]?.cells[0]?.number).toBe('1.');
    expect(pages[0]?.cells[19]?.number).toBe('20.');
    expect(pages[1]?.cells[0]?.number).toBe('1.');
  });

  it('creates two actual A4 PDF pages and rotates the answer page', async () => {
    const bytes = await generateWorksheetPdfBytes(fixtureWorksheet(), metadata);
    const document = await PDFDocument.load(bytes);
    expect(document.getPages()).toHaveLength(2);
    expect(document.getPages()[0]?.getSize().width).toBeCloseTo(595.28, 1);
    expect(document.getPages()[1]?.getRotation().angle).toBe(180);
  });

  it('places the printable answer box immediately after the equals sign', () => {
    const line = getPdfProblemLineGeometry({ x: 42, width: 255 }, 71);
    expect(line.answerBoxX).toBe(line.expressionX + 71 + line.answerGap);
    expect(line.answerBoxX).toBeLessThan(42 + 255 - 56);
    expect(line.answerBoxWidth).toBe(25);
  });

  it('keeps identical metadata in both footer models and maps the rotated footer to physical bottom-right', () => {
    const pages = buildPdfPageModel(fixtureWorksheet(), metadata);
    expect(pages[0]?.footer).toEqual({ text: 'date: 2026-07-30 / seed: repeatMe', physical_corner: 'bottom-right' });
    expect(pages[1]?.footer).toEqual(pages[0]?.footer);

    const textWidth = 140;
    const fontSize = 7;
    const normal = getFooterPosition(A4_PAGE, textWidth, fontSize, false);
    const rotated = getFooterPosition(A4_PAGE, textWidth, fontSize, true);
    const normalPhysical = getFooterPhysicalBounds(A4_PAGE, normal, textWidth, fontSize);
    const rotatedPhysical = getFooterPhysicalBounds(A4_PAGE, rotated, textWidth, fontSize);

    expect(normalPhysical.right).toBeCloseTo(A4_PAGE.width - A4_PAGE.margin, 5);
    expect(rotatedPhysical.right).toBeCloseTo(A4_PAGE.width - A4_PAGE.margin, 5);
    expect(normalPhysical.bottom).toBeCloseTo(A4_PAGE.margin + 4, 5);
    expect(rotatedPhysical.bottom).toBeCloseTo(A4_PAGE.margin, 5);
    expect(rotated.x).toBe(A4_PAGE.margin + textWidth);
    expect(rotated.y).toBe(A4_PAGE.height - A4_PAGE.margin);
    expect(rotated.text_rotation).toBe(180);
  });
});
