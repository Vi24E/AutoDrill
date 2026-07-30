import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import { AutoDrillApp } from '@/components/AutoDrillApp';
import { A4_PAGE, buildSharedWorksheetLayout, getCellTopPosition } from '@/domain/layout';
import { buildPdfPageModel } from '@/pdf/worksheet-pdf';
import { fixtureEngine, fixtureSettings, fixtureWorksheet } from '@/test/fixtures';
import type { DrillEngine } from '@/domain/drill-engine';

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

describe('AutoDrillApp', () => {
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
