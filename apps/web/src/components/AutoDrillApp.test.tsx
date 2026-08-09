import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { AutoDrillApp } from '@/components/AutoDrillApp';
import { createWebDrillSettings, LINEAR_EQUATION_1_THEME, ONE_DIGIT_ADDITION_THEME } from '@/domain/curriculum';
import { A4_PAGE, buildSharedWorksheetLayout, getCellTopPosition } from '@/domain/layout';
import { buildPdfPageModel } from '@/pdf/worksheet-pdf';
import { fixtureEngine, fixtureSettings, fixtureWorksheet, linearFixtureWorksheet } from '@/test/fixtures';
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

function warningFixtureEngine(): DrillEngine {
  const base = fixtureEngine();
  return {
    ...base,
    async gradeAnswer(request) {
      const result = await base.gradeAnswer(request);
      return {
        ...result,
        items: result.items.map((item, index) => index === 0 ? {
          ...item,
          correct: true,
          warnings: ['fraction_not_reduced', 'redundant_negative', 'redundant_decimal', 'integer_form_required'] as const,
        } : item),
        correct_count: result.correct_count + 1,
      };
    },
  };
}

function simpleNumericFixtureEngine(allowNegative: boolean): DrillEngine {
  const worksheet = fixtureWorksheet();
  worksheet.problems = worksheet.problems.map((problem) => ({
    ...problem,
    input_interface: { type: 'simple_numeric', allow_decimal: true, allow_negative: allowNegative },
  }));
  return fixtureEngine(worksheet);
}

