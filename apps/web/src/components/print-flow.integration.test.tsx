import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { AutoDrillApp } from '@/components/AutoDrillApp';
import { fixtureEngine } from '@/test/fixtures';

function renderApp() {
  return render(
    <AutoDrillApp
      engine={fixtureEngine()}
      seedGenerator={() => 'A1b2'}
      dateGenerator={() => new Date(2026, 7, 12)}
    />,
  );
}

async function expectPreviewOpen(printSpy: ReturnType<typeof vi.spyOn>) {
  const preview = await screen.findByRole('dialog', { name: '印刷プレビュー' });
  expect(printSpy).not.toHaveBeenCalled();
  expect(preview.querySelectorAll('[data-print-page]')).toHaveLength(2);
  expect(preview.querySelectorAll('math-span.problem-math-expression').length).toBeGreaterThan(0);
  return preview;
}

async function closePreview() {
  fireEvent.click(screen.getByRole('button', { name: '戻る' }));
  await waitFor(() => expect(screen.queryByRole('dialog', { name: '印刷プレビュー' })).not.toBeInTheDocument());
  expect(document.querySelector('.worksheet-print-host')).toBeNull();
}

describe('print flow integration paths', () => {
  let printSpy: ReturnType<typeof vi.spyOn>;

  beforeEach(() => {
    window.history.replaceState(null, '', '/');
    printSpy = vi.spyOn(window, 'print').mockImplementation(() => undefined);
  });

  afterEach(() => {
    document.querySelectorAll('.worksheet-print-host').forEach((element) => element.remove());
    document.body.style.overflow = '';
    printSpy.mockRestore();
    cleanup();
  });

  it('settings -> print creates a worksheet and opens the in-app preview without native printing', async () => {
    renderApp();
    fireEvent.click(screen.getByRole('button', { name: '印刷 (pdfで出力)' }));
    await expectPreviewOpen(printSpy);
    expect(screen.getByRole('heading', { name: 'まいんドリル' })).toBeInTheDocument();
    await closePreview();
    expect(screen.getByRole('heading', { name: 'まいんドリル' })).toBeInTheDocument();
  });

  it('lets the user choose whether the answer page is upside down', async () => {
    renderApp();
    fireEvent.click(screen.getByRole('button', { name: '印刷 (pdfで出力)' }));
    const preview = await expectPreviewOpen(printSpy);
    const rotate = screen.getByRole('checkbox', { name: '解答を逆さにする' });
    expect(rotate).toBeChecked();
    expect(preview.querySelector('[data-print-page="answers"] .worksheet-print-page-inner')).toHaveClass('worksheet-print-page-inner-rotated');

    fireEvent.click(rotate);
    expect(rotate).not.toBeChecked();
    expect(preview.querySelector('[data-print-page="answers"] .worksheet-print-page-inner')).not.toHaveClass('worksheet-print-page-inner-rotated');
    await closePreview();
  });

  it('settings preview -> print invokes native printing only from the preview action', async () => {
    renderApp();
    fireEvent.click(screen.getByRole('button', { name: '印刷 (pdfで出力)' }));
    await expectPreviewOpen(printSpy);
    const printButton = screen.getByRole('button', { name: '印刷する' });
    expect(printButton).toBeEnabled();
    fireEvent.click(printButton);
    await waitFor(() => expect(printSpy).toHaveBeenCalledTimes(1));
    expect(screen.getByRole('dialog', { name: '印刷プレビュー' })).toBeInTheDocument();
    await closePreview();
  });

  it('reopened settings preview closes immediately with Escape after a print-and-back cycle', async () => {
    renderApp();
    fireEvent.click(screen.getByRole('button', { name: '印刷 (pdfで出力)' }));
    await expectPreviewOpen(printSpy);
    fireEvent.click(screen.getByRole('button', { name: '印刷する' }));
    await waitFor(() => expect(printSpy).toHaveBeenCalledTimes(1));
    await closePreview();
    printSpy.mockClear();

    fireEvent.click(screen.getByRole('button', { name: '印刷 (pdfで出力)' }));
    await expectPreviewOpen(printSpy);
    fireEvent.keyDown(window, { key: 'Escape' });
    await waitFor(() => expect(screen.queryByRole('dialog', { name: '印刷プレビュー' })).not.toBeInTheDocument());
    expect(document.querySelector('.worksheet-print-host')).toBeNull();
  });

  it('worksheet editing -> print preview -> back preserves the worksheet state', async () => {
    renderApp();
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    await screen.findByRole('heading', { name: '1けたのたしざん(1)' });

    fireEvent.click(screen.getByRole('button', { name: '印刷' }));
    await expectPreviewOpen(printSpy);
    await closePreview();

    expect(screen.getByRole('heading', { name: '1けたのたしざん(1)' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '採点' })).toBeEnabled();
  });

  it('worksheet with an answer field selected -> print preview -> back preserves the open input state', async () => {
    renderApp();
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    await screen.findByRole('heading', { name: '1けたのたしざん(1)' });
    fireEvent.click(screen.getByRole('textbox', { name: '1番の答え 未入力' }));
    expect(screen.getByLabelText('数式入力パネル')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: '印刷' }));
    await expectPreviewOpen(printSpy);
    await closePreview();

    expect(screen.getByLabelText('数式入力パネル')).toBeInTheDocument();
    expect(screen.getByRole('textbox', { name: '1番の答え 未入力' })).toBeInTheDocument();
  });

  it('graded worksheet -> print preview -> back preserves the graded state', async () => {
    renderApp();
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    await screen.findByRole('heading', { name: '1けたのたしざん(1)' });
    fireEvent.click(screen.getByRole('button', { name: '採点' }));
    await waitFor(() => expect(screen.getByRole('status')).toHaveTextContent('0 / 20'));

    fireEvent.click(screen.getByRole('button', { name: '印刷' }));
    await expectPreviewOpen(printSpy);
    await closePreview();

    expect(screen.getByRole('status')).toHaveTextContent('0 / 20');
    expect(screen.getByRole('button', { name: '問題に戻る' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '採点' })).toBeDisabled();
  });
});
