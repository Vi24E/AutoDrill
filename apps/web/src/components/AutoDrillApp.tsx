'use client';

import { createContext, useCallback, useContext, useEffect, useRef, useState, type CSSProperties } from 'react';

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
import { ColumnArithmeticAnswerInput } from '@/components/ColumnArithmeticAnswerInput';
import { deleteEmptyMathLiveStructureBackward } from '@/components/mathlive-structure';
import { useWorksheetAnswerController, type ColumnDigitSelection, type MathfieldSlot } from '@/components/useWorksheetAnswerController';
import type { AutoDrillMathfield } from '@/components/MathLiveMath';
import { answerNodeLatex, answerPrefixLatex, mathTemplateInsertLatex } from '@/domain/mathlive-format';
import { liarPersonLabel } from '@/domain/problem-format';
import { answerCoordinate, answerPresentationPlan } from '@/domain/answer-presentation';
import { columnArithmeticGridVariables, columnArithmeticPageGridVariables } from '@/domain/column-arithmetic-presentation';
import {
  columnAnswerPart,
  columnDigitSpec,
  columnDigitsFromAnswer,
  columnDigitsToAnswer,
  nextColumnDigitIndex,
  previousColumnDigitIndex,
  replaceColumnAnswerPart,
  type ColumnAnswerSlot,
} from '@/domain/column-arithmetic-input';
import { createWasmDrillEngine } from '@/domain/wasm-adapter';
import { loadGeneratedWasmRuntime } from '@/wasm/load-generated';
import { A4_PAGE, buildSharedWorksheetLayout, getCellTopPosition } from '@/domain/layout';
import { worksheetGradeBandClass } from '@/domain/grade-band';
import { hasThemeTag } from '@/domain/theme-registry';
import { generateAutomaticSeed } from '@/domain/seed';
import { AUTODRILL_VERSION_LABEL } from '@/domain/version';
import {
  createWorksheetMetadata,
  formatWorksheetFooter,
  type WorksheetDateGenerator,
  type WorksheetMetadata,
} from '@/domain/worksheet-metadata';


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
  '閉じる': [["閉", "と"], 'じる'],
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
  '小1': [["小1", "しょう"]],
  '小2': [["小2", "しょう"]],
  '小3': [["小3", "しょう"]],
  '小4': [["小4", "しょう"]],
  '小5': [["小5", "しょう"]],
  '小6': [["小6", "しょう"]],
  '中1': [["中1", "ちゅう"]],
  '中2': [["中2", "ちゅう"]],
  '中3': [["中3", "ちゅう"]],
  '足し算と引き算': [["足", "た"], 'し', ["算", "ざん"], 'と', ["引", "ひ"], 'き', ["算", "ざん"]],
  '掛け算と割り算': [["掛", "か"], 'け', ["算", "ざん"], 'と', ["割", "わ"], 'り', ["算", "ざん"]],
  '負の数': [["負", "ふ"], 'の', ["数", "すう"]],
  '正の数・負の数': [["正", "せい"], 'の', ["数", "すう"], '・', ["負", "ふ"], 'の', ["数", "すう"]],
  '一桁の足し算': [["一桁", "ひとけた"], 'の', ["足", "た"], 'し', ["算", "ざん"]],
  '一桁の引き算': [["一桁", "ひとけた"], 'の', ["引", "ひ"], 'き', ["算", "ざん"]],
  '二桁の足し算': [["二桁", "ふたけた"], 'の', ["足", "た"], 'し', ["算", "ざん"]],
  '九九': [["九九", "くく"]],
  '割り算(1)': [["割", "わ"], 'り', ["算", "ざん"], '(1)'],
  '小数の足し算と引き算': [["小数", "しょうすう"], 'の', ["足", "た"], 'し', ["算", "ざん"], 'と', ["引", "ひ"], 'き', ["算", "ざん"]],
  '小数の掛け算': [["小数", "しょうすう"], 'の', ["掛", "か"], 'け', ["算", "ざん"]],
  '小数の割り算': [["小数", "しょうすう"], 'の', ["割", "わ"], 'り', ["算", "ざん"]],
  '二桁の足し算の筆算': [["二桁", "ふたけた"], 'の', ["足", "た"], 'し', ["算", "ざん"], 'の', ["筆算", "ひっさん"]],
  '二桁の引き算の筆算': [["二桁", "ふたけた"], 'の', ["引", "ひ"], 'き', ["算", "ざん"], 'の', ["筆算", "ひっさん"]],
  '三・四桁の足し算の筆算': [['三', 'さん'], '・', ['四桁', 'よんけた'], 'の', ['足', 'た'], 'し', ['算', 'ざん'], 'の', ['筆算', 'ひっさん']],
  '三・四桁の引き算の筆算': [['三', 'さん'], '・', ['四桁', 'よんけた'], 'の', ['引', 'ひ'], 'き', ['算', 'ざん'], 'の', ['筆算', 'ひっさん']],
  '一桁をかける掛け算の筆算': [['一桁', 'ひとけた'], 'をかける', ['掛', 'か'], 'け', ['算', 'ざん'], 'の', ['筆算', 'ひっさん']],
  '二桁をかける掛け算の筆算': [['二桁', 'ふたけた'], 'をかける', ['掛', 'か'], 'け', ['算', 'ざん'], 'の', ['筆算', 'ひっさん']],
  '一桁で割る割り算の筆算': [['一桁', 'ひとけた'], 'で', ['割', 'わ'], 'る', ['割', 'わ'], 'り', ['算', 'ざん'], 'の', ['筆算', 'ひっさん']],
  '二桁で割る割り算の筆算': [['二桁', 'ふたけた'], 'で', ['割', 'わ'], 'る', ['割', 'わ'], 'り', ['算', 'ざん'], 'の', ['筆算', 'ひっさん']],
  '小数の足し算と引き算の筆算': [['小数', 'しょうすう'], 'の', ['足', 'た'], 'し', ['算', 'ざん'], 'と', ['引', 'ひ'], 'き', ['算', 'ざん'], 'の', ['筆算', 'ひっさん']],
  '小数と整数の掛け算の筆算': [['小数', 'しょうすう'], 'と', ['整数', 'せいすう'], 'の', ['掛', 'か'], 'け', ['算', 'ざん'], 'の', ['筆算', 'ひっさん']],
  '小数と整数の割り算の筆算': [['小数', 'しょうすう'], 'と', ['整数', 'せいすう'], 'の', ['割', 'わ'], 'り', ['算', 'ざん'], 'の', ['筆算', 'ひっさん']],
  '小数の掛け算の筆算': [['小数', 'しょうすう'], 'の', ['掛', 'か'], 'け', ['算', 'ざん'], 'の', ['筆算', 'ひっさん']],
  '小数の割り算の筆算': [['小数', 'しょうすう'], 'の', ['割', 'わ'], 'り', ['算', 'ざん'], 'の', ['筆算', 'ひっさん']],
  '分数の足し算': [["分数", "ぶんすう"], 'の', ["足", "た"], 'し', ["算", "ざん"]],
  '分数の引き算': [["分数", "ぶんすう"], 'の', ["引", "ひ"], 'き', ["算", "ざん"]],
  '分数の掛け算': [["分数", "ぶんすう"], 'の', ["掛", "か"], 'け', ["算", "ざん"]],
  '分数の割り算': [["分数", "ぶんすう"], 'の', ["割", "わ"], 'り', ["算", "ざん"]],
  '分数と整数の掛け算': [["分数", "ぶんすう"], 'と', ["整数", "せいすう"], 'の', ["掛", "か"], 'け', ["算", "ざん"]],
  '分数と整数の割り算': [["分数", "ぶんすう"], 'と', ["整数", "せいすう"], 'の', ["割", "わ"], 'り', ["算", "ざん"]],
  '分数総まとめ(仮分数)': [["分数", "ぶんすう"], ["総", "そう"], 'まとめ(', ["仮分数", "かぶんすう"], ')'],
  '二次方程式': [["二次方程式", "にじほうていしき"]],
  '二次方程式(1)': [["二次方程式", "にじほうていしき"], '(1)'],
  '二次方程式(2)': [["二次方程式", "にじほうていしき"], '(2)'],
  '二次方程式(3)': [["二次方程式", "にじほうていしき"], '(3)'],
  '負の数の計算(1)': [["負", "ふ"], 'の', ["数", "すう"], 'の', ["計算", "けいさん"], '(1)'],
  '負の数の計算(2)': [["負", "ふ"], 'の', ["数", "すう"], 'の', ["計算", "けいさん"], '(2)'],
  '連立方程式': [["連立方程式", "れんりつほうていしき"]],
  '連立方程式(1)': [["連立方程式", "れんりつほうていしき"], '(1)'],
  '次の連立方程式を解きなさい。': ['次の', ["連立方程式", "れんりつほうていしき"], 'を', ["解", "と"], 'きなさい。'],
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
  'この問題は紙に印刷して解くことをおすすめします。': ['この', ['問題', 'もんだい'], 'は', ['紙', 'かみ'], 'に', ['印刷', 'いんさつ'], 'して', ['解', 'と'], 'くことをおすすめします。'],
  '問題の生成・入力状態・採点は Rust/WASM が担当します。': [["問題", "もんだい"], 'の', ["生成", "せいせい"], '・', ["入力状態", "にゅうりょくじょうたい"], '・', ["採点", "さいてん"], 'は Rust/WASM が', ["担当", "たんとう"], 'します。'],
  '回答時間': [["回答時間", "かいとうじかん"]],
  '採点': [["採点", "さいてん"]],
  'TOPに戻る': ['TOPに', ["戻", "もど"], 'る'],
  '正解': [["正解", "せいかい"]],
  '約分しましょう': [["約分", "やくぶん"], 'しましょう'],
  '整数でこたえましょう': [["整数", "せいすう"], 'でこたえましょう'],
  '分数でこたえましょう': [["分数", "ぶんすう"], 'でこたえましょう'],
  '最後まで計算しましょう': [["最後", "さいご"], 'まで', ["計算", "けいさん"], 'しましょう'],
  '採点設定': [["採点設定", "さいてんせってい"]],
  '冗長なマイナス': [["冗長", "じょうちょう"], 'なマイナス'],
  '±が重複しています': ['±が', ["重複", "ちょうふく"], 'しています'],
  '余計な小数点': [["余計", "よけい"], 'な', ["小数点", "しょうすうてん"]],
  '同じ解が重複しています': [['同', 'おな'], 'じ', ["解", "かい"], 'が', ["重複", "ちょうふく"], 'しています'],
  '複数の解はカンマで入力しましょう': [["複数", "ふくすう"], 'の', ["解", "かい"], 'はカンマで', ["入力", "にゅうりょく"], 'しましょう'],
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
  fraction_not_reduced: '約分しましょう',
  redundant_negative: '最後まで計算しましょう',
  redundant_plus_minus: '最後まで計算しましょう',
  redundant_decimal: '最後まで計算しましょう',
  duplicate_solution: '最後まで計算しましょう',
  solution_list_required: '最後まで計算しましょう',
  fraction_form_required: '分数でこたえましょう',
  mixed_fraction_form_required: '帯分数でこたえましょう',
  integer_form_required: '整数でこたえましょう',
};

