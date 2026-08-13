# Effort model

Effortは標準解法graph、denseなoperation vector、重みを分離する。同じgraph/vectorを保持したまま重みだけを差し替えて再評価できる。

`SolutionGraph.steps`の各nodeはtyped `Operation`とdependency IDを持つ。Vector化ではgraph nodeを一度だけ数え、複数nodeから参照されるdependencyを再帰的に重複加算しない。一桁の足し算graphは答えの正確な`BigNum(left + right)`と`BasePlus`を持ち、繰り上がり時には`Increment`と`OverheadCarryPlus`を別nodeとして追加する。被演算子を個別のBigNumとして二重計上しない。

算術追加テーマも同じoperationを再利用する。一桁引き算は`BaseMinus`と必要な借り`Decrement + OverheadCarryMinus`、二桁加算は各桁の`BasePlus`と1の位/10の位それぞれの繰り上がりを記録する。負の数(1)/(2)と分数加算/減算/乗算はtyped arithmetic ASTをbottom-upに評価し、各binary operationの標準cost、必要な`OverheadNegative`、通分`OverheadLCM`、約分`OverheadGCD`を加え、最後に正解ASTの`BigNum`を加える。**九九だけは製品仕様上の例外で、`BaseTimes`を記録せず正解`c`の`BigNum(c)`だけを持つため、既定重みではeffort=`log10(c)`となる。**

分数の掛け算では分子同士・分母同士を別operationとして数える。通常の一桁積は`BaseTimes`だが、一方が`1`（符号を無視して`±1`）なら転記に近い操作として`Identity`を使う。 また、GCDを求める2数`a,b`について`a|b`または`b|a`なら、小さい方がそのままGCDだと認識できるため`OverheadGcd`の通常コストの1/4だけを課す。たとえば`1/2 × 1/2`の積部分は`Identity + BaseTimes`、`2/3 × 3/4`は`BaseTimes + BaseTimes`であり、後者には必要なら`OverheadGCD`も加わる。これにより旧実装の一律`BaseTimes × 2`で失われていた難易度差をoperation vectorへ残す。

一次方程式`ax+b=cx+d`はcurriculum.mdの標準解法に従い、まず`A=a-c`、`B=d-b`として`Ax=B`へ整理し、最後に`x=B/A`とする。graphは`OverheadLinear`を1回、実際に辺をまたぐ非零項ごとに`Transposition`、係数・定数をまとめるexactな加減算、最後の`BaseDivide`を記録する。分母が異なる有理係数の整理には`OverheadLCM`と必要な乗算、非自明な分数簡約には`OverheadGCD`を加える。`A=0`のcandidateはgeneratorで除算前に棄却するためeffort graphへ入らない。正解の整数成分は既存の`BigNum`規則で加算する。

二次方程式の平方根計算は`BaseRoot`を基礎操作とする。`√16=4`のような完全平方数は`BaseRoot`だけでよいが、`√12=2√3`のように平方因子を取り出す簡約では追加で`OverheadFactorPerfectSquare`を課す。したがって既定重みでも前者のeffortは後者より小さい。解の公式でも根号簡約が必要な場合は同じoverheadを使う。

## Dense vector order and base weights

`OperationVector`と`OperationWeights`は次の固定27成分を同じ順で持つ。未使用成分も0として残る。

| index | operation | base weight |
|---:|---|---:|
| 0 | Identity | 1 |
| 1 | Count | 0.2 |
| 2 | Increment | 1 |
| 3 | Decrement | 1 |
| 4 | BasePlus | 3 |
| 5 | BaseMinus | 3.1 |
| 6 | BaseTimes | 3.5 |
| 7 | BaseDivide | 4 |
| 8 | BigNum | 1 |
| 9 | Round | 1 |
| 10 | TimeTen | 0.2 |
| 11 | OverheadPF | 2 |
| 12 | OverheadGCD | 4 |
| 13 | OverheadLCM | 4 |
| 14 | OverheadNegative | 1.5 |
| 15 | OverheadCarryPlus | 0.5 |
| 16 | OverheadCarryMinus | 0.5 |
| 17 | OverheadCarryMult | 0.5 |
| 18 | Transposition | 2 |
| 19 | OverheadLinear | 2 |
| 20 | OverheadDistribution | 2 |
| 21 | OverheadEqSystem | 4 |
| 22 | OverheadFactorPerfectSquare | 3 |
| 23 | OverheadFactorDifferenceOfSquares | 2 |
| 24 | OverheadFactorGeneral | 5 |
| 25 | OverheadQuadratic | 6 |
| 26 | BaseRoot | 3 |

Parameterized operationはvector側に重みが掛かるquantityを蓄積する。Count(n)はn、BigNum(n)は正確な整数magnitudeから計算した安全な`log10(n)`（n=0は0）、TimeTen(n)は`n+5`（`0.2(n+5)=1+0.2n`）、Distribution(n)はnである。数学値そのものは整数/ASTに残り、Floatは最終log10とscalar effortだけに使う。BigNum magnitudeはJSON/WASM境界ではcanonical unsigned decimal stringとして保持する。

## Weight composition

`WeightProfile`はgrade、theme、masteryの3つの倍率layerを持ち、`resolved = base × grade × theme × mastery`として成分ごとに合成する。Alpha 1.2は全layerをidentity 1.0とする。Registryの`operation_weight_overrides`はtheme layerだけを上書きするため、将来のテーマ調整でもgraphやvectorを複製しない。

`OverheadNegative`は負号表示のcostではなく、負のoperandを含む演算ごとに1回加える。唯一の一般形の例外は、正の`a`、`b`に対する構造的な`a + (-b)`で、`a > b`、`a = b`、`a < b`のいずれでも`a - b`への読み替えとして0回とする。順序を区別するため`(-b) + a`は1回、`a - (-b)`も正の加算へ書き換えても1回、その他の負のoperandを含む演算も1回である。単独の`-0.57`は演算ではないため0で、BigNum(57)だけを数える。


## 連立方程式(1)

標準解法は加減法として近似する。`OverheadEqSystem`を1回置き、x消去とy消去のうち係数をそろえるための整数倍率が小さい方を採用する。必要なら式全体の倍率計算、消去の減算、残った変数の除算、代入後の乗算・減算・除算を順にgraphへ記録する。現段階ではgenerator品質確認前の基礎modelであり、後続調整で操作分解を精密化できる。
