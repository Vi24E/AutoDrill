# Problem schema v7

上位原則は[`../principles.md`](../principles.md)を参照する。未解決の既知問題は [GitHub Issues](https://github.com/Vi24E/AutoDrill/issues) で追跡する。

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

`timeout_ms`と`max_attempts`は省略可能で、既定値は15,000msと10,000試行である。問題数はrequestやIDへ入れず、revision付きgenerator registryから得る。

現行theme登録の唯一のauthorityはRust `drill-core` のregistryと、そこから `web_contract()` が機械的に投影するcontractである。この文書ではnumeric theme ID / revision / curriculum path / layoutの一覧を手書きで複製しない。Webは`apps/web/src/generated/drill-core-contract.ts`、QAは`apps/qa/generated/drill-core-contract.json`をそのcontractから生成し、`pnpm contract:check`でtracked projectionとの一致を検証する。themeの追加・分割・移動ではRust registrationを更新し、consumerはgenerated metadataを読む。表示label・slug・numeric IDから数学的意味やcurriculum metadataを逆算しない。

## Problem-set identity

ID形式は次の可逆なASCII表現である。

```text
{schema_version}-{numeric_theme_id}-{generator_revision}-{seed}-{difficulty}
```

現行例は`7-1-5-Ab3Z-2`。identity内部のraw `seed` componentは1〜16文字で、`1-9`、`a-z`、`A-Z`から`I`、`l`、`O`を除いた集合だけを許可する。`-`を許可しないためfull IDの分割は曖昧にならない。product UIで表示・入力・共有する『Seed』はこの**full problem-set ID**であり、raw `seed` component単独ではない。現行revisionの`ProblemSetIdentity`は同じ現行generatorで決定的に再生成できる。identityのfieldはcoreではprivateで、constructor / `FromStr` / custom `Deserialize`のすべてがschema・raw seed・nonzero theme/revisionを検証する。Webはfull IDをopaque stringとしてWASMへ渡し、theme/difficulty/revisionを文字列から再解析しない。

Worksheetはcoreではdecoded `identity`、registry由来のskill/curriculum/layout、registry所定個数のProblemだけを保持するvalidated aggregateである。`problem_set_id`はcore内でidentityから純粋に導出し、Webの共有Seed・URL・footer consumerが使うためWorksheet wireへread-only projectionとして載せる。Worksheet wireは`schema_version`、`identity`、derived `problem_set_id`、`layout`、`problems`を公開し、skill/curriculumはgenerated theme contract側をauthorityとする。`Worksheet::generated`はidentityとregistrationのschema/theme/revision、problem数、各Problemのschema/themeを一度に検証する。coreの`Problem`はversion、通し番号、numeric theme ID、型付きprompt、validated answer schema、validated canonical answer、そこからregistration contractで導出した`input_interface`、promptとcanonical answerからcore自身が導出したopaque `worked_solution`、そして排他的な`EffortModel`を持つ。generatorはworked-solution fieldを直接組み立てて`Problem`へ注入できない。Column multiplication / long divisionのpartial/step dataはcore domainのprivate valueであり、Webへ必要なshapeだけを`wire.rs`のDTOへ投影する。
Webの再現URLはstatic exportでも任意のproblem-set IDを受け取れるよう、現在のrouteに`?seed=<percent-encoded problem_set_id>`を付ける。ページload時はこのquery valueをそのままWASMの`generate_problem_set`へ渡し、Rustがparse・schema/theme/revision/seed/difficulty validationとregenerationを行う。成功後はWorksheetのcanonical `problem_set_id`をSeed欄・URL・print/footerへ投影し、Userが単元またはdifficultyを手動変更した時点で古いreplay queryを除去する。pre-releaseでは旧schema/旧generator revisionをproduction codeへ保持しないため、それらのIDは対応error codeでfail closedする。


`Problem::generated`は単にprompt/schemaのkindだけを比較しない。`AnswerSchema`の構造条件（整数範囲の`min <= max`、有理数の正の最大分母、非空tuple length等）を検証し、canonical answerがその具体的schemaのrange/shapeを満たした後、`semantics.rs`のgenerator-independent domain semanticsで**そのpromptの実際の正解であること**まで検証してからprivate `ValidatedAnswerSchema` / `CanonicalAnswer`としてaggregateへ格納する。Addition / Arithmetic / ColumnArithmeticはexact arithmetic、Linear / Simultaneous / Quadraticは方程式のexact solution semantics、LinearExpressionはexact affine-expression semantics、Liar Puzzleはstatement truthによる唯一解、Mini Sudokuはgivensを保存する唯一の合法盤面をauthorityとする。Mini Sudokuでは同一`DigitGridSpec`からtuple lengthとdigit domainも検証する。Liar Puzzleのcanonical answerはperson indexの昇順tupleで、各indexがpromptの`people_count`内かつ空/all-liarでないことまでcontract boundaryで検証する。筆算のworked solutionはprompt operand/operatorとcanonical answerから一意に再構成できる場合だけaggregateへ格納されるため、arbitrary divisor/product/offset等の内部値をsafe generator APIから注入できない。native Rust / WASM grading境界から受け取るschemaも同じstructural/canonical-answer validationを通し、不正schemaは`GradeError`として拒否する。`parse_mathlive_answer`も`AnswerInputInterface`の構造条件を最初に検証する。

JavaScriptへ`number`として投影するProblemPrompt / worked-solution内の整数は、`Problem::generated`のaggregate invariantとして`Number.MAX_SAFE_INTEGER`相当の範囲内であることも検証する。AnswerNodeのinteger/coefficientとAnswerSchemaの`i64` boundsなど、64-bit exactnessをwire上で保持すべき値はcanonical decimal stringを使う。`BigNum`はeffort内部primitiveであり現行Problem wireへ公開しない。これによりRust `i64`を無条件にJavaScript `number`へ落とす経路は作らない。

Generator callbackは`Result<Option<Problem>, GenerationError>`で、`None`は数学的candidate rejection、`Err`はgenerator/aggregate contract failureとして意味を分離する。`ProblemInvariantError` / `WorksheetInvariantError`はそれぞれ`invalid_generated_problem` / `invalid_generated_worksheet`へ変換し、`AttemptLimit`へ偽装しない。WASM/Web境界も`invalid_sampling_strategy` / `invalid_registry`を含むconfiguration error codeを独立したkindのまま保持する。

通常解法は`EffortModel::Operations(OperationPlan)`、真正のtheme固有式は`EffortModel::ThemeSpecific`であり、plan/vector/scalarを独立SoTとして同居させない。`OperationPlan`は現在productが実際に消費するprimitive operation列だけを保持し、未使用のdependency DAGは持たない。operation vectorとscalar effortはこの1つのmodelからRust内部で導出する。**現行schema v7のProblem wireは`operation_plan` / `operation_vector` / `theme_specific_effort` / `effort`を持たない。** これらはgenerator / difficulty selection / Rust testの内部状態であり、cross-language consumerが存在するまで公開しない。

固定domainも生のwire primitiveをcore invariantに使わない。Mini Sudokuのgivensは16cell固定かつcanonical `MINI_SUDOKU_GRID_SPEC`のdigit domainを保証する`MiniSudokuGrid`、liar puzzleのperson/countは`PersonIndex` / `PeopleCount` / `LiarCount`で表す。Mini Sudoku solverのcandidate digit iterationも同じ`MINI_SUDOKU_GRID_SPEC.min_digit()..=max_digit()`から導出し、1..=4を別SoTとして手入力しない。`ProblemSetIdentity`は`new` / `FromStr` / custom `Deserialize`のすべてが同一validation pathを通り、schema・seed・nonzero theme/revisionのinvariantを共有する。`GradeResult`は`status`だけをcorrectnessのSoTとして保持し、`is_correct`はmethod/wire projectionで導出する。

operation vectorの現行固定長はRust effort module内部で`OperationKind`のbasisと一緒に管理し、Webへdimensionを同期しない。basis変更で生成結果が変わる場合は必要に応じてgenerator revisionを更新するが、内部basis変更だけを理由にWeb schemaを増やさない。数学値は整数またはAnswerNodeの正確な十進表現で保持し、binary floatは使用しない。JSON/WASM境界のAnswerNode integer/coefficientとanswer-schema bounds（`i64`）はcanonical decimal stringとし、JavaScriptの安全整数範囲を超える18桁値も可逆にする。effort内部のweighted score / operation quantityは評価値なのでRust内部では`f64`を許可するが、現行Problem wireには含めない。

`input_interface`の例:

```json
{"type":"simple_numeric","allow_decimal":false,"allow_negative":false}
```

構造化テーマは`{"type":"structured_math","allowed_structures":["fraction","root"]}`のように
許可する構造を返す。WebはこのProblemから選択したinterfaceを`parse_mathlive_answer`とgrading境界へ渡し、
`answer_schema`から入力capabilityを推測しない。MathLive parse結果とgradeのexpected/actualは、このinterfaceの
capability projectionを満たす必要がある。`nan_error`だけはboundedなraw-text recovery sentinelとして
保持できるが、digits-only `simple_numeric`から小数・負数・構造のtyped nodeを回復経路で生成することはできない。

筆算problemはこれに加えてRustが解決済みの`column_input` metadataをwireへ載せる。各`single` / `quotient` / `remainder` partは`least_significant_first` / `natural_division_flow` / `big_endian`のtyped input orderと、`none` / `fixed(scale)` / `editable`のdecimal-point policyを持つ。Theme registrationのpresentation policyが教材上の方針を所有し、`fixed_canonical_scale`だけはProblem生成時にRustがcanonical answerのscaleを具体値へ解決する。Webはoperator、theme ID、canonical answerから入力方向や固定小数点位置を再推測せず、このProblem metadataをinteractionへ投影する。


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

`problem_set_id`はnumeric theme ID / 現行generator revision / Seed / difficulty contractで再生成される。筆算themeの現行revisionはRust registryが所有し、worked-solutionやgenerator semanticsが変わる場合はthemeごとにrevisionを更新する。pre-release policyでは旧revision generatorを互換経路として保持しない。


## 一次式generator

`ProblemPrompt::LinearExpression { expression: LinearExpression }` は、式そのものを簡単にして答える教材surfaceである。方程式のsolution semanticsを持つ`AnswerSchema::Algebraic`とは分離し、現行は`AnswerSchema::LinearExpression { variable: X, require_collected_form: true }`を使う。Web/WASMへは共有`LinearExpression` ASTをそのまま投影し、Web/printは同じformatterで描画する。

中1「一次式の整理・加減」(ID 75, rev1) は16問、2列×8行で、次の3 layerを同一theme内に持つ。各layerはequal weightかつ最低2問を保証する。

- 同類項をまとめる4項の一次式
- `(ax+b)+(cx+d)` の加法
- `(ax+b)-(cx+d)` の減法

係数と定数は現行scopeでは整数とし、結果も見た目上一次式を必ず練習できるようvariable coefficientとconstantが0になるcandidateを除外する。Rustはpromptをexactな`ax+b`へ正規化し、MathLiveからparseしたAnswerNodeも`x`を含むexact affine formへ評価して同値性を判定する。`2x+3x+3`のように数学的には同値だがcollected formでない答案はcorrectnessを保ったまま`expression_not_simplified` warningを返す。一般多項式、展開、因数分解はこのrevisionのscope外であり、#153の後続拡張で扱う。

## 一次方程式generator

`ProblemPrompt::LinearEquation` は表示surfaceを保持する `left/right: LinearExpression` を持つ。`LinearExpression` は `variable / constant / add / subtract / scale / group` の共有一次式ASTで、variableはtyped `LinearVariable::X | Y` を明示する。`group` は数学演算を追加せず、`(ax+b)` のような明示的な括弧surfaceだけを保持し、semantic normalization / effortでは内側の式へ透過的に委譲する。`LinearScalar` は整数・既約分数・有限小数を区別して保持するため、`1/2(x-3)` と `0.5(x-3)` は数学的には同じ係数でも同じwire表現へ潰さない。Rustの `semantics.rs` はASTをexactな affine form `ax+by+c` へ正規化する。通常の一元一次方程式では左右の`y`係数が0であることをsemantic invariantとして要求し、従来どおり`x`だけの方程式としてcanonical answerを検証する。Web/PDFはASTのsurfaceを再帰描画するだけで、変数・係数・括弧の意味を再推論しない。

同一family generatorは **solution domain** と **教材surface** を別のtyped axisとして持つ。answer-conditioned samplingではcanonical answerをdomainから先に選び、その解を満たす式surfaceを生成する。

- `簡単な一次方程式` (ID 69): `x+a=b` と `ax=b` の一段階基本形だけを扱う。両形を同一themeに含め、整数解domainを使う。
- `一次方程式(1)：基本形` (ID 2, rev9): 括弧を持たない整数係数の基本形。整数解domainを使う。
- `一次方程式(2)：括弧・整数係数中心` (ID 3, rev9): 全問題に実際の展開を要する `scale(add/subtract(...))` を含め、surface coefficientは整数に限定する。整数解・分数解を同一themeでsupportする。
- `一次方程式(3)：括弧・分数・小数係数` (ID 70): 全問題に括弧と非整数係数surfaceを含める。分数係数または有限小数係数を用い、sampling support全体では両surfaceを持つ。整数解・分数解を同一themeでsupportする。

括弧の展開は表示上の飾りとして扱わない。`effort.rs` は各 `scale` の分配で必要なexact multiplicationを既存の整数/有理数operation builderで数えた後、正規化された `ax+b=cx+d -> Ax=B -> x=B/A` の標準planへ接続する。difficulty selectionはこの完全なOperationPlanを比較し、d4=`ランダム` の既存sampling semanticsは変更しない。

有理解domainは整数 `-15..=15` に加え、既約形で分母2・`|numerator|<=20`、または分母3..12・`|numerator|<=15` の非整数有理数を含む。同じ数学値はdomain内で1要素へ正規化する。分数係数の整理と、結果として分数解になる場合は同一教材目標であり、「分数係数・整数解」と「分数解」へ分離しない。

一次方程式は全7構造を持つ `structured_math` を返す。`answer_schema` は `簡単` / `(1)` が `integer { min:-15,max:15 }`、`(2)` / `(3)` が `rational { max_abs_numerator:20,max_denominator:12,require_reduced_fraction_form:true }` である。非整数解では未約分の通常分数は `fraction_not_reduced`、数学的に同値な帯分数・有限小数・繁分数などは `fraction_form_required` を返す。Rust coreが数学的同値性とwarningを返し、warningを○/×のどちらとして扱うかはWebの採点設定が決める。

## 二次方程式generator

`ProblemPrompt::QuadraticEquation` は `equation: QuadraticEquationSurface` と `solve_method: QuadraticSolveMethod` を保持する。`QuadraticEquationSurface` の左右は `QuadraticExpression` で、`linear / square / add / subtract / scale` を再帰的に表す。`linear` と `square` の内部には共有 `LinearExpression` を使うため、`a(x+c)^2+b=0`、分数係数、有限小数係数、括弧によるsurface変形をtheme metadataへ依存せず表現できる。Rustの `semantics.rs` がsurfaceをexactな `ax^2+bx+c=0` へ正規化し、`solve_method` がそのsurfaceへ適用可能であることと、canonical answerが表示surface自体を満たすことを `Problem::generated` から独立に検証する。Web/PDF/QAはtyped surfaceを描画するだけで、theme ID・slug・labelから係数や解法を推測しない。

現行教材は4themeである。

- `二次方程式(1)` (ID 14, rev4): `solve_method = square_root`。`ax^2+b=0` の直接平方根型と、最初から平方の形が明示された `a(x+c)^2+b=0` のshifted-square型を同一themeの2 layerとして扱う。一般式を平方完成して解く専用archetypeは持たない。
- `二次方程式(2)` (ID 15, rev5): `solve_method = factoring`。整数解から逆生成したモニック `x^2+bx+c=0` に限定する。共通因数型・一般非モニック因数分解型はこのthemeへ拡張しない。
- `二次方程式(3)` (ID 16, rev4): `solve_method = formula`。整数係数の一般二次方程式を扱い、判別式が非平方数の問題だけでなく、正の平方数となって整数/有理数解へ簡約される問題も正当なsupportとして保持する。平方判別式の出題集中制御はgenerator supportではなく将来のsingularity制御の責任とする。
- `二次方程式(4)` (ID 74): 平方根法・因数分解・解の公式の3 methodに、括弧・分数係数・有限小数係数の3 transformを組み合わせる9 layerの総合theme。各surfaceはexactに既知のcanonical quadraticへ帰着し、見慣れない表面形でも既知の解法を適用する練習を目的とする。

d4=`ランダム` の意味は変更せず、各themeのsemantic support内でrandom selectionを行う。`solve_method` はdifficultyやanswer surfaceからWeb側が推測する値ではなく、Rust generatorが問題ごとに保持する数学的solve strategyである。

## 連立方程式generator

`ProblemPrompt::SimultaneousEquation` は `equations: [LinearEquationSurface; 2]` と `solve_method: SimultaneousSolveMethod` を保持する。`LinearEquationSurface` は左右に上記の共有 `LinearExpression` を持ち、Rustが各surfaceをexactに `ax+by=c` へ正規化する。2本の正規化後の式について行列式 `ae-bd != 0` を `Problem::generated` のgenerator-independent semanticsでも必須として一意解を保証し、表示surfaceそのものへcanonical answerを代入して検証する。`solve_method` はRust-owned strategy metadataとして同じsemantic validation境界を通る。Web/PDFは2本のsurfaceを描画するだけで、theme ID・slug・labelから解法や係数を推測しない。

現行教材は4themeである。

- `連立方程式（加減法）` (ID 19, rev4): 整数解 `(x,y) in [-15,15]^2` から逆生成し、少なくとも一方の変数をそのまま加減消去できる係数構造を持たせる。`solve_method = elimination` を保持する。
- `連立方程式（代入法）` (ID 71): 片方の式を実際に `x=...` または `y=...` の直接代入可能なsurfaceとして生成し、`solve_method = substitution` を保持する。
- `連立方程式（まとめ(1)）` (ID 72): 基本的な整数surfaceのまま加減法・代入法をlayerとして混合し、解法選択そのものを練習する。
- `連立方程式（まとめ(2)）` (ID 73): 加減法・代入法の双方に、括弧展開・分数係数・有限小数係数のsurface変形を組み合わせる。method × transform の6 layerを同一theme内で扱い、各surfaceはexactに既知のcanonical systemへ帰着する。

`まとめ(1)` / `まとめ(2)` はgeneric compatibility themeではなく重要な総合練習であり、通常のequation taxonomyとしてRecommendedにも投影する。`A=B=C` の連鎖等式は現行contractには含めず、仕様決定まではIssueで追跡する。解 `(x,y)` が与えられ式中の未定定数を逆算する型も現行連立計算themeのscope外である。

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

`grade_answer`はnormalizedな`expected`/`actual`、`is_correct`、statusに加え、表記上の注意を`warnings`配列で返す。識別子は`fraction_not_reduced`、`integer_form_required`、`redundant_negative`、`redundant_plus_minus`、`redundant_decimal`、`duplicate_solution`、`solution_list_required`、`fraction_form_required`、`mixed_fraction_form_required`、`expression_not_simplified`で、表示文言とは分離する。Webでは「約分しましょう」「整数でこたえましょう」「分数でこたえましょう／帯分数でこたえましょう」「最後まで計算しましょう」の採点カテゴリへまとめ、詳細設定→採点設定でカテゴリごとに○/×を選ぶ。
Web grade DTOのstatusは`correct`、`incorrect`、`unanswered`だけを許可する。actualが`empty`なら
`is_correct=false`かつ`unanswered`、actualがnon-emptyなら`is_correct`に応じて`correct`または`incorrect`
でなければfail closedする。

`nan_error`はcoreが解析できなかったbounded raw textを保持するvariantである。DTO検証、表示、編集、採点結果は
このtextをそのまま保持し、JavaScriptの数値、空欄、暗黙の不正解値へ変換しない。`nan_error`を含む回答は常に不正解で、採点側が明示的に不正解を返す場合もWebのgrade itemにはraw textを表示する。
