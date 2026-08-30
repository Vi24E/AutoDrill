const app = document.querySelector('#app');
const nav = document.querySelector('#nav');
const statusStrip = document.querySelector('#status-strip');
const toastElement = document.querySelector('#toast');

const model = {
  state: null, view: 'evaluate', currentDetail: null, selectedRating: null,
  ratingStartedMonotonic: null, ratingEventPromise: null, starting: false,
};

const h = (value) => String(value ?? '').replace(/[&<>'"]/g, (char) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', "'": '&#39;', '"': '&quot;' })[char]);
const fmt = (value) => value ? new Intl.DateTimeFormat('ja-JP', { dateStyle: 'short', timeStyle: 'medium' }).format(new Date(value)) : '—';
const eventStamp = () => ({ client_wall_at: new Date().toISOString(), client_monotonic_ms: performance.now() });
const scaleValues = () => {
  const { min, max } = model.state.metadata.ratingScale;
  return Array.from({ length: max - min + 1 }, (_, index) => min + index);
};

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
  model.state = await api('/api/state');
  const active = model.state.activeAttempt;
  if (!active || active.observation_mode !== 'rating_only_answer_shown') {
    await startRandomAttempt();
    return;
  }
  model.view = 'evaluate';
  model.ratingStartedMonotonic = performance.now() - Math.max(0, Date.now() - Date.parse(active.rating_started_at));
  updateChrome();
  render();
}

function updateChrome() {
  const active = model.state?.activeAttempt;
  statusStrip.innerHTML = `<div>答えを見て、グリッドで評価するだけ</div><div>${active ? h(active.unit_name) : '評価はSQLiteへ保存済み'}</div>`;
  nav.querySelectorAll('button').forEach((button) => {
    button.disabled = Boolean(active) && button.dataset.view !== 'evaluate';
    button.classList.toggle('active', button.dataset.view === model.view);
  });
}

function render() {
  if (model.view === 'history') { renderHistory(); return; }
  if (model.currentDetail) { renderSaved(model.currentDetail); return; }
  if (model.state?.activeAttempt) { renderRating(model.state.activeAttempt); return; }
  renderLoading();
}

function renderLoading() {
  app.innerHTML = '<section class="attempt-shell"><div class="panel loading-card"><div class="spinner" aria-hidden="true"></div><h2>次の問題を作っています</h2></div></section>';
}

async function startRandomAttempt() {
  if (model.starting) return;
  model.starting = true;
  model.view = 'evaluate';
  model.currentDetail = null;
  renderLoading();
  try {
    const attempt = await api('/api/quick/next', { method: 'POST', body: JSON.stringify({
      local_timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
      browser_version: navigator.userAgent,
      ...eventStamp(),
    }) });
    model.state.activeAttempt = attempt;
    model.ratingStartedMonotonic = performance.now() - Math.max(0, Date.now() - Date.parse(attempt.rating_started_at));
    model.selectedRating = null;
    updateChrome();
    render();
  } finally {
    model.starting = false;
  }
}

function ratingGrid(selected = model.selectedRating, prefix = 'rate') {
  const { ratingScale } = model.state.metadata;
  const steps = ratingScale.max - ratingScale.min + 1;
  let cells = '<div class="matrix-corner" aria-hidden="true">特異性<br>↓</div>';
  for (let difficulty = ratingScale.min; difficulty <= ratingScale.max; difficulty++) {
    cells += `<div class="matrix-column"><strong>${difficulty}</strong><span>${h(ratingScale.axes.difficulty.anchors[difficulty] ?? '')}</span></div>`;
  }
  for (let singularity = ratingScale.max; singularity >= ratingScale.min; singularity--) {
    cells += `<div class="matrix-row"><strong>${singularity}</strong><span>${h(ratingScale.axes.singularity.anchors[singularity] ?? '')}</span></div>`;
    for (let difficulty = ratingScale.min; difficulty <= ratingScale.max; difficulty++) {
      const active = selected?.difficulty === difficulty && selected?.singularity === singularity;
      cells += `<button type="button" class="rating-cell ${active ? 'selected' : ''}" data-rating-cell data-prefix="${prefix}" data-d="${difficulty}" data-s="${singularity}" aria-label="難しさ${difficulty}、特異性${singularity}" aria-pressed="${active}"><span>${active ? '✓' : ''}</span></button>`;
    }
  }
  return `<div class="rating-widget">
    <div class="axis-guide"><span><strong>縦：特異性</strong> 1 典型的 → 7 珍しい</span><span><strong>横：難しさ</strong> 1 易しい → 7 難しい</span></div>
    <div class="rating-matrix" style="--rating-steps:${steps}">${cells}</div>
    <div class="matrix-x-title">難しさ →</div>
  </div>`;
}

