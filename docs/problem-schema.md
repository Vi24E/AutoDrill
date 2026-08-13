# Problem schema v4

Alpha 1.2のRust domainとWASM JSON境界は`schema_version = 4`を共通で使用する。生成requestは次の値だけを持つ。
すべての公開requestは`schema_version: 4`を明示し、v2/v3、version欠落、互換DTOはfail closedで拒否する。

```json
{
  "schema_version": 4,
  "numeric_theme_id": 1,
  "seed": "Ab3Z",
  "difficulty": 2,
  "timeout_ms": 500,
  "max_attempts": 10000
}
```

`timeout_ms`と`max_attempts`は省略可能で、既定値は500msと10,000試行である。問題数はrequestやIDへ入れず、revision付きgenerator registryから得る。現在の登録は次の通り。

| theme | numeric ID | revision | layout |
|---|---:|---:|---|
| 一桁の足し算 | 1 | 3 | 20問・2列10行 |
| 一次方程式(1) | 2 | 6 | 16問・2列8行 |
| 一次方程式(2) | 3 | 6 | 16問・2列8行 |
| 一桁の引き算 | 4 | 1 | 20問・2列10行 |
| 二桁の足し算 | 5 | 1 | 20問・2列10行 |
| 九九 | 6 | 1 | 20問・2列10行 |
| 負の数の計算(1) | 7 | 1 | 20問・2列10行 |
| 負の数の計算(2) | 8 | 1 | 20問・2列10行 |
| 分数の足し算 | 9 | 1 | 16問・2列8行 |
| 分数の掛け算 | 10 | 1 | 16問・2列8行 |
| 分数の引き算 | 11 | 1 | 16問・2列8行 |

## Problem-set identity

ID形式は次の可逆なASCII表現である。

```text
{schema_version}-{numeric_theme_id}-{generator_revision}-{seed}-{difficulty}
```

例は`4-1-3-Ab3Z-2`。Seedは1〜16文字で、`1-9`、`a-z`、`A-Z`から`I`、`l`、`O`を除いた集合だけを許可する。`-`を許可しないため分割は曖昧にならない。decodeした`ProblemSetIdentity`と保存済みrevisionのgeneratorで同じ問題セットを再生成できる。

Worksheetは`problem_set_id`に加えてdecode済み`identity`、registry由来のskill/curriculum/layout、registry所定個数のProblemを持つ。Problemはversion、通し番号、numeric theme ID、型付きprompt、`answer_schema`、それとは直交するtyped `input_interface`、正解AnswerNode、標準解法graph、固定長operation vector、解決済み重みによるeffortを持つ。数学値は整数またはAnswerNodeの正確な十進表現で保持し、binary floatは使用しない。JSON/WASM境界のAnswerNode integer/coefficientとanswer-schema bounds（`i64`）、BigNum magnitude（`u64`）はcanonical decimal stringとし、JavaScriptの安全整数範囲を超える18桁値も可逆にする。`effort`とoperation vector quantityだけは評価結果なので`f64`を許可する。

`input_interface`の例:

```json
{"type":"simple_numeric","allow_decimal":false,"allow_negative":false}
```

構造化テーマは`{"type":"structured_math","allowed_structures":["fraction","root"]}`のように
許可する構造を返す。Webの`apply_editor_action` requestは、このProblemから選択したinterfaceを
`state`と`action`と同じv3 envelope内で送る。Webはinterfaceを`answer_schema`から推測しない。
Typed leaf/composite answer、editor state、editor candidate、gradeのexpected/actualは、このinterfaceの
capability projectionを満たす必要がある。`nan_error`だけはboundedなraw-text recovery sentinelとして
保持できるが、digits-only `simple_numeric`から小数・負数・構造のtyped nodeを回復経路で生成することはできない。


## 算術式generator

`ProblemPrompt::Arithmetic`は整数/rational literalと`+,-,×,÷`のbinary nodeからなるexactな`ArithmeticExpression`を保持する。一桁引き算、二桁加算、九九、負の数(1)/(2)、分数加算/減算/乗算はこのpromptを共有し、Web/PDFは同じtyped ASTから式を描画する。

- 一桁引き算: `a-b=c`、`1<=a<=18`、`1<=b,c<=9`。
- 二桁加算: `a+b=c`、`10<=a,b<=99`。
- 九九: `1<=a,b<=9`。effortは例外的に正解`c`の`BigNum(c)=log10(c)`だけを使う。
- 負の数(1): 2〜4整数項、演算子は加減のみ。少なくとも1つ負整数を含む。
- 負の数(2): 2〜4整数leafの四則演算AST。0除算を拒否し、最終値が整数になる候補だけを採用する。
- 分数加算/減算/乗算: 一次方程式(2)の係数用`linear_fraction_domain()`（分母2〜9、`|numerator|<=10-denominator`、既約・非整数・正負）のうち、`numerator>0`だけを抽出した`positive_linear_fraction_domain()`を共有する。加算/乗算は`a,b,c>0`かつ3値すべてがこのdomainに属する候補だけを採用する。減算は`a-b=c`で`a,b,c>0`かつ3値すべてが同domainに属する候補だけを採用する。

小学生registrationには共通のfail-closed制約を適用する。`curriculum_path`が`小学...`を含む場合、prompt literal/係数、canonical answerに負数があるcandidate、または`allow_negative`/`negative`/`plus_minus`入力capabilityを持つcandidateは共通generator境界で採用しない。これは個別themeのgenerator実装に依存しない。


## 一次方程式generator

