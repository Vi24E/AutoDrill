# Chibimusu reference corpus — 使い方と解釈規則

この文書は、AutoDrill の教材品質を既存ドリルと比較するために作成したローカル研究コーパスの**使い方・解釈規則・既知の限界**を残すための記録である。

対象は、ちびむすドリル（happylilac.net）から公開されている算数・数学教材のうち、問題PDFとして発見できた教材をローカル研究用に正規化したコーパス。

第三者著作物を含むため、コーパス本体はGit管理しない。AutoDrill repositoryには、再現可能な取得・抽出toolと、この文書のような**分析方法・集計結果・設計上の知見だけ**を残す。

## 1. 何のために使うか

このコーパスは、ちびむすドリルをground truthとして模倣するためのものではない。

AutoDrillが「既存ドリル程度の教材品質」を満たしているかを見るための empirical baseline として使う。

主な用途は次の3つ。

1. **curriculum / pedagogical archetype の存在確認**
   - 既存教材では何を独立した練習単元として切り出しているか。
   - AutoDrillの1 themeに、既存教材では別単元のものが混ざっていないか。
2. **AutoDrill-only archetype の検出**
   - AutoDrillでは生成するが、同等のreference教材では一度も確認できない構造がないか。
   - これは即「数学的に誤り」とはしないが、出題ミス・scope mismatch・過剰生成候補としてUser判断へ回す。
3. **worksheet composition の比較**
   - 1問単位の平均値だけでなく、1枚のworksheetがどの技能へ集中しているかを見る。

特に重要なのは、**pooled marginal frequencyだけを一致させない**こと。

たとえばAutoDrillの2桁足し算全体で繰り上がり率がreferenceと似ていても、reference側が「繰り上がりなし」「十の位で繰り上がり」「一・十の位で繰り上がり」の専用worksheetへ分離しているなら、AutoDrillが1枚に全部を中途半端に混ぜることは教材構成として別物である。

## 2. Canonical data と repository boundary

ローカルのcanonical corpus:

`research/corpora/chibimusu/corpus.sqlite3`

このdirectory全体は `.gitignore` の `research/corpora/` で除外する。

Git管理するもの:

- `research/tools/chibimusu_corpus.py`
  - 公開catalogueから数学教材ページと問題PDF URLを発見する。
- `research/tools/chibimusu_ingest.py`
  - 問題PDFをbounded pipelineで一時取得し、text / layout / raster fallbackをSQLiteへ保存する。
- `research/tools/chibimusu_schema.sql`
  - normalized SQLite schema。
- `research/README.md`
  - 再取得・運用手順。
- この文書
  - corpusの意味、分析ルール、既知の結果。

Git管理しないもの:

- PDF本体
- HTML snapshot
- SQLite corpus
- raster fallback画像
- crawl結果JSONL
- 第三者教材そのものの再配布物

PDFは抽出成功・DB commit成功後に削除する設計であり、canonical dataではない。

## 3. 現在のコーパス規模

2026-08-31時点でローカルDBを再集計した値:

| 指標 | 件数 |
|---|---:|
| worksheets | 3,181 |
| non-summary worksheets | 2,835 |
| summary worksheets | 346 |
| documents | 3,181 |
| pages | 5,483 |
| text blocks | 155,990 |
| textless pages | 93 |
| raster fallback pages | 93 |
| source pages | 421 |
| non-summary worksheetを持つsource pages | 370 |
| word-problem-source worksheets | 257 |

DB integrity:

- `PRAGMA integrity_check` = `ok`
- foreign-key violations = 0

## 4. Schemaの意味

主なtable:

- `source_pages`
  - 教材catalogue上の1ページ。
  - 現時点では**pedagogical archetypeの第一近似**として最も重要。
- `categories`
  - 算数・数学上の大分類。
- `source_page_categories`
  - source pageとcategoryの多対多関係。
- `worksheets`
  - 問題PDF URL単位のlogical worksheet。
  - `summary=1` はまとめPDF等。
- `worksheet_sources`
  - worksheetがどのsource page / category / anchorから発見されたかというprovenance。
- `documents`
  - PDF内容sha256で一意化した抽出document。
- `pages`
  - ページ本文textとgeometry。
- `text_blocks`
  - 座標付きtext block。
- `page_rasters`
  - textが完全に存在しないページだけのgrayscale PNG fallback。
