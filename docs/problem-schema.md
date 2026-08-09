# Problem schema v3

Alpha 1.1のRust domainとWASM JSON境界は`schema_version = 3`を共通で使用する。生成requestは次の値だけを持つ。
すべての公開requestは`schema_version: 3`を明示し、v2、version欠落、互換DTOはfail closedで拒否する。

```json
{
  "schema_version": 3,
  "numeric_theme_id": 1,
  "seed": "Ab3Z",
  "difficulty": 3,
  "timeout_ms": 100,
  "max_attempts": 10000
}
```

`timeout_ms`と`max_attempts`は省略可能で、既定値は100msと10,000試行である。問題数はrequestやIDへ入れず、revision付きgenerator registryから得る。現在の登録は次の通り。

| theme | numeric ID | revision | layout |
|---|---:|---:|---|
| 一桁の足し算 | 1 | 3 | 20問・2列10行 |
| 一次方程式(1) | 2 | 6 | 16問・2列8行 |
| 一次方程式(2) | 3 | 6 | 16問・2列8行 |

## Problem-set identity

ID形式は次の可逆なASCII表現である。

```text
{schema_version}-{numeric_theme_id}-{generator_revision}-{seed}-{difficulty}
```

例は`3-1-3-Ab3Z-3`。Seedは1〜16文字で、`1-9`、`a-z`、`A-Z`から`I`、`l`、`O`を除いた集合だけを許可する。`-`を許可しないため分割は曖昧にならない。decodeした`ProblemSetIdentity`と保存済みrevisionのgeneratorで同じ問題セットを再生成できる。

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


## 一次方程式generator

`ProblemPrompt::LinearEquation`は`a,b,c,d`をexactな`RationalCoefficient`として保持し、`ax+b=cx+d`を表す。Rustの`LinearEquationGenerator`を(1)/(2)で共有し、違いはregistryとmodeだけに置く。revision 6では、**各candidateについてcanonical answerをanswer domainから一様に復元抽出し、その答えに条件づけて式を生成する**。式生成に失敗した場合は答えを引き直さず、その答えのまま式だけを再生成する。

- `(1)`のanswer domainは整数`-15..=15`。
- `(2)`は上記整数に加え、既約形で分母2・`|numerator|<=20`、または分母3..12・`|numerator|<=15`の非整数有理数を含む。値として重複する分数はdomain内で1要素に正規化する。
- 同じ答えが複数candidateまたは最終worksheetに現れることは許容する。答えの重複を避けるための再抽選は行わない。
- 答えを固定した後、式形は`ax+b=0` / `ax+b=d` / `ax+b=cx` / `ax+b=cx+d`の4種から生成する。`ax=cx`、恒等式、解なし、0除算は棄却する。
- `(2)`では最終的な`B/A`に共通因子を持たせ、実際の約分が必要になるcandidateを高確率で生成する。
- 負定数は表示metadataにより`a−b`または`a+(-b)`とし、`0x`を表示せず、係数`1/-1`は`x/−x`とする。Webでは数学用minus、PDFでは線分を描画する。

一次方程式は全7構造を持つ`structured_math`を返す。`answer_schema`は(1)が`integer { min:-15,max:15 }`、(2)が`rational { max_abs_numerator:20,max_denominator:12,require_reduced_fraction_form:true }`である。(2)の非整数解では、未約分の通常分数は`fraction_not_reduced` warning付き不正解、数学的に同値な帯分数・有限小数・繁分数などは`fraction_form_required` warning付き正解とする。整数解を分母1の分数やexactな平方根など別表現で答えた場合は`integer_form_required` warning付き正解とする。

## Difficulty sampling

n問に対して`8n`候補を1つの共通poolとして生成する。一次方程式はn=16なので**128候補**である。

1. 128個のcandidate slotそれぞれでcanonical answerをanswer domainから独立一様に1つ選ぶ。同じ答えの重複を許す。
2. その答えを固定したまま、条件を満たす一次方程式を1つ生成する。失敗時は答えではなく式だけを再試行する。
3. 完成した128式すべてについて、`ax+b=cx+d -> Ax=B -> x=B/A -> 最終既約解`までの`SolutionGraph`からeffortを計算する。答えの分子・分母の大きさや、実際に必要な約分もこのeffortへ含む。
4. 128候補をeffort昇順、式、生成ordinalの順で決定的にsortする。ここでは答えごとのグループ分けをしない。例えば同程度の式構造なら`x=13/12`は`x=1`より一般に高effortになる。
5. 残りpoolから独立な一様indexを5個引き、難易度iはi番目のorder statisticが指すcandidateを選ぶ操作を、`n+4`問が得られるまで繰り返す。完全に同じ式の重複だけは除く。
6. 選ばれた`n+4`問をeffort順に並べ、最小2問と最大2問を外れ値として捨ててn問にする。
7. 最終n問を同じRNG streamでFisher–Yates shuffleする。

候補生成、pool再生成、重複式の再選択は共通のattempt budgetとmonotonic deadlineで打ち切る。エラーcodeは`generation_timeout`と`generation_attempt_limit`で区別する。

WASM adapterはcore DTOをそのまま`ApiResponse`の`data`へserializeする薄い境界であり、同じschemaを別形式へ再定義しない。

`grade_answer`はnormalizedな`expected`/`actual`、`is_correct`、statusに加え、表記上の注意を`warnings`配列で返す。識別子は`fraction_not_reduced`、`redundant_negative`、`redundant_decimal`、`fraction_form_required`、`integer_form_required`で、表示文言とは分離する。warningはcorrect専用ではなく、明示された回答形式違反を説明するincorrect結果にも付けられる。
Web grade DTOのstatusは`correct`、`incorrect`、`unanswered`だけを許可する。actualが`empty`なら
`is_correct=false`かつ`unanswered`、actualがnon-emptyなら`is_correct`に応じて`correct`または`incorrect`
でなければfail closedする。

`nan_error`はcoreが解析できなかったbounded raw textを保持するvariantである。DTO検証、表示、編集、採点結果は
このtextをそのまま保持し、JavaScriptの数値、空欄、暗黙の不正解値へ変換しない。`nan_error`を含む回答は常に不正解で、採点側が明示的に不正解を返す場合もWebのgrade itemにはraw textを表示する。
