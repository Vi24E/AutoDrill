const app = document.querySelector('#app');
const nav = document.querySelector('#nav');
const statusStrip = document.querySelector('#status-strip');
const toastElement = document.querySelector('#toast');

const model = {
  state: null, view: 'evaluate', selectedRating: null,
  ratingStartedMonotonic: null, ratingEventPromise: null, starting: false, saving: false,
  units: [], selectedSkillId: null, prefetch: null, activeRenderOverlay: null,
  samplingMode: localStorage.getItem('autodrill.qa.samplingMode') === 'custom' ? 'custom' : 'random',
};

const h = (value) => String(value ?? '').replace(/[&<>'"]/g, (char) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', "'": '&#39;', '"': '&quot;' })[char]);
const parsedJson = (value, fallback = {}) => { try { return JSON.parse(value) ?? fallback; } catch { return fallback; } };
const fmt = (value) => value ? new Intl.DateTimeFormat('ja-JP', { dateStyle: 'short', timeStyle: 'medium' }).format(new Date(value)) : '—';
const eventStamp = () => ({ client_wall_at: new Date().toISOString(), client_monotonic_ms: performance.now() });
const scaleValues = () => {
  const { min, max } = model.state.metadata.ratingScale;
  return Array.from({ length: max - min + 1 }, (_, index) => min + index);
};
const ratingMidpoint = () => {
  const { min, max } = model.state.metadata.ratingScale;
  return min + Math.floor((max - min) / 2);
};
const ratingCoordinate = (value) => value - ratingMidpoint();
const coordinateLabel = (value) => ratingCoordinate(value) > 0 ? `+${ratingCoordinate(value)}` : String(ratingCoordinate(value));
const clamp01 = (value) => Math.max(0, Math.min(1, value));
const ratingFromPosition = (position) => {
  const { min, max } = model.state.metadata.ratingScale;
  return Math.round(min + clamp01(position) * (max - min));
};
const continuousCoordinateLabel = (position) => {
  const { min, max } = model.state.metadata.ratingScale;
  const coordinate = Math.round(((clamp01(position) - 0.5) * (max - min)) * 100) / 100;
  if (coordinate === 0) return '0';
  return coordinate > 0 ? `+${coordinate}` : String(coordinate);
};
const evaluationCoordinate = (evaluation, axis) => {
  const position = evaluation?.[`${axis}_position`];
  return position == null ? coordinateLabel(evaluation?.[`${axis}_rating`]) : continuousCoordinateLabel(position);
};
const selectionStatus = (selection) => selection
  ? `<strong>難しさ ${continuousCoordinateLabel(selection.difficulty_position)}</strong><span>特異性 ${continuousCoordinateLabel(selection.singularity_position)}</span><small>区分 ${selection.difficulty}, ${selection.singularity}</small>`
  : 'まだ選択されていません';

async function api(path, options = {}) {
  const response = await fetch(path, {
    ...options,
    headers: { ...(options.body ? { 'Content-Type': 'application/json', 'X-AutoDrill-QA': '1' } : {}), ...options.headers },
  });
  const contentType = response.headers.get('content-type') ?? '';
  const payload = contentType.includes('json') ? await response.json() : await response.text();
  if (!response.ok) throw new Error(payload.error ?? `HTTP ${response.status}`);
  return payload;
}

function toast(message) {
  toastElement.textContent = message;
  toastElement.hidden = false;
  clearTimeout(toastElement.timer);
  toastElement.timer = setTimeout(() => { toastElement.hidden = true; }, 3200);
}

async function refresh() {
  [model.state, model.units] = await Promise.all([api('/api/state'), api('/api/quick/units')]);
  const active = model.state.activeAttempt;
  if (active?.source_skill_id) model.selectedSkillId = active.source_skill_id;
  if (active && active.observation_mode !== 'rating_only_answer_shown') {
    await startAttempt();
    return;
  }
  model.view = 'evaluate';
  model.selectedRating = active?.rating_draft ?? null;
  model.ratingStartedMonotonic = active ? performance.now() - Math.max(0, Date.now() - Date.parse(active.rating_started_at)) : null;
  updateChrome();
  render();
}

