'use client';

import { createContext, useCallback, useContext, useEffect, useRef, useState, type CSSProperties } from 'react';

import {
  ADDITION_LAYOUT,
  DEFAULT_ADDITION_SETTINGS,
  DrillEngineError,
  emptyEditorState,
  editorValue,
  type DrillEngine,
  type DrillSettings,
  type EditorAction,
  type EditorState,
  type GradeResult,
  type WorksheetDto,
} from '@/domain/drill-engine';
import { CURRICULUM_TREE, findCurriculumSelection, type CurriculumUnit } from '@/domain/curriculum';
import { RubyText, type RubyPart } from '@/components/RubyText';
import { problemExpression, openWorksheetPdf } from '@/pdf/worksheet-pdf';
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
  '学年': [["学年", "がくねん"]],
  '小学1年生': [["小学", "しょうがく"], '1', ["年生", "ねんせい"]],
  '領域': [["領域", "りょういき"]],
  '数と計算': [["数", "かず"], 'と', ["計算", "けいさん"]],
  '単元': [["単元", "たんげん"]],
  '難易度': [["難易度", "なんいど"]],
  '標準（準備中）': [["標準", "ひょうじゅん"], '（', ["準備中", "じゅんびちゅう"], '）'],
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
  seedGenerator?: () => string;
  dateGenerator?: WorksheetDateGenerator;
};

function formatElapsed(startedAt: number | null, now: number): string {
  if (startedAt === null) return '00:00';
  const seconds = Math.max(0, Math.floor((now - startedAt) / 1000));
  return `${String(Math.floor(seconds / 60)).padStart(2, '0')}:${String(seconds % 60).padStart(2, '0')}`;
}

function editorActionForKey(event: KeyboardEvent): EditorAction | null {
  if (event.isComposing || event.altKey || event.ctrlKey || event.metaKey) return null;
  if (event.key >= '0' && event.key <= '9') {
    return { kind: 'insert_digit', digit: Number(event.key) };
  }
  if (event.key === 'Enter') return { kind: 'commit' };
  if (event.key === 'Backspace') return { kind: 'delete_backward' };
  if (event.key === 'Delete') return { kind: 'delete_forward' };
  if (event.key === 'ArrowLeft') return { kind: 'move_left' };
  if (event.key === 'ArrowRight') return { kind: 'move_right' };
  return null;
}

function answerFontSize(digitCount: number): number {
  if (digitCount <= 2) return 20;
  return Math.max(11, 20 - (digitCount - 2) * 0.5625);
}

function answerBoxWidth(digitCount: number, withCaret: boolean): number {
  const compactWidth = 42;
  const contentWidth = 12 + digitCount * 7 + (withCaret ? 2 : 0);
  return Math.max(compactWidth, contentWidth);
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
  initialSettings = DEFAULT_ADDITION_SETTINGS,
  seedGenerator = generateAutomaticSeed,
  dateGenerator = () => new Date(),
}: AutoDrillAppProps) {
  const engine = injectedEngine ?? createWasmDrillEngine();
  const [screen, setScreen] = useState<Screen>('settings');
  const [settings, setSettings] = useState<DrillSettings>(initialSettings);
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
  // Default ON keeps the server and first client render identical. The saved
  // browser preference is applied only after hydration.
  const [furiganaEnabled, setFuriganaEnabled] = useState(true);
  const answersRef = useRef<Record<string, EditorState>>({});
  const selectedIndexRef = useRef<number | null>(null);
  const inputEnabledRef = useRef(false);
  const actionQueueRef = useRef(Promise.resolve());
  const noticeTimerRef = useRef<number | null>(null);

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
    setError(null);
    setGradeResult(null);
    dismissNotice();
    setBusy(true);
    setSettingsBusyAction(printAfterGeneration ? 'print' : 'generate');
    const pdfTarget = printAfterGeneration && typeof window !== 'undefined' ? window.open('about:blank', '_blank') : null;
    try {
      const seed = settings.seed === '' ? seedGenerator() : settings.seed;
      const metadata = createWorksheetMetadata(seed, dateGenerator());
      const generatedWorksheet = await engine.generateWorksheet({ ...settings, seed, layout: ADDITION_LAYOUT });
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
  }, [dateGenerator, dismissNotice, engine, installWorksheet, seedGenerator, settings, showEngineError]);

  const selectProblem = useCallback((index: number) => {
    inputEnabledRef.current = true;
    selectedIndexRef.current = index;
    setSelectedIndex(index);
    setError(null);
    dismissNotice();
  }, [dismissNotice]);

  const applyAction = useCallback((action: EditorAction, requestedIndex?: number) => {
    const run = async () => {
      const index = requestedIndex ?? selectedIndexRef.current;
      if (!worksheet || index === null || !worksheet.problems[index]) return;
      const problemId = worksheet.problems[index].problem_id;
      const current = answersRef.current[problemId] ?? emptyEditorState();
      setBusy(true);
      setError(null);
      dismissNotice();
      try {
        const next = await engine.applyEditorAction(current, action);
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
      const action = editorActionForKey(event);
      if (!action) return;
      // Capture + preventDefault avoids Enter activating a focused keypad or
      // ribbon button in addition to committing the selected answer.
      event.preventDefault();
      void applyAction(action);
    };
    window.addEventListener('keydown', onKeyDown, true);
    return () => window.removeEventListener('keydown', onKeyDown, true);
  }, [applyAction, screen, selectedIndex]);

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
        schema_version: 1,
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
      const generatedWorksheet = await engine.generateWorksheet({ ...settings, seed, layout: ADDITION_LAYOUT });
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
            furiganaEnabled={furiganaEnabled}
            onSettingsChange={setSettings}
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
  furiganaEnabled: boolean;
  onSettingsChange: (settings: DrillSettings) => void;
  onFuriganaChange: (enabled: boolean) => void;
  onGenerate: () => void;
  onPrint: () => void;
};

