# Problem schema v7

上位原則は[`../principles.md`](../principles.md)を参照する。未解決の既知問題は[`../issues.md`](../issues.md)で追跡する。

現行Rust domainとWASM JSON境界は新規生成について`schema_version = 7`を使用する。生成requestは次の値だけを持つ。
すべてのrequestは`schema_version: 7`を明示し、それ以外のschema、version欠落、互換DTOはfail closedで拒否する。pre-releaseでは旧schemaのproblem-set ID再生成をsupportしない。

```json
{
  "schema_version": 7,
  "numeric_theme_id": 1,
  "seed": "Ab3Z",
  "difficulty": 2,
  "timeout_ms": 15000,
  "max_attempts": 10000
}
```

`timeout_ms`と`max_attempts`は省略可能で、既定値は15,000msと10,000試行である。問題数はrequestやIDへ入れず、revision付きgenerator registryから得る。現在の登録は次の通り。

| theme | numeric ID | revision | layout |
|---|---:|---:|---|
| 一桁の足し算 | 1 | 5 | 20問・2列10行 |
| 一次方程式(1) | 2 | 8 | 16問・2列8行 |
| 一次方程式(2) | 3 | 8 | 16問・2列8行 |
| 一桁の引き算 | 4 | 3 | 20問・2列10行 |
| 二桁の足し算 | 5 | 3 | 20問・2列10行 |
| 九九 | 6 | 3 | 20問・2列10行 |
| 負の数の計算(1) | 7 | 3 | 20問・2列10行 |
| 負の数の計算(2) | 8 | 3 | 20問・2列10行 |
| 分数の足し算 | 9 | 5 | 16問・2列8行 |
| 分数の掛け算 | 10 | 5 | 16問・2列8行 |
| 分数の引き算 | 11 | 5 | 16問・2列8行 |
| 分数の割り算 | 12 | 6 | 16問・2列8行 |
| 割り算(1) | 13 | 3 | 20問・2列10行 |
| 二次方程式(1) | 14 | 3 | 16問・2列8行 |
| 二次方程式(2) | 15 | 4 | 16問・2列8行 |
| 二次方程式(3) | 16 | 3 | 16問・2列8行 |
| 小数の足し算と引き算 | 17 | 5 | 20問・2列10行 |
| 小数の掛け算 | 18 | 6 | 20問・2列10行 |
| 連立方程式(1) | 19 | 3 | 12問・2列6行 |
| うそつきだれだ | 20 | 4 | 6問・1列6行 |
| 分数と整数の掛け算 | 21 | 2 | 16問・2列8行 |
| 分数と整数の割り算 | 22 | 2 | 16問・2列8行 |
| 分数総まとめ(仮分数) | 23 | 3 | 16問・2列8行 |
| 小数の割り算 | 24 | 1 | 20問・2列10行 |
| 二桁の足し算の筆算 | 25 | 2 | 16問・4列4行 |
| 二桁の引き算の筆算 | 26 | 2 | 16問・4列4行 |
| 三・四桁の足し算の筆算 | 27 | 2 | 16問・4列4行 |
| 三・四桁の引き算の筆算 | 28 | 2 | 16問・4列4行 |
| 一桁をかける掛け算の筆算 | 29 | 2 | 16問・4列4行 |
| 二桁をかける掛け算の筆算 | 30 | 2 | 16問・4列4行 |
| 一桁で割る割り算の筆算 | 31 | 2 | 12問・4列3行 |
| 二桁で割る割り算の筆算 | 32 | 2 | 12問・4列3行 |
| 小数の足し算と引き算の筆算 | 33 | 2 | 16問・4列4行 |
| 小数と整数の掛け算の筆算 | 34 | 2 | 16問・4列4行 |
| 小数と整数の割り算の筆算 | 35 | 2 | 12問・4列3行 |
| 小数の掛け算の筆算 | 36 | 2 | 16問・4列4行 |
| 小数の割り算の筆算 | 37 | 2 | 12問・4列3行 |
| すうじはひとりぼっち（Mini Sudoku） | 38 | 1 | 4問・2列2行 |