function updateChrome() {
  const active = model.state?.activeAttempt;
  statusStrip.innerHTML = `<div>単元を選び、答えを見て評価するだけ</div><div>${active ? h(active.unit_name) : '出題する単元を選択'}</div>`;
  nav.querySelectorAll('button').forEach((button) => {
    button.disabled = false;
    button.classList.toggle('active', button.dataset.view === model.view);
  });
}

function render() {
  if (model.view === 'history') { renderHistory(); return; }
  if (model.state?.activeAttempt) { renderRating(model.state.activeAttempt); return; }
  renderUnitChooser();
}

function renderLoading() {
  removeActiveRenderOverlay();
  app.innerHTML = '<section class="attempt-shell"><div class="panel loading-card"><div class="spinner" aria-hidden="true"></div><h2>次の問題を作っています</h2></div></section>';
}

function createPrefetchBin() {
  const bin = document.createElement('div');
  bin.className = 'print-prefetch-bin';
  bin.setAttribute('aria-hidden', 'true');
  document.body.appendChild(bin);
  return bin;
}

function discardPrefetch() {
  model.prefetch?.resolveReady?.(false);
  model.prefetch?.bin?.remove();
  model.prefetch = null;
}

function removeActiveRenderOverlay() {
  model.activeRenderOverlay?.cleanup?.();
  model.activeRenderOverlay?.bin?.remove();
  model.activeRenderOverlay = null;
}

function showRenderOverlay(bin, shell) {
  const updatePosition = () => {
    if (!bin.isConnected || !shell.isConnected) return;
    const bounds = shell.getBoundingClientRect();
    Object.assign(bin.style, {
      left: `${bounds.left}px`, top: `${bounds.top}px`,
      width: `${bounds.width}px`, height: `${bounds.height}px`,
    });
  };
  updatePosition();
  bin.classList.add('print-render-overlay');
  bin.removeAttribute('aria-hidden');
  window.addEventListener('resize', updatePosition);
  window.addEventListener('scroll', updatePosition, true);
  const resizeObserver = new ResizeObserver(updatePosition);
  resizeObserver.observe(shell);
  return () => {
    resizeObserver.disconnect();
    window.removeEventListener('resize', updatePosition);
    window.removeEventListener('scroll', updatePosition, true);
  };
}

async function prefetchNext() {
  const attempt = model.state?.activeAttempt;
  const skillId = model.selectedSkillId;
  if (!attempt || !skillId || model.view !== 'evaluate' || model.prefetch) return;
  const ownerAttemptId = attempt.id;
  let resolveReady;
  const readyPromise = new Promise((resolve) => { resolveReady = resolve; });
  const samplingMode = model.samplingMode;
  const entry = { id: null, skillId, samplingMode, ownerAttemptId, bin: null, frame: null, ready: false, readyPromise, resolveReady };
  model.prefetch = entry;
  try {
    const reservation = await api('/api/quick/prefetch', { method: 'POST', body: JSON.stringify({ skill_id: skillId, sampling_mode: samplingMode }) });
    if (model.prefetch !== entry || model.state?.activeAttempt?.id !== ownerAttemptId || model.selectedSkillId !== skillId || model.samplingMode !== samplingMode) {
      entry.resolveReady(false);
      return;
    }
    const shell = document.querySelector('.print-render-shell');
    if (!shell) { discardPrefetch(); return; }
    const bin = createPrefetchBin();
    bin.style.width = `${shell.clientWidth}px`;
    bin.style.height = `${shell.clientHeight}px`;
    const frame = document.createElement('iframe');
    frame.className = 'print-prefetch-frame';
    frame.tabIndex = -1;
    frame.title = '次の問題を準備中';
    frame.src = `/renderer/index.html?prefetch=${encodeURIComponent(reservation.id)}`;
    Object.assign(entry, { id: reservation.id, bin, frame });
    bin.replaceChildren(frame);
  } catch {
    if (model.prefetch === entry) discardPrefetch();
  }
}

