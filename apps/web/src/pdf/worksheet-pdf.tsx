import { createRoot, type Root } from 'react-dom/client';
import { useEffect, type CSSProperties } from 'react';

import { MathLiveStatic } from '@/components/MathLiveMath';
import { ProblemExpression } from '@/components/ProblemExpression';
import { A4_PAGE, buildSharedWorksheetLayout, getCellTopPosition } from '@/domain/layout';
import { worksheetGradeBandClass } from '@/domain/grade-band';
import { answerNodeText, type WorksheetDto } from '@/domain/drill-engine';
import { answerNodeLatex } from '@/domain/mathlive-format';
import { problemExpression } from '@/domain/problem-format';
import { findThemeDefinitionByNumericId, type ThemeDefinition } from '@/domain/theme-registry';
import { formatWorksheetFooter, type WorksheetMetadata } from '@/domain/worksheet-metadata';

export type PdfPageModel = {
  kind: 'problems' | 'answers';
  rotated: boolean;
  title: string;
  instruction: string;
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

type MathSpanWithRender = HTMLElement & { render?: () => void };

function themeForWorksheet(worksheet: WorksheetDto): ThemeDefinition {
  const theme = findThemeDefinitionByNumericId(worksheet.identity.numeric_theme_id);
  if (!theme) throw new Error(`Unknown worksheet theme ${worksheet.identity.numeric_theme_id}.`);
  return theme;
}

export function buildPdfPageModel(worksheet: WorksheetDto, metadata?: WorksheetMetadata): readonly PdfPageModel[] {
  const layout = buildSharedWorksheetLayout(worksheet);
  const theme = themeForWorksheet(worksheet);
  const cells = layout.cells.map(({ problem }, index) => ({
    number: `${index + 1}.`,
    problem_id: problem.problem_id,
    expression: problemExpression(problem),
    answer: answerNodeText(problem.canonical_answer) || undefined,
  }));
  const shared = {
    title: theme.worksheet.title,
    instruction: theme.worksheet.instruction,
    ...(metadata ? { footer: { text: formatWorksheetFooter(metadata), physical_corner: 'bottom-right' as const } } : {}),
  };
  return [
    {
      kind: 'problems',
      rotated: false,
      ...shared,
      cells: cells.map(({ number, problem_id, expression }) => ({ number, problem_id, expression })),
    },
    {
      kind: 'answers',
      rotated: true,
      ...shared,
      title: `${theme.worksheet.title} 解答`,
      cells,
    },
  ];
}

function toPagePercent(value: number, total: number): string {
  return `${(value / total) * 100}%`;
}

function PrintAnswer({
  problem,
  answerPrefix,
  answers,
}: {
  problem: WorksheetDto['problems'][number];
  answerPrefix: string | null;
  answers: boolean;
}) {
  if (!answers) {
    return (
      <span className="problem-answer-area worksheet-print-problem-answer" aria-hidden="true">
        {answerPrefix ? (
          <MathLiveStatic
            className="answer-prefix-label"
            latex={answerPrefix.replaceAll(' ', '\\,')}
            ariaLabel={answerPrefix}
          />
        ) : null}
        <span className="answer-box worksheet-print-empty-answer" />
      </span>
    );
  }

  const answerText = answerNodeText(problem.canonical_answer);
  return (
    <span className="problem-answer-area worksheet-print-answer-area">
      {answerPrefix ? (
        <MathLiveStatic
          className="answer-prefix-label"
          latex={answerPrefix.replaceAll(' ', '\\,')}
          ariaLabel={answerPrefix}
        />
      ) : null}
      <MathLiveStatic
        className="canonical-answer-math worksheet-print-answer-value"
        latex={answerNodeLatex(problem.canonical_answer)}
        ariaLabel={answerText}
      />
    </span>
  );
}

function WorksheetPrintPage({
  worksheet,
  metadata,
  answers,
}: {
  worksheet: WorksheetDto;
  metadata?: WorksheetMetadata;
  answers: boolean;
}) {
  const layout = buildSharedWorksheetLayout(worksheet);
  const theme = themeForWorksheet(worksheet);
  const gradeBandClass = worksheetGradeBandClass(theme.grade.slug);
  const contentTop = A4_PAGE.margin + A4_PAGE.headerHeight;
  const contentHeight = A4_PAGE.height - A4_PAGE.margin * 2 - A4_PAGE.headerHeight - A4_PAGE.footerHeight;
  const dividerStyle: CSSProperties = {
    left: toPagePercent(layout.dividerX, A4_PAGE.width),
    top: toPagePercent(contentTop, A4_PAGE.height),
    height: toPagePercent(contentHeight, A4_PAGE.height),
  };
  const footerStyle: CSSProperties = {
    right: toPagePercent(A4_PAGE.margin, A4_PAGE.width),
    bottom: toPagePercent(A4_PAGE.margin, A4_PAGE.height),
  };

  return (
    <article
      className={`worksheet-print-page ${gradeBandClass} ${answers ? 'worksheet-print-page-answers' : 'worksheet-print-page-problems'}`}
      data-print-page={answers ? 'answers' : 'problems'}
      aria-label={`${theme.worksheet.title}${answers ? ' 解答' : ''}`}
    >
      <div className={`worksheet-print-page-inner ${answers ? 'worksheet-print-page-inner-rotated' : ''}`}>
        <div className="worksheet-print-heading">
          <span>{theme.grade.label}</span>
          <strong>{theme.worksheet.title}{answers ? ' 解答' : ''}</strong>
        </div>
        <div className="problem-grid">
          {theme.worksheet.instruction ? (
            <p className="worksheet-instruction">{theme.worksheet.instruction}</p>
          ) : null}
          <div className="problem-divider" style={dividerStyle} />
          {layout.cells.map((cell) => {
            const { problem, index } = cell;
            const position = getCellTopPosition(layout, cell);
            const isLinearEquation = problem.prompt.kind === 'linear_equation';
            const cellStyle: CSSProperties = {
              left: toPagePercent(position.x, A4_PAGE.width),
              top: toPagePercent(position.y, A4_PAGE.height),
              width: toPagePercent(position.width, A4_PAGE.width),
              height: toPagePercent(position.height, A4_PAGE.height),
            };
            return (
              <div
                className={`problem-cell worksheet-print-problem-cell ${isLinearEquation ? 'problem-cell-linear-equation' : ''}`}
                data-print-problem-index={index}
                style={cellStyle}
                key={problem.problem_id}
              >
                <span className="problem-number">{index + 1}.</span>
                <span className="expression"><ProblemExpression problem={problem} /></span>
                <PrintAnswer
                  problem={problem}
                  answerPrefix={theme.worksheet.answerPrefix}
                  answers={answers}
                />
              </div>
            );
          })}
          {metadata ? (
            <div className="worksheet-footer" style={footerStyle}>{formatWorksheetFooter(metadata)}</div>
          ) : null}
        </div>
      </div>
    </article>
  );
}

/**
 * Printable worksheet DOM. All mathematical typesetting is delegated to the
 * same MathLive components used by the interactive worksheet; this module has
 * no PDF-specific fraction, radical, baseline, glyph, or spacing renderer.
 */
export function WorksheetPrintDocument({
  worksheet,
  metadata,
}: {
  worksheet: WorksheetDto;
  metadata?: WorksheetMetadata;
}) {
  return (
    <div className="worksheet-print-document">
      <WorksheetPrintPage worksheet={worksheet} metadata={metadata} answers={false} />
      <WorksheetPrintPage worksheet={worksheet} metadata={metadata} answers />
    </div>
  );
}

function WorksheetPrintPreview({
  worksheet,
  metadata,
  onClose,
  onPrint,
}: {
  worksheet: WorksheetDto;
  metadata?: WorksheetMetadata;
  onClose: () => void;
  onPrint: () => void;
}) {
  useEffect(() => {
    const previousOverflow = document.body.style.overflow;
    document.body.style.overflow = 'hidden';
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') onClose();
    };
    window.addEventListener('keydown', onKeyDown);
    return () => {
      window.removeEventListener('keydown', onKeyDown);
      document.body.style.overflow = previousOverflow;
    };
  }, [onClose]);

  return (
    <div
      className="worksheet-print-preview"
      role="dialog"
      aria-modal="true"
      aria-labelledby="worksheet-print-preview-title"
    >
      <header className="worksheet-print-preview-toolbar">
        <button type="button" className="worksheet-print-preview-back" onClick={onClose}>戻る</button>
        <div className="worksheet-print-preview-title-group">
          <h2 id="worksheet-print-preview-title">印刷プレビュー</h2>
          <p>問題・解答の2ページ</p>
        </div>
        <button type="button" className="worksheet-print-preview-print" onClick={onPrint} autoFocus>印刷する</button>
      </header>
      <div className="worksheet-print-preview-scroll">
        <div className="worksheet-print-preview-document">
          <WorksheetPrintDocument worksheet={worksheet} metadata={metadata} />
        </div>
      </div>
    </div>
  );
}