## Problem-set identity

ID形式は次の可逆なASCII表現である。

```text
{schema_version}-{numeric_theme_id}-{generator_revision}-{seed}-{difficulty}
```

現行例は`7-1-5-Ab3Z-2`。Seedは1〜16文字で、`1-9`、`a-z`、`A-Z`から`I`、`l`、`O`を除いた集合だけを許可する。`-`を許可しないため分割は曖昧にならない。現行revisionの`ProblemSetIdentity`は同じ現行generatorで決定的に再生成できる。identityのfieldはcoreではprivateで、constructor / `FromStr` / custom `Deserialize`のすべてがschema・seed・nonzero theme/revisionを検証する。

Worksheetはcoreではdecoded `identity`、registry由来のskill/curriculum/layout、registry所定個数のProblemだけを保持するvalidated aggregateである。`problem_set_id`はidentityから純粋に導出し、wire DTO生成時だけ文字列として出力する。`Worksheet::generated`はidentityとregistrationのschema/theme/revision、problem数、各Problemのschema/themeを一度に検証する。coreの`Problem`はversion、通し番号、numeric theme ID、型付きprompt、validated answer schema、validated canonical answer、そこからregistration contractで導出した`input_interface`、promptとcanonical answerからcore自身が導出したopaque `worked_solution`、そして排他的な`EffortModel`を持つ。generatorはworked-solution fieldを直接組み立てて`Problem`へ注入できない。Column multiplication / long divisionのpartial/step dataはcore domainのprivate valueであり、Webへ必要なshapeだけを`wire.rs`のDTOへ投影する。

`Problem::generated`は単にprompt/schemaのkindだけを比較しない。`AnswerSchema`の構造条件（整数範囲の`min <= max`、有理数の正の最大分母、非空tuple length等）を検証し、canonical answerがその具体的schemaのrange/shapeを満たした後、`semantics.rs`のgenerator-independent domain semanticsで**そのpromptの実際の正解であること**まで検証してからprivate `ValidatedAnswerSchema` / `CanonicalAnswer`としてaggregateへ格納する。Addition / Arithmetic / ColumnArithmeticはexact arithmetic、Linear / Simultaneous / Quadraticは方程式のexact solution semantics、Liar Puzzleはstatement truthによる唯一解、Mini Sudokuはgivensを保存する唯一の合法盤面をauthorityとする。Mini Sudokuでは同一`DigitGridSpec`からtuple lengthとdigit domainも検証する。Liar Puzzleのcanonical answerはperson indexの昇順tupleで、各indexがpromptの`people_count`内かつ空/all-liarでないことまでcontract boundaryで検証する。筆算のworked solutionはprompt operand/operatorとcanonical answerから一意に再構成できる場合だけaggregateへ格納されるため、arbitrary divisor/product/offset等の内部値をsafe generator APIから注入できない。native Rust / WASM grading境界から受け取るschemaも同じstructural/canonical-answer validationを通し、不正schemaは`GradeError`として拒否する。`parse_mathlive_answer`も`AnswerInputInterface`の構造条件を最初に検証する。

JavaScriptへ`number`として投影するProblemPrompt / worked-solution内の整数は、`Problem::generated`のaggregate invariantとして`Number.MAX_SAFE_INTEGER`相当の範囲内であることも検証する。AnswerNode / AnswerSchema / BigNum等、64-bit exactnessをwire上で保持すべき値はcanonical decimal stringを使う。これによりRust `i64`を無条件にJavaScript `number`へ落とす経路は作らない。

Generator callbackは`Result<Option<Problem>, GenerationError>`で、`None`は数学的candidate rejection、`Err`はgenerator/aggregate contract failureとして意味を分離する。`ProblemInvariantError` / `WorksheetInvariantError`はそれぞれ`invalid_generated_problem` / `invalid_generated_worksheet`へ変換し、`AttemptLimit`へ偽装しない。WASM/Web境界も`invalid_sampling_strategy` / `invalid_registry`を含むconfiguration error codeを独立したkindのまま保持する。

