# Answer AST v2

`AnswerNode`は入力、表示、正規化、採点で共用する正確な型付き構文木である。

| node | exact payload |
|---|---|
| `empty` | なし |
| `integer` | Rust内部は`i64`、JSONではcanonical decimal string |
| `exact_decimal` | Rust内部は十進`coefficient: i64`と`scale: u32`。JSONのcoefficientはcanonical decimal string |
| `fraction` | numerator、denominator |
| `mixed_fraction` | whole、numerator、denominator |
| `root` | radicand、任意のindex |
| `negative` | child |
| `plus_minus` | child |
| `tuple` | child配列 |
| `variable` | 名前 |

小数はbinary floating-pointへ変換しない。例えば`0.3`は`coefficient = 3, scale = 1`、`0.57`は`57, 2`である。JSON/WASM境界ではi64 payloadを`"3"`のようなcanonical decimal stringにし、18桁整数もJavaScriptの`number`へ変換しない。BigNumもこのcoefficientや整数nodeから直接導出し、表示floatから桁列を復元しない。分数`41/57`は整数child 41と57を別々のBigNum sourceにする。

`AnswerRepresentation`はユーザーが入力・表示したtreeと採点用のnormalized treeを別fieldで保持する。`normalize_answer`は新しいcanonical treeを返し、呼び出し側のdisplay treeを書き換えない。整数、有限小数、分数、帯分数、negativeから構成できる正確な数値は、Floatを使わず符号を分子へ集約した既約分数（分母1ならinteger）へ統一する。従って`2/4`、`0.5`、`1/2`は同じnormalized treeになり、`4.0`はinteger `4`になる。現行整数editorもdraftの`EditorState.answer`と正規化処理を別境界としている。

## Structural size

emptyは0、integerは符号を除いた十進桁数である。ExactDecimalはcoefficientの桁数と、小数点以下桁を表すために必要な`scale + 1`の大きい方を使う。Composite nodeは親1に全childのsizeを加える。従って`frac(num(12), num(42))`は`1 + 2 + 2 = 5`。integer editorはsize 18を超える操作を状態変更なしで拒否する。

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
