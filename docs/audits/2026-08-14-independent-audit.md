> **Historical document:** この文書は履歴保存用であり、現行仕様のsource of truthではありません。現在の設計は `docs/principles.md` / `docs/architecture/`、未解決事項は `docs/issues.md` を参照してください。

# AutoDrill 独立監査記録 — 2026-08-14

## 0. 監査情報

- 監査日: **2026-08-14 (JST)**
- 対象: `/Users/koharayu/Desktop/LLM/projects/AutoDrill`
- 役割: 独立 TESTER
- 方針: 既存テスト・既存実装判断・「動いている」という報告を正しさの根拠とせず、コード、実生成、多数Seed、独立再計算、実ブラウザ系検査を組み合わせて確認した。
- production code の既存未コミット差分は監査開始時に記録し、`git reset / checkout / restore / clean` は使用していない。
- commit / push は行っていない。
- mobile / narrow viewport 対応は、開発者方針により **alpha の対象外**。技術的な問題は確認済みだが、alpha release blocker とは扱わない。

監査開始時点で、以下の8ファイルに既存の staged 変更があった。

```text
apps/web/src/app/globals.css
apps/web/src/components/AutoDrillApp.test.tsx
apps/web/src/components/AutoDrillApp.tsx
apps/web/src/components/CustomSelect.tsx
apps/web/src/generated/drill-core-contract.ts
crates/drill-core/src/generator.rs
crates/drill-core/src/model.rs
docs/implementation-status.md
```

## 1. 結論

数学コアの正しさ、Seed決定性、重複排除、Rust/WASM/Web DTO 境界はかなり堅牢である。一方、教材としての分布と effort の意味付けには重要な改善余地がある。

特に今回の追加監査で重要だったのは次の4点である。

1. **High: 小数の effort graph が、小数計算を既約分数の計算として評価している。** これは児童が実際に行う筆算・小数点操作と異なり、同程度に見える問題へ大きく異なる effort を与える。
2. **High: 二次方程式(3)で、解の公式の `±` を二つの正しい解へ展開して入力すると不正解になる場合がある。** 採点の solution-set 展開が top-level `±` にしか対応していない。
3. **Medium: 分数の掛け算 easy の極端な単位分数偏重は、difficulty sampler だけでなく候補domain自体が主因。** samplerを穏当化するだけでは十分に直らない。
4. **Medium: 二次方程式(2)の effort は絶対値・相対値とも粗い。** 現在は実質 `5 + log10(|c|)` であり、因数分解の探索負荷を直接表していない。

今回、明確なUIバグと明示的に追加指定された小数除法については production code を修正した。

- 二次方程式等の回答欄で `x \\, =` のようにバックスラッシュが見える問題を修正。
- 小5「小数の掛け算と割り算」の除法で、**表示される被除数を有効数字3桁以内**に制限。

その他の sampler / 分数generator / effort graph / 二次方程式採点は、この監査では勝手に変更していない。

---

## 2. 大規模監査の範囲と数学的正しさ

### 2.1 主監査

全20テーマについて、4 difficulty × 300 Seedを基本単位として生成した。

- 20テーマ × 4 difficulty × 300 Seed
- **24,000 worksheet**
- **410,400問**を独立実装した計算器・論理解法で再検算
- 追加の分布特化監査まで含め、生成観測数は **828,012問以上**

主監査では以下を検出しなかった。

- canonical answer の数学的不整合: **0**
- 同一worksheet内の重複prompt: **0**
- 同一Seed・同一revisionの決定性違反: **0**
- `problem_set_id` からの再生成不一致: **0**
- easy / normal の effort 非減少順違反: **0**
- 「うそつきだれだ」の一意解違反: **0**
- 「うそつきだれだ」の全員正直・全員うそつき: **0**
- 「うそつきだれだ」での含意生成: **0**

全20テーマで、多数Seedの平均effortは

```text
かんたん < ふつう < むずかしい
```

となった。したがって、現行difficulty selectionは「effortで定義された難しさを分離する」という意味では機能している。

ただし、**effort と教材上の体感難易度が一致していることとは別問題**である。以下の追加監査でそのずれを確認した。

---

# 3. effort model 監査

## 3.1 effort architecture 自体の評価

`crates/drill-core/src/effort.rs` は、

- `SolutionGraph`
- 27次元の `OperationVector`
- `OperationWeights`
- grade / theme / mastery の multiplier layer

を分離している。これは設計として良い。

各問題を「標準解法に必要な操作ベクトル」へ落とし、その後に重み付けする構成なので、将来の学年差・習熟度差をgraphを書き換えず調整できる。また、「easyでは解法の基本を練習し、hardでは計算負荷を上げる」「無駄に面倒な問題を抑える」という目的とも整合する。

特に以下は概ね妥当である。

- 一桁加減・二桁加算: 基本演算、繰上がり・繰下がりを別cost化。
- 九九 / 九九の逆算: 製品仕様どおり `log10` 系のanswer/dividend costのみを使う例外設計。
- 符号付き整数: ASTをbottom-upに評価し、負数overheadを分離。
- 分数: 通分、乗算、約分、GCD等を別操作として持つ。
- 一次方程式: 移項、係数整理、除算を標準解法として表す。
- 連立方程式: 加減法のための係数調整、消去、代入を近似。
- 二次方程式(1)/(3): 平方根化・解の公式という解法構造を明示している。
- うそつきだれだ: 仕様上、SAT式が参照するliteral数そのものをeffortとするため、実装は仕様に忠実。

