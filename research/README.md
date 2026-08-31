# AutoDrill research data

`research/` は、AutoDrillの教材設計を検証するための再現可能な調査toolと研究メモを置くdirectory。

第三者教材そのものをrepositoryへ保存する場所ではない。

## Chibimusu reference corpus

ちびむすドリルから作成したローカル参照コーパスの**分析上の意味・使い方・既知の限界**は次を読む。

- [`../docs/research/chibimusu-reference-corpus.md`](../docs/research/chibimusu-reference-corpus.md)

ローカルデータ:

- `research/corpora/chibimusu/corpus.sqlite3`
- `research/corpora/chibimusu/source_pages.jsonl`
- `research/corpora/chibimusu/pdf_links.discovered.jsonl`

`research/corpora/` は `.gitignore` 対象。第三者著作物・crawl snapshot・SQLite DBをcommitしない。

Git管理する再現tool:

- `tools/chibimusu_corpus.py` — 公開catalogue discovery
- `tools/chibimusu_ingest.py` — PDFのbounded download/extraction/SQLite ingest
- `tools/chibimusu_schema.sql` — normalized schema

## Rebuild / refresh

ネットワークを使う調査なので、CIではなくローカルで明示的に実行する。

PyMuPDFが必要。

```bash
python3 -m venv .venv
.venv/bin/python -m pip install pymupdf

.venv/bin/python research/tools/chibimusu_corpus.py discover \
  --output research/corpora/chibimusu

.venv/bin/python research/tools/chibimusu_ingest.py \
  --output research/corpora/chibimusu
```

`chibimusu_ingest.py` はresume/idempotent前提。

- downloader → bounded queue → extractor → single SQLite writer
- PDFは一時staging artifact
- SQLite commit成功後にだけPDFを削除
- extraction / DB error時は失敗状態を残して再試行可能
- textless pageだけgrayscale PNG fallbackをSQLiteへ保持

既存corpusをrefreshするときも、PDFを恒久保存する方式へ変えない。

## Quick integrity check

```bash
python3 - <<'PY'
import sqlite3

path = 'research/corpora/chibimusu/corpus.sqlite3'
con = sqlite3.connect(path)
print('integrity:', con.execute('PRAGMA integrity_check').fetchone()[0])
print('foreign-key violations:', len(con.execute('PRAGMA foreign_key_check').fetchall()))
print('worksheets:', con.execute('SELECT COUNT(*) FROM worksheets').fetchone()[0])
print('pages:', con.execute('SELECT COUNT(*) FROM pages').fetchone()[0])
PY
```

## Research-data ownership

次を区別する。

### Canonical local evidence

- Chibimusu SQLite corpus
- AutoDrill local QA SQLite
- production generator code / generated problem DTO

### Derived / reproducible evidence

- feature counts
- worksheet composition metrics
- problem-unit extraction
- statistical models
- charts / reports

Derived resultはraw sourceへ上書きしない。
長期的に使う結果は`.tmp/`だけに置かず、再現toolまたは`docs/research/`の研究記録へ昇格させる。

### Temporary artifacts

- 一時PDF
- page screenshot
- ad-hoc parser output
- debug JSON

調査が終わったら削除する。重要な知見は削除前にtracked document / reproducible scriptへ移す。

## Copyright / redistribution boundary

このresearch corpusはAutoDrill内部の比較研究用。

- 第三者PDFをcommitしない
- raster/image snapshotをcommitしない
- corpus DBを配布しない
- 問題文の大量転載を研究成果として残さない
- repositoryへ残すのはschema、取得tool、aggregate metrics、archetype taxonomy、分析方法

## Tracked aggregate snapshots

2026-08-31時点の主要なaggregate resultは `research/results/2026-08-31/` に保存している。第三者の問題本文ではなく、AutoDrill sample / Chibimusu aggregate metric / worksheet-composition statisticsだけを残している。

これらはhistorical derived snapshotであり、generator revisionやcorpus extractionを変更した後の現行値ではない。raw SQLite / current generatorから再計算できる場合は再計算を優先する。

## Existing research notes

`research/notes/` はカリキュラム調査のdraft/history。
現行product仕様のsource of truthは `docs/principles.md`、`curriculum.md`、`docs/architecture/`、GitHub Issues。
