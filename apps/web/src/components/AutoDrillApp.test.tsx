import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import { AutoDrillApp } from '@/components/AutoDrillApp';
import { A4_PAGE, buildSharedWorksheetLayout, getCellTopPosition } from '@/domain/layout';
import { buildPdfPageModel } from '@/pdf/worksheet-pdf';
import { fixtureEngine, fixtureSettings, fixtureWorksheet } from '@/test/fixtures';
import type { DrillEngine, WorksheetDto } from '@/domain/drill-engine';

vi.mock('@/pdf/worksheet-pdf', async () => {
  const actual = await vi.importActual<typeof import('@/pdf/worksheet-pdf')>('@/pdf/worksheet-pdf');
  return { ...actual, openWorksheetPdf: vi.fn().mockResolvedValue(undefined) };
});

function delayedFixtureEngine() {
  const base = fixtureEngine();
  return {
    ...base,
    async applyEditorAction(...args: Parameters<NonNullable<typeof base.applyEditorAction>>) {
      await new Promise<void>((resolve) => window.setTimeout(resolve, 5));
      return base.applyEditorAction(...args);
    },
  };
}

function deferredGradingEngine() {
  const base = fixtureEngine();
  let resolveGrade!: (result: Awaited<ReturnType<DrillEngine['gradeAnswer']>>) => void;
  const pendingGrade = new Promise<Awaited<ReturnType<DrillEngine['gradeAnswer']>>>((resolve) => {
    resolveGrade = resolve;
  });
  const engine: DrillEngine = {
    ...base,
    gradeAnswer: vi.fn(() => pendingGrade),
  };
  return { engine, resolveGrade };
}

function seedRecordingEngine() {
  const seeds: string[] = [];
  const base = fixtureEngine();
  const engine: DrillEngine = {
    ...base,
    async generateWorksheet(settings) {
      seeds.push(settings.seed);
      return { ...fixtureWorksheet(), seed: settings.seed };
    },
  };
  return { engine, seeds };
}

function deferredGenerationEngine() {
  const base = fixtureEngine();
  let resolveGeneration!: (worksheet: WorksheetDto) => void;
  const pendingWorksheet = new Promise<WorksheetDto>((resolve) => {
    resolveGeneration = resolve;
  });
  const engine: DrillEngine = {
    ...base,
    generateWorksheet: vi.fn(() => pendingWorksheet),
  };
  return { engine, resolveGeneration };
}