一方、以下は改善が必要である。

---

## 3.2 High — 小数が「分数」としてeffort計算されている

### 対象

小4「小数の足し算と引き算」、小5「小数の掛け算と割り算」。

### 根拠コード

`crates/drill-core/src/effort.rs:780` 前後の `arithmetic_expression_operations()`。

`ArithmeticExpression::ExactDecimal` はまず

```text
coefficient / 10^scale
```

という `RationalCoefficient` に変換される。`RationalCoefficient::new()` は既約化するため、その後の加減算では `rational_addition_operations` / `rational_subtraction_operations` が使われ、通分・LCM・乗算・GCDのcostが発生する。

### 実例

独立に `arithmetic_expression_graph()` を呼び出した結果:

```text
0.7 + 0.2
  effort = 14.9542
  [OverheadLcm, BaseTimes, BaseTimes, BasePlus, BigNum(9)]

0.7 + 0.3
  effort = 3.0000
  [BasePlus, BigNum(1)]
```

同様に、

```text
0.4 + 0.5 -> 14.9542
0.4 + 0.6 ->  3.0000
```

となった。

これは、`0.2 = 1/5` のように内部で約分されるかどうかによって「分母が違う分数の通分」として扱われるためである。しかし小4児童にとって、これらはいずれも同じ小数第1位同士の加法であり、この差は体感計算量を表していない。

乗算でも:

```text
0.2 × 0.5 -> effort 4.5000
0.3 × 0.4 -> effort 12.0792
```

と大差が生じる。

### 評価

**現行 effort architecture を捨てる必要はない。** 問題は operation vector へ落とす方法である。

小数だけは専用の標準解法graphを持たせるのがよい。

- 加減: 小数点位置合わせ、各桁の加減、繰上がり・繰下がり。
- 乗算: 整数部分としての筆算 + 小数点位置決定。
- 除算: 除数を整数化する10の冪移動 + 除算step + 商の桁数。

既存の `BasePlus`, `BaseMinus`, `BaseTimes`, `BaseDivide`, `TimeTen`, carry overhead等を再利用できるため、統一的なeffort設計は維持できる。

---

## 3.3 difficulty sampler の現行仕様

`crates/drill-core/src/generator.rs:829-842`。

各選抜stepで候補を5個引き、effort順に並べ、現在は:

- easy: 1番目（最小）
- normal: 3番目（中央値）
- hard: 5番目（最大）

を常に採る。

5個のiid drawに対するorder statisticとして見ると、母集団の期待quantileは概ね:

- easy: `1/6 = 16.7%`
- normal: `3/6 = 50%`
- hard: `5/6 = 83.3%`

である。

これはdifficulty差を強く作れる一方、ある「問題形式」とeffortが相関していると、その形式自体をeasy/hardから消しやすい。

---

## 3.4 提案された5順位の確率混合の評価

提案:

```text
easy:
  rank 1: 60%
  rank 2: 30%
  rank 3: 10%

normal:
  rank 2: 25%
  rank 3: 50%
  rank 4: 25%

hard:
  rank 3: 10%
  rank 4: 30%
  rank 5: 60%
```

### 理論評価

期待quantileは:

```text
easy   = (0.6*1 + 0.3*2 + 0.1*3) / 6 = 25%
normal = (0.25*2 + 0.5*3 + 0.25*4) / 6 = 50%
hard   = (0.1*3 + 0.3*4 + 0.6*5) / 6 = 75%
```

となる。

つまり現在の `16.7 / 50 / 83.3%` から `25 / 50 / 75%` へ寄せる方式であり、**difficultyの向きは保ったまま極端なtail selectionを弱める**。

### production外の一時コピーでの実験

production codeは変更せず、一時コピーだけでこのsamplerを実装し、全20テーマ × 4difficulty × 200 Seedを再生成した。

結果:

- 全20テーマで `easy < normal < hard` の平均effort順を維持。
- generation error: 0
- math error: 0
- duplicate worksheet prompt: 0
- easy/normal sort violation: 0
- easy-hardの平均effort差は概ね現行の75%程度となり、約25%圧縮された。

代表例:

```text
                 current E/N/H       proposed E/N/H
一次方程式(1)   12.23/15.84/20.08   13.14/15.98/19.02
分数の掛け算     5.99/ 8.08/11.25    6.35/ 8.15/10.51
二次方程式(2)    5.79/ 6.28/ 6.65    5.92/ 6.27/ 6.56
連立方程式       30.65/40.08/48.31   32.77/40.23/46.50
うそつきだれだ    4.88/ 6.33/ 8.27    5.22/ 6.35/ 7.72
```

「うそつきだれだ」の人数偏重も軽減した。

```text
current easy: 3人 89.1%
proposed easy: 3人 81.1%

current hard: 4人 91.6%
proposed hard: 4人 83.8%
```

### 結論

**このsampler変更には賛成寄り。** 実装が単純で、現行effort architectureを壊さず、形式collapseを緩和する。

ただし、これは「samplingによる増幅」を弱めるだけで、元のgenerator domainが偏っている場合は直せない。特に分数の掛け算easyがその例である。

---

# 4. 分数 easy の偏りの原因

## 4.1 分数の掛け算の候補domain

`crates/drill-core/src/generator.rs:1613-1631`

operandは:

- 分母 2..=9
- 分子 1..=(10 - 分母)
- 約分後に整数になるものは除外
- 重複を除外