type GradingWarningCategory = 'fraction_reduction' | 'integer_form' | 'finish_calculation' | 'fraction_form';
type WarningGradeMode = 'correct' | 'incorrect';
type GradingSettings = Readonly<Record<GradingWarningCategory, WarningGradeMode>>;

const DEFAULT_GRADING_SETTINGS: GradingSettings = {
  fraction_reduction: 'incorrect',
  integer_form: 'incorrect',
  finish_calculation: 'incorrect',
  fraction_form: 'correct',
};

const GRADING_SETTING_ROWS: readonly { category: GradingWarningCategory; label: string; description: string }[] = [
  { category: 'fraction_reduction', label: '約分しましょう', description: '例: 2/4 と 1/2 の表記を区別します。' },
  { category: 'integer_form', label: '整数でこたえましょう', description: '例: √16 と 4 の表記を区別します。' },
  { category: 'fraction_form', label: '分数でこたえましょう', description: '例: 0.5 と 1/2 の表記を区別します。' },
  { category: 'finish_calculation', label: '最後まで計算しましょう', description: '上の3項目以外の、数学的に同値だが未整理・冗長な表記の違いを区別します。' },
];

function warningCategory(warning: GradeWarningCode): GradingWarningCategory {
  if (warning === 'fraction_not_reduced') return 'fraction_reduction';
  if (warning === 'integer_form_required') return 'integer_form';
  if (warning === 'fraction_form_required' || warning === 'mixed_fraction_form_required') return 'fraction_form';
  return 'finish_calculation';
}

function warningMessages(warnings: readonly GradeWarningCode[]): string[] {
  return [...new Set(warnings.map((warning) => GRADE_WARNING_LABELS[warning]))];
}

function applyGradingSettings(result: GradeResult, settings: GradingSettings): GradeResult {
  const items = result.items.map((item) => {
    const warningMakesIncorrect = item.warnings.some((warning) => settings[warningCategory(warning)] === 'incorrect');
    return { ...item, correct: item.correct && !warningMakesIncorrect };
  });
  return {
    ...result,
    items,
    correct_count: items.filter((item) => item.correct).length,
  };
}


const STRUCTURE_LABELS: Readonly<Record<Exclude<AnswerInputStructure, 'decimal' | 'arithmetic'>, string>> = {
  fraction: '分数',
  mixed_fraction: '帯分数',
  root: '平方根',
  negative: 'マイナス',
  plus_minus: 'プラスマイナス',
  tuple: '複数解',
};

const JUNIOR_HIGH_STRUCTURE_KEYS = ['fraction', 'mixed_fraction', 'root', 'tuple'] as const satisfies readonly AnswerInputStructure[];

type MathInputCommand =
  | { kind: 'insert_digit'; digit: number }
  | { kind: 'insert_structure'; structure: AnswerInputStructure }
  | { kind: 'insert_latex'; latex: string }
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

const ANSWER_CELL_GUTTER = 6;

type PaintedBounds = Pick<DOMRect, 'left' | 'right' | 'top' | 'bottom' | 'width' | 'height'>;

function mathfieldPaintedBounds(mathfield: AutoDrillMathfield): PaintedBounds | null {
  const content = mathfield.shadowRoot?.querySelector<HTMLElement>('[part~="content"]');
  if (!content) return null;
  const rects = [content, ...content.querySelectorAll<HTMLElement>('*')]
    .map((element) => element.getBoundingClientRect())
    .filter((rect) => rect.width > 0 && rect.height > 0);
  if (rects.length === 0) return null;
  const left = Math.min(...rects.map((rect) => rect.left));
  const right = Math.max(...rects.map((rect) => rect.right));
  const top = Math.min(...rects.map((rect) => rect.top));
  const bottom = Math.max(...rects.map((rect) => rect.bottom));
  return { left, right, top, bottom, width: right - left, height: bottom - top };
}

function mathfieldPaintFitsCell(mathfield: AutoDrillMathfield, problemIndex: number): boolean {
  const frame = mathfield.closest<HTMLElement>('.answer-box');
  if (!frame) return true;

  // The visible frame is intrinsically sized by CSS. Input validity is checked
  // against MathLive's painted content rather than this outer frame: tall
  // structures such as fractions may legitimately change frame geometry without
  // any glyph escaping the worksheet cell. JSDOM has no MathLive shadow layout,
  // so tests fall back to the frame rectangle.
  frame.style.removeProperty('width');
  frame.style.removeProperty('height');

  const cell = document.querySelector<HTMLElement>(`[data-problem-index="${problemIndex}"]`);
  const cellRect = cell?.getBoundingClientRect();
  if (!cellRect || cellRect.width <= 0 || cellRect.height <= 0) return true;

  const contentRect = mathfieldPaintedBounds(mathfield) ?? frame.getBoundingClientRect();
  const escapesHorizontally = contentRect.left < cellRect.left + ANSWER_CELL_GUTTER
    || contentRect.right > cellRect.right - ANSWER_CELL_GUTTER;
  const escapesVertically = contentRect.top < cellRect.top + ANSWER_CELL_GUTTER
    || contentRect.bottom > cellRect.bottom - ANSWER_CELL_GUTTER;
  return !escapesHorizontally && !escapesVertically;
}

function waitForMathfieldLayout(): Promise<void> {
  if (typeof window === 'undefined' || typeof window.requestAnimationFrame !== 'function') return Promise.resolve();
  return new Promise((resolve) => {
    window.requestAnimationFrame(() => {
      window.requestAnimationFrame(() => resolve());
    });
  });
}

function columnDraftKey(problemId: string, slot: ColumnAnswerSlot): string {
  return `${problemId}:${slot}`;
}

function isColumnAnswerSlot(slot: MathfieldSlot): slot is ColumnAnswerSlot {
  return slot === 'single' || slot === 'quotient';
}

function mathfieldMapKey(index: number, slot: MathfieldSlot): string {
  return `${index}:${slot}`;
}