async function startAttempt() {
  if (model.starting) return;
  if (!model.selectedSkillId && !model.state.activeAttempt) { renderUnitChooser(); return; }
  model.starting = true;
  model.view = 'evaluate';
  let prefetched = model.prefetch?.skillId === model.selectedSkillId && model.prefetch?.samplingMode === model.samplingMode ? model.prefetch : null;
  if (prefetched && !prefetched.ready) {
    const ready = await Promise.race([
      prefetched.readyPromise,
      new Promise((resolve) => setTimeout(() => resolve(false), 8_000)),
    ]);
    if (!ready || model.prefetch !== prefetched) {
      if (model.prefetch === prefetched) discardPrefetch();
      prefetched = null;
    }
  }
  if (!prefetched) renderLoading();
  try {
    const requestAttempt = (prefetchId) => api('/api/quick/next', { method: 'POST', body: JSON.stringify({
      local_timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
      browser_version: navigator.userAgent,
      skill_id: model.selectedSkillId,
      sampling_mode: model.samplingMode,
      ...(prefetchId ? { prefetch_id: prefetchId } : {}),
      ...eventStamp(),
    }) });
    let attempt;
    try { attempt = await requestAttempt(prefetched?.id); }
    catch (error) {
      if (!prefetched) throw error;
      discardPrefetch();
      renderLoading();
      attempt = await requestAttempt();
    }
    model.state.activeAttempt = attempt;
    if (attempt.source_skill_id) model.selectedSkillId = attempt.source_skill_id;
    model.ratingStartedMonotonic = performance.now() - Math.max(0, Date.now() - Date.parse(attempt.rating_started_at));
    model.selectedRating = attempt.rating_draft ?? null;
    updateChrome();
    renderRating(attempt, attempt.render_prefetch_id === prefetched?.id ? prefetched : null);
  } finally {
    model.starting = false;
  }
}

function unitOptions(selectedSkillId = model.selectedSkillId, currentAttempt = model.state?.activeAttempt) {
  const currentIsExcluded = selectedSkillId && !model.units.some((unit) => unit.skill_id === selectedSkillId);
  const currentOption = currentIsExcluded
    ? `<option value="${h(selectedSkillId)}" selected disabled>${h(currentAttempt?.unit_name ?? selectedSkillId)}（現在の問題のみ）</option>`
    : '';
  return `${currentOption}<option value="" ${selectedSkillId ? '' : 'selected'} disabled>単元を選択</option>${model.units.map((unit) => `<option value="${h(unit.skill_id)}" ${unit.skill_id === selectedSkillId ? 'selected' : ''}>${h(unit.name)}（${Number(unit.observation_count ?? 0)}件）</option>`).join('')}`;
}

function samplingToggle(compact = false) {
  const checked = model.samplingMode === 'custom' ? 'checked' : '';
  if (compact) return `<label class="sampling-toggle compact" title="OFF: ランダム / ON: 情報の薄いeffort vectorを優先"><input type="checkbox" data-custom-sampling ${checked}><span><strong>Custom</strong></span></label>`;
  return `<label class="sampling-toggle"><input type="checkbox" data-custom-sampling ${checked}><span><strong>Custom sampling</strong><small>OFF: ランダム / ON: 情報の薄いeffort vectorを優先</small></span></label>`;
}

function bindSamplingToggle() {
  document.querySelectorAll('[data-custom-sampling]').forEach((toggle) => toggle.addEventListener('change', () => {
    model.samplingMode = toggle.checked ? 'custom' : 'random';
    localStorage.setItem('autodrill.qa.samplingMode', model.samplingMode);
    discardPrefetch();
    document.querySelectorAll('[data-custom-sampling]').forEach((other) => { other.checked = toggle.checked; });
    if (model.state?.activeAttempt && model.view === 'evaluate') void prefetchNext();
  }));
}