で作られる。

実際の異なるoperandは22個。そのうち単位分数は8個、**36.4%**。

さらに `generator.rs:1676-1679` で、分数の掛け算・引き算は結果についても

```rust
operand_domain.contains(&result)
```

を要求している。

つまり「答えも同じ狭いoperand domainへ戻る問題」しか候補にならない。

### 掛け算domainの完全列挙

この条件を満たす掛け算は **94 ordered triples** しかない。

- 候補94問中、少なくとも一方が単位分数: 54問 = 57.4%
- 答えが単位分数: 50問 = **53.2%**

したがってdifficulty selectionを行う前から、randomで単位分数の答えが約半数になる。

これは実測randomの53.7%と一致する。

## 4.2 effortとの相互作用

分数乗算のeffortでは、分子・分母の積を別操作として数えるが、±1との掛け算は `Identity` として安く扱う。

これは「1×nは九九を本格的に計算しない」という人間モデルとして妥当である。しかし、元の候補domainが単位分数へ偏っているため、その妥当なcost設計がeasyの偏りをさらに増幅する。

94候補をeffort順に完全列挙すると:

- 最もeasyな25%: 単位分数答え **83.3%**
- 最もeasyな33%: **87.1%**
- 最もeasyな50%: **91.5%**

となった。

### worksheet実測（300 Seed / difficulty）

```text
current easy   88.38% 単位分数
current normal 65.42%
current hard   12.69%
current random 53.71%
```

提案された穏当samplerでも:

```text
proposed easy   85.12%
proposed normal 62.94%
proposed hard   26.31%
random          53.71%
```

であり、**easyの88.4% → 85.1%にしか改善しない。**

### 修正方針候補

分数の掛け算についてはsamplerより先にgeneratorを直すべきである。

1. operand domain と result domain を分離する。
2. `operand_domain.contains(result)` をやめ、結果の分子・分母に教材上の独立した上限を設ける。
3. 小6の乗法では整数相手も候補へ入れる。現状は分数×整数 / 整数×分数が0件である一方、除法側は整数1..9を明示的に追加している。
4. 修正後にsource distributionを再監査し、その後にsamplerを評価する。

文部科学省『小学校学習指導要領（平成29年告示）解説 算数編』の分数の乗除の扱いとも照合して、整数相手を含む基本パターンを欠落させないことが望ましい。

---

# 5. 二次方程式(2)の effort 監査

## 5.1 generator

`crates/drill-core/src/generator.rs:2511-2534`。

- 根 `r1, r2` を ±1..±9 から生成。
- 等しい場合は符号反転を試み、結果的に同じ根を持つ問題を除外。
- `b = -(r1+r2)`
- `c = r1*r2`
- 外側scaleを1..5から生成。
- 表示は `scale(x² + bx + c) = 0`。

これは因数分解で解く `x²+px+q=0` 型を中心にするという中3教材設計としては自然である。

文部科学省『中学校学習指導要領（平成29年告示）解説 数学編』の二次方程式の項でも、因数分解、平方根への帰着、解の公式を扱い、因数分解では `x²+px+q=0` 型を中心に扱う構成が示されている。

## 5.2 現在のeffort

`crates/drill-core/src/effort.rs:741-744`:

```rust
pub fn quadratic_factoring_graph(answer: &AnswerNode) -> SolutionGraph {
    let mut operations = vec![Operation::OverheadFactorGeneral];
    operations.extend(big_num_operations(answer));
    operations_graph(operations)
}
```

既定weightは `OverheadFactorGeneral = 5`。

根が `r1,r2` なので、現在のeffortは実質:

```text
effort = 5 + log10(|r1|) + log10(|r2|)
       = 5 + log10(|c|)
```

である。

したがって:

- `b` を見ていない。
- 符号の組合せを見ていない。
- 因数候補を何組検討する必要がありそうかを直接見ていない。
- 外側scaleを見ていない。
- 絶対値は5.0〜約6.91にしかならない。

### 実例

```text
5(x² + 4x + 3) = 0
roots = -3,-1
 effort = 5.4771

3(x² - 7x - 8) = 0
roots = -1,8
 effort = 5.9031

4(x² - 81) = 0
roots = -9,9
 effort = 6.9085
```

対して二次方程式(1)では、平方根に帰着するだけでも `OverheadQuadratic + BaseDivide + BaseRoot` が入り、13前後になる。二次方程式(3)は解の公式の演算列により25以上が基本となる。

したがって、**二次方程式(2)の絶対effortが不当に小さく見えるという指摘は正しい。**

ただし注意点がある。Q2の全問題へ `OverheadQuadratic=6` を一律追加するだけなら、表示上の絶対値は改善するが、Q2内の順位は一切変わらない。difficulty selectionを改善するには、因数分解内部の相対的な探索負荷をmodel化する必要がある。

## 5.3 現在のeffortにも一定の有効性はある

300 Seed × difficultyでQ2を調べると、difficulty上昇に伴い `|c|` と約数候補数が増えている。

```text
              mean |c|
easy            7.17
normal         20.92
hard           46.35
random         25.57
```

したがって `log|c|` は「因数候補の多さ」の弱いproxyとして機能している。完全に無意味ではない。

しかし、hardでも根は±1..±9に限定されるため、計算能力向上を狙うdifficultyとしては表現幅が小さい。

## 5.4 改善案

