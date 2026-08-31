# 2026-08-31 curriculum-composition analysis snapshot

このdirectoryは、Chibimusu reference corpusとAutoDrill generatorを比較した2026-08-31時点の**aggregate / derived result snapshot**を保存する。

第三者教材の問題文・PDF・画像は含めない。raw evidenceのsource of truthではなく、当時の分析値を失わず追跡するためのhistorical research artifact。

解釈方法と最新の利用ルールは [`../../../docs/research/chibimusu-reference-corpus.md`](../../../docs/research/chibimusu-reference-corpus.md) を参照する。

## Files

### `autodrill-multidomain-trends.json`

AutoDrillの26 theme × difficulty 1..4について、generator sampleから集計した構造feature trend。

主な用途:

- d4=`random`のbase support確認
- difficulty別にarchetype / novice-friction featureがどう動くかの探索
- external corpus比較前のAutoDrill側baseline

注意: difficultyは順序尺度として解釈しない。特にd4は「最難」ではなくrandom。

### `chibimusu-comparison-metrics.json`

Chibimusuの専用教材から復元した筆算・分数等のaggregate metric。

主な用途:

- carry / borrow / zero digitなどのproblem-level比較
- proper/improper fraction、cross-cancel、reduction等の比較
- AutoDrillとの差分を定量化するためのhistorical snapshot

一部は対象専用教材だけから復元したsampleであり、全コーパス0 occurrenceを意味しない。

### `worksheet-composition.json`

AutoDrillのselected themeについて、500 worksheetずつ生成し、1枚の中に特定featureが占める割合を集計したもの。

対象:

- theme 25: 2桁足し算 / multi-carry
- theme 30: 2桁乗算 / zero digit
- theme 31: 1桁除算 / quotient digit zero
- theme 9: 分数加算 / same denominator

このsnapshotは、「pooled frequencyが近くてもworksheet compositionが違う」ことを検証するために重要。

## Canonicality

優先順位:

1. raw local Chibimusu SQLite / AutoDrill QA SQLite / production generator
2. 再実行可能な分析
3. このhistorical derived snapshot

generator revisionやcorpus extractionを更新したら、このsnapshotを現行値とみなさず再計算する。

## Why tracked

元の分析は`.tmp/`で行われたが、次の理由でaggregate JSONだけを正式な研究資産へ昇格した。

- 教材archetype設計の根拠として今後も参照価値がある
- 第三者の問題本文を含まない
- 数十KB程度で小さい
- issue / future analysisの比較基準を失わずに済む

ad-hoc parserや一時PDF / screenshotはこのdirectoryへ保存しない。
