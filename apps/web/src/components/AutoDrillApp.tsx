'use client';

import { createContext, useCallback, useContext, useEffect, useRef, useState, type CSSProperties } from 'react';
import type { MathfieldElement } from 'mathlive';

import {
  DEFAULT_DRILL_SETTINGS,
  DRILL_SCHEMA_VERSION,
  DrillEngineError,
  answerNodeText,
  inputCapabilities,
  type AnswerInputStructure,
  type DrillEngine,
  type DrillSettings,
  type AnswerNode,
  type GradeResult,
  type GradeWarningCode,
  type WorksheetDto,
} from '@/domain/drill-engine';
import {
  CURRICULUM_TREE,
  DEFAULT_WEB_DRILL_SETTINGS,
  DIFFICULTY_OPTIONS,
  ONE_DIGIT_ADDITION_THEME,
  RECOMMENDED_GENRES,
  createWebDrillSettings,
  findCurriculumSelection,
  findImplementedThemeByNumericId,
  findTheme,
  type CurriculumMode,
  type CurriculumTheme,
  type DifficultyLevel,
  type WebDrillSettings,
} from '@/domain/curriculum';
import { RubyText, type RubyPart } from '@/components/RubyText';
import { CustomSelect } from '@/components/CustomSelect';
import { deleteEmptyMathLiveStructureBackward } from '@/components/mathlive-structure';
import { answerNodeLatex, mathTemplateInsertLatex } from '@/domain/mathlive-format';
import { createWasmDrillEngine } from '@/domain/wasm-adapter';
import { loadGeneratedWasmRuntime } from '@/wasm/load-generated';
import { A4_PAGE, buildSharedWorksheetLayout, getCellTopPosition } from '@/domain/layout';
import { worksheetGradeBandClass } from '@/domain/grade-band';
import { generateAutomaticSeed } from '@/domain/seed';
import { AUTODRILL_VERSION_LABEL } from '@/domain/version';
import {
  createWorksheetMetadata,
  formatWorksheetFooter,
  type WorksheetDateGenerator,
  type WorksheetMetadata,
} from '@/domain/worksheet-metadata';

type AutoDrillMathfield = MathfieldElement;

type WorksheetUiComponents = {
  MathLiveAnswerInput: typeof import('@/components/MathLiveMath').MathLiveAnswerInput;
  MathLiveStatic: typeof import('@/components/MathLiveMath').MathLiveStatic;
  MathTemplateIcon: typeof import('@/components/MathTemplateIcon').MathTemplateIcon;
  ProblemExpression: typeof import('@/components/ProblemExpression').ProblemExpression;
};

let worksheetUiPromise: Promise<WorksheetUiComponents> | null = null;

function preloadWorksheetUi(): Promise<WorksheetUiComponents> {
  if (!worksheetUiPromise) {
    worksheetUiPromise = Promise.all([
      import('@/components/MathLiveMath'),
      import('@/components/MathTemplateIcon'),
      import('@/components/ProblemExpression'),
    ]).then(([mathLive, templateIcon, problemExpression]) => ({
      MathLiveAnswerInput: mathLive.MathLiveAnswerInput,
      MathLiveStatic: mathLive.MathLiveStatic,
      MathTemplateIcon: templateIcon.MathTemplateIcon,
      ProblemExpression: problemExpression.ProblemExpression,
    })).catch((error: unknown) => {
      worksheetUiPromise = null;
      throw error;
    });
  }
  return worksheetUiPromise;
}

type Screen = 'settings' | 'worksheet';
type WorksheetPhase = 'editing' | 'grading' | 'graded' | 'replacing';
type SettingsBusyAction = 'generate' | 'print' | null;
const FURIGANA_STORAGE_KEY = 'autodrill:furigana-enabled';

async function openWorksheetPdfLazy(worksheet: WorksheetDto, targetWindow?: Window | null, metadata?: WorksheetMetadata): Promise<void> {
  const { openWorksheetPdf } = await import('@/pdf/worksheet-pdf');
  return openWorksheetPdf(worksheet, targetWindow, metadata);
}

const FuriganaContext = createContext(true);