1. Q2にも `OverheadQuadratic` を入れて、異なる解法テーマ間のabsolute effortの意味を揃える。
2. relative difficultyには、`c`の因数候補数や符号候補数を `Count` 等で入れる、または因数探索専用operationを追加する。
3. `b=0` の差の平方型など、すぐ見抜ける特殊形は別costにする。
4. 外側scale 1..5は約80%で1以外になるが、解法上ほぼただの共通因子なので、教材上必要が薄ければ出現率を下げる。
5. 重解は現generatorでは0件。必須違反ではないが、二次方程式の解の個数を学ぶ教材coverageとしては別途入れる価値がある。

提案samplerをQ2へ使うとeasy-hard差はさらに小さくなるため、Q2 effort graphの改善を先に、または同時に行う方がよい。

---

# 6. 二次方程式 UI / 採点監査

## 6.1 修正済み — `x \\, =` と表示される

### 症状

二次方程式の回答欄で、本来 `x =` となるべき部分が

```text
x \\, =
```

のようにバックスラッシュを可視化していた。

### 原因

Web側だけ `answerPrefix.replaceAll()` のMathLive spacing commandを二重escapeしており、MathLiveへ `\\,` が渡っていた。

PDF側は同じ場所で正しく `\,` を作っていたため、Web/PDFの実装も不一致だった。

### 修正

`apps/web/src/domain/mathlive-format.ts:4` に

```ts
answerPrefixLatex()
```

を追加し、WebとPDFの両方から同じformatterを利用するようにした。

- `apps/web/src/components/AutoDrillApp.tsx:495`
- `apps/web/src/pdf/worksheet-pdf.tsx:131,146`

回帰テストで `x =` が**MathLiveのspacing command 1個だけ**へ変換されることを固定した。

この修正は二次方程式だけでなく、同じ `answerPrefix: 'x ='` を利用する一次方程式にも効く。

---

## 6.2 High — 二次方程式(3)で、明示した2解を正解と認識しない

### 再現

canonical answer:

```latex
\frac{6\pm\sqrt{21}}{3}
```

を、数学的に同じ二解として

```latex
\frac{6+\sqrt{21}}{3},\frac{6-\sqrt{21}}{3}
```

と入力する。

実際の `parse_mathlive_answer()` + `grade_answer()` で検査した結果:

```text
compact ± form: correct=true
explicit 2-root tuple: correct=false
```

### 原因

`crates/drill-core/src/grade.rs:86-98` の `solutions_mathematically_equal()` は、

```text
top-level PlusMinus <-> Tuple
```

だけを特別扱いする。

Q3のcanonical answerは多くの場合、`Fraction(Binary(... PlusMinus(...)), denominator)` のように `±` が内部に埋まっているため、solution setへ展開されない。

### 期待

二つの根をそれぞれ正しく書いた答案は正解にすべきである。

### 修正候補

AnswerNodeを「有限solution set」へ展開する正規化関数を採点専用に用意し、`PlusMinus` がfraction/binary内部にあっても全体の値へ分配した上で集合比較する。

今回の監査では、採点coreまで無断で変更せず**未修正**として残した。

---

## 6.3 Medium — Q2の2解入力のdiscoverability

Q2 canonical answerは `Tuple([root1, root2])` であり、順序は採点時にcanonicalizeされるので、`2,3` と `3,2` は同値として扱える。この点は正しい。

ただしUIは

```text
x = [一つのMathLive入力欄]
```

で、ユーザーは数式パネルの「複数解」テンプレートを使うかカンマを入力する必要がある。問題文はその入力方法を説明していない。

数学的には問題ないが、中学生向けUIとしては

```text
x = [   ], [   ]
```

のような二枠UI、または短い入力説明の方がdiscoverabilityは高い。

ただし、Q3では `±` 1式で答える場合もあるため、全二次方程式を単純に二枠固定にはしない方がよい。

---

# 7. 小5小数除法 — 被除数3有効数字制限（修正済み）

## 7.1 仕様

ユーザー指定により、除法で表示される**被除数は有効数字3桁以内**とする。

除数・商の既存仕様は維持する。

## 7.2 修正

`crates/drill-core/src/generator.rs:1989` 付近。

reverse generationで

```text
被除数 = 商 × 除数
```

を計算して表示用decimalへ変換した後、`arithmetic_leaf_significant_digits()` で3桁を超える候補をrejectする。

`crates/drill-core/src/model.rs` の

```text
GENERATOR_REVISION_DECIMAL_MULTIPLY_DIVIDE
```

を **2 -> 3** に更新し、Web contractも同期した。

## 7.3 独立検証

300 Seed × 4difficultyで除法だけを再集計。

```text
easy   division 2372問, >3 significant digits = 0
normal division 2701問, >3 significant digits = 0
hard   division 3728問, >3 significant digits = 0
random division 2865問, >3 significant digits = 0
```

合計 **11,666問の除法で違反0件**。

なお「3有効数字以内」であり、小数点以下桁数を3以内へ変更したわけではない。例えば非常に小さい数は有効数字が少なくても小数第4位等を持ち得る。これをさらに禁止するかは別仕様である。

---

# 8. その他の教材分布に関する既知事項

## 8.1 分数の掛け算に整数相手がない

分数の掛け算では、生成された19,200問のoperandに整数が0件だった。

一方、分数の割り算はgenerator側で整数1..9を明示追加している。

分数×整数 / 整数×分数を基本パターンとして追加すべきである。

## 8.2 分数の引き算

