# AutoDrill Problem QA

AutoDrillの問題品質をUser本人が継続評価するためのlocal-only application。公開用`apps/web`とは別のworkspace packageであり、GitHub Pages buildには含まれない。

## 起動

```sh
pnpm --filter @autodrill/qa dev
```

既定では `http://127.0.0.1:4179` を開く。portは`AUTODRILL_QA_PORT`、databaseは`AUTODRILL_QA_DB_PATH`で変更できる。

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
