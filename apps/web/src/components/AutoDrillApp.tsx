'use client';

import { createContext, useCallback, useContext, useEffect, useRef, useState, type CSSProperties } from 'react';
import { flushSync } from 'react-dom';
import { createRoot } from 'react-dom/client';

import {
  DEFAULT_DRILL_SETTINGS,
  DRILL_SCHEMA_VERSION,
  DrillEngineError,
  emptyEditorState,
  editorValue,
  answerNodeText,
  inputCapabilities,
  isEditorActionAllowed,
  type AnswerInputInterface,
  type AnswerInputStructure,
  type AnswerNode,
  type DrillEngine,
  type DrillSettings,
  type EditorAction,
  type EditorState,
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
import { MathTemplateIcon } from '@/components/MathTemplateIcon';
import { openWorksheetPdf } from '@/pdf/worksheet-pdf';
import { ProblemExpression } from '@/components/ProblemExpression';
import { createWasmDrillEngine } from '@/domain/wasm-adapter';
import { loadGeneratedWasmRuntime } from '@/wasm/load-generated';
import { A4_PAGE, buildSharedWorksheetLayout, getCellTopPosition } from '@/domain/layout';
import { generateAutomaticSeed } from '@/domain/seed';
import {
  createWorksheetMetadata,
  formatWorksheetFooter,
  type WorksheetDateGenerator,
  type WorksheetMetadata,
} from '@/domain/worksheet-metadata';

type Screen = 'settings' | 'worksheet';
type SettingsBusyAction = 'generate' | 'print' | null;
const FURIGANA_STORAGE_KEY = 'autodrill:furigana-enabled';
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
  '一桁の足し算': [["一桁", "ひとけた"], 'の', ["足", "た"], 'し', ["算", "ざん"]],
  '難易度': [["難易度", "なんいど"]],
  'このテーマはまだ利用できません': ['このテーマはまだ', ["利用", "りよう"], 'できません'],
  '問題数': [["問題数", "もんだいすう"]],
  '問': [["問", "もん"]],
  '任意': [["任意", "にんい"]],
  '空欄なら毎回自動生成': [["空欄", "くうらん"], 'なら', ["毎回", "まいかい"], ["自動生成", "じどうせいせい"]],
  '同じSeedで同じ問題を再現できます。空欄なら毎回新しく生成します。': [["同", "おな"], 'じSeedで', ["同", "おな"], 'じ', ["問題", "もんだい"], 'を', ["再現", "さいげん"], 'できます。', ["空欄", "くうらん"], 'なら', ["毎回", "まいかい"], ["新", "あたら"], 'しく', ["生成", "せいせい"], 'します。'],
  '前回': [["前回", "ぜんかい"]],
  '問題生成': [["問題生成", "もんだいせいせい"]],
  '問題を生成中…': [["問題", "もんだい"], 'を', ["生成中", "せいせいちゅう"], '…'],
  '印刷': [["印刷", "いんさつ"]],
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

function editorActionForKey(event: KeyboardEvent, inputInterface: AnswerInputInterface): EditorAction | null {
  if (event.isComposing || event.altKey || event.ctrlKey || event.metaKey) return null;
  let action: EditorAction | null = null;
  if (event.key >= '0' && event.key <= '9') {
    action = { kind: 'insert_digit', digit: Number(event.key) };
  }
  else if (event.key === 'Enter') action = { kind: 'commit' };
  else if (event.key === 'Backspace') action = { kind: 'delete_backward' };
  else if (event.key === 'Delete') action = { kind: 'delete_forward' };
  else if (event.key === 'ArrowLeft') action = { kind: 'move_left' };
  else if (event.key === 'ArrowRight') action = { kind: 'move_right' };
  else if (event.key === '.') action = { kind: 'insert_structure', structure: 'decimal' };
  else if (event.key === '/') action = { kind: 'insert_structure', structure: 'fraction' };
  else if (event.key === '-') action = { kind: 'insert_structure', structure: 'negative' };
  else if (event.key === '+') action = { kind: 'insert_structure', structure: 'plus_minus' };
  else if (event.key === ',') action = { kind: 'insert_structure', structure: 'tuple' };
  return action && isEditorActionAllowed(inputInterface, action) ? action : null;
}

function answerFontSize(digitCount: number): number {
  if (digitCount <= 2) return 20;
  return Math.max(11, 20 - (digitCount - 2) * 0.5625);
}

function answerNodeSize(answer: AnswerNode): number {
  switch (answer.type) {
    case 'empty': return 0;
    case 'integer': return answer.value.replace('-', '').length;
    case 'exact_decimal': return Math.max(answer.value.coefficient.replace('-', '').length, answer.value.scale + 1);
    case 'nan_error': return Math.max(1, [...answer.value].length);
    case 'fraction': return 1 + answerNodeSize(answer.value.numerator) + answerNodeSize(answer.value.denominator);
    case 'mixed_fraction': return 1 + answerNodeSize(answer.value.whole) + answerNodeSize(answer.value.numerator) + answerNodeSize(answer.value.denominator);
    case 'root': return 1 + answerNodeSize(answer.value.radicand) + (answer.value.index ? answerNodeSize(answer.value.index) : 0);
    case 'negative':
    case 'plus_minus': return 1 + answerNodeSize(answer.value);
    case 'tuple': return 1 + answer.value.reduce((total, item) => total + answerNodeSize(item), 0);
    case 'variable': return Math.max(1, [...answer.value].length);
  }
}

function pathsEqual(left: readonly number[], right: readonly number[]): boolean {
  return left.length === right.length && left.every((value, index) => value === right[index]);
}

function answerSlotLabel(answer: AnswerNode, path: readonly number[]): string {
  let node = answer;
  const labels: string[] = [];
  for (const index of path) {
    switch (node.type) {
      case 'fraction':
        if (index === 0) { labels.push('分子'); node = node.value.numerator; }
        else if (index === 1) { labels.push('分母'); node = node.value.denominator; }
        else return '答え';
        break;
      case 'mixed_fraction':
        if (index === 0) { labels.push('整数部分'); node = node.value.whole; }
        else if (index === 1) { labels.push('分子'); node = node.value.numerator; }
        else if (index === 2) { labels.push('分母'); node = node.value.denominator; }
        else return '答え';
        break;
      case 'root':
        if (index === 0) { labels.push('ルートの中'); node = node.value.radicand; }
        else if (index === 1 && node.value.index) { labels.push('指数'); node = node.value.index; }
        else return '答え';
        break;
      case 'negative':
      case 'plus_minus':
        if (index !== 0) return '答え';
        node = node.value;
        break;
      case 'tuple':
        if (!node.value[index]) return '答え';
        labels.push(`${index + 1}個目の解`);
        node = node.value[index];
        break;
      case 'empty':
      case 'integer':
      case 'exact_decimal':
      case 'nan_error':
      case 'variable': return '答え';
    }
  }
  return labels.length > 0 ? labels.join('の') : '答え';
}

type MathMlEditorContext = {
  state: EditorState;
  selected: boolean;
  testIdPrefix: number;
  onSelectSlot: (path: readonly number[], cursor: number) => void;
};

type MathMlAnswerNodeProps = {
  node: AnswerNode;
  path: readonly number[];
  editor?: MathMlEditorContext;
};

/**
 * Recursive AnswerNode renderer for the Web worksheet. Mathematical layout is
 * delegated entirely to native MathML; CSS only marks editing state.
 */
function MathMlAnswerNode({ node, path, editor }: MathMlAnswerNodeProps) {
  const child = (value: AnswerNode, index: number) => (
    <MathMlAnswerNode node={value} path={[...path, index]} editor={editor} />
  );

  if (node.type === 'empty' || node.type === 'integer' || node.type === 'exact_decimal' || node.type === 'nan_error') {
    const text = answerNodeText(node);
    if (!editor) return <mtext>{text}</mtext>;

    const active = editor.selected && pathsEqual(path, editor.state.active_path);
    const characters = [...text];
    const cursor = Math.min(editor.state.cursor, characters.length);
    return (
      <mrow
        className={`answer-math-slot ${active ? 'answer-math-slot-active' : ''} ${text === '' ? 'answer-math-slot-empty' : ''}`}
        data-slot-path={path.join('.')}
        onClick={(event) => {
          event.stopPropagation();
          editor.onSelectSlot(path, characters.length);
        }}
      >
        {active ? (
          <>
            <mtext data-testid={`answer-before-caret-${editor.testIdPrefix}`}>{characters.slice(0, cursor).join('')}</mtext>
            <mpadded width="0" height="0" depth="0"><mtext className="answer-math-caret" data-testid={`answer-caret-${editor.testIdPrefix}`}>│</mtext></mpadded>
            <mtext data-testid={`answer-after-caret-${editor.testIdPrefix}`}>{characters.slice(cursor).join('')}</mtext>
          </>
        ) : <mtext>{text || (path.length === 0 ? '' : '□')}</mtext>}
      </mrow>
    );
  }

  switch (node.type) {
    case 'fraction':
      return <mfrac>{child(node.value.numerator, 0)}{child(node.value.denominator, 1)}</mfrac>;
    case 'mixed_fraction':
      return <mrow>{child(node.value.whole, 0)}<mfrac>{child(node.value.numerator, 1)}{child(node.value.denominator, 2)}</mfrac></mrow>;
    case 'root':
      return node.value.index
        ? <mroot>{child(node.value.radicand, 0)}{child(node.value.index, 1)}</mroot>
        : <msqrt>{child(node.value.radicand, 0)}</msqrt>;
    case 'negative':
      return <mrow><mo>−</mo>{child(node.value, 0)}</mrow>;
    case 'plus_minus':
      return <mrow><mo>±</mo>{child(node.value, 0)}</mrow>;
    case 'tuple':
      return <mrow>{node.value.map((value, index) => <mrow key={index}>{index > 0 ? <mo>,</mo> : null}{child(value, index)}</mrow>)}</mrow>;
    case 'variable':
      return <mi>{node.value}</mi>;
  }
}

type StructuredAnswerProps = {
  node: AnswerNode;
  path: readonly number[];
  state: EditorState;
  selected: boolean;
  testIdPrefix: number;
  onSelectSlot: (path: readonly number[], cursor: number) => void;
};

function StructuredAnswer({ node, path, state, selected, testIdPrefix, onSelectSlot }: StructuredAnswerProps) {
  return (
    <math className="answer-math" aria-label={answerNodeText(node)}>
      <MathMlAnswerNode
        node={node}
        path={path}
        editor={{ state, selected, testIdPrefix, onSelectSlot }}
      />
    </math>
  );
}

function StaticMathAnswer({ node }: { node: AnswerNode }) {
  return (
    <math className="answer-math" aria-label={answerNodeText(node)}>
      <MathMlAnswerNode node={node} path={[]} />
    </math>
  );
}


type RenderedAnswerSize = {
  width: number;
  height: number;
};

const FALLBACK_MAX_RENDERED_ANSWER_WIDTH = 180;
const FALLBACK_MAX_RENDERED_ANSWER_HEIGHT = 80;
const MIN_RENDERED_ANSWER_WIDTH_LIMIT = 96;
const MIN_RENDERED_ANSWER_HEIGHT_LIMIT = 56;

function measureRenderedAnswer(state: EditorState): RenderedAnswerSize | null {
  if (typeof document === 'undefined' || !document.body) return null;
  const probe = document.createElement('div');
  probe.className = 'answer-render-probe';
  probe.style.fontSize = `${answerFontSize(answerNodeSize(state.answer))}px`;
  document.body.appendChild(probe);
  const root = createRoot(probe);
  try {
    flushSync(() => {
      root.render(
        <span className="answer-value">
          <StructuredAnswer
            node={state.answer}
            path={[]}
            state={state}
            selected={false}
            testIdPrefix={-1}
            onSelectSlot={() => undefined}
          />
        </span>,
      );
    });
    const value = probe.firstElementChild as HTMLElement | null;
    if (!value) return null;
    const rect = value.getBoundingClientRect();
    if (rect.width <= 0 || rect.height <= 0) return null;
    return { width: rect.width, height: rect.height };
  } finally {
    flushSync(() => root.unmount());
    probe.remove();
  }
}

function renderedAnswerFitsProblem(state: EditorState, problemIndex: number): boolean {
  const measured = measureRenderedAnswer(state);
  // jsdom and non-layout renderers report zero-sized boxes. Runtime browser
  // enforcement remains authoritative because only it has real CSS geometry.
  if (!measured) return true;
  const cell = document.querySelector<HTMLElement>(`[data-problem-index="${problemIndex}"]`);
  const cellRect = cell?.getBoundingClientRect();
  const maxWidth = cellRect && cellRect.width > 0
    ? Math.max(MIN_RENDERED_ANSWER_WIDTH_LIMIT, cellRect.width * 0.56)
    : FALLBACK_MAX_RENDERED_ANSWER_WIDTH;
  const maxHeight = cellRect && cellRect.height > 0
    ? Math.max(MIN_RENDERED_ANSWER_HEIGHT_LIMIT, cellRect.height * 0.72)
    : FALLBACK_MAX_RENDERED_ANSWER_HEIGHT;
  return measured.width <= maxWidth && measured.height <= maxHeight;
}

function actionCanGrowRenderedAnswer(action: EditorAction): boolean {
  return action.kind === 'insert_digit' || action.kind === 'insert_structure';
}

type WorksheetAnswerFieldProps = {
  problem: WorksheetDto['problems'][number];
  index: number;
  editor: EditorState;
  isSelected: boolean;
  result: GradeResult['items'][number] | undefined;
  gradeResult: GradeResult | null;
  answerPrefix: string | null;
  onSelect: (index: number) => void;
  onAction: (action: EditorAction) => void;
};

function WorksheetAnswerField({
  problem,
  index,
  editor,
  isSelected,
  result,
  gradeResult,
  answerPrefix,
  onSelect,
  onAction,
}: WorksheetAnswerFieldProps) {
  const answer = editorValue(editor) ?? '';
  const astSize = answerNodeSize(editor.answer);
  const structured = !['empty', 'integer', 'exact_decimal'].includes(editor.answer.type);
  const answerStyle: CSSProperties = {
    width: 'max-content',
    fontSize: answerFontSize(astSize),
    flexGrow: 0,
    flexShrink: 0,
  };
  const canonicalAnswer = answerNodeText(problem.canonical_answer);

  return (
    <span className="problem-answer-area">
      {answerPrefix ? <math className="answer-prefix-label" aria-label={answerPrefix}><mtext>{answerPrefix}</mtext></math> : null}
      <button
        type="button"
        className={`answer-box ${structured ? 'answer-box-structured' : ''} ${isSelected ? 'answer-box-selected' : ''} ${result ? (result.correct ? 'answer-box-correct' : 'answer-box-wrong') : ''}`}
        data-answer-length={astSize}
        style={answerStyle}
        onClick={() => onSelect(index)}
        disabled={Boolean(gradeResult)}
        aria-label={`${index + 1}番の答え ${answer || '未入力'}`}
      >
        <span className="answer-value" aria-hidden="true">
          <StructuredAnswer
            node={editor.answer}
            path={[]}
            state={editor}
            selected={isSelected}
            testIdPrefix={index}
            onSelectSlot={(path, cursor) => {
              onSelect(index);
              onAction({ kind: 'select_slot', path, cursor });
            }}
          />
        </span>
      </button>
      {result?.correct ? <span className="result-mark" aria-label="正解">○</span> : null}
      {result && !result.correct ? (
        <span className="correct-answer" aria-label={`正しい答え ${canonicalAnswer}`}>
          <StaticMathAnswer node={problem.canonical_answer} />
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
  const [answers, setAnswers] = useState<Record<string, EditorState>>({});
  const [selectedIndex, setSelectedIndex] = useState<number | null>(null);
  const [startedAt, setStartedAt] = useState<number | null>(null);
  const [finishedAt, setFinishedAt] = useState<number | null>(null);
  const [now, setNow] = useState(() => Date.now());
  const [gradeResult, setGradeResult] = useState<GradeResult | null>(null);
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
  const answersRef = useRef<Record<string, EditorState>>({});
  const selectedIndexRef = useRef<number | null>(null);
  const inputEnabledRef = useRef(false);
  const actionQueueRef = useRef(Promise.resolve());
  const noticeTimerRef = useRef<number | null>(null);
  const selectedTheme = findTheme(webSettings.themeKey) ?? ONE_DIGIT_ADDITION_THEME;

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
    const nextAnswers = Object.fromEntries(nextWorksheet.problems.map((problem) => [problem.problem_id, emptyEditorState()]));
    answersRef.current = nextAnswers;
    setWorksheet(nextWorksheet);
    setWorksheetMetadata(metadata);
    setAnswers(nextAnswers);
    setGradeResult(null);
    setSelectedIndex(null);
    selectedIndexRef.current = null;
    inputEnabledRef.current = false;
    const timerStart = Date.now();
    setStartedAt(timerStart);
    setFinishedAt(null);
    setNow(timerStart);
  }, []);

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
    const pdfTarget = printAfterGeneration && typeof window !== 'undefined' ? window.open('about:blank', '_blank') : null;
    try {
      const seed = settings.seed === '' ? seedGenerator() : settings.seed;
      const metadata = createWorksheetMetadata(seed, dateGenerator());
      const generatedWorksheet = await engine.generateWorksheet({ ...settings, seed });
      // The Rust DTO remains the source of the problems. The spread adds the
      // exact seed used by this UI invocation when a fixture/runtime returns a
      // stale or normalized seed string.
      const nextWorksheet = { ...generatedWorksheet, seed };
      if (printAfterGeneration) await openWorksheetPdf(nextWorksheet, pdfTarget, metadata);
      if (printAfterGeneration) {
        setWorksheet(nextWorksheet);
        setWorksheetMetadata(metadata);
        setScreen('settings');
      } else {
        installWorksheet(nextWorksheet, metadata);
        setScreen('worksheet');
      }
    } catch (value) {
      pdfTarget?.close();
      showEngineError(value);
    } finally {
      setBusy(false);
      setSettingsBusyAction(null);
    }
  }, [dateGenerator, dismissNotice, engine, installWorksheet, seedGenerator, selectedTheme, settings, showEngineError]);

  const selectProblem = useCallback((index: number) => {
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

  const applyAction = useCallback((action: EditorAction, requestedIndex?: number) => {
    const run = async () => {
      const index = requestedIndex ?? selectedIndexRef.current;
      if (!worksheet || index === null || !worksheet.problems[index]) return;
      const problem = worksheet.problems[index];
      if (!isEditorActionAllowed(problem.input_interface, action)) return;
      const problemId = problem.problem_id;
      const current = answersRef.current[problemId] ?? emptyEditorState();
      setBusy(true);
      setError(null);
      dismissNotice();
      try {
        const next = await engine.applyEditorAction(current, action, problem.input_interface);
        if (actionCanGrowRenderedAnswer(action) && !renderedAnswerFitsProblem(next, index)) {
          return;
        }
        const nextAnswers = { ...answersRef.current, [problemId]: next };
        answersRef.current = nextAnswers;
        setAnswers(nextAnswers);
        if (action.kind === 'commit' && index < worksheet.problems.length - 1) {
          selectedIndexRef.current = index + 1;
          if (inputEnabledRef.current) {
            setSelectedIndex(index + 1);
            scheduleProblemScroll(index, index + 1);
          }
        }
      } catch (value) {
        showEngineError(value);
      } finally {
        setBusy(false);
      }
    };

    // Hardware/keypad events can arrive before the previous WASM promise
    // resolves. A single FIFO chain keeps each action on the latest editor
    // snapshot and preserves digit-then-commit ordering.
    const queued = actionQueueRef.current.then(run, run);
    actionQueueRef.current = queued.then(() => undefined, () => undefined);
    return queued;
  }, [dismissNotice, engine, showEngineError, worksheet]);

  const drainActionQueue = useCallback(async () => {
    // Read the current tail before awaiting it. If an event enqueues another
    // action while the tail settles, loop once more so grading observes the
    // final committed editor state rather than an intermediate snapshot.
    while (true) {
      const pending = actionQueueRef.current;
      await pending;
      if (pending === actionQueueRef.current) return;
    }
  }, []);

  useEffect(() => {
    if (screen !== 'worksheet' || selectedIndex === null) return undefined;
    const onKeyDown = (event: KeyboardEvent) => {
      if (!inputEnabledRef.current) return;
      const problem = worksheet?.problems[selectedIndex];
      if (!problem) return;
      const action = editorActionForKey(event, problem.input_interface);
      if (!action) return;
      // Capture + preventDefault avoids Enter activating a focused keypad or
      // ribbon button in addition to committing the selected answer.
      event.preventDefault();
      void applyAction(action);
    };
    window.addEventListener('keydown', onKeyDown, true);
    return () => window.removeEventListener('keydown', onKeyDown, true);
  }, [applyAction, screen, selectedIndex, worksheet]);

  const grade = useCallback(async () => {
    if (!worksheet) return;
    const stoppedAt = finishedAt ?? Date.now();
    setFinishedAt(stoppedAt);
    setNow(stoppedAt);
    inputEnabledRef.current = false;
    setSelectedIndex(null);
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
          editor_state: latestAnswers[problem.problem_id] ?? emptyEditorState(),
        })),
      });
      setGradeResult(result);
    } catch (value) {
      showEngineError(value);
    } finally {
      selectedIndexRef.current = null;
      setBusy(false);
    }
  }, [drainActionQueue, engine, finishedAt, showEngineError, worksheet]);

  const returnToProblems = useCallback(() => {
    const resumedAt = Date.now();
    const frozenElapsed = startedAt === null || finishedAt === null ? 0 : Math.max(0, finishedAt - startedAt);
    setStartedAt(resumedAt - frozenElapsed);
    setFinishedAt(null);
    setNow(resumedAt);
    setGradeResult(null);
    setSelectedIndex(null);
    selectedIndexRef.current = null;
    inputEnabledRef.current = false;
    setError(null);
    dismissNotice();
  }, [dismissNotice, finishedAt, startedAt]);

  const retryWorksheet = useCallback(() => {
    if (!worksheet || !worksheetMetadata) return;
    installWorksheet(worksheet, worksheetMetadata);
    setError(null);
    dismissNotice();
  }, [dismissNotice, installWorksheet, worksheet, worksheetMetadata]);

  const generateDifferentWorksheet = useCallback(async () => {
    setBusy(true);
    setError(null);
    dismissNotice();
    try {
      const seed = seedGenerator();
      const metadata = createWorksheetMetadata(seed, dateGenerator());
      const generatedWorksheet = await engine.generateWorksheet({ ...settings, seed });
      installWorksheet({ ...generatedWorksheet, seed }, metadata);
    } catch (value) {
      showEngineError(value);
    } finally {
      setBusy(false);
    }
  }, [dateGenerator, dismissNotice, engine, installWorksheet, seedGenerator, settings, showEngineError]);

  const backToTop = useCallback(() => {
    setScreen('settings');
    setSelectedIndex(null);
    selectedIndexRef.current = null;
    inputEnabledRef.current = false;
    setStartedAt(null);
    setFinishedAt(null);
    setGradeResult(null);
    setError(null);
    dismissNotice();
  }, [dismissNotice]);

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
        ) : worksheet ? (
          <WorksheetScreen
            worksheet={worksheet}
            worksheetMetadata={worksheetMetadata}
            answers={answers}
            selectedIndex={selectedIndex}
            elapsed={formatElapsed(startedAt, finishedAt ?? now)}
            gradeResult={gradeResult}
            busy={busy}
            error={error}
            notice={notice}
            onSelect={selectProblem}
            onAction={(action) => void applyAction(action)}
            onGrade={() => void grade()}
            onReturnToProblems={returnToProblems}
            onRetryWorksheet={retryWorksheet}
            onDifferentWorksheet={() => void generateDifferentWorksheet()}
            onPrint={() => {
              void Promise.resolve(openWorksheetPdf(worksheet, undefined, worksheetMetadata ?? undefined)).catch(showEngineError);
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

  const statusText = busyAction === 'generate'
    ? '問題を生成しています。しばらくお待ちください。'
    : busyAction === 'print'
      ? '印刷用PDFを準備しています。しばらくお待ちください。'
      : '問題の生成・入力状態・採点は Rust/WASM が担当します。';

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
          <p className="eyebrow"><span aria-hidden="true" /> AutoDrill alpha 1.1</p>
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
                <label className="field-label" htmlFor="grade-select"><RubyMessage text="学年" /></label>
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
              <label className="field-label" htmlFor="genre-select">ジャンル</label>
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
              <label className="field-label" htmlFor="theme-select">テーマ</label>
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
              <label className="field-label" htmlFor="difficulty-select"><RubyMessage text="難易度" /></label>
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

          <div className="seed-field">
            <label className="field-label" htmlFor="seed-input">Seed <span><RubyMessage text="任意" /></span></label>
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
            <p className="field-note" aria-label="同じSeedで同じ問題を再現できます。空欄なら毎回新しく生成します。"><RubyMessage text="同じSeedで同じ問題を再現できます。空欄なら毎回新しく生成します。" /></p>
          </div>
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
          <button type="button" className="secondary-button" aria-label={busyAction === 'print' ? 'PDFを準備中…' : '印刷'} disabled={busy || unavailable} onClick={onPrint}>
            <span className="button-icon" aria-hidden="true">▣</span>
            <RubyMessage text={busyAction === 'print' ? 'PDFを準備中…' : '印刷'} />
          </button>
        </div>
        <p className="wasm-note" aria-label={statusText} aria-live="polite">
          <RubyMessage text={statusText} />
        </p>
      </div>
    </section>
  );
}

type WorksheetScreenProps = {
  worksheet: WorksheetDto;
  worksheetMetadata: WorksheetMetadata | null;
  answers: Record<string, EditorState>;
  selectedIndex: number | null;
  elapsed: string;
  gradeResult: GradeResult | null;
  busy: boolean;
  error: string | null;
  notice: string | null;
  onSelect: (index: number) => void;
  onAction: (action: EditorAction) => void;
  onGrade: () => void;
  onReturnToProblems: () => void;
  onRetryWorksheet: () => void;
  onDifferentWorksheet: () => void;
  onPrint: () => void;
  onBack: () => void;
};

function WorksheetScreen({ worksheet, worksheetMetadata, answers, selectedIndex, elapsed, gradeResult, busy, error, notice, onSelect, onAction, onGrade, onReturnToProblems, onRetryWorksheet, onDifferentWorksheet, onPrint, onBack }: WorksheetScreenProps) {
  const sharedLayout = buildSharedWorksheetLayout(worksheet);
  const worksheetTheme = findImplementedThemeByNumericId(worksheet.identity.numeric_theme_id) ?? ONE_DIGIT_ADDITION_THEME;
  const selectedProblem = selectedIndex === null ? null : worksheet.problems[selectedIndex];
  const selectedState = selectedProblem ? answers[selectedProblem.problem_id] ?? emptyEditorState() : null;
  const selectedCapabilities = selectedProblem ? inputCapabilities(selectedProblem.input_interface) : null;
  const visibleStructures = selectedCapabilities?.allowed_structures.filter(
    (structure): structure is Exclude<AnswerInputStructure, 'decimal'> => structure !== 'decimal',
  ) ?? [];
  if (selectedCapabilities?.allow_negative && !visibleStructures.includes('negative')) {
    visibleStructures.push('negative');
  }
  const selectedSlotLabel = selectedState ? answerSlotLabel(selectedState.answer, selectedState.active_path) : null;
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
        <button type="button" className="ribbon-button" aria-label="採点" onClick={onGrade} disabled={busy}><RubyMessage text="採点" /></button>
        <button type="button" className="ribbon-icon" onClick={onPrint} aria-label="印刷" disabled={busy}><RubyMessage text="印刷" /></button>
        <button type="button" className="ribbon-link" aria-label="TOPに戻る" onClick={onBack}><RubyMessage text="TOPに戻る" /></button>
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
        <article className="paper" style={{ aspectRatio: `${A4_PAGE.width} / ${A4_PAGE.height}` }} aria-label={`${worksheet.layout.problem_count}問の${worksheetTheme.worksheet.title}ワークシート`}>
          <div className="problem-grid">
            {worksheetTheme.worksheet.instruction ? (
              <p className="worksheet-instruction">{worksheetTheme.worksheet.instruction}</p>
            ) : null}
            <div className="problem-divider" data-testid="problem-divider" style={dividerStyle} />
            {sharedLayout.cells.map((cell) => {
              const { problem, index } = cell;
              const editor = answers[problem.problem_id] ?? emptyEditorState();
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
                    problem={problem}
                    index={index}
                    editor={editor}
                    isSelected={isSelected}
                    result={result}
                    gradeResult={gradeResult}
                    answerPrefix={worksheetTheme.worksheet.answerPrefix}
                    onSelect={onSelect}
                    onAction={onAction}
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

      {selectedProblem && selectedState ? (
        <div className="input-panel" aria-label="数式入力パネル">
          <div
            className="sr-only"
            role="status"
            aria-live="polite"
            aria-label={`入力位置 ${selectedSlotLabel ?? '答え'}。左右矢印キーで入力欄を移動できます`}
          />
          <div className="input-panel-inner">
            {visibleStructures.length > 0 ? (
              <div className="formula-keypad" aria-label="数式テンプレート">
                {visibleStructures.map((structure) => {
                  const label = STRUCTURE_LABELS[structure];
                  return (
                    <button
                      type="button"
                      key={structure}
                      onClick={() => onAction({ kind: 'insert_structure', structure })}
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
                  onClick={() => onAction({ kind: 'insert_digit', digit })}
                  disabled={busy}
                >
                  {digit}
                </button>
              ))}
              {selectedCapabilities?.allow_decimal ? (
                <button
                  type="button"
                  className="keypad-decimal"
                  onClick={() => onAction({ kind: 'insert_structure', structure: 'decimal' })}
                  disabled={busy}
                  aria-label="小数点"
                >
                  .
                </button>
              ) : null}
            </div>
            <div className="keypad-controls" aria-label="編集キー">
              <button type="button" onClick={() => onAction({ kind: 'move_left' })} disabled={busy} aria-label="カーソルを左へ">←</button>
              <button type="button" onClick={() => onAction({ kind: 'move_right' })} disabled={busy} aria-label="カーソルを右へ">→</button>
              <button type="button" onClick={() => onAction({ kind: 'delete_backward' })} disabled={busy} aria-label="一文字戻す">⌫</button>
              <button type="button" className="keypad-clear" onClick={() => onAction({ kind: 'clear' })} disabled={busy}>クリア</button>
              <button type="button" className="keypad-commit" aria-label="確定" onClick={() => onAction({ kind: 'commit' })} disabled={busy}><RubyMessage text="確定" /></button>
            </div>
          </div>
        </div>
      ) : null}
    </section>
  );
}