小5分数引き算では:

```text
easy:   同分母 85.0%
normal: 同分母 45.8%
hard:    同分母 7.0%
random: 同分母 47.7%
```

となった。easyが既習の同分母問題へ寄りすぎている。

また単位分数答えもeasy〜hardを通して高頻度で、結果domainの制約が強すぎる。

## 8.3 符号付き四則(2)の除法

randomでも除法を含む問題は約15%で、加減乗の約47%より少ない。

演算operator自体は一様選択だが、全AST nodeで整数結果のみ許すため、division candidateのreject率が高いことが主因。

## 8.4 「うそつきだれだ」

仕様上の正しさは確認できた。

- 6問
- 3人 / 4人のみ
- candidate poolの母集団は厳密1:1
- 含意なし
- 一意解
- 全員正直 / 全員うそつきなし
- 6statement種すべて出現
- UIはクリック直後に黒背景・白文字へ変わる

一方、現行のextreme order statisticによって、最終worksheetではeasyが3人、hardが4人へ強く寄る。提案samplerでこの偏りは軽減する。

SAT literal長effortは仕様通りだが、人間難度の完全なmodelではない。同一statementの重複や対称性が、人間には推論を簡単にしてもeffortには直接反映されない。

---

# 9. mobile / narrow viewport

前回の独立ブラウザ検査では390px viewportで、長い式の隣列越境や、正当なMathLive入力がgeometry guardで拒否されるケースを確認した。

ただし開発方針として **alphaではmobile対応を考慮しない**ことが明示されたため、この監査記録では:

```text
confirmed limitation / deferred
```

とし、alpha blockerとしてのHigh severityから外す。

将来mobile対応を開始する際には、2列absolute layoutを単純縮小するのではなく、1列reflow、紙面全体scale、worksheet単位の横スクロール等を設計段階で選ぶべきである。

---

# 10. 今回実施した修正

production codeへ加えた変更は次の2系統のみ。

### A. answer prefix UI

- `apps/web/src/domain/mathlive-format.ts`
- `apps/web/src/components/AutoDrillApp.tsx`
- `apps/web/src/pdf/worksheet-pdf.tsx`
- `apps/web/src/domain/problem-format.test.ts`

Web/PDF共通 `answerPrefixLatex()` を追加し、二重escapeを除去。

### B. 小数除法の被除数3有効数字上限

- `crates/drill-core/src/generator.rs`
- `crates/drill-core/src/model.rs`
- `apps/web/src/generated/drill-core-contract.ts`

被除数3有効数字超をrejectし、generator revisionを3へ更新。

以下は**監査・実験のみでproduction code未変更**。

- 提案されたdifficulty sampler
- 分数掛け算generator
- 二次方程式(2) effort graph
- 小数専用 effort graph
- 二次方程式(3) solution-set grading
- mobile layout

---

# 11. テスト結果

今回の修正後:

### Rust / WASM

```text
cargo fmt -- --check
cargo test --workspace --all-targets
```

結果:

```text
drill-core: 81 passed
WASM:       10 passed
total:      91 passed
0 failed
```

### Web

`apps/web` から:

```text
vitest run
tsc --noEmit
eslint . --max-warnings=0
```

結果:

```text
12 test files passed
139 tests passed
0 failed
TypeScript: pass
ESLint: pass
```

### Contract

```text
node scripts/sync-web-contract.mjs
```

同期済みで成功。

### 小数除法 dedicated audit

300 Seed × 4difficulty、除法11,666問を追加確認し、3有効数字超の被除数は0件。

### experimental sampler audit

production外コピーで20テーマ × 4difficulty × 200 Seedを検査。全20テーマで平均effortの `easy < normal < hard` を維持し、数学的不整合・重複・sort違反を検出しなかった。

---

# 12. 推奨優先順位

1. **小数専用effort graphを作る。** 現状の「小数を分数として評価」はdifficultyの意味を直接歪める。
2. **二次方程式(3)のsolution-set採点を修正する。** 明示した二つの正答が×になるのは採点上の実害。
3. **分数の掛け算domainを再設計する。** resultをoperand domainへ閉じ込めない。整数相手も入れる。
4. **二次方程式(2)のeffortをfactor-searchベースへ改善する。** absolute baselineだけを上げてもdifficulty順位は改善しない。
5. 上記source/effort修正後に、**提案された5順位確率sampler**を導入する。現行より穏当で、設計思想にも合う。
6. mobileはalpha後に別タスクとして扱う。

## 総合評価

- 数学コア / 決定性: **A-**
- Rust/WASM/Web境界: **A-**
- effort architecture: **A-**
- effort feature engineering: **B-**（小数は要修正、Q2は粗い）
- 教材分布: **B-**（分数domainとdifficulty tail selectionに偏り）
- desktop UI: **A-**（今回answer-prefixバグを修正、Q3採点は未修正）
- mobile: **alpha scope外 / deferred**

---

## 追加監査: effort標準解法モデル整合性

監査日時: 2026-08-14 22時台 JST

### 監査基準

`plan.txt` / `goal.txt` / `curriculum.md` の既存仕様と、追加確認された設計意図を基準にした。