const RUBY_TEXT: Readonly<Record<string, readonly RubyPart[]>> = {
  '計算ドリルをつくる': [["計算", "けいさん"], 'ドリルをつくる'],
  '出題範囲': [["出題", "しゅつだい"], ["範囲", "はんい"]],
  '学年から選ぶ': [["学年", "がくねん"], 'から', ["選", "えら"], 'ぶ'],
  '分数': [["分数", "ぶんすう"]],
  '帯分数': [["帯分数", "たいぶんすう"]],
  '小数': [["小数", "しょうすう"]],
  '平方根': [["平方根", "へいほうこん"]],
  '複数解': [["複数解", "ふくすうかい"]],
  '方程式': [["方程式", "ほうていしき"]],
  '一次方程式': [["一次方程式", "いちじほうていしき"]],
  '一次方程式(1)': [["一次方程式", "いちじほうていしき"], '(1)'],
  '一次方程式(2)': [["一次方程式", "いちじほうていしき"], '(2)'],
  '学年': [["学年", "がくねん"]],
  '小学1年生': [["小学", "しょうがく"], '1', ["年生", "ねんせい"]],
  '小学2年生': [["小学", "しょうがく"], '2', ["年生", "ねんせい"]],
  '小学3年生': [["小学", "しょうがく"], '3', ["年生", "ねんせい"]],
  '小学4年生': [["小学", "しょうがく"], '4', ["年生", "ねんせい"]],
  '小学5年生': [["小学", "しょうがく"], '5', ["年生", "ねんせい"]],
  '小学6年生': [["小学", "しょうがく"], '6', ["年生", "ねんせい"]],
  '中学1年生': [["中学", "ちゅうがく"], '1', ["年生", "ねんせい"]],
  '中学2年生': [["中学", "ちゅうがく"], '2', ["年生", "ねんせい"]],
  '中学3年生': [["中学", "ちゅうがく"], '3', ["年生", "ねんせい"]],
  '足し算と引き算': [["足", "た"], 'し', ["算", "ざん"], 'と', ["引", "ひ"], 'き', ["算", "ざん"]],
  '掛け算と割り算': [["掛", "か"], 'け', ["算", "ざん"], 'と', ["割", "わ"], 'り', ["算", "ざん"]],
  '負の数': [["負", "ふ"], 'の', ["数", "すう"]],
  '正の数・負の数': [["正", "せい"], 'の', ["数", "すう"], '・', ["負", "ふ"], 'の', ["数", "すう"]],
  '一桁の足し算': [["一桁", "ひとけた"], 'の', ["足", "た"], 'し', ["算", "ざん"]],
  '一桁の引き算': [["一桁", "ひとけた"], 'の', ["引", "ひ"], 'き', ["算", "ざん"]],
  '二桁の足し算': [["二桁", "ふたけた"], 'の', ["足", "た"], 'し', ["算", "ざん"]],
  '九九': [["九九", "くく"]],
  '分数の足し算': [["分数", "ぶんすう"], 'の', ["足", "た"], 'し', ["算", "ざん"]],
  '分数の引き算': [["分数", "ぶんすう"], 'の', ["引", "ひ"], 'き', ["算", "ざん"]],
  '分数の掛け算': [["分数", "ぶんすう"], 'の', ["掛", "か"], 'け', ["算", "ざん"]],
  '負の数の計算(1)': [["負", "ふ"], 'の', ["数", "すう"], 'の', ["計算", "けいさん"], '(1)'],
  '負の数の計算(2)': [["負", "ふ"], 'の', ["数", "すう"], 'の', ["計算", "けいさん"], '(2)'],
  '難易度': [["難易度", "なんいど"]],
  'このテーマはまだ利用できません': ['このテーマはまだ', ["利用", "りよう"], 'できません'],
  '問題数': [["問題数", "もんだいすう"]],
  '問': [["問", "もん"]],
  '任意': [["任意", "にんい"]],
  '詳細設定': [["詳細設定", "しょうさいせってい"]],
  '同じSeedでは同じ問題が生成されます。': [["同", "おな"], 'じSeedでは', ["同", "おな"], 'じ', ["問題", "もんだい"], 'が', ["生成", "せいせい"], 'されます。'],
  '空欄なら毎回自動生成': [["空欄", "くうらん"], 'なら', ["毎回", "まいかい"], ["自動生成", "じどうせいせい"]],
  '同じSeedで同じ問題を再現できます。空欄なら毎回新しく生成します。': [["同", "おな"], 'じSeedで', ["同", "おな"], 'じ', ["問題", "もんだい"], 'を', ["再現", "さいげん"], 'できます。', ["空欄", "くうらん"], 'なら', ["毎回", "まいかい"], ["新", "あたら"], 'しく', ["生成", "せいせい"], 'します。'],
  '前回': [["前回", "ぜんかい"]],
  '問題生成': [["問題生成", "もんだいせいせい"]],
  '問題を生成中…': [["問題", "もんだい"], 'を', ["生成中", "せいせいちゅう"], '…'],
  '印刷': [["印刷", "いんさつ"]],
  '印刷 (pdfで出力)': [["印刷", "いんさつ"], ' (pdfで', ["出力", "しゅつりょく"], ')'],
  'PDFを準備中…': ['PDFを', ["準備中", "じゅんびちゅう"], '…'],
  '問題を生成しています。しばらくお待ちください。': [["問題", "もんだい"], 'を', ["生成", "せいせい"], 'しています。しばらくお', ["待", "ま"], 'ちください。'],
  '印刷用PDFを準備しています。しばらくお待ちください。': [["印刷用", "いんさつよう"], 'PDFを', ["準備", "じゅんび"], 'しています。しばらくお', ["待", "ま"], 'ちください。'],
  '問題の生成・入力状態・採点は Rust/WASM が担当します。': [["問題", "もんだい"], 'の', ["生成", "せいせい"], '・', ["入力状態", "にゅうりょくじょうたい"], '・', ["採点", "さいてん"], 'は Rust/WASM が', ["担当", "たんとう"], 'します。'],
  '回答時間': [["回答時間", "かいとうじかん"]],
  '採点': [["採点", "さいてん"]],
  'TOPに戻る': ['TOPに', ["戻", "もど"], 'る'],
  '正解': [["正解", "せいかい"]],
  '約分': [["約分", "やくぶん"]],
  '冗長なマイナス': [["冗長", "じょうちょう"], 'なマイナス'],
  '余計な小数点': [["余計", "よけい"], 'な', ["小数点", "しょうすうてん"]],
  '整数で答えましょう': [["整数", "せいすう"], 'で', ["答", "こた"], 'えましょう'],
  '最も簡単な分数の形で答えましょう': [["最", "もっと"], 'も', ["簡単", "かんたん"], 'な', ["分数", "ぶんすう"], 'の', ["形", "かたち"], 'で', ["答", "こた"], 'えましょう'],
  '採点後の操作': [["採点後", "さいてんご"], 'の', ["操作", "そうさ"]],
  '問題に戻る': [["問題", "もんだい"], 'に', ["戻", "もど"], 'る'],
  'もう一回問題を解く': ['もう', ["一回", "いっかい"], ["問題", "もんだい"], 'を', ["解", "と"], 'く'],
  '別の問題を解く': [["別", "べつ"], 'の', ["問題", "もんだい"], 'を', ["解", "と"], 'く'],
  '確定': [["確定", "かくてい"]],
  '式が大きすぎます！': [["式", "しき"], 'が', ["大", "おお"], 'きすぎます！'],
  '問題生成がタイムアウトしました。': [["問題生成", "もんだいせいせい"], 'がタイムアウトしました。'],
  '問題生成の試行上限に達しました。': [["問題生成", "もんだいせいせい"], 'の', ["試行上限", "しこうじょうげん"], 'に', ["達", "たっ"], 'しました。'],
  'Rust/WASMの実行環境を読み込めません。WASMパッケージを生成してから再試行してください。': ['Rust/WASMの', ["実行環境", "じっこうかんきょう"], 'を', ["読", "よ"], 'み', ["込", "こ"], 'めません。WASMパッケージを', ["生成", "せいせい"], 'してから', ["再試行", "さいしこう"], 'してください。'],
  'Rust/WASMの実行環境を読み込めません。WASMパッケージを生成してから再読み込みしてください。': ['Rust/WASMの', ["実行環境", "じっこうかんきょう"], 'を', ["読", "よ"], 'み', ["込", "こ"], 'めません。WASMパッケージを', ["生成", "せいせい"], 'してから', ["再読み込み", "さいよみこみ"], 'してください。'],
  '処理に失敗しました。': [["処理", "しょり"], 'に', ["失敗", "しっぱい"], 'しました。'],
};

const GRADE_WARNING_LABELS: Readonly<Record<GradeWarningCode, string>> = {
  fraction_not_reduced: '約分',
  redundant_negative: '冗長なマイナス',
  redundant_decimal: '余計な小数点',
  fraction_form_required: '最も簡単な分数の形で答えましょう',
  integer_form_required: '整数で答えましょう',
};

const STRUCTURE_LABELS: Readonly<Record<Exclude<AnswerInputStructure, 'decimal'>, string>> = {
  fraction: '分数',
  mixed_fraction: '帯分数',
  root: '平方根',
  negative: 'マイナス',
  plus_minus: 'プラスマイナス',
  tuple: '複数解',
};

type MathInputCommand =
  | { kind: 'insert_digit'; digit: number }
  | { kind: 'insert_structure'; structure: AnswerInputStructure }
  | { kind: 'move_left' }
  | { kind: 'move_right' }
  | { kind: 'delete_backward' }
  | { kind: 'delete_forward' }
  | { kind: 'clear' }
  | { kind: 'commit' };

if (process.env.NODE_ENV !== 'production') {
  for (const [text, parts] of Object.entries(RUBY_TEXT)) {
    const baseText = parts.map((part) => typeof part === 'string' ? part : part[0]).join('');
    if (baseText !== text) throw new Error(`Ruby text must preserve its source: ${text}`);
  }
}

function RubyMessage({ text }: { text: string }) {
  const parts = RUBY_TEXT[text];
  const furiganaEnabled = useContext(FuriganaContext);
  return parts && furiganaEnabled ? <RubyText parts={parts} /> : text;
}

export type AutoDrillAppProps = {
  engine?: DrillEngine;
  initialSettings?: DrillSettings;
  initialWebSettings?: WebDrillSettings;
  onWebSettingsChange?: (settings: WebDrillSettings) => void;
  seedGenerator?: () => string;
  dateGenerator?: WorksheetDateGenerator;
};

function formatElapsed(startedAt: number | null, now: number): string {
  if (startedAt === null) return '00:00';
  const seconds = Math.max(0, Math.floor((now - startedAt) / 1000));
  return `${String(Math.floor(seconds / 60)).padStart(2, '0')}:${String(seconds % 60).padStart(2, '0')}`;
}

const MIN_RENDERED_ANSWER_WIDTH_LIMIT = 42;
const MIN_RENDERED_ANSWER_HEIGHT_LIMIT = 38;
const ANSWER_CELL_RIGHT_GUTTER = 6;

function numericCssPixels(value: string): number {
  const parsed = Number.parseFloat(value);
  return Number.isFinite(parsed) ? parsed : 0;
}

function renderedMathfieldContentSize(mathfield: AutoDrillMathfield): { width: number; height: number } {
  const hostRect = mathfield.getBoundingClientRect();
  const content = mathfield.shadowRoot?.querySelector<HTMLElement>('[part~="content"]');
  const contentRect = content?.getBoundingClientRect();
  return {
    width: Math.max(hostRect.width, mathfield.scrollWidth, contentRect?.width ?? 0),
    height: Math.max(hostRect.height, mathfield.scrollHeight, contentRect?.height ?? 0),
  };
}