通常解法は`EffortModel::Operations(OperationPlan)`、真正のtheme固有式は`EffortModel::ThemeSpecific`であり、plan/vector/scalarを独立SoTとして同居させない。`OperationPlan`は現在productが実際に消費するprimitive operation列だけを保持し、未使用のdependency DAGは持たない。Web wire上の`operation_plan` / `operation_vector` / `theme_specific_effort` / `effort`はこの1つのmodelから導出する。schema v7のoperation vectorは32次元で、pre-releaseでは旧wire次元へのprojectionを持たない。

固定domainも生のwire primitiveをcore invariantに使わない。Mini Sudokuのgivensは16cell固定かつcanonical `MINI_SUDOKU_GRID_SPEC`のdigit domainを保証する`MiniSudokuGrid`、liar puzzleのperson/countは`PersonIndex` / `PeopleCount` / `LiarCount`で表す。Mini Sudoku solverのcandidate digit iterationも同じ`MINI_SUDOKU_GRID_SPEC.min_digit()..=max_digit()`から導出し、1..=4を別SoTとして手入力しない。`ProblemSetIdentity`は`new` / `FromStr` / custom `Deserialize`のすべてが同一validation pathを通り、schema・seed・nonzero theme/revisionのinvariantを共有する。`GradeResult`は`status`だけをcorrectnessのSoTとして保持し、`is_correct`はmethod/wire projectionで導出する。

operation vectorの現行固定長はRust `WebContract.operation_kind_count`からWebへ同期する。basis追加はwire schema変更として扱うが、pre-releaseでは旧schema shapeをproduction codeへ保持しない。出力が変わる場合は必要に応じてgenerator revisionも更新する。数学値は整数またはAnswerNodeの正確な十進表現で保持し、binary floatは使用しない。JSON/WASM境界のAnswerNode integer/coefficientとanswer-schema bounds（`i64`）、BigNum magnitude（`u64`）はcanonical decimal stringとし、JavaScriptの安全整数範囲を超える18桁値も可逆にする。`effort`とoperation vector quantityだけは評価結果なので`f64`を許可する。

`input_interface`の例:

```json
{"type":"simple_numeric","allow_decimal":false,"allow_negative":false}
```

構造化テーマは`{"type":"structured_math","allowed_structures":["fraction","root"]}`のように
許可する構造を返す。WebはこのProblemから選択したinterfaceを`parse_mathlive_answer`とgrading境界へ渡し、
`answer_schema`から入力capabilityを推測しない。MathLive parse結果とgradeのexpected/actualは、このinterfaceの
capability projectionを満たす必要がある。`nan_error`だけはboundedなraw-text recovery sentinelとして
保持できるが、digits-only `simple_numeric`から小数・負数・構造のtyped nodeを回復経路で生成することはできない。


## 算術式generator

`ProblemPrompt::Arithmetic`は整数/rational literalと`+,-,×,÷`のbinary nodeからなるexactな`ArithmeticExpression`を保持する。一桁引き算、二桁加算、九九、負の数(1)/(2)、分数加算/減算/乗算はこのpromptを共有し、Web/PDFは同じtyped ASTから式を描画する。