function renderRating(attempt) {
  app.innerHTML = `<section class="attempt-shell"><div class="panel rating-panel">
    <div class="problem-area"><span class="unit-label">${h(attempt.unit_name)}</span><div class="problem-text">${h(attempt.problem_representation)}</div>
      <div class="canonical-answer"><span>答え</span><strong>${h(attempt.canonical_answer)}</strong></div>
    </div>
    <div class="rating-heading"><h2>この問題を評価</h2><p>縦と横が交わるマスを選んでください。</p></div>
    ${ratingGrid()}
    <div class="selection-status" id="selection-status" aria-live="polite">まだ選択されていません</div>
    <details class="secondary-options"><summary>任意メモ</summary><label class="field"><span>メモ</span><input id="rating-note" placeholder="計算量が多い、教科書的、境界ケース…"></label></details>
    <div class="actions confirm-rating-row"><button class="button primary-action" id="confirm-rating" disabled>評価を保存して次へ <span class="small">Enter</span></button></div>
  </div></section>`;
  bindRatingCells('rate');
  document.querySelector('#confirm-rating').addEventListener('click', confirmRating);
  document.querySelector('[data-prefix="rate"][data-d="4"][data-s="4"]')?.focus();
}

function bindRatingCells(prefix, onSelect) {
  const cells = [...document.querySelectorAll(`[data-rating-cell][data-prefix="${prefix}"]`)];
  for (const button of cells) {
    button.addEventListener('click', async () => {
      const selection = { difficulty: Number(button.dataset.d), singularity: Number(button.dataset.s) };
      if (prefix === 'rate') {
        model.selectedRating = selection;
        document.querySelectorAll('[data-prefix="rate"]').forEach((cell) => {
          const active = cell === button;
          cell.classList.toggle('selected', active);
          cell.setAttribute('aria-pressed', String(active));
          cell.querySelector('span').textContent = active ? '✓' : '';
        });
        document.querySelector('#selection-status').innerHTML = `<strong>難しさ ${selection.difficulty}</strong><span>特異性 ${selection.singularity}</span>`;
        document.querySelector('#confirm-rating').disabled = false;
        model.ratingEventPromise = recordEvent('rating_selected', selection);
        await model.ratingEventPromise;
      } else onSelect?.(selection, button);
    });
    button.addEventListener('keydown', (event) => {
      const delta = { ArrowLeft: [-1, 0], ArrowRight: [1, 0], ArrowUp: [0, 1], ArrowDown: [0, -1] }[event.key];
      if (!delta) return;
      event.preventDefault();
      const { min, max } = model.state.metadata.ratingScale;
      const difficulty = Math.min(max, Math.max(min, Number(button.dataset.d) + delta[0]));
      const singularity = Math.min(max, Math.max(min, Number(button.dataset.s) + delta[1]));
      document.querySelector(`[data-prefix="${prefix}"][data-d="${difficulty}"][data-s="${singularity}"]`)?.focus();
    });
  }
}

