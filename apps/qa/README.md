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
4. 「評価を保存して次へ」を押す。評価は即座に保存され、完了画面を挟まず同じ単元の次のランダム問題へ進む。

session開始、問題登録、queue操作は不要。分数を含むQA対応単元から選択でき、一桁の足し算・引き算、九九、九九型の割り算は通常の選択肢から除外する。単元は問題表示中にも変更でき、その場合は現在の問題を未評価として保存してから切り替える。保存完了は操作を妨げない短い通知だけで示す。履歴画面では過去の評価確認、rating revision、Full JSON / Analysis CSV exportができる。

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
- 通常flowは`observation_mode=rating_only_answer_shown`としてcanonical answerをrating前から表示する。User answerは収集せず、correctnessは`ungraded`、grading methodは`not_collected_assumed_solved_v1`として明示する。
- ratingの連続位置は両軸0〜1の`difficulty_position` / `singularity_position`としてlosslessに保存する。1〜7のordinal ratingは互換・集計用に連続位置から導出する。
- 旧`answer_then_rating` observationは互換性のため保持し、回答・採点履歴を破壊しない。AutoDrillの数学的gradingをTypeScriptへ複製しない。
- AutoDrill snapshotはsource payload JSONをlosslessに保存する。QAのためにproduction wireを拡張しない。
- 単元内ランダム出題はRust `drill-core`の既存WASM境界を再利用する。themeのskill identityをcanonical contractから選び、theme IDや数学ロジックをQA側へ複製しない。
- model resultはraw tableへ書き戻さず、`model_runs` / `derived_results`へversioned projectionとして保存する。

## 検証

```sh
pnpm --filter @autodrill/qa test
pnpm --filter @autodrill/qa test:browser
```
