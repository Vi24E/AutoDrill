import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { AutoDrillApp } from '@/components/AutoDrillApp';
import { deleteEmptyMathLiveStructureBackward, type AutoDrillMathfield } from '@/components/MathLiveMath';
import { createWebDrillSettings, findImplementedThemeByNumericId, LINEAR_EQUATION_1_THEME, ONE_DIGIT_ADDITION_THEME } from '@/domain/curriculum';
import { DECIMAL_ADD_SUBTRACT_DEFINITION } from '@/domain/themes/decimal-add-subtract';
import { MINI_SUDOKU_DEFINITION } from '@/domain/themes/mini-sudoku';
import { COLUMN_DIVIDE_2DIGIT_BY_1DIGIT_DEFINITION } from '@/domain/themes/column-divide-two-digit-by-one-digit';
import { COLUMN_DECIMAL_MULTIPLICATION_DEFINITION } from '@/domain/themes/column-decimal-multiplication';
import { SIGNED_ARITHMETIC_1_DEFINITION } from '@/domain/themes/signed-arithmetic-1';
import { A4_PAGE, buildSharedWorksheetLayout, getCellTopPosition } from '@/domain/layout';
import { problemSetIdFromSearch } from '@/domain/problem-set-url';
import { buildPdfPageModel } from '@/pdf/worksheet-pdf';
import { columnDecimalMultiplicationFixtureWorksheet, columnDivisionFixtureWorksheet, fixtureEngine, fixtureSettings, fixtureWorksheet, liarFixtureWorksheet, linearExpressionFixtureWorksheet, linearFixtureWorksheet, miniSudokuFixtureWorksheet, simultaneousFixtureWorksheet } from '@/test/fixtures';
import { DRILL_SCHEMA_VERSION, type DrillEngine, type WorksheetDto } from '@/domain/drill-engine';
import { DRILL_CORE_CONTRACT } from '@/generated/drill-core-contract';

function pressKey(key: string) {
  const target = document.querySelector<HTMLElement>('math-field.answer-mathfield-selected') ?? document.activeElement;
  if (!(target instanceof HTMLElement)) throw new Error('No active answer mathfield.');
  fireEvent.keyDown(target, { key });
}

function answerFrame(field: HTMLElement): HTMLElement {
  const frame = field.closest<HTMLElement>('.answer-box');
  if (!frame) throw new Error('Answer field has no answer-box frame.');
  return frame;
}
vi.mock('@/pdf/worksheet-pdf', async () => {
  const actual = await vi.importActual<typeof import('@/pdf/worksheet-pdf')>('@/pdf/worksheet-pdf');
  return { ...actual, openWorksheetPdf: vi.fn().mockResolvedValue(undefined) };
});

function delayedFixtureEngine() {
  const base = fixtureEngine();
  return {
    ...base,
    async parseMathLiveAnswer(...args: Parameters<typeof base.parseMathLiveAnswer>) {
      await new Promise<void>((resolve) => window.setTimeout(resolve, 5));
      return base.parseMathLiveAnswer(...args);
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

function failingGradingEngine(): DrillEngine {
  const base = fixtureEngine();
  return {
    ...base,
    gradeAnswer: vi.fn(async () => {
      throw new Error('grade failed');
    }),
  };
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
          warnings: ['fraction_not_reduced', 'redundant_negative', 'redundant_decimal', 'integer_form_required', 'fraction_form_required'] as const,
        } : item),
        correct_count: result.correct_count + 1,
      };
    },
  };
}

function fractionFormWarningFixtureEngine(): DrillEngine {
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
          warnings: ['fraction_form_required'] as const,
        } : item),
        correct_count: result.correct_count + 1,
      };
    },
  };
}

function numericThemeFixtureEngine(definition: typeof DECIMAL_ADD_SUBTRACT_DEFINITION | typeof SIGNED_ARITHMETIC_1_DEFINITION): DrillEngine {
  const worksheet = fixtureWorksheet();
  worksheet.identity.numeric_theme_id = definition.numeric_theme_id;
  worksheet.identity.generator_revision = definition.generator_revision;
  worksheet.layout = definition.layout;
  worksheet.problems = worksheet.problems.slice(0, definition.problemCount).map((problem) => ({
    ...problem,
    numeric_theme_id: definition.numeric_theme_id,
    input_interface: definition.inputInterface,
  }));
  return fixtureEngine(worksheet);
}

function parseStructuredFixtureLatex(latex: string): Awaited<ReturnType<DrillEngine['parseMathLiveAnswer']>> {
  const clean = latex.replaceAll('\\placeholder{}', '');
  if (clean === '') return { type: 'empty' };
  if (/^\d+$/.test(clean)) return { type: 'integer', value: String(BigInt(clean)) };
  if (/^\d+\.\d*$/.test(clean)) {
    const [whole, fraction = ''] = clean.split('.');
    return { type: 'exact_decimal', value: { coefficient: `${whole}${fraction}`, scale: fraction.length } };
  }
  const fraction = /^\\frac\{([^{}]*)\}\{([^{}]*)\}$/.exec(clean);
  if (fraction) {
    return {
      type: 'fraction',
      value: {
        numerator: parseStructuredFixtureLatex(fraction[1]!),
        denominator: parseStructuredFixtureLatex(fraction[2]!),
      },
    };
  }
  const mixed = /^(\d+)\\frac\{([^{}]*)\}\{([^{}]*)\}$/.exec(clean);
  if (mixed) {
    return {
      type: 'mixed_fraction',
      value: {
        whole: { type: 'integer', value: mixed[1]! },
        numerator: parseStructuredFixtureLatex(mixed[2]!),
        denominator: parseStructuredFixtureLatex(mixed[3]!),
      },
    };
  }
  const root = /^\\sqrt\{(.*)\}$/.exec(clean);
  if (root) return { type: 'root', value: { radicand: parseStructuredFixtureLatex(root[1]!), index: null } };
  if (latex.startsWith('\\pm')) return { type: 'plus_minus', value: parseStructuredFixtureLatex(latex.slice(3)) };
  if (latex.startsWith('-')) return { type: 'negative', value: parseStructuredFixtureLatex(latex.slice(1)) };
  if (latex.includes(',')) return { type: 'tuple', value: latex.split(',').map(parseStructuredFixtureLatex) };
  return { type: 'nan_error', value: latex };
}

function structuredFixtureEngine(): DrillEngine {
  const worksheet = linearFixtureWorksheet(2);
  const base = fixtureEngine(worksheet);
  return {
    ...base,
    async parseMathLiveAnswer(latex) {
      return parseStructuredFixtureLatex(latex);
    },
  };
}