- `pages_fts`
  - page text検索用FTS5 index。derived indexでありsource of truthではない。

## 5. 分析単位の優先順位

### 5.1 source_pageをarchetypeの第一近似にする

ちびむすでは、同じ数学領域でも教材ページをかなり細かく分けている。

例:

- 分数の同分母加算 / 異分母加算
- 3桁筆算 / 4桁筆算
- 2桁×2桁 / 2桁×何十
- 除法の余りあり / 余りなし / 商の特定桁が0
- 一次方程式の通常形 / 括弧 / 小数係数 / 分数係数

そのため、全PDFを一つの大きな母集団として混ぜるより、まず `source_page_id` ごとに何を練習する教材なのかを見る。

### 5.2 summary worksheetを独立サンプルとして数えない

まとめPDFは個別worksheetを再収録している場合が多い。

既存分析ではsummary text pageの約82%がnon-summary側と同じnormalized text hashを持っていた。

したがって分布比較の基本は:

- `worksheets.summary = 0`
- individual worksheet
- source_pageごとのworksheet composition

とする。

summaryを含めて単純countすると教材頻度を二重計上しやすい。

### 5.3 problem-unitへ分解するときはdomain別parserを使う

全領域へ1個の万能regex parserを作らない。

- 横式 arithmetic
- fraction
- equation
- column arithmetic
- word problem
- geometry / graph

ではPDF内の文法が違う。

問題単位のexact metricが必要なときは、対象domain専用の抽出規則を作り、少量の目視verificationを行う。

## 6. 推奨比較workflow

### Step 1: 比較するsemantic featureを先に定義する

AutoDrill側のgenerator/problem DTOから、教材上意味のあるfeatureを定義する。

例:

- fraction addition: same denominator / unlike denominator
- multiplication: multiplierが10の倍数か
- long division: dividend digits / divisor digits / remainder / quotientの途中桁0
- linear equation: `ax+b=cx+d` / `ax+b=c` / `ax+b=0`、整理後 `|A|=1`
- quadratic formula: discriminantが平方数か、表示係数が整数か分数か

表示文字列やnumeric theme IDから意味を逆算しない。

### Step 2: reference側のdedicated source pageを特定する

タイトル・description・category・FTS textから該当source pageを探す。

まず「このarchetypeの専用教材が存在するか」を確認する。

### Step 3: non-summary individual worksheetsだけで構成を見る

同じsource pageに紐づく `summary=0` worksheetを列挙し、1枚ごとの問題構成を見る。

ここで重要なのは、単に全問題をpoolして比率を出すだけでなく、各worksheetのfeature shareを見ること。

例:

- reference: carry 2回のsheetが 0% / 0% / 100% のように技能別に分かれている
- AutoDrill: 各sheetに20〜40%程度ずつ混在する

なら、pooled平均が同じでも教材構成は一致していない。

### Step 4: support差を先に見る

次の順で判断する。

1. reference dedicated教材で明示的に存在するか
2. referenceの別学年・別まとめ教材には存在するか
3. 全problem-unitを調べても0件か

0件判定は強い主張なので、dedicated subsetだけ見て「存在しない」と言わない。

### Step 5: 必要ならfrequency / compositionを測る

supportが同じ場合に、初めて頻度差を見る。

優先する比較:

- feature occurrence rate
- within-sheet feature share distribution
- archetype coverage
- dedicated vs summary composition

必要ならconfidence intervalやdistribution distanceを使う。

### Step 6: human QAがある領域では交差確認する

一次方程式(1)は現時点で特別に重要。

次の3種類が揃っている。

- AutoDrillの大量生成sample
- ちびむすreference
- Userによる100件human singularity QA

この3者が揃うと、単なる「referenceに少ない」より強い判断ができる。

例:

- 整理後 `|A|=1`
  - AutoDrill d4=random: 約40.3%
  - reference direct-integer sample: 約16.1%
  - human QAでもsingularityを大きく押し上げる
  - → generic supportの過剰生成候補
- 左右に同じ非zero定数
  - humanにはsingular
  - reference約1.8% / AutoDrill約1.9%
  - → rareだが現実教材にも同程度存在するvariation。singularityだけで除外しない。

## 7. やってはいけない解釈

### d4を「最難」と扱わない

AutoDrillのdifficulty 4は `random`。

順序尺度として `d1 < d2 < d3 < d4` を仮定しない。

