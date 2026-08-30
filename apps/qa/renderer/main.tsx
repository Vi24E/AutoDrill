import { createRoot } from 'react-dom/client';
import 'mathlive/fonts.css';
import { WorksheetPrintDocument } from '@/pdf/worksheet-pdf';
import '@/app/globals.css';
import './renderer.css';

type RawWorksheet = {
  identity: { seed: string };
  problems: Array<{ id: number } & Record<string, unknown>>;
} & Record<string, unknown>;

type RenderPayload = { worksheet: RawWorksheet; problem_index: number };

function worksheetForWeb(worksheet: RawWorksheet) {
  return {
    ...worksheet,
    seed: worksheet.identity.seed,
    problems: worksheet.problems.map((problem) => ({ ...problem, problem_id: String(problem.id) })),
  };
}

function notify(type: 'qa-render-ready' | 'qa-render-error', attemptId: string, message?: string) {
  window.parent.postMessage({ type, attemptId, message }, window.location.origin);
}

function fitSelectedCell(problemIndex: number, attemptId: string) {
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
  notify('qa-render-ready', attemptId);
}

async function main() {
  const attemptId = new URLSearchParams(window.location.search).get('attempt');
  if (!attemptId) throw new Error('attempt idがありません。');
  const response = await fetch(`/api/attempts/${encodeURIComponent(attemptId)}/render`);
  const payload = await response.json() as RenderPayload & { error?: string };
  if (!response.ok) throw new Error(payload.error ?? `HTTP ${response.status}`);
  const root = createRoot(document.querySelector('#root')!);
  root.render(<WorksheetPrintDocument worksheet={worksheetForWeb(payload.worksheet) as never} rotateAnswers={false} />);
  await customElements.whenDefined('math-span');
  await document.fonts?.ready;
  requestAnimationFrame(() => requestAnimationFrame(() => {
    try { fitSelectedCell(payload.problem_index, attemptId); }
    catch (error) { notify('qa-render-error', attemptId, error instanceof Error ? error.message : String(error)); }
  }));
}

main().catch((error) => {
  const attemptId = new URLSearchParams(window.location.search).get('attempt') ?? '';
  const message = error instanceof Error ? error.message : String(error);
  document.querySelector('#root')!.innerHTML = `<p class="qa-render-status qa-render-error">${message}</p>`;
  notify('qa-render-error', attemptId, message);
});
