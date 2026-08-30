import { createRoot } from 'react-dom/client';
import 'mathlive/fonts.css';
import { prepareMathLiveForPrint, WorksheetPrintDocument } from '@/pdf/worksheet-pdf';
import '@/app/globals.css';
import './renderer.css';

type RawWorksheet = {
  identity: { seed: string };
  problems: Array<{ id: number } & Record<string, unknown>>;
} & Record<string, unknown>;

type RenderPayload = { worksheet: RawWorksheet; problem_index: number };
type RenderIdentity = { attemptId?: string; prefetchId?: string };

function worksheetForWeb(worksheet: RawWorksheet) {
  return {
    ...worksheet,
    seed: worksheet.identity.seed,
    problems: worksheet.problems.map((problem) => ({ ...problem, problem_id: String(problem.id) })),
  };
}

function notify(type: 'qa-render-ready' | 'qa-render-error', identity: RenderIdentity, message?: string) {
  window.parent.postMessage({ type, ...identity, message }, window.location.origin);
}

async function waitForElement<T extends Element>(root: ParentNode, selector: string, timeoutMs = 5_000): Promise<T> {
  const deadline = performance.now() + timeoutMs;
  while (performance.now() < deadline) {
    const element = root.querySelector<T>(selector);
    if (element) return element;
    await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
  }
  throw new Error(`印刷用DOMを取得できませんでした: ${selector}`);
}

function fitSelectedCell(problemIndex: number, identity: RenderIdentity) {
  const page = document.querySelector<HTMLElement>('[data-print-page="answers"]');
  const cell = page?.querySelector<HTMLElement>(`[data-print-problem-index="${problemIndex}"]`);
  const viewport = document.querySelector<HTMLElement>('#root');
  if (!page || !cell || !viewport) throw new Error('印刷用の問題cellを取得できませんでした。');
  const pageRect = page.getBoundingClientRect();
  const cellRect = cell.getBoundingClientRect();
  const padding = 12;
  const cellX = cellRect.left - pageRect.left;
  const cellY = cellRect.top - pageRect.top;
  const scale = Math.min(
    (viewport.clientWidth - padding * 2) / cellRect.width,
    (viewport.clientHeight - padding * 2) / cellRect.height,
  );
  const left = (viewport.clientWidth - cellRect.width * scale) / 2 - cellX * scale;
  const top = (viewport.clientHeight - cellRect.height * scale) / 2 - cellY * scale;
  const clipRight = pageRect.width - cellX - cellRect.width;
  const clipBottom = pageRect.height - cellY - cellRect.height;
  page.style.transformOrigin = '0 0';
  page.style.transform = `translate(${left}px, ${top}px) scale(${scale})`;
  page.style.clipPath = `inset(${cellY}px ${clipRight}px ${clipBottom}px ${cellX}px)`;
  page.dataset.qaSelectedPage = 'true';
  document.body.dataset.ready = 'true';
  notify('qa-render-ready', identity);
}

async function main() {
  document.body.dataset.renderStage = 'loading-payload';
  const parameters = new URLSearchParams(window.location.search);
  const attemptId = parameters.get('attempt') ?? undefined;
  const prefetchId = parameters.get('prefetch') ?? undefined;
  if (Boolean(attemptId) === Boolean(prefetchId)) throw new Error('render idが正しくありません。');
  const identity = attemptId ? { attemptId } : { prefetchId };
  const endpoint = attemptId
    ? `/api/attempts/${encodeURIComponent(attemptId)}/render`
    : `/api/quick/prefetch/${encodeURIComponent(prefetchId!)}/render`;
  const response = await fetch(endpoint);
  const payload = await response.json() as RenderPayload & { error?: string };
  if (!response.ok) throw new Error(payload.error ?? `HTTP ${response.status}`);
  document.body.dataset.renderStage = 'mounting-worksheet';
  const host = document.querySelector<HTMLElement>('#root')!;
  const root = createRoot(host);
  root.render(<WorksheetPrintDocument worksheet={worksheetForWeb(payload.worksheet) as never} rotateAnswers={false} />);
  document.body.dataset.renderStage = 'waiting-mathlive-definition';
  await customElements.whenDefined('math-span');
  document.body.dataset.renderStage = 'waiting-react-paint';
  const answerPage = await waitForElement<HTMLElement>(host, '[data-print-page="answers"]');
  answerPage.dataset.qaSelectedPage = 'true';
  const selectedCell = await waitForElement<HTMLElement>(answerPage, `[data-print-problem-index="${payload.problem_index}"]`);
  document.body.dataset.renderStage = 'stabilizing-mathlive';
  await prepareMathLiveForPrint(selectedCell, 5_000);
  selectedCell.dataset.qaMathliveReady = 'true';
  document.body.dataset.mathliveReadyCount = String(selectedCell.querySelectorAll('math-span').length);
  document.body.dataset.renderStage = 'fitting-cell';
  fitSelectedCell(payload.problem_index, identity);
}

main().catch((error) => {
  const parameters = new URLSearchParams(window.location.search);
  const identity = parameters.get('attempt')
    ? { attemptId: parameters.get('attempt')! }
    : { prefetchId: parameters.get('prefetch') ?? '' };
  const message = error instanceof Error ? error.message : String(error);
  document.body.dataset.renderStage = 'error';
  document.body.dataset.renderError = message;
  document.querySelector('#root')!.innerHTML = `<p class="qa-render-status qa-render-error">${message}</p>`;
  notify('qa-render-error', identity, message);
});