async function confirmRating() {
  if (!model.selectedRating) return;
  await model.ratingEventPromise;
  const attempt = model.state.activeAttempt;
  const detail = await api(`/api/attempts/${attempt.id}/ratings`, { method: 'POST', body: JSON.stringify({
    difficulty_rating: model.selectedRating.difficulty,
    singularity_rating: model.selectedRating.singularity,
    rating_duration_ms: performance.now() - model.ratingStartedMonotonic,
    note: document.querySelector('#rating-note')?.value ?? '',
    ...eventStamp(),
  }) });
  model.state.activeAttempt = null;
  model.currentDetail = detail;
  model.selectedRating = null;
  updateChrome();
  render();
}

function renderSaved(detail) {
  const evaluation = detail.evaluations.at(-1);
  app.innerHTML = `<section class="attempt-shell"><div class="panel saved-panel">
    <div class="saved-mark" aria-hidden="true">✓</div><h2>保存しました</h2>
    <span class="unit-label">${h(detail.unit_name)}</span><div class="saved-problem">${h(detail.problem_representation)} <strong>${h(detail.canonical_answer)}</strong></div>
    <div class="saved-rating"><strong>難しさ ${evaluation.difficulty_rating}</strong><span>特異性 ${evaluation.singularity_rating}</span></div>
    <div class="actions next-row"><button class="button primary-action" id="next-problem">次の問題 <span class="small">N</span></button><button class="button secondary" id="show-history">履歴を見る</button></div>
  </div></section>`;
  document.querySelector('#next-problem').addEventListener('click', startRandomAttempt);
  document.querySelector('#show-history').addEventListener('click', () => { model.currentDetail = null; model.view = 'history'; updateChrome(); render(); });
}