function structuredFixtureEngine(): DrillEngine {
  const worksheet = fixtureWorksheet();
  worksheet.problems = worksheet.problems.map((problem) => ({
    ...problem,
    input_interface: {
      type: 'structured_math',
      allowed_structures: ['fraction', 'mixed_fraction', 'decimal', 'root', 'negative', 'plus_minus', 'tuple'] as const,
    },
  }));
  const base = fixtureEngine(worksheet);
  return {
    ...base,
    async applyEditorAction(state, action, inputInterface) {
      if (action.kind === 'insert_structure' && action.structure === 'fraction') {
        return {
          answer: {
            type: 'fraction',
            value: { numerator: { type: 'empty' }, denominator: { type: 'empty' } },
          },
          active_path: [0],
          cursor: 0,
          committed: false,
        };
      }
      if (action.kind === 'insert_structure' && action.structure === 'root') {
        return {
          answer: {
            type: 'root',
            value: { radicand: { type: 'empty' }, index: null },
          },
          active_path: [0],
          cursor: 0,
          committed: false,
        };
      }
      if (action.kind === 'select_slot') {
        return { ...state, active_path: [...action.path], cursor: action.cursor };
      }
      if (action.kind === 'insert_digit' && state.answer.type === 'fraction') {
        const numerator = state.active_path[0] === 0
          ? { type: 'integer' as const, value: String(action.digit) }
          : state.answer.value.numerator;
        const denominator = state.active_path[0] === 1
          ? { type: 'integer' as const, value: String(action.digit) }
          : state.answer.value.denominator;
        return {
          ...state,
          answer: { type: 'fraction', value: { numerator, denominator } },
          cursor: 1,
        };
      }
      return base.applyEditorAction(state, action, inputInterface);
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

function expectVisibleKanjiToUseRuby(container: HTMLElement) {
  const offenders = Array.from(container.querySelectorAll<HTMLElement>('*')).flatMap((element) =>
    Array.from(element.childNodes)
      .filter((node): node is Text => node.nodeType === Node.TEXT_NODE)
      .filter((node) => /\p{Script=Han}/u.test(node.textContent ?? ''))
      .filter((node) => !element.closest('ruby'))
      .map((node) => node.textContent?.trim()),
  );
  expect(offenders).toEqual([]);
}

describe('AutoDrillApp', () => {
  beforeEach(() => {
    window.localStorage.clear();
  });

  it('enables furigana by default on q1', () => {
    const { container } = render(<AutoDrillApp engine={fixtureEngine()} />);
    expect(screen.getByRole('checkbox', { name: 'ふりがな' })).toBeChecked();
    expect(container.querySelector('ruby')).toBeInTheDocument();
    expect(container.querySelector('rt')).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: '計算ドリルをつくる' })).toBeInTheDocument();
    expect(screen.queryByLabelText('今日のステージを選んで、20問のドリルを始めよう。')).not.toBeInTheDocument();
  });

  it('turns furigana off across q1 and q2 and retains that setting after TOP', async () => {
    const { container } = render(<AutoDrillApp engine={fixtureEngine()} />);
    fireEvent.click(screen.getByRole('checkbox', { name: 'ふりがな' }));
    expect(screen.getByRole('checkbox', { name: 'ふりがな' })).not.toBeChecked();
    expect(container.querySelector('ruby')).not.toBeInTheDocument();
    expect(container.querySelector('rt')).not.toBeInTheDocument();
    expect(screen.getByRole('heading', { name: '計算ドリルをつくる' })).toHaveTextContent('計算ドリルをつくる');

    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    await screen.findByRole('heading', { name: '1けたのたしざん(1)' });
    expect(container.querySelector('ruby')).not.toBeInTheDocument();
    expect(container.querySelector('rt')).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: '採点' })).toHaveTextContent('採点');

    fireEvent.click(screen.getByRole('button', { name: 'TOPに戻る' }));
    expect(screen.getByRole('checkbox', { name: 'ふりがな' })).not.toBeChecked();
    expect(container.querySelector('ruby')).not.toBeInTheDocument();
  });

  it('restores the saved furigana preference after remount', async () => {
    const first = render(<AutoDrillApp engine={fixtureEngine()} />);
    fireEvent.click(screen.getByRole('checkbox', { name: 'ふりがな' }));
    expect(window.localStorage.getItem('autodrill:furigana-enabled')).toBe('false');
    first.unmount();

    const second = render(<AutoDrillApp engine={fixtureEngine()} />);
    await waitFor(() => expect(screen.getByRole('checkbox', { name: 'ふりがな' })).not.toBeChecked());
    expect(second.container.querySelector('ruby')).not.toBeInTheDocument();
    expect(screen.getByRole('heading', { name: '計算ドリルをつくる' })).toHaveTextContent('計算ドリルをつくる');
  });

  it('keeps addition and equations in Recommended and uses ruby inside custom dropdown options', async () => {
    const { container } = render(<AutoDrillApp engine={fixtureEngine()} />);

    expect(screen.getByRole('button', { name: 'おすすめ' })).toHaveAttribute('aria-pressed', 'true');
    expect(screen.queryByRole('combobox', { name: '学年' })).not.toBeInTheDocument();
    expect(screen.getByRole('combobox', { name: 'ジャンル' })).toHaveAttribute('data-selected-label', '足し算と引き算');
    expect(screen.getByRole('combobox', { name: 'テーマ' })).toHaveAttribute('data-selected-label', '一桁の足し算');
    expect(screen.getByRole('combobox', { name: '難易度' })).toHaveAttribute('data-selected-label', '3: ふつう');

    fireEvent.click(screen.getByRole('combobox', { name: 'ジャンル' }));
    const recommendedOptions = within(screen.getByRole('listbox', { name: 'ジャンルの選択肢' })).getAllByRole('option');
    expect(recommendedOptions.map((option) => option.getAttribute('aria-label'))).toEqual(['足し算と引き算', '方程式']);
    expect(screen.getByRole('option', { name: '足し算と引き算' }).querySelectorAll('rt').length).toBeGreaterThan(0);
    fireEvent.click(screen.getByRole('option', { name: '方程式' }));
    expect(screen.getByRole('combobox', { name: 'ジャンル' })).toHaveAttribute('data-selected-label', '方程式');
    expect(screen.getByRole('combobox', { name: 'テーマ' })).toHaveAttribute('data-selected-label', '一次方程式(1)');

    fireEvent.click(screen.getByRole('button', { name: '学年から選ぶ' }));
    expect(screen.getByRole('combobox', { name: '学年' })).toHaveAttribute('data-value', 'grade-7');
    expect(screen.getByRole('combobox', { name: '学年' })).toHaveAttribute('data-selected-label', '中学1年生');
    fireEvent.click(screen.getByRole('combobox', { name: '学年' }));
    expect(within(screen.getByRole('listbox', { name: '学年の選択肢' })).getAllByRole('option')).toHaveLength(9);
    expect(screen.getByRole('option', { name: '中学1年生' }).querySelector('rt')).toBeInTheDocument();
    fireEvent.keyDown(screen.getByRole('combobox', { name: '学年' }), { key: 'Escape' });
    expect(screen.queryByRole('listbox', { name: '学年の選択肢' })).not.toBeInTheDocument();

    expectVisibleKanjiToUseRuby(container);
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    await screen.findByRole('heading', { name: '1けたのたしざん(1)' });
  });

  it('removes ruby from custom dropdown options when furigana is off', () => {
    render(<AutoDrillApp engine={fixtureEngine()} />);
    fireEvent.click(screen.getByRole('checkbox', { name: 'ふりがな' }));
    fireEvent.click(screen.getByRole('combobox', { name: 'ジャンル' }));
    expect(screen.getByRole('option', { name: '足し算と引き算' }).querySelector('rt')).not.toBeInTheDocument();
    expect(screen.getByRole('option', { name: '方程式' }).querySelector('rt')).not.toBeInTheDocument();
  });

  it('selects Dummy structure by grade and disables unavailable actions', () => {
    render(<AutoDrillApp engine={fixtureEngine()} />);
    fireEvent.click(screen.getByRole('button', { name: '学年から選ぶ' }));
    fireEvent.click(screen.getByRole('combobox', { name: '学年' }));
    fireEvent.click(screen.getByRole('option', { name: '小学2年生' }));

    expect(screen.getByRole('combobox', { name: '学年' })).toHaveAttribute('data-selected-label', '小学2年生');
    expect(screen.getByRole('combobox', { name: 'ジャンル' })).toHaveAttribute('data-selected-label', 'Dummy1');
    expect(screen.getByRole('combobox', { name: 'テーマ' })).toHaveAttribute('data-selected-label', 'Dummy1');
    expect(screen.getByRole('button', { name: '問題生成' })).toBeDisabled();
    expect(screen.getByRole('button', { name: '印刷' })).toBeDisabled();
    expect(screen.getByLabelText('このテーマはまだ利用できません')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'おすすめ' }));
    expect(screen.queryByRole('combobox', { name: '学年' })).not.toBeInTheDocument();
    expect(screen.getByRole('combobox', { name: 'ジャンル' })).toHaveAttribute('data-selected-label', '足し算と引き算');
    expect(screen.getByRole('combobox', { name: 'テーマ' })).toHaveAttribute('data-selected-label', '一桁の足し算');
    expect(screen.getByRole('button', { name: '問題生成' })).toBeEnabled();
  });

  it('stores difficulty changes through the custom difficulty dropdown', async () => {
    const onWebSettingsChange = vi.fn();
    render(<AutoDrillApp engine={fixtureEngine()} onWebSettingsChange={onWebSettingsChange} />);
    expect(screen.getByRole('combobox', { name: '難易度' })).toHaveAttribute('data-selected-label', '3: ふつう');

    fireEvent.click(screen.getByRole('combobox', { name: '難易度' }));
    fireEvent.click(screen.getByRole('option', { name: '4: むずかしい' }));
    expect(screen.getByRole('combobox', { name: '難易度' })).toHaveAttribute('data-selected-label', '4: むずかしい');
    await waitFor(() => expect(onWebSettingsChange).toHaveBeenLastCalledWith({
      schema_version: 3,
      numeric_theme_id: 1,
      themeKey: 'jp.grade1.addition.one_digit',
      difficulty: 4,
      seed: '',
    }));
  });

  it('preselects an implemented unit from route-provided Web settings', () => {
    render(
      <AutoDrillApp
        engine={fixtureEngine()}
        initialWebSettings={createWebDrillSettings(ONE_DIGIT_ADDITION_THEME, 5)}
      />,
    );
    expect(screen.getByRole('combobox', { name: 'ジャンル' })).toHaveAttribute('data-selected-label', '足し算と引き算');
    expect(screen.getByRole('combobox', { name: 'テーマ' })).toHaveAttribute('data-selected-label', '一桁の足し算');
    expect(screen.getByRole('combobox', { name: '難易度' })).toHaveAttribute('data-selected-label', '5: とてもむずかしい');
  });

  it('disables q1 actions and announces problem generation while it is pending', async () => {
    const { engine, resolveGeneration } = deferredGenerationEngine();
    render(<AutoDrillApp engine={engine} />);

    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    expect(screen.getByRole('button', { name: '問題を生成中…' })).toBeDisabled();
    expect(screen.getByRole('button', { name: '印刷' })).toBeDisabled();
    expect(screen.getByLabelText('問題を生成しています。しばらくお待ちください。')).toBeInTheDocument();

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
    expect(screen.getByLabelText('印刷用PDFを準備しています。しばらくお待ちください。')).toBeInTheDocument();

    resolveGeneration(fixtureWorksheet());
    await waitFor(() => expect(screen.getByRole('button', { name: '印刷' })).toBeEnabled());
  });

  it('transitions from q1 generation to the q2 worksheet', async () => {
    render(<AutoDrillApp engine={fixtureEngine()} />);
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    expect(await screen.findByRole('heading', { name: '1けたのたしざん(1)' })).toBeInTheDocument();
    expect(screen.getByLabelText('20問の1けたのたしざん(1)ワークシート')).toBeInTheDocument();
    expect(screen.queryByLabelText('数式入力パネル')).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: /^1番の答え/ }));
    expect(screen.getByLabelText('数式入力パネル')).toBeInTheDocument();
  });

  it('renders the 16-problem linear-equation worksheet with x = answer slots and the full rich keyboard', async () => {
    const worksheet = linearFixtureWorksheet(2);
    render(
      <AutoDrillApp
        engine={fixtureEngine(worksheet)}
        initialWebSettings={createWebDrillSettings(LINEAR_EQUATION_1_THEME)}
      />,
    );
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    expect(await screen.findByRole('heading', { name: '一次方程式(1)' })).toBeInTheDocument();
    expect(screen.getByLabelText('16問の一次方程式(1)ワークシート')).toBeInTheDocument();
    expect(screen.getByText('次の一次方程式を解きなさい。ただし、答えが整数でない場合は約分によって最も簡単な形の仮分数で答えなさい。')).toBeInTheDocument();

    const firstCell = screen.getByTestId('problem-cell-0');
    expect(firstCell).toHaveClass('problem-cell-linear-equation');
    expect(firstCell.querySelector('.problem-math-expression')).toHaveTextContent('2x = x + (−5)');
    expect(within(firstCell).getByText('x =')).toBeTruthy();

    fireEvent.click(within(firstCell).getByRole('button', { name: /^1番の答え/ }));
    const formulaPad = screen.getByLabelText('数式テンプレート');
    expect(within(formulaPad).getAllByRole('button').map((button) => button.getAttribute('aria-label'))).toEqual([
      '分数', '帯分数', '平方根', 'マイナス', 'プラスマイナス', '複数解',
    ]);
    expect(within(screen.getByLabelText('数字キー')).getByRole('button', { name: '小数点' })).toBeInTheDocument();
    expect(within(screen.getByLabelText('編集キー')).getAllByRole('button').slice(0, 2).map((button) => button.textContent)).toEqual(['←', '→']);
  });

  it('derives q2 cell positions and order from the shared A4 layout', async () => {
    render(<AutoDrillApp engine={fixtureEngine()} />);
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    await screen.findByRole('heading', { name: '1けたのたしざん(1)' });

    const layout = buildSharedWorksheetLayout(fixtureWorksheet());
    const first = getCellTopPosition(layout, layout.cells[0]!);
    const tenth = getCellTopPosition(layout, layout.cells[9]!);
    const eleventh = getCellTopPosition(layout, layout.cells[10]!);
    const last = getCellTopPosition(layout, layout.cells[19]!);
    const firstCell = screen.getByTestId('problem-cell-0');
    const tenthCell = screen.getByTestId('problem-cell-9');
    const eleventhCell = screen.getByTestId('problem-cell-10');
    const lastCell = screen.getByTestId('problem-cell-19');
    const percent = (value: number, total: number) => `${(value / total) * 100}%`;

    expect(firstCell.style.top).toBe(percent(first.y, A4_PAGE.height));
    expect(tenthCell.style.top).toBe(percent(tenth.y, A4_PAGE.height));
    expect(eleventhCell.style.top).toBe(percent(eleventh.y, A4_PAGE.height));
    expect(lastCell.style.top).toBe(percent(last.y, A4_PAGE.height));
    expect(tenthCell.style.left).toBe(firstCell.style.left);
    expect(Number.parseFloat(tenthCell.style.top)).toBeGreaterThan(Number.parseFloat(firstCell.style.top));
    expect(eleventhCell.style.top).toBe(firstCell.style.top);
    expect(Number.parseFloat(eleventhCell.style.left)).toBeGreaterThan(Number.parseFloat(firstCell.style.left));
    expect(firstCell.dataset.layoutIndex).toBe('0');
    expect(lastCell.dataset.layoutIndex).toBe('19');
    expect(screen.getByTestId('problem-divider').style.left).toBe(percent(layout.dividerX, A4_PAGE.width));
  });

  it('resets the selected editor on TOP and on each regenerated worksheet', async () => {
    render(<AutoDrillApp engine={fixtureEngine()} />);
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    await screen.findByRole('heading', { name: '1けたのたしざん(1)' });
    fireEvent.click(screen.getByRole('button', { name: /^1番の答え/ }));
    expect(screen.getByLabelText('数式入力パネル')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'TOPに戻る' }));
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    await screen.findByRole('heading', { name: '1けたのたしざん(1)' });
    expect(screen.queryByLabelText('数式入力パネル')).not.toBeInTheDocument();
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
    expect(screen.getByLabelText('数式入力パネル')).toBeInTheDocument();
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
    expect(within(numberPad).queryByRole('button', { name: '小数点' })).not.toBeInTheDocument();
    expect(screen.queryByLabelText('数式テンプレート')).not.toBeInTheDocument();
    const editControls = within(screen.getByLabelText('編集キー'));
    expect(editControls.getByRole('button', { name: '一文字戻す' })).toBeInTheDocument();
    expect(editControls.queryByRole('button', { name: '一文字削除' })).not.toBeInTheDocument();
    expect(editControls.getAllByRole('button').slice(0, 2).map((button) => button.getAttribute('aria-label'))).toEqual([
      'カーソルを左へ', 'カーソルを右へ',
    ]);
    expect(screen.queryByText(/^AST:/)).not.toBeInTheDocument();
  });

  it('projects an allowed simple-numeric negative control and shares its action with physical minus', async () => {
    const base = simpleNumericFixtureEngine(true);
    const applyEditorAction = vi.fn(base.applyEditorAction);
    render(<AutoDrillApp engine={{ ...base, applyEditorAction }} />);
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    await screen.findByRole('heading', { name: '1けたのたしざん(1)' });
    fireEvent.click(screen.getByRole('button', { name: /^1番の答え/ }));

    expect(screen.getByRole('button', { name: 'マイナス' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '小数点' })).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'マイナス' }));
    await waitFor(() => expect(applyEditorAction).toHaveBeenCalledTimes(1));
    fireEvent.keyDown(window, { key: '-' });
    await waitFor(() => expect(applyEditorAction).toHaveBeenCalledTimes(2));
    expect(applyEditorAction.mock.calls[0]?.[1]).toEqual({ kind: 'insert_structure', structure: 'negative' });
    expect(applyEditorAction.mock.calls[1]?.[1]).toEqual(applyEditorAction.mock.calls[0]?.[1]);
  });

  it('hides the negative control and ignores physical minus when simple-numeric negatives are disallowed', async () => {
    const base = simpleNumericFixtureEngine(false);
    const applyEditorAction = vi.fn(base.applyEditorAction);
    render(<AutoDrillApp engine={{ ...base, applyEditorAction }} />);
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    await screen.findByRole('heading', { name: '1けたのたしざん(1)' });
    fireEvent.click(screen.getByRole('button', { name: /^1番の答え/ }));

    expect(screen.queryByRole('button', { name: 'マイナス' })).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: '小数点' })).toBeInTheDocument();
    fireEvent.keyDown(window, { key: '-' });
    expect(applyEditorAction).not.toHaveBeenCalled();
  });

  it('keeps nan_error text visible and editable without treating it as a number', async () => {
    const base = fixtureEngine();
    const engine: DrillEngine = {
      ...base,
      async applyEditorAction(state, action, inputInterface) {
        if (action.kind === 'insert_digit') {
          return {
            ...state,
            answer: { type: 'nan_error', value: '1e+' },
            cursor: 3,
            committed: false,
          };
        }
        if (action.kind === 'select_slot') {
          return { ...state, active_path: [...action.path], cursor: action.cursor };
        }
        return base.applyEditorAction(state, action, inputInterface);
      },
    };
    render(<AutoDrillApp engine={engine} />);
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    await screen.findByRole('heading', { name: '1けたのたしざん(1)' });
    fireEvent.click(screen.getByRole('button', { name: /^1番の答え/ }));
    fireEvent.keyDown(window, { key: '1' });

    const malformed = await screen.findByRole('button', { name: '1番の答え 1e+' });
    expect(malformed).toHaveClass('answer-box-selected');
    expect(malformed.querySelector('[data-slot-path=""]')).toHaveTextContent('1e+');
    fireEvent.click(malformed.querySelector('[data-slot-path=""]')!);
    fireEvent.click(screen.getByRole('button', { name: '採点' }));
    await waitFor(() => expect(screen.getByRole('status')).toHaveTextContent('0 / 20'));
    expect(screen.getByRole('button', { name: '1番の答え 1e+' })).toHaveClass('answer-box-wrong');
  });

  it('shows visual math templates to the left of the number pad and renders editable fraction slots', async () => {
    const { container } = render(<AutoDrillApp engine={structuredFixtureEngine()} />);
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    await screen.findByRole('heading', { name: '1けたのたしざん(1)' });
    fireEvent.click(screen.getByRole('button', { name: /^1番の答え/ }));

    const templates = screen.getByLabelText('数式テンプレート');
    expect(within(templates).getAllByRole('button').map((button) => button.getAttribute('aria-label'))).toEqual([
      '分数', '帯分数', '平方根', 'マイナス', 'プラスマイナス', '複数解',
    ]);

    expect(templates.querySelectorAll('math.math-template-icon')).toHaveLength(6);
    expect(templates.querySelectorAll('.math-template-slot').length).toBeGreaterThanOrEqual(8);
    expect(templates.querySelector('math.math-template-icon mfrac')).not.toBeNull();
    expect(templates.querySelector('math.math-template-icon msqrt')).not.toBeNull();

    fireEvent.click(within(templates).getByRole('button', { name: '分数' }));
    await waitFor(() => expect(container.querySelector('math.answer-math mfrac')).not.toBeNull());
    expect(screen.getByRole('status', { name: /入力位置 分子/ })).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: '7' }));
    await waitFor(() => expect(screen.getByRole('button', { name: '1番の答え 7/' })).toBeInTheDocument());
    const caretPad = container.querySelector('math.answer-math mpadded');
    expect(caretPad?.getAttribute('width')).toBe('0');
    expect(caretPad?.getAttribute('height')).toBe('0');
    expect(caretPad?.getAttribute('depth')).toBe('0');
    fireEvent.click(container.querySelector('[data-slot-path="1"]')!);
    await waitFor(() => expect(screen.getByRole('status', { name: /入力位置 分母/ })).toBeInTheDocument());
    fireEvent.click(screen.getByRole('button', { name: '2' }));
    await waitFor(() => expect(screen.getByRole('button', { name: '1番の答え 7/2' })).toBeInTheDocument());
    expectVisibleKanjiToUseRuby(container);
  });

  it('renders square roots entirely through native MathML', async () => {
    const { container } = render(<AutoDrillApp engine={structuredFixtureEngine()} />);
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    await screen.findByRole('heading', { name: '1けたのたしざん(1)' });
    fireEvent.click(screen.getByRole('button', { name: /^1番の答え/ }));
    fireEvent.click(screen.getByRole('button', { name: '平方根' }));

    await waitFor(() => expect(container.querySelector('math.answer-math > msqrt')).not.toBeNull());
    expect(container.querySelector('math.answer-math svg')).toBeNull();
    expect(container.querySelector('.answer-radicand')).toBeNull();
  });

  it('removes an empty structured template when Backspace is pressed', async () => {
    const { container } = render(<AutoDrillApp engine={structuredFixtureEngine()} />);
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    await screen.findByRole('heading', { name: '1けたのたしざん(1)' });
    fireEvent.click(screen.getByRole('button', { name: /^1番の答え/ }));
    fireEvent.click(screen.getByRole('button', { name: '分数' }));
    await waitFor(() => expect(container.querySelector('math.answer-math mfrac')).not.toBeNull());

    fireEvent.keyDown(window, { key: 'Backspace' });
    await waitFor(() => expect(container.querySelector('math.answer-math mfrac')).toBeNull());
    expect(screen.getByRole('button', { name: '1番の答え 未入力' })).toBeInTheDocument();
  });

  it('maps physical decimal, fraction, negative, plus-minus, and tuple keys to AST actions', async () => {
    const base = structuredFixtureEngine();
    const applyEditorAction = vi.fn(base.applyEditorAction);
    render(<AutoDrillApp engine={{ ...base, applyEditorAction }} />);
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    await screen.findByRole('heading', { name: '1けたのたしざん(1)' });
    fireEvent.click(screen.getByRole('button', { name: /^1番の答え/ }));

    for (const key of ['.', '/', '-', '+', ',']) fireEvent.keyDown(window, { key });
    await waitFor(() => expect(applyEditorAction).toHaveBeenCalledTimes(5));
    expect(applyEditorAction.mock.calls.map(([, action]) => action)).toEqual([
      { kind: 'insert_structure', structure: 'decimal' },
      { kind: 'insert_structure', structure: 'fraction' },
      { kind: 'insert_structure', structure: 'negative' },
      { kind: 'insert_structure', structure: 'plus_minus' },
      { kind: 'insert_structure', structure: 'tuple' },
    ]);
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

  it('renders the editor cursor at its actual insertion position', async () => {
    render(<AutoDrillApp engine={fixtureEngine()} />);
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    await screen.findByRole('heading', { name: '1けたのたしざん(1)' });
    fireEvent.click(screen.getByRole('button', { name: /^1番の答え/ }));
    fireEvent.keyDown(window, { key: '1' });
    fireEvent.keyDown(window, { key: '2' });
    await screen.findByRole('button', { name: '1番の答え 12' });

    fireEvent.keyDown(window, { key: 'ArrowLeft' });
    await waitFor(() => {
      expect(screen.getByTestId('answer-before-caret-0')).toHaveTextContent('1');
      expect(screen.getByTestId('answer-caret-0')).toBeTruthy();
      expect(screen.getByTestId('answer-after-caret-0')).toHaveTextContent('2');
    });
    fireEvent.keyDown(window, { key: '9' });
    expect(await screen.findByRole('button', { name: '1番の答え 192' })).toBeInTheDocument();
  });

  it('scrolls exactly one row from the first to second problem while both are in the safe viewport', async () => {
    const requestAnimationFrameSpy = vi.spyOn(window, 'requestAnimationFrame').mockImplementation((callback) => {
      callback(0);
      return 1;
    });
    const scrollBySpy = vi.spyOn(window, 'scrollBy').mockImplementation(() => undefined);
    try {
      render(<AutoDrillApp engine={fixtureEngine()} />);
      fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
      await screen.findByRole('heading', { name: '1けたのたしざん(1)' });
      fireEvent.click(screen.getByRole('button', { name: /^1番の答え/ }));

      const currentCell = screen.getByTestId('problem-cell-0');
      const nextCell = screen.getByTestId('problem-cell-1');
      const scrollIntoView = vi.fn();
      Object.defineProperty(nextCell, 'scrollIntoView', { configurable: true, value: scrollIntoView });
      vi.spyOn(currentCell, 'getBoundingClientRect').mockReturnValue({
        bottom: 250, height: 50, left: 0, right: 100, top: 200, width: 100, x: 0, y: 200, toJSON: () => ({}),
      });
      vi.spyOn(nextCell, 'getBoundingClientRect').mockReturnValue({
        bottom: 300, height: 50, left: 0, right: 100, top: 250, width: 100, x: 0, y: 250, toJSON: () => ({}),
      });
      vi.spyOn(document.querySelector<HTMLElement>('.ribbon')!, 'getBoundingClientRect').mockReturnValue({
        bottom: 80, height: 80, left: 0, right: 720, top: 0, width: 720, x: 0, y: 0, toJSON: () => ({}),
      });
      vi.spyOn(document.querySelector<HTMLElement>('.input-panel')!, 'getBoundingClientRect').mockReturnValue({
        bottom: 700, height: 300, left: 0, right: 720, top: 400, width: 720, x: 0, y: 400, toJSON: () => ({}),
      });

      fireEvent.keyDown(window, { key: 'Enter' });
      await waitFor(() => expect(scrollIntoView).toHaveBeenCalledWith({ block: 'nearest', inline: 'nearest' }));
      expect(scrollBySpy).toHaveBeenCalledWith({ top: 50, behavior: 'auto' });
    } finally {
      requestAnimationFrameSpy.mockRestore();
      scrollBySpy.mockRestore();
    }
  });

  it('resets problem 11 to the safe ribbon top when advancing from problem 10', async () => {
    const requestAnimationFrameSpy = vi.spyOn(window, 'requestAnimationFrame').mockImplementation((callback) => {
      callback(0);
      return 1;
    });
    const scrollBySpy = vi.spyOn(window, 'scrollBy').mockImplementation(() => undefined);
    try {
      render(<AutoDrillApp engine={fixtureEngine()} />);
      fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
      await screen.findByRole('heading', { name: '1けたのたしざん(1)' });
      fireEvent.click(screen.getByRole('button', { name: /^10番の答え/ }));

      const currentCell = screen.getByTestId('problem-cell-9');
      const nextCell = screen.getByTestId('problem-cell-10');
      const scrollIntoView = vi.fn();
      Object.defineProperty(nextCell, 'scrollIntoView', { configurable: true, value: scrollIntoView });
      vi.spyOn(currentCell, 'getBoundingClientRect').mockReturnValue({
        bottom: 350, height: 50, left: 0, right: 100, top: 300, width: 100, x: 0, y: 300, toJSON: () => ({}),
      });
      vi.spyOn(nextCell, 'getBoundingClientRect').mockReturnValue({
        bottom: -100, height: 50, left: 360, right: 460, top: -150, width: 100, x: 360, y: -150, toJSON: () => ({}),
      });
      vi.spyOn(document.querySelector<HTMLElement>('.ribbon')!, 'getBoundingClientRect').mockReturnValue({
        bottom: 80, height: 80, left: 0, right: 720, top: 0, width: 720, x: 0, y: 0, toJSON: () => ({}),
      });
      vi.spyOn(document.querySelector<HTMLElement>('.input-panel')!, 'getBoundingClientRect').mockReturnValue({
        bottom: 700, height: 300, left: 0, right: 720, top: 400, width: 720, x: 0, y: 400, toJSON: () => ({}),
      });

      fireEvent.keyDown(window, { key: 'Enter' });
      await waitFor(() => expect(scrollIntoView).toHaveBeenCalledWith({ block: 'nearest', inline: 'nearest' }));
      expect(scrollBySpy).toHaveBeenCalledWith({ top: -242, behavior: 'auto' });
      expect(scrollBySpy).not.toHaveBeenCalledWith({ top: -450, behavior: 'auto' });
    } finally {
      requestAnimationFrameSpy.mockRestore();
      scrollBySpy.mockRestore();
    }
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
    expect(emptyAnswer).toHaveStyle({ width: 'max-content', flexGrow: '0', flexShrink: '0' });
    fireEvent.click(emptyAnswer);

    fireEvent.keyDown(window, { key: '1' });
    fireEvent.keyDown(window, { key: '1' });
    const twoDigits = await screen.findByRole('button', { name: '1番の答え 11' });
    expect(twoDigits).toHaveStyle({ width: 'max-content', flexGrow: '0', flexShrink: '0' });

    for (let index = 2; index < 19; index += 1) fireEvent.keyDown(window, { key: '1' });
    const eighteenDigits = '1'.repeat(18);
    const answer = await screen.findByRole('button', { name: `1番の答え ${eighteenDigits}` });
    expect(answer).toHaveAttribute('data-answer-length', '18');
    expect(answer).toHaveStyle({ width: 'max-content', fontSize: '11px', flexGrow: '0', flexShrink: '0' });
    expect(await screen.findByLabelText('式が大きすぎます！')).toBeInTheDocument();

    fireEvent.keyDown(window, { key: 'Backspace' });
    await screen.findByRole('button', { name: `1番の答え ${'1'.repeat(17)}` });
    await waitFor(() => expect(screen.queryByLabelText('式が大きすぎます！')).not.toBeInTheDocument());
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

  it('shows representation warnings on mathematically correct answers', async () => {
    const { container } = render(<AutoDrillApp engine={warningFixtureEngine()} />);
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    await screen.findByRole('heading', { name: '1けたのたしざん(1)' });
    fireEvent.click(screen.getByRole('button', { name: '採点' }));

    const warnings = await screen.findByLabelText('注意 約分、冗長なマイナス、余計な小数点、整数で答えましょう');
    expect(warnings).toBeInTheDocument();
    expect(warnings.querySelectorAll('ruby')).toHaveLength(6);
    expectVisibleKanjiToUseRuby(container);
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

      resolveGrade({ schema_version: 3, items: [], correct_count: 0, total_count: 20 });
      await act(async () => {
        await Promise.resolve();
        await Promise.resolve();
      });
      expect(screen.getByTestId('elapsed-time')).toHaveTextContent('00:03');
    } finally {
      vi.useRealTimers();
    }
  });

  it('hides the keypad as soon as grading starts and resumes preserved work from frozen elapsed time', async () => {
    vi.useFakeTimers();
    vi.setSystemTime(1_000);
    try {
      render(<AutoDrillApp engine={fixtureEngine()} />);
      fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
      await act(async () => {
        await Promise.resolve();
        await Promise.resolve();
      });
      fireEvent.click(screen.getByRole('button', { name: /^1番の答え/ }));
      fireEvent.keyDown(window, { key: '9' });
      await act(async () => {
        await Promise.resolve();
        await Promise.resolve();
      });
      act(() => vi.advanceTimersByTime(3_000));
      fireEvent.click(screen.getByRole('button', { name: '採点' }));
      expect(screen.queryByLabelText('数字入力パネル')).not.toBeInTheDocument();
      await act(async () => {
        await Promise.resolve();
        await Promise.resolve();
      });
      expect(screen.getByRole('button', { name: '問題に戻る' })).toBeInTheDocument();
      expect(screen.getByTestId('elapsed-time')).toHaveTextContent('00:03');

      act(() => vi.advanceTimersByTime(5_000));
      fireEvent.click(screen.getByRole('button', { name: '問題に戻る' }));
      expect(screen.getByRole('button', { name: '1番の答え 9' })).not.toHaveClass('answer-box-wrong');
      expect(screen.queryByLabelText('数字入力パネル')).not.toBeInTheDocument();
      expect(screen.queryByRole('button', { name: '問題に戻る' })).not.toBeInTheDocument();
      act(() => vi.advanceTimersByTime(1_000));
      expect(screen.getByTestId('elapsed-time')).toHaveTextContent('00:04');
    } finally {
      vi.useRealTimers();
    }
  });

  it('restarts the same worksheet with cleared answers, grades, timer, and keypad', async () => {
    const engine = fixtureEngine();
    const generateSpy = vi.spyOn(engine, 'generateWorksheet');
    render(<AutoDrillApp engine={engine} initialSettings={fixtureSettings()} />);
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    await screen.findByRole('heading', { name: '1けたのたしざん(1)' });
    const footer = screen.getByTestId('worksheet-footer').textContent;
    fireEvent.click(screen.getByRole('button', { name: /^1番の答え/ }));
    fireEvent.keyDown(window, { key: '2' });
    await screen.findByRole('button', { name: '1番の答え 2' });
    fireEvent.click(screen.getByRole('button', { name: '採点' }));
    await screen.findByRole('button', { name: 'もう一回問題を解く' });

    fireEvent.click(screen.getByRole('button', { name: 'もう一回問題を解く' }));
    expect(screen.getByRole('button', { name: '1番の答え 未入力' })).toBeInTheDocument();
    expect(screen.getByTestId('worksheet-footer')).toHaveTextContent(footer ?? '');
    expect(screen.getByTestId('elapsed-time')).toHaveTextContent('00:00');
    expect(screen.queryByLabelText('数字入力パネル')).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'もう一回問題を解く' })).not.toBeInTheDocument();
    expect(generateSpy).toHaveBeenCalledTimes(1);
  });

  it('generates a different automatic-seed worksheet in the same unit and resets q2', async () => {
    const { engine, seeds } = seedRecordingEngine();
    const automaticSeeds = ['firstSeed', 'nextSeed'];
    let index = 0;
    render(<AutoDrillApp engine={engine} seedGenerator={() => automaticSeeds[index++]!} dateGenerator={() => new Date(2026, 6, 31)} />);
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    await screen.findByRole('heading', { name: '1けたのたしざん(1)' });
    fireEvent.click(screen.getByRole('button', { name: /^1番の答え/ }));
    fireEvent.keyDown(window, { key: '9' });
    await screen.findByRole('button', { name: '1番の答え 9' });
    fireEvent.click(screen.getByRole('button', { name: '採点' }));
    await screen.findByRole('button', { name: '別の問題を解く' });

    fireEvent.click(screen.getByRole('button', { name: '別の問題を解く' }));
    await waitFor(() => expect(seeds).toEqual(automaticSeeds));
    expect(screen.getByRole('heading', { name: '1けたのたしざん(1)' })).toBeInTheDocument();
    expect(screen.getByTestId('worksheet-footer')).toHaveTextContent('nextSeed');
    expect(screen.getByRole('button', { name: '1番の答え 未入力' })).toBeInTheDocument();
    expect(screen.getByTestId('elapsed-time')).toHaveTextContent('00:00');
    expect(screen.queryByLabelText('数字入力パネル')).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: '別の問題を解く' })).not.toBeInTheDocument();
  });

  it('renders every visible q1 and q2 kanji string with semantic ruby', async () => {
    const { container } = render(<AutoDrillApp engine={fixtureEngine()} />);
    expectVisibleKanjiToUseRuby(container);
    expect(container.querySelector('.custom-select-value ruby')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    await screen.findByRole('heading', { name: '1けたのたしざん(1)' });
    expectVisibleKanjiToUseRuby(container);
    fireEvent.click(screen.getByRole('button', { name: '採点' }));
    await screen.findByRole('button', { name: '問題に戻る' });
    expectVisibleKanjiToUseRuby(container);
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
