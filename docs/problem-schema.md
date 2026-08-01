# Problem schema v2

Alpha 1.1のRust domainとWASM JSON境界は`schema_version = 2`を共通で使用する。生成requestは次の値だけを持つ。

```json
{
  "schema_version": 2,
  "numeric_theme_id": 1,
  "seed": "Ab3Z",
  "difficulty": 3,
  "timeout_ms": 100,
  "max_attempts": 10000
}
```

`timeout_ms`と`max_attempts`は省略可能で、既定値は100msと10,000試行である。問題数はrequestやIDへ入れず、revision付きgenerator registryから得る。一桁の足し算はtheme ID 1、generator revision 2、20問、2列10行である。

## Problem-set identity

ID形式は次の可逆なASCII表現である。

```text
{schema_version}-{numeric_theme_id}-{generator_revision}-{seed}-{difficulty}
```

例は`2-1-2-Ab3Z-3`。Seedは1〜16文字で、`1-9`、`a-z`、`A-Z`から`I`、`l`、`O`を除いた集合だけを許可する。`-`を許可しないため分割は曖昧にならない。decodeした`ProblemSetIdentity`と保存済みrevisionのgeneratorで同じ問題セットを再生成できる。

Worksheetは`problem_set_id`に加えてdecode済み`identity`、registry由来のskill/curriculum/layout、20個のProblemを持つ。Problemはversion、通し番号、numeric theme ID、型付きprompt、answer schema、正解AnswerNode、標準解法graph、固定長operation vector、解決済み重みによるeffortを持つ。数学値は整数またはAnswerNodeの正確な十進表現で保持し、binary floatは使用しない。JSON/WASM境界のAnswerNode integer/coefficientとanswer-schema bounds（`i64`）、BigNum magnitude（`u64`）はcanonical decimal stringとし、JavaScriptの安全整数範囲を超える18桁値も可逆にする。`effort`とoperation vector quantityだけは評価結果なので`f64`を許可する。

## Difficulty sampling

n問に対して同じ決定的RNG streamで次を行う。

1. 重複を許して5n候補を生成する。異なる式が2n種類未満ならpool全体を捨てて生成し直す。
2. effort昇順、式、生成ordinalの順で決定的にsortする。
3. k問目では残り`[1, 5n-k+1]`から独立な一様乱数を5個引く。難易度iはi番目のorder statisticを選ぶ。
4. 最終採用済みと同じ式ならpoolから除かず、同じkを再試行する。採用時だけ候補を除く。
5. n問採用後、同じRNG streamでFisher–Yates shuffleする。

候補生成、pool再生成、重複選択の再試行は共通のattempt budgetとmonotonic deadlineで打ち切る。エラーcodeは`generation_timeout`と`generation_attempt_limit`で区別する。

WASM adapterはcore DTOをそのまま`ApiResponse`の`data`へserializeする薄い境界であり、同じschemaを別形式へ再定義しない。

`grade_answer`はnormalizedな`expected`/`actual`、`is_correct`、statusに加え、表記だけが冗長な正解へ`warnings`配列を返す。alpha 1.1の識別子は`fraction_not_reduced`、`redundant_negative`、`redundant_decimal`で、表示文言とは分離する。