async function renderHistory() {
  app.innerHTML = `<section class="panel history-panel"><div class="panel-heading"><div><h2>評価履歴</h2><p class="muted small">回答や正誤は収集せず、問題・答え・評価・操作時刻を保存しています。</p></div><div class="actions"><a class="button secondary" href="/api/export/full">全データ JSON</a><a class="button secondary" href="/api/export/analysis.csv">分析用 CSV</a></div></div>
    <form id="history-filters" class="filters"><input name="unit" placeholder="単元"><input name="date_from" type="date"><input name="date_to" type="date"><select name="difficulty"><option value="">難しさすべて</option>${scaleValues().map((n) => `<option>${n}</option>`).join('')}</select><select name="singularity"><option value="">特異性すべて</option>${scaleValues().map((n) => `<option>${n}</option>`).join('')}</select></form>
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
  document.querySelector('#history-content').innerHTML = `<div class="stats"><div class="stat"><span>評価数</span><strong>${summary.sample_count}</strong></div><div class="stat"><span>難しさ 中央値</span><strong>${summary.median_difficulty ?? '—'}</strong></div><div class="stat"><span>特異性 中央値</span><strong>${summary.median_singularity ?? '—'}</strong></div></div>
    <div class="table-wrap"><table><thead><tr><th>日時</th><th>単元 / 問題</th><th>答え</th><th>難しさ × 特異性</th><th>方式</th></tr></thead><tbody>${history.rows.map((row) => `<tr data-id="${h(row.id)}"><td>${fmt(row.shown_at)}</td><td><strong>${h(row.unit_name)}</strong><br>${h(row.problem_representation)}</td><td>${h(row.canonical_answer)}</td><td><strong>${row.difficulty_rating ?? '—'} × ${row.singularity_rating ?? '—'}</strong></td><td>${row.observation_mode === 'rating_only_answer_shown' ? '評価のみ' : '旧方式'}</td></tr>`).join('')}</tbody></table></div>`;
  document.querySelectorAll('tbody tr[data-id]').forEach((row) => row.addEventListener('click', async () => {
    const detail = await api(`/api/attempts/${row.dataset.id}`);
    document.querySelector('#history-detail').innerHTML = detailBlock(detail, true);
    bindRevision(detail);
  }));
}

function detailBlock(detail, allowRevision = false) {
  return `<div class="panel history-detail"><div class="panel-heading"><h3>保存データ詳細</h3><span class="badge">attempt ${h(detail.id.slice(0, 8))}</span></div>
    <div class="answer-pair"><div class="answer-box"><strong>問題</strong>${h(detail.problem_representation)}</div><div class="answer-box"><strong>答え</strong>${h(detail.canonical_answer)}</div></div>
    <p class="observation-note">${detail.observation_mode === 'rating_only_answer_shown' ? '回答未収集（解ける前提）' : `旧方式の回答: ${h(detail.raw_user_answer ?? '—')} / ${h(detail.correctness ?? '—')}`}</p>
    <h3>評価の変更履歴</h3><div class="table-wrap"><table><thead><tr><th>版</th><th>日時</th><th>難しさ</th><th>特異性</th><th>メモ</th><th>答え表示前</th></tr></thead><tbody>${detail.evaluations.map((evaluation) => `<tr><td>${evaluation.revision_number}</td><td>${fmt(evaluation.rated_at)}</td><td>${evaluation.difficulty_rating}</td><td>${evaluation.singularity_rating}</td><td>${h(evaluation.note ?? '')}</td><td>${evaluation.pre_answer_reveal ? 'はい' : 'いいえ'}</td></tr>`).join('')}</tbody></table></div>
    ${allowRevision && detail.evaluations.length ? `<details><summary>評価を修正する</summary><div id="revision-grid">${ratingGrid(null, 'revise')}</div><label class="field"><span>修正理由</span><input id="revision-note" required></label><button class="button" id="save-revision" disabled>変更履歴を追加</button></details>` : ''}
    <details><summary>操作時刻と元データ</summary><div class="timeline">${detail.events.map((event) => `<div class="timeline-item"><strong>#${event.sequence_number} ${h(event.event_type)}</strong><div class="muted small">${fmt(event.occurred_at)} · client mono ${event.client_monotonic_ms?.toFixed?.(1) ?? '—'} ms</div><div class="small">${h(JSON.stringify(event.payload))}</div></div>`).join('')}</div><pre class="detail-json">${h(JSON.stringify({ selection: detail.selection, source_payload: detail.original_source_payload, item_revisions: detail.item_revisions }, null, 2))}</pre></details>
  </div>`;
}

function bindRevision(detail) {
  let selection = null;
  bindRatingCells('revise', (next, button) => {
    selection = next;
    document.querySelectorAll('[data-prefix="revise"]').forEach((cell) => {
      const active = cell === button;
      cell.classList.toggle('selected', active);
      cell.querySelector('span').textContent = active ? '✓' : '';
    });
    document.querySelector('#save-revision').disabled = false;
  });
  document.querySelector('#save-revision')?.addEventListener('click', async () => {
    const revised = await api(`/api/attempts/${detail.id}/ratings`, { method: 'POST', body: JSON.stringify({
      difficulty_rating: selection.difficulty,
      singularity_rating: selection.singularity,
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

nav.addEventListener('click', (event) => {
  const button = event.target.closest('[data-view]');
  if (!button || button.disabled) return;
  model.view = button.dataset.view;
  if (model.view === 'evaluate' && !model.state.activeAttempt) {
    model.currentDetail = null;
    startRandomAttempt();
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
  if (!event.metaKey && !event.ctrlKey && event.key.toLowerCase() === 'n' && model.currentDetail && !['INPUT', 'TEXTAREA'].includes(event.target.tagName)) startRandomAttempt();
});
document.addEventListener('visibilitychange', () => recordEvent(document.hidden ? 'visibility_hidden' : 'visibility_visible'));
window.addEventListener('blur', () => recordEvent('window_blurred'));
window.addEventListener('focus', () => recordEvent('window_focused'));
window.addEventListener('unhandledrejection', (event) => { toast(event.reason?.message ?? '操作に失敗しました。'); event.preventDefault(); });

refresh().then(() => {
  if (model.state.activeAttempt) recordEvent('resumed', { state: model.state.activeAttempt.state, observation_mode: model.state.activeAttempt.observation_mode });
}).catch((error) => {
  app.innerHTML = `<section class="panel warning"><h2>開始できませんでした</h2><p>${h(error.message)}</p></section>`;
});