- effort は、原則として標準解法を人間が定数時間で実行できる primitive operation に分解した**操作回数vector**を先に作り、その後で重みを掛ける。
- `BasePlus/Minus/Times/Divide` はそれぞれ一桁の基本演算であり、多桁計算を1回へ畳んではならない。
- `Overhead*` は主に間違いやすさ等の追加costであり、本体の計算手順の代替ではない。
- 明示的な例外は九九、九九の逆算、「うそつきだれだ」等。一般テーマを例外扱いして粗い指標だけに置き換えない。
- 二次方程式(2) `x^2+bx+c=0` は、`c` の約数を列挙し、積が `c` となる符号付き因数対について和が `b` かを確認する標準モデルとする。

### 総括

現行effort architecture（SolutionGraph → operation vector → weights）は維持すべきだが、**SolutionGraphの構築が複数テーマで標準計算モデルより粗い**。特に、次の4系統は修正が必要である。

1. 多桁演算を `BasePlus/Minus/Times/Divide` 1回に畳んでいる。
2. LCM/GCD/PF/因数分解の `Overhead*` を、実際の探索・列挙計算の代わりにしている。
3. 小数を十進筆算ではなく有理数演算として評価している。
4. 二次方程式のgraph constructorへ式係数が渡らず、問題ごとの標準計算回数を原理的に数えられない。

### 構造上の強い証拠

- `quadratic_square_graph(answer)` は `a,c,form` を受け取らない。そのため `a=1` で除算不要か、`ax^2+c=0` で移項が必要かを区別できない。
- `quadratic_factoring_graph(answer)` は `b,c` を受け取らない。そのため `c` の約数個数、因数対の探索回数、和が `b` かの比較回数を計算できない。
- `quadratic_formula_graph(has_fraction_coefficients, answer)` は `a,b,c` の値を受け取らない。そのため `b^2`, `4ac`, `2a` 等の実計算を値に応じて分解できない。
- `OperationKind` には `Compare` と `Reciprocal` がなく、`plan.txt` が想定する「一桁の大小比較」「逆数の取得」を表現できない。
- 現行productionで `TimeTen`, `OverheadPf`, `OverheadCarryMult`, `OverheadFactorDifferenceOfSquares` は一度もSolutionGraphへ生成されていない。小数・平方根簡約・因数分解の標準モデルと不整合である。

### テーマ別評価

| ID | テーマ | 評価 | 主な理由 |
|---:|---|---|---|
| 1 | 一桁足し算 | 概ね整合 | BasePlus 1回、必要なIncrement/Carry、BigNumを分離。 |
| 2 | 一次方程式(1) | 要修正 | 多桁の係数整理・最終除算を一桁primitive 1回へ畳む。 |
| 3 | 一次方程式(2) | 要修正（強） | ID2に加え、LCM/GCDを定数overheadで代用。 |
| 4 | 一桁引き算 | 概ね整合 | BaseMinus、必要なDecrement/Carry、BigNumを分離。 |
| 5 | 二桁足し算 | 概ね整合 | 2桁を2回のBasePlusへ分解し、carryのIncrementも別計上。 |
| 6 | 九九 | 明示的例外 | `log10(answer)` 指標のみ。例外として仕様化済み。 |
| 7 | 符号付き四則(1) | 要修正（強） | 10以上を含む加減算も1回のBasePlus/Minus。 |
| 8 | 符号付き四則(2) | 要修正（強） | 中間値が多桁でも×/÷を1回。長除法モデルも未使用。 |
| 9 | 分数加算 | 要修正 | LCM列挙をOverhead 1回で代用、通分×1もBaseTimes扱い、多桁分子和も1 BasePlus。 |
| 10 | 分数乗算 | 要修正 | 一桁積部分は概ね良いが、約分GCD探索を定数overheadで代用。 |
| 11 | 分数減算 | 要修正 | ID9と同種。 |
| 12 | 分数除算 | 要修正 | 逆数取得primitiveがなく、GCD探索も粗い。 |
| 13 | 九九の逆算 | 明示的例外候補 | dividendのlog指標のみ。九九系例外として扱うなら整合。 |
| 14 | 二次方程式(1) | 要修正（強） | 係数/formをgraphが見ず、不要な除算・必要な移項を区別不能。根号簡約PFも定数overhead。 |
| 15 | 二次方程式(2) | 要修正（最優先） | 約数列挙・因数対の和チェックを全く実装せず、`5 + BigNum(roots)`のみ。 |
| 16 | 二次方程式(3) | 要修正（最優先） | 解の公式のprimitive分解が不足し、係数値もgraphへ渡らない。 |
| 17 | 小数加減 | 要修正（最優先） | 十進筆算ではなくRationalへ変換してLCM/GCDを数える。 |
| 18 | 小数乗除 | 要修正（最優先） | 十進筆算/小数点移動/long divisionでなくRational演算。TimeTen/CarryMult未使用。 |
| 19 | 連立方程式 | 要修正（強） | x/y消去の真のeffort最小ではなく倍率和で選択。多桁演算を1 primitiveへ畳み、negative overheadも全体1回。 |
| 20 | うそつきだれだ | 明示的例外 | SAT式の参照literal数をeffortとする特殊モデル。 |

### 多数Seedでの定量確認

300 Seed × 4 difficultyで追加機械検査した。