function nextAnimationFrame(): Promise<void> {
  return new Promise((resolve) => {
    if (typeof window.requestAnimationFrame === 'function') window.requestAnimationFrame(() => resolve());
    else window.setTimeout(resolve, 0);
  });
}

async function prepareMathLiveForPrint(host: HTMLElement): Promise<void> {
  if (typeof customElements !== 'undefined') await customElements.whenDefined('math-span');
  await nextAnimationFrame();
  for (const element of host.querySelectorAll<MathSpanWithRender>('math-span')) element.render?.();
  if (document.fonts?.ready) await document.fonts.ready;
  await nextAnimationFrame();
}

export type MountedWorksheetPrintDocument = {
  host: HTMLElement;
  root: Root;
  cleanup: () => void;
};

/** Browser/integration-test seam: mount the exact DOM that native printing consumes. */
export async function mountWorksheetPrintDocument(
  worksheet: WorksheetDto,
  metadata?: WorksheetMetadata,
): Promise<MountedWorksheetPrintDocument> {
  if (typeof document === 'undefined') throw new Error('Worksheet printing requires a browser document.');
  const host = document.createElement('div');
  host.className = 'worksheet-print-host worksheet-print-host-hidden';
  host.setAttribute('aria-hidden', 'true');
  document.body.append(host);
  const root = createRoot(host);
  root.render(<WorksheetPrintDocument worksheet={worksheet} metadata={metadata} />);
  await prepareMathLiveForPrint(host);
  let cleaned = false;
  return {
    host,
    root,
    cleanup: () => {
      if (cleaned) return;
      cleaned = true;
      root.unmount();
      host.remove();
    },
  };
}

let activePreviewCleanup: (() => void) | null = null;

/**
 * Open an in-app print preview first. The preview and the eventual native
 * print/PDF workflow use the exact same MathLive DOM/CSS; no parallel PDF
 * math renderer is introduced.
 *
 * `targetWindow` is retained only for call-site compatibility with alpha 1.1.
 */
export async function openWorksheetPdf(
  worksheet: WorksheetDto,
  targetWindow?: Window | null,
  metadata?: WorksheetMetadata,
): Promise<void> {
  if (typeof window === 'undefined') return;
  targetWindow?.close();
  activePreviewCleanup?.();

  const host = document.createElement('div');
  host.className = 'worksheet-print-host worksheet-print-host-preview';
  document.body.append(host);
  const root = createRoot(host);
  let cleaned = false;
  const cleanup = () => {
    if (cleaned) return;
    cleaned = true;
    root.unmount();
    host.remove();
    if (activePreviewCleanup === cleanup) activePreviewCleanup = null;
  };
  activePreviewCleanup = cleanup;

  root.render(
    <WorksheetPrintPreview
      worksheet={worksheet}
      metadata={metadata}
      onClose={cleanup}
      onPrint={() => window.print()}
    />,
  );
  await prepareMathLiveForPrint(host);
}