- 一桁引き算: `a-b=c`、`1<=a<=18`、`1<=b,c<=9`。
- 二桁加算: `a+b=c`、`10<=a,b<=99`。
- 九九: `1<=a,b<=9`。effortは通常operation planではなく `EffortModel::ThemeSpecific(log10(c))` としてtheme側が所有する。`BigNum` primitiveを特殊式の代用には使わない。
- 負の数(1): 2〜4整数項、演算子は加減のみ。少なくとも1つ負整数を含む。
- 負の数(2): 2〜4整数leafの四則演算AST。0除算を拒否し、最終値が整数になる候補だけを採用する。
- 通常分数operand domainは、既約後に非整数となる正の分数で `numerator + denominator <= 15` を満たすもの（実装上は分母2〜14）を重複排除した集合である。加算/乗算は可換な鏡像をdomain index上で重複させない。引き算は正の結果だけを採用する。
- 分数加算はoperand domainより広い正の非整数answerを許可し、`numerator <= 65`, `denominator <= 72` とする。引き算・掛け算は結果もoperand domain内に閉じる。通常の分数割り算は左右とも分数operandとし、結果はoperand domainまたは1〜9の整数に制限する。分数×整数/÷整数は別themeとして独立domainを持つ。
- `分数総まとめ(仮分数)` は加減乗除4layerを持ち、掛け算・割り算では1〜9の整数operandも含める。通常単元は帯分数presentation、総まとめは仮分数presentationを維持する。

小学生registrationにはtyped `CurriculumSafetyPolicy::NonNegativeOnly` を明示し、共通generator境界でfail-closed制約を適用する。prompt literal/係数、canonical answerに負数があるcandidate、または`allow_negative`/`negative`/`plus_minus`入力capabilityを持つcandidateは採用しない。表示labelや`curriculum_path`文字列から小学生かどうかを推測しない。


## 筆算generator

筆算themeは通常の横式`arithmetic` promptとはpresentationが異なるため、Rust DTOに専用promptを持つ。

```text
ProblemPrompt::ColumnArithmetic {
  operator: Add | Subtract | Multiply | Divide,
  left: Integer | ExactDecimal,
  right: Integer | ExactDecimal,
}
```

数学的source of truthはoperand/operator/canonical answerを持つRust DTOである。掛け算の部分積と長除法の各「掛ける→引く→下ろす」stepはactive rev2でtyped `worked_solution` としてRustが生成し、Web/PDFはそのsemantic dataを同じ縦式componentへ配置する。pixel/CSS座標はWeb側に残し、途中計算をAnswer ASTへ混ぜない。Web gradingは最終答案だけを既存Rust graderへ送る。

Presentation上は、各筆算problemが独立した方眼を持つのではなく、worksheetの問題文より下に1枚のA4-relative page gridを敷く。各problemはshared A4 cell geometryからそのpage gridへlaneの右端/開始行をsnapし、Web/PDF/解答で同じ座標を使う。小数点はgrid cellを消費せず桁境界の黒点として扱う。掛け算の未解答problemはoperand直下の主線1本だけを描き、途中計算はpage gridへ自由記入する。

- 加減算・掛け算の筆算registration: `problem_count=16`, `columns=4`, `rows=4`
- 割り算の筆算registration: `problem_count=12`, `columns=4`, `rows=3`（解答pageの完成した長除法を問題と同じ文字サイズで保持するため）
- 整数加減乗・小数筆算: 既存`Integer` / `Decimal` answer schema
- 整数除法: 常に`AnswerSchema::OrderedPair`、canonical answerは`Tuple([商, 余り])`
- 余り0でもschemaを変えない
- elementary registrationの共通nonnegative validationを通す

`problem_set_id`はnumeric theme ID / 現行generator revision / Seed / difficulty contractで再生成される。ID 25〜37の現行revisionはworked-solutionを含むrevision 2であり、rev1 generatorはpre-release policyに従って保持しない。

## 一次方程式generator

`ProblemPrompt::LinearEquation`は`a,b,c,d`をexactな`RationalCoefficient`として保持し、`ax+b=cx+d`を表す。`themes/equations.rs` の同一family generator APIを(1)/(2)で共有し、違いはtheme registrationとmodeだけに置く。現active revision 8では、**各candidateについてcanonical answerをanswer domainから一様に復元抽出し、その答えに条件づけて式を生成する**。式生成に失敗した場合は答えを引き直さず、その答えのまま式だけを再生成する。

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

難易度は4段階とし、wire値は `1=かんたん`, `2=ふつう`, `3=むずかしい`, `4=ランダム` とする。現行schema v7で意味を固定し、それ以外のschemaはfail closedで拒否する。

