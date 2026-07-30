'use client';

import { useCallback, useEffect, useMemo, useRef, useState, type CSSProperties } from 'react';

import {
  ADDITION_CURRICULUM_PATH,
  ADDITION_LAYOUT,
  DEFAULT_ADDITION_SETTINGS,
  DrillEngineError,
  emptyEditorState,
  formatCurriculumPath,
  editorValue,
  type DrillEngine,
  type DrillSettings,
  type EditorAction,
  type EditorState,
  type GradeResult,
  type WorksheetDto,
} from '@/domain/drill-engine';
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

export type AutoDrillAppProps = {
  engine?: DrillEngine;
  initialSettings?: DrillSettings;
  seedGenerator?: () => string;
  dateGenerator?: WorksheetDateGenerator;
};

function formatElapsed(startedAt: number | null, now: number): string {
  if (!startedAt) return '00:00';
  const seconds = Math.max(0, Math.floor((now - startedAt) / 1000));
  return `${String(Math.floor(seconds / 60)).padStart(2, '0')}:${String(seconds % 60).padStart(2, '0')}`;
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
  const [now, setNow] = useState(() => Date.now());
  const [gradeResult, setGradeResult] = useState<GradeResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const answersRef = useRef<Record<string, EditorState>>({});
  const actionQueueRef = useRef(Promise.resolve());

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
    if (!startedAt) return undefined;
    const timer = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(timer);
  }, [startedAt]);

  const curriculumLabel = useMemo(() => formatCurriculumPath(settings.curriculum_path), [settings.curriculum_path]);

  const showEngineError = useCallback((value: unknown) => {
    if (value instanceof DrillEngineError) {
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
  }, []);

  const generate = useCallback(async (printAfterGeneration: boolean) => {
    setError(null);
    setGradeResult(null);
    setBusy(true);
    const pdfTarget = printAfterGeneration && typeof window !== 'undefined' ? window.open('about:blank', '_blank') : null;
    try {
      const seed = settings.seed === '' ? seedGenerator() : settings.seed;
      const metadata = createWorksheetMetadata(seed, dateGenerator());
      const generatedWorksheet = await engine.generateWorksheet({ ...settings, seed, layout: ADDITION_LAYOUT });
      // The Rust DTO remains the source of the problems. The spread adds the
      // exact seed used by this UI invocation when a fixture/runtime returns a
      // stale or normalized seed string.
      const nextWorksheet = { ...generatedWorksheet, seed };
      const nextAnswers = Object.fromEntries(nextWorksheet.problems.map((problem) => [problem.problem_id, emptyEditorState()]));
      answersRef.current = nextAnswers;
      setWorksheet(nextWorksheet);
      setWorksheetMetadata(metadata);
      setAnswers(nextAnswers);
      // q2 starts with no active editor. The keypad/control panel appears
      // only after the learner clicks a problem row; commit then advances to
      // the next row and keeps the panel active there.
      setSelectedIndex(null);
      setStartedAt(printAfterGeneration ? null : Date.now());
      setNow(Date.now());
      if (printAfterGeneration) await openWorksheetPdf(nextWorksheet, pdfTarget, metadata);
      setScreen(printAfterGeneration ? 'settings' : 'worksheet');
    } catch (value) {
      pdfTarget?.close();
      showEngineError(value);
    } finally {
      setBusy(false);
    }
  }, [dateGenerator, engine, seedGenerator, settings, showEngineError]);

  const selectProblem = useCallback((index: number) => {
    setSelectedIndex(index);
    setError(null);
  }, []);

  const applyAction = useCallback((action: EditorAction, index = selectedIndex) => {
    const run = async () => {
      if (!worksheet || index === null || !worksheet.problems[index]) return;
      const problemId = worksheet.problems[index].problem_id;
      const current = answersRef.current[problemId] ?? emptyEditorState();
      setBusy(true);
      setError(null);
      try {
        const next = await engine.applyEditorAction(current, action);
        const nextAnswers = { ...answersRef.current, [problemId]: next };
        answersRef.current = nextAnswers;
        setAnswers(nextAnswers);
        if (action.kind === 'commit' && index < worksheet.problems.length - 1) {
          setSelectedIndex(index + 1);
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
  }, [engine, selectedIndex, showEngineError, worksheet]);

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
      if (event.key >= '0' && event.key <= '9') {
        event.preventDefault();
        void applyAction({ kind: 'insert_digit', digit: Number(event.key) });
      } else if (event.key === 'Enter') {
        event.preventDefault();
        void applyAction({ kind: 'commit' });
      } else if (event.key === 'Backspace') {
        event.preventDefault();
        void applyAction({ kind: 'delete_backward' });
      } else if (event.key === 'Delete') {
        event.preventDefault();
        void applyAction({ kind: 'delete_forward' });
      } else if (event.key === 'ArrowLeft') {
        event.preventDefault();
        void applyAction({ kind: 'move_left' });
      } else if (event.key === 'ArrowRight') {
        event.preventDefault();
        void applyAction({ kind: 'move_right' });
      }
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [applyAction, screen, selectedIndex]);

  const grade = useCallback(async () => {
    if (!worksheet) return;
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
      setBusy(false);
    }
  }, [drainActionQueue, engine, showEngineError, worksheet]);

  const backToTop = useCallback(() => {
    setScreen('settings');
    setSelectedIndex(null);
    setStartedAt(null);
    setGradeResult(null);
    setError(null);
  }, []);

  return (
    <main className="app-shell">
      {screen === 'settings' ? (
        <SettingsScreen
          settings={settings}
          curriculumLabel={curriculumLabel}
          busy={busy}
          error={error}
          hasWorksheet={Boolean(worksheet)}
          worksheetMetadata={worksheetMetadata}
          onSettingsChange={setSettings}
          onGenerate={() => void generate(false)}
          onPrint={() => void generate(true)}
        />
      ) : worksheet ? (
        <WorksheetScreen
          worksheet={worksheet}
          worksheetMetadata={worksheetMetadata}
          answers={answers}
          selectedIndex={selectedIndex}
          elapsed={formatElapsed(startedAt, now)}
          gradeResult={gradeResult}
          busy={busy}
          error={error}
          onSelect={selectProblem}
          onAction={(action) => void applyAction(action)}
          onGrade={() => void grade()}
          onPrint={() => {
            void Promise.resolve(openWorksheetPdf(worksheet, undefined, worksheetMetadata ?? undefined)).catch(showEngineError);
          }}
          onBack={backToTop}
        />
      ) : null}
    </main>
  );
}

type SettingsScreenProps = {
  settings: DrillSettings;
  curriculumLabel: string;
  busy: boolean;
  error: string | null;
  hasWorksheet: boolean;
  worksheetMetadata: WorksheetMetadata | null;
  onSettingsChange: (settings: DrillSettings) => void;
  onGenerate: () => void;
  onPrint: () => void;
};

function SettingsScreen({ settings, curriculumLabel, busy, error, hasWorksheet, worksheetMetadata, onSettingsChange, onGenerate, onPrint }: SettingsScreenProps) {
  return (
    <section className="settings-screen" aria-labelledby="settings-title">
      <header className="page-heading">
        <p className="eyebrow">AutoDrill alpha 1.0</p>
        <h1 id="settings-title">計算ドリルをつくる</h1>
        <p className="description">白黒のプリントで、今日の20問に取り組みます。</p>
      </header>

      <div className="settings-card">
        <label className="field-label" htmlFor="grade-select">学年</label>
        <select id="grade-select" className="select-field" value="grade-1" onChange={() => undefined}>
          <option value="grade-1">小学1年生</option>
        </select>

        <label className="field-label" htmlFor="curriculum-select">カリキュラム</label>
        <select id="curriculum-select" className="select-field" value={settings.skill_id} onChange={() => undefined}>
          <option value={settings.skill_id}>{curriculumLabel}</option>
        </select>
        <p className="field-note">階層: {ADDITION_CURRICULUM_PATH.map((segment) => segment.label).join(' › ')}</p>

        <label className="field-label" htmlFor="difficulty-select">難易度</label>
        <select id="difficulty-select" className="select-field" value="default" onChange={() => undefined}>
          <option value="default">標準（準備中）</option>
        </select>

        <div className="fixed-count" aria-label="問題数20問">
          <span>問題数</span>
          <strong>20問</strong>
        </div>
        <label className="field-label" htmlFor="seed-input">Seed</label>
        <input
          id="seed-input"
          className="text-field"
          value={settings.seed}
          onChange={(event) => onSettingsChange({ ...settings, seed: event.target.value })}
          placeholder="空欄なら毎回自動生成"
          autoComplete="off"
          spellCheck={false}
        />
        <p className="field-note">空欄なら「問題生成」または「印刷」のたびに新しいSeedを使います。</p>
      </div>

      {error ? <p className="error-message" role="alert">{error}</p> : null}
      {hasWorksheet && worksheetMetadata ? (
        <p className="muted-message" data-testid="last-worksheet-metadata">
          前回: {formatWorksheetFooter(worksheetMetadata)}
        </p>
      ) : null}

      <div className="settings-actions">
        <button type="button" className="primary-button" disabled={busy} onClick={onGenerate}>問題生成</button>
        <button type="button" className="secondary-button" disabled={busy} onClick={onPrint}>印刷</button>
      </div>
      <p className="wasm-note">問題の生成・入力状態・採点は Rust/WASM が担当します。</p>
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
  onSelect: (index: number) => void;
  onAction: (action: EditorAction) => void;
  onGrade: () => void;
  onPrint: () => void;
  onBack: () => void;
};

function WorksheetScreen({ worksheet, worksheetMetadata, answers, selectedIndex, elapsed, gradeResult, busy, error, onSelect, onAction, onGrade, onPrint, onBack }: WorksheetScreenProps) {
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
    <section className="worksheet-screen" aria-labelledby="worksheet-title">
      <div className="ribbon">
        <div>
          <p className="ribbon-label">小学1年生</p>
          <h1 id="worksheet-title">1けたのたしざん(1)</h1>
        </div>
        <div className="ribbon-meta"><span>回答時間</span><strong>{elapsed}</strong></div>
        <button type="button" className="ribbon-button" onClick={onGrade} disabled={busy}>採点</button>
        <button type="button" className="ribbon-icon" onClick={onPrint} aria-label="印刷" disabled={busy}>印刷</button>
        <button type="button" className="ribbon-link" onClick={onBack}>TOPに戻る</button>
      </div>

      {error ? <p className="error-message worksheet-error" role="alert">{error}</p> : null}
      {gradeResult ? <div className="grade-summary" role="status"><strong>{gradeResult.correct_count} / {gradeResult.total_count}</strong><span>正解</span></div> : null}

      <div className="paper-wrap">
        <article className="paper" style={{ aspectRatio: `${A4_PAGE.width} / ${A4_PAGE.height}` }} aria-label="20問の一桁足し算ワークシート">
          <div className="problem-grid">
            <div className="problem-divider" data-testid="problem-divider" style={dividerStyle} />
            {sharedLayout.cells.map((cell) => {
              const { problem, index } = cell;
              const editor = answers[problem.problem_id] ?? emptyEditorState();
              const result = resultById.get(problem.problem_id);
              const position = getCellTopPosition(sharedLayout, cell);
              const cellStyle: CSSProperties = {
                left: toPagePercent(position.x, A4_PAGE.width),
                top: toPagePercent(position.y, A4_PAGE.height),
                width: toPagePercent(position.width, A4_PAGE.width),
                height: toPagePercent(position.height, A4_PAGE.height),
              };
              return (
                <div className="problem-cell" data-layout-index={index} data-testid={`problem-cell-${index}`} style={cellStyle} key={problem.problem_id}>
                  <span className="problem-number">{index + 1}.</span>
                  <span className="expression">{problemExpression(problem)}</span>
                  <button
                    type="button"
                    className={`answer-box ${selectedIndex === index ? 'answer-box-selected' : ''} ${result ? (result.correct ? 'answer-box-correct' : 'answer-box-wrong') : ''}`}
                    onClick={() => onSelect(index)}
                    aria-label={`${index + 1}番の答え ${editorValue(editor) ?? '未入力'}`}
                  >
                    {editorValue(editor) ?? ''}
                  </button>
                  {result ? <span className="result-mark" aria-label={result.correct ? '正解' : '不正解'}>{result.correct ? '○' : '×'}</span> : null}
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
          <div className="ast-state" aria-live="polite">AST: integer({editorValue(selectedState) ?? 'empty'})</div>
          <div className="keypad">
            {[1, 2, 3, 4, 5, 6, 7, 8, 9, 0].map((digit) => <button type="button" key={digit} onClick={() => onAction({ kind: 'insert_digit', digit })} disabled={busy}>{digit}</button>)}
            <button type="button" onClick={() => onAction({ kind: 'delete_backward' })} disabled={busy}>⌫</button>
            <button type="button" onClick={() => onAction({ kind: 'delete_forward' })} disabled={busy}>削除</button>
            <button type="button" onClick={() => onAction({ kind: 'move_left' })} disabled={busy}>←</button>
            <button type="button" onClick={() => onAction({ kind: 'move_right' })} disabled={busy}>→</button>
            <button type="button" onClick={() => onAction({ kind: 'clear' })} disabled={busy}>クリア</button>
            <button type="button" className="keypad-commit" onClick={() => onAction({ kind: 'commit' })} disabled={busy}>確定</button>
          </div>
        </div>
      ) : null}
    </section>
  );
}