function SettingsScreen({ settings, busy, busyAction, error, hasWorksheet, worksheetMetadata, furiganaEnabled, onSettingsChange, onFuriganaChange, onGenerate, onPrint }: SettingsScreenProps) {
  const selection = findCurriculumSelection(settings.skill_id);
  const selectUnit = (unit: CurriculumUnit) => {
    onSettingsChange({
      ...settings,
      skill_id: unit.skillId,
      curriculum_path: unit.curriculumPath,
    });
  };

  const selectGrade = (gradeId: string) => {
    const grade = CURRICULUM_TREE.find((candidate) => candidate.id === gradeId) ?? CURRICULUM_TREE[0];
    selectUnit(grade.areas[0].units[0]);
  };

  const selectArea = (areaId: string) => {
    const area = selection.grade.areas.find((candidate) => candidate.id === areaId) ?? selection.grade.areas[0];
    selectUnit(area.units[0]);
  };

  const selectCurriculumUnit = (unitId: string) => {
    const unit = selection.area.units.find((candidate) => candidate.id === unitId) ?? selection.area.units[0];
    selectUnit(unit);
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
          <p className="eyebrow"><span aria-hidden="true" /> AutoDrill alpha 1.0</p>
          <h1 id="settings-title" aria-label="計算ドリルをつくる"><RubyMessage text="計算ドリルをつくる" /></h1>
        </header>

        <div className="settings-card">
          <div className="curriculum-fields" aria-label="出題範囲">
            <div className="field-group">
              <label className="field-label" htmlFor="grade-select"><RubyMessage text="学年" /></label>
              <div className="ruby-select">
                <select id="grade-select" className="select-field" aria-label="学年" value={selection.grade.id} onChange={(event) => selectGrade(event.target.value)}>
                  {CURRICULUM_TREE.map((grade) => <option value={grade.id} key={grade.id}>{grade.label}</option>)}
                </select>
                <span className="ruby-select-display" aria-hidden="true"><RubyMessage text={selection.grade.label} /></span>
              </div>
            </div>

            <div className="field-group">
              <label className="field-label" htmlFor="area-select"><RubyMessage text="領域" /></label>
              <div className="ruby-select">
                <select id="area-select" className="select-field" aria-label="領域" value={selection.area.id} onChange={(event) => selectArea(event.target.value)}>
                  {selection.grade.areas.map((area) => <option value={area.id} key={area.id}>{area.label}</option>)}
                </select>
                <span className="ruby-select-display" aria-hidden="true"><RubyMessage text={selection.area.label} /></span>
              </div>
            </div>

            <div className="field-group field-group-unit">
              <label className="field-label" htmlFor="unit-select"><RubyMessage text="単元" /></label>
              <div className="ruby-select">
                <select id="unit-select" className="select-field" aria-label="単元" value={selection.unit.id} onChange={(event) => selectCurriculumUnit(event.target.value)}>
                  {selection.area.units.map((unit) => <option value={unit.id} key={unit.id}>{unit.label}</option>)}
                </select>
                <span className="ruby-select-display" aria-hidden="true">{selection.unit.label}</span>
              </div>
            </div>
          </div>

          <div className="settings-options">
            <div className="field-group">
              <label className="field-label" htmlFor="difficulty-select"><RubyMessage text="難易度" /></label>
              <div className="ruby-select">
                <select id="difficulty-select" className="select-field" aria-label="難易度" value="default" onChange={() => undefined}>
                  <option value="default">標準（準備中）</option>
                </select>
                <span className="ruby-select-display" aria-hidden="true"><RubyMessage text="標準（準備中）" /></span>
              </div>
            </div>

            <div className="fixed-count" aria-label="問題数20問">
              <span><RubyMessage text="問題数" /></span>
              <strong>20<span><RubyMessage text="問" /></span></strong>
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

        {error ? <p className="error-message" role="alert" aria-label={error}><RubyMessage text={error} /></p> : null}
        {hasWorksheet && worksheetMetadata ? (
          <p className="muted-message" data-testid="last-worksheet-metadata">
            <RubyMessage text="前回" />: {formatWorksheetFooter(worksheetMetadata)}
          </p>
        ) : null}

        <div className="settings-actions">
          <button type="button" className="primary-button" aria-label={busyAction === 'generate' ? '問題を生成中…' : '問題生成'} disabled={busy} onClick={onGenerate}>
            <span className="button-icon" aria-hidden="true">▶</span>
            <RubyMessage text={busyAction === 'generate' ? '問題を生成中…' : '問題生成'} />
          </button>
          <button type="button" className="secondary-button" aria-label={busyAction === 'print' ? 'PDFを準備中…' : '印刷'} disabled={busy} onClick={onPrint}>
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
  const selectedProblem = selectedIndex === null ? null : worksheet.problems[selectedIndex];
  const selectedState = selectedProblem ? answers[selectedProblem.problem_id] ?? emptyEditorState() : null;
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
          <p className="ribbon-label" aria-label="小学1年生"><RubyMessage text="小学1年生" /></p>
          <h1 id="worksheet-title">1けたのたしざん(1)</h1>
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
        <article className="paper" style={{ aspectRatio: `${A4_PAGE.width} / ${A4_PAGE.height}` }} aria-label="20問の一桁足し算ワークシート">
          <div className="problem-grid">
            <div className="problem-divider" data-testid="problem-divider" style={dividerStyle} />
            {sharedLayout.cells.map((cell) => {
              const { problem, index } = cell;
              const editor = answers[problem.problem_id] ?? emptyEditorState();
              const answer = editorValue(editor) ?? '';
              const isSelected = selectedIndex === index;
              const cursor = Math.min(editor.cursor, answer.length);
              const result = resultById.get(problem.problem_id);
              const position = getCellTopPosition(sharedLayout, cell);
              const cellStyle: CSSProperties = {
                left: toPagePercent(position.x, A4_PAGE.width),
                top: toPagePercent(position.y, A4_PAGE.height),
                width: toPagePercent(position.width, A4_PAGE.width),
                height: toPagePercent(position.height, A4_PAGE.height),
              };
              const answerStyle: CSSProperties = {
                width: answerBoxWidth(answer.length, isSelected),
                fontSize: answerFontSize(answer.length),
                flexGrow: 0,
                flexShrink: 1,
              };
              return (
                <div className={`problem-cell ${result ? 'problem-cell-graded' : ''}`} data-layout-index={index} data-layout-column={cell.column} data-problem-index={index} data-testid={`problem-cell-${index}`} style={cellStyle} key={problem.problem_id}>
                  <span className="problem-number">{index + 1}.</span>
                  <span className="expression">{problemExpression(problem)}</span>
                  <button
                    type="button"
                    className={`answer-box ${isSelected ? 'answer-box-selected' : ''} ${result ? (result.correct ? 'answer-box-correct' : 'answer-box-wrong') : ''}`}
                    data-answer-length={answer.length}
                    style={answerStyle}
                    onClick={() => onSelect(index)}
                    disabled={Boolean(gradeResult)}
                    aria-label={`${index + 1}番の答え ${answer || '未入力'}`}
                  >
                    <span className="answer-value" aria-hidden="true">
                      {isSelected ? (
                        <>
                          <span data-testid={`answer-before-caret-${index}`}>{answer.slice(0, cursor)}</span>
                          <span className="answer-caret" data-testid={`answer-caret-${index}`} />
                          <span data-testid={`answer-after-caret-${index}`}>{answer.slice(cursor)}</span>
                        </>
                      ) : answer}
                    </span>
                  </button>
                  {result?.correct ? <span className="result-mark" aria-label="正解">○</span> : null}
                  {result && !result.correct ? (
                    <span className="correct-answer" aria-label={`正しい答え ${problem.canonical_answer.value}`}>
                      {problem.canonical_answer.value}
                    </span>
                  ) : null}
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
        <div className="input-panel" aria-label="数字入力パネル">
          <div className="input-panel-inner">
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
            </div>
            <div className="keypad-controls" aria-label="編集キー">
              <button type="button" onClick={() => onAction({ kind: 'delete_backward' })} disabled={busy} aria-label="一文字戻す">⌫</button>
              <button type="button" onClick={() => onAction({ kind: 'move_left' })} disabled={busy} aria-label="カーソルを左へ">←</button>
              <button type="button" onClick={() => onAction({ kind: 'move_right' })} disabled={busy} aria-label="カーソルを右へ">→</button>
              <button type="button" className="keypad-clear" onClick={() => onAction({ kind: 'clear' })} disabled={busy}>クリア</button>
              <button type="button" className="keypad-commit" aria-label="確定" onClick={() => onAction({ kind: 'commit' })} disabled={busy}><RubyMessage text="確定" /></button>
            </div>
          </div>
        </div>
      ) : null}
    </section>
  );
}