- 符号付き四則(1): binary演算 47,826個中 **32,051個 (67.0%)** が、少なくとも一方の実計算operandが10以上なのに単一 `BasePlus/Minus` として処理される。
- 符号付き四則(2): binary演算 45,033個中 **8,341個 (18.5%)** が多桁operandを含み、そのうち **3,053個** は多桁の乗除算である。
- 一次方程式(1): 19,200問中 **84.9%** で係数/定数整理に多桁値が現れ、**74.0%** で最終除算operandが一桁範囲を超える。
- 一次方程式(2): 19,200問中 **87.8% / 87.1%**。
- 分数加算: 19,200問中、異分母通分で片方の倍率が1なのに両方を `BaseTimes` と数えるケース **2,502問**。通分後分子の加算に多桁値が現れるケース **14,263問**。
- 分数減算: 同じく **7,186問 / 2,229問**。
- 二次方程式(1): 19,200問中 `a=1` で本来係数除算不要なもの **2,120問**。`ax^2+c=0` 型で移項を要するもの **9,757問**。平方因子抽出を要する根号 **3,550問**。
- 二次方程式(2): 19,200問中、差の平方 `x^2-k^2` の特殊形が **1,834問**あるが `OverheadFactorDifferenceOfSquares` は一度も使われない。指定モデルで必要な「符号付き因数対の和チェック」は平均約 **10.84回/問**（最小2、最大24）だが、現行graphには0回しか存在しない。
- 連立方程式: 14,400問中 **51.4%** で式の倍率計算に多桁primitiveが必要。**97.2%** が負係数を含むが、現行は負数operationごとではなく問題全体に `OverheadNegative` を最大1回しか加えない。

### 二次方程式(2)の具体例

現行:

`quadratic_factoring_graph(answer) = OverheadFactorGeneral + BigNum(root1) + BigNum(root2)`

したがって、ほぼ `5 + log10(|c|)` だけで順位付けされる。

しかし指定標準モデルでは、例えば

- `x^2 - 9x + 20 = 0`: `20` の約数は 1,2,4,5,10,20
- `x^2 - 10x + 21 = 0`: `21` の約数は 1,3,7,21

であり、前者の方が列挙・因数対確認が多い。現行effortは `log10(21) > log10(20)` のため逆方向へ評価する。

また `x^2 - 81 = 0` は差の平方として即座に特殊因数分解できるべきだが、現行は一般因数分解と同じgraphを使う。

### 二次方程式(3)の不足

解の公式を標準計算として数えるなら、最低でも

- `b*b`
- `a*c`
- `4*(ac)`
- discriminantの加減
- 平方根
- `2*a`
- 最終分数の簡約

を値と桁数に応じて分解する必要がある。現行は通常係数の場合 `BaseTimes ×2 + BaseMinus + BaseRoot + BaseDivide` しか持たず、`4ac` と `2a` を含めた乗算回数が不足している。fraction coefficient時の「分母を払う」処理も `OverheadLcm + BaseTimes ×1` の固定形であり、式の3係数を実際に処理する回数と一致しない。

### 分数GCD/LCMの問題

`curriculum.md` は

- LCM: 倍数を必要なところまで列挙
- GCD: 約数をすべて列挙

と規定している。現行はこれを `OverheadLcm` / `OverheadGcd` 1 nodeで置換している。`Overhead*` が追加costであるという全体設計に反する。

例 `1/2 + 1/4` では、現行は `OverheadLcm + BaseTimes + BaseTimes + BasePlus`。実モデルではLCM探索そのものを分解し、通分倍率は片方が2、片方が1なので、少なくとも後者はfull `BaseTimes` ではなくIdentity/転記に近い。

### 小数

前節の監査結果をより強く確認した。`ExactDecimal` を `RationalCoefficient` へ変換した時点で標準解法strategyが変わってしまうため、重み調整では修正できない。テーマ17/18には専用solution graphが必要。

- 小数加減: 桁合わせ → 各桁BasePlus/Minus → carry/borrow
- 小数乗算: 整数筆算 → carry → `TimeTen`相当の小数点位置決定
- 小数除算: 除数を整数化する桁移動 → long division（curriculum.mdの4 BaseTimes/商の各桁）→ 小数点位置

### 推奨修正順

1. 二次方程式(2) graphを `(b,c,answer)` 依存に変更し、指定の約数列挙＋和チェックモデルを実装。
2. 共通の多桁 `add/subtract/multiply/divide` graph builderを作り、Base primitiveへ正確に分解。
3. 小数専用graphを追加。
4. LCM/GCD/PFを「overhead + 実探索operation」へ分解。
5. 二次方程式(1)/(3)を係数・form依存graphへ変更。
6. 一次方程式・連立方程式・符号付き四則・分数を共通多桁builderへ移行。
7. operation vocabularyへ少なくとも `Compare` と `Reciprocal` の追加要否を確定する。

この修正が完了する前は、difficulty sampling方式を調整しても、元になるeffort順位自体が一部テーマで標準計算モデルを正しく表していないため、sampling調整は後段に置くのが安全である。



### 追加監査の修正状況（今回実装後）

上記「現行」は監査時点の状態を保存した記録である。今回の修正後の状態は次のとおり。

