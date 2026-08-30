# AutoDrill Problem QA

AutoDrillの問題品質をUser本人が継続評価するためのlocal-only application。公開用`apps/web`とは別のworkspace packageであり、GitHub Pages buildには含まれない。

## 起動（推奨）

macOSではrepository直下の`AutoDrill Problem QA.app`をFinderからダブルクリックする。Terminalやbrowser toolbar、URL欄を表示せず専用windowが開き、空きportは自動選択される。windowを閉じるとlocal serverも終了するため、portを管理する必要はない。app bundleにはQA runtimeが同梱されているため、Finder起動時にrepositoryがあるDesktop folderへの追加accessを要求しない。

QA sourceを変更した場合は、commit後にapp bundleを再生成する。

```sh
pnpm --filter @autodrill/qa build:macos
```

起動失敗の詳細は`~/Library/Logs/AutoDrill/problem-qa-launcher.log`へ記録される。

## 使い方

通常操作は次の4段階だけである。回答入力と採点は行わない。

1. 起動後、評価したい単元を選ぶ。
2. 選んだ単元からランダムに表示された問題と答えを見る。
3. 中央を原点とする2D平面上でcursorを自由にドラッグし、難しさと特異性を評価する。横軸が難しさ、縦軸が特異性である。
4. 「評価を保存して次へ」を押す。評価は即座に保存され、完了画面や保存通知を挟まず同じ単元の次の問題へ進む。

session開始、問題登録、queue操作は不要。現在のAutoDrill contractにある38単元のうち、User指定で除外した一桁の足し算・引き算、九九、九九型の割り算を除く34単元を選べる。分数、一次・二次・連立方程式、論理問題、4×4数独も対象である。各選択肢には、その単元で保存済みの完了評価件数を表示する。単元は問題表示中にも変更でき、その場合は現在の問題を未評価として保存してから切り替える。履歴画面では過去の評価確認、rating revision、Full JSON / Analysis CSV exportができる。

表示は`apps/web`の既存`WorksheetPrintDocument`を使い、生成済みworksheetの解答ページから対象の1問をそのまま切り抜く。問題用の別レイアウトをQA側に再実装していないため、印刷/PDF用レイアウトの崩れも評価中に発見できる。1 worksheet分をRust/WASM generatorでまとめて生成してmemoryへprefetchし、同じ単元では未表示の問題を重複なしで消費する。さらに評価入力中に次のproblemを予約し、印刷DOM・数式fontまでhidden frameで描画しておく。確定後はiframeを再読込せず描画済みframeの表示位置だけを切り替えるため、plain-text仮表示から印刷表示へ切り替わる待ち時間を通常flowへ出さない。worksheetを使い切ったときだけ次のbatchを生成する。

terminalから同じ専用windowを開く場合:

```sh
pnpm --filter @autodrill/qa open
```

## Browser tabでの開発起動

```sh
pnpm --filter @autodrill/qa dev
```

開発時だけ既定の `http://127.0.0.1:4179` をbrowserで開く。portは`AUTODRILL_QA_PORT`、databaseは`AUTODRILL_QA_DB_PATH`で変更できる。

macOSの既定database:

```text
~/Library/Application Support/AutoDrill/qa.sqlite3
```

databaseをrepository内へ置く場合も、`apps/qa/*.sqlite*`はignoreされる。datasetのbackup / transferには画面のFull JSON exportを使う。

## Data contract

- SQLiteのraw session / item revision / attempt / selection / input event / evaluation revisionがsource of truth。
- sessionと各attemptにGit HEAD SHA、およびworktreeのclean/dirty、porcelain status、status/diff fingerprintをJSON保存する。AutoDrill item snapshotにも生成時のGit stateを含め、Full JSONとAnalysis CSVへ出力する。
- 通常flowは`observation_mode=rating_only_answer_shown`としてcanonical answerをrating前から表示する。User answerは収集せず、correctnessは`ungraded`、grading methodは`not_collected_assumed_solved_v1`として明示する。
- ratingの連続位置は両軸0〜1の`difficulty_position` / `singularity_position`としてlosslessに保存する。1〜7のordinal ratingは互換・集計用に連続位置から導出する。
- 旧`answer_then_rating` observationは互換性のため保持し、回答・採点履歴を破壊しない。AutoDrillの数学的gradingをTypeScriptへ複製しない。
- AutoDrill snapshotはsource payload JSONをlosslessに保存する。QAのためにproduction wireを拡張しない。
- 単元内ランダム出題はRust `drill-core`の既存WASM境界を再利用する。1 worksheetをbatch生成し、ランダム順で重複なく消費する。themeのskill identityをcanonical contractから選び、theme IDや数学ロジックをQA側へ複製しない。
- 問題・答えの表示はproductionと同じ印刷/PDF用React rendererを別bundleとして参照する。QA bundleはproduction buildへ含まれない。
- model resultはraw tableへ書き戻さず、`model_runs` / `derived_results`へversioned projectionとして保存する。

## 検証

```sh
pnpm --filter @autodrill/qa test
pnpm --filter @autodrill/qa test:browser
```