function renderUnitChooser() {
  removeActiveRenderOverlay();
  app.innerHTML = `<section class="attempt-shell"><div class="panel unit-chooser">
    <p class="eyebrow dark">AUTODRILL QA</p><h2>評価する単元を選択</h2><p class="muted">通常はランダム出題です。Custom samplingでは既存評価で情報が薄いeffort vector方向を優先します。</p>
    <label class="field unit-choice"><span>出題単元</span><select id="unit-choice">${unitOptions()}</select></label>
    ${samplingToggle()}
    <button class="button primary-action" id="start-unit" ${model.selectedSkillId ? '' : 'disabled'}>この単元で開始</button>
  </div></section>`;
  const select = document.querySelector('#unit-choice');
  const start = document.querySelector('#start-unit');
  select.addEventListener('change', () => { model.selectedSkillId = select.value; start.disabled = !select.value; });
  start.addEventListener('click', startAttempt);
  bindSamplingToggle();
  select.focus();
}

async function changeUnit(select) {
  const nextSkillId = select.value;
  const previousSkillId = model.selectedSkillId;
  if (!nextSkillId || nextSkillId === previousSkillId) return;
  if (model.state.activeAttempt && !confirm('現在の問題を未評価として記録し、単元を切り替えますか？')) {
    select.value = previousSkillId;
    return;
  }
  select.disabled = true;
  discardPrefetch();
  if (model.state.activeAttempt) {
    await api(`/api/attempts/${model.state.activeAttempt.id}/abandon`, {
      method: 'POST', body: JSON.stringify({ reason: 'unit_changed_before_rating', ...eventStamp() }),
    });
    model.state.activeAttempt = null;
    model.selectedRating = null;
  }
  model.selectedSkillId = nextSkillId;
  await startAttempt();
}

function ratingGrid(selected = model.selectedRating, prefix = 'rate') {
  const { ratingScale } = model.state.metadata;
  const steps = ratingScale.max - ratingScale.min + 1;
  const midpoint = ratingMidpoint();
  let cells = '';
  for (let singularity = ratingScale.max; singularity >= ratingScale.min; singularity--) {
    for (let difficulty = ratingScale.min; difficulty <= ratingScale.max; difficulty++) {
      const origin = difficulty === midpoint && singularity === midpoint;
      cells += `<div class="rating-cell ${origin ? 'origin' : ''}" data-rating-cell data-prefix="${prefix}" aria-hidden="true"></div>`;
    }
  }
  const xLabels = scaleValues().map((value) => `<span class="${value === midpoint ? 'origin-label' : ''}">${coordinateLabel(value)}</span>`).join('');
  const yLabels = [...scaleValues()].reverse().map((value) => `<span class="${value === midpoint ? 'origin-label' : ''}">${coordinateLabel(value)}</span>`).join('');
  const cursorStyle = selected
    ? `left:${selected.difficulty_position * 100}%;top:${(1 - selected.singularity_position) * 100}%` : '';
  return `<div class="rating-widget">
    <div class="axis-explainer"><span><strong>横軸：難しさ</strong>　左ほど易しい／右ほど難しい</span><span><strong>縦軸：特異性</strong>　下ほど典型的／上ほど珍しい</span></div>
    <div class="plane-layout">
      <div></div><div class="plane-endpoint top">↑ 珍しい・特異</div><div></div>
      <div class="plane-endpoint left">← 易しい</div>
      <div class="coordinate-grid" style="--rating-steps:${steps}">
        <div class="coordinate-corner">中心 0</div>
        <div class="x-ruler">${xLabels}</div>
        <div class="y-ruler">${yLabels}</div>
        <div class="rating-surface" id="rating-surface-${prefix}" data-rating-surface="${prefix}" role="application" tabindex="0" aria-label="横軸は難しさ。左が易しく右が難しい。縦軸は特異性。下が典型的で上が珍しい。ドラッグまたは矢印キーで位置を選択。">
          <div class="rating-matrix">${cells}</div>
          <span class="rating-cursor" data-rating-cursor ${selected ? '' : 'hidden'} style="${cursorStyle}" aria-hidden="true"></span>
        </div>
      </div>
      <div class="plane-endpoint right">難しい →</div>
      <div></div><div class="plane-endpoint bottom">↓ 典型的</div><div></div>
    </div>
  </div>`;
}