| 監査指摘 | 状態 | 実装後の扱い |
|---|---|---|
| 多桁Base演算が1 primitiveへ潰れる | **修正済み** | 共通integer add/sub/multiply/divide builderを導入。各桁、carry/borrow、部分積、商桁探索をprimitiveへ分解。 |
| LCM/GCD/PFがOverheadだけ | **修正済み** | Overheadを追加costとして残し、倍数列挙・約数列挙・試し割りをCount/Compare/Base演算/Identityへ展開。 |
| 分数通分倍率1をBaseTimes扱い | **修正済み** | 倍率1はIdentity、非自明倍率は共通integer multiplication。 |
| 分数除法に逆数operationがない | **修正済み** | `Reciprocal`を新設し、除法で1回発生。その後は乗算・約分標準graphを使用。 |
| Compareがなく因数対和の一致確認を表現できない | **修正済み** | `Compare`を新設。Q2の各符号付き因数対で`p+q`計算後に1回発生。 |
| 小数をRationalとしてeffort評価 | **修正済み** | decimal専用graphへ分離。加減は桁合わせ、乗算は整数筆算+小数点配置、除算は小数点移動+長除法。LCM/GCDは小数graphに入らない。 |
| Q1がanswerしか見ずform/a/cを区別不能 | **修正済み** | `quadratic_square_graph(form,a,c,answer)`へ変更。a=1、係数除算、移項を区別。 |
| Q2が`OverheadFactorGeneral + BigNum`だけ | **修正済み** | `quadratic_factoring_graph(b,c,answer)`へ変更。約数列挙、全符号付き因数対、和計算、Compareを実装。 |
| Q2平方差を一般探索している | **修正済み** | `x^2-a^2`は`OverheadFactorDifferenceOfSquares` strategyへ分岐し一般探索を行わない。 |
| Q2完全平方を一般探索と区別できない | **修正済み** | `x^2+2ax+a^2`を検出可能な専用branchを追加。現generatorは重根を原則生成しないため主に将来互換用。 |
| Q3が係数を見ず解の公式の操作が不足 | **修正済み** | `quadratic_formula_graph(a,b,c,answer)`へ変更。分母払い、b²、ac、4ac、D、sqrt(D)、2a、最終逆数、簡約を展開。 |
| 一次方程式がA/B計算を粗く数える | **修正済み** | A=a-c、B=d-b、B/Aを共通integer/rational builderへ移行。 |
| 連立方程式が倍率和proxyでstrategy選択 | **修正済み** | x消去/y消去の完全graphを両方構築し、weight適用後のeffort最小strategyを採用。 |
| 連立方程式のnegative overheadが問題全体1回 | **修正済み** | 共通signed builderを経由し、負数operandを含む各演算ごとに発生。 |
| 九九/九九逆算/うそつきだれだの特殊model | **維持** | 明示的例外として変更していない。 |
| difficulty samplerの方式変更 | **未変更（意図的）** | 今回はeffort自体の是正のみ。既存samplerを維持。 |

### 実装上残る近似

- `Compare`は標準解法上の1回の大小・等値判定をprimitiveとして扱う。比較する整数の桁ごとの文字走査まではさらに分解していない。
- LCMは2本の倍数列を昇順に進めて最初の一致まで列挙するモデル、GCDは各数の約数候補を列挙するモデルとして固定した。実際の児童が使うショートカットはこの標準modelには入れていない。
- Q3の最終的な`(−b±√D)/(2a)`は、無理数を整数長除法へ誤変換せず、`Reciprocal`で`1/(2a)`を取る代数操作として数える。
- 完全平方trinomialの専用branchは実装済みだが、現行Q2 generatorは同根を避けるため通常生成ではほぼ通らない。

今回の修正ではweight値をdifficulty調整目的で変更していない。strategy/operation countを先に是正し、sampling調整は引き続き後段とする。

OperationVectorは27→29次元となり、既存27 indexを維持して`Compare`/`Reciprocal`を末尾追加した。serialized Worksheetが全themeで変化するため、厳密な再生成identityを守る目的で全登録themeのgenerator revisionを1段階上げた。request/envelopeのfield構成は変えずschema v4を維持し、vector長はRust WebContractからWebへ生成同期する。

## 2026-08-15 follow-up: primitive model correction

前回修正の再検証後、Base演算を「各桁の機械的な筆算回数」と解釈しすぎていた点を修正した。基準は1〜9の加算/乗算lookup tableを双方向に使う人間モデルである。

- `13-5=8`は`BaseMinus×1`、`56÷7=8`は一般計算では`BaseDivide×1`。
- lookupで決まらない商digit探索は`BaseTimes×3`。九九逆算themeだけは割り切れても常にこの探索を使う。
- `97+86`の最上位carryは`Identity`。carry overheadは発生するが、足す対象がないため`Increment`は発生しない。乗算の最終carryも同様。
- GCDは全約数の総当たりを廃止し、両数のPF→共通素因数比較へ変更。PF対象nが9×9乗算表上で認識できる場合はPF overheadのみ。
- 二次方程式(2)は`c`をPFして重複しない因数対を構成し、和と`b`を比較する。
- 根号簡約は一般PFではなく`2^2,3^2,5^2,...`による平方因子探索。
- `BaseFractionCancel` (`k/n × n`) と `BaseRootSquareCancel` (`(sqrt(n))^2`) を新primitiveとして追加。
- legacy `OverheadGcdDivisible`を削除。
- 再利用可能なeffort計算は`effort.rs`へ集約。九九・九九逆算・うそつきだれだの真の例外は`crates/drill-core/src/themes/`の各theme moduleへ配置。
- OperationVectorは29→31次元。serialized Worksheetが変わるため全themeのgenerator revisionを1段上げた。schema v4は維持。

詳細な現行モデルは`docs/effort-model.md`を正とする。
- 追加レビューで、根号から複数因子を外へ出した後の積も共通乗算builderでcountするよう補完した。
- 二次方程式(2)の因数対探索は正解pairを発見した時点で終了し、発見後の無意味な候補検査をcountしない。