corpusとの比較でもd4は「semantic supportのランダムサンプル」として使う。

### external rarity = singularity とみなさない

referenceで珍しいことと、人間が「構造的に珍しい」と評価することは別の量。

外部frequencyはcurriculum compositionのevidence。
human singularityはhuman observation。

相互に補助できるが、片方で片方を置き換えない。

### pooled marginalだけで合否を出さない

worksheet compositionとdedicated archetypeを必ず見る。

### text hashを「同じ問題」と断定しない

visual objectがtext抽出に入っていないページでは、同じtext hashでも図形・配置が違う可能性がある。

normalized text hashはduplicate候補検出には使えるが、visual domainでproblem identityのSoTにしない。

### partial sampleの0件を「存在しない」と言わない

専用教材70問で0件、という結果と、全コーパスで0件、は区別する。

Issue化するときもevidence scopeを書く。

## 8. 現在までに得た重要なbaseline

以下は2026-08-31までの分析で再利用価値が高いもの。新しい分析ではraw corpusを再確認し、この表だけをground truthとして使わない。

### Fraction

- 分数加減:
  - referenceは小4同分母 / 小5異分母を明示的に分離。
  - AutoDrill genericではd4でsame denominatorが加算約10.6%、減算約37.6%。
- 分数×分数 dedicated sample:
  - 機械復元70問ではproper-fraction input 70/70。
  - AutoDrill fraction multiplication d4ではimproper input約68.4%。
  - 結論は「improperは不正」ではなく、proper-fraction dedicated themeが欠けている、というもの。

### Integer column arithmetic

- 3/4桁加減:
  - referenceは3桁 / 4桁を別教材として扱う。
  - AutoDrill genericはmixed-widthが約50%。
- 2桁加算:
  - pooled carry rateは比較的近いが、referenceはcarry条件別worksheetへ分離。
  - worksheet composition parityの重要な反例。
- 掛け算:
  - reference ordinary 2桁×2桁 sampleではzero digitをほぼ含めず、`×何十`を別教材化。
  - AutoDrill theme 30 d4ではzero digitを含むproblemが約22.5%。
- 一桁除算:
  - `843 ÷ 4 = 210 あまり3` のような「商の特定桁に0が立つ」問題はreferenceに存在する。
  - `5 ÷ 8 = 0 あまり5` のような商全体0とは別物。
  - 問題はarchetype自体ではなく、小3genericへ小4相当scopeが混ざること。

### Decimal division

referenceではscale・余り・概数・わり進み等をかなり細かく別教材にしている。

AutoDrill generic column decimal division d4では、異なるscale、除数<1、被除数<除数、整数解などが広く混在するため、dedicated curriculum archetypeとの比較が必要。

### Linear equations

human QAが揃うcalibration domain。

- `|A|=1`: AutoDrill d4約40.3%、reference direct-integer約16.1%、human QAでもhigh singularity側。
- literal RHS=0 (`x+13=0`, `10x=0`):
  - AutoDrill d4約16.25%
  - reference direct-integer 56問では0件
  - human QAでもかなりsingular
  - → User判断前提のgeneric support再検討候補。
- referenceの一次方程式124問では119/124が整数解。
- 分数係数を含む27問は27/27整数解。
- AutoDrill一次方程式(2) d4では整数解約13.2%。
  - 「分数係数処理」と「分数解」を強く結びつけすぎている可能性。

### Simultaneous equations

referenceは基本教材を加減法 / 代入法 / まとめに分離。

現行AutoDrillは標準形2本が中心で、`x=...` / `y=...` の直接代入向け提示を持たない。

### Quadratic equations

- 平方根・平方完成:
  - `ax²+c=0`だけでなく `(x-h)²=k` / 平方完成がreferenceに存在。
- 因数分解:
  - monicだけでなく、共通因数・non-monicがreferenceに存在。
- 解の公式:
  - discriminantが平方数で最終的に有理数解になる問題もreferenceに存在。
  - AutoDrillは現行modeでradicalが消えるcandidateをrejectしている。
- 中3二次方程式本編46問では分数係数0件。
  - AutoDrill theme 16 d4では表示分数係数約50.1%。
  - basic supportとしては過剰候補。advancedとしての存在可否とは分けて判断する。

## 9. 既知のデータ品質上の限界

### 9.1 vector / imageがtextのあるpageで落ちる