function renderRating(attempt, prefetched = null) {
  const usePrefetchedFrame = Boolean(prefetched?.ready && attempt.render_prefetch_id === prefetched.id);
  removeActiveRenderOverlay();
  app.innerHTML = `<section class="attempt-shell"><div class="panel rating-panel">
    <div class="unit-picker"><label for="unit-select">出題単元</label><select id="unit-select">${unitOptions(model.selectedSkillId, attempt)}</select>${samplingToggle(true)}</div>
    <div class="problem-area print-problem-area"><div class="print-render-shell" data-render-attempt="${h(attempt.id)}">
      ${usePrefetchedFrame ? '' : `<iframe class="print-render-frame" title="AutoDrill印刷レイアウトの問題と答え" src="/renderer/index.html?attempt=${encodeURIComponent(attempt.id)}"></iframe>`}
      <div class="print-render-fallback"><div class="problem-text">${h(attempt.problem_representation)}</div><div class="canonical-answer"><span>答え</span><strong>${h(attempt.canonical_answer)}</strong></div></div>
    </div>
    </div>
    <div class="rating-heading"><h2>この問題を評価</h2><p>中央を原点として、平面上の位置を選んでください。</p></div>
    ${ratingGrid()}
    <div class="selection-status" id="selection-status" aria-live="polite">${selectionStatus(model.selectedRating)}</div>
    <details class="secondary-options"><summary>任意メモ</summary><label class="field"><span>メモ</span><input id="rating-note" placeholder="計算量が多い、教科書的、境界ケース…"></label></details>
    <div class="actions confirm-rating-row"><button class="button primary-action" id="confirm-rating" ${model.selectedRating ? '' : 'disabled'}>評価を保存して次へ <span class="small">Enter</span></button></div>
  </div></section>`;
  if (usePrefetchedFrame) {
    const shell = document.querySelector('.print-render-shell');
    prefetched.frame.title = 'AutoDrill印刷レイアウトの問題と答え';
    const cleanup = showRenderOverlay(prefetched.bin, shell);
    shell.classList.add('ready');
    model.activeRenderOverlay = { bin: prefetched.bin, frame: prefetched.frame, cleanup };
    model.prefetch = null;
  }
  bindRatingSurface('rate');
  document.querySelector('#unit-select').addEventListener('change', (event) => changeUnit(event.currentTarget));
  bindSamplingToggle();
  if (model.selectedRating) model.ratingEventPromise = Promise.resolve();
  document.querySelector('#confirm-rating').addEventListener('click', confirmRating);
  document.querySelector('[data-rating-surface="rate"]')?.focus();
  void prefetchNext();
}

