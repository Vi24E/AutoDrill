const app = document.querySelector('#app');
const nav = document.querySelector('#nav');
const statusStrip = document.querySelector('#status-strip');
const toastElement = document.querySelector('#toast');

const model = {
  state: null,
  view: 'evaluate',
  currentDetail: null,
  selectedRating: null,
  shownMonotonic: null,
  ratingStartedMonotonic: null,
  ratingEventPromise: null,
  draftTimer: null,
  starting: false,
};

const h = (value) => String(value ?? '').replace(/[&<>'"]/g, (char) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', "'": '&#39;', '"': '&quot;' })[char]);
const fmt = (value) => value ? new Intl.DateTimeFormat('ja-JP', { dateStyle: 'short', timeStyle: 'medium' }).format(new Date(value)) : '—';
const ms = (value) => Number.isFinite(value) ? `${(value / 1000).toFixed(1)}秒` : '—';
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

async function refresh({ startIfIdle = true } = {}) {
  model.state = await api('/api/state');
  const active = model.state.activeAttempt;
  if (active) {
    model.view = 'evaluate';
    model.shownMonotonic = performance.now() - Math.max(0, Date.now() - Date.parse(active.shown_at));
    if (active.state === 'rating') model.ratingStartedMonotonic = performance.now() - Math.max(0, Date.now() - Date.parse(active.rating_started_at));
  }
  updateChrome();
  if (!active && !model.currentDetail && model.view === 'evaluate' && startIfIdle) await startRandomAttempt();
  else render();
}

function updateChrome() {
  const active = model.state?.activeAttempt;
  statusStrip.innerHTML = `<div>回答・評価・操作履歴をSQLiteへ自動保存</div><div>${active ? h(active.unit_name) : '次の問題を自動生成'}</div>`;
  nav.querySelectorAll('button').forEach((button) => {
    button.disabled = Boolean(active) && button.dataset.view !== 'evaluate';
    button.classList.toggle('active', button.dataset.view === model.view);
  });
}

function render() {
  if (model.view === 'history') { renderHistory(); return; }
  if (model.currentDetail) { renderReveal(model.currentDetail); return; }
  if (model.state?.activeAttempt) { renderAttempt(model.state.activeAttempt); return; }
  renderLoading();
}

function renderLoading() {
  app.innerHTML = '<section class="attempt-shell"><div class="panel loading-card"><div class="spinner" aria-hidden="true"></div><h2>AutoDrillが問題を作っています</h2><p class="muted">単元と問題は自動でランダムに選ばれます。</p></div></section>';
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
    model.shownMonotonic = performance.now();
    updateChrome();
    render();
  } finally {
    model.starting = false;
  }
}

function renderAttempt(attempt) {
  if (attempt.state === 'rating') { renderRating(attempt); return; }
  app.innerHTML = `<section class="attempt-shell"><div class="panel problem-card">
    <span class="unit-label">${h(attempt.unit_name)}</span>
    <div class="problem-text">${h(attempt.problem_representation)}</div>
    <label class="field answer-field"><span>答え</span><input class="answer-input" id="answer" autocomplete="off" inputmode="decimal" value="${h(attempt.raw_user_answer ?? '')}" autofocus></label>
    <div id="save-indicator" class="save-indicator">入力内容は自動保存されます</div>
    <div class="actions outcome-actions"><button class="button primary-action" id="submit-answer">回答する <span class="small">⌘/Ctrl+Enter</span></button><button class="button secondary" data-outcome="unable_to_solve">解けない</button></div>
    <details class="secondary-options"><summary>この問題を評価できない場合</summary><div class="actions"><button class="button ghost" data-outcome="broken_unrateable">問題が壊れている</button><button class="button ghost" data-outcome="skipped">スキップ</button><button class="button danger" id="abandon">中断して記録を残す</button></div></details>
  </div></section>`;
  const input = document.querySelector('#answer');
  let hadInput = Boolean(input.value);
  input.addEventListener('focus', () => recordEvent('answer_focused'));
  input.addEventListener('blur', () => { recordEvent('answer_blurred'); saveDraftNow(); });
  input.addEventListener('input', () => {
    if (!hadInput && input.value) {
      hadInput = true;
      recordEvent('first_input', { raw_user_answer: input.value });
      recordEvent('answer_started');
    }
    clearTimeout(model.draftTimer);
    model.draftTimer = setTimeout(saveDraftNow, 280);
  });
  document.querySelector('#submit-answer').addEventListener('click', () => submit('answered'));
  document.querySelectorAll('[data-outcome]').forEach((button) => button.addEventListener('click', () => submit(button.dataset.outcome)));
  document.querySelector('#abandon').addEventListener('click', abandon);
  input.focus();
}