function seedRecordingEngine() {
  const seeds: string[] = [];
  const problemSetIds: string[] = [];
  const base = fixtureEngine();
  const engine: DrillEngine = {
    ...base,
    async generateWorksheet(settings) {
      seeds.push(settings.seed);
      const worksheet = fixtureWorksheet();
      worksheet.identity.numeric_theme_id = settings.numeric_theme_id;
      worksheet.identity.seed = settings.seed;
      worksheet.identity.difficulty = settings.difficulty;
      worksheet.seed = settings.seed;
      worksheet.problem_set_id = `${worksheet.identity.schema_version}-${worksheet.identity.numeric_theme_id}-${worksheet.identity.generator_revision}-${settings.seed}-${worksheet.identity.difficulty}`;
      return worksheet;
    },
    async generateWorksheetById(problemSetId) {
      problemSetIds.push(problemSetId);
      return fixtureWorksheet();
    },
  };
  return { engine, seeds, problemSetIds };
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
  beforeEach(() => {
    window.localStorage.clear();
    window.history.replaceState(null, '', '/');
    delete window.__AUTODRILL_WASM__;
    delete window.__AUTODRILL_SCHEMA_VERSION__;
  });

  it('enables furigana by default on q1', () => {
    const { container } = render(<AutoDrillApp engine={fixtureEngine()} />);
    expect(screen.getByRole('checkbox', { name: 'ふりがな' })).toBeChecked();
    expect(container.querySelector('ruby')).toBeInTheDocument();
    expect(container.querySelector('rt')).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'まいんドリル' })).toBeInTheDocument();
    expect(screen.getByLabelText('自分だけの計算ドリルを作る')).toBeInTheDocument();
    expect(screen.queryByLabelText('今日のステージを選んで、20問のドリルを始めよう。')).not.toBeInTheDocument();
  });

  it('keeps Seed in collapsed details and exposes the revised q1 chrome', () => {
    const { container } = render(<AutoDrillApp engine={fixtureEngine()} />);
    const details = container.querySelector<HTMLDetailsElement>('details.advanced-settings');
    expect(details).not.toBeNull();
    expect(details?.open).toBe(false);
    expect(screen.getByText('詳細設定')).toBeInTheDocument();
    const seedLabel = container.querySelector<HTMLLabelElement>('label[for="seed-input"]');
    expect(seedLabel).not.toBeNull();
    const seedLabelClone = seedLabel?.cloneNode(true) as HTMLElement;
    seedLabelClone.querySelectorAll('rt').forEach((reading) => reading.remove());
    expect(seedLabelClone.textContent).toContain('Seed (単元・難易度を含む問題ID)');
    expect(screen.queryByText('同じSeedで同じ問題を再現できます。空欄なら毎回新しく生成します。')).not.toBeInTheDocument();
    expect(screen.queryByText('問題の生成・入力状態・採点は Rust/WASM が担当します。')).not.toBeInTheDocument();

    const printButton = screen.getByRole('button', { name: '印刷 (pdfで出力)' });
    expect(printButton.querySelector('.share-pdf-icon')).toBeInTheDocument();
    const version = screen.getByLabelText('AutoDrill alpha 1.3');
    expect(version).toHaveClass('settings-version');
    expect(version.closest('.lobby-panel')).toBeNull();

    fireEvent.click(screen.getByText('詳細設定'));
    expect(details?.open).toBe(true);
    expect(screen.getByLabelText('Seed')).toBeInTheDocument();
  });

  it('never renders furigana inside detailed settings or its grading modal', async () => {
    const { container } = render(<AutoDrillApp engine={fixtureEngine()} />);
    expect(screen.getByRole('checkbox', { name: 'ふりがな' })).toBeChecked();

    const details = container.querySelector<HTMLDetailsElement>('details.advanced-settings');
    expect(details).not.toBeNull();
    fireEvent.click(screen.getByText('詳細設定'));
    expect(details?.querySelector('ruby')).toBeNull();
    expect(details?.querySelector('rt')).toBeNull();

    fireEvent.click(screen.getByRole('button', { name: '採点設定' }));
    const dialog = await screen.findByRole('dialog', { name: '採点設定' });
    expect(dialog.querySelector('ruby')).toBeNull();
    expect(dialog.querySelector('rt')).toBeNull();
    expect(dialog).toHaveTextContent('約分しましょう');
    expect(dialog).toHaveTextContent('整数でこたえましょう');
    expect(dialog).toHaveTextContent('分数でこたえましょう');
    expect(dialog).toHaveTextContent('最後まで計算しましょう');
  });

  it('turns furigana off across q1 and q2 and retains that setting after TOP', async () => {
    const { container } = render(<AutoDrillApp engine={fixtureEngine()} />);
    fireEvent.click(screen.getByRole('checkbox', { name: 'ふりがな' }));
    expect(screen.getByRole('checkbox', { name: 'ふりがな' })).not.toBeChecked();
    expect(container.querySelector('ruby')).not.toBeInTheDocument();
    expect(container.querySelector('rt')).not.toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'まいんドリル' })).toBeInTheDocument();
    expect(screen.getByRole('combobox', { name: 'ジャンル' })).toHaveAttribute('data-selected-label', '足し算と引き算');
    fireEvent.click(screen.getByRole('combobox', { name: 'ジャンル' }));
    expect(screen.getByRole('option', { name: '方程式' })).toBeInTheDocument();
    fireEvent.keyDown(screen.getByRole('combobox', { name: 'ジャンル' }), { key: 'Escape' });

    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    await screen.findByRole('heading', { name: '1けたのたしざん(1)' });
    expect(container.querySelector('ruby')).not.toBeInTheDocument();
    expect(container.querySelector('rt')).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: '採点' })).toHaveTextContent('採点');

    fireEvent.click(screen.getByRole('button', { name: 'TOPに戻る' }));
    expect(screen.getByRole('checkbox', { name: 'ふりがな' })).not.toBeChecked();
    expect(container.querySelector('ruby')).not.toBeInTheDocument();
  });

  it('shows furigana on worksheet title and instruction when enabled', async () => {
    const theme = findImplementedThemeByNumericId(SIGNED_ARITHMETIC_1_DEFINITION.numeric_theme_id)!;
    const { container } = render(
      <AutoDrillApp
        engine={numericThemeFixtureEngine(SIGNED_ARITHMETIC_1_DEFINITION)}
        initialWebSettings={createWebDrillSettings(theme, 3, 'fixtureSeed')}
      />,
    );
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    const title = await screen.findByRole('heading', { name: '正負の数の加法・減法' });
    const instruction = container.querySelector('.worksheet-instruction');
    expect(title.querySelector('ruby')).not.toBeNull();
    expect(instruction?.querySelector('ruby')).not.toBeNull();
  });

  it('restores the saved furigana preference after remount', async () => {
    const first = render(<AutoDrillApp engine={fixtureEngine()} />);
    fireEvent.click(screen.getByRole('checkbox', { name: 'ふりがな' }));
    expect(window.localStorage.getItem('autodrill:furigana-enabled')).toBe('false');
    first.unmount();

    const second = render(<AutoDrillApp engine={fixtureEngine()} />);
    await waitFor(() => expect(screen.getByRole('checkbox', { name: 'ふりがな' })).not.toBeChecked());
    expect(second.container.querySelector('ruby')).not.toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'まいんドリル' })).toBeInTheDocument();
  });

  it('keeps Recommended and grade CustomSelect semantics operable', async () => {
    render(<AutoDrillApp engine={fixtureEngine()} />);

    expect(screen.getByRole('button', { name: 'おすすめ' })).toHaveAttribute('aria-pressed', 'true');
    expect(screen.queryByRole('combobox', { name: '学年' })).not.toBeInTheDocument();
    expect(screen.getByRole('combobox', { name: 'ジャンル' })).toHaveAttribute('data-selected-label', '足し算と引き算');
    expect(screen.getByRole('combobox', { name: 'テーマ' })).toHaveAttribute('data-selected-label', '一桁の足し算（まとめ）');
    expect(screen.getByRole('combobox', { name: '難易度' })).toHaveAttribute('data-selected-label', 'ふつう');
    fireEvent.click(screen.getByRole('combobox', { name: '難易度' }));
    expect(within(screen.getByRole('listbox', { name: '難易度の選択肢' })).getAllByRole('option').map((option) => option.getAttribute('aria-label'))).toEqual(['かんたん', 'ふつう', 'むずかしい', 'ランダム']);
    fireEvent.keyDown(screen.getByRole('combobox', { name: '難易度' }), { key: 'Escape' });

    fireEvent.click(screen.getByRole('combobox', { name: 'ジャンル' }));
    const recommendedOptions = within(screen.getByRole('listbox', { name: 'ジャンルの選択肢' })).getAllByRole('option');
    expect(recommendedOptions.map((option) => option.getAttribute('aria-label'))).toEqual(['足し算と引き算', '掛け算と割り算', '小数', '分数', '負の数', '文字式', '方程式', 'おまけ']);
    fireEvent.click(screen.getByRole('option', { name: '方程式' }));
    expect(screen.getByRole('combobox', { name: 'ジャンル' })).toHaveAttribute('data-selected-label', '方程式');
    expect(screen.getByRole('combobox', { name: 'テーマ' })).toHaveAttribute('data-selected-label', '簡単な一次方程式');

    fireEvent.click(screen.getByRole('button', { name: '学年から選ぶ' }));
    expect(screen.getByRole('combobox', { name: '学年' })).toHaveAttribute('data-value', 'grade-7');
    expect(screen.getByRole('combobox', { name: '学年' })).toHaveAttribute('data-selected-label', '中学1年生');
    expect(screen.getByRole('combobox', { name: '単元' })).toHaveAttribute('data-selected-label', '一次方程式');
    expect(screen.queryByRole('combobox', { name: 'ジャンル' })).not.toBeInTheDocument();
    expect(screen.queryByRole('combobox', { name: 'テーマ' })).not.toBeInTheDocument();
    const equationGroup = screen.getByRole('group', { name: '一次方程式の教材' });
    expect(within(equationGroup).getAllByRole('button')).toHaveLength(4);
    expect(within(equationGroup).getByRole('button', { name: '簡単な一次方程式' })).toHaveAttribute('aria-pressed', 'true');
    expect(within(equationGroup).getByRole('button', { name: '一次方程式(1)：基本形' })).toHaveAttribute('aria-pressed', 'false');
    expect(within(equationGroup).getByRole('button', { name: '一次方程式(2)：括弧・整数係数中心' })).toHaveAttribute('aria-pressed', 'false');
    expect(within(equationGroup).getByRole('button', { name: '一次方程式(3)：括弧・分数・小数係数' })).toHaveAttribute('aria-pressed', 'false');
    fireEvent.click(screen.getByRole('combobox', { name: '学年' }));
    expect(within(screen.getByRole('listbox', { name: '学年の選択肢' })).getAllByRole('option')).toHaveLength(9);
    fireEvent.keyDown(screen.getByRole('combobox', { name: '学年' }), { key: 'Escape' });
    expect(screen.queryByRole('listbox', { name: '学年の選択肢' })).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    await screen.findByRole('heading', { name: '1けたのたしざん(1)' });
  });

  it('shows all ten multiplication-table siblings as direct curriculum tiles', () => {
    render(<AutoDrillApp engine={fixtureEngine()} />);
    fireEvent.click(screen.getByRole('button', { name: '学年から選ぶ' }));
    fireEvent.click(screen.getByRole('combobox', { name: '学年' }));
    fireEvent.click(screen.getByRole('option', { name: '小学2年生' }));
    fireEvent.click(screen.getByRole('combobox', { name: '単元' }));
    fireEvent.click(screen.getByRole('option', { name: '九九' }));

    const group = screen.getByRole('group', { name: '九九の教材' });
    const tiles = within(group).getAllByRole('button');
    expect(tiles.map((button) => button.textContent)).toEqual([
      '全段混合', '1の段', '2の段', '3の段', '4の段', '5の段',
      '6の段', '7の段', '8の段', '9の段',
    ]);
    expect(screen.getByRole('button', { name: '全段混合' })).toHaveAttribute('aria-pressed', 'true');
    const sevenTile = screen.getByRole('button', { name: '7の段' });
    sevenTile.focus();
    expect(sevenTile).toHaveFocus();
    fireEvent.click(sevenTile);
    expect(sevenTile).toHaveAttribute('aria-pressed', 'true');
    expect(screen.getByRole('button', { name: '全段混合' })).toHaveAttribute('aria-pressed', 'false');
  });

  it('shows dedicated two-digit regrouping themes as sibling curriculum tiles', () => {
    render(<AutoDrillApp engine={fixtureEngine()} />);
    fireEvent.click(screen.getByRole('button', { name: '学年から選ぶ' }));
    fireEvent.click(screen.getByRole('combobox', { name: '学年' }));
    fireEvent.click(screen.getByRole('option', { name: '小学2年生' }));
    fireEvent.click(screen.getByRole('combobox', { name: '単元' }));
    fireEvent.click(screen.getByRole('option', { name: '加法，減法' }));

    const group = screen.getByRole('group', { name: '加法，減法の教材' });
    expect(within(group).getAllByRole('button').map((button) => button.getAttribute('aria-label'))).toEqual([
      '二桁の足し算の筆算（まとめ）',
      '二桁の引き算の筆算（まとめ）',
      '足し算・繰り上がりなし',
      '足し算・繰り上がりあり',
      '引き算・繰り下がりなし',
      '引き算・繰り下がりあり',
    ]);
    const carry = within(group).getByRole('button', { name: '足し算・繰り上がりあり' });
    fireEvent.click(carry);
    expect(carry).toHaveAttribute('aria-pressed', 'true');
  });

  it('shows decimal addition, subtraction, and summary as sibling curriculum tiles', () => {
    render(<AutoDrillApp engine={fixtureEngine()} />);
    fireEvent.click(screen.getByRole('button', { name: '学年から選ぶ' }));
    fireEvent.click(screen.getByRole('combobox', { name: '学年' }));
    fireEvent.click(screen.getByRole('option', { name: '小学4年生' }));
    fireEvent.click(screen.getByRole('combobox', { name: '単元' }));
    fireEvent.click(screen.getByRole('option', { name: '小数の仕組みとその計算' }));

    const group = screen.getByRole('group', { name: '小数の仕組みとその計算の教材' });
    expect(within(group).getAllByRole('button').map((button) => button.getAttribute('aria-label'))).toEqual([
      '小数の足し算と引き算（まとめ）',
      '小数の足し算の筆算',
      '小数の引き算の筆算',
      '小数と整数の掛け算の筆算',
      '小数と整数の割り算の筆算',
    ]);
    const subtraction = within(group).getByRole('button', { name: '小数の引き算の筆算' });
    fireEvent.click(subtraction);
    expect(subtraction).toHaveAttribute('aria-pressed', 'true');
  });

  it('shows exact, remainder, and rounded decimal division as sibling curriculum tiles', () => {
    render(<AutoDrillApp engine={fixtureEngine()} />);
    fireEvent.click(screen.getByRole('button', { name: '学年から選ぶ' }));
    fireEvent.click(screen.getByRole('combobox', { name: '学年' }));
    fireEvent.click(screen.getByRole('option', { name: '小学5年生' }));
    fireEvent.click(screen.getByRole('combobox', { name: '単元' }));
    fireEvent.click(screen.getByRole('option', { name: '小数の乗法，除法' }));

    const group = screen.getByRole('group', { name: '小数の乗法，除法の教材' });
    expect(within(group).getAllByRole('button').map((button) => button.getAttribute('aria-label'))).toEqual([
      '小数の掛け算の筆算',
      '小数の割り算の筆算',
      '余りを答える小数の割り算の筆算',
      '商を四捨五入する小数の割り算の筆算',
    ]);
    const rounded = within(group).getByRole('button', { name: '商を四捨五入する小数の割り算の筆算' });
    fireEvent.click(rounded);
    expect(rounded).toHaveAttribute('aria-pressed', 'true');
  });

  it('shows the print recommendation only for themes with the print_recommended presentation capability', () => {
    render(<AutoDrillApp engine={fixtureEngine()} />);
    const note = 'この問題は紙に印刷して解くことをおすすめします。';
    expect(screen.queryByRole('note', { name: note })).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole('combobox', { name: 'テーマ' }));
    fireEvent.click(screen.getByRole('option', { name: '二桁の足し算の筆算（まとめ）' }));
    expect(screen.getByRole('note', { name: note })).toHaveClass('print-recommended-note');

    fireEvent.click(screen.getByRole('combobox', { name: 'テーマ' }));
    fireEvent.click(screen.getByRole('option', { name: '一桁の足し算（まとめ）' }));
    expect(screen.queryByRole('note', { name: note })).not.toBeInTheDocument();
  });

  it('shows the color-coded grade tag in the selected Recommended theme and before option checkmarks', () => {
    render(<AutoDrillApp engine={fixtureEngine()} />);
    const themeSelect = screen.getByRole('combobox', { name: 'テーマ' });
    expect(themeSelect.querySelector('.grade-tag')).toHaveTextContent('小1');
    expect(themeSelect.querySelector('.grade-tag')).toHaveClass('grade-tag-grade-1');
    expect(themeSelect.querySelector('.grade-tag ruby')).toBeNull();
    fireEvent.click(themeSelect);
    const grade1 = screen.getByRole('option', { name: '一桁の足し算（まとめ）' });
    expect(grade1.querySelector('.grade-tag')).toHaveTextContent('小1');
    expect(grade1.querySelector('.grade-tag')).toHaveClass('grade-tag-grade-1');
    expect(grade1.querySelector('.grade-tag ruby')).toBeNull();
    expect(screen.getByRole('option', { name: '二桁の足し算' }).querySelector('.grade-tag')).toHaveClass('grade-tag-grade-2');
    const end = grade1.querySelector('.custom-select-option-end');
    expect(end?.children[0]).toHaveClass('grade-tag');
    expect(end?.children[1]).toHaveClass('custom-select-check');
    fireEvent.keyDown(screen.getByRole('combobox', { name: 'テーマ' }), { key: 'Escape' });

    fireEvent.click(screen.getByRole('combobox', { name: 'ジャンル' }));
    fireEvent.click(screen.getByRole('option', { name: '掛け算と割り算' }));
    fireEvent.click(screen.getByRole('combobox', { name: 'テーマ' }));
    expect(screen.getByRole('option', { name: 'あまりのない割り算' }).querySelector('.grade-tag')).toHaveTextContent('小3');
    expect(screen.getByRole('option', { name: 'あまりのない割り算' }).querySelector('.grade-tag')).toHaveClass('grade-tag-grade-3');
    fireEvent.keyDown(screen.getByRole('combobox', { name: 'テーマ' }), { key: 'Escape' });

    fireEvent.click(screen.getByRole('combobox', { name: 'ジャンル' }));
    fireEvent.click(screen.getByRole('option', { name: '小数' }));
    expect(screen.getByRole('combobox', { name: 'テーマ' }).querySelector('.grade-tag')).toHaveTextContent('小4');
    expect(screen.getByRole('combobox', { name: 'テーマ' }).querySelector('.grade-tag')).toHaveClass('grade-tag-grade-4');
    fireEvent.click(screen.getByRole('combobox', { name: 'テーマ' }));
    expect(screen.getByRole('option', { name: '小数の足し算と引き算' }).querySelector('.grade-tag')).toHaveClass('grade-tag-grade-4');
    fireEvent.keyDown(screen.getByRole('combobox', { name: 'テーマ' }), { key: 'Escape' });

    fireEvent.click(screen.getByRole('combobox', { name: 'ジャンル' }));
    fireEvent.click(screen.getByRole('option', { name: '分数' }));
    fireEvent.click(screen.getByRole('combobox', { name: 'テーマ' }));
    expect(screen.getByRole('option', { name: '同分母の分数の足し算' }).querySelector('.grade-tag')).toHaveTextContent('小4');
    expect(screen.getByRole('option', { name: '同分母の分数の足し算' }).querySelector('.grade-tag')).toHaveClass('grade-tag-grade-4');
    expect(screen.getByRole('option', { name: '異分母の分数の足し算' }).querySelector('.grade-tag')).toHaveClass('grade-tag-grade-5');
    expect(screen.getByRole('option', { name: '分数の足し算（まとめ）' }).querySelector('.grade-tag')).toHaveClass('grade-tag-grade-5');
    expect(screen.getByRole('option', { name: '分数の掛け算' }).querySelector('.grade-tag')).toHaveClass('grade-tag-grade-6');
    fireEvent.keyDown(screen.getByRole('combobox', { name: 'テーマ' }), { key: 'Escape' });

    fireEvent.click(screen.getByRole('combobox', { name: 'ジャンル' }));
    fireEvent.click(screen.getByRole('option', { name: '方程式' }));
    fireEvent.click(screen.getByRole('combobox', { name: 'テーマ' }));
    const expected = [
      ['簡単な一次方程式', '中1', 'grade-tag-grade-7'],
      ['連立方程式（加減法）', '中2', 'grade-tag-grade-8'],
      ['二次方程式(1)', '中3', 'grade-tag-grade-9'],
    ] as const;
    for (const [label, tagText, className] of expected) {
      const option = screen.getByRole('option', { name: label });
      expect(option.querySelector('.grade-tag')).toHaveTextContent(tagText);
      expect(option.querySelector('.grade-tag')).toHaveClass(className);
    }
  });

  it('uses the implemented simultaneous-equation theme for grade 8', () => {
    render(<AutoDrillApp engine={fixtureEngine()} />);
    fireEvent.click(screen.getByRole('button', { name: '学年から選ぶ' }));
    fireEvent.click(screen.getByRole('combobox', { name: '学年' }));
    fireEvent.click(screen.getByRole('option', { name: '中学2年生' }));

    expect(screen.getByRole('combobox', { name: '学年' })).toHaveAttribute('data-selected-label', '中学2年生');
    expect(screen.getByRole('combobox', { name: '単元' })).toHaveAttribute('data-selected-label', '連立方程式');
    const simultaneousTiles = screen.getByRole('group', { name: '連立方程式の教材' });
    for (const label of ['連立方程式（加減法）', '連立方程式（代入法）', '連立方程式（まとめ(1)）', '連立方程式（まとめ(2)）']) {
      expect(within(simultaneousTiles).getByRole('button', { name: label })).toBeInTheDocument();
    }
    expect(screen.getByRole('button', { name: '問題生成' })).toBeEnabled();
    expect(screen.getByRole('button', { name: '印刷 (pdfで出力)' })).toBeEnabled();
  });

  it('keeps long-division quotient and remainder separate through focus transition and grading', async () => {
    const worksheet = columnDivisionFixtureWorksheet();
    const base = fixtureEngine(worksheet);
    const gradeAnswer = vi.fn(base.gradeAnswer);
    render(
      <AutoDrillApp
        engine={{ ...base, gradeAnswer }}
        initialWebSettings={createWebDrillSettings(findImplementedThemeByNumericId(COLUMN_DIVIDE_2DIGIT_BY_1DIGIT_DEFINITION.numeric_theme_id)!, 3, 'fixtureSeed')}
      />,
    );
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    const firstProblem = await screen.findByTestId('problem-cell-0');
    const quotientDigits = within(firstProblem).getAllByRole('button', { name: /^1番の商 / });
    expect(quotientDigits).toHaveLength(2);

    // All dividend-aligned quotient positions are offered so the UI does not
    // reveal that this particular quotient starts in the second cell.
    fireEvent.click(quotientDigits[0]!);
    fireEvent.keyDown(window, { key: '1' });
    fireEvent.keyDown(window, { key: '2' });

    const emptyRemainder = await within(firstProblem).findByRole('textbox', { name: '1番のあまり 未入力' });
    await waitFor(() => expect(answerFrame(emptyRemainder)).toHaveClass('answer-box-selected'));
    pressKey('2');
    pressKey('1');

    const remainder = await within(firstProblem).findByRole('textbox', { name: '1番のあまり 21' }) as HTMLElement & { value: string };
    expect(remainder.value).toBe('21');
    expect(screen.queryByLabelText('式が大きすぎます！')).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: '採点' }));
    await waitFor(() => expect(gradeAnswer).toHaveBeenCalledTimes(1));
    const submitted = gradeAnswer.mock.calls[0]![0].answers.find((entry) => entry.problem_id === '1')?.answer;
    expect(submitted).toEqual({
      type: 'tuple',
      value: [{ type: 'integer', value: '12' }, { type: 'integer', value: '21' }],
    });
    const gradedRemainder = within(firstProblem).getByRole('textbox', { name: '1番のあまり 21' }) as HTMLElement & { value: string };
    expect(gradedRemainder.value).toBe('21');
    expect(within(firstProblem).getByLabelText('1番の商 十の位 1')).toBeInTheDocument();
    expect(within(firstProblem).getByLabelText('1番の商 一の位 2')).toBeInTheDocument();
  });

  it('lets decimal multiplication place and reposition the decimal point without exposing the canonical location', async () => {
    const worksheet = columnDecimalMultiplicationFixtureWorksheet();
    const base = fixtureEngine(worksheet);
    const gradeAnswer = vi.fn(base.gradeAnswer);
    render(
      <AutoDrillApp
        engine={{ ...base, gradeAnswer }}
        initialWebSettings={createWebDrillSettings(findImplementedThemeByNumericId(COLUMN_DECIMAL_MULTIPLICATION_DEFINITION.numeric_theme_id)!, 3, 'fixtureSeed')}
      />,
    );
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    const firstProblem = await screen.findByTestId('problem-cell-0');
    const answerEditor = firstProblem.querySelector('.column-digit-answer-single')!;
    expect(answerEditor.querySelector('.column-digit-decimal-marker')).toBeNull();

    const digitSlots = within(firstProblem).getAllByRole('button', { name: /^1番の答え 解答欄/ });
    fireEvent.click(digitSlots.at(-1)!);
    fireEvent.keyDown(window, { key: '6' });
    fireEvent.keyDown(window, { key: '3' });
    expect(answerEditor.querySelector('.column-digit-decimal-marker')).toBeNull();

    fireEvent.click(screen.getByRole('button', { name: '小数点' }));
    const firstMarker = answerEditor.querySelector<HTMLElement>('.column-digit-decimal-marker');
    expect(firstMarker).not.toBeNull();
    const firstLeft = firstMarker!.style.left;

    fireEvent.keyDown(window, { key: 'ArrowRight' });
    fireEvent.keyDown(window, { key: '.' });
    const movedMarker = answerEditor.querySelector<HTMLElement>('.column-digit-decimal-marker');
    expect(movedMarker).not.toBeNull();
    expect(movedMarker!.style.left).not.toBe(firstLeft);

    fireEvent.keyDown(window, { key: 'ArrowLeft' });
    fireEvent.keyDown(window, { key: '.' });
    expect(answerEditor.querySelector<HTMLElement>('.column-digit-decimal-marker')!.style.left).toBe(firstLeft);

    fireEvent.click(screen.getByRole('button', { name: '採点' }));
    await waitFor(() => expect(gradeAnswer).toHaveBeenCalledTimes(1));
    const submitted = gradeAnswer.mock.calls[0]![0].answers.find((entry) => entry.problem_id === '1')?.answer;
    expect(submitted).toEqual({ type: 'exact_decimal', value: { coefficient: '36', scale: 2 } });
  });

  it('edits the fixed 4x4 digit grid through the shared numeric keypad', async () => {
    const worksheet = miniSudokuFixtureWorksheet();
    render(
      <AutoDrillApp
        engine={fixtureEngine(worksheet)}
        initialWebSettings={createWebDrillSettings(findImplementedThemeByNumericId(MINI_SUDOKU_DEFINITION.numeric_theme_id)!, 3, 'fixtureSeed')}
      />,
    );
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    const firstProblem = await screen.findByTestId('problem-cell-0');
    expect([
      '--mini-sudoku-grid-left',
      '--mini-sudoku-grid-top',
      '--mini-sudoku-number-left',
      '--mini-sudoku-number-top',
    ].every((property) => firstProblem.style.getPropertyValue(property).endsWith('cqw'))).toBe(true);
    const firstEditable = within(firstProblem).getByRole('button', { name: '2番目のマス 未入力' });
    fireEvent.click(firstEditable);
    const inputPanel = screen.getByLabelText('数式入力パネル');
    expect(within(inputPanel).queryByRole('button', { name: '5' })).toBeNull();
    expect(within(inputPanel).getByLabelText('数字キー')).toHaveClass('keypad-numbers-digit-grid');
    fireEvent.click(within(inputPanel).getByRole('button', { name: '2' }));
    expect(firstProblem.querySelector('[data-digit-grid-cell="1"] .digit-grid-cell-value')?.textContent).toBe('2');
    expect(firstProblem.querySelectorAll('[data-digit-grid-cell]')).toHaveLength(16);
  });

  it('maps every physical digit-grid editing key to the shared command semantics', async () => {
    const worksheet = miniSudokuFixtureWorksheet();
    render(
      <AutoDrillApp
        engine={fixtureEngine(worksheet)}
        initialWebSettings={createWebDrillSettings(findImplementedThemeByNumericId(MINI_SUDOKU_DEFINITION.numeric_theme_id)!, 3, 'fixtureSeed')}
      />,
    );
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    const firstProblem = await screen.findByTestId('problem-cell-0');
    const secondProblem = await screen.findByTestId('problem-cell-1');
    const editable = within(firstProblem).getAllByRole('button', { name: /番目のマス/ });
    const first = editable[0]!;
    const second = editable[1]!;
    fireEvent.click(first);

    fireEvent.keyDown(window, { key: 'ArrowRight' });
    expect(second).toHaveAttribute('aria-pressed', 'true');
    fireEvent.keyDown(window, { key: 'ArrowLeft' });
    expect(first).toHaveAttribute('aria-pressed', 'true');

    fireEvent.keyDown(window, { key: '2' });
    expect(first.querySelector('.digit-grid-cell-value')).toHaveTextContent('2');
    expect(second).toHaveAttribute('aria-pressed', 'true');
    fireEvent.keyDown(window, { key: 'ArrowLeft' });
    fireEvent.keyDown(window, { key: 'Delete' });
    expect(first.querySelector('.digit-grid-cell-value')).toHaveTextContent('');

    fireEvent.keyDown(window, { key: '3' });
    fireEvent.keyDown(window, { key: 'ArrowLeft' });
    expect(first.querySelector('.digit-grid-cell-value')).toHaveTextContent('3');
    fireEvent.keyDown(window, { key: 'Backspace' });
    expect(first.querySelector('.digit-grid-cell-value')).toHaveTextContent('');

    fireEvent.keyDown(window, { key: '4' });
    fireEvent.keyDown(window, { key: 'Enter' });
    expect(secondProblem.querySelector('.digit-grid-cell-selected')).not.toBeNull();
  });

  it('shows Mini Sudoku corrections with the common wrong-problem mark above its number', async () => {
    const worksheet = miniSudokuFixtureWorksheet();
    render(
      <AutoDrillApp
        engine={fixtureEngine(worksheet)}
        initialWebSettings={createWebDrillSettings(findImplementedThemeByNumericId(MINI_SUDOKU_DEFINITION.numeric_theme_id)!, 3, 'fixtureSeed')}
      />,
    );
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    await screen.findByTestId('problem-cell-0');
    fireEvent.click(screen.getByRole('button', { name: '採点' }));
    await waitFor(() => expect(screen.getByRole('status')).toHaveTextContent('0 / 4'));
    expect(document.querySelectorAll('.digit-grid-cell-correction').length).toBeGreaterThan(0);
    const firstProblem = screen.getByTestId('problem-cell-0');
    const wrongMark = within(firstProblem).getByLabelText('不正解');
    expect(wrongMark).toHaveTextContent('✓');
    expect(wrongMark).toHaveClass('problem-grade-mark-wrong');
    expect(wrongMark.parentElement).toHaveClass('problem-number-stack');
  });

  it('renders simultaneous equations with separate x and y answer boxes and 12 problems', async () => {
    const worksheet = simultaneousFixtureWorksheet();
    render(<AutoDrillApp engine={fixtureEngine(worksheet)} />);
    fireEvent.click(screen.getByRole('button', { name: '学年から選ぶ' }));
    fireEvent.click(screen.getByRole('combobox', { name: '学年' }));
    fireEvent.click(screen.getByRole('option', { name: '中学2年生' }));
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));

    await screen.findByRole('heading', { name: '連立方程式（加減法）' });
    expect(document.querySelectorAll('[data-problem-index]')).toHaveLength(12);
    expect(screen.getByRole('textbox', { name: '1番のxの答え 未入力' })).toBeInTheDocument();
    expect(screen.getByRole('textbox', { name: '1番のyの答え 未入力' })).toBeInTheDocument();
    expect(screen.queryByText('(x, y) =')).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole('textbox', { name: '1番のxの答え 未入力' }));
    const formulaPad = screen.getByLabelText('数式キー');
    const formulaButtons = within(formulaPad).getAllByRole('button');
    expect(formulaButtons.map((button) => button.getAttribute('aria-label'))).toEqual(['分数', '帯分数', '平方根', 'x, y']);
    expect(formulaButtons.every((button) => !button.hasAttribute('disabled'))).toBe(true);
    const operators = screen.getByLabelText('演算子キー');
    expect(within(operators).getAllByRole('button').every((button) => !button.hasAttribute('disabled'))).toBe(true);
  });

  it('renders liar puzzles as six questions with oval person selection instead of math input', async () => {
    const worksheet = liarFixtureWorksheet();
    render(<AutoDrillApp engine={fixtureEngine(worksheet)} />);
    fireEvent.click(screen.getByRole('combobox', { name: 'ジャンル' }));
    fireEvent.click(screen.getByRole('option', { name: 'おまけ' }));
    expect(screen.getByLabelText('問題数6問')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('combobox', { name: 'テーマ' }));
    fireEvent.click(screen.getByRole('option', { name: 'すうじはひとりぼっち' }));
    expect(screen.getByLabelText('問題数4問')).toBeInTheDocument();
    expect(screen.getByRole('combobox', { name: 'テーマ' })).toHaveAttribute('data-value', expect.stringContaining('mini_sudoku'));

    fireEvent.click(screen.getByRole('combobox', { name: 'テーマ' }));
    fireEvent.click(screen.getByRole('option', { name: 'うそつきだれだ' }));
    expect(screen.getByLabelText('問題数6問')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));

    await screen.findByRole('heading', { name: 'うそつきだれだ' });
    expect(document.querySelectorAll('[data-problem-index]')).toHaveLength(6);
    const first = screen.getByTestId('problem-cell-0');
    expect(first).toHaveTextContent('このなかの2人がうそつきだ。');
    expect(first).toHaveTextContent('Cさんはうそつきではない。');
    const a = within(first).getByRole('button', { name: 'Aさん' });
    const c = within(first).getByRole('button', { name: 'Cさん' });
    expect(a).toHaveAttribute('aria-pressed', 'false');
    fireEvent.click(a);
    expect(a).toHaveAttribute('aria-pressed', 'true');
    expect(a).toHaveClass('liar-person-choice-selected');
    fireEvent.click(c);
    expect(c).toHaveAttribute('aria-pressed', 'true');
    fireEvent.click(a);
    expect(a).toHaveAttribute('aria-pressed', 'false');
    expect(a).not.toHaveClass('liar-person-choice-selected');
    expect(screen.queryByLabelText('数式入力パネル')).not.toBeInTheDocument();
    expect(within(first).queryByRole('textbox')).not.toBeInTheDocument();
  });
  it('keeps custom-select hit targets inside their visible trigger boxes', () => {
    const { container } = render(<AutoDrillApp engine={fixtureEngine()} />);
    expect(container.querySelector('label[for="difficulty-select"]')).toBeNull();
    expect(container.querySelector('label[for="genre-select"]')).toBeNull();
    expect(container.querySelector('label[for="theme-select"]')).toBeNull();

    fireEvent.click(screen.getByRole('button', { name: '学年から選ぶ' }));
    expect(container.querySelector('label[for="grade-select"]')).toBeNull();
  });

  it('stores difficulty changes through the custom difficulty dropdown', async () => {
    const onWebSettingsChange = vi.fn();
    render(<AutoDrillApp engine={fixtureEngine()} onWebSettingsChange={onWebSettingsChange} />);
    expect(screen.getByRole('combobox', { name: '難易度' })).toHaveAttribute('data-selected-label', 'ふつう');

    fireEvent.click(screen.getByRole('combobox', { name: '難易度' }));
    fireEvent.click(screen.getByRole('option', { name: 'ランダム' }));
    expect(screen.getByRole('combobox', { name: '難易度' })).toHaveAttribute('data-selected-label', 'ランダム');
    await waitFor(() => expect(onWebSettingsChange).toHaveBeenLastCalledWith({
      schema_version: DRILL_SCHEMA_VERSION,
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
        initialWebSettings={createWebDrillSettings(ONE_DIGIT_ADDITION_THEME, 3)}
      />,
    );
    expect(screen.getByRole('combobox', { name: 'ジャンル' })).toHaveAttribute('data-selected-label', '足し算と引き算');
    expect(screen.getByRole('combobox', { name: 'テーマ' })).toHaveAttribute('data-selected-label', '一桁の足し算（まとめ）');
    expect(screen.getByRole('combobox', { name: '難易度' })).toHaveAttribute('data-selected-label', 'むずかしい');
  });

  it('disables q1 actions and announces problem generation while it is pending', async () => {
    const { engine, resolveGeneration } = deferredGenerationEngine();
    render(<AutoDrillApp engine={engine} />);

    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    expect(screen.getByRole('button', { name: '問題を生成中…' })).toBeDisabled();
    expect(screen.getByRole('button', { name: '印刷 (pdfで出力)' })).toBeDisabled();
    expect(screen.getByLabelText('問題を生成しています。しばらくお待ちください。')).toBeInTheDocument();

    resolveGeneration(fixtureWorksheet());
    expect(await screen.findByRole('heading', { name: '1けたのたしざん(1)' })).toBeInTheDocument();
  });

  it('disables q1 actions and announces PDF preparation while it is pending', async () => {
    const { engine, resolveGeneration } = deferredGenerationEngine();
    render(<AutoDrillApp engine={engine} />);

    fireEvent.click(screen.getByRole('button', { name: '印刷 (pdfで出力)' }));
    expect(screen.getByRole('button', { name: 'PDFを準備中…' })).toBeDisabled();
    expect(screen.getByRole('button', { name: '問題生成' })).toBeDisabled();
    expect(screen.getByLabelText('印刷用PDFを準備しています。しばらくお待ちください。')).toBeInTheDocument();

    resolveGeneration(fixtureWorksheet());
    await waitFor(() => expect(screen.getByRole('button', { name: '印刷 (pdfで出力)' })).toBeEnabled());
  });

  it('transitions from q1 generation to the q2 worksheet', async () => {
    render(<AutoDrillApp engine={fixtureEngine()} />);
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    expect(await screen.findByRole('heading', { name: '1けたのたしざん(1)' })).toBeInTheDocument();
    expect(screen.getByLabelText('20問の1けたのたしざん(1)ワークシート')).toBeInTheDocument();
    expect(screen.queryByLabelText('数式入力パネル')).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole('textbox', { name: /^1番の答え/ }));
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
    expect(await screen.findByRole('heading', { name: '一次方程式(1)：基本形' })).toBeInTheDocument();
    expect(screen.getByLabelText('16問の一次方程式(1)：基本形ワークシート')).toBeInTheDocument();
    expect(screen.getByLabelText('次の一次方程式を解きなさい。ただし、答えが整数でない場合は約分によって最も簡単な形の仮分数で答えなさい。')).toBeInTheDocument();

    const firstCell = screen.getByTestId('problem-cell-0');
    expect(firstCell).toHaveClass('problem-cell-linear-equation');
    expect(firstCell.querySelector('.problem-math-expression')).toHaveAttribute('aria-label', '2x = x − 5');
    expect(within(firstCell).getByLabelText('x =')).toBeTruthy();

    fireEvent.click(within(firstCell).getByRole('textbox', { name: /^1番の答え/ }));
    const formulaPad = screen.getByLabelText('数式キー');
    expect(within(formulaPad).getAllByRole('button').map((button) => button.getAttribute('aria-label'))).toEqual([
      '分数', '帯分数', '平方根', '複数解',
    ]);
    expect(within(formulaPad).getAllByRole('button').every((button) => !button.hasAttribute('disabled'))).toBe(true);
    expect(within(screen.getByLabelText('数字キー')).getByRole('button', { name: '小数点' })).toBeEnabled();
    const operators = screen.getByLabelText('演算子キー');
    expect(within(operators).getAllByRole('button').map((button) => button.textContent)).toEqual(['+', '−', '±']);
    expect(within(operators).getAllByRole('button').every((button) => !button.hasAttribute('disabled'))).toBe(true);
    expect(within(screen.getByLabelText('編集キー')).getAllByRole('button').slice(0, 2).map((button) => button.textContent)).toEqual(['←', '→']);
  });

  it('uses the dedicated symbolic-expression keypad and forwards variable input to Rust parsing', async () => {
    const worksheet = linearExpressionFixtureWorksheet();
    const base = fixtureEngine(worksheet);
    const parseMathLiveAnswer = vi.fn(async (latex: string) => ({ type: 'nan_error' as const, value: latex }));
    const symbolicTheme = findImplementedThemeByNumericId(75)!;
    render(
      <AutoDrillApp
        engine={{ ...base, parseMathLiveAnswer }}
        initialWebSettings={createWebDrillSettings(symbolicTheme)}
      />,
    );

    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    expect(await screen.findByRole('heading', { name: '一次式の整理・加減' })).toBeInTheDocument();
    expect(screen.getByLabelText('次の式を簡単にしなさい。')).toBeInTheDocument();

    const firstCell = screen.getByTestId('problem-cell-0');
    expect(firstCell).toHaveClass('problem-cell-answer-below');
    expect(firstCell.querySelector('.problem-math-expression')).toHaveAttribute('aria-label', '(2x + 1) + (3x + 2)');
    fireEvent.click(within(firstCell).getByRole('textbox', { name: /^1番の答え/ }));

    const formulaPad = screen.getByLabelText('数式キー');
    expect(within(formulaPad).getAllByRole('button').map((button) => button.getAttribute('aria-label'))).toEqual(['文字 x']);
    expect(within(screen.getByLabelText('数字キー')).queryByRole('button', { name: '小数点' })).not.toBeInTheDocument();
    expect(within(screen.getByLabelText('演算子キー')).getAllByRole('button').map((button) => button.textContent)).toEqual(['+', '−']);

    fireEvent.click(within(screen.getByLabelText('数字キー')).getByRole('button', { name: '5' }));
    fireEvent.click(within(formulaPad).getByRole('button', { name: '文字 x' }));
    fireEvent.click(within(screen.getByLabelText('演算子キー')).getByRole('button', { name: 'プラスを挿入' }));
    fireEvent.click(within(screen.getByLabelText('数字キー')).getByRole('button', { name: '3' }));

    await waitFor(() => expect(parseMathLiveAnswer).toHaveBeenLastCalledWith(
      '5x+3',
      { type: 'structured_math', allowed_structures: ['negative', 'arithmetic', 'variable'] },
    ));
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
    expect(screen.getByTestId('problem-divider').style.left).toBe(percent(layout.dividerXs[0]!, A4_PAGE.width));
  });

  it('resets the selected editor on TOP and on each regenerated worksheet', async () => {
    render(<AutoDrillApp engine={fixtureEngine()} />);
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    await screen.findByRole('heading', { name: '1けたのたしざん(1)' });
    fireEvent.click(screen.getByRole('textbox', { name: /^1番の答え/ }));
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
    const first = screen.getByRole('textbox', { name: /^1番の答え/ });
    fireEvent.click(first);
    fireEvent.click(screen.getByRole('button', { name: '1' }));
    await waitFor(() => expect(screen.getByRole('textbox', { name: /1番の答え 1/ })).toBeInTheDocument());
    pressKey('Enter');
    await waitFor(() => expect(answerFrame(screen.getByRole('textbox', { name: /^2番の答え/ }))).toHaveClass('answer-box-selected'));
    expect(screen.getByLabelText('数式入力パネル')).toBeInTheDocument();
  });

  it('renders the fixed keypad in standard calculator order with clear controls', async () => {
    render(<AutoDrillApp engine={fixtureEngine()} />);
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    await screen.findByRole('heading', { name: '1けたのたしざん(1)' });
    fireEvent.click(screen.getByRole('textbox', { name: /^1番の答え/ }));

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

  it('projects signed numeric capability through the canonical junior-high keypad', async () => {
    const base = numericThemeFixtureEngine(SIGNED_ARITHMETIC_1_DEFINITION);
    const parseMathLiveAnswer = vi.fn(base.parseMathLiveAnswer);
    render(<AutoDrillApp engine={{ ...base, parseMathLiveAnswer }} />);
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    fireEvent.click(await screen.findByRole('textbox', { name: /^1番の答え/ }));

    expect(screen.getByLabelText('数式キー')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'マイナスを挿入' })).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'マイナスを挿入' }));
    await waitFor(() => expect(parseMathLiveAnswer).toHaveBeenCalled());
    expect(parseMathLiveAnswer.mock.calls.at(-1)?.[0]).toContain('-');
    expect(parseMathLiveAnswer.mock.calls.at(-1)?.[1]).toEqual(SIGNED_ARITHMETIC_1_DEFINITION.editorInputInterface);
  });

  it('shows decimal input but no negative structure for an elementary decimal theme', async () => {
    render(<AutoDrillApp engine={numericThemeFixtureEngine(DECIMAL_ADD_SUBTRACT_DEFINITION)} />);
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    fireEvent.click(await screen.findByRole('textbox', { name: /^1番の答え/ }));

    expect(screen.queryByRole('button', { name: 'マイナス' })).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: '小数点' })).toBeInTheDocument();
  });

  it('keeps malformed MathLive text as nan_error without treating it as a number', async () => {
    render(<AutoDrillApp engine={fixtureEngine()} />);
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    await screen.findByRole('heading', { name: '1けたのたしざん(1)' });
    const field = screen.getByRole('textbox', { name: /^1番の答え/ }) as HTMLElement & {
      setValue(value: string): void;
      value: string;
    };
    fireEvent.click(field);
    field.setValue('1e+');
    fireEvent.input(field);

    const malformed = await screen.findByRole('textbox', { name: '1番の答え 1e+' });
    expect(answerFrame(malformed)).toHaveClass('answer-box-selected');
    expect((malformed as typeof field).value).toBe('1e+');
    fireEvent.click(screen.getByRole('button', { name: '採点' }));
    await waitFor(() => expect(screen.getByRole('status')).toHaveTextContent('0 / 20'));
    expect(answerFrame(screen.getByRole('textbox', { name: '1番の答え 1e+' }))).toHaveClass('answer-box-wrong');
  });

  it('grades a MathLive 11/1 fraction without editor cursor state', async () => {
    const base = structuredFixtureEngine();
    const gradeAnswer = vi.fn(base.gradeAnswer);
    render(<AutoDrillApp engine={{ ...base, gradeAnswer }} />);
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    await screen.findByRole('heading', { name: '一次方程式(1)：基本形' });
    const field = screen.getByRole('textbox', { name: /^1番の答え/ }) as HTMLElement & {
      setValue(value: string): void;
    };
    fireEvent.click(field);
    field.setValue('\\frac{11}{1}');
    fireEvent.input(field);

    await screen.findByRole('textbox', { name: '1番の答え 11/1' });
    fireEvent.click(screen.getByRole('button', { name: '採点' }));
    await waitFor(() => expect(gradeAnswer).toHaveBeenCalledTimes(1));

    expect(gradeAnswer.mock.calls[0]?.[0].answers[0]).toEqual({
      problem_id: '1',
      answer: {
        type: 'fraction',
        value: {
          numerator: { type: 'integer', value: '11' },
          denominator: { type: 'integer', value: '1' },
        },
      },
    });
    expect(screen.queryByText(/editor path.*editable slot/i)).not.toBeInTheDocument();
  });

  it('uses MathLive for palette previews and editable fraction input', async () => {
    const { container } = render(<AutoDrillApp engine={structuredFixtureEngine()} />);
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    await screen.findByRole('heading', { name: '一次方程式(1)：基本形' });
    fireEvent.click(screen.getByRole('textbox', { name: /^1番の答え/ }));

    const templates = screen.getByLabelText('数式キー');
    expect(within(templates).getAllByRole('button').map((button) => button.getAttribute('aria-label'))).toEqual([
      '分数', '帯分数', '平方根', '複数解',
    ]);
    const previewMath = [...templates.querySelectorAll('math-span.math-template-icon')];
    expect(previewMath).toHaveLength(4);
    expect(previewMath.map((node) => node.textContent)).toEqual([
      '\\frac{\\square}{\\square}',
      '\\square\\frac{\\square}{\\square}',
      '\\sqrt{\\square}',
      '\\square,\\square',
    ]);
    expect(previewMath.every((node) => !node.textContent?.includes('\\placeholder'))).toBe(true);
    expect(templates.querySelector('math, mfrac, msqrt, svg')).toBeNull();

    fireEvent.click(within(templates).getByRole('button', { name: '分数' }));
    await waitFor(() => {
      const field = container.querySelector('math-field.answer-mathfield-selected') as (HTMLElement & { value: string }) | null;
      expect(field?.value).toContain('\\frac');
    });
    const selectedField = container.querySelector('math-field.answer-mathfield-selected') as HTMLElement | null;
    expect(selectedField).not.toBeNull();
    expect(answerFrame(selectedField!)).toHaveClass('answer-box', 'answer-box-selected');
    expect(selectedField).not.toHaveClass('answer-box');
  });

  it('delegates square-root rendering and editing entirely to MathLive', async () => {
    const { container } = render(<AutoDrillApp engine={structuredFixtureEngine()} />);
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    await screen.findByRole('heading', { name: '一次方程式(1)：基本形' });
    fireEvent.click(screen.getByRole('textbox', { name: /^1番の答え/ }));
    fireEvent.click(screen.getByRole('button', { name: '平方根' }));

    await waitFor(() => {
      const field = container.querySelector('math-field.answer-mathfield-selected') as (HTMLElement & { value: string }) | null;
      expect(field?.value).toContain('\\sqrt');
    });
    const field = container.querySelector('math-field.answer-mathfield-selected') as HTMLElement & { value: string; placeholderSymbol: string };
    expect(field.placeholderSymbol).toBe('☐');
    expect(field.value).toContain('\\sqrt');
    expect(answerFrame(field)).toHaveClass('answer-box-selected');
    expect(container.querySelector('math, msqrt, svg.answer-root, .answer-radicand')).toBeNull();
  });

  it('deletes the smallest empty MathLive structure through the public range API', () => {
    const fake = {
      position: 2,
      lastOffset: 4,
      selection: { ranges: [[2, 2]], direction: 'none' },
      getValue(range: readonly [number, number]) {
        const [start, end] = range;
        if (start === 1 && end === 2) return '\\placeholder[numerator]{}';
        if (start === 0 && end === 3) return '\\frac{\\placeholder[numerator]{}}{\\placeholder[denominator]{}}';
        return '';
      },
      executeCommand: vi.fn(() => true),
    } as unknown as AutoDrillMathfield;

    expect(deleteEmptyMathLiveStructureBackward(fake)).toBe(true);
    expect(fake.selection).toEqual({ ranges: [[0, 3]], direction: 'none' });
    expect(fake.executeCommand).toHaveBeenCalledWith('deleteBackward');
  });

  it('routes palette structures through MathLive into parseMathLiveAnswer', async () => {
    const base = structuredFixtureEngine();
    const parseMathLiveAnswer = vi.fn(base.parseMathLiveAnswer);
    render(<AutoDrillApp engine={{ ...base, parseMathLiveAnswer }} />);
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    await screen.findByRole('heading', { name: '一次方程式(1)：基本形' });
    fireEvent.click(screen.getByRole('textbox', { name: /^1番の答え/ }));

    fireEvent.click(screen.getByRole('button', { name: '分数' }));
    fireEvent.click(screen.getByRole('button', { name: '平方根' }));
    fireEvent.click(screen.getByRole('button', { name: 'プラスマイナスを挿入' }));
    await waitFor(() => expect(parseMathLiveAnswer).toHaveBeenCalledTimes(3));
    const latex = parseMathLiveAnswer.mock.calls.map(([value]) => value);
    expect(latex[0]).toContain('\\frac');
    expect(latex[1]).toContain('\\sqrt');
    expect(latex[2]).toContain('\\pm');
  });

  it('supports physical Backspace, Delete, and cursor movement through MathLive', async () => {
    render(<AutoDrillApp engine={fixtureEngine()} />);
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    await screen.findByRole('heading', { name: '1けたのたしざん(1)' });
    fireEvent.click(screen.getByRole('textbox', { name: /^1番の答え/ }));

    pressKey('1');
    pressKey('2');
    await waitFor(() => expect(screen.getByRole('textbox', { name: '1番の答え 12' })).toBeInTheDocument());
    pressKey('ArrowLeft');
    pressKey('Delete');
    await waitFor(() => expect(screen.getByRole('textbox', { name: '1番の答え 1' })).toBeInTheDocument());
    pressKey('2');
    pressKey('ArrowLeft');
    pressKey('Backspace');
    await waitFor(() => expect(screen.getByRole('textbox', { name: '1番の答え 2' })).toBeInTheDocument());
  });

  it('leaves caret placement to MathLive while preserving insertion semantics', async () => {
    render(<AutoDrillApp engine={fixtureEngine()} />);
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    await screen.findByRole('heading', { name: '1けたのたしざん(1)' });
    fireEvent.click(screen.getByRole('textbox', { name: /^1番の答え/ }));
    pressKey('1');
    pressKey('2');
    await screen.findByRole('textbox', { name: '1番の答え 12' });

    pressKey('ArrowLeft');
    pressKey('9');
    const field = await screen.findByRole('textbox', { name: '1番の答え 192' }) as HTMLElement & { value: string };
    expect(field.value).toBe('192');
    expect(document.querySelector('[data-testid^="answer-caret-"]')).toBeNull();
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
      fireEvent.click(screen.getByRole('textbox', { name: /^1番の答え/ }));

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

      pressKey('Enter');
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
      fireEvent.click(screen.getByRole('textbox', { name: /^10番の答え/ }));

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

      pressKey('Enter');
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
    fireEvent.click(screen.getByRole('textbox', { name: /^1番の答え/ }));

    pressKey('1');
    pressKey('2');
    pressKey('Enter');

    await waitFor(() => expect(screen.getByRole('textbox', { name: '1番の答え 12' })).toBeInTheDocument(), {
      timeout: 1000,
    });
    await waitFor(() => expect(answerFrame(screen.getByRole('textbox', { name: /^2番の答え/ }))).toHaveClass('answer-box-selected'), {
      timeout: 1000,
    });
  });

  it('routes a rapid digit after Enter to the newly selected problem', async () => {
    render(<AutoDrillApp engine={delayedFixtureEngine()} />);
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    await screen.findByRole('heading', { name: '1けたのたしざん(1)' });
    fireEvent.click(screen.getByRole('textbox', { name: /^1番の答え/ }));

    pressKey('1');
    pressKey('Enter');
    pressKey('2');

    await waitFor(() => expect(screen.getByRole('textbox', { name: '1番の答え 1' })).toBeInTheDocument(), { timeout: 1000 });
    await waitFor(() => expect(answerFrame(screen.getByRole('textbox', { name: '2番の答え 2' }))).toHaveClass('answer-box-selected'), { timeout: 1000 });
  });

  it('clears through the shared MathLive path even when the empty caret geometry would fail the overflow guard', async () => {
    render(<AutoDrillApp engine={fixtureEngine()} />);
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    await screen.findByRole('heading', { name: '1けたのたしざん(1)' });
    fireEvent.click(screen.getByRole('textbox', { name: '1番の答え 未入力' }));
    pressKey('1');
    const field = await screen.findByRole('textbox', { name: '1番の答え 1' });
    const cell = screen.getByTestId('problem-cell-0');
    const frame = answerFrame(field);
    vi.spyOn(cell, 'getBoundingClientRect').mockReturnValue({
      left: 0, right: 100, top: 0, bottom: 100, width: 100, height: 100, x: 0, y: 0, toJSON: () => ({}),
    });
    vi.spyOn(frame, 'getBoundingClientRect').mockReturnValue({
      left: 150, right: 200, top: 150, bottom: 200, width: 50, height: 50, x: 150, y: 150, toJSON: () => ({}),
    });

    fireEvent.click(screen.getByRole('button', { name: 'クリア' }));

    await screen.findByRole('textbox', { name: '1番の答え 未入力' });
    expect(screen.queryByLabelText('式が大きすぎます！')).not.toBeInTheDocument();
  });

  it('keeps an answer at the Rust AST limit inside its box and shows a stable size-limit notice on the next digit', async () => {
    render(<AutoDrillApp engine={fixtureEngine()} />);
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    await screen.findByRole('heading', { name: '1けたのたしざん(1)' });
    const emptyAnswer = screen.getByRole('textbox', { name: '1番の答え 未入力' });
    expect(emptyAnswer).toHaveClass('answer-mathfield');
    fireEvent.click(emptyAnswer);

    const answerLimit = DRILL_CORE_CONTRACT.max_answer_ast_size;
    const limitDigits = '1'.repeat(answerLimit);
    const field = emptyAnswer as HTMLElement & { setValue(value: string): void; value: string };

    // This test owns the AST boundary, not rapid-key serialization. Enter the
    // accepted boundary value in one MathLive input event so CI timing does not
    // depend on draining one layout cycle per digit. The separate rapid-input
    // tests above exercise the action queue explicitly.
    field.setValue(limitDigits);
    fireEvent.input(field);
    const answer = await screen.findByRole('textbox', { name: `1番の答え ${limitDigits}` }) as typeof field;
    expect(answer.value).toBe(limitDigits);

    pressKey('1');
    expect(await screen.findByLabelText('式が大きすぎます！')).toBeInTheDocument();
    expect(screen.getByRole('textbox', { name: `1番の答え ${limitDigits}` })).toHaveValue(limitDigits);

    pressKey('Backspace');
    await screen.findByRole('textbox', { name: `1番の答え ${'1'.repeat(answerLimit - 1)}` });
    await waitFor(() => expect(screen.queryByLabelText('式が大きすぎます！')).not.toBeInTheDocument());
  });

  it('generates distinct raw seeds while exposing the canonical problem-set ID in the URL and footer', async () => {
    const { engine, seeds } = seedRecordingEngine();
    const generatedSeeds = ['A1b2', 'C3d4'];
    let seedIndex = 0;
    const pdfModule = await import('@/pdf/worksheet-pdf');
    const openSpy = vi.mocked(pdfModule.openWorksheetPdf);
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
    expect(window.location.search).toContain(`seed=${encodeURIComponent(`7-1-${ONE_DIGIT_ADDITION_THEME.generator_revision}-A1b2-2`)}`);

    fireEvent.click(screen.getByRole('button', { name: 'TOPに戻る' }));
    expect(screen.getByLabelText('Seed')).toHaveValue('');
    fireEvent.click(screen.getByRole('button', { name: '印刷 (pdfで出力)' }));
    await waitFor(() => expect(openSpy).toHaveBeenCalledTimes(1));

    expect(seeds).toEqual(generatedSeeds);
    expect(new Set(seeds).size).toBe(2);
    expect(openSpy.mock.calls[0]?.[1]).toEqual({
      generated_date: '2026-07-30',
      problem_set_id: `7-1-${ONE_DIGIT_ADDITION_THEME.generator_revision}-C3d4-2`,
    });
  });

  it('replays a full Seed through the Rust-owned problem-set identity boundary', async () => {
    const { engine, seeds, problemSetIds } = seedRecordingEngine();
    const problemSetId = fixtureWorksheet().problem_set_id;
    render(<AutoDrillApp engine={engine} seedGenerator={() => 'unused'} dateGenerator={() => new Date(2026, 6, 30)} />);

    fireEvent.click(screen.getByText('詳細設定'));
    fireEvent.change(screen.getByLabelText('Seed'), { target: { value: problemSetId } });
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    await screen.findByRole('heading', { name: '1けたのたしざん(1)' });

    expect(problemSetIds).toEqual([problemSetId]);
    expect(seeds).toEqual([]);
    expect(screen.getByTestId('worksheet-footer')).toHaveTextContent(problemSetId);
    expect(problemSetIdFromSearch(window.location.search)).toBe(problemSetId);
  });

  it('uses a full Seed for print replay without generating a new raw seed', async () => {
    const { engine, seeds, problemSetIds } = seedRecordingEngine();
    const problemSetId = fixtureWorksheet().problem_set_id;
    const pdfModule = await import('@/pdf/worksheet-pdf');
    const openSpy = vi.mocked(pdfModule.openWorksheetPdf);
    render(<AutoDrillApp engine={engine} seedGenerator={() => 'unused'} dateGenerator={() => new Date(2026, 6, 30)} />);

    fireEvent.click(screen.getByText('詳細設定'));
    fireEvent.change(screen.getByLabelText('Seed'), { target: { value: problemSetId } });
    fireEvent.click(screen.getByRole('button', { name: '印刷 (pdfで出力)' }));
    await waitFor(() => expect(openSpy).toHaveBeenCalledTimes(1));

    expect(problemSetIds).toEqual([problemSetId]);
    expect(seeds).toEqual([]);
    expect(openSpy.mock.calls[0]?.[1]).toEqual({ generated_date: '2026-07-30', problem_set_id: problemSetId });
  });

  it('replays the Seed query parameter on load and restores its theme and difficulty', async () => {
    const worksheet = linearFixtureWorksheet(2);
    window.history.replaceState(null, '', `/?seed=${encodeURIComponent(worksheet.problem_set_id)}`);
    render(<AutoDrillApp engine={fixtureEngine(worksheet)} />);

    await screen.findByRole('heading', { name: '一次方程式(1)：基本形' });
    fireEvent.click(screen.getByRole('button', { name: 'TOPに戻る' }));
    expect(screen.getByLabelText('Seed')).toHaveValue(worksheet.problem_set_id);
    expect(screen.getByRole('combobox', { name: '難易度' })).toHaveAttribute('data-selected-label', 'むずかしい');
    expect(screen.getByRole('combobox', { name: 'テーマ' })).toHaveAttribute('data-selected-label', '一次方程式(1)：基本形');
  });


  it('clears a replay Seed and permalink when difficulty changes', async () => {
    const worksheet = fixtureWorksheet();
    window.history.replaceState(null, '', `/?seed=${encodeURIComponent(worksheet.problem_set_id)}`);
    render(<AutoDrillApp engine={fixtureEngine(worksheet)} />);

    await screen.findByRole('heading', { name: '1けたのたしざん(1)' });
    fireEvent.click(screen.getByRole('button', { name: 'TOPに戻る' }));
    expect(screen.getByLabelText('Seed')).toHaveValue(worksheet.problem_set_id);

    fireEvent.click(screen.getByRole('combobox', { name: '難易度' }));
    fireEvent.click(screen.getByRole('option', { name: 'ふつう' }));

    expect(screen.getByLabelText('Seed')).toHaveValue('');
    expect(problemSetIdFromSearch(window.location.search)).toBeNull();
  });


  it('keeps the production WASM engine stable while permalink replay updates local state', async () => {
    const worksheet = fixtureWorksheet();
    const generateProblemSet = vi.fn(async (_input: string) => JSON.stringify({
      schema_version: DRILL_SCHEMA_VERSION,
      ok: true,
      data: worksheet,
      error: null,
    }));
    window.__AUTODRILL_WASM__ = { generate_problem_set: generateProblemSet };
    window.history.replaceState(null, '', `/?seed=${encodeURIComponent(worksheet.problem_set_id)}`);

    render(<AutoDrillApp />);

    await screen.findByRole('heading', { name: '1けたのたしざん(1)' });
    expect(generateProblemSet).toHaveBeenCalledTimes(1);
    expect(JSON.parse(generateProblemSet.mock.calls[0]?.[0] as string)).toEqual({ problem_set_id: worksheet.problem_set_id });
  });

  it('grades the latest answer when grading immediately after queued input', async () => {
    render(<AutoDrillApp engine={delayedFixtureEngine()} />);
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    await screen.findByRole('heading', { name: '1けたのたしざん(1)' });
    fireEvent.click(screen.getByRole('textbox', { name: /^1番の答え/ }));

    pressKey('2');
    fireEvent.click(screen.getByRole('button', { name: '採点' }));

    await waitFor(() => expect(screen.getByRole('status')).toHaveTextContent('1 / 20'), {
      timeout: 1000,
    });
    const correctMark = within(screen.getByTestId('problem-cell-0')).getByLabelText('正解');
    expect(correctMark).toHaveTextContent('○');
    expect(correctMark).toHaveClass('problem-grade-mark-correct');
  });

  it('locks grading synchronously and rejects a same-tick second grade request', async () => {
    const { engine, resolveGrade } = deferredGradingEngine();
    render(<AutoDrillApp engine={engine} />);
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    await screen.findByRole('heading', { name: '1けたのたしざん(1)' });
    fireEvent.click(screen.getByRole('textbox', { name: /^1番の答え/ }));

    const gradeButton = screen.getByRole('button', { name: '採点' });
    fireEvent.click(gradeButton);
    fireEvent.click(gradeButton);

    expect(gradeButton).toBeDisabled();
    expect(gradeButton).toHaveAttribute('aria-pressed', 'true');
    expect(gradeButton).toHaveAttribute('data-grade-state', 'grading');
    expect(screen.getByRole('button', { name: '印刷' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'TOPに戻る' })).toBeDisabled();
    expect(screen.queryByLabelText('数式入力パネル')).not.toBeInTheDocument();
    expect(screen.getByRole('textbox', { name: /^1番の答え/ })).toHaveAttribute('aria-readonly', 'true');
    await waitFor(() => expect(engine.gradeAnswer).toHaveBeenCalledTimes(1));

    resolveGrade({ schema_version: DRILL_SCHEMA_VERSION, items: [], correct_count: 0, total_count: 20 });
    await screen.findByRole('button', { name: '問題に戻る' });
    expect(gradeButton).toBeDisabled();
    expect(gradeButton).toHaveAttribute('data-grade-state', 'graded');
  });

  it('keeps graded answers immutable until the explicit return-to-problems transition', async () => {
    render(<AutoDrillApp engine={fixtureEngine()} />);
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    await screen.findByRole('heading', { name: '1けたのたしざん(1)' });
    fireEvent.click(screen.getByRole('textbox', { name: /^1番の答え/ }));
    pressKey('9');
    await screen.findByRole('textbox', { name: '1番の答え 9' });

    fireEvent.click(screen.getByRole('button', { name: '採点' }));
    await waitFor(() => expect(screen.getByRole('status')).toHaveTextContent('0 / 20'));
    const gradeButton = screen.getByRole('button', { name: '採点' });
    const gradedField = screen.getByRole('textbox', { name: '1番の答え 9' }) as HTMLElement & { readOnly: boolean; value: string };
    const gradedValue = gradedField.value;

    expect(gradeButton).toBeDisabled();
    expect(gradeButton).toHaveAttribute('aria-pressed', 'true');
    expect(gradeButton).toHaveAttribute('data-grade-state', 'graded');
    expect(gradedField).toHaveAttribute('aria-readonly', 'true');
    expect(gradedField.readOnly).toBe(true);
    fireEvent.click(gradedField);
    fireEvent.keyDown(gradedField, { key: '7' });
    expect(screen.queryByLabelText('数式入力パネル')).not.toBeInTheDocument();
    expect(gradedField.value).toBe(gradedValue);

    fireEvent.click(screen.getByRole('button', { name: '問題に戻る' }));
    expect(gradeButton).not.toBeDisabled();
    expect(gradeButton).toHaveAttribute('aria-pressed', 'false');
    expect(gradeButton).toHaveAttribute('data-grade-state', 'editing');
    const editableField = screen.getByRole('textbox', { name: '1番の答え 9' }) as HTMLElement & { readOnly: boolean };
    expect(editableField).toHaveAttribute('aria-readonly', 'false');
    expect(editableField.readOnly).toBe(false);
    fireEvent.click(editableField);
    expect(screen.getByLabelText('数式入力パネル')).toBeInTheDocument();
  });

  it('returns to editing, and only editing, when grading itself fails', async () => {
    render(<AutoDrillApp engine={failingGradingEngine()} />);
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    await screen.findByRole('heading', { name: '1けたのたしざん(1)' });
    fireEvent.click(screen.getByRole('textbox', { name: /^1番の答え/ }));
    fireEvent.click(screen.getByRole('button', { name: '採点' }));

    await screen.findByRole('alert', { name: 'grade failed' });
    const gradeButton = screen.getByRole('button', { name: '採点' });
    const field = screen.getByRole('textbox', { name: /^1番の答え/ }) as HTMLElement & { readOnly: boolean };
    expect(gradeButton).not.toBeDisabled();
    expect(gradeButton).toHaveAttribute('aria-pressed', 'false');
    expect(gradeButton).toHaveAttribute('data-grade-state', 'editing');
    expect(field.readOnly).toBe(false);
    fireEvent.click(field);
    expect(screen.getByLabelText('数式入力パネル')).toBeInTheDocument();
  });

  it('opens grading settings in a modal with examples and lets each warning category choose ○ or ×', async () => {
    render(<AutoDrillApp engine={warningFixtureEngine()} />);

    fireEvent.click(screen.getByText('詳細設定'));
    fireEvent.click(screen.getByRole('button', { name: '採点設定' }));

    const dialog = await screen.findByRole('dialog', { name: '採点設定' });
    expect(dialog).toBeInTheDocument();
    const examples = [
      ['例: 2/4 と 1/2 の表記を区別します。', ['2/4', '1/2']],
      ['例: √16 と 4 の表記を区別します。', ['√16', '4']],
      ['例: 0.5 と 1/2 の表記を区別します。', ['0.5', '1/2']],
    ] as const;
    for (const [description, mathLabels] of examples) {
      const example = within(dialog).getByLabelText(description);
      expect([...example.querySelectorAll('math-span')].map((math) => math.getAttribute('aria-label'))).toEqual(mathLabels);
      expect([...example.querySelectorAll('math-span')].every((math) => math.getAttribute('mode') === 'displaystyle')).toBe(true);
    }
    expect(within(dialog).getByText('上の3項目以外の、数学的に同値だが未整理・冗長な表記の違いを区別します。')).toBeInTheDocument();

    expect(screen.getByRole('button', { name: '約分しましょうをバツにする' })).toHaveAttribute('aria-pressed', 'true');
    expect(screen.getByRole('button', { name: '整数でこたえましょうをバツにする' })).toHaveAttribute('aria-pressed', 'true');
    expect(screen.getByRole('button', { name: '最後まで計算しましょうをバツにする' })).toHaveAttribute('aria-pressed', 'true');
    expect(screen.getByRole('button', { name: '分数でこたえましょうを丸にする' })).toHaveAttribute('aria-pressed', 'true');

    fireEvent.click(screen.getByRole('button', { name: '約分しましょうを丸にする' }));
    fireEvent.click(screen.getByRole('button', { name: '整数でこたえましょうを丸にする' }));
    fireEvent.click(screen.getByRole('button', { name: '最後まで計算しましょうを丸にする' }));
    fireEvent.click(screen.getByRole('button', { name: '採点設定を閉じる' }));
    expect(screen.queryByRole('dialog', { name: '採点設定' })).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    await screen.findByRole('heading', { name: '1けたのたしざん(1)' });
    fireEvent.click(screen.getByRole('button', { name: '採点' }));

    await screen.findByLabelText('注意 約分しましょう、最後まで計算しましょう、整数でこたえましょう、分数でこたえましょう');
    expect(answerFrame(screen.getByRole('textbox', { name: /^1番の答え/ }))).toHaveClass('answer-box-correct');
  });

  it('keeps the fraction-versus-decimal form warning correct by default', async () => {
    render(<AutoDrillApp engine={fractionFormWarningFixtureEngine()} />);
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    await screen.findByRole('heading', { name: '1けたのたしざん(1)' });
    fireEvent.click(screen.getByRole('button', { name: '採点' }));

    await screen.findByLabelText('注意 分数でこたえましょう');
    expect(answerFrame(screen.getByRole('textbox', { name: /^1番の答え/ }))).toHaveClass('answer-box-correct');
  });

  it('shows representation warnings on mathematically correct answers', async () => {
    render(<AutoDrillApp engine={warningFixtureEngine()} />);
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    await screen.findByRole('heading', { name: '1けたのたしざん(1)' });
    fireEvent.click(screen.getByRole('button', { name: '採点' }));

    const warnings = await screen.findByLabelText('注意 約分しましょう、最後まで計算しましょう、整数でこたえましょう、分数でこたえましょう');
    expect(warnings).toBeInTheDocument();
    expect(warnings.querySelectorAll(':scope > span')).toHaveLength(4);
    expect(answerFrame(screen.getByRole('textbox', { name: /^1番の答え/ }))).toHaveClass('answer-box-wrong');
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

      resolveGrade({ schema_version: DRILL_SCHEMA_VERSION, items: [], correct_count: 0, total_count: 20 });
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
      fireEvent.click(screen.getByRole('textbox', { name: /^1番の答え/ }));
      pressKey('9');
      await act(async () => {
        await Promise.resolve();
        await Promise.resolve();
      });
      act(() => vi.advanceTimersByTime(3_000));
      fireEvent.click(screen.getByRole('button', { name: '採点' }));
      expect(screen.queryByLabelText('数式入力パネル')).not.toBeInTheDocument();
      await act(async () => {
        await Promise.resolve();
        await Promise.resolve();
      });
      expect(screen.getByRole('button', { name: '問題に戻る' })).toBeInTheDocument();
      expect(screen.getByTestId('elapsed-time')).toHaveTextContent('00:03');

      act(() => vi.advanceTimersByTime(5_000));
      fireEvent.click(screen.getByRole('button', { name: '問題に戻る' }));
      expect(answerFrame(screen.getByRole('textbox', { name: '1番の答え 9' }))).not.toHaveClass('answer-box-wrong');
      expect(screen.queryByLabelText('数式入力パネル')).not.toBeInTheDocument();
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
    fireEvent.click(screen.getByRole('textbox', { name: /^1番の答え/ }));
    pressKey('2');
    await screen.findByRole('textbox', { name: '1番の答え 2' });
    fireEvent.click(screen.getByRole('button', { name: '採点' }));
    await screen.findByRole('button', { name: 'もう一回問題を解く' });
    const gradedActions = screen.getByLabelText('採点後の操作');
    expect(within(gradedActions).getAllByRole('button')).toHaveLength(2);
    expect(within(gradedActions).queryByRole('button', { name: '別の問題を解く' })).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'もう一回問題を解く' }));
    expect(screen.getByRole('textbox', { name: '1番の答え 未入力' })).toBeInTheDocument();
    expect(screen.getByTestId('worksheet-footer')).toHaveTextContent(footer ?? '');
    expect(screen.getByTestId('elapsed-time')).toHaveTextContent('00:00');
    expect(screen.queryByLabelText('数式入力パネル')).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'もう一回問題を解く' })).not.toBeInTheDocument();
    expect(generateSpy).toHaveBeenCalledTimes(1);
  });

  it('marks wrong and unanswered boxes in red and shows each correct answer beside it', async () => {
    render(<AutoDrillApp engine={fixtureEngine()} />);
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    await screen.findByRole('heading', { name: '1けたのたしざん(1)' });
    fireEvent.click(screen.getByRole('textbox', { name: /^1番の答え/ }));
    pressKey('9');
    fireEvent.click(screen.getByRole('button', { name: '採点' }));
    await waitFor(() => expect(screen.getByRole('status')).toHaveTextContent('0 / 20'));

    const firstProblem = screen.getByTestId('problem-cell-0');
    const wrongMark = within(firstProblem).getByLabelText('不正解');
    expect(wrongMark).toHaveTextContent('✓');
    expect(wrongMark).toHaveClass('problem-grade-mark-wrong');
    expect(wrongMark.parentElement).toHaveClass('problem-number-stack');
    expect(answerFrame(screen.getByRole('textbox', { name: '1番の答え 9' }))).toHaveClass('answer-box-wrong');
    expect(answerFrame(screen.getByRole('textbox', { name: '2番の答え 未入力' }))).toHaveClass('answer-box-wrong');
    expect(within(screen.getByTestId('problem-cell-0')).getByLabelText('正しい答え 2')).toHaveTextContent('2');
    expect(within(screen.getByTestId('problem-cell-1')).getByLabelText('正しい答え 3')).toHaveTextContent('3');
  });

  it('sends the same generated worksheet object to q2 print', async () => {
    const pdfModule = await import('@/pdf/worksheet-pdf');
    const openSpy = vi.mocked(pdfModule.openWorksheetPdf);
    const { engine, seeds } = seedRecordingEngine();
    render(<AutoDrillApp engine={engine} initialSettings={fixtureSettings()} seedGenerator={() => 'generatedSeed'} dateGenerator={() => new Date(2026, 6, 30)} />);
    fireEvent.click(screen.getByRole('button', { name: '問題生成' }));
    await screen.findByRole('heading', { name: '1けたのたしざん(1)' });
    fireEvent.click(screen.getByRole('button', { name: '印刷' }));
    await waitFor(() => expect(openSpy).toHaveBeenCalledTimes(1));
    expect(openSpy.mock.calls[0]?.[0].seed).toBe('generatedSeed');
    expect(openSpy.mock.calls[0]?.[1]).toEqual({ generated_date: '2026-07-30', problem_set_id: `7-1-${ONE_DIGIT_ADDITION_THEME.generator_revision}-generatedSeed-2` });
    expect(seeds).toEqual(['generatedSeed']);
  });

  it('uses the same PDF pipeline for q1 print after generation', async () => {
    const pdfModule = await import('@/pdf/worksheet-pdf');
    const openSpy = vi.mocked(pdfModule.openWorksheetPdf);
    render(<AutoDrillApp engine={fixtureEngine()} />);
    fireEvent.click(screen.getByRole('button', { name: '印刷 (pdfで出力)' }));
    await waitFor(() => expect(openSpy).toHaveBeenCalledTimes(1));
    expect(openSpy.mock.calls[0]?.[0].layout).toMatchObject({ problem_count: 20, columns: 2, rows: 10 });
    expect(screen.getByRole('heading', { name: 'まいんドリル' })).toBeInTheDocument();
  });

  it('does not retain a timer for q1 print and clears q2 timer on TOP', async () => {
    vi.useFakeTimers();
    vi.setSystemTime(1000);
    try {
      render(<AutoDrillApp engine={fixtureEngine()} />);
      fireEvent.click(screen.getByRole('button', { name: '印刷 (pdfで出力)' }));
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
      expect(screen.getByRole('heading', { name: 'まいんドリル' })).toBeInTheDocument();
      expect(vi.getTimerCount()).toBe(0);
    } finally {
      vi.useRealTimers();
    }
  });
});