function bindRatingSurface(prefix, onSelect) {
  const surface = document.querySelector(`[data-rating-surface="${prefix}"]`);
  if (!surface) return;
  let dragging = false;
  let selection = prefix === 'rate' ? model.selectedRating : null;

  const applyPosition = (difficultyPosition, singularityPosition, record = false) => {
    const next = {
      difficulty: ratingFromPosition(difficultyPosition),
      singularity: ratingFromPosition(singularityPosition),
      difficulty_position: clamp01(difficultyPosition),
      singularity_position: clamp01(singularityPosition),
    };
    selection = next;
    const cursor = surface.querySelector('[data-rating-cursor]');
    cursor.hidden = false;
    cursor.style.left = `${next.difficulty_position * 100}%`;
    cursor.style.top = `${(1 - next.singularity_position) * 100}%`;
    if (prefix === 'rate') {
      model.selectedRating = next;
      document.querySelector('#selection-status').innerHTML = selectionStatus(next);
      document.querySelector('#confirm-rating').disabled = false;
      if (record) model.ratingEventPromise = recordEvent('rating_selected', next);
    } else onSelect?.(next);
  };

  const applyPointer = (event, record = false) => {
    const bounds = surface.getBoundingClientRect();
    applyPosition((event.clientX - bounds.left) / bounds.width, 1 - ((event.clientY - bounds.top) / bounds.height), record);
  };
  surface.addEventListener('pointerdown', (event) => {
    event.preventDefault();
    dragging = true;
    surface.focus();
    try { surface.setPointerCapture(event.pointerId); } catch {}
    applyPointer(event, false);
  });
  surface.addEventListener('pointermove', (event) => { if (dragging) applyPointer(event, false); });
  surface.addEventListener('pointerup', (event) => {
    if (!dragging) return;
    dragging = false;
    applyPointer(event, true);
  });
  surface.addEventListener('pointercancel', () => { dragging = false; });
  surface.addEventListener('keydown', (event) => {
    const delta = { ArrowLeft: [-1, 0], ArrowRight: [1, 0], ArrowUp: [0, 1], ArrowDown: [0, -1] }[event.key];
    if (!delta) return;
    event.preventDefault();
    const step = event.shiftKey ? 0.1 : 0.02;
    const current = selection ?? { difficulty_position: 0.5, singularity_position: 0.5 };
    applyPosition(current.difficulty_position + delta[0] * step, current.singularity_position + delta[1] * step, true);
  });
}

async function confirmRating() {
  if (!model.selectedRating || model.saving) return;
  model.saving = true;
  document.querySelector('#confirm-rating')?.setAttribute('disabled', '');
  try {
    await model.ratingEventPromise;
    const attempt = model.state.activeAttempt;
    await api(`/api/attempts/${attempt.id}/ratings`, { method: 'POST', body: JSON.stringify({
      difficulty_rating: model.selectedRating.difficulty,
      singularity_rating: model.selectedRating.singularity,
      difficulty_position: model.selectedRating.difficulty_position,
      singularity_position: model.selectedRating.singularity_position,
      rating_duration_ms: performance.now() - model.ratingStartedMonotonic,
      note: document.querySelector('#rating-note')?.value ?? '',
      ...eventStamp(),
    }) });
    const unit = model.units.find((candidate) => candidate.skill_id === attempt.source_skill_id);
    if (unit) unit.observation_count = Number(unit.observation_count ?? 0) + 1;
    model.state.activeAttempt = null;
    model.selectedRating = null;
    model.ratingEventPromise = null;
    updateChrome();
    await startAttempt();
  } finally {
    model.saving = false;
  }
}

async function renderHistory() {
  removeActiveRenderOverlay();
  app.innerHTML = `<section class="panel history-panel"><div class="panel-heading"><div><h2>評価履歴</h2><p class="muted small">回答や正誤は収集せず、問題・答え・評価・操作時刻を保存しています。</p></div><div class="actions"><a class="button secondary" href="/api/export/full">全データ JSON</a><a class="button secondary" href="/api/export/analysis.csv">分析用 CSV</a></div></div>
    <form id="history-filters" class="filters"><input name="unit" placeholder="単元"><input name="date_from" type="date"><input name="date_to" type="date"><select name="difficulty"><option value="">難しさすべて</option>${scaleValues().map((n) => `<option value="${n}">${coordinateLabel(n)}</option>`).join('')}</select><select name="singularity"><option value="">特異性すべて</option>${scaleValues().map((n) => `<option value="${n}">${coordinateLabel(n)}</option>`).join('')}</select></form>
    <div id="history-content"><p class="muted">読み込み中…</p></div><div id="history-detail"></div>
  </section>`;
  document.querySelector('#history-filters').addEventListener('change', loadHistory);
  await loadHistory();
}

