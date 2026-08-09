# Answer AST v3

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

入力許可は`Problem.input_interface`が所有し、`answer_schema`とは直交する。`simple_numeric`は桁キーを
基本にし、`allow_decimal`と`allow_negative`だけを追加許可する。`structured_math`は
`allowed_structures`に列挙された構造だけを数式パレット、ボタン、物理キーへ投影する。
一桁の足し算はdigits-only keypadで、構造テンプレート、小数点、負数キーを表示しない。

`EditorState`は`answer`に入力途中のtree、`active_path`に選択中のnumeric slotまでのchild index列、`cursor`にそのslotの表示文字列内offsetを持つ。例えば分数の分子は`[0]`、分母は`[1]`、帯分数の整数・分子・分母は`[0]`、`[1]`、`[2]`である。各slotはempty、非負integer、非負exact decimalを直接編集し、符号は`negative` nodeとして構造化する。

`insert_structure`は`fraction`、`mixed_fraction`、`decimal`、`root`、`negative`、`plus_minus`、`tuple`を受け付ける。分数等は現在slotを置換または包み、tupleは複数解を表すtop-level nodeへ項を追加する。左右移動はslot内cursorを動かし、端に達すると前後のslotへ移る。`select_slot`により表示された分子・分母等を直接選べる。`select_slot` requestはpathとslot内cursorを必須とし、現在stateのactive path/cursorと選択先path/cursorをclone・編集前に検証する。範囲外や欠落値は補正せず拒否し、`clear`だけは壊れたstateからの無条件復旧として受け付ける。

小数入力は表示文字列をFloatへ変換せず、常にcoefficientとscaleへ戻す。入力途中の`12.`は`ExactDecimal { coefficient: 12, scale: 0 }`、続けて`3`を入力すると`12.3`、すなわち`{ coefficient: 123, scale: 1 }`になる。rootとplusminusは今回構文として保持・表示するが、数値評価規則はまだ定義しない。解析できないbounded raw textは`nan_error`としてそのまま表示し、編集可能なslotに残す。`nan_error`は入力interfaceに依存しない回復用sentinelだが、そこから生成される候補もinterface検証を通り、許可されない小数・符号・構造をtyped nodeへ昇格させない。`nan_error`をJavaScript `number`へ変換したり、採点で空欄・数値へ置き換えたりせず、含まれる回答は常に不正解とする。

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

採点はnormalized valueが等しければ正解とする。ただし期待表現と実回答表現が異なり、実回答に簡約可能な冗長性がある場合は`GradeResult.warnings`へ安定した識別子を加える。

| identifier | alpha 1.1表示 | 判定 |
|---|---|---|
| `fraction_not_reduced` | 約分 | integer分子・分母の最大公約数が1より大きい |
| `redundant_negative` | 冗長なマイナス | `negative`が負の表現をさらに包み、マイナスが2回ある |
| `redundant_decimal` | 余計な小数点 | scale 0、またはcoefficient末尾0を除去できるexact decimal |

warningは正誤を変更せず、複数該当時は上表順で併記する。期待表現と実回答が同じ場合、または不正解の場合は出さない。UI文言はidentifierと分離し、後続版で変更できる。