function acceptedLatexKey(problemId: string, slot: MathfieldSlot): string {
  return slot === 'single' ? problemId : `${problemId}:${slot}`;
}

function isCoordinateAnswer(answer: AnswerNode): boolean {
  if (answer.type === 'empty' || answer.type === 'integer' || answer.type === 'nan_error') return true;
  return answer.type === 'negative' && isCoordinateAnswer(answer.value);
}

function selectedPeople(answer: AnswerNode): Set<number> {
  if (answer.type !== 'tuple') return new Set();
  return new Set(answer.value.flatMap((item) => item.type === 'integer' ? [Number(item.value)] : []));
}

type WorksheetAnswerFieldProps = {
  worksheetUi: WorksheetUiComponents;
  problem: WorksheetDto['problems'][number];
  index: number;
  answer: AnswerNode;
  isSelected: boolean;
  selectedSlot: MathfieldSlot;
  selectedColumnDigit: ColumnDigitSelection | null;
  columnDrafts: Record<string, Array<string | null>>;
  result: GradeResult['items'][number] | undefined;
  gradeResult: GradeResult | null;
  inputLocked: boolean;
  answerPrefix: string | null;
  onSelect: (index: number, slot: MathfieldSlot) => void;
  onSelectColumnDigit: (index: number, slot: ColumnAnswerSlot, digitIndex: number) => void;
  onRegisterMathfield: (index: number, slot: MathfieldSlot, mathfield: AutoDrillMathfield | null) => void;
  onMathInput: (index: number, slot: MathfieldSlot, mathfield: AutoDrillMathfield, latex: string) => void;
  onCommit: (index: number, slot: MathfieldSlot) => void;
  onTogglePerson: (index: number, person: number) => void;
};