describe('AutoDrillApp', () => {
  it('projects the curriculum hierarchy into three linked q1 selects only', async () => {
    render(<AutoDrillApp engine={fixtureEngine()} />);

    expect(screen.getByRole('combobox', { name: '学年' })).toHaveValue('jp-grade-1');
    expect(screen.getByRole('combobox', { name: '学年' })).toHaveDisplayValue('小学1年生');
    expect(screen.getByRole('combobox', { name: '領域' })).toHaveDisplayValue('数と計算');
    expect(screen.getByRole('combobox', { name: '単元' })).toHaveDisplayValue('1けたのたしざん(1)');
    expect(screen.getByRole('combobox', { name: '難易度' })).toHaveDisplayValue('標準（準備中）');

    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    await screen.findByRole('heading', { name: '1けたのたしざん(1)' });
    expect(screen.queryByRole('combobox', { name: '学年' })).not.toBeInTheDocument();
    expect(screen.queryByRole('combobox', { name: '領域' })).not.toBeInTheDocument();
    expect(screen.queryByRole('combobox', { name: '単元' })).not.toBeInTheDocument();
  });

  it('disables q1 actions and announces problem generation while it is pending', async () => {
    const { engine, resolveGeneration } = deferredGenerationEngine();
    render(<AutoDrillApp engine={engine} />);

    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    expect(screen.getByRole('button', { name: '問題を生成中…' })).toBeDisabled();
    expect(screen.getByRole('button', { name: '印刷' })).toBeDisabled();
    expect(screen.getByText('問題を生成しています。しばらくお待ちください。')).toBeInTheDocument();

    resolveGeneration(fixtureWorksheet());
    expect(await screen.findByRole('heading', { name: '1けたのたしざん(1)' })).toBeInTheDocument();
  });

  it('disables q1 actions and announces PDF preparation while it is pending', async () => {
    const { engine, resolveGeneration } = deferredGenerationEngine();
    vi.spyOn(window, 'open').mockReturnValue({ close: vi.fn() } as unknown as Window);
    render(<AutoDrillApp engine={engine} />);

    fireEvent.click(screen.getByRole('button', { name: '印刷' }));
    expect(screen.getByRole('button', { name: 'PDFを準備中…' })).toBeDisabled();
    expect(screen.getByRole('button', { name: '問題生成' })).toBeDisabled();
    expect(screen.getByText('印刷用PDFを準備しています。しばらくお待ちください。')).toBeInTheDocument();

    resolveGeneration(fixtureWorksheet());
    await waitFor(() => expect(screen.getByRole('button', { name: '印刷' })).toBeEnabled());
  });

  it('transitions from q1 generation to the q2 worksheet', async () => {
    render(<AutoDrillApp engine={fixtureEngine()} />);
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    expect(await screen.findByRole('heading', { name: '1けたのたしざん(1)' })).toBeInTheDocument();
    expect(screen.getByLabelText('20問の一桁足し算ワークシート')).toBeInTheDocument();
    expect(screen.queryByLabelText('数字入力パネル')).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: /^1番の答え/ }));
    expect(screen.getByLabelText('数字入力パネル')).toBeInTheDocument();
  });

  it('derives q2 cell positions and order from the shared A4 layout', async () => {
    render(<AutoDrillApp engine={fixtureEngine()} />);
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    await screen.findByRole('heading', { name: '1けたのたしざん(1)' });

    const layout = buildSharedWorksheetLayout(fixtureWorksheet());
    const first = getCellTopPosition(layout, layout.cells[0]!);
    const last = getCellTopPosition(layout, layout.cells[19]!);
    const firstCell = screen.getByTestId('problem-cell-0');
    const lastCell = screen.getByTestId('problem-cell-19');
    const percent = (value: number, total: number) => `${(value / total) * 100}%`;

    expect(firstCell.style.top).toBe(percent(first.y, A4_PAGE.height));
    expect(lastCell.style.top).toBe(percent(last.y, A4_PAGE.height));
    expect(Number.parseFloat(firstCell.style.top)).toBeLessThan(Number.parseFloat(lastCell.style.top));
    expect(firstCell.dataset.layoutIndex).toBe('0');
    expect(lastCell.dataset.layoutIndex).toBe('19');
    expect(screen.getByTestId('problem-divider').style.left).toBe(percent(layout.dividerX, A4_PAGE.width));
  });

  it('resets the selected editor on TOP and on each regenerated worksheet', async () => {
    render(<AutoDrillApp engine={fixtureEngine()} />);
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    await screen.findByRole('heading', { name: '1けたのたしざん(1)' });
    fireEvent.click(screen.getByRole('button', { name: /^1番の答え/ }));
    expect(screen.getByLabelText('数字入力パネル')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'TOPに戻る' }));
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    await screen.findByRole('heading', { name: '1けたのたしざん(1)' });
    expect(screen.queryByLabelText('数字入力パネル')).not.toBeInTheDocument();
  });

  it('supports keypad input, physical Enter, and next-problem focus', async () => {
    render(<AutoDrillApp engine={fixtureEngine()} />);
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    await screen.findByRole('heading', { name: '1けたのたしざん(1)' });
    const first = screen.getByRole('button', { name: /^1番の答え/ });
    fireEvent.click(first);
    fireEvent.click(screen.getByRole('button', { name: '1' }));
    await waitFor(() => expect(screen.getByRole('button', { name: /1番の答え 1/ })).toBeInTheDocument());
    fireEvent.keyDown(window, { key: 'Enter' });
    await waitFor(() => expect(screen.getByRole('button', { name: /^2番の答え/ })).toHaveClass('answer-box-selected'));
    expect(screen.getByLabelText('数字入力パネル')).toBeInTheDocument();
  });

  it('renders the fixed keypad in standard calculator order with clear controls', async () => {
    render(<AutoDrillApp engine={fixtureEngine()} />);
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    await screen.findByRole('heading', { name: '1けたのたしざん(1)' });
    fireEvent.click(screen.getByRole('button', { name: /^1番の答え/ }));

    const numberPad = screen.getByLabelText('数字キー');
    expect(within(numberPad).getAllByRole('button').map((button) => button.textContent)).toEqual([
      '7', '8', '9', '4', '5', '6', '1', '2', '3', '0',
    ]);
    expect(within(screen.getByLabelText('編集キー')).getByRole('button', { name: '一文字戻す' })).toBeInTheDocument();
    expect(within(screen.getByLabelText('編集キー')).getByRole('button', { name: '一文字削除' })).toBeInTheDocument();
    expect(screen.queryByText(/^AST:/)).not.toBeInTheDocument();
  });

  it('supports physical Backspace, Delete, and cursor movement without native button collisions', async () => {
    render(<AutoDrillApp engine={fixtureEngine()} />);
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    await screen.findByRole('heading', { name: '1けたのたしざん(1)' });
    fireEvent.click(screen.getByRole('button', { name: /^1番の答え/ }));

    fireEvent.keyDown(window, { key: '1' });
    fireEvent.keyDown(window, { key: '2' });
    await waitFor(() => expect(screen.getByRole('button', { name: '1番の答え 12' })).toBeInTheDocument());
    fireEvent.keyDown(window, { key: 'ArrowLeft' });
    fireEvent.keyDown(window, { key: 'Delete' });
    await waitFor(() => expect(screen.getByRole('button', { name: '1番の答え 1' })).toBeInTheDocument());
    fireEvent.keyDown(window, { key: '2' });
    fireEvent.keyDown(window, { key: 'ArrowLeft' });
    fireEvent.keyDown(window, { key: 'Backspace' });
    await waitFor(() => expect(screen.getByRole('button', { name: '1番の答え 2' })).toBeInTheDocument());
    fireEvent.keyDown(window, { key: '1' });
    fireEvent.keyDown(window, { key: 'ArrowRight' });
    await waitFor(() => expect(screen.getByRole('button', { name: '1番の答え 12' })).toBeInTheDocument());
  });

  it('serializes rapid digits and Enter against a delayed engine', async () => {
    render(<AutoDrillApp engine={delayedFixtureEngine()} />);
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    await screen.findByRole('heading', { name: '1けたのたしざん(1)' });
    fireEvent.click(screen.getByRole('button', { name: /^1番の答え/ }));

    fireEvent.keyDown(window, { key: '1' });
    fireEvent.keyDown(window, { key: '2' });
    fireEvent.keyDown(window, { key: 'Enter' });

    await waitFor(() => expect(screen.getByRole('button', { name: '1番の答え 12' })).toBeInTheDocument(), {
      timeout: 1000,
    });
    await waitFor(() => expect(screen.getByRole('button', { name: /^2番の答え/ })).toHaveClass('answer-box-selected'), {
      timeout: 1000,
    });
  });

  it('routes a rapid digit after Enter to the newly selected problem', async () => {
    render(<AutoDrillApp engine={delayedFixtureEngine()} />);
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    await screen.findByRole('heading', { name: '1けたのたしざん(1)' });
    fireEvent.click(screen.getByRole('button', { name: /^1番の答え/ }));

    fireEvent.keyDown(window, { key: '1' });
    fireEvent.keyDown(window, { key: 'Enter' });
    fireEvent.keyDown(window, { key: '2' });

    await waitFor(() => expect(screen.getByRole('button', { name: '1番の答え 1' })).toBeInTheDocument(), { timeout: 1000 });
    await waitFor(() => expect(screen.getByRole('button', { name: '2番の答え 2' })).toHaveClass('answer-box-selected'), { timeout: 1000 });
  });

  it('keeps an 18-digit answer inside its box and shows a stable size-limit notice on the next digit', async () => {
    render(<AutoDrillApp engine={fixtureEngine()} />);
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    await screen.findByRole('heading', { name: '1けたのたしざん(1)' });
    const emptyAnswer = screen.getByRole('button', { name: '1番の答え 未入力' });
    expect(emptyAnswer).toHaveStyle({ width: '42px', flexGrow: '0', flexShrink: '1' });
    fireEvent.click(emptyAnswer);

    fireEvent.keyDown(window, { key: '1' });
    fireEvent.keyDown(window, { key: '1' });
    const twoDigits = await screen.findByRole('button', { name: '1番の答え 11' });
    expect(twoDigits).toHaveStyle({ width: '42px', flexGrow: '0', flexShrink: '1' });

    for (let index = 2; index < 19; index += 1) fireEvent.keyDown(window, { key: '1' });
    const eighteenDigits = '1'.repeat(18);
    const answer = await screen.findByRole('button', { name: `1番の答え ${eighteenDigits}` });
    expect(answer).toHaveAttribute('data-answer-length', '18');
    expect(answer).toHaveStyle({ width: '138px', fontSize: '6px', flexGrow: '0', flexShrink: '1' });
    expect(await screen.findByText('式が大きすぎます！')).toBeInTheDocument();

    fireEvent.keyDown(window, { key: 'Backspace' });
    await screen.findByRole('button', { name: `1番の答え ${'1'.repeat(17)}` });
    await waitFor(() => expect(screen.queryByText('式が大きすぎます！')).not.toBeInTheDocument());
  });

  it('generates distinct automatic seeds for blank q1 generation and print', async () => {
    const { engine, seeds } = seedRecordingEngine();
    const generatedSeeds = ['A1b2', 'C3d4'];
    let seedIndex = 0;
    const pdfModule = await import('@/pdf/worksheet-pdf');
    const openSpy = vi.mocked(pdfModule.openWorksheetPdf);
    vi.spyOn(window, 'open').mockReturnValue({ close: vi.fn() } as unknown as Window);
    render(
      <AutoDrillApp
        engine={engine}
        seedGenerator={() => generatedSeeds[seedIndex++]!}
        dateGenerator={() => new Date(2026, 6, 30)}
      />,
    );

    expect(screen.getByLabelText('Seed')).toHaveValue('');
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    await screen.findByRole('heading', { name: '1けたのたしざん(1)' });
    fireEvent.click(screen.getByRole('button', { name: 'TOPに戻る' }));
    expect(screen.getByLabelText('Seed')).toHaveValue('');
    fireEvent.click(screen.getByRole('button', { name: '印刷' }));
    await waitFor(() => expect(openSpy).toHaveBeenCalledTimes(1));

    expect(seeds).toEqual(generatedSeeds);
    expect(new Set(seeds).size).toBe(2);
    expect(openSpy.mock.calls[0]?.[2]).toEqual({ generated_date: '2026-07-30', seed: 'C3d4' });
  });

  it('reuses an explicit q1 seed exactly and repeats the worksheet', async () => {
    const { engine, seeds } = seedRecordingEngine();
    render(
      <AutoDrillApp
        engine={engine}
        initialSettings={{ ...fixtureSettings(), seed: '' }}
        seedGenerator={() => 'must-not-be-used'}
        dateGenerator={() => new Date(2026, 6, 30)}
      />,
    );

    fireEvent.change(screen.getByLabelText('Seed'), { target: { value: 'repeatMe' } });
    expect(screen.getByLabelText('Seed')).toHaveValue('repeatMe');
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    await screen.findByRole('heading', { name: '1けたのたしざん(1)' });
    expect(screen.queryByLabelText('Seed')).not.toBeInTheDocument();
    const pdfPages = buildPdfPageModel(fixtureWorksheet(), { generated_date: '2026-07-30', seed: 'repeatMe' });
    const visibleFooter = screen.getByTestId('worksheet-footer').textContent;
    expect(visibleFooter).toBe(pdfPages[0]?.footer?.text);
    expect(pdfPages[1]?.footer?.text).toBe(pdfPages[0]?.footer?.text);
    fireEvent.click(screen.getByRole('button', { name: 'TOPに戻る' }));
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    await screen.findByRole('heading', { name: '1けたのたしざん(1)' });

    expect(seeds).toEqual(['repeatMe', 'repeatMe']);
  });

  it('passes valid one- and sixteen-character manual seeds unchanged', async () => {
    const { engine, seeds } = seedRecordingEngine();
    render(<AutoDrillApp engine={engine} initialSettings={{ ...fixtureSettings(), seed: '' }} />);

    const seedInput = screen.getByLabelText('Seed');
    fireEvent.change(seedInput, { target: { value: '1' } });
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    await screen.findByRole('heading', { name: '1けたのたしざん(1)' });
    fireEvent.click(screen.getByRole('button', { name: 'TOPに戻る' }));

    fireEvent.change(screen.getByLabelText('Seed'), { target: { value: '123456789abcdeFG' } });
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    await screen.findByRole('heading', { name: '1けたのたしざん(1)' });

    expect(seeds).toEqual(['1', '123456789abcdeFG']);
  });

  it('grades the latest answer when grading immediately after queued input', async () => {
    render(<AutoDrillApp engine={delayedFixtureEngine()} />);
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    await screen.findByRole('heading', { name: '1けたのたしざん(1)' });
    fireEvent.click(screen.getByRole('button', { name: /^1番の答え/ }));

    fireEvent.keyDown(window, { key: '2' });
    fireEvent.click(screen.getByRole('button', { name: '採点' }));

    await waitFor(() => expect(screen.getByRole('status')).toHaveTextContent('1 / 20'), {
      timeout: 1000,
    });
  });

  it('freezes elapsed time when grading starts, including while grading is pending', async () => {
    vi.useFakeTimers();
    vi.setSystemTime(1_000);
    const { engine, resolveGrade } = deferredGradingEngine();
    try {
      render(<AutoDrillApp engine={engine} />);
      fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
      await act(async () => {
        await Promise.resolve();
        await Promise.resolve();
      });
      act(() => vi.advanceTimersByTime(3_000));
      expect(screen.getByTestId('elapsed-time')).toHaveTextContent('00:03');

      fireEvent.click(screen.getByRole('button', { name: '採点' }));
      act(() => vi.advanceTimersByTime(5_000));
      expect(screen.getByTestId('elapsed-time')).toHaveTextContent('00:03');

      resolveGrade({ schema_version: 1, items: [], correct_count: 0, total_count: 20 });
      await act(async () => {
        await Promise.resolve();
        await Promise.resolve();
      });
      expect(screen.getByTestId('elapsed-time')).toHaveTextContent('00:03');
    } finally {
      vi.useRealTimers();
    }
  });

  it('marks wrong and unanswered boxes in red and shows each correct answer beside it', async () => {
    render(<AutoDrillApp engine={fixtureEngine()} />);
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    await screen.findByRole('heading', { name: '1けたのたしざん(1)' });
    fireEvent.click(screen.getByRole('button', { name: /^1番の答え/ }));
    fireEvent.keyDown(window, { key: '9' });
    fireEvent.click(screen.getByRole('button', { name: '採点' }));
    await waitFor(() => expect(screen.getByRole('status')).toHaveTextContent('0 / 20'));

    expect(screen.getByRole('button', { name: '1番の答え 9' })).toHaveClass('answer-box-wrong');
    expect(screen.getByRole('button', { name: '2番の答え 未入力' })).toHaveClass('answer-box-wrong');
    expect(within(screen.getByTestId('problem-cell-0')).getByLabelText('正しい答え 2')).toHaveTextContent('2');
    expect(within(screen.getByTestId('problem-cell-1')).getByLabelText('正しい答え 3')).toHaveTextContent('3');
  });

  it('sends the same worksheet object to q2 print', async () => {
    const pdfModule = await import('@/pdf/worksheet-pdf');
    const openSpy = vi.mocked(pdfModule.openWorksheetPdf);
    const { engine, seeds } = seedRecordingEngine();
    render(<AutoDrillApp engine={engine} initialSettings={fixtureSettings()} dateGenerator={() => new Date(2026, 6, 30)} />);
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    await screen.findByRole('heading', { name: '1けたのたしざん(1)' });
    fireEvent.click(screen.getByRole('button', { name: '印刷' }));
    await waitFor(() => expect(openSpy).toHaveBeenCalledTimes(1));
    expect(openSpy.mock.calls[0]?.[0].seed).toBe('fixtureSeed');
    expect(openSpy.mock.calls[0]?.[2]).toEqual({ generated_date: '2026-07-30', seed: 'fixtureSeed' });
    expect(seeds).toEqual(['fixtureSeed']);
  });

  it('uses the same PDF pipeline for q1 print after generation', async () => {
    const pdfModule = await import('@/pdf/worksheet-pdf');
    const openSpy = vi.mocked(pdfModule.openWorksheetPdf);
    vi.spyOn(window, 'open').mockReturnValue({ close: vi.fn() } as unknown as Window);
    render(<AutoDrillApp engine={fixtureEngine()} />);
    fireEvent.click(screen.getByRole('button', { name: '印刷' }));
    await waitFor(() => expect(openSpy).toHaveBeenCalledTimes(1));
    expect(openSpy.mock.calls[0]?.[0].layout).toMatchObject({ problem_count: 20, columns: 2, rows: 10 });
    expect(screen.getByRole('heading', { name: '計算ドリルをつくる' })).toBeInTheDocument();
  });

  it('does not retain a timer for q1 print and clears q2 timer on TOP', async () => {
    vi.useFakeTimers();
    vi.setSystemTime(1000);
    vi.spyOn(window, 'open').mockReturnValue({ close: vi.fn() } as unknown as Window);
    try {
      render(<AutoDrillApp engine={fixtureEngine()} />);
      fireEvent.click(screen.getByRole('button', { name: '印刷' }));
      await act(async () => {
        await Promise.resolve();
        await Promise.resolve();
      });
      expect(vi.getTimerCount()).toBe(0);

      fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
      await act(async () => {
        await Promise.resolve();
        await Promise.resolve();
      });
      expect(screen.getByRole('heading', { name: '1けたのたしざん(1)' })).toBeInTheDocument();
      expect(vi.getTimerCount()).toBe(1);
      fireEvent.click(screen.getByRole('button', { name: 'TOPに戻る' }));
      expect(screen.getByRole('heading', { name: '計算ドリルをつくる' })).toBeInTheDocument();
      expect(vi.getTimerCount()).toBe(0);
    } finally {
      vi.useRealTimers();
    }
  });
});