function resizeMathfieldFrameAndCheck(mathfield: AutoDrillMathfield, problemIndex: number): boolean {
  const frame = mathfield.closest<HTMLElement>('.answer-box');
  if (!frame) return true;
  const frameStyle = window.getComputedStyle(frame);
  const horizontalChrome = numericCssPixels(frameStyle.paddingLeft)
    + numericCssPixels(frameStyle.paddingRight)
    + numericCssPixels(frameStyle.borderLeftWidth)
    + numericCssPixels(frameStyle.borderRightWidth);
  const verticalChrome = numericCssPixels(frameStyle.paddingTop)
    + numericCssPixels(frameStyle.paddingBottom)
    + numericCssPixels(frameStyle.borderTopWidth)
    + numericCssPixels(frameStyle.borderBottomWidth);
  const contentSize = renderedMathfieldContentSize(mathfield);
  const requiredWidth = Math.max(MIN_RENDERED_ANSWER_WIDTH_LIMIT, Math.ceil(contentSize.width + horizontalChrome));
  const requiredHeight = Math.max(MIN_RENDERED_ANSWER_HEIGHT_LIMIT, Math.ceil(contentSize.height + verticalChrome));

  const cell = document.querySelector<HTMLElement>(`[data-problem-index="${problemIndex}"]`);
  const cellRect = cell?.getBoundingClientRect();
  const currentFrameRect = frame.getBoundingClientRect();
  const maxWidth = cellRect && cellRect.width > 0 && currentFrameRect.left > 0
    ? Math.max(MIN_RENDERED_ANSWER_WIDTH_LIMIT, cellRect.right - currentFrameRect.left - ANSWER_CELL_RIGHT_GUTTER)
    : 180;
  const maxHeight = cellRect && cellRect.height > 0
    ? Math.max(MIN_RENDERED_ANSWER_HEIGHT_LIMIT, cellRect.height * 0.72)
    : 80;

  if (requiredWidth > maxWidth || requiredHeight > maxHeight) return false;
  frame.style.width = `${requiredWidth}px`;
  frame.style.height = `${requiredHeight}px`;
  return true;
}

function waitForMathfieldLayout(): Promise<void> {
  if (typeof window === 'undefined' || typeof window.requestAnimationFrame !== 'function') return Promise.resolve();
  return new Promise((resolve) => {
    window.requestAnimationFrame(() => resolve());
  });
}

type WorksheetAnswerFieldProps = {
  worksheetUi: WorksheetUiComponents;
  problem: WorksheetDto['problems'][number];
  index: number;
  answer: AnswerNode;
  isSelected: boolean;
  result: GradeResult['items'][number] | undefined;
  gradeResult: GradeResult | null;
  inputLocked: boolean;
  answerPrefix: string | null;
  onSelect: (index: number) => void;
  onRegisterMathfield: (index: number, mathfield: AutoDrillMathfield | null) => void;
  onMathInput: (index: number, mathfield: AutoDrillMathfield, latex: string) => void;
  onCommit: (index: number) => void;
};

function WorksheetAnswerField({
  worksheetUi,
  problem,
  index,
  answer,
  isSelected,
  result,
  gradeResult,
  inputLocked,
  answerPrefix,
  onSelect,
  onRegisterMathfield,
  onMathInput,
  onCommit,
}: WorksheetAnswerFieldProps) {
  const { MathLiveAnswerInput, MathLiveStatic } = worksheetUi;
  const answerText = answerNodeText(answer);
  const canonicalAnswer = answerNodeText(problem.canonical_answer);

  return (
    <span className="problem-answer-area">
      {answerPrefix ? (
        <MathLiveStatic
          className="answer-prefix-label"
          latex={answerPrefix.replaceAll(' ', '\\,')}
          ariaLabel={answerPrefix}
        />
      ) : null}
      <MathLiveAnswerInput
        key={`${problem.problem_id}:${gradeResult ? 'graded' : 'editing'}`}
        initialLatex={answerNodeLatex(answer)}
        frameClassName={`answer-box ${isSelected ? 'answer-box-selected' : ''} ${result ? (result.correct ? 'answer-box-correct' : 'answer-box-wrong') : ''}`}
        ariaLabel={`${index + 1}番の答え ${answerText || '未入力'}`}
        selected={isSelected}
        readOnly={inputLocked}
        onSelect={() => onSelect(index)}
        onInputLatex={(mathfield, latex) => onMathInput(index, mathfield, latex)}
        onCommit={() => onCommit(index)}
        onRegister={(mathfield) => onRegisterMathfield(index, mathfield)}
      />
      {result?.correct ? <span className="result-mark" aria-label="正解">○</span> : null}
      {result && !result.correct ? (
        <span className="correct-answer" aria-label={`正しい答え ${canonicalAnswer}`}>
          <MathLiveStatic
            className="canonical-answer-math"
            latex={answerNodeLatex(problem.canonical_answer)}
            ariaLabel={canonicalAnswer}
          />
        </span>
      ) : null}
      {result && result.warnings.length > 0 ? (
        <span className="grade-warnings" aria-label={`注意 ${result.warnings.map((warning) => GRADE_WARNING_LABELS[warning]).join('、')}`}>
          {result.warnings.map((warning) => (
            <span key={warning}><RubyMessage text={GRADE_WARNING_LABELS[warning]} /></span>
          ))}
        </span>
      ) : null}
    </span>
  );
}

function scheduleProblemScroll(currentIndex: number, nextIndex: number) {
  const run = () => {
    const currentCell = document.querySelector<HTMLElement>(`[data-problem-index="${currentIndex}"]`);
    const nextCell = document.querySelector<HTMLElement>(`[data-problem-index="${nextIndex}"]`);
    if (!currentCell || !nextCell) return;
    const ribbonBottom = document.querySelector<HTMLElement>('.ribbon')?.getBoundingClientRect().bottom ?? 0;
    const keypadTop = document.querySelector<HTMLElement>('.input-panel')?.getBoundingClientRect().top ?? window.innerHeight;
    const currentRect = currentCell.getBoundingClientRect();
    const nextRect = nextCell.getBoundingClientRect();
    // jsdom and hidden/offscreen renderers report a zero-sized rectangle.
    // There is no meaningful viewport correction to make in that case.
    if (currentRect.height <= 0 || nextRect.height <= 0) return;
    const safeTop = ribbonBottom + 12;
    const safeBottom = keypadTop - 12;
    const sameColumn = currentCell.dataset.layoutColumn === nextCell.dataset.layoutColumn;
    // Within a column, advance the paper by one exact row even when both rows
    // already fit. At the 10 -> 11 column boundary, reset the new column's
    // first problem below the fixed ribbon instead of applying a nine-row jump.
    const top = sameColumn ? nextRect.top - currentRect.top : nextRect.top - safeTop;
    if (top !== 0) window.scrollBy({ top, behavior: 'auto' });
    const positionedTop = nextRect.top - top;
    const positionedBottom = positionedTop + nextRect.height;
    const safetyTop = positionedTop < safeTop
      ? positionedTop - safeTop
      : positionedBottom > safeBottom
        ? positionedBottom - safeBottom
        : 0;
    if (safetyTop !== 0) window.scrollBy({ top: safetyTop, behavior: 'auto' });
    // After the deterministic vertical movement, nearest keeps the selected
    // cell horizontally visible and provides a final keypad/ribbon safety net.
    if (typeof nextCell.scrollIntoView === 'function') {
      nextCell.scrollIntoView({ block: 'nearest', inline: 'nearest' });
    }
  };
  if (typeof window.requestAnimationFrame === 'function') window.requestAnimationFrame(run);
  else window.setTimeout(run, 0);
}