async function saveDraftNow(strict = false) {
  const attempt = model.state.activeAttempt;
  const input = document.querySelector('#answer');
  if (!attempt || attempt.state !== 'solving' || !input) return;
  clearTimeout(model.draftTimer);
  const indicator = document.querySelector('#save-indicator');
  try {
    const result = await api(`/api/attempts/${attempt.id}/draft`, { method: 'PATCH', body: JSON.stringify({ raw_user_answer: input.value, ...eventStamp() }) });
    attempt.raw_user_answer = input.value;
    if (indicator) indicator.textContent = `保存済み ${new Date(result.saved_at).toLocaleTimeString('ja-JP')}`;
  } catch (error) {
    if (indicator) indicator.textContent = `保存失敗: ${error.message}`;
    if (strict) throw error;
  }
}

async function submit(outcome) {
  const attempt = model.state.activeAttempt;
  const input = document.querySelector('#answer');
  if (!attempt) return;
  await saveDraftNow(true);
  const result = await api(`/api/attempts/${attempt.id}/submit`, { method: 'POST', body: JSON.stringify({
    outcome,
    raw_user_answer: input?.value ?? attempt.raw_user_answer ?? '',
    elapsed_since_shown_ms: performance.now() - model.shownMonotonic,
    ...eventStamp(),
  }) });
  if (result.state === 'rating') {
    attempt.state = 'rating';
    attempt.rating_started_at = result.rating_started_at;
    attempt.raw_user_answer = input?.value ?? '';
    model.ratingStartedMonotonic = performance.now();
    model.selectedRating = null;
    render();
  } else {
    model.state.activeAttempt = null;
    model.currentDetail = result;
    updateChrome();
    render();
  }
}

async function abandon() {
  if (!confirm('回答途中の記録を削除せず、中断として保存しますか？')) return;
  await api(`/api/attempts/${model.state.activeAttempt.id}/abandon`, { method: 'POST', body: JSON.stringify({ reason: 'explicit_user_abandon', ...eventStamp() }) });
  model.state.activeAttempt = null;
  toast('中断として保存しました。');
  await startRandomAttempt();
}

function ratingGrid(selected = model.selectedRating, prefix = 'rate') {
  const { ratingScale } = model.state.metadata;
  const steps = ratingScale.max - ratingScale.min + 1;
  let html = `<div class="rating-layout"><div class="rating-y-label">${h(ratingScale.axes.singularity.label)}　${ratingScale.min} ${h(ratingScale.axes.singularity.anchors[ratingScale.min])} → ${ratingScale.max} ${h(ratingScale.axes.singularity.anchors[ratingScale.max])}</div><div><div class="rating-board" style="--rating-steps:${steps}">`;
  for (let singularity = ratingScale.max; singularity >= ratingScale.min; singularity--) {
    html += `<div class="anchor">${h(ratingScale.axes.singularity.anchors[singularity] ?? '')}</div>`;
    for (let difficulty = ratingScale.min; difficulty <= ratingScale.max; difficulty++) {
      const active = selected?.difficulty === difficulty && selected?.singularity === singularity;
      html += `<button type="button" class="rating-cell ${active ? 'selected' : ''}" data-rating-cell data-prefix="${prefix}" data-d="${difficulty}" data-s="${singularity}" aria-label="難しさ${difficulty}、特異性${singularity}" aria-pressed="${active}">${difficulty},${singularity}</button>`;
    }
  }
  html += '<div></div>';
  for (let difficulty = ratingScale.min; difficulty <= ratingScale.max; difficulty++) html += `<div class="x-anchor"><strong>${difficulty}</strong><span>${h(ratingScale.axes.difficulty.anchors[difficulty] ?? '')}</span></div>`;
  return `${html}</div><div class="x-axis-title">${h(ratingScale.axes.difficulty.label)}　${ratingScale.min} ${h(ratingScale.axes.difficulty.anchors[ratingScale.min])} → ${ratingScale.max} ${h(ratingScale.axes.difficulty.anchors[ratingScale.max])}</div></div></div>`;
}

