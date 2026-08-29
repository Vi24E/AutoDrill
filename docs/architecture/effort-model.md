# Effort model

上位原則は[`../principles.md`](../principles.md)を参照する。未解決の既知問題は [GitHub Issues](https://github.com/Vi24E/AutoDrill/issues) で追跡する。

Effortは、標準解法を人間が定数時間で実行できるprimitiveへ分解した回数vectorと、その重みの内積である。

`effort = operation_counts · operation_weights`

## 実装境界

再利用可能な計算モデルは`crates/drill-core/src/effort.rs`へ集約する。テーマ側は標準解法を選び、共通builderを呼ぶだけにする。整数四則、分数、LCM/GCD/PF、小数、一次・連立・二次方程式のeffortをテーマごとに再実装してはならない。

九九、九九の逆算、「うそつきだれだ」のように一般primitiveモデルを意図的に外れるものだけは`crates/drill-core/src/themes/<theme>.rs`側に置く。

`Overhead*`は追加costであり、探索・筆算そのものの代用品ではない。`BigNum`は正解の読み書き・保持costだけを表す。

## Primitive

頭の中に1〜9の加算表・乗算表があり、lookupは双方向に使えると仮定する。

| operation | weight | 意味 |
|---|---:|---|
| Identity | 1 | 転記・演算不要の構造変換 |
| Count(n) | 0.2n | n個を列挙・数える |
| Increment / Decrement | 1 | 既存値への±1 |
| BasePlus | 3 | 加算表の正方向lookup |
| BaseMinus | 3.1 | 加算表の逆方向lookup。例 `13-5=8` |
| BaseTimes | 3.5 | 乗算表の正方向lookup |
| BaseDivide | 4 | 乗算表の逆方向lookup。例 `56÷7=8` |
| BaseFractionCancel | 1 | `k/n × n -> k` 型の構造消去 |
| FractionSelfDivision | 1 | `x/x -> 1` の既知同一量による除算 |
| BaseRoot | 3 | 一桁の完全平方根lookup |
| Compare | 1 | 大小・等値判定 |
| Reciprocal | 1 | 逆数取得 |
| BigNum(n) | log10(n) | 正解整数成分の読み書き・保持 |
| TimeTen(n) | 1+0.2n | 小数点を10^nだけ移動 |

その他の既存primitiveは`Transposition`, `OverheadPF/GCD/LCM/Negative/Carry*`, `OverheadLinear/EqSystem/Factor*/Quadratic`である。既定weightは`OperationWeights::default()`を正とする。

OperationVectorの現行Rust内部basisは29次元で、28=`FractionSelfDivision`である。これはgenerator / difficulty計算のcurrent-only internal contractであり、Web wire schemaではない。pre-releaseでは旧basis用compatibility表現を保持しない。 `TimeTen(n)`はvector上で`TimeTen × 1 + Count × n`へ分解し、固定作業と移動桁数を別weightで表す。既定weightでは`1 + 0.2n`となり、特定weight値を逆算したmagic offsetをoperation countへ埋め込まない。

## 共通整数builder

### 加算

右から列ごとに処理する。両方に非零digitがあれば`BasePlus`。片方しかなければ転記なので`Identity`。前列からcarryが既存digitへ入る場合だけ`Increment`を数える。

carryが発生するたび`OverheadCarryPlus`を加える。最上位へ新しく出たcarryは足す相手がないので`Identity`であり`Increment`ではない。

例: `97+86`は`BasePlus×2 + Increment + OverheadCarryPlus×2 + Identity`。

### 減算

加算表の逆lookupで直接決まる形は`BaseMinus`一回とする。したがって`13-5`も一回。筆算が必要なら各列を同じlookupへ分解し、borrowごとに`Decrement + OverheadCarryMinus`を加える。0を引くだけの列は`Identity`。

### 乗算

各非零digit pairを`BaseTimes`で引く。carryが出れば`OverheadCarryMult`。carryを次の実在する積へ足す場合だけ`Increment`または加算builderを使い、最後に残るcarryは`Identity`。部分積同士の和は共通加算builderを使う。

### 除算

`divisor × quotient = dividend`が9×9乗算表の逆lookupで直接得られるなら`BaseDivide`一回。例: `56÷7=8`。

それ以外の商digit探索は、余りの有無を含め九九上の二分探索相当として`BaseTimes×3`とする。積との差が非零なら共通減算builderで余りを求める。桁下ろしは`Identity`、大小判定は`Compare`。

九九の逆算テーマだけは例外で、割り切れる問題でも常に`BaseTimes×3`で探索する。

## 筆算theme

筆算theme専用の「桁数×定数」effortは持たない。表示が縦式であることと数学的標準解法は分離し、既存の共通整数/小数builderをそのまま使う。

- 足し算: `BasePlus` + carry時の`Increment` / `OverheadCarryPlus`
- 引き算: `BaseMinus` + borrow時の`Decrement` / `OverheadCarryMinus`
- 掛け算: `BaseTimes` + carry + 部分積の共通整数加算
- 割り算: 商digit探索の`BaseTimes`/`Compare`、積との差の共通減算、桁下ろし`Identity`
- 小数加減: decimal alignment後に共通整数加減
- 小数乗算: 共通整数乗算 + 最終小数点位置`TimeTen`
- 小数除算: 除数整数化の`TimeTen` + 共通長除法

商と余りを答える整数除法も、計算本体は同じ`divide_or_identity_operations`を使う。OrderedPairとしての最終答案に必要な通常のread/write costだけを追加し、余りあり/なしを別difficulty heuristicにはしない。

小数加減の筆算だけはAddition/Subtractionを教材coverage上のlayerとして8問ずつ確保する。carry/borrow、余りあり/なしはlayerではなく、実際に発生したprimitive operation数がscalar effortへ反映される。

## PF / GCD / LCM / 平方根

### 素因数分解

`OverheadPF`を追加する。nが9×9乗算表上の積として認識できる場合、因数分解部分はoverheadだけで済む。それ以外は`2,3,5,7,...`の素数を試し、必要な範囲まで試し割りする。試し割り自体は共通除算builderを使う。

### GCD

`OverheadGCD`を追加し、両数を上記PFで分解して共通素因数を`Compare`で突き合わせる。旧`OverheadGcdDivisible` shortcutは削除する。

### LCM

`OverheadLCM`を追加し、2本の倍数列を最初の一致まで列挙する。倍率計算は共通乗算builder、列比較は`Compare`。

### 平方根の簡約

一桁完全平方なら`BaseRoot`。それ以外は`OverheadFactorPerfectSquare`を加え、`2^2, 3^2, 5^2, 7^2, ...`でradicandを試し割りして平方因子を探す。外へ出した因子が複数なら共通乗算builderで掛け合わせる。一般PFへ置き換えない。


## Difficulty samplingとlayer

`effort = operation_counts · operation_weights`というscalar difficultyモデルは維持する。operation vectorのcosine similarity、PCA、farthest-point等をdifficulty samplerへ導入しない。

教材上同一worksheet内で複数アーキタイプのcoverageを保証する必要があるテーマだけ、generator内部metadataとしてlayerを宣言できる。quotaは各layerのminimumを確保してからweight比例のlargest-remainder方式で残数を配分し、各layer内で既存のscalar effort samplerを独立に適用する。randomもlayer quotaを守った上でlayer内random samplingする。layer情報は公開Worksheet schemaへ含めない。

`SamplingStrategy`は自由なenum literalではなくvalidated constructorから作る。answer-conditioned strategyはnon-empty answer domain、layered strategyはnon-empty layer setと`minimum`合計がworksheet problem count以下であること、constructive layered strategyはさらにnonzero bootstrap multiplierをconstruction時に保証する。themeのclassifierが返す生の`usize`はframework内でbounded `LayerIndex`へ変換し、範囲外は`SamplingError`として明示的に失敗させる。answer-conditioned callbackの返却answerとrequested answer、constructive-layered callbackの返却layerとrequested layerもsampling直後に照合する。capability contract違反を`next_bounded(0)` panic・silent retry・`AttemptLimit`へ化かさない。

現行のlayered themeは次の3つ。

- 小数の足し算と引き算: Addition / Subtraction（20問で10/10）
- 分数総まとめ(仮分数): Addition / Subtraction / Multiplication / Division（16問で4/4/4/4）
- 二次方程式(2): DifferenceOfSquares / PerfectSquare / General（16問で2/2/12、20問なら2/2/16）

`うそつきだれだ`はlayered themeではない。3人/4人比率がdifficultyで変化すること自体が想定されたscalar difficulty behaviorである。

## 分数・小数

分数加減はLCM→分子scale→共通加減→必要なGCD約分を使う。分数乗算は共通乗算→GCD約分。除算は`Reciprocal`後に同じ乗算モデル。`k/n × n`は`BaseFractionCancel`を使う。

小数は分数へ変換してeffortを計算しない。小数点位置合わせ・移動だけdecimal固有処理とし、桁の加減乗除は上記共通整数builderを再利用する。

## 方程式

一次方程式は`ax+b=cx+d -> Ax=B -> x=B/A`。係数整理・除算は共通整数/有理数builderへ委譲する。

連立方程式はx消去・y消去の完全な加減法operation planを両方作り、weight適用後の小さい方を採用する。内部四則はすべて共通builder。

二次方程式(1)はformと係数を見て移項・除算・平方根modelを組み合わせる。

二次方程式(2)の一般形`x^2+bx+c=0`は、`c`をPFし、そこから重複しない因数対を列挙し、各`p+q`を共通加算で計算して`b`と`Compare`する。一致した時点で探索を終了する。平方差・完全平方は専用strategy。

二次方程式(3)は分母払い後、`b^2`, `ac`, `4ac`, `D`, `sqrt(D)`, `2a`, `1/(2a)`、根号/分数簡約を共通builderで構成する。

## 負数

`OverheadNegative`は負数operandを含む演算ごとに追加する。単なる負号表示には付けない。正のa,bに対する構造的`a+(-b)`だけは`a-b`への直接読み替えとして例外扱いする。

## 明示的なテーマ例外

coreではeffort evidenceを`EffortModel`というsum typeで一元化する。

- `EffortModel::Operations`: `OperationPlan`（標準解法を構成するprimitive operationの順序付き列）だけをevidenceとして保持する。operation vectorとweighted scoreはplanから都度導出し、cached scoreを保存しない。
- `EffortModel::ThemeSpecific`: 標準解法primitiveへ意味を保ったまま分解できない真正のtheme固有scalarを、finite/nonnegativeを保証したvalueとして保持する。

現行productには解法primitive間のprerequisite edgeを消費する機能がないため、coreにDAG/`depends_on` metadataを持たせない。将来、解説表示等で依存関係そのものがproduct requirementになった時点で、そのconsumerとinvariantを伴う型として再導入する。

従って通常modelと特殊modelを同時に格納する状態、planとvectorだけが食い違う状態、stored scoreだけが古い状態はcore型として作らない。`EffortModel`、`OperationPlan`、`OperationVector`、theme固有effort値はgenerator / difficulty selection / Rust内部testのための内部表現であり、**現行のProblem Web wireには公開しない**。具体的なcross-language consumerが存在しないdiagnostic stateを「将来使うかもしれない」という理由だけでwire contractへ追加しない。

現行のtheme固有経路:

- 九九: `log10(answer)`。旧実装の`BigNum`借用は廃止した。
- すうじはひとりぼっち: `(非自明な空マス数) + 0.3 × (自明な空マス数)`。自明な空マスは、行・列・2×2 blockのいずれかに既知値が3つある空マス。
- うそつきだれだ: statement formula length。`Operation::Identity`等の無関係primitiveへ数値を偽装しない。

九九の逆算は`BaseTimes×3 + BigNum(dividend)`として通常operation modelを使う。特殊式の計算自体は`effort.rs`へtheme名の分岐を入れず、`crates/drill-core/src/themes/<theme>.rs`側が所有する。

## Versioning

operation basisと`OperationVector`の次元はRust effort実装内部のcurrent-only contractとして一元管理する。**basisの追加・削除それ自体は現行Web wire schema変更ではない**。現行Problem wireはeffort diagnosticsをserializeせず、`WebContract`にもvector dimensionを投影しない。effort semanticsの変更によって生成結果やdifficulty selectionが変わる場合はgenerator revisionを必要に応じて破壊的に更新する。将来、解説・診断表示など具体的なcross-language consumerが生じた場合だけ、そのconsumerに必要な最小DTOを新しいwire contractとして設計し、generated typeとruntime Serde shapeを同期する。旧revisionや未使用compatibility projectionはproduction codeへ保持しない。