export function AutoDrillApp({
  engine: injectedEngine,
  initialSettings = DEFAULT_DRILL_SETTINGS,
  initialWebSettings = DEFAULT_WEB_DRILL_SETTINGS,
  onWebSettingsChange,
  seedGenerator = generateAutomaticSeed,
  dateGenerator = () => new Date(),
}: AutoDrillAppProps) {
  const engine = injectedEngine ?? createWasmDrillEngine();
  const [screen, setScreen] = useState<Screen>('settings');
  const [worksheetUi, setWorksheetUi] = useState<WorksheetUiComponents | null>(null);
  const [settings, setSettings] = useState<DrillSettings>(() => ({
    ...initialSettings,
    // Route-provided Web settings are the canonical selection for the first
    // q1 request. Preserve an explicit engine fixture seed when the route
    // leaves the user-facing seed blank.
    numeric_theme_id: initialWebSettings.numeric_theme_id,
    difficulty: initialWebSettings.difficulty,
    seed: initialWebSettings.seed === '' ? initialSettings.seed : initialWebSettings.seed,
  }));
  const [worksheet, setWorksheet] = useState<WorksheetDto | null>(null);
  const [worksheetMetadata, setWorksheetMetadata] = useState<WorksheetMetadata | null>(null);
  const [answers, setAnswers] = useState<Record<string, AnswerNode>>({});
  const [selectedIndex, setSelectedIndex] = useState<number | null>(null);
  const [startedAt, setStartedAt] = useState<number | null>(null);
  const [finishedAt, setFinishedAt] = useState<number | null>(null);
  const [now, setNow] = useState(() => Date.now());
  const [gradeResult, setGradeResult] = useState<GradeResult | null>(null);
  const [worksheetPhase, setWorksheetPhase] = useState<WorksheetPhase>('editing');
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [settingsBusyAction, setSettingsBusyAction] = useState<SettingsBusyAction>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [curriculumMode, setCurriculumMode] = useState<CurriculumMode>(() => {
    const initialTheme = findTheme(initialWebSettings.themeKey);
    return initialTheme?.implemented && initialTheme.recommendedGenre ? 'recommended' : 'grade';
  });
  const [webSettings, setWebSettings] = useState<WebDrillSettings>(() => {
    const theme = findTheme(initialWebSettings.themeKey) ?? ONE_DIGIT_ADDITION_THEME;
    return createWebDrillSettings(theme, initialWebSettings.difficulty, initialWebSettings.seed);
  });
  // Default ON keeps the server and first client render identical. The saved
  // browser preference is applied only after hydration.
  const [furiganaEnabled, setFuriganaEnabled] = useState(true);
  const answersRef = useRef<Record<string, AnswerNode>>({});
  const selectedIndexRef = useRef<number | null>(null);
  const inputEnabledRef = useRef(false);
  const worksheetPhaseRef = useRef<WorksheetPhase>('editing');
  const actionQueueRef = useRef(Promise.resolve());
  const mathfieldRefs = useRef(new Map<number, AutoDrillMathfield>());
  const acceptedLatexRef = useRef<Record<string, string>>({});
  const noticeTimerRef = useRef<number | null>(null);
  const selectedTheme = findTheme(webSettings.themeKey) ?? ONE_DIGIT_ADDITION_THEME;

  const transitionWorksheetPhase = useCallback((next: WorksheetPhase) => {
    worksheetPhaseRef.current = next;
    setWorksheetPhase(next);
  }, []);

  useEffect(() => {
    onWebSettingsChange?.(webSettings);
  }, [onWebSettingsChange, webSettings]);

  useEffect(() => {
    try {
      const stored = window.localStorage.getItem(FURIGANA_STORAGE_KEY);
      if (stored === 'false') setFuriganaEnabled(false);
      if (stored === 'true') setFuriganaEnabled(true);
    } catch {
      // Storage can be unavailable in privacy-restricted contexts. The
      // documented default remains ON and the toggle still works in memory.
    }
  }, []);

  const changeFurigana = useCallback((enabled: boolean) => {
    setFuriganaEnabled(enabled);
    try {
      window.localStorage.setItem(FURIGANA_STORAGE_KEY, String(enabled));
    } catch {
      // Keep the in-memory preference usable when persistence is unavailable.
    }
  }, []);

  useEffect(() => {
    if (injectedEngine || typeof window === 'undefined') return undefined;
    let cancelled = false;
    const idleWindow = window as Window & {
      requestIdleCallback?: (callback: () => void, options?: { timeout: number }) => number;
      cancelIdleCallback?: (handle: number) => void;
    };
    const preload = () => {
      if (!cancelled) void preloadWorksheetUi();
    };
    if (idleWindow.requestIdleCallback) {
      const handle = idleWindow.requestIdleCallback(preload, { timeout: 1_000 });
      return () => {
        cancelled = true;
        idleWindow.cancelIdleCallback?.(handle);
      };
    }
    const handle = window.setTimeout(preload, 500);
    return () => {
      cancelled = true;
      window.clearTimeout(handle);
    };
  }, [injectedEngine]);

  useEffect(() => {
    // Tests and embedders may inject a deterministic engine. The production
    // path preloads the ignored wasm-pack package and exposes its functions
    // through the adapter's existing global seam. A missing package remains a
    // normal, actionable wasm_unavailable error when the user presses a button.
    if (injectedEngine || typeof window === 'undefined' || window.__AUTODRILL_WASM__) return undefined;
    let active = true;
    void loadGeneratedWasmRuntime()
      .then((runtime) => {
        if (active) window.__AUTODRILL_WASM__ = runtime;
      })
      .catch(() => {
        if (active) setError('Rust/WASMの実行環境を読み込めません。WASMパッケージを生成してから再読み込みしてください。');
      });
    return () => {
      active = false;
    };
  }, [injectedEngine]);

  useEffect(() => {
    if (startedAt === null || finishedAt !== null) return undefined;
    const timer = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(timer);
  }, [finishedAt, startedAt]);

  const dismissNotice = useCallback(() => {
    if (noticeTimerRef.current !== null) window.clearTimeout(noticeTimerRef.current);
    noticeTimerRef.current = null;
    setNotice(null);
  }, []);

  const showNotice = useCallback((message: string) => {
    if (noticeTimerRef.current !== null) window.clearTimeout(noticeTimerRef.current);
    setNotice(message);
    noticeTimerRef.current = window.setTimeout(() => {
      noticeTimerRef.current = null;
      setNotice(null);
    }, 4_000);
  }, []);

  useEffect(() => () => {
    if (noticeTimerRef.current !== null) window.clearTimeout(noticeTimerRef.current);
  }, []);

  const changeTheme = useCallback((theme: CurriculumTheme) => {
    setWebSettings((current) => createWebDrillSettings(theme, current.difficulty, current.seed));
    if (theme.implemented) {
      setSettings((current) => ({
        ...current,
        numeric_theme_id: theme.numeric_theme_id,
      }));
    }
    setError(null);
  }, []);

  const changeCurriculumMode = useCallback((mode: CurriculumMode) => {
    setCurriculumMode(mode);
    if (mode === 'recommended') {
      const recommended = RECOMMENDED_GENRES[0]?.themes[0];
      changeTheme(recommended ?? ONE_DIGIT_ADDITION_THEME);
    }
  }, [changeTheme]);

  const changeDifficulty = useCallback((difficulty: DifficultyLevel) => {
    setWebSettings((current) => ({ ...current, difficulty }));
    setSettings((current) => ({ ...current, difficulty }));
  }, []);

  const changeSettings = useCallback((next: DrillSettings) => {
    setSettings(next);
    setWebSettings((current) => ({ ...current, seed: next.seed }));
  }, []);

  const installWorksheet = useCallback((nextWorksheet: WorksheetDto, metadata: WorksheetMetadata) => {
    const nextAnswers = Object.fromEntries(nextWorksheet.problems.map((problem) => [problem.problem_id, { type: 'empty' } satisfies AnswerNode]));
    answersRef.current = nextAnswers;
    acceptedLatexRef.current = Object.fromEntries(nextWorksheet.problems.map((problem) => [problem.problem_id, '']));
    setWorksheet(nextWorksheet);
    setWorksheetMetadata(metadata);
    setAnswers(nextAnswers);
    setGradeResult(null);
    transitionWorksheetPhase('editing');
    setSelectedIndex(null);
    selectedIndexRef.current = null;
    inputEnabledRef.current = false;
    const timerStart = Date.now();
    setStartedAt(timerStart);
    setFinishedAt(null);
    setNow(timerStart);
  }, [transitionWorksheetPhase]);

  const showEngineError = useCallback((value: unknown) => {
    if (value instanceof DrillEngineError) {
      if (value.kind === 'answer_ast_size_limit') {
        showNotice('式が大きすぎます！');
        return;
      }
      setError(
        value.kind === 'generation_timeout'
          ? '問題生成がタイムアウトしました。'
          : value.kind === 'generation_attempt_limit'
            ? '問題生成の試行上限に達しました。'
            : value.kind === 'wasm_unavailable'
              ? 'Rust/WASMの実行環境を読み込めません。WASMパッケージを生成してから再試行してください。'
              : value.message,
      );
      return;
    }
    setError(value instanceof Error ? value.message : '処理に失敗しました。');
  }, [showNotice]);

  const generate = useCallback(async (printAfterGeneration: boolean) => {
    if (!selectedTheme.implemented) {
      setError('このテーマはまだ利用できません');
      return;
    }
    setError(null);
    setGradeResult(null);
    dismissNotice();
    setBusy(true);
    setSettingsBusyAction(printAfterGeneration ? 'print' : 'generate');
    const worksheetUiReady = printAfterGeneration ? null : preloadWorksheetUi();
    try {
      const seed = settings.seed === '' ? seedGenerator() : settings.seed;
      const metadata = createWorksheetMetadata(seed, dateGenerator());
      const generatedWorksheet = await engine.generateWorksheet({ ...settings, seed });
      const loadedWorksheetUi = worksheetUiReady ? await worksheetUiReady : null;
      // The Rust DTO remains the source of the problems. The spread adds the
      // exact seed used by this UI invocation when a fixture/runtime returns a
      // stale or normalized seed string.
      const nextWorksheet = { ...generatedWorksheet, seed };
      if (printAfterGeneration) await openWorksheetPdfLazy(nextWorksheet, undefined, metadata);
      if (printAfterGeneration) {
        setWorksheet(nextWorksheet);
        setWorksheetMetadata(metadata);
        setScreen('settings');
      } else {
        if (!loadedWorksheetUi) throw new Error('Worksheet UI failed to load.');
        setWorksheetUi(loadedWorksheetUi);
        installWorksheet(nextWorksheet, metadata);
        setScreen('worksheet');
      }
    } catch (value) {
      showEngineError(value);
    } finally {
      setBusy(false);
      setSettingsBusyAction(null);
    }
  }, [dateGenerator, dismissNotice, engine, installWorksheet, seedGenerator, selectedTheme, settings, showEngineError]);

  const selectProblem = useCallback((index: number) => {
    if (worksheetPhaseRef.current !== 'editing') return;
    inputEnabledRef.current = true;
    selectedIndexRef.current = index;
    setSelectedIndex(index);
    // The input panel is mounted by this selection. Running on the next frame
    // lets the shared viewport guard see its real top edge and keeps even a
    // bottom-aligned answer field (for example x = [...]) unobscured.
    scheduleProblemScroll(index, index);
    setError(null);
    dismissNotice();
  }, [dismissNotice]);

  const registerMathfield = useCallback((index: number, mathfield: AutoDrillMathfield | null) => {
    if (mathfield) mathfieldRefs.current.set(index, mathfield);
    else mathfieldRefs.current.delete(index);
  }, []);

  const updateMathLiveAnswer = useCallback((index: number, mathfield: AutoDrillMathfield, latex: string) => {
    if (!worksheet || worksheetPhaseRef.current !== 'editing' || !inputEnabledRef.current || !worksheet.problems[index]) return Promise.resolve();
    const problem = worksheet.problems[index];
    const problemId = problem.problem_id;

    const run = async () => {
      const previous = answersRef.current[problemId] ?? ({ type: 'empty' } satisfies AnswerNode);
      const previousLatex = acceptedLatexRef.current[problemId] ?? answerNodeLatex(previous);

      let answer: AnswerNode;
      try {
        // Parse first: AST-size rejection is independent of layout and should
        // behave as an immediate NOP, without leaving the rejected value visible
        // while waiting for animation frames.
        answer = await engine.parseMathLiveAnswer(latex, problem.input_interface);
      } catch (value) {
        mathfield.setValue(previousLatex, { silenceNotifications: true });
        if (value instanceof DrillEngineError && value.kind === 'answer_ast_size_limit') {
          showEngineError(value);
        }
        return;
      }

      // MathLive updates its shadow DOM after the input event. Measure only
      // after layout has settled; otherwise a deeply nested fraction can look
      // small at measurement time and overflow the frame one paint later.
      await waitForMathfieldLayout();
      if (!resizeMathfieldFrameAndCheck(mathfield, index)) {
        // Render-size rejection is also a true NOP. Restore the exact accepted
        // LaTeX, not a normalized/reconstructed AnswerNode representation.
        mathfield.setValue(previousLatex, { silenceNotifications: true });
        await waitForMathfieldLayout();
        resizeMathfieldFrameAndCheck(mathfield, index);
        setNotice('式が大きすぎます！');
        return;
      }

      acceptedLatexRef.current = { ...acceptedLatexRef.current, [problemId]: latex };
      const nextAnswers = { ...answersRef.current, [problemId]: answer };
      answersRef.current = nextAnswers;
      setAnswers(nextAnswers);
      setError(null);
      dismissNotice();
    };

    const queued = actionQueueRef.current.then(run, run);
    actionQueueRef.current = queued.then(() => undefined, () => undefined);
    return queued;
  }, [dismissNotice, engine, showEngineError, worksheet]);

  const commitMathfield = useCallback((index: number) => {
    if (!worksheet || worksheetPhaseRef.current !== 'editing' || !worksheet.problems[index]) return Promise.resolve();
    if (index < worksheet.problems.length - 1 && inputEnabledRef.current) {
      selectedIndexRef.current = index + 1;
      setSelectedIndex(index + 1);
      scheduleProblemScroll(index, index + 1);
    }

    return actionQueueRef.current;
  }, [worksheet]);

  const applyMathCommand = useCallback((command: MathInputCommand) => {
    const index = selectedIndexRef.current;
    if (index === null || worksheetPhaseRef.current !== 'editing' || !inputEnabledRef.current) return;
    const mathfield = mathfieldRefs.current.get(index);
    if (!mathfield) return;
    mathfield.focus();

    switch (command.kind) {
      case 'insert_digit':
        mathfield.insert(String(command.digit), { selectionMode: 'after' });
        break;
      case 'insert_structure':
        if (command.structure === 'decimal') {
          mathfield.insert('.', { selectionMode: 'after' });
        } else {
          mathfield.insert(mathTemplateInsertLatex(command.structure), { selectionMode: 'placeholder' });
        }
        break;
      case 'move_left':
        mathfield.executeCommand('moveToPreviousChar');
        break;
      case 'move_right':
        mathfield.executeCommand('moveToNextChar');
        break;
      case 'delete_backward':
        if (deleteEmptyMathLiveStructureBackward(mathfield)) {
          // The helper performs a programmatic MathLive deletion, which does not
          // emit input here. Synchronize the resulting field value explicitly.
          void updateMathLiveAnswer(index, mathfield, mathfield.value);
        } else {
          mathfield.executeCommand('deleteBackward');
        }
        break;
      case 'delete_forward':
        mathfield.executeCommand('deleteForward');
        break;
      case 'clear':
        mathfield.executeCommand('deleteAll');
        break;
      case 'commit':
        void commitMathfield(index);
        break;
    }
  }, [commitMathfield, updateMathLiveAnswer]);

  const drainActionQueue = useCallback(async () => {
    while (true) {
      const pending = actionQueueRef.current;
      await pending;
      if (pending === actionQueueRef.current) return;
    }
  }, []);

  const grade = useCallback(async () => {
    if (!worksheet || worksheetPhaseRef.current !== 'editing') return;

    // Lock synchronously before the first await. This closes the same-tick
    // double-click/keyboard window that React state alone cannot close.
    transitionWorksheetPhase('grading');
    inputEnabledRef.current = false;
    selectedIndexRef.current = null;
    setSelectedIndex(null);
    for (const mathfield of mathfieldRefs.current.values()) {
      mathfield.readOnly = true;
      mathfield.blur();
    }

    const stoppedAt = finishedAt ?? Date.now();
    setFinishedAt(stoppedAt);
    setNow(stoppedAt);
    setBusy(true);
    setError(null);
    try {
      await drainActionQueue();
      const latestAnswers = answersRef.current;
      const result = await engine.gradeAnswer({
        schema_version: DRILL_SCHEMA_VERSION,
        worksheet,
        answers: worksheet.problems.map((problem) => ({
          problem_id: problem.problem_id,
          answer: latestAnswers[problem.problem_id] ?? ({ type: 'empty' } satisfies AnswerNode),
        })),
      });
      setGradeResult(result);
      transitionWorksheetPhase('graded');
    } catch (value) {
      // A failed grade attempt returns to the only editable state. Preserve the
      // elapsed time at the instant grading started instead of charging the
      // user for a failed engine request.
      const resumedAt = Date.now();
      if (startedAt !== null) {
        const frozenElapsed = Math.max(0, stoppedAt - startedAt);
        setStartedAt(resumedAt - frozenElapsed);
      }
      setFinishedAt(null);
      setNow(resumedAt);
      transitionWorksheetPhase('editing');
      for (const mathfield of mathfieldRefs.current.values()) mathfield.readOnly = false;
      showEngineError(value);
    } finally {
      selectedIndexRef.current = null;
      setBusy(false);
    }
  }, [drainActionQueue, engine, finishedAt, showEngineError, startedAt, transitionWorksheetPhase, worksheet]);

  const returnToProblems = useCallback(() => {
    if (worksheetPhaseRef.current !== 'graded') return;
    const resumedAt = Date.now();
    const frozenElapsed = startedAt === null || finishedAt === null ? 0 : Math.max(0, finishedAt - startedAt);
    setStartedAt(resumedAt - frozenElapsed);
    setFinishedAt(null);
    setNow(resumedAt);
    setGradeResult(null);
    transitionWorksheetPhase('editing');
    setSelectedIndex(null);
    selectedIndexRef.current = null;
    inputEnabledRef.current = false;
    setError(null);
    dismissNotice();
  }, [dismissNotice, finishedAt, startedAt, transitionWorksheetPhase]);

  const retryWorksheet = useCallback(() => {
    if (worksheetPhaseRef.current !== 'graded' || !worksheet || !worksheetMetadata) return;
    installWorksheet(worksheet, worksheetMetadata);
    setError(null);
    dismissNotice();
  }, [dismissNotice, installWorksheet, worksheet, worksheetMetadata]);

  const generateDifferentWorksheet = useCallback(async () => {
    if (worksheetPhaseRef.current !== 'graded') return;
    // Replacing is a worksheet phase, not just a generic loading flag. Lock it
    // synchronously so stale result-panel actions cannot launch a second
    // replacement before React has painted disabled buttons.
    transitionWorksheetPhase('replacing');
    setBusy(true);
    setError(null);
    dismissNotice();
    try {
      const seed = seedGenerator();
      const metadata = createWorksheetMetadata(seed, dateGenerator());
      const generatedWorksheet = await engine.generateWorksheet({ ...settings, seed });
      installWorksheet({ ...generatedWorksheet, seed }, metadata);
    } catch (value) {
      transitionWorksheetPhase('graded');
      showEngineError(value);
    } finally {
      setBusy(false);
    }
  }, [dateGenerator, dismissNotice, engine, installWorksheet, seedGenerator, settings, showEngineError, transitionWorksheetPhase]);

  const backToTop = useCallback(() => {
    if (worksheetPhaseRef.current === 'grading' || worksheetPhaseRef.current === 'replacing') return;
    setScreen('settings');
    setSelectedIndex(null);
    selectedIndexRef.current = null;
    inputEnabledRef.current = false;
    setStartedAt(null);
    setFinishedAt(null);
    setGradeResult(null);
    transitionWorksheetPhase('editing');
    setError(null);
    dismissNotice();
  }, [dismissNotice, transitionWorksheetPhase]);

  return (
    <FuriganaContext.Provider value={furiganaEnabled}>
      <main className="app-shell">
        {screen === 'settings' ? (
          <SettingsScreen
            settings={settings}
            busy={busy}
            busyAction={settingsBusyAction}
            error={error}
            hasWorksheet={Boolean(worksheet)}
            worksheetMetadata={worksheetMetadata}
            curriculumMode={curriculumMode}
            webSettings={webSettings}
            furiganaEnabled={furiganaEnabled}
            onSettingsChange={changeSettings}
            onCurriculumModeChange={changeCurriculumMode}
            onThemeChange={changeTheme}
            onDifficultyChange={changeDifficulty}
            onFuriganaChange={changeFurigana}
            onGenerate={() => void generate(false)}
            onPrint={() => void generate(true)}
          />
        ) : worksheet && worksheetUi ? (
          <WorksheetScreen
            worksheetUi={worksheetUi}
            worksheet={worksheet}
            worksheetMetadata={worksheetMetadata}
            answers={answers}
            selectedIndex={selectedIndex}
            elapsed={formatElapsed(startedAt, finishedAt ?? now)}
            gradeResult={gradeResult}
            worksheetPhase={worksheetPhase}
            busy={busy}
            error={error}
            notice={notice}
            onSelect={selectProblem}
            onCommand={applyMathCommand}
            onRegisterMathfield={registerMathfield}
            onMathInput={(index, mathfield, latex) => void updateMathLiveAnswer(index, mathfield, latex)}
            onCommit={(index) => void commitMathfield(index)}
            onGrade={() => void grade()}
            onReturnToProblems={returnToProblems}
            onRetryWorksheet={retryWorksheet}
            onDifferentWorksheet={() => void generateDifferentWorksheet()}
            onPrint={() => {
              void openWorksheetPdfLazy(worksheet, undefined, worksheetMetadata ?? undefined).catch(showEngineError);
            }}
            onBack={backToTop}
          />
        ) : null}
      </main>
    </FuriganaContext.Provider>
  );
}

