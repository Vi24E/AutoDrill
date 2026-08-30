const app = document.querySelector('#app');
const nav = document.querySelector('#nav');
const statusStrip = document.querySelector('#status-strip');
const toastElement = document.querySelector('#toast');

const model = {
  state: null,
  view: 'queue',
  currentDetail: null,
  selectedRating: null,
  shownMonotonic: null,
  ratingStartedMonotonic: null,
  ratingEventPromise: null,
  draftTimer: null,
  queueUnit: '',
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

async function refresh() {
  model.state = await api('/api/state');
  const active = model.state.activeAttempt;
  if (active) {
    model.view = 'attempt';
    model.shownMonotonic = performance.now() - Math.max(0, Date.now() - Date.parse(active.shown_at));
    if (active.state === 'rating') model.ratingStartedMonotonic = performance.now() - Math.max(0, Date.now() - Date.parse(active.rating_started_at));
  }
  updateChrome();
  render();
}

function updateChrome() {
  const { metadata, session, activeAttempt, databasePath } = model.state;
  statusStrip.innerHTML = `<div>schema v${metadata.qaSchemaVersion} · app ${h(metadata.appVersion)} · Git ${h(metadata.gitSha.slice(0, 9))}</div><div>${session ? `Session: ${h(session.evaluator)} / ${fmt(session.started_at)}` : 'セッション未開始'} · DB: ${h(databasePath)}</div>`;
  nav.querySelectorAll('button').forEach((button) => {
    const locked = Boolean(activeAttempt) || Boolean(model.currentDetail);
    button.disabled = locked;
    button.classList.toggle('active', button.dataset.view === model.view);
  });
}

function render() {
  if (model.currentDetail) return renderReveal(model.currentDetail);
  if (model.state.activeAttempt) return renderAttempt(model.state.activeAttempt);
  ({ queue: renderQueue, history: renderHistory, sessions: renderSessions, problems: renderProblems }[model.view] ?? renderQueue)();
}

function renderQueue() {
  const { session, items } = model.state;
  app.innerHTML = `<div class="grid">
    <section>
      ${session ? `<div class="panel"><div class="panel-heading"><div><h2>評価キュー</h2><p class="muted small">問題を選ぶか、random selectionを開始します。選択方針と候補集合は保存されます。</p></div><button class="button secondary" id="random-start" ${items.length ? '' : 'disabled'}>ランダムに開始</button></div>
        <div class="form-row"><label class="field"><span>単元filter</span><input id="queue-unit" value="${h(model.queueUnit)}" placeholder="すべての単元"></label><label class="field"><span>random seed（任意）</span><input id="random-seed" placeholder="未指定なら自動発行"></label></div>
        <div class="item-list" id="queue-items">${items.length ? items.map(itemCard).join('') : '<p class="muted">問題がまだありません。右側から登録してください。</p>'}</div></div>` : sessionStartPanel()}
    </section>
    <aside>${itemForm()}</aside>
  </div>`;
  document.querySelector('#start-session')?.addEventListener('submit', startSession);
  document.querySelector('#item-form')?.addEventListener('submit', createItem);
  document.querySelector('#queue-unit')?.addEventListener('change', async (event) => {
    model.queueUnit = event.target.value;
    const items = await api(`/api/items?unit=${encodeURIComponent(model.queueUnit)}`);
    model.state.items = items; renderQueue();
  });
  document.querySelectorAll('[data-start-item]').forEach((button) => button.addEventListener('click', () => startAttempt({ item_id: button.dataset.startItem })));
  document.querySelector('#random-start')?.addEventListener('click', () => startAttempt({ selection_policy: 'random', random_seed: document.querySelector('#random-seed').value, unit_filter: document.querySelector('#queue-unit').value }));
}

function sessionStartPanel() {
  return `<div class="panel"><h2>QAセッションを開始</h2><p class="muted">評価者、timezone、versionをsession snapshotとして保存します。</p><form id="start-session" class="stack">
    <label class="field"><span>評価者</span><input name="evaluator" value="User" required></label>
    <label class="field"><span>セッションnote（任意）</span><textarea name="note"></textarea></label>
    <button class="button" type="submit">セッション開始</button>
  </form></div>`;
}

function itemForm() {
  return `<div class="panel"><h2>問題を登録</h2><form id="item-form" class="stack">
    <label class="field"><span>source</span><select name="source"><option value="manual">manual</option><option value="autodrill">AutoDrill snapshot</option><option value="imported">imported</option><option value="other">other</option></select></label>
    <label class="field"><span>単元名</span><input name="unit_name" required maxlength="1000"></label>
    <label class="field"><span>問題</span><textarea name="problem_representation" required></textarea></label>
    <label class="field"><span>canonical answer</span><input name="canonical_answer" required></label>
    <details><summary>source metadata</summary><div class="stack">
      <label class="field"><span>source identifier</span><input name="source_identifier"></label>
      <label class="field"><span>original source payload (JSON)</span><textarea name="original_source_payload" placeholder='{"problem": {...}, "seed": "..."}'></textarea></label>
    </div></details>
    <button class="button" type="submit">登録してqueueへ追加</button>
  </form></div>`;
}

function itemCard(item) {
  return `<article class="item-card"><div><div class="actions"><span class="badge">${h(item.source)}</span><span class="badge">exposure ${item.exposure_count}</span></div><h3>${h(item.unit_name)}</h3><p>${h(item.problem_representation)}</p></div><button class="button secondary" data-start-item="${h(item.id)}">この問題を解く</button></article>`;
}

async function startSession(event) {
  event.preventDefault();
  const data = Object.fromEntries(new FormData(event.currentTarget));
  await api('/api/sessions', { method: 'POST', body: JSON.stringify({ ...data, local_timezone: Intl.DateTimeFormat().resolvedOptions().timeZone }) });
  toast('QAセッションを開始しました。'); await refresh();
}

async function createItem(event) {
  event.preventDefault();
  const form = event.currentTarget;
  const data = Object.fromEntries(new FormData(form));
  const result = await api('/api/items', { method: 'POST', body: JSON.stringify(data) });
  toast(result.duplicate_count > 1 ? '同一content hashの問題を登録しました。attemptは別々に保持されます。' : '問題を登録しました。');
  form.reset(); await refresh();
}

async function startAttempt(extra) {
  if (!model.state.session) return;
  const attempt = await api('/api/attempts', { method: 'POST', body: JSON.stringify({ session_id: model.state.session.id, selection_policy: 'manual_order', browser_version: navigator.userAgent, ...eventStamp(), ...extra }) });
  model.state.activeAttempt = attempt; model.view = 'attempt'; model.shownMonotonic = performance.now(); model.currentDetail = null;
  updateChrome(); render();
}

function renderAttempt(attempt) {
  if (attempt.state === 'rating') return renderRating(attempt);
  app.innerHTML = `<section class="attempt-shell"><div class="panel problem-card">
    <div class="actions"><span class="badge">${h(attempt.unit_name)}</span><span class="badge">exposure ${attempt.exposure_count}</span></div>
    <div class="problem-text">${h(attempt.problem_representation)}</div>
    <label class="field" style="width:100%;align-items:center"><span>あなたの回答</span><input class="answer-input" id="answer" autocomplete="off" value="${h(attempt.raw_user_answer ?? '')}" autofocus></label>
    <div id="save-indicator" class="save-indicator">回答はSQLiteへ自動保存されます</div>
    <div class="actions outcome-actions"><button class="button" id="submit-answer">回答をsubmit <span class="small">⌘/Ctrl+Enter</span></button><button class="button secondary" data-outcome="unable_to_solve">解けなかった</button><button class="button ghost" data-outcome="broken_unrateable">壊れている・評価不能</button><button class="button ghost" data-outcome="skipped">skip</button><button class="button danger" id="abandon">attemptを破棄せず中断</button></div>
  </div></section>`;
  const input = document.querySelector('#answer');
  let hadInput = Boolean(input.value);
  input.addEventListener('focus', () => recordEvent('answer_focused'));
  input.addEventListener('blur', () => { recordEvent('answer_blurred'); saveDraftNow(); });
  input.addEventListener('input', () => {
    if (!hadInput && input.value) { hadInput = true; recordEvent('first_input', { raw_user_answer: input.value }); recordEvent('answer_started'); }
    clearTimeout(model.draftTimer); model.draftTimer = setTimeout(saveDraftNow, 280);
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
  const result = await api(`/api/attempts/${attempt.id}/submit`, { method: 'POST', body: JSON.stringify({ outcome, raw_user_answer: input?.value ?? attempt.raw_user_answer ?? '', elapsed_since_shown_ms: performance.now() - model.shownMonotonic, ...eventStamp() }) });
  if (result.state === 'rating') {
    attempt.state = 'rating'; attempt.rating_started_at = result.rating_started_at; attempt.raw_user_answer = input?.value ?? '';
    model.ratingStartedMonotonic = performance.now(); model.selectedRating = null; render();
  } else { model.state.activeAttempt = null; model.currentDetail = result; updateChrome(); render(); }
}

async function abandon() {
  if (!confirm('記録は削除せず、明示的なabandoned attemptとして残します。続けますか？')) return;
  await api(`/api/attempts/${model.state.activeAttempt.id}/abandon`, { method: 'POST', body: JSON.stringify({ reason: 'explicit_user_abandon', ...eventStamp() }) });
  model.state.activeAttempt = null; toast('attemptをabandonedとして保存しました。'); await refresh();
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
  app.innerHTML = `<section class="attempt-shell"><div class="panel"><div class="panel-heading"><div><h2>問題を評価</h2><p class="muted">正答・正誤・過去分布は、評価確定まで表示しません。</p></div><span class="badge">${h(attempt.unit_name)}</span></div>
    <p class="problem-text" style="font-size:20px">${h(attempt.problem_representation)}</p>
    ${ratingGrid()}
    <div class="form-row" style="margin-top:18px"><label class="field"><span>confidence（任意・1〜5）</span><select id="confidence"><option value="">未指定</option>${[1,2,3,4,5].map((n) => `<option>${n}</option>`).join('')}</select></label><label class="field"><span>annotation（任意）</span><input id="rating-note" placeholder="計算量が多い、教科書的、境界ケース…"></label></div>
    <div class="actions" style="justify-content:center;margin-top:18px"><button class="button" id="confirm-rating" disabled>評価を確定して答えを表示</button><button class="button danger" id="abandon">attemptを破棄せず中断</button></div>
  </div></section>`;
  bindRatingCells('rate');
  document.querySelector('#confirm-rating').addEventListener('click', confirmRating);
  document.querySelector('#abandon').addEventListener('click', abandon);
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
  const detail = await api(`/api/attempts/${attempt.id}/ratings`, { method: 'POST', body: JSON.stringify({ difficulty_rating: model.selectedRating.difficulty, singularity_rating: model.selectedRating.singularity, rating_duration_ms: performance.now() - model.ratingStartedMonotonic, confidence: document.querySelector('#confidence').value || null, note: document.querySelector('#rating-note').value, ...eventStamp() }) });
  model.state.activeAttempt = null; model.currentDetail = detail; model.selectedRating = null; updateChrome(); render();
}

function renderReveal(detail) {
  const evaluation = detail.evaluations.at(-1);
  app.innerHTML = `<section class="attempt-shell"><div class="panel reveal"><div class="panel-heading"><div><h2>観測を保存しました</h2><p class="muted">raw answer、event chronology、rating revisionをSQLiteへ確定済みです。</p></div><div class="result ${detail.correctness}">${detail.correctness === 'correct' ? '正解' : detail.correctness === 'incorrect' ? '不正解' : '採点対象外'}</div></div>
    <h3>${h(detail.unit_name)}</h3><p class="problem-text" style="font-size:22px">${h(detail.problem_representation)}</p>
    <div class="answer-pair"><div class="answer-box"><strong>User answer</strong>${h(detail.raw_user_answer || '—')}</div><div class="answer-box"><strong>Canonical answer</strong>${h(detail.canonical_answer)}</div></div>
    ${evaluation ? `<p><strong>評価:</strong> 難しさ ${evaluation.difficulty_rating} / 特異性 ${evaluation.singularity_rating} · revision ${evaluation.revision_number}${evaluation.note ? ` · ${h(evaluation.note)}` : ''}</p>` : '<p class="warning">このoutcomeはratingなしで完了しました。</p>'}
    ${evaluation ? statsBlock(detail.unit_stats, evaluation) : ''}
    <div class="actions" style="margin-top:20px"><button class="button" id="next-problem">次の問題へ <span class="small">N</span></button><button class="button secondary" id="open-detail">chronologyを表示</button></div>
  </div><div id="reveal-detail"></div></section>`;
  document.querySelector('#next-problem').addEventListener('click', finishReveal);
  document.querySelector('#open-detail').addEventListener('click', () => { document.querySelector('#reveal-detail').innerHTML = detailBlock(detail); });
}

function statsBlock(stats, current) {
  const values = scaleValues();
  const cells = values.flatMap((singularity) => values.map((difficulty) => {
    const count = stats.cell_counts[`${difficulty},${singularity}`] ?? 0;
    const isCurrent = difficulty === current.difficulty_rating && singularity === current.singularity_rating;
    return `<div class="plane-cell ${isCurrent ? 'current' : ''}" title="難しさ${difficulty} / 特異性${singularity}: ${count}件">${count || ''}</div>`;
  })).join('');
  return `<div class="panel" style="box-shadow:none;margin-top:18px"><div class="panel-heading"><h3>同じ単元の観測</h3><span class="badge">N=${stats.sample_count}</span></div><div class="stats"><div class="stat"><span>難しさ median</span><strong>${stats.median_difficulty ?? '—'}</strong></div><div class="stat"><span>特異性 median</span><strong>${stats.median_singularity ?? '—'}</strong></div><div class="stat"><span>正答率</span><strong>${stats.correctness_rate == null ? '—' : `${Math.round(stats.correctness_rate * 100)}%`}</strong></div><div class="stat"><span>回答時間 median</span><strong>${ms(stats.median_response_ms)}</strong></div></div><div style="margin-top:16px"><div class="plane" style="--rating-steps:${values.length}">${cells}</div><p class="muted small">横: 難しさ / 縦: 特異性。枠が今回のpointです。小標本のNを隠しません。</p></div></div>`;
}

function finishReveal() { model.currentDetail = null; model.view = 'queue'; refresh(); }

async function renderHistory() {
  app.innerHTML = `<section class="panel"><div class="panel-heading"><div><h2>Observation history</h2><p class="muted small">filter後のlatest ratingを一覧し、detailでは全revisionとevent chronologyを表示します。</p></div><div class="actions"><a class="button secondary" href="/api/export/full">Full JSON</a><a class="button secondary" href="/api/export/analysis.csv">Analysis CSV</a></div></div>
    <form id="history-filters" class="filters"><input name="unit" placeholder="単元"><input name="date_from" type="date"><input name="date_to" type="date"><select name="correctness"><option value="">正誤すべて</option><option value="correct">correct</option><option value="incorrect">incorrect</option><option value="ungraded">ungraded</option></select><select name="difficulty"><option value="">難しさすべて</option>${scaleValues().map(n=>`<option>${n}</option>`).join('')}</select><select name="singularity"><option value="">特異性すべて</option>${scaleValues().map(n=>`<option>${n}</option>`).join('')}</select></form><div id="history-content"><p class="muted">読み込み中…</p></div><div id="history-detail"></div>
  </section>`;
  document.querySelector('#history-filters').addEventListener('change', loadHistory);
  await loadHistory();
}

async function loadHistory() {
  const form = document.querySelector('#history-filters');
  const query = new URLSearchParams([...new FormData(form)].filter(([, value]) => value));
  const history = await api(`/api/history?${query}`);
  const summary = history.summary;
  document.querySelector('#history-content').innerHTML = `<div class="stats"><div class="stat"><span>N</span><strong>${summary.sample_count}</strong></div><div class="stat"><span>rated</span><strong>${summary.rated_count}</strong></div><div class="stat"><span>correct</span><strong>${summary.correctness_rate == null ? '—' : `${Math.round(summary.correctness_rate*100)}%`}</strong></div><div class="stat"><span>response median</span><strong>${ms(summary.median_response_ms)}</strong></div></div><div class="table-wrap"><table><thead><tr><th>日時</th><th>単元 / 問題</th><th>回答</th><th>正答</th><th>結果</th><th>時間</th><th>D × S</th><th>note</th></tr></thead><tbody>${history.rows.map((row) => `<tr data-id="${h(row.id)}"><td>${fmt(row.shown_at)}</td><td><strong>${h(row.unit_name)}</strong><br>${h(row.problem_representation)}</td><td>${h(row.raw_user_answer ?? '—')}</td><td>${h(row.canonical_answer)}</td><td>${h(row.correctness ?? row.state)}</td><td>${ms(row.answer_elapsed_ms)}</td><td>${row.difficulty_rating ?? '—'} × ${row.singularity_rating ?? '—'}</td><td>${h(row.note ?? '')}</td></tr>`).join('')}</tbody></table></div>`;
  document.querySelectorAll('tbody tr[data-id]').forEach((row) => row.addEventListener('click', async () => {
    const detail = await api(`/api/attempts/${row.dataset.id}`); document.querySelector('#history-detail').innerHTML = detailBlock(detail, true); bindRevision(detail);
  }));
}

function detailBlock(detail, allowRevision = false) {
  return `<div class="panel" style="box-shadow:none"><div class="panel-heading"><h3>Raw observation detail</h3><span class="badge">attempt ${h(detail.id.slice(0,8))}</span></div>
    <div class="answer-pair"><div class="answer-box"><strong>User answer / outcome</strong>${h(detail.raw_user_answer ?? '—')} / ${h(detail.outcome)}</div><div class="answer-box"><strong>Canonical / correctness</strong>${h(detail.canonical_answer)} / ${h(detail.correctness)}</div></div>
    <h3 style="margin-top:18px">Rating revisions</h3><table><thead><tr><th>rev</th><th>rated at</th><th>D</th><th>S</th><th>confidence</th><th>note</th><th>pre-reveal</th></tr></thead><tbody>${detail.evaluations.map((evaluation) => `<tr><td>${evaluation.revision_number}</td><td>${fmt(evaluation.rated_at)}</td><td>${evaluation.difficulty_rating}</td><td>${evaluation.singularity_rating}</td><td>${evaluation.confidence ?? '—'}</td><td>${h(evaluation.note ?? '')}</td><td>${evaluation.pre_answer_reveal ? 'yes' : 'no'}</td></tr>`).join('')}</tbody></table>
    ${allowRevision && detail.evaluations.length ? `<details style="margin-top:16px"><summary>評価をrevisionとして修正</summary><div id="revision-grid">${ratingGrid(null, 'revise')}</div><label class="field"><span>修正note / 理由</span><input id="revision-note" required></label><button class="button" id="save-revision" disabled>新しいrevisionを保存</button></details>` : ''}
    <h3 style="margin-top:18px">Event chronology</h3><div class="timeline">${detail.events.map((event) => `<div class="timeline-item"><strong>#${event.sequence_number} ${h(event.event_type)}</strong><div class="muted small">server ${fmt(event.occurred_at)} · client mono ${event.client_monotonic_ms?.toFixed?.(1) ?? '—'} ms</div><div class="small">${h(JSON.stringify(event.payload))}</div></div>`).join('')}</div>
    <details style="margin-top:18px"><summary>Selection / source raw JSON</summary><pre class="detail-json">${h(JSON.stringify({ selection: detail.selection, source_payload: detail.original_source_payload, item_revisions: detail.item_revisions }, null, 2))}</pre></details>
  </div>`;
}

function bindRevision(detail) {
  let selection = null;
  bindRatingCells('revise', (next, button) => {
    selection = next; document.querySelectorAll('[data-prefix="revise"]').forEach((cell) => cell.classList.toggle('selected', cell === button)); document.querySelector('#save-revision').disabled = false;
  });
  document.querySelector('#save-revision')?.addEventListener('click', async () => {
    const revised = await api(`/api/attempts/${detail.id}/ratings`, { method: 'POST', body: JSON.stringify({ difficulty_rating: selection.difficulty, singularity_rating: selection.singularity, note: document.querySelector('#revision-note').value, ...eventStamp() }) });
    document.querySelector('#history-detail').innerHTML = detailBlock(revised, true); bindRevision(revised); toast('旧評価を残したままrevisionを追加しました。');
  });
}

async function renderSessions() {
  app.innerHTML = '<section class="panel"><div class="panel-heading"><h2>QA sessions</h2><button class="button secondary" id="end-session">current sessionを終了</button></div><div id="session-list"></div></section>';
  const sessions = await api('/api/sessions');
  document.querySelector('#session-list').innerHTML = `<table><thead><tr><th>開始</th><th>終了</th><th>評価者</th><th>timezone</th><th>attempt</th><th>Git SHA</th><th>note</th></tr></thead><tbody>${sessions.map((session) => `<tr><td>${fmt(session.started_at)}</td><td>${fmt(session.ended_at)}</td><td>${h(session.evaluator)}</td><td>${h(session.local_timezone)}</td><td>${session.completed_count}/${session.attempt_count}</td><td>${h(session.autodrill_git_sha.slice(0,9))}</td><td>${h(session.note ?? '')}</td></tr>`).join('')}</tbody></table>`;
  const end = document.querySelector('#end-session'); end.disabled = !model.state.session;
  end.addEventListener('click', async () => { await api(`/api/sessions/${model.state.session.id}/end`, { method:'POST', body:'{}' }); toast('セッションを終了しました。'); await refresh(); });
}

async function renderProblems() {
  const items = await api('/api/items');
  app.innerHTML = `<div class="grid"><section class="panel"><h2>Problem snapshots</h2><p class="muted small">同一content hashはitem候補として検出しますが、attempt observationはdeduplicateしません。</p><div class="item-list">${items.map((item) => `<article class="item-card"><div><span class="badge">${h(item.source)}</span><h3>${h(item.unit_name)}</h3><p>${h(item.problem_representation)}</p><p class="muted small">hash ${h(item.content_hash.slice(0,12))} · revision ${item.current_revision_number} · exposure ${item.exposure_count}</p></div><button class="button ghost" data-item-detail="${h(item.id)}">raw detail</button></article>`).join('')}</div><div id="problem-detail"></div></section><aside>${itemForm()}</aside></div>`;
  document.querySelector('#item-form').addEventListener('submit', createItem);
  document.querySelectorAll('[data-item-detail]').forEach((button) => button.addEventListener('click', async () => {
    const detail = await api(`/api/items/${button.dataset.itemDetail}`);
    const sourcePayload = detail.original_source_payload_json ? JSON.stringify(JSON.parse(detail.original_source_payload_json), null, 2) : '';
    document.querySelector('#problem-detail').innerHTML = `<div class="panel" style="box-shadow:none"><h3>Problem revision ${detail.current_revision_number}</h3><p class="muted small">変更はoverwriteせず、新しいitem revisionとchange auditを追加します。</p><form id="item-revision-form" class="stack">
      <label class="field"><span>単元名</span><input name="unit_name" value="${h(detail.unit_name)}" required></label>
      <label class="field"><span>問題</span><textarea name="problem_representation" required>${h(detail.problem_representation)}</textarea></label>
      <label class="field"><span>canonical answer</span><input name="canonical_answer" value="${h(detail.canonical_answer)}" required></label>
      <label class="field"><span>original source payload (JSON)</span><textarea name="original_source_payload">${h(sourcePayload)}</textarea></label>
      <label class="field"><span>変更理由</span><input name="reason" required></label>
      <button class="button" type="submit">revisionを追加</button>
    </form><details style="margin-top:16px"><summary>current raw record</summary><pre class="detail-json">${h(JSON.stringify(detail, null, 2))}</pre></details></div>`;
    document.querySelector('#item-revision-form').addEventListener('submit', async (event) => {
      event.preventDefault(); const data = Object.fromEntries(new FormData(event.currentTarget));
      data.original_source_payload = data.original_source_payload ? JSON.parse(data.original_source_payload) : null;
      await api(`/api/items/${detail.id}`, { method: 'PATCH', body: JSON.stringify(data) }); toast('旧snapshotを保持したままproblem revisionを追加しました。'); await renderProblems();
    });
  }));
}

async function recordEvent(event_type, payload = {}) {
  const attempt = model.state?.activeAttempt;
  if (!attempt) return;
  try { await api(`/api/attempts/${attempt.id}/events`, { method: 'POST', body: JSON.stringify({ event_type, payload, ...eventStamp() }) }); } catch { /* unload-adjacent telemetry is best effort; draft data uses its own acknowledged write. */ }
}

nav.addEventListener('click', (event) => {
  const button = event.target.closest('[data-view]'); if (!button || button.disabled) return;
  model.view = button.dataset.view; updateChrome(); render();
});

document.addEventListener('keydown', (event) => {
  if ((event.metaKey || event.ctrlKey) && event.key === 'Enter' && model.state?.activeAttempt?.state === 'solving') { event.preventDefault(); submit('answered'); }
  if (!event.metaKey && !event.ctrlKey && event.key.toLowerCase() === 'n' && model.currentDetail && !['INPUT','TEXTAREA'].includes(event.target.tagName)) finishReveal();
});
document.addEventListener('visibilitychange', () => recordEvent(document.hidden ? 'visibility_hidden' : 'visibility_visible'));
window.addEventListener('blur', () => recordEvent('window_blurred'));
window.addEventListener('focus', () => recordEvent('window_focused'));
window.addEventListener('unhandledrejection', (event) => { toast(event.reason?.message ?? '操作に失敗しました。'); event.preventDefault(); });

refresh().then(() => {
  if (model.state.activeAttempt) recordEvent('resumed', { state: model.state.activeAttempt.state });
}).catch((error) => { app.innerHTML = `<section class="panel warning"><h2>QA applicationを開始できません</h2><p>${h(error.message)}</p></section>`; });