現在のingestは、ページにtextが1文字でもある場合、そのpage全体のrasterを保存しない。

そのため:

- geometry
- graph
- diagram
- 数式本体がvector/pathで、headerだけtext

のようなpageでは重要内容を取りこぼすことがある。

連立方程式の一部PDFで、タイトルだけtext抽出され、肝心の連立式が欠落する例を確認済み。

visual domainを本格分析する前に、次のどちらかが必要。

- text有pageも含めたvector/image抽出を追加する
- 対象PDFだけ一時再取得し、page rasterを研究用に解析する

### 9.2 PDF font mappingで `10` が `I0` 等になることがある

PyMuPDFの文字抽出で、PDF font encodingによって数字の `1` が `I` のように取れるページがある。

文章題の数値をexactに掘る前に、domain-aware glyph normalizationを挟む。

機械的に全 `I` を `1` へ置換してはいけない。

### 9.3 summary duplicate

summary PDFはindividual worksheetの再収録が多い。

sample countへそのまま入れない。

## 10. SQL query examples

### source page一覧

```sql
SELECT id, grade, title, url
FROM source_pages
ORDER BY grade, title;
```

### source pageごとのnon-summary worksheet数

```sql
SELECT
  sp.id,
  sp.grade,
  sp.title,
  COUNT(DISTINCT w.id) AS worksheet_count
FROM source_pages sp
JOIN worksheet_sources ws ON ws.source_page_id = sp.id
JOIN worksheets w ON w.id = ws.worksheet_id
WHERE w.summary = 0
GROUP BY sp.id
ORDER BY worksheet_count DESC, sp.id;
```

### source pageに紐づくindividual worksheet

```sql
SELECT DISTINCT
  w.id,
  w.filename,
  w.pdf_url,
  w.document_id
FROM worksheets w
JOIN worksheet_sources ws ON ws.worksheet_id = w.id
WHERE ws.source_page_id = ?
  AND w.summary = 0
ORDER BY w.id;
```

### page text検索

```sql
SELECT
  p.id AS page_id,
  p.document_id,
  p.page_number,
  snippet(pages_fts, 0, '[', ']', ' … ', 16) AS hit
FROM pages_fts
JOIN pages p ON p.id = pages_fts.rowid
WHERE pages_fts MATCH '方程式'
LIMIT 50;
```

### worksheetからpage textを読む

```sql
SELECT
  w.id AS worksheet_id,
  p.page_number,
  p.text
FROM worksheets w
JOIN pages p ON p.document_id = w.document_id
WHERE w.id = ?
ORDER BY p.page_number;
```

## 11. Issueへ反映するときの基準

### dedicated archetype欠落

referenceに明示的な専用教材があり、AutoDrillではgeneric themeへしか存在しない場合:

- dedicated theme / skill追加候補
- genericを残すなら「まとめ」であることを明示

### curriculum scope mismatch

referenceでは別学年・別単元なのにAutoDrillのthemeへ混入する場合:

- bug候補
- 例: 小3一桁除算themeへ小4相当の3桁÷1桁・商の桁0が混ざる

### reference 0 occurrence

十分なproblem-unit確認を行っても0件で、AutoDrillでは頻出する場合:

- 出題ミス / overproduction候補
- 数学的validityとは分ける
- 最終採否はUser判断
- human QAがあれば併記する

### referenceとfrequencyが近いrare variation

human singularityが高くても、referenceとAutoDrillの頻度が整合するなら、単純除外しない。

## 12. 今後の拡張

優先度が高いのは次。

1. arithmetic / fractions / equationsのproblem-unit extractionを正式なderived layerへする
2. `source_page`を起点に `pedagogical_archetype` metadataをderived tableとして付与する
3. within-sheet feature-shareを自動比較するreport toolを作る
4. visual/vector pageの保存方式を補強する
5. word problemは quantitative relation / semantic schema単位でsegmentationする

canonical corpus text/pagesを直接書き換えず、archetypeやproblem-unitは再生成可能なderived dataとして追加するのが望ましい。

## 13. 関連Issue

- #109 ドリル品質を制御するsingularityモデル
- #121 低コストarchetype拡充tracker
- #110〜#138 のcurriculum/archetype系Issue

これらのIssueはGitHub Issuesをsource of truthとし、この文書をbacklog mirrorとして使わない。
