# AutoDrill Problem QA

AutoDrillの問題品質をUser本人が継続評価するためのlocal-only application。公開用`apps/web`とは別のworkspace packageであり、GitHub Pages buildには含まれない。

## 起動（推奨）

macOSではrepository直下の`AutoDrill Problem QA.app`をFinderからダブルクリックする。Terminalやbrowser toolbar、URL欄を表示せず専用windowが開き、空きportは自動選択される。windowを閉じるとlocal serverも終了するため、portを管理する必要はない。

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
- rating前のAPI responseにはcanonical answer、correctness、過去分布を含めない。
- manual problemのgradingはUnicode NFKC + whitespace正規化後のexact text equalityとして`grading_method`に記録する。AutoDrillの数学的gradingをTypeScriptへ複製しない。
- AutoDrill snapshotはsource payload JSONをlosslessに保存する。QAのためにproduction wireを拡張しない。
- model resultはraw tableへ書き戻さず、`model_runs` / `derived_results`へversioned projectionとして保存する。

## 検証

```sh
pnpm --filter @autodrill/qa test
pnpm --filter @autodrill/qa test:browser
```