async function loadHistory() {
  const form = document.querySelector('#history-filters');
  const query = new URLSearchParams([...new FormData(form)].filter(([, value]) => value));
  const history = await api(`/api/history?${query}`);
  const { summary } = history;
  document.querySelector('#history-content').innerHTML = `<div class="stats"><div class="stat"><span>完了評価</span><strong>${summary.rated_count}</strong></div><div class="stat"><span>難しさ 中央値</span><strong>${summary.median_difficulty == null ? '—' : coordinateLabel(summary.median_difficulty)}</strong></div><div class="stat"><span>特異性 中央値</span><strong>${summary.median_singularity == null ? '—' : coordinateLabel(summary.median_singularity)}</strong></div></div>
    <div class="table-wrap"><table><thead><tr><th>日時</th><th>単元 / 問題</th><th>答え</th><th>中心座標（難しさ × 特異性）</th><th>状態</th></tr></thead><tbody>${history.rows.map((row) => `<tr data-id="${h(row.id)}" data-state="${h(row.state)}"><td>${fmt(row.shown_at)}</td><td><strong>${h(row.unit_name)}</strong><br>${h(row.problem_representation)}</td><td>${h(row.canonical_answer)}</td><td><strong>${row.difficulty_rating == null ? '—' : evaluationCoordinate(row, 'difficulty')} × ${row.singularity_rating == null ? '—' : evaluationCoordinate(row, 'singularity')}</strong></td><td>${row.state === 'complete' ? '評価済み' : '未評価終了'}</td></tr>`).join('')}</tbody></table></div>`;
  document.querySelectorAll('tbody tr[data-id]').forEach((row) => row.addEventListener('click', async () => {
    const detail = await api(`/api/attempts/${row.dataset.id}`);
    document.querySelector('#history-detail').innerHTML = detailBlock(detail, true);
    bindRevision(detail);
  }));
}