type SettingsScreenProps = {
  settings: DrillSettings;
  busy: boolean;
  busyAction: SettingsBusyAction;
  error: string | null;
  hasWorksheet: boolean;
  worksheetMetadata: WorksheetMetadata | null;
  curriculumMode: CurriculumMode;
  webSettings: WebDrillSettings;
  furiganaEnabled: boolean;
  onSettingsChange: (settings: DrillSettings) => void;
  onCurriculumModeChange: (mode: CurriculumMode) => void;
  onThemeChange: (theme: CurriculumTheme) => void;
  onDifficultyChange: (difficulty: DifficultyLevel) => void;
  onFuriganaChange: (enabled: boolean) => void;
  onGenerate: () => void;
  onPrint: () => void;
};

function SettingsScreen({
  settings,
  busy,
  busyAction,
  error,
  hasWorksheet,
  worksheetMetadata,
  curriculumMode,
  webSettings,
  furiganaEnabled,
  onSettingsChange,
  onCurriculumModeChange,
  onThemeChange,
  onDifficultyChange,
  onFuriganaChange,
  onGenerate,
  onPrint,
}: SettingsScreenProps) {
  const selection = findCurriculumSelection(webSettings.themeKey);
  const genres = curriculumMode === 'recommended' ? RECOMMENDED_GENRES : selection.grade.genres;
  const activeGenre = curriculumMode === 'recommended'
    ? genres.find((genre) => genre.themes.some((theme) => theme.themeKey === webSettings.themeKey)) ?? genres[0]!
    : selection.genre;
  const unavailable = !selection.theme.implemented;

  const selectGrade = (gradeSlug: string) => {
    const grade = CURRICULUM_TREE.find((candidate) => candidate.slug === gradeSlug) ?? CURRICULUM_TREE[0]!;
    onThemeChange(grade.genres[0]!.themes[0]!);
  };

  const selectGenre = (genreKey: string) => {
    const genre = genres.find((candidate) => candidate.genreKey === genreKey) ?? genres[0]!;
    onThemeChange(genre.themes[0]!);
  };

  const selectTheme = (themeKey: string) => {
    const genre = genres.find((candidate) => candidate.genreKey === activeGenre.genreKey) ?? genres[0]!;
    const theme = genre.themes.find((candidate) => candidate.themeKey === themeKey) ?? genre.themes[0]!;
    onThemeChange(theme);
  };

  const busyStatusText = busyAction === 'generate'
    ? '問題を生成しています。しばらくお待ちください。'
    : busyAction === 'print'
      ? '印刷用PDFを準備しています。しばらくお待ちください。'
      : null;

  return (
    <section className="settings-screen" aria-labelledby="settings-title">
      <div className="lobby-decoration" aria-hidden="true">
        <span className="lobby-shape lobby-shape-square" />
        <span className="lobby-shape lobby-shape-circle" />
        <span className="lobby-shape lobby-shape-triangle" />
      </div>

      <div className="lobby-panel" aria-busy={busy}>
        <header className="page-heading">
          <label className="furigana-toggle">
            <input type="checkbox" checked={furiganaEnabled} onChange={(event) => onFuriganaChange(event.target.checked)} />
            <span>ふりがな</span>
          </label>
          <h1 id="settings-title" aria-label="計算ドリルをつくる"><RubyMessage text="計算ドリルをつくる" /></h1>
        </header>

        <div className="settings-card">
          <div className="selection-mode-tabs" aria-label="選び方">
            <button type="button" aria-pressed={curriculumMode === 'recommended'} onClick={() => onCurriculumModeChange('recommended')}>おすすめ</button>
            <button type="button" aria-label="学年から選ぶ" aria-pressed={curriculumMode === 'grade'} onClick={() => onCurriculumModeChange('grade')}><RubyMessage text="学年から選ぶ" /></button>
          </div>

          <div className={`curriculum-fields ${curriculumMode === 'recommended' ? 'curriculum-fields-recommended' : ''}`} aria-label="出題範囲">
            {curriculumMode === 'grade' ? (
              <div className="field-group">
                <div className="field-label"><RubyMessage text="学年" /></div>
                <CustomSelect
                  id="grade-select"
                  ariaLabel="学年"
                  value={selection.grade.slug}
                  options={CURRICULUM_TREE.map((grade) => ({ value: grade.slug, label: grade.label }))}
                  onChange={selectGrade}
                  renderLabel={(option) => <RubyMessage text={option.label} />}
                />
              </div>
            ) : null}
            <div className="field-group">
              <div className="field-label">ジャンル</div>
              <CustomSelect
                id="genre-select"
                ariaLabel="ジャンル"
                value={activeGenre.genreKey}
                options={genres.map((genre) => ({ value: genre.genreKey, label: genre.label }))}
                onChange={selectGenre}
                renderLabel={(option) => <RubyMessage text={option.label} />}
              />
            </div>

            <div className="field-group field-group-theme">
              <div className="field-label">テーマ</div>
              <CustomSelect
                id="theme-select"
                ariaLabel="テーマ"
                value={selection.theme.themeKey}
                options={activeGenre.themes.map((theme) => ({ value: theme.themeKey, label: theme.label }))}
                onChange={selectTheme}
                renderLabel={(option) => <RubyMessage text={option.label} />}
              />
            </div>
          </div>

          <div className="settings-options">
            <div className="field-group">
              <div className="field-label"><RubyMessage text="難易度" /></div>
              <CustomSelect
                id="difficulty-select"
                ariaLabel="難易度"
                value={String(webSettings.difficulty)}
                options={DIFFICULTY_OPTIONS.map((option) => ({ value: String(option.value), label: option.label }))}
                onChange={(value) => {
                  const next = DIFFICULTY_OPTIONS.find((option) => String(option.value) === value);
                  if (next) onDifficultyChange(next.value);
                }}
                renderLabel={(option) => <RubyMessage text={option.label} />}
              />
            </div>

            <div className="fixed-count" aria-label={`問題数${selection.theme.problemCount ?? 0}問`}>
              <span><RubyMessage text="問題数" /></span>
              <strong>{selection.theme.problemCount ?? '—'}<span><RubyMessage text="問" /></span></strong>
            </div>
          </div>

          <details className="advanced-settings">
            <summary>
              <RubyMessage text="詳細設定" />
              <svg className="advanced-settings-chevron" viewBox="0 0 12 8" aria-hidden="true"><path d="M1 1.5 6 6.5 11 1.5" /></svg>
            </summary>
            <div className="advanced-settings-body">
              <div className="seed-field">
                <label className="field-label seed-label" htmlFor="seed-input">
                  Seed <span>(<RubyMessage text="同じSeedでは同じ問題が生成されます。" />)</span>
                </label>
                <div className="ruby-input">
                  <input
                    id="seed-input"
                    className="text-field"
                    aria-label="Seed"
                    aria-placeholder="空欄なら毎回自動生成"
                    value={settings.seed}
                    onChange={(event) => onSettingsChange({ ...settings, seed: event.target.value })}
                    autoComplete="off"
                    spellCheck={false}
                  />
                  {settings.seed === '' ? <span className="ruby-input-placeholder" aria-hidden="true"><RubyMessage text="空欄なら毎回自動生成" /></span> : null}
                </div>
              </div>
            </div>
          </details>
        </div>

        {unavailable ? <p className="unavailable-message" role="status" aria-label="このテーマはまだ利用できません"><RubyMessage text="このテーマはまだ利用できません" /></p> : null}
        {error ? <p className="error-message" role="alert" aria-label={error}><RubyMessage text={error} /></p> : null}
        {hasWorksheet && worksheetMetadata ? (
          <p className="muted-message" data-testid="last-worksheet-metadata">
            <RubyMessage text="前回" />: {formatWorksheetFooter(worksheetMetadata)}
          </p>
        ) : null}

        <div className="settings-actions">
          <button type="button" className="primary-button" aria-label={busyAction === 'generate' ? '問題を生成中…' : '問題生成'} disabled={busy || unavailable} onClick={onGenerate}>
            <span className="button-icon" aria-hidden="true">▶</span>
            <RubyMessage text={busyAction === 'generate' ? '問題を生成中…' : '問題生成'} />
          </button>
          <button type="button" className="secondary-button" aria-label={busyAction === 'print' ? 'PDFを準備中…' : '印刷 (pdfで出力)'} disabled={busy || unavailable} onClick={onPrint}>
            <svg className="share-pdf-icon" viewBox="0 0 24 24" aria-hidden="true">
              <path d="M12 15V3" />
              <path d="m8 7 4-4 4 4" />
              <path d="M5 11v9h14v-9" />
            </svg>
            <RubyMessage text={busyAction === 'print' ? 'PDFを準備中…' : '印刷 (pdfで出力)'} />
          </button>
        </div>
        {busyStatusText ? (
          <p className="sr-only" aria-label={busyStatusText} aria-live="polite">
            <RubyMessage text={busyStatusText} />
          </p>
        ) : null}
      </div>
      <p className="settings-version" aria-label={AUTODRILL_VERSION_LABEL}>{AUTODRILL_VERSION_LABEL}</p>
    </section>
  );
}