function WorksheetAnswerField({
  worksheetUi,
  problem,
  index,
  answer,
  isSelected,
  selectedSlot,
  selectedColumnDigit,
  columnDrafts,
  result,
  gradeResult,
  inputLocked,
  answerPrefix,
  onSelect,
  onSelectColumnDigit,
  onRegisterMathfield,
  onMathInput,
  onCommit,
  onTogglePerson,
}: WorksheetAnswerFieldProps) {
  const { MathLiveAnswerInput, MathLiveStatic } = worksheetUi;
  const canonicalAnswer = answerNodeText(problem.canonical_answer);
  const presentation = answerPresentationPlan(problem);
  const commonFrameClass = (slot: MathfieldSlot) => `answer-box ${isSelected && selectedSlot === slot ? 'answer-box-selected' : ''} ${result ? (result.correct ? 'answer-box-correct' : 'answer-box-wrong') : ''}`;
  const selectedDigitFor = (slot: ColumnAnswerSlot) => (
    selectedColumnDigit?.problemIndex === index && selectedColumnDigit.slot === slot
      ? selectedColumnDigit.digitIndex
      : null
  );
  const columnDraftFor = (slot: ColumnAnswerSlot) => columnDrafts[columnDraftKey(problem.problem_id, slot)];
  const columnFeedback = result ? (
    <span className={`column-grade-feedback ${result.correct ? 'column-grade-feedback-correct' : 'column-grade-feedback-wrong'}`}>
      <span className="column-grade-mark" aria-label={result.correct ? '正解' : '不正解'}>{result.correct ? '○' : '×'}</span>
    </span>
  ) : null;

  if (presentation.kind === 'liar_puzzle') {
    const chosen = selectedPeople(answer);
    const canonical = selectedPeople(problem.canonical_answer);
    const peopleCount = presentation.peopleCount;
    const renderPeople = (selection: Set<number>, interactive: boolean) => (
      <span className="liar-person-choice-row">
        {Array.from({ length: peopleCount }, (_, personIndex) => personIndex + 1).map((person) => (
          <button
            key={`${problem.problem_id}-person-${person}-${interactive ? 'input' : 'answer'}`}
            type="button"
            className={`liar-person-choice ${selection.has(person) ? 'liar-person-choice-selected' : ''}`}
            aria-label={`${liarPersonLabel(person)}さん${selection.has(person) ? ' 選択中' : ''}`}
            aria-pressed={selection.has(person)}
            disabled={!interactive}
            onClick={() => interactive && onTogglePerson(index, person)}
          >{liarPersonLabel(person)}</button>
        ))}
      </span>
     );
    return (
      <span className="problem-answer-area problem-answer-area-liar">
        {renderPeople(chosen, !inputLocked)}
        {result?.correct ? <span className="result-mark" aria-label="正解">○</span> : null}
        {result && !result.correct ? (
          <span className="correct-answer liar-correct-answer" aria-label={`正しい答え ${[...canonical].map(liarPersonLabel).join('、')}`}>
            {renderPeople(canonical, false)}
          </span>
        ) : null}
      </span>
    );
  }

  if (presentation.kind === 'column_division') {
    const { hasRemainder, quotientSlot } = presentation;
    const quotientValue = hasRemainder ? answerCoordinate(answer, 0) : answer;
    const canonicalQuotient = hasRemainder ? answerCoordinate(problem.canonical_answer, 0) : problem.canonical_answer;
    const remainderValue = hasRemainder ? answerCoordinate(answer, 1) : null;
    const canonicalRemainder = hasRemainder ? answerCoordinate(problem.canonical_answer, 1) : null;
    const showCorrection = Boolean(result && !result.correct);
    return (
      <span className="problem-answer-area problem-answer-area-column-division">
        <span className="column-division-answer-coordinate column-division-answer-coordinate-quotient">
          <span className="column-division-answer-label">商</span>
          <ColumnArithmeticAnswerInput
            problem={problem}
            problemNumber={index + 1}
            slot={quotientSlot}
            value={quotientValue}
            draft={columnDraftFor(quotientSlot)}
            selectedDigit={inputLocked ? null : selectedDigitFor(quotientSlot)}
            readOnly={inputLocked}
            onSelectDigit={(digitIndex) => onSelectColumnDigit(index, quotientSlot, digitIndex)}
          />
        </span>
        {showCorrection ? (
          <span className="column-division-correction column-division-correction-quotient" aria-label={`正しい商 ${answerNodeText(canonicalQuotient)}`}>
            <ColumnArithmeticAnswerInput
              problem={problem}
              problemNumber={index + 1}
              slot={quotientSlot}
              value={canonicalQuotient}
              selectedDigit={null}
              readOnly
              correction
              onSelectDigit={() => undefined}
            />
          </span>
        ) : null}
        {hasRemainder && remainderValue ? (
          <span className="column-division-answer-coordinate column-division-answer-coordinate-remainder">
            <span className="column-division-answer-label">あまり</span>
            <MathLiveAnswerInput
              key={`${problem.problem_id}:remainder:${gradeResult ? 'graded' : 'editing'}`}
              initialLatex={answerNodeLatex(remainderValue)}
              frameClassName={commonFrameClass('remainder')}
              ariaLabel={`${index + 1}番のあまり ${answerNodeText(remainderValue) || '未入力'}`}
              selected={!inputLocked && isSelected && selectedSlot === 'remainder'}
              readOnly={inputLocked}
              numericSansFont
              onSelect={() => onSelect(index, 'remainder')}
              onInputLatex={(mathfield, latex) => onMathInput(index, 'remainder', mathfield, latex)}
              onCommit={() => onCommit(index, 'remainder')}
              onRegister={(mathfield) => onRegisterMathfield(index, 'remainder', mathfield)}
            />
          </span>
        ) : null}
        {showCorrection && canonicalRemainder ? (
          <span className="column-division-correction column-division-correction-remainder" aria-label={`正しいあまり ${answerNodeText(canonicalRemainder)}`}>
            <MathLiveStatic className="column-remainder-correction-value" latex={answerNodeLatex(canonicalRemainder)} ariaLabel={`正しいあまり ${answerNodeText(canonicalRemainder)}`} />
          </span>
        ) : null}
        {columnFeedback}
        {result && result.warnings.length > 0 ? (() => {
          const messages = warningMessages(result.warnings);
          return (
            <span className="grade-warnings" aria-label={`注意 ${messages.join('、')}`}>
              {messages.map((message) => <span key={message}><RubyMessage text={message} /></span>)}
            </span>
          );
        })() : null}
      </span>
    );
  }

  if (presentation.kind === 'column_arithmetic') {
    const showCorrection = Boolean(result && !result.correct);
    return (
      <span className="problem-answer-area problem-answer-area-column-digits">
        <span className="column-answer-user">
          <ColumnArithmeticAnswerInput
            problem={problem}
            problemNumber={index + 1}
            slot="single"
            value={answer}
            draft={columnDraftFor('single')}
            selectedDigit={inputLocked ? null : selectedDigitFor('single')}
            readOnly={inputLocked}
            onSelectDigit={(digitIndex) => onSelectColumnDigit(index, 'single', digitIndex)}
          />
        </span>
        {showCorrection ? (
          <span className="column-answer-correction" aria-label={`正しい答え ${canonicalAnswer}`}>
            <ColumnArithmeticAnswerInput
              problem={problem}
              problemNumber={index + 1}
              slot="single"
              value={problem.canonical_answer}
              selectedDigit={null}
              readOnly
              correction
              onSelectDigit={() => undefined}
            />
          </span>
        ) : null}
        {columnFeedback}
        {result && result.warnings.length > 0 ? (() => {
          const messages = warningMessages(result.warnings);
          return (
            <span className="grade-warnings" aria-label={`注意 ${messages.join('、')}`}>
              {messages.map((message) => <span key={message}><RubyMessage text={message} /></span>)}
            </span>
          );
        })() : null}
      </span>
    );
  }

  if (presentation.kind === 'simultaneous_equation') {
    const xAnswer = answerCoordinate(answer, 0);
    const yAnswer = answerCoordinate(answer, 1);
    const canonicalX = answerCoordinate(problem.canonical_answer, 0);
    const canonicalY = answerCoordinate(problem.canonical_answer, 1);
    return (
      <span className="problem-answer-area problem-answer-area-simultaneous">
        <span className="simultaneous-answer-coordinate">
          <MathLiveStatic className="answer-prefix-label" latex="x=" ariaLabel="x =" />
          <MathLiveAnswerInput
            key={`${problem.problem_id}:x:${gradeResult ? 'graded' : 'editing'}`}
            initialLatex={answerNodeLatex(xAnswer)}
            frameClassName={commonFrameClass('x')}
            ariaLabel={`${index + 1}番のxの答え ${answerNodeText(xAnswer) || '未入力'}`}
            selected={isSelected && selectedSlot === 'x'}
            readOnly={inputLocked}
            onSelect={() => onSelect(index, 'x')}
            onInputLatex={(mathfield, latex) => onMathInput(index, 'x', mathfield, latex)}
            onCommit={() => onCommit(index, 'x')}
            onRegister={(mathfield) => onRegisterMathfield(index, 'x', mathfield)}
          />
        </span>
        <span className="simultaneous-answer-coordinate">
          <MathLiveStatic className="answer-prefix-label" latex="y=" ariaLabel="y =" />
          <MathLiveAnswerInput
            key={`${problem.problem_id}:y:${gradeResult ? 'graded' : 'editing'}`}
            initialLatex={answerNodeLatex(yAnswer)}
            frameClassName={commonFrameClass('y')}
            ariaLabel={`${index + 1}番のyの答え ${answerNodeText(yAnswer) || '未入力'}`}
            selected={isSelected && selectedSlot === 'y'}
            readOnly={inputLocked}
            onSelect={() => onSelect(index, 'y')}
            onInputLatex={(mathfield, latex) => onMathInput(index, 'y', mathfield, latex)}
            onCommit={() => onCommit(index, 'y')}
            onRegister={(mathfield) => onRegisterMathfield(index, 'y', mathfield)}
          />
        </span>
        {result?.correct ? <span className="result-mark" aria-label="正解">○</span> : null}
        {result && !result.correct ? (
          <span className="correct-answer" aria-label={`正しい答え ${canonicalAnswer}`}>
            <MathLiveStatic
              className="canonical-answer-math"
              latex={`x=${answerNodeLatex(canonicalX)}\;y=${answerNodeLatex(canonicalY)}`}
              ariaLabel={`x = ${answerNodeText(canonicalX)}, y = ${answerNodeText(canonicalY)}`}
            />
          </span>
        ) : null}
        {result && result.warnings.length > 0 ? (() => {
          const messages = warningMessages(result.warnings);
          return (
            <span className="grade-warnings" aria-label={`注意 ${messages.join('、')}`}>
              {messages.map((message) => <span key={message}><RubyMessage text={message} /></span>)}
            </span>
          );
        })() : null}
      </span>
    );
  }

  const answerText = answerNodeText(answer);
  return (
    <span className="problem-answer-area">
      {answerPrefix ? (
        <MathLiveStatic
          className="answer-prefix-label"
          latex={answerPrefixLatex(answerPrefix)}
          ariaLabel={answerPrefix}
        />
      ) : null}
      <MathLiveAnswerInput
        key={`${problem.problem_id}:${gradeResult ? 'graded' : 'editing'}`}
        initialLatex={answerNodeLatex(answer)}
        frameClassName={commonFrameClass('single')}
        ariaLabel={`${index + 1}番の答え ${answerText || '未入力'}`}
        selected={isSelected && selectedSlot === 'single'}
        readOnly={inputLocked}
        onSelect={() => onSelect(index, 'single')}
        onInputLatex={(mathfield, latex) => onMathInput(index, 'single', mathfield, latex)}
        onCommit={() => onCommit(index, 'single')}
        onRegister={(mathfield) => onRegisterMathfield(index, 'single', mathfield)}
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
      {result && result.warnings.length > 0 ? (() => {
        const messages = warningMessages(result.warnings);
        return (
          <span className="grade-warnings" aria-label={`注意 ${messages.join('、')}`}>
            {messages.map((message) => (
              <span key={message}><RubyMessage text={message} /></span>
            ))}
          </span>
        );
      })() : null}
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
  const {
    answers,
    selectedIndex,
    selectedSlot,
    selectedColumnDigit,
    columnDrafts,
    answersRef,
    selectedIndexRef,
    selectedSlotRef,
    selectedColumnDigitRef,
    columnDraftsRef,
    inputEnabledRef,
    actionQueueRef,
    acceptedLatexRef,
    setAnswer,
    setColumnDraft,
    setSelectedIndex,
    setSelectedSlot,
    setSelectedColumnDigit,
    select: selectAnswer,
    selectColumnDigit: selectColumnAnswerDigit,
    clearSelection,
    registerMathfield: registerControllerMathfield,
    getMathfield,
    blurMathfields,
    setMathfieldsReadOnly,
    resetForWorksheet,
    disableInputAndClearSelection,
  } = useWorksheetAnswerController();
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
  const [gradingSettings, setGradingSettings] = useState<GradingSettings>(DEFAULT_GRADING_SETTINGS);
  const worksheetPhaseRef = useRef<WorksheetPhase>('editing');
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
        if (active) {
          window.__AUTODRILL_WASM__ = runtime;
          window.__AUTODRILL_SCHEMA_VERSION__ = DRILL_SCHEMA_VERSION;
        }
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
    } else if (selectedTheme.implemented && selectedTheme.grade === null) {
      changeTheme(ONE_DIGIT_ADDITION_THEME);
    }
  }, [changeTheme, selectedTheme]);

  const changeDifficulty = useCallback((difficulty: DifficultyLevel) => {
    setWebSettings((current) => ({ ...current, difficulty }));
    setSettings((current) => ({ ...current, difficulty }));
  }, []);

  const changeSettings = useCallback((next: DrillSettings) => {
    setSettings(next);
    setWebSettings((current) => ({ ...current, seed: next.seed }));
  }, []);

  const installWorksheet = useCallback((nextWorksheet: WorksheetDto, metadata: WorksheetMetadata) => {
    resetForWorksheet(nextWorksheet);
    setWorksheet(nextWorksheet);
    setWorksheetMetadata(metadata);
    setGradeResult(null);
    transitionWorksheetPhase('editing');
    const timerStart = Date.now();
    setStartedAt(timerStart);
    setFinishedAt(null);
    setNow(timerStart);
  }, [resetForWorksheet, transitionWorksheetPhase]);

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

  const selectProblem = useCallback((index: number, slot: MathfieldSlot = 'single') => {
    if (worksheetPhaseRef.current !== 'editing') return;
    selectAnswer(index, slot);
    // The input panel is mounted by this selection. Running on the next frame
    // lets the shared viewport guard see its real top edge and keeps even a
    // bottom-aligned answer field (for example x = [...]) unobscured.
    scheduleProblemScroll(index, index);
    setError(null);
    dismissNotice();
  }, [dismissNotice, selectAnswer]);

  const selectColumnDigit = useCallback((index: number, slot: ColumnAnswerSlot, digitIndex: number) => {
    if (!worksheet || worksheetPhaseRef.current !== 'editing') return;
    const problem = worksheet.problems[index];
    if (!problem || problem.prompt.kind !== 'column_arithmetic') return;
    const spec = columnDigitSpec(problem, slot);
    if (digitIndex < spec.activeStart || digitIndex > spec.activeEnd) return;
    const key = columnDraftKey(problem.problem_id, slot);
    if (!columnDraftsRef.current[key]) {
      const current = answersRef.current[problem.problem_id] ?? ({ type: 'empty' } satisfies AnswerNode);
      const draft = columnDigitsFromAnswer(columnAnswerPart(current, slot), spec);
      setColumnDraft(key, draft);
    }
    const selection = { problemIndex: index, slot, digitIndex };
    selectColumnAnswerDigit(selection);
    scheduleProblemScroll(index, index);
    setError(null);
    dismissNotice();
  }, [answersRef, columnDraftsRef, dismissNotice, selectColumnAnswerDigit, setColumnDraft, worksheet]);

  const closeInputPanel = useCallback(() => {
    if (worksheetPhaseRef.current !== 'editing') return;
    clearSelection();
    blurMathfields();
  }, [blurMathfields, clearSelection]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== 'Escape' || selectedIndexRef.current === null) return;
      event.preventDefault();
      closeInputPanel();
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [closeInputPanel, selectedIndexRef]);

  const registerMathfield = useCallback((index: number, slot: MathfieldSlot, mathfield: AutoDrillMathfield | null) => {
    const key = mathfieldMapKey(index, slot);
    registerControllerMathfield(key, mathfield);
  }, [registerControllerMathfield]);

  const updateMathLiveAnswer = useCallback((index: number, slot: MathfieldSlot, mathfield: AutoDrillMathfield, latex: string) => {
    if (!worksheet || worksheetPhaseRef.current !== 'editing' || !inputEnabledRef.current || !worksheet.problems[index]) return Promise.resolve();
    const problem = worksheet.problems[index];
    const problemId = problem.problem_id;

    const run = async () => {
      const previous = answersRef.current[problemId] ?? ({ type: 'empty' } satisfies AnswerNode);
      const latexKey = acceptedLatexKey(problemId, slot);
      const isCoordinateSlot = slot !== 'single' && (
        problem.prompt.kind === 'simultaneous_equation'
        || (problem.prompt.kind === 'column_arithmetic' && problem.prompt.operator === 'divide' && problem.answer_schema.kind === 'ordered_pair')
      );
      const previousPart = isCoordinateSlot
        ? answerCoordinate(previous, slot === 'x' || slot === 'quotient' ? 0 : 1)
        : previous;
      const previousLatex = acceptedLatexRef.current[latexKey] ?? answerNodeLatex(previousPart);

      let parsed: AnswerNode;
      try {
        // Parse first: AST-size rejection is independent of layout and should
        // behave as an immediate NOP, without leaving the rejected value visible
        // while waiting for animation frames.
        const editorInputInterface = findImplementedThemeByNumericId(problem.numeric_theme_id)?.editorInputInterface
          ?? problem.input_interface;
        parsed = await engine.parseMathLiveAnswer(latex, editorInputInterface);
      } catch (value) {
        mathfield.setValue(previousLatex, { silenceNotifications: true });
        if (value instanceof DrillEngineError && value.kind === 'answer_ast_size_limit') {
          showEngineError(value);
        }
        return;
      }

      let answer = parsed;
      if (isCoordinateSlot) {
        if (!isCoordinateAnswer(parsed)) {
          mathfield.setValue(previousLatex, { silenceNotifications: true });
          return;
        }
        const values: [AnswerNode, AnswerNode] = [answerCoordinate(previous, 0), answerCoordinate(previous, 1)];
        values[slot === 'x' || slot === 'quotient' ? 0 : 1] = parsed;
        answer = { type: 'tuple', value: values };
      }

      // Empty is always a valid visual state. In particular, MathLive may leave
      // a caret/placeholder paint box after deleteAll; that UI chrome must never
      // turn a successful clear into a false "too large" rejection.
      if (parsed.type !== 'empty') {
        // MathLive and the intrinsic answer frame resolve in the same layout pass.
        // Wait only to validate the settled geometry against the worksheet cell;
        // JS no longer resizes the visible frame after paint.
        await waitForMathfieldLayout();
        if (!mathfieldPaintFitsCell(mathfield, index)) {
          // Render-size rejection is also a true NOP. Restore the exact accepted
          // LaTeX, not a normalized/reconstructed AnswerNode representation.
          mathfield.setValue(previousLatex, { silenceNotifications: true });
          await waitForMathfieldLayout();
          mathfieldPaintFitsCell(mathfield, index);
          setNotice('式が大きすぎます！');
          return;
        }
      }

      acceptedLatexRef.current = { ...acceptedLatexRef.current, [latexKey]: latex };
      setAnswer(problemId, answer);
      setError(null);
      dismissNotice();
    };

    const queued = actionQueueRef.current.then(run, run);
    actionQueueRef.current = queued.then(() => undefined, () => undefined);
    return queued;
  }, [acceptedLatexRef, actionQueueRef, answersRef, dismissNotice, engine, inputEnabledRef, setAnswer, showEngineError, worksheet]);

  const updateColumnDigitDraft = useCallback((index: number, slot: ColumnAnswerSlot, nextDraft: Array<string | null>) => {
    if (!worksheet || worksheetPhaseRef.current !== 'editing') return;
    const problem = worksheet.problems[index];
    if (!problem || problem.prompt.kind !== 'column_arithmetic') return;
    const spec = columnDigitSpec(problem, slot);
    const key = columnDraftKey(problem.problem_id, slot);
    const normalizedDraft = Array.from({ length: spec.cellCount }, (_, digitIndex) => nextDraft[digitIndex] ?? null);
    setColumnDraft(key, normalizedDraft);

    const current = answersRef.current[problem.problem_id] ?? ({ type: 'empty' } satisfies AnswerNode);
    const part = columnDigitsToAnswer(normalizedDraft, spec);
    const answer = replaceColumnAnswerPart(current, slot, part);
    setAnswer(problem.problem_id, answer);
    setError(null);
    dismissNotice();
  }, [answersRef, dismissNotice, setAnswer, setColumnDraft, worksheet]);

  const togglePerson = useCallback((index: number, person: number) => {
    if (!worksheet || worksheetPhaseRef.current !== 'editing') return;
    const problem = worksheet.problems[index];
    if (!problem || problem.prompt.kind !== 'liar_puzzle' || person < 1 || person > problem.prompt.people_count) return;
    const current = selectedPeople(answersRef.current[problem.problem_id] ?? ({ type: 'tuple', value: [] } satisfies AnswerNode));
    if (current.has(person)) current.delete(person);
    else current.add(person);
    const value = [...current].sort((left, right) => left - right).map((selected) => ({ type: 'integer', value: String(selected) } as const));
    setAnswer(problem.problem_id, { type: 'tuple', value } as AnswerNode);
    setError(null);
    dismissNotice();
  }, [answersRef, dismissNotice, setAnswer, worksheet]);

  const commitMathfield = useCallback((index: number, slot: MathfieldSlot) => {
    if (!worksheet || worksheetPhaseRef.current !== 'editing' || !worksheet.problems[index]) return Promise.resolve();
    const problem = worksheet.problems[index];
    if (problem.prompt.kind === 'simultaneous_equation' && slot === 'x' && inputEnabledRef.current) {
      setSelectedSlot('y');
      setSelectedColumnDigit(null);
      return actionQueueRef.current;
    }
    if (
      problem.prompt.kind === 'column_arithmetic'
      && problem.prompt.operator === 'divide'
      && problem.answer_schema.kind === 'ordered_pair'
      && slot === 'quotient'
      && inputEnabledRef.current
    ) {
      selectProblem(index, 'remainder');
      return actionQueueRef.current;
    }
    if (index < worksheet.problems.length - 1 && inputEnabledRef.current) {
      const nextIndex = index + 1;
      const nextProblem = worksheet.problems[nextIndex];
      if (nextProblem?.prompt.kind === 'column_arithmetic') {
        const nextColumnSlot: ColumnAnswerSlot = nextProblem.prompt.operator === 'divide' && nextProblem.answer_schema.kind === 'ordered_pair'
          ? 'quotient'
          : 'single';
        const spec = columnDigitSpec(nextProblem, nextColumnSlot);
        selectColumnDigit(nextIndex, nextColumnSlot, spec.initialIndex);
        return actionQueueRef.current;
      }
      const nextSlot: MathfieldSlot = nextProblem?.prompt.kind === 'simultaneous_equation' ? 'x' : 'single';
      selectAnswer(nextIndex, nextSlot);
      scheduleProblemScroll(index, nextIndex);
    }

    return actionQueueRef.current;
  }, [actionQueueRef, inputEnabledRef, selectAnswer, selectColumnDigit, selectProblem, setSelectedColumnDigit, setSelectedSlot, worksheet]);


  const applyMathCommand = useCallback((command: MathInputCommand) => {
    const index = selectedIndexRef.current;
    if (index === null || worksheetPhaseRef.current !== 'editing' || !inputEnabledRef.current) return;
    const slot = selectedSlotRef.current;
    const columnSelection = selectedColumnDigitRef.current;
    const columnProblem = worksheet?.problems[index];
    if (
      columnSelection
      && columnSelection.problemIndex === index
      && isColumnAnswerSlot(slot)
      && columnSelection.slot === slot
      && columnProblem?.prompt.kind === 'column_arithmetic'
    ) {
      const spec = columnDigitSpec(columnProblem, slot);
      const key = columnDraftKey(columnProblem.problem_id, slot);
      const currentAnswer = answersRef.current[columnProblem.problem_id] ?? ({ type: 'empty' } satisfies AnswerNode);
      const currentDraft = [...(
        columnDraftsRef.current[key]
        ?? columnDigitsFromAnswer(columnAnswerPart(currentAnswer, slot), spec)
      )];
      const setDigitSelection = (digitIndex: number) => {
        const selection = { problemIndex: index, slot, digitIndex };
        setSelectedColumnDigit(selection);
      };
      const writeDraft = (draft: Array<string | null>) => updateColumnDigitDraft(index, slot, draft);

      switch (command.kind) {
        case 'insert_digit': { // 筆算の各桁は独立slot。入力方向だけ演算に応じて変える。
          currentDraft[columnSelection.digitIndex] = String(command.digit);
          writeDraft(currentDraft);
          if (
            slot === 'quotient'
            && columnProblem.prompt.operator === 'divide'
            && columnProblem.answer_schema.kind === 'ordered_pair'
            && columnSelection.digitIndex === spec.activeEnd
          ) {
            // The quotient is written left-to-right. Once its final place is
            // entered, continue directly into the ordinary big-endian remainder
            // field rather than requiring an extra confirmation click.
            selectProblem(index, 'remainder');
          } else {
            setDigitSelection(nextColumnDigitIndex(spec, columnSelection.digitIndex));
          }
          break;
        }
        case 'move_left':
          setDigitSelection(Math.max(spec.activeStart, columnSelection.digitIndex - 1));
          break;
        case 'move_right':
          setDigitSelection(Math.min(spec.activeEnd, columnSelection.digitIndex + 1));
          break;
        case 'delete_backward': { // 直前に入力した桁へ戻って消す。
          const previous = previousColumnDigitIndex(spec, columnSelection.digitIndex);
          const target = currentDraft[columnSelection.digitIndex] === null && previous !== columnSelection.digitIndex
            ? previous
            : columnSelection.digitIndex;
          currentDraft[target] = null;
          writeDraft(currentDraft);
          setDigitSelection(target);
          break;
        }
        case 'delete_forward':
          currentDraft[columnSelection.digitIndex] = null;
          writeDraft(currentDraft);
          break;
        case 'clear': {
          const cleared = currentDraft.map((digit, digitIndex) => (
            digitIndex >= spec.activeStart && digitIndex <= spec.activeEnd ? null : digit
          ));
          writeDraft(cleared);
          setDigitSelection(spec.initialIndex);
          break;
        }
        case 'commit':
          void commitMathfield(index, slot);
          break;
        case 'insert_structure':
        case 'insert_latex':
          break;
      }
      return;
    }

    const mathfield = getMathfield(mathfieldMapKey(index, slot));
    if (!mathfield) return;
    mathfield.focus();

    switch (command.kind) {
      case 'insert_digit':
        mathfield.insert(String(command.digit), { selectionMode: 'after' });
        break;
      case 'insert_structure':
        if (command.structure === 'arithmetic') break;
        if (command.structure === 'decimal') {
          mathfield.insert('.', { selectionMode: 'after' });
        } else {
          mathfield.insert(mathTemplateInsertLatex(command.structure), { selectionMode: 'placeholder' });
        }
        break;
      case 'insert_latex':
        mathfield.insert(command.latex, { selectionMode: 'after' });
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
          void updateMathLiveAnswer(index, slot, mathfield, mathfield.value);
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
        void commitMathfield(index, slot);
        break;
    }
  }, [answersRef, columnDraftsRef, commitMathfield, getMathfield, inputEnabledRef, selectProblem, selectedColumnDigitRef, selectedIndexRef, selectedSlotRef, setSelectedColumnDigit, updateColumnDigitDraft, updateMathLiveAnswer, worksheet]);

  useEffect(() => {
    const onColumnKeyDown = (event: KeyboardEvent) => {
      if (!selectedColumnDigitRef.current || worksheetPhaseRef.current !== 'editing') return;
      let command: MathInputCommand | null = null;
      if (/^[0-9]$/.test(event.key)) command = { kind: 'insert_digit', digit: Number(event.key) };
      else if (event.key === 'ArrowLeft') command = { kind: 'move_left' };
      else if (event.key === 'ArrowRight') command = { kind: 'move_right' };
      else if (event.key === 'Backspace') command = { kind: 'delete_backward' };
      else if (event.key === 'Delete') command = { kind: 'delete_forward' };
      else if (event.key === 'Enter') command = { kind: 'commit' };
      if (!command) return;
      event.preventDefault();
      applyMathCommand(command);
    };
    window.addEventListener('keydown', onColumnKeyDown);
    return () => window.removeEventListener('keydown', onColumnKeyDown);
  }, [applyMathCommand, selectedColumnDigitRef]);

  const drainActionQueue = useCallback(async () => {
    while (true) {
      const pending = actionQueueRef.current;
      await pending;
      if (pending === actionQueueRef.current) return;
    }
  }, [actionQueueRef]);

  const grade = useCallback(async () => {
    if (!worksheet || worksheetPhaseRef.current !== 'editing') return;

    // Lock synchronously before the first await. This closes the same-tick
    // double-click/keyboard window that React state alone cannot close.
    transitionWorksheetPhase('grading');
    disableInputAndClearSelection();
    setMathfieldsReadOnly(true);

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
      setGradeResult(applyGradingSettings(result, gradingSettings));
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
      setMathfieldsReadOnly(false);
      showEngineError(value);
    } finally {
      setSelectedIndex(null);
      setSelectedColumnDigit(null);
      setBusy(false);
    }
  }, [answersRef, disableInputAndClearSelection, drainActionQueue, engine, finishedAt, gradingSettings, setMathfieldsReadOnly, setSelectedColumnDigit, setSelectedIndex, showEngineError, startedAt, transitionWorksheetPhase, worksheet]);

  const returnToProblems = useCallback(() => {
    if (worksheetPhaseRef.current !== 'graded') return;
    const resumedAt = Date.now();
    const frozenElapsed = startedAt === null || finishedAt === null ? 0 : Math.max(0, finishedAt - startedAt);
    setStartedAt(resumedAt - frozenElapsed);
    setFinishedAt(null);
    setNow(resumedAt);
    setGradeResult(null);
    transitionWorksheetPhase('editing');
    clearSelection();
    setError(null);
    dismissNotice();
  }, [clearSelection, dismissNotice, finishedAt, startedAt, transitionWorksheetPhase]);

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
    clearSelection();
    setStartedAt(null);
    setFinishedAt(null);
    setGradeResult(null);
    transitionWorksheetPhase('editing');
    setError(null);
    dismissNotice();
  }, [clearSelection, dismissNotice, transitionWorksheetPhase]);

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
            gradingSettings={gradingSettings}
            onSettingsChange={changeSettings}
            onCurriculumModeChange={changeCurriculumMode}
            onThemeChange={changeTheme}
            onDifficultyChange={changeDifficulty}
            onFuriganaChange={changeFurigana}
            onGradingSettingsChange={setGradingSettings}
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
            selectedSlot={selectedSlot}
            selectedColumnDigit={selectedColumnDigit}
            columnDrafts={columnDrafts}
            elapsed={formatElapsed(startedAt, finishedAt ?? now)}
            gradeResult={gradeResult}
            worksheetPhase={worksheetPhase}
            busy={busy}
            error={error}
            notice={notice}
            onSelect={selectProblem}
            onSelectColumnDigit={selectColumnDigit}
            onCommand={applyMathCommand}
            onRegisterMathfield={registerMathfield}
            onMathInput={(index, slot, mathfield, latex) => void updateMathLiveAnswer(index, slot, mathfield, latex)}
            onCommit={(index, slot) => void commitMathfield(index, slot)}
            onTogglePerson={togglePerson}
            onCloseInput={closeInputPanel}
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