function detailBlock(detail, allowRevision = false) {
  const gitState = parsedJson(detail.autodrill_git_state_json);
  return `<div class="panel history-detail"><div class="panel-heading"><h3>保存データ詳細</h3><span class="badge">attempt ${h(detail.id.slice(0, 8))}</span></div>
    <div class="answer-pair"><div class="answer-box"><strong>問題</strong>${h(detail.problem_representation)}</div><div class="answer-box"><strong>答え</strong>${h(detail.canonical_answer)}</div></div>
    <p class="observation-note">${detail.observation_mode === 'rating_only_answer_shown' ? '回答未収集（解ける前提）' : `旧方式の回答: ${h(detail.raw_user_answer ?? '—')} / ${h(detail.correctness ?? '—')}`}</p>
    <p class="muted small">生成commit ${h(detail.autodrill_git_sha)} · ${h(gitState.worktree_state ?? 'unknown')}</p>
    <h3>評価の変更履歴</h3><div class="table-wrap"><table><thead><tr><th>版</th><th>日時</th><th>難しさ</th><th>特異性</th><th>保存区分</th><th>メモ</th><th>答え表示前</th></tr></thead><tbody>${detail.evaluations.map((evaluation) => `<tr><td>${evaluation.revision_number}</td><td>${fmt(evaluation.rated_at)}</td><td>${evaluationCoordinate(evaluation, 'difficulty')}</td><td>${evaluationCoordinate(evaluation, 'singularity')}</td><td>${evaluation.difficulty_rating}, ${evaluation.singularity_rating}</td><td>${h(evaluation.note ?? '')}</td><td>${evaluation.pre_answer_reveal ? 'はい' : 'いいえ'}</td></tr>`).join('')}</tbody></table></div>
    ${allowRevision && detail.evaluations.length ? `<details><summary>評価を修正する</summary><div id="revision-grid">${ratingGrid(null, 'revise')}</div><label class="field"><span>修正理由</span><input id="revision-note" required></label><button class="button" id="save-revision" disabled>変更履歴を追加</button></details>` : ''}
    <details><summary>操作時刻と元データ</summary><div class="timeline">${detail.events.map((event) => `<div class="timeline-item"><strong>#${event.sequence_number} ${h(event.event_type)}</strong><div class="muted small">${fmt(event.occurred_at)} · client mono ${event.client_monotonic_ms?.toFixed?.(1) ?? '—'} ms</div><div class="small">${h(JSON.stringify(event.payload))}</div></div>`).join('')}</div><pre class="detail-json">${h(JSON.stringify({ selection: detail.selection, source_payload: detail.original_source_payload, item_revisions: detail.item_revisions }, null, 2))}</pre></details>
  </div>`;
}

function bindRevision(detail) {
  let selection = null;
  bindRatingSurface('revise', (next) => {
    selection = next;
    document.querySelector('#save-revision').disabled = false;
  });
  document.querySelector('#save-revision')?.addEventListener('click', async () => {
    const revised = await api(`/api/attempts/${detail.id}/ratings`, { method: 'POST', body: JSON.stringify({
      difficulty_rating: selection.difficulty,
      singularity_rating: selection.singularity,
      difficulty_position: selection.difficulty_position,
      singularity_position: selection.singularity_position,
      note: document.querySelector('#revision-note').value,
      ...eventStamp(),
    }) });
    document.querySelector('#history-detail').innerHTML = detailBlock(revised, true);
    bindRevision(revised);
    toast('以前の評価を残したまま修正しました。');
  });
}

async function recordEvent(event_type, payload = {}) {
  const attempt = model.state?.activeAttempt;
  if (!attempt) return;
  try {
    await api(`/api/attempts/${attempt.id}/events`, { method: 'POST', body: JSON.stringify({ event_type, payload, ...eventStamp() }) });
  } catch {
    // Window lifecycle events are best effort. Ratings use acknowledged writes.
  }
}

nav.addEventListener('click', async (event) => {
  const button = event.target.closest('[data-view]');
  if (!button || button.disabled) return;
  if (button.dataset.view === 'history' && model.state.activeAttempt) {
    if (!confirm('現在表示中の問題を未評価として記録し、履歴を開きますか？')) return;
    await api(`/api/attempts/${model.state.activeAttempt.id}/abandon`, {
      method: 'POST', body: JSON.stringify({ reason: 'open_history_before_rating', ...eventStamp() }),
    });
    model.state.activeAttempt = null;
    model.selectedRating = null;
    discardPrefetch();
  }
  model.view = button.dataset.view;
  if (model.view === 'evaluate' && !model.state.activeAttempt) {
    startAttempt();
  } else {
    updateChrome();
    render();
  }
});

document.addEventListener('keydown', (event) => {
  if (event.key === 'Enter' && model.selectedRating && model.state?.activeAttempt && !['INPUT', 'TEXTAREA'].includes(event.target.tagName)) {
    event.preventDefault();
    confirmRating();
  }
});
document.addEventListener('visibilitychange', () => recordEvent(document.hidden ? 'visibility_hidden' : 'visibility_visible'));
window.addEventListener('blur', () => recordEvent('window_blurred'));
window.addEventListener('focus', () => recordEvent('window_focused'));
window.addEventListener('message', (event) => {
  if (event.origin !== window.location.origin || !event.data) return;
  if (event.data.prefetchId && event.data.prefetchId === model.prefetch?.id) {
    if (event.data.type === 'qa-render-ready') {
      model.prefetch.ready = true;
      model.prefetch.frame.dataset.prefetchReady = 'true';
      model.prefetch.resolveReady(true);
    } else if (event.data.type === 'qa-render-error') discardPrefetch();
    return;
  }
  if (event.data.attemptId !== model.state?.activeAttempt?.id) return;
  const shell = document.querySelector(`[data-render-attempt="${CSS.escape(event.data.attemptId)}"]`);
  if (event.data.type === 'qa-render-ready') shell?.classList.add('ready');
  if (event.data.type === 'qa-render-error') toast(`印刷レイアウトを表示できません: ${event.data.message ?? 'unknown error'}`);
});
window.addEventListener('unhandledrejection', (event) => { toast(event.reason?.message ?? '操作に失敗しました。'); event.preventDefault(); });

refresh().then(() => {
  if (model.state.activeAttempt) recordEvent('resumed', { state: model.state.activeAttempt.state, observation_mode: model.state.activeAttempt.observation_mode });
}).catch((error) => {
  app.innerHTML = `<section class="panel warning"><h2>開始できませんでした</h2><p>${h(error.message)}</p></section>`;
});