type WorksheetScreenProps = {
  worksheetUi: WorksheetUiComponents;
  worksheet: WorksheetDto;
  worksheetMetadata: WorksheetMetadata | null;
  answers: Record<string, AnswerNode>;
  selectedIndex: number | null;
  elapsed: string;
  gradeResult: GradeResult | null;
  worksheetPhase: WorksheetPhase;
  busy: boolean;
  error: string | null;
  notice: string | null;
  onSelect: (index: number) => void;
  onCommand: (command: MathInputCommand) => void;
  onRegisterMathfield: (index: number, mathfield: AutoDrillMathfield | null) => void;
  onMathInput: (index: number, mathfield: AutoDrillMathfield, latex: string) => void;
  onCommit: (index: number) => void;
  onGrade: () => void;
  onReturnToProblems: () => void;
  onRetryWorksheet: () => void;
  onDifferentWorksheet: () => void;
  onPrint: () => void;
  onBack: () => void;
};

function WorksheetScreen({ worksheetUi, worksheet, worksheetMetadata, answers, selectedIndex, elapsed, gradeResult, worksheetPhase, busy, error, notice, onSelect, onCommand, onRegisterMathfield, onMathInput, onCommit, onGrade, onReturnToProblems, onRetryWorksheet, onDifferentWorksheet, onPrint, onBack }: WorksheetScreenProps) {
  const { MathTemplateIcon, ProblemExpression } = worksheetUi;
  const sharedLayout = buildSharedWorksheetLayout(worksheet);
  const worksheetTheme = findImplementedThemeByNumericId(worksheet.identity.numeric_theme_id) ?? ONE_DIGIT_ADDITION_THEME;
  const gradeBandClass = worksheetGradeBandClass(worksheetTheme.grade.slug);
  const selectedProblem = worksheetPhase === 'editing' && selectedIndex !== null ? worksheet.problems[selectedIndex] : null;
  const selectedCapabilities = selectedProblem ? inputCapabilities(selectedProblem.input_interface) : null;
  const visibleStructures = selectedCapabilities?.allowed_structures.filter(
    (structure): structure is Exclude<AnswerInputStructure, 'decimal'> => structure !== 'decimal',
  ) ?? [];
  if (selectedCapabilities?.allow_negative && !visibleStructures.includes('negative')) {
    visibleStructures.push('negative');
  }
  const resultById = new Map((gradeResult?.items ?? []).map((item) => [item.problem_id, item]));
  const toPagePercent = (value: number, total: number) => `${(value / total) * 100}%`;
  const contentTop = A4_PAGE.margin + A4_PAGE.headerHeight;
  const contentHeight = A4_PAGE.height - A4_PAGE.margin * 2 - A4_PAGE.headerHeight - A4_PAGE.footerHeight;
  const dividerStyle: CSSProperties = {
    left: toPagePercent(sharedLayout.dividerX, A4_PAGE.width),
    top: toPagePercent(contentTop, A4_PAGE.height),
    height: toPagePercent(contentHeight, A4_PAGE.height),
  };
  const footerStyle: CSSProperties = {
    right: toPagePercent(A4_PAGE.margin, A4_PAGE.width),
    bottom: toPagePercent(A4_PAGE.margin, A4_PAGE.height),
  };

  return (
    <section className={`worksheet-screen ${selectedProblem ? 'worksheet-input-open' : ''}`} aria-labelledby="worksheet-title">
      <div className="ribbon">
        <div>
          <p className="ribbon-label" aria-label={worksheetTheme.grade.label}><RubyMessage text={worksheetTheme.grade.label} /></p>
          <h1 id="worksheet-title">{worksheetTheme.worksheet.title}</h1>
        </div>
        <div className="ribbon-meta"><span><RubyMessage text="回答時間" /></span><strong data-testid="elapsed-time">{elapsed}</strong></div>
        <button type="button" className="ribbon-button" aria-label="採点" aria-pressed={worksheetPhase !== 'editing'} data-grade-state={worksheetPhase} onClick={onGrade} disabled={busy || worksheetPhase !== 'editing'}><RubyMessage text="採点" /></button>
        <button type="button" className="ribbon-icon" onClick={onPrint} aria-label="印刷" disabled={busy}><RubyMessage text="印刷" /></button>
        <button type="button" className="ribbon-link" aria-label="TOPに戻る" onClick={onBack} disabled={busy || worksheetPhase === 'grading' || worksheetPhase === 'replacing'}><RubyMessage text="TOPに戻る" /></button>
      </div>

      {notice ? <div className="worksheet-toast" role="status" aria-label={notice} aria-live="polite" aria-atomic="true"><RubyMessage text={notice} /></div> : null}

      {error ? <p className="error-message worksheet-error" role="alert" aria-label={error}><RubyMessage text={error} /></p> : null}
      {gradeResult ? (
        <div className="grade-result-panel">
          <div className="grade-summary" role="status"><strong>{gradeResult.correct_count} / {gradeResult.total_count}</strong><span><RubyMessage text="正解" /></span></div>
          <div className="grade-actions" aria-label="採点後の操作">
            <button type="button" aria-label="問題に戻る" onClick={onReturnToProblems} disabled={busy}><RubyMessage text="問題に戻る" /></button>
            <button type="button" aria-label="もう一回問題を解く" onClick={onRetryWorksheet} disabled={busy}><RubyMessage text="もう一回問題を解く" /></button>
            <button type="button" aria-label="別の問題を解く" onClick={onDifferentWorksheet} disabled={busy}><RubyMessage text="別の問題を解く" /></button>
          </div>
        </div>
      ) : null}

      <div className="paper-wrap">
        <article className={`paper ${gradeBandClass}`} style={{ aspectRatio: `${A4_PAGE.width} / ${A4_PAGE.height}` }} aria-label={`${worksheet.layout.problem_count}問の${worksheetTheme.worksheet.title}ワークシート`}>
          <div className="problem-grid">
            {worksheetTheme.worksheet.instruction ? (
              <p className="worksheet-instruction">{worksheetTheme.worksheet.instruction}</p>
            ) : null}
            <div className="problem-divider" data-testid="problem-divider" style={dividerStyle} />
            {sharedLayout.cells.map((cell) => {
              const { problem, index } = cell;
              const answer = answers[problem.problem_id] ?? ({ type: 'empty' } satisfies AnswerNode);
              const isSelected = selectedIndex === index;
              const result = resultById.get(problem.problem_id);
              const position = getCellTopPosition(sharedLayout, cell);
              const isLinearEquation = problem.prompt.kind === 'linear_equation';
              const cellStyle: CSSProperties = {
                left: toPagePercent(position.x, A4_PAGE.width),
                top: toPagePercent(position.y, A4_PAGE.height),
                width: toPagePercent(position.width, A4_PAGE.width),
                height: toPagePercent(position.height, A4_PAGE.height),
              };
              return (
                <div className={`problem-cell ${isLinearEquation ? 'problem-cell-linear-equation' : ''} ${result ? 'problem-cell-graded' : ''}`} data-layout-index={index} data-layout-column={cell.column} data-problem-index={index} data-testid={`problem-cell-${index}`} style={cellStyle} key={problem.problem_id}>
                  <span className="problem-number">{index + 1}.</span>
                  <span className="expression"><ProblemExpression problem={problem} /></span>
                  <WorksheetAnswerField
                    worksheetUi={worksheetUi}
                    problem={problem}
                    index={index}
                    answer={answer}
                    isSelected={isSelected}
                    result={result}
                    gradeResult={gradeResult}
                    inputLocked={worksheetPhase !== 'editing'}
                    answerPrefix={worksheetTheme.worksheet.answerPrefix}
                    onSelect={onSelect}
                    onRegisterMathfield={onRegisterMathfield}
                    onMathInput={onMathInput}
                    onCommit={onCommit}
                  />
                </div>
              );
            })}
            {worksheetMetadata ? (
              <div className="worksheet-footer" data-testid="worksheet-footer" style={footerStyle}>
                {formatWorksheetFooter(worksheetMetadata)}
              </div>
            ) : null}
          </div>
        </article>
      </div>

      {selectedProblem ? (
        <div className="input-panel" aria-label="数式入力パネル">
          <div className="input-panel-inner">
            {visibleStructures.length > 0 ? (
              <div className="formula-keypad" aria-label="数式テンプレート">
                {visibleStructures.map((structure) => {
                  const label = STRUCTURE_LABELS[structure];
                  return (
                    <button
                      type="button"
                      key={structure}
                      onClick={() => onCommand({ kind: 'insert_structure', structure })}
                      disabled={busy}
                      aria-label={label}
                      title={label}
                    >
                      <span className="formula-key-symbol" aria-hidden="true"><MathTemplateIcon structure={structure} /></span>
                      <span className="formula-key-label"><RubyMessage text={label} /></span>
                    </button>
                  );
                })}
              </div>
            ) : null}
            <div className="keypad-numbers" aria-label="数字キー">
              {[7, 8, 9, 4, 5, 6, 1, 2, 3, 0].map((digit) => (
                <button
                  type="button"
                  className={digit === 0 ? 'keypad-zero' : undefined}
                  key={digit}
                  onClick={() => onCommand({ kind: 'insert_digit', digit })}
                  disabled={busy}
                >
                  {digit}
                </button>
              ))}
              {selectedCapabilities?.allow_decimal ? (
                <button
                  type="button"
                  className="keypad-decimal"
                  onClick={() => onCommand({ kind: 'insert_structure', structure: 'decimal' })}
                  disabled={busy}
                  aria-label="小数点"
                >
                  .
                </button>
              ) : null}
            </div>
            <div className="keypad-controls" aria-label="編集キー">
              <button type="button" onClick={() => onCommand({ kind: 'move_left' })} disabled={busy} aria-label="カーソルを左へ">←</button>
              <button type="button" onClick={() => onCommand({ kind: 'move_right' })} disabled={busy} aria-label="カーソルを右へ">→</button>
              <button type="button" onClick={() => onCommand({ kind: 'delete_backward' })} disabled={busy} aria-label="一文字戻す">⌫</button>
              <button type="button" className="keypad-clear" onClick={() => onCommand({ kind: 'clear' })} disabled={busy}>クリア</button>
              <button type="button" className="keypad-commit" aria-label="確定" onClick={() => onCommand({ kind: 'commit' })} disabled={busy}><RubyMessage text="確定" /></button>
            </div>
          </div>
        </div>
      ) : null}
    </section>
  );
}