`ProblemPrompt::LinearEquation`は`a,b,c,d`をexactな`RationalCoefficient`として保持し、`ax+b=cx+d`を表す。Rustの`LinearEquationGenerator`を(1)/(2)で共有し、違いはregistryとmodeだけに置く。revision 6では、**各candidateについてcanonical answerをanswer domainから一様に復元抽出し、その答えに条件づけて式を生成する**。式生成に失敗した場合は答えを引き直さず、その答えのまま式だけを再生成する。

- `(1)`のanswer domainは整数`-15..=15`。
- `(2)`は上記整数に加え、既約形で分母2・`|numerator|<=20`、または分母3..12・`|numerator|<=15`の非整数有理数を含む。値として重複する分数はdomain内で1要素に正規化する。
- 同じ答えが複数candidateまたは最終worksheetに現れることは許容する。答えの重複を避けるための再抽選は行わない。
- 答えを固定した後、式形は`ax+b=0` / `ax+b=d` / `ax+b=cx` / `ax+b=cx+d`の4種から生成する。`ax=cx`、恒等式、解なし、0除算は棄却する。
- `(2)`では最終的な`B/A`に共通因子を持たせ、実際の約分が必要になるcandidateを高確率で生成する。
- 負定数は表示metadataにより`a−b`または`a+(-b)`とし、`0x`を表示せず、係数`1/-1`は`x/−x`とする。Webでは数学用minus、PDFでは線分を描画する。

一次方程式は全7構造を持つ`structured_math`を返す。`answer_schema`は(1)が`integer { min:-15,max:15 }`、(2)が`rational { max_abs_numerator:20,max_denominator:12,require_reduced_fraction_form:true }`である。(2)の非整数解では、未約分の通常分数は`fraction_not_reduced`、数学的に同値な帯分数・有限小数・繁分数などは`fraction_form_required`を返す。整数解を分母1の分数やexactな平方根など別表現で答えた場合は`integer_form_required`を返す。Rust coreは数学的同値性とwarningを返し、warningを○/×のどちらとして扱うかはWebの詳細設定→採点設定で決める。

## 連立方程式generator

`ProblemPrompt::SimultaneousEquation`は `ax+by=c`, `dx+ey=f` の6整数係数を保持する。連立方程式(1)では `a,b,d,e` を0でない整数、`c,f`を整数とし、6値すべての絶対値を15以下に制限する。まず整数解 `(x,y)` を `[-15,15]^2` から選び、その解を満たす2本の式を逆算する。行列式 `ae-bd != 0` を必須として一意解を保証する。

canonical answerは2要素`tuple`を内部表現として使うが、`answer_schema = ordered_pair` により二次方程式の解集合とは意味を分離する。したがって `(2,3)` と `(3,2)` は同値ではなく、`(2,2)`も重複解warningの対象にならない。入力UIは各問題に独立した `x = [ ]` / `y = [ ]` の2欄を表示し、Web側で2欄を順序付きtupleへ合成してRustへ渡す。

## Difficulty sampling

難易度は4段階とし、wire値は `1=かんたん`, `2=ふつう`, `3=むずかしい`, `4=ランダム` とする。schema v4で意味を固定し、旧schema v3のproblem-set IDはfail closedで拒否する。

n問に対して`8n`候補を1つの共通poolとして生成する。一次方程式はn=16なので**128候補**である。

1. candidate slotごとにcanonical answerをanswer domainから独立一様に選び、必要ならその答えを固定したまま式だけを再生成する。
2. 完成candidateについてSolutionGraphからeffortを計算する。
3. `かんたん` / `ふつう` / `むずかしい` はpoolをeffort順に並べ、独立に5個引いた一様indexのorder statisticから、それぞれ旧difficulty 1 / 3 / 5相当の最小・中央値・最大を使う。`n+4`問を得た後、effort最小2問・最大2問を外れ値として除く。
4. `ランダム` はeffort sortもorder statisticもtrimも行わない。candidate poolを式の完全一致で集合化して偶発的な重複multiplicityを除き、その**distinct candidate集合から一様に、非復元抽出**でn問を選ぶ。したがってdifficulty由来のeasy/hard biasを一切入れない。
5. 最終n問は同じRNG streamでFisher–Yates shuffleする。

候補生成、pool再生成、重複式の再選択は共通のattempt budgetとmonotonic deadlineで打ち切る。エラーcodeは`generation_timeout`と`generation_attempt_limit`で区別する。

WASM adapterはcore DTOをそのまま`ApiResponse`の`data`へserializeする薄い境界であり、同じschemaを別形式へ再定義しない。

`grade_answer`はnormalizedな`expected`/`actual`、`is_correct`、statusに加え、表記上の注意を`warnings`配列で返す。識別子は`fraction_not_reduced`、`integer_form_required`、`redundant_negative`、`redundant_plus_minus`、`redundant_decimal`、`duplicate_solution`、`solution_list_required`、`fraction_form_required`で、表示文言とは分離する。Webでは「約分しましょう」「整数でこたえましょう」「分数でこたえましょう」「最後まで計算しましょう」の4表示カテゴリへまとめ、詳細設定→採点設定でカテゴリごとに○/×を選ぶ。
Web grade DTOのstatusは`correct`、`incorrect`、`unanswered`だけを許可する。actualが`empty`なら
`is_correct=false`かつ`unanswered`、actualがnon-emptyなら`is_correct`に応じて`correct`または`incorrect`
でなければfail closedする。

`nan_error`はcoreが解析できなかったbounded raw textを保持するvariantである。DTO検証、表示、編集、採点結果は
このtextをそのまま保持し、JavaScriptの数値、空欄、暗黙の不正解値へ変換しない。`nan_error`を含む回答は常に不正解で、採点側が明示的に不正解を返す場合もWebのgrade itemにはraw textを表示する。