function gradeTagForTheme(theme: CurriculumTheme): { label: string; className: string } | null {
  if (!theme.implemented || !theme.grade) return null;
  const grade = theme.grade.number;
  const label = grade <= 6 ? `小${grade}` : `中${grade - 6}`;
  const className = `grade-tag-grade-${grade}`;
  return { label, className };
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
  gradingSettings: GradingSettings;
  onSettingsChange: (settings: DrillSettings) => void;
  onCurriculumModeChange: (mode: CurriculumMode) => void;
  onThemeChange: (theme: CurriculumTheme) => void;
  onDifficultyChange: (difficulty: DifficultyLevel) => void;
  onFuriganaChange: (enabled: boolean) => void;
  onGradingSettingsChange: (settings: GradingSettings) => void;
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
  gradingSettings,
  onSettingsChange,
  onCurriculumModeChange,
  onThemeChange,
  onDifficultyChange,
  onFuriganaChange,
  onGradingSettingsChange,
  onGenerate,
  onPrint,
}: SettingsScreenProps) {
  const selection = findCurriculumSelection(webSettings.themeKey);
  const genres = curriculumMode === 'recommended' ? RECOMMENDED_GENRES : selection.grade.genres;
  const activeGenre = curriculumMode === 'recommended'
    ? genres.find((genre) => genre.themes.some((theme) => theme.themeKey === webSettings.themeKey)) ?? genres[0]!
    : selection.genre;
  const unavailable = !selection.theme.implemented;
  const [gradingSettingsOpen, setGradingSettingsOpen] = useState(false);
  const gradingDialogCloseRef = useRef<HTMLButtonElement | null>(null);

  useEffect(() => {
    if (!gradingSettingsOpen) return undefined;
    gradingDialogCloseRef.current?.focus();
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') setGradingSettingsOpen(false);
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [gradingSettingsOpen]);

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
                renderValue={(option) => {
                  const theme = activeGenre.themes.find((candidate) => candidate.themeKey === option.value);
                  const tag = curriculumMode === 'recommended' && theme ? gradeTagForTheme(theme) : null;
                  return (
                    <span className="theme-select-value-content">
                      <RubyMessage text={option.label} />
                      {tag ? <span className={`grade-tag ${tag.className}`}>{tag.label}</span> : null}
                    </span>
                  );
                }}
                renderOptionEnd={(option) => {
                  if (curriculumMode !== 'recommended') return null;
                  const theme = activeGenre.themes.find((candidate) => candidate.themeKey === option.value);
                  if (!theme) return null;
                  const tag = gradeTagForTheme(theme);
                  return tag ? <span className={`grade-tag ${tag.className}`}>{tag.label}</span> : null;
                }}
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

          {selection.theme.implemented && hasThemeTag(selection.theme, 'print_recommended') ? (
            <p className="print-recommended-note" role="note" aria-label="この問題は紙に印刷して解くことをおすすめします。">
              <RubyMessage text="この問題は紙に印刷して解くことをおすすめします。" />
            </p>
          ) : null}

          <FuriganaContext.Provider value={false}>
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
              <button
                type="button"
                className="grading-settings-open-button"
                aria-haspopup="dialog"
                onClick={() => setGradingSettingsOpen(true)}
              >
                <RubyMessage text="採点設定" />
                <span aria-hidden="true">›</span>
              </button>
            </div>
          </details>
          </FuriganaContext.Provider>
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
      {gradingSettingsOpen ? (
        <FuriganaContext.Provider value={false}>
        <div
          className="grading-settings-modal-backdrop"
          onMouseDown={(event) => {
            if (event.target === event.currentTarget) setGradingSettingsOpen(false);
          }}
        >
          <section
            className="grading-settings-modal"
            role="dialog"
            aria-modal="true"
            aria-labelledby="grading-settings-title"
          >
            <header className="grading-settings-modal-header">
              <h2 id="grading-settings-title"><RubyMessage text="採点設定" /></h2>
              <button
                ref={gradingDialogCloseRef}
                type="button"
                className="grading-settings-modal-close"
                aria-label="採点設定を閉じる"
                onClick={() => setGradingSettingsOpen(false)}
              >×</button>
            </header>
            <p className="grading-settings-modal-note">○では数学的に同じ答えを正解として扱い、×では次の表記の違いを採点に反映します。</p>
            <div className="grading-settings-modal-body">
              {GRADING_SETTING_ROWS.map(({ category, label, description }) => (
                <div className="grading-setting-row" key={category}>
                  <div className="grading-setting-copy">
                    <strong><RubyMessage text={label} /></strong>
                    <span>{description}</span>
                  </div>
                  <div className="grading-setting-toggle" role="group" aria-label={`${label}の採点`}>
                    <button
                      type="button"
                      aria-label={`${label}を丸にする`}
                      aria-pressed={gradingSettings[category] === 'correct'}
                      onClick={() => onGradingSettingsChange({ ...gradingSettings, [category]: 'correct' })}
                    >○</button>
                    <button
                      type="button"
                      aria-label={`${label}をバツにする`}
                      aria-pressed={gradingSettings[category] === 'incorrect'}
                      onClick={() => onGradingSettingsChange({ ...gradingSettings, [category]: 'incorrect' })}
                    >×</button>
                  </div>
                </div>
              ))}
            </div>
          </section>
        </div>
        </FuriganaContext.Provider>
      ) : null}
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
  selectedSlot: MathfieldSlot;
  selectedColumnDigit: ColumnDigitSelection | null;
  columnDrafts: Record<string, Array<string | null>>;
  elapsed: string;
  gradeResult: GradeResult | null;
  worksheetPhase: WorksheetPhase;
  busy: boolean;
  error: string | null;
  notice: string | null;
  onSelect: (index: number, slot: MathfieldSlot) => void;
  onSelectColumnDigit: (index: number, slot: ColumnAnswerSlot, digitIndex: number) => void;
  onCommand: (command: MathInputCommand) => void;
  onRegisterMathfield: (index: number, slot: MathfieldSlot, mathfield: AutoDrillMathfield | null) => void;
  onMathInput: (index: number, slot: MathfieldSlot, mathfield: AutoDrillMathfield, latex: string) => void;
  onCommit: (index: number, slot: MathfieldSlot) => void;
  onTogglePerson: (index: number, person: number) => void;
  onCloseInput: () => void;
  onGrade: () => void;
  onReturnToProblems: () => void;
  onRetryWorksheet: () => void;
  onDifferentWorksheet: () => void;
  onPrint: () => void;
  onBack: () => void;
};

function WorksheetScreen({ worksheetUi, worksheet, worksheetMetadata, answers, selectedIndex, selectedSlot, selectedColumnDigit, columnDrafts, elapsed, gradeResult, worksheetPhase, busy, error, notice, onSelect, onSelectColumnDigit, onCommand, onRegisterMathfield, onMathInput, onCommit, onTogglePerson, onCloseInput, onGrade, onReturnToProblems, onRetryWorksheet, onDifferentWorksheet, onPrint, onBack }: WorksheetScreenProps) {
  const { MathTemplateIcon, ProblemExpression } = worksheetUi;
  const sharedLayout = buildSharedWorksheetLayout(worksheet);
  const worksheetTheme = findImplementedThemeByNumericId(worksheet.identity.numeric_theme_id) ?? ONE_DIGIT_ADDITION_THEME;
  const isColumnArithmeticWorksheet = worksheetTheme.presentation.column_arithmetic;
  const isEquationWorksheet = worksheetTheme.presentation.equation_layout;
  const gradeBandClass = worksheetTheme.grade ? worksheetGradeBandClass(worksheetTheme.grade.number) : 'worksheet-grade-junior-high';
  const worksheetCategoryLabel = worksheetTheme.grade?.label ?? 'おまけ';
  const selectedProblem = worksheetPhase === 'editing' && selectedIndex !== null ? worksheet.problems[selectedIndex] : null;
  const columnDigitInputActive = selectedProblem?.prompt.kind === 'column_arithmetic' && selectedColumnDigit !== null;
  const selectedCapabilities = selectedProblem ? inputCapabilities(worksheetTheme.editorInputInterface) : null;
  const juniorHighFullKeypad = Boolean(selectedProblem && worksheetTheme.grade && worksheetTheme.grade.number >= 7);
  const arithmeticOperatorsEnabled = selectedCapabilities?.allowed_structures.includes('arithmetic') ?? false;
  const visibleStructures = juniorHighFullKeypad
    ? [...JUNIOR_HIGH_STRUCTURE_KEYS]
    : selectedCapabilities?.allowed_structures.filter(
        (structure): structure is Exclude<AnswerInputStructure, 'decimal' | 'arithmetic'> => (
          structure !== 'decimal'
          && structure !== 'arithmetic'
          && !(selectedProblem?.prompt.kind === 'simultaneous_equation' && structure === 'tuple')
          && !(selectedProblem?.prompt.kind === 'column_arithmetic' && selectedProblem.prompt.operator === 'divide' && structure === 'tuple')
          && !(arithmeticOperatorsEnabled && (structure === 'negative' || structure === 'plus_minus'))
        ),
      ) ?? [];
  if (!juniorHighFullKeypad && selectedCapabilities?.allow_negative && !arithmeticOperatorsEnabled && !visibleStructures.includes('negative')) {
    visibleStructures.push('negative');
  }
  const resultById = new Map((gradeResult?.items ?? []).map((item) => [item.problem_id, item]));
  const toPagePercent = (value: number, total: number) => `${(value / total) * 100}%`;
  const contentTop = A4_PAGE.margin + A4_PAGE.headerHeight;
  const contentHeight = A4_PAGE.height - A4_PAGE.margin * 2 - A4_PAGE.headerHeight - A4_PAGE.footerHeight;
  const dividerStyles: readonly CSSProperties[] = (isColumnArithmeticWorksheet ? [] : sharedLayout.dividerXs).map((dividerX) => ({
    left: toPagePercent(dividerX, A4_PAGE.width),
    top: toPagePercent(contentTop, A4_PAGE.height),
    height: toPagePercent(contentHeight, A4_PAGE.height),
  }));
  const footerStyle: CSSProperties = {
    right: toPagePercent(A4_PAGE.margin, A4_PAGE.width),
    bottom: toPagePercent(A4_PAGE.margin, A4_PAGE.height),
  };

  return (
    <section className={`worksheet-screen ${selectedProblem ? 'worksheet-input-open' : ''}`} aria-labelledby="worksheet-title">
      <div className="ribbon">
        <div>
          <p className="ribbon-label" aria-label={worksheetCategoryLabel}><RubyMessage text={worksheetCategoryLabel} /></p>
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
        <article className={`paper ${gradeBandClass}`} style={{ aspectRatio: `${A4_PAGE.width} / ${A4_PAGE.height}`, ...(isColumnArithmeticWorksheet ? columnArithmeticPageGridVariables() : {}) }} aria-label={`${worksheet.layout.problem_count}問の${worksheetTheme.worksheet.title}ワークシート`}>
          <div className={`problem-grid ${isColumnArithmeticWorksheet ? 'problem-grid-column-arithmetic' : ''}`}>
            {worksheetTheme.worksheet.instruction ? (
              <p className="worksheet-instruction">{worksheetTheme.worksheet.instruction}</p>
            ) : null}
            {dividerStyles.map((style, index) => (
              <div
                className="problem-divider"
                data-testid={index === 0 ? 'problem-divider' : `problem-divider-${index + 1}`}
                style={style}
                key={`divider-${index}`}
              />
            ))}
            {sharedLayout.cells.map((cell) => {
              const { problem, index } = cell;
              const answer = answers[problem.problem_id] ?? ({ type: 'empty' } satisfies AnswerNode);
              const isSelected = selectedIndex === index;
              const result = resultById.get(problem.problem_id);
              const position = getCellTopPosition(sharedLayout, cell);
              const isLinearEquation = isEquationWorksheet;
              const isLiarPuzzle = problem.prompt.kind === 'liar_puzzle';
              const isColumnArithmetic = problem.prompt.kind === 'column_arithmetic';
              const stackAnswerBelow = worksheetTheme.worksheet.answerPlacement === 'below' && !isLinearEquation;
              const cellStyle: CSSProperties = {
                left: toPagePercent(position.x, A4_PAGE.width),
                top: toPagePercent(position.y, A4_PAGE.height),
                width: toPagePercent(position.width, A4_PAGE.width),
                height: toPagePercent(position.height, A4_PAGE.height),
                ...(isColumnArithmetic ? columnArithmeticGridVariables(problem, position) : {}),
              };
              return (
                <div className={`problem-cell ${isLinearEquation ? 'problem-cell-linear-equation' : ''} ${isLiarPuzzle ? 'problem-cell-liar' : ''} ${isColumnArithmetic ? `problem-cell-column-arithmetic problem-cell-column-arithmetic-${problem.prompt.kind === 'column_arithmetic' ? problem.prompt.operator : ''}` : ''} ${stackAnswerBelow ? 'problem-cell-answer-below' : ''} ${result ? 'problem-cell-graded' : ''}`} data-layout-index={index} data-layout-column={cell.column} data-problem-index={index} data-testid={`problem-cell-${index}`} style={cellStyle} key={problem.problem_id}>
                  <span className="problem-number">{index + 1}.</span>
                  <span className="expression"><ProblemExpression problem={problem} includeAnswerEquals={!stackAnswerBelow} /></span>
                  <WorksheetAnswerField
                      worksheetUi={worksheetUi}
                      problem={problem}
                      index={index}
                      answer={answer}
                      isSelected={isSelected}
                      selectedSlot={selectedSlot}
                      selectedColumnDigit={selectedColumnDigit}
                      columnDrafts={columnDrafts}
                      result={result}
                      gradeResult={gradeResult}
                      inputLocked={worksheetPhase !== 'editing'}
                      answerPrefix={worksheetTheme.worksheet.answerPrefix}
                      onSelect={onSelect}
                      onSelectColumnDigit={onSelectColumnDigit}
                      onRegisterMathfield={onRegisterMathfield}
                      onMathInput={onMathInput}
                      onCommit={onCommit}
                      onTogglePerson={onTogglePerson}
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
        <div className={`input-panel ${columnDigitInputActive ? 'input-panel-column-digits' : ''}`} aria-label="数式入力パネル">
          <button type="button" className="input-panel-close" onClick={onCloseInput} aria-label="入力パネルを閉じる" title="入力パネルを閉じる">
            <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false"><path d="M5 9l7 7 7-7" /></svg>
          </button>
          <div className={`input-panel-inner ${juniorHighFullKeypad ? 'input-panel-inner-junior-high' : arithmeticOperatorsEnabled ? 'input-panel-inner-algebraic' : ''}`}>
            {visibleStructures.length > 0 ? (
              <div className={`formula-keypad ${juniorHighFullKeypad ? 'formula-keypad-junior-high' : arithmeticOperatorsEnabled ? 'formula-keypad-algebraic' : ''}`} aria-label={juniorHighFullKeypad || arithmeticOperatorsEnabled ? '数式キー' : '数式テンプレート'}>
                {visibleStructures.map((structure) => {
                  const label = structure === 'tuple' && selectedProblem.answer_schema.kind === 'ordered_pair'
                    ? 'x, y'
                    : STRUCTURE_LABELS[structure];
                  return (
                    <button
                      type="button"
                      className="formula-structure-key"
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
            <div
              className={`keypad-numbers ${juniorHighFullKeypad ? 'keypad-numbers-junior-high' : arithmeticOperatorsEnabled ? 'keypad-numbers-algebraic' : ''} ${!juniorHighFullKeypad && arithmeticOperatorsEnabled && selectedCapabilities?.allow_decimal ? 'keypad-numbers-algebraic-decimal' : ''}`}
              aria-label="数字キー"
            >
              {[7, 8, 9, 4, 5, 6, 1, 2, 3, 0].map((digit) => (
                <button
                  type="button"
                  className={digit === 0 ? 'keypad-zero' : 'keypad-digit'}
                  key={digit}
                  onClick={() => onCommand({ kind: 'insert_digit', digit })}
                  disabled={busy}
                >
                  {digit}
                </button>
              ))}
              {!columnDigitInputActive && (juniorHighFullKeypad || selectedCapabilities?.allow_decimal) ? (
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
            {juniorHighFullKeypad ? (
              <div className="keypad-operators" aria-label="演算子キー">
                <button type="button" onClick={() => onCommand({ kind: 'insert_latex', latex: '+' })} disabled={busy} aria-label="プラスを挿入">+</button>
                <button type="button" onClick={() => onCommand({ kind: 'insert_latex', latex: '-' })} disabled={busy} aria-label="マイナスを挿入">−</button>
                <button type="button" onClick={() => onCommand({ kind: 'insert_latex', latex: '\\pm' })} disabled={busy} aria-label="プラスマイナスを挿入">±</button>
              </div>
            ) : null}
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