function renderRating(attempt) {
  app.innerHTML = `<section class="attempt-shell"><div class="panel"><div class="rating-intro"><span class="unit-label">${h(attempt.unit_name)}</span><h2>この問題を評価</h2><p class="muted">答えと正誤は、評価を確定するまで表示しません。</p></div>
    <p class="problem-text rating-problem">${h(attempt.problem_representation)}</p>
    ${ratingGrid()}
    <details class="secondary-options"><summary>メモを残す</summary><label class="field"><span>任意メモ</span><input id="rating-note" placeholder="計算量が多い、教科書的、境界ケース…"></label></details>
    <div class="actions confirm-rating-row"><button class="button primary-action" id="confirm-rating" disabled>評価を確定</button></div>
  </div></section>`;
  bindRatingCells('rate');
  document.querySelector('#confirm-rating').addEventListener('click', confirmRating);
}

function bindRatingCells(prefix, onSelect) {
  const cells = [...document.querySelectorAll(`[data-rating-cell][data-prefix="${prefix}"]`)];
  for (const button of cells) {
    button.addEventListener('click', async () => {
      const selection = { difficulty: Number(button.dataset.d), singularity: Number(button.dataset.s) };
      if (prefix === 'rate') {
        model.selectedRating = selection;
        document.querySelectorAll('[data-prefix="rate"]').forEach((cell) => cell.classList.toggle('selected', cell === button));
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

function renderReveal(detail) {
  const evaluation = detail.evaluations.at(-1);
  app.innerHTML = `<section class="attempt-shell"><div class="panel reveal"><div class="reveal-heading"><div><span class="unit-label">${h(detail.unit_name)}</span><h2>保存しました</h2></div><div class="result ${detail.correctness}">${detail.correctness === 'correct' ? '正解' : detail.correctness === 'incorrect' ? '不正解' : '採点対象外'}</div></div>
    <p class="problem-text rating-problem">${h(detail.problem_representation)}</p>
    <div class="answer-pair"><div class="answer-box"><strong>あなたの答え</strong>${h(detail.raw_user_answer || '—')}</div><div class="answer-box"><strong>正答</strong>${h(detail.canonical_answer)}</div></div>
    ${evaluation ? `<p class="saved-rating"><strong>難しさ ${evaluation.difficulty_rating}</strong><span>特異性 ${evaluation.singularity_rating}</span></p>` : '<p class="warning">この問題は評価なしで記録しました。</p>'}
    <div class="actions next-row"><button class="button primary-action" id="next-problem">次のランダム問題 <span class="small">N</span></button><button class="button secondary" id="show-history">履歴を見る</button></div>
  </div></section>`;
  document.querySelector('#next-problem').addEventListener('click', finishReveal);
  document.querySelector('#show-history').addEventListener('click', () => { model.currentDetail = null; model.view = 'history'; updateChrome(); render(); });
}

async function finishReveal() {
  model.currentDetail = null;
  await startRandomAttempt();
}

async function renderHistory() {
  app.innerHTML = `<section class="panel"><div class="panel-heading"><div><h2>評価履歴</h2><p class="muted small">必要なときだけ、保存した回答・評価・操作履歴を確認できます。</p></div><div class="actions"><a class="button secondary" href="/api/export/full">全データ JSON</a><a class="button secondary" href="/api/export/analysis.csv">分析用 CSV</a></div></div>
    <form id="history-filters" class="filters"><input name="unit" placeholder="単元"><input name="date_from" type="date"><input name="date_to" type="date"><select name="correctness"><option value="">正誤すべて</option><option value="correct">正解</option><option value="incorrect">不正解</option><option value="ungraded">採点対象外</option></select><select name="difficulty"><option value="">難しさすべて</option>${scaleValues().map((n) => `<option>${n}</option>`).join('')}</select><select name="singularity"><option value="">特異性すべて</option>${scaleValues().map((n) => `<option>${n}</option>`).join('')}</select></form><div id="history-content"><p class="muted">読み込み中…</p></div><div id="history-detail"></div>
  </section>`;
  document.querySelector('#history-filters').addEventListener('change', loadHistory);
  await loadHistory();
}

async function loadHistory() {
  const form = document.querySelector('#history-filters');
  const query = new URLSearchParams([...new FormData(form)].filter(([, value]) => value));
  const history = await api(`/api/history?${query}`);
  const summary = history.summary;
  document.querySelector('#history-content').innerHTML = `<div class="stats"><div class="stat"><span>評価数</span><strong>${summary.sample_count}</strong></div><div class="stat"><span>正答率</span><strong>${summary.correctness_rate == null ? '—' : `${Math.round(summary.correctness_rate * 100)}%`}</strong></div><div class="stat"><span>回答時間 中央値</span><strong>${ms(summary.median_response_ms)}</strong></div></div><div class="table-wrap"><table><thead><tr><th>日時</th><th>単元 / 問題</th><th>回答</th><th>正答</th><th>結果</th><th>時間</th><th>難しさ × 特異性</th></tr></thead><tbody>${history.rows.map((row) => `<tr data-id="${h(row.id)}"><td>${fmt(row.shown_at)}</td><td><strong>${h(row.unit_name)}</strong><br>${h(row.problem_representation)}</td><td>${h(row.raw_user_answer ?? '—')}</td><td>${h(row.canonical_answer)}</td><td>${h(row.correctness ?? row.state)}</td><td>${ms(row.answer_elapsed_ms)}</td><td>${row.difficulty_rating ?? '—'} × ${row.singularity_rating ?? '—'}</td></tr>`).join('')}</tbody></table></div>`;
  document.querySelectorAll('tbody tr[data-id]').forEach((row) => row.addEventListener('click', async () => {
    const detail = await api(`/api/attempts/${row.dataset.id}`);
    document.querySelector('#history-detail').innerHTML = detailBlock(detail, true);
    bindRevision(detail);
  }));
}

function detailBlock(detail, allowRevision = false) {
  return `<div class="panel history-detail"><div class="panel-heading"><h3>保存データ詳細</h3><span class="badge">attempt ${h(detail.id.slice(0, 8))}</span></div>
    <div class="answer-pair"><div class="answer-box"><strong>回答 / outcome</strong>${h(detail.raw_user_answer ?? '—')} / ${h(detail.outcome)}</div><div class="answer-box"><strong>正答 / correctness</strong>${h(detail.canonical_answer)} / ${h(detail.correctness)}</div></div>
    <h3>評価revision</h3><table><thead><tr><th>rev</th><th>日時</th><th>難しさ</th><th>特異性</th><th>note</th><th>答え表示前</th></tr></thead><tbody>${detail.evaluations.map((evaluation) => `<tr><td>${evaluation.revision_number}</td><td>${fmt(evaluation.rated_at)}</td><td>${evaluation.difficulty_rating}</td><td>${evaluation.singularity_rating}</td><td>${h(evaluation.note ?? '')}</td><td>${evaluation.pre_answer_reveal ? 'yes' : 'no'}</td></tr>`).join('')}</tbody></table>
    ${allowRevision && detail.evaluations.length ? `<details><summary>評価を修正する</summary><div id="revision-grid">${ratingGrid(null, 'revise')}</div><label class="field"><span>修正理由</span><input id="revision-note" required></label><button class="button" id="save-revision" disabled>revisionを追加</button></details>` : ''}
    <details><summary>操作履歴とsource metadata</summary><div class="timeline">${detail.events.map((event) => `<div class="timeline-item"><strong>#${event.sequence_number} ${h(event.event_type)}</strong><div class="muted small">${fmt(event.occurred_at)} · client mono ${event.client_monotonic_ms?.toFixed?.(1) ?? '—'} ms</div><div class="small">${h(JSON.stringify(event.payload))}</div></div>`).join('')}</div><pre class="detail-json">${h(JSON.stringify({ selection: detail.selection, source_payload: detail.original_source_payload, item_revisions: detail.item_revisions }, null, 2))}</pre></details>
  </div>`;
}

function bindRevision(detail) {
  let selection = null;
  bindRatingCells('revise', (next, button) => {
    selection = next;
    document.querySelectorAll('[data-prefix="revise"]').forEach((cell) => cell.classList.toggle('selected', cell === button));
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
    // Unload-adjacent telemetry is best effort; draft data uses its own acknowledged write.
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
  if ((event.metaKey || event.ctrlKey) && event.key === 'Enter' && model.state?.activeAttempt?.state === 'solving') {
    event.preventDefault();
    submit('answered');
  }
  if (!event.metaKey && !event.ctrlKey && event.key.toLowerCase() === 'n' && model.currentDetail && !['INPUT', 'TEXTAREA'].includes(event.target.tagName)) finishReveal();
});
document.addEventListener('visibilitychange', () => recordEvent(document.hidden ? 'visibility_hidden' : 'visibility_visible'));
window.addEventListener('blur', () => recordEvent('window_blurred'));
window.addEventListener('focus', () => recordEvent('window_focused'));
window.addEventListener('unhandledrejection', (event) => { toast(event.reason?.message ?? '操作に失敗しました。'); event.preventDefault(); });

refresh().then(() => {
  if (model.state.activeAttempt) recordEvent('resumed', { state: model.state.activeAttempt.state });
}).catch((error) => {
  app.innerHTML = `<section class="panel warning"><h2>開始できませんでした</h2><p>${h(error.message)}</p></section>`;
});
