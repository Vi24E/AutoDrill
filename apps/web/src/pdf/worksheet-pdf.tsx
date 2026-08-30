import { createRoot, type Root } from 'react-dom/client';
import { useState, type CSSProperties } from 'react';

import { MathLiveStatic } from '@/components/MathLiveMath';
import { ProblemExpression } from '@/components/ProblemExpression';
import { MiniSudokuGrid } from '@/components/MiniSudokuGrid';
import { WorksheetProblemCell } from '@/components/WorksheetProblemCell';
import { A4_PAGE, buildSharedWorksheetLayout, getCellTopPosition } from '@/domain/layout';
import { worksheetGradeBandClass } from '@/domain/grade-band';
import { answerNodeText, type AnswerNode, type WorksheetDto } from '@/domain/drill-engine';
import { answerNodeLatex, answerPrefixLatex } from '@/domain/mathlive-format';
import { liarPersonLabel, problemExpression } from '@/domain/problem-format';
import { answerCoordinate, answerPresentationPlan } from '@/domain/answer-presentation';
import { worksheetPageGridVariables } from '@/domain/worksheet-grid-presentation';
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

export function buildPdfPageModel(worksheet: WorksheetDto, metadata?: WorksheetMetadata, rotateAnswers = true): readonly PdfPageModel[] {
  const layout = buildSharedWorksheetLayout(worksheet);
  const theme = themeForWorksheet(worksheet);
  const cells = layout.cells.map(({ problem }, index) => ({
    number: `${index + 1}.`,
    problem_id: problem.problem_id,
    expression: problemExpression(problem, theme.worksheet.answerPlacement !== 'below'),
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
      rotated: rotateAnswers,
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
  const presentation = answerPresentationPlan(problem);
  if (presentation.kind === 'digit_grid') return null;
  if (presentation.kind === 'liar_puzzle') {
    const selected = new Set(problem.canonical_answer.type === 'tuple'
      ? problem.canonical_answer.value.flatMap((item) => item.type === 'integer' ? [Number(item.value)] : [])
      : []);
    return (
      <span className={`problem-answer-area problem-answer-area-liar ${answers ? 'worksheet-print-answer-area' : 'worksheet-print-problem-answer'}`}>
        <span className="liar-person-choice-row">
          {Array.from({ length: presentation.peopleCount }, (_, index) => index + 1).map((person) => (
            <span key={person} className={`liar-person-choice ${answers && selected.has(person) ? 'liar-person-choice-selected' : ''}`}>{liarPersonLabel(person)}</span>
          ))}
        </span>
      </span>
    );
  }

  if (presentation.kind === 'column_division') {
    const quotient = presentation.hasRemainder
      ? answerCoordinate(problem.canonical_answer, 0)
      : problem.canonical_answer;
    return (
      <span className={`problem-answer-area problem-answer-area-column-division ${answers ? 'worksheet-print-answer-area' : 'worksheet-print-problem-answer'}`} aria-hidden={answers ? undefined : 'true'}>
        <span className="column-division-answer-coordinate column-division-answer-coordinate-quotient">
          <span className="column-division-answer-label">商</span>
          {answers
            ? <MathLiveStatic className="canonical-answer-math worksheet-print-answer-value" latex={answerNodeLatex(quotient)} ariaLabel={answerNodeText(quotient)} />
            : <span className="answer-box worksheet-print-empty-answer" />}
        </span>
      </span>
    );
  }

  if (presentation.kind === 'simultaneous_equation') {
    const xAnswer = answerCoordinate(problem.canonical_answer, 0);
    const yAnswer = answerCoordinate(problem.canonical_answer, 1);
    return (
      <span className={`problem-answer-area problem-answer-area-simultaneous ${answers ? 'worksheet-print-answer-area' : 'worksheet-print-problem-answer'}`} aria-hidden={answers ? undefined : 'true'}>
        <span className="simultaneous-answer-coordinate">
          <MathLiveStatic className="answer-prefix-label" latex="x=" ariaLabel="x =" />
          {answers
            ? <MathLiveStatic className="canonical-answer-math worksheet-print-answer-value" latex={answerNodeLatex(xAnswer)} ariaLabel={answerNodeText(xAnswer)} />
            : <span className="answer-box worksheet-print-empty-answer" />}
        </span>
        <span className="simultaneous-answer-coordinate">
          <MathLiveStatic className="answer-prefix-label" latex="y=" ariaLabel="y =" />
          {answers
            ? <MathLiveStatic className="canonical-answer-math worksheet-print-answer-value" latex={answerNodeLatex(yAnswer)} ariaLabel={answerNodeText(yAnswer)} />
            : <span className="answer-box worksheet-print-empty-answer" />}
        </span>
      </span>
    );
  }

  if (!answers) {
    return (
      <span className="problem-answer-area worksheet-print-problem-answer" aria-hidden="true">
        {answerPrefix ? (
          <MathLiveStatic
            className="answer-prefix-label"
            latex={answerPrefixLatex(answerPrefix)}
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
          latex={answerPrefixLatex(answerPrefix)}
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
  rotateAnswers,
}: {
  worksheet: WorksheetDto;
  metadata?: WorksheetMetadata;
  answers: boolean;
  rotateAnswers: boolean;
}) {
  const layout = buildSharedWorksheetLayout(worksheet);
  const theme = themeForWorksheet(worksheet);
  const usesWorksheetGrid = theme.presentation.worksheet_grid;
  const isEquationWorksheet = theme.presentation.equation_layout;
  const gradeBandClass = theme.grade ? worksheetGradeBandClass(theme.grade.number) : 'worksheet-grade-junior-high';
  const categoryLabel = theme.grade?.label ?? 'おまけ';
  const contentTop = A4_PAGE.margin + A4_PAGE.headerHeight;
  const contentHeight = A4_PAGE.height - A4_PAGE.margin * 2 - A4_PAGE.headerHeight - A4_PAGE.footerHeight;
  const dividerStyles: readonly CSSProperties[] = (usesWorksheetGrid ? [] : layout.dividerXs).map((dividerX) => ({
    left: toPagePercent(dividerX, A4_PAGE.width),
    top: toPagePercent(contentTop, A4_PAGE.height),
    height: toPagePercent(contentHeight, A4_PAGE.height),
  }));
  const footerStyle: CSSProperties = {
    right: toPagePercent(A4_PAGE.margin, A4_PAGE.width),
    bottom: toPagePercent(A4_PAGE.margin, A4_PAGE.height),
  };

  return (
    <article
      className={`worksheet-print-page ${gradeBandClass} ${answers ? 'worksheet-print-page-answers' : 'worksheet-print-page-problems'}`}
      data-print-page={answers ? 'answers' : 'problems'}
      aria-label={`${theme.worksheet.title}${answers ? ' 解答' : ''}`}
      style={usesWorksheetGrid ? worksheetPageGridVariables() : undefined}
    >
      <div className={`worksheet-print-page-inner ${answers && rotateAnswers ? 'worksheet-print-page-inner-rotated' : ''}`}>
        <div className="worksheet-print-heading">
          <span>{categoryLabel}</span>
          <strong>{theme.worksheet.title}{answers ? ' 解答' : ''}</strong>
        </div>
        <div className={`problem-grid ${usesWorksheetGrid ? 'problem-grid-worksheet-grid' : ''}`}>
          {theme.worksheet.instruction ? (
            <p className="worksheet-instruction">{theme.worksheet.instruction}</p>
          ) : null}
          {dividerStyles.map((style, index) => (
            <div className="problem-divider" style={style} key={`divider-${index}`} />
          ))}
          {layout.cells.map((cell) => {
            const { problem, index } = cell;
            const position = getCellTopPosition(layout, cell);
            return (
              <WorksheetProblemCell
                problem={problem}
                index={index}
                position={position}
                answerPlacement={theme.worksheet.answerPlacement}
                equationLayout={isEquationWorksheet}
                mode="print"
                showSolution={Boolean(answers)}
                renderExpression={({ includeAnswerEquals, solution }) => (
                  <ProblemExpression
                    problem={problem}
                    includeAnswerEquals={includeAnswerEquals}
                    solution={solution}
                  />
                )}
                answer={(
                  <PrintAnswer
                    problem={problem}
                    answerPrefix={theme.worksheet.answerPrefix}
                    answers={answers}
                  />
                )}
                miniSudoku={(
                  <MiniSudokuGrid
                    problem={problem}
                    answer={answers ? problem.canonical_answer : undefined}
                    readOnly
                  />
                )}
                key={problem.problem_id}
              />
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
  rotateAnswers = true,
}: {
  worksheet: WorksheetDto;
  metadata?: WorksheetMetadata;
  rotateAnswers?: boolean;
}) {
  return (
    <div className="worksheet-print-document">
      <WorksheetPrintPage worksheet={worksheet} metadata={metadata} answers={false} rotateAnswers={false} />
      <WorksheetPrintPage worksheet={worksheet} metadata={metadata} answers rotateAnswers={rotateAnswers} />
    </div>
  );
}

function WorksheetPrintPreview({
  worksheet,
  metadata,
  host,
  onClose,
}: {
  worksheet: WorksheetDto;
  metadata?: WorksheetMetadata;
  host: HTMLElement;
  onClose: () => void;
}) {
  const [printing, setPrinting] = useState(false);
  const [printError, setPrintError] = useState<string | null>(null);
  const [rotateAnswers, setRotateAnswers] = useState(true);

  const printNow = async () => {
    if (printing) return;
    setPrinting(true);
    setPrintError(null);
    try {
      // Native print snapshots the DOM synchronously. Wait only after the user
      // requests printing, then require every MathLive element to have stable,
      // non-zero shadow content before opening the browser print dialog.
      await prepareMathLiveForPrint(host, 5_000);
      window.print();
    } catch (error) {
      console.error('Worksheet print preparation failed.', error);
      setPrintError('数式の描画が完了しませんでした。もう一度「印刷する」を押してください。');
    } finally {
      setPrinting(false);
    }
  };

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
          <p>{printError ?? '問題・解答の2ページ'}</p>
        </div>
        <div className="worksheet-print-preview-actions">
          <label className="worksheet-print-preview-rotate">
            <input
              type="checkbox"
              checked={rotateAnswers}
              onChange={(event) => setRotateAnswers(event.target.checked)}
            />
            <span>解答を逆さにする</span>
          </label>
          <button
            type="button"
            className="worksheet-print-preview-print"
            onClick={() => { void printNow(); }}
            disabled={printing}
            autoFocus
          >
            {printing ? '準備中…' : '印刷する'}
          </button>
        </div>
      </header>
      <div className="worksheet-print-preview-scroll">
        <div className="worksheet-print-preview-document">
          <WorksheetPrintDocument worksheet={worksheet} metadata={metadata} rotateAnswers={rotateAnswers} />
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

function mathSpanRenderSignature(element: MathSpanWithRender): string | null {
  const content = element.shadowRoot?.querySelector<HTMLElement>('[part~="render"]');
  if (!content) return null;
  const rect = content.getBoundingClientRect();
  if (rect.width <= 0 || rect.height <= 0) return null;
  return `${rect.width.toFixed(2)}x${rect.height.toFixed(2)}`;
}

export async function prepareMathLiveForPrint(host: HTMLElement, timeoutMs = 4_000): Promise<void> {
  if (typeof customElements !== 'undefined') await customElements.whenDefined('math-span');
  if (document.fonts?.ready) await document.fonts.ready;
  const deadline = performance.now() + timeoutMs;
  let previousSignature = '';
  let stableFrames = 0;

  while (performance.now() < deadline) {
    const elements = [...host.querySelectorAll<MathSpanWithRender>('math-span')];
    for (const element of elements) element.render?.();
    await nextAnimationFrame();

    if (elements.length === 0) return;
    // jsdom/test doubles intentionally have no layout or MathLive shadow DOM.
    // Production MathLive elements have measurable host rectangles once mounted.
    const hasLayout = elements.some((element) => {
      const rect = element.getBoundingClientRect();
      return rect.width > 0 || rect.height > 0;
    });
    if (!hasLayout && elements.every((element) => !element.shadowRoot)) return;

    const signatures = elements.map(mathSpanRenderSignature);
    if (signatures.every((signature): signature is string => signature !== null)) {
      const signature = signatures.join('|');
      stableFrames = signature === previousSignature ? stableFrames + 1 : 0;
      previousSignature = signature;
      // Two identical paint samples ensure fonts and all off-screen answer rows
      // have settled before the browser takes its native print snapshot.
      if (stableFrames >= 1) {
        await nextAnimationFrame();
        return;
      }
    } else {
      stableFrames = 0;
      previousSignature = '';
    }
  }

  const total = host.querySelectorAll('math-span').length;
  const ready = [...host.querySelectorAll<MathSpanWithRender>('math-span')]
    .filter((element) => mathSpanRenderSignature(element) !== null).length;
  throw new Error(`MathLive print rendering did not settle before printing (${ready}/${total} ready).`);
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
 */
export async function openWorksheetPdf(
  worksheet: WorksheetDto,
  metadata?: WorksheetMetadata,
): Promise<void> {
  if (typeof window === 'undefined') return;
  activePreviewCleanup?.();

  const host = document.createElement('div');
  host.className = 'worksheet-print-host worksheet-print-host-preview';
  document.body.append(host);
  const root = createRoot(host);
  const previousOverflow = document.body.style.overflow;
  document.body.style.overflow = 'hidden';
  let cleaned = false;
  const cleanup = () => {
    if (cleaned) return;
    cleaned = true;
    window.removeEventListener('keydown', onKeyDown);
    document.body.style.overflow = previousOverflow;
    root.unmount();
    host.remove();
    if (activePreviewCleanup === cleanup) activePreviewCleanup = null;
  };
  const onKeyDown = (event: KeyboardEvent) => {
    if (event.key === 'Escape') cleanup();
  };
  window.addEventListener('keydown', onKeyDown);
  activePreviewCleanup = cleanup;

  root.render(
    <WorksheetPrintPreview
      worksheet={worksheet}
      metadata={metadata}
      host={host}
      onClose={cleanup}
    />,
  );
}
