# Answer AST v3

上位原則は[`../principles.md`](../principles.md)を参照する。未解決の既知問題は[`../issues.md`](../issues.md)で追跡する。

`AnswerNode`は入力、表示、正規化、採点で共用する正確な型付き構文木である。

| node | exact payload |
|---|---|
| `empty` | なし |
| `integer` | Rust内部は`i64`、JSONではcanonical decimal string |
| `exact_decimal` | Rust内部は十進`coefficient: i64`と`scale: u32`。JSONのcoefficientはcanonical decimal string |
| `nan_error` | 数値へ変換できなかったraw text。表示・編集用で、数値として扱わない |
| `fraction` | numerator、denominator |
| `mixed_fraction` | whole、numerator、denominator |
| `root` | radicand、任意のindex |
| `negative` | child |
| `plus_minus` | child |
| `tuple` | child配列 |
| `variable` | 名前 |

小数はbinary floating-pointへ変換しない。例えば`0.3`は`coefficient = 3, scale = 1`、`0.57`は`57, 2`である。JSON/WASM境界ではi64 payloadを`"3"`のようなcanonical decimal stringにし、18桁整数もJavaScriptの`number`へ変換しない。BigNumもこのcoefficientや整数nodeから直接導出し、表示floatから桁列を復元しない。分数`41/57`は整数child 41と57を別々のBigNum sourceにする。

`AnswerRepresentation`はユーザーが入力・表示したtreeと採点用のnormalized treeを別fieldで保持する。`normalize_answer`は新しいcanonical treeを返し、呼び出し側のdisplay treeを書き換えない。整数、有限小数、分数、帯分数、negativeから構成できる正確な数値は、Floatを使わず符号を分子へ集約した既約分数（分母1ならinteger）へ統一する。従って`2/4`、`0.5`、`1/2`は同じnormalized treeになり、`4.0`はinteger `4`になる。

## Structured editor

入力許可は`Problem.input_interface`が所有し、`answer_schema`とは直交する。`simple_numeric`は桁入力を
基本にし、`allow_decimal`と`allow_negative`だけを追加許可する。`structured_math`は
`allowed_structures`に列挙された構造だけを画面下部パレットへ投影する。一桁の足し算はdigits-only
keypadで、構造template、小数点、負数キーを表示しない。

Webの編集・caret・placeholder移動・fraction/root layoutはMathLiveが担当する。MathLiveの各`input`
で得たLaTeXは`parse_mathlive_answer`という明示adapterを通り、Rustの`AnswerNode`へ変換されて初めて
回答stateとなる。従ってMathLiveのmodelやLaTeX自体を採点authorityにはしない。adapterはMathLiveが
正規化して返す`\frac72`や`\sqrt2`のような1-token TeX引数も受理し、許可されない構造は
`input_interface`検証で拒否する。

MathLive worksheetの回答stateはRustが受理した`AnswerNode`そのものとし、採点にも`AnswerNode`を直接渡す。
旧`EditorState.active_path`/`cursor`は`apply_editor_action`互換境界だけに残し、MathLive production pathでは
生成・検証しない。selectionはMathLiveだけが所有する。空placeholderでのBackspaceはMathLiveの公開
range/selection/command APIから最小の空構造を削除し、その直後のfield値を明示的に再parseして
`AnswerNode`へ同期する。自前caret overlayやfraction/rootのpixel geometryは持たない。

小数は表示文字列をFloatへ変換せず、adapterが`ExactDecimal { coefficient, scale }`へ戻す。rootと
plusminusは今回構文として保持・表示するが、数値評価規則はまだ定義しない。解析不能なbounded raw textは
`nan_error`として保持し、JavaScript `number`へcoerceしない。`nan_error`を含む回答は採点で常に不正解である。

## Structural size

emptyは0、integerは符号を除いた十進桁数である。ExactDecimalはcoefficientの桁数と、小数点以下桁を表すために必要な`scale + 1`の大きい方を使う。Composite nodeは親1に全childのsizeを加える。従って`frac(num(12), num(42))`は`1 + 2 + 2 = 5`。この表示サイズとは別に、入力検証では構造node数も最大18とする。検証は19個目のnodeを見つけた時点で短絡して拒否するため、empty childだけのtupleもnode予算を消費する。structured editorとWASMのnormalize/grade境界は、両方の上限を適用前に検証する。

Serde表現は`type` discriminatorと`value` payloadを使用する。

```json
{"type":"exact_decimal","value":{"coefficient":"3","scale":1}}
```

上記はdisplay/input表現である。normalized表現は次の既約分数になる。

```json
{"type":"fraction","value":{"numerator":{"type":"integer","value":"3"},"denominator":{"type":"integer","value":"10"}}}
```

## Grading warnings

Rust coreは数学的同値性と安定したwarning identifierを返し、Webは詳細設定→採点設定でwarningカテゴリを○/×のどちらとして扱うか決める。

| identifier | Web表示 |
|---|---|
| `fraction_not_reduced` | 約分しましょう |
| `integer_form_required` | 整数でこたえましょう |
| `redundant_negative` | 最後まで計算しましょう |
| `redundant_plus_minus` | 最後まで計算しましょう |
| `redundant_decimal` | 最後まで計算しましょう |
| `duplicate_solution` | 最後まで計算しましょう |
| `solution_list_required` | 最後まで計算しましょう |
| `fraction_form_required` | 分数でこたえましょう |
| `mixed_fraction_form_required` | 帯分数でこたえましょう |

同じ表示カテゴリに複数codeが該当しても画面には1回だけ表示する。既定では「約分しましょう」「整数でこたえましょう」「最後まで計算しましょう」を×、「分数でこたえましょう」を○とする。採点設定は詳細設定からモーダルを開いて変更し、約分は「2/4 と 1/2」、整数化は「√16 と 4」、分数形式は「0.5 と 1/2」を具体例として示す。「最後まで計算しましょう」は、それら3項目以外の数学的に同値だが未整理・冗長な表記差をまとめて扱う。`fraction_form_required`は、数値としては同じでも通常分数指定に対して小数・帯分数など別形式で答えた場合の独立した表記警告である。`mixed_fraction_form_required`は、小学校の通常分数単元でcanonical answerが帯分数である値に対し、数学的に同値な仮分数など別形式で答えた場合に返す。どちらも数学的同値性そのものとは分離される。


`tuple` ASTは複数値の内部表現として共用するが、意味は`AnswerSchema`で分離する。二次方程式の複数解では順序を無視する一方、連立方程式の`ordered_pair` schemaではchild順を `(x,y)` として保持し、座標交換を同値とみなさない。連立方程式のWeb UIはtupleを直接入力させず、`x`欄と`y`欄を別々に編集してから順序付きtupleへ合成する。