原則としてn問に対して`8n`候補をbootstrap poolとして生成する。一次方程式はn=16なので**128候補**である。有限index domainを持つthemeは母集団全体をProblem化せず、Seedに対して一様なunique indexを必要数だけ抽出できる。さらに分数のようにoperand spaceから候補を直接構成できるthemeは、finite candidate vector自体をmaterializeせずrejection samplingする。`分数総まとめ(仮分数)`は4演算layerを1/4ずつ直接生成するため、4layer合計で`8n`候補を使う。

1. candidate slotごとにcanonical answerをanswer domainから独立一様に選び、必要ならその答えを固定したまま式だけを再生成する。
2. 完成candidateについて`OperationPlan`からoperation vectorを導出し、weightsとの内積でeffortを計算する。theme固有scalarは`EffortModel::ThemeSpecific`として別variantに保持する。
3. `かんたん` / `ふつう` / `むずかしい` はpoolをeffort順に並べ、独立に5個引いた一様indexのorder statisticから、それぞれ旧difficulty 1 / 3 / 5相当の最小・中央値・最大を使う。`n+4`問を得た後、effort最小2問・最大2問を外れ値として除く。
4. `ランダム` はeffort sortもorder statisticもtrimも行わない。candidate poolを式の完全一致で集合化して偶発的な重複multiplicityを除き、その**distinct candidate集合から一様に、非復元抽出**でn問を選ぶ。したがってdifficulty由来のeasy/hard biasを一切入れない。
5. 最終n問は、`かんたん` / `ふつう`ではeffort非減少順に並べてworksheet内にも緩やかなdifficulty rampを持たせる。`むずかしい` / `ランダム`では同じRNG streamでFisher–Yates shuffleする。

候補生成、pool再生成、重複式の再選択は共通のattempt budgetとmonotonic deadlineで打ち切る。`max_attempts`をdeterministicなprimary budgetとし、wall-clockは15秒のemergency watchdogとして扱う。WASM/JS clock境界の過剰な呼び出しを避けるため、attempt loop中のwall-clock確認は64attemptごとに行い、phase境界でも必ず確認する。エラーcodeは`generation_timeout`と`generation_attempt_limit`で区別する。

WASM adapterはcore DTOをそのまま`ApiResponse`の`data`へserializeする薄い境界であり、同じschemaを別形式へ再定義しない。

二次方程式などの解集合比較では、canonical Answer AST内部の`PlusMinus`を最大4 branchまでexactに展開する。分数・根号・負号・加減乗除の内部に`±`が埋め込まれていても、浮動小数へ落とさず正規化したAnswer AST集合としてTuple入力と比較する。重複解の意味は保持し、`(2,2)`を単一解`2`へ勝手にdedupしない。

`grade_answer`はnormalizedな`expected`/`actual`、`is_correct`、statusに加え、表記上の注意を`warnings`配列で返す。識別子は`fraction_not_reduced`、`integer_form_required`、`redundant_negative`、`redundant_plus_minus`、`redundant_decimal`、`duplicate_solution`、`solution_list_required`、`fraction_form_required`、`mixed_fraction_form_required`で、表示文言とは分離する。Webでは「約分しましょう」「整数でこたえましょう」「分数でこたえましょう／帯分数でこたえましょう」「最後まで計算しましょう」の採点カテゴリへまとめ、詳細設定→採点設定でカテゴリごとに○/×を選ぶ。
Web grade DTOのstatusは`correct`、`incorrect`、`unanswered`だけを許可する。actualが`empty`なら
`is_correct=false`かつ`unanswered`、actualがnon-emptyなら`is_correct`に応じて`correct`または`incorrect`
でなければfail closedする。

`nan_error`はcoreが解析できなかったbounded raw textを保持するvariantである。DTO検証、表示、編集、採点結果は
このtextをそのまま保持し、JavaScriptの数値、空欄、暗黙の不正解値へ変換しない。`nan_error`を含む回答は常に不正解で、採点側が明示的に不正解を返す場合もWebのgrade itemにはraw textを表示する。
