# AutoDrill alpha 1.1 実装状況

更新日: 2026-08-10

この文書は、現在の作業ツリーを後続の開発者が再現・調査・拡張するための実装記録です。製品仕様の正典はリポジトリ直下の[`goal.txt`](../goal.txt)、学習内容とeffort定義は[`curriculum.md`](../curriculum.md)です。公開境界の詳細は[`problem-schema.md`](problem-schema.md)、[`answer-ast.md`](answer-ast.md)、[`effort-model.md`](effort-model.md)、Web/PDFの責務は[`web-pdf.md`](web-pdf.md)を参照してください。

## 1. 現在の到達点

alpha 1.0のq1設定、q2回答・採点、q3別タブPDFを維持しながら、alpha 1.1のカリキュラム選択、難易度生成、共通問題形式、共通解答AST、努力量計算、単元URLを実装しました。

- q1上部に`おすすめ`（既定）と`学年から選ぶ`の切替を持つ。
- `おすすめ`は`ジャンル → テーマ`の2段階、`学年から選ぶ`は`学年 → ジャンル → テーマ`の3段階select。
- 学年は小学1年生から中学3年生までで、slugは`grade-1`から`grade-9`。中1・中2・中3は`grade-7`・`grade-8`・`grade-9`。
- 実装済みテーマはnumeric theme ID 1の`一桁の足し算`、ID 2の`一次方程式(1)`、ID 3の`一次方程式(2)`。おすすめと学年選択は同じregistry object・generatorを参照する。
- 各学年の未実装枝は`Dummy1`で表し、選択中は生成・印刷を無効化して`このテーマはまだ利用できません`と表示する。DummyはURL、metadata、sitemapへ出さない。
- 難易度1〜5を実装し、既定3。UI、生成request、問題セットIDは同じ値を使う。
- 一桁足し算は候補8n件、2n種類の多様性条件、effort order statistic、最終重複排除、決定的shuffleで固定20問を生成する。
- 共通`AnswerNode`、標準解法`SolutionGraph`、固定27成分`OperationVector`、分離された重みlayerをRust coreに実装した。
- 問題セットIDは`{schema_version}-{numeric_theme_id}-{generator_revision}-{seed}-{difficulty}`。例は`3-1-3-Ab3Z-3`。
- 実装済み単元URLは`/drills/grade-1/one-digit-addition`。`/`はTOPのまま。
- Rust/WASM境界はschema version 3。TypeScript側に数学規則のfallbackを置かない。
- Problemの`input_interface`は`answer_schema`と直交する。現行の一桁足し算は`simple_numeric`で、q2はdigits-only keypadを表示する。`structured_math`では`allowed_structures`だけをパレットと物理キーへ投影する。

## 2. 画面と遷移

| 状態 | URL/入口 | 表示・操作 | 遷移 |
|---|---|---|---|
| q1 設定 | `/`または実装済み単元URL | ふりがな、選択モード、カリキュラムselect、難易度、Seed、問題生成、印刷 | 生成→q2、印刷→q3 |
| q2 回答 | q1で生成 | 2列×10行、固定リボン、固定数式パレット・10キー、物理キー、タイマー、採点、印刷 | 印刷→q3、TOP→q1 |
| q2 採点後 | q2で採点 | 誤答赤枠、赤字正答、問題に戻る、同一問題再挑戦、別Seed生成 | 各操作に応じq2へ |
| q3 印刷 | q1またはq2の印刷 | Blob URLの2ページPDF | ブラウザのタブ操作 |

q2は共有A4 geometryを使い、左列を1〜10、右列を11〜20の縦順で表示します。回答欄選択時だけ画面下のinterface投影による入力パネルを表示し、リボンと入力パネルはviewport固定です。一桁足し算の`simple_numeric`は10個の数字キーだけ、`structured_math`は許可された構造キーと共通の数字・編集キーを表示します。物理キーとボタンは同じcapability projectionを使い、未許可の構造キーは無視します。小数点は許可された場合だけ10キー最下段に表示します。Enterは確定して次問へ移動し、同列の1行分を自動スクロールします。

回答は表示sizeと構造node数をそれぞれ18以下にします。`EditorState.active_path`が分子・分母等の選択slot、`cursor`がslot内位置を示します。選択中は実カーソルを表示し、11pxを下限に回答枠を右方向へ広げます。いずれかの上限を超える入力は状態を変更せず、リボン付近へ`式が大きすぎます！`を表示します。

採点ボタンは入力FIFOを排出してから最新回答を読み、タイマーを停止します。数学的に正解でも表記に冗長性がある場合は、正解扱いのまま問題横へ`約分`、`冗長なマイナス`、`余計な小数点`のwarningを複数表示できます。`問題に戻る`は回答を保持してタイマーを再開、`もう一回問題を解く`は同じ問題セットで回答と時間を初期化、`別の問題を解く`は同じテーマ・難易度の新しい自動Seedを生成します。

## 3. カリキュラムregistry

Webの正規registryは`apps/web/src/domain/curriculum.ts`です。

- `CURRICULUM_TREE`: 9学年のcanonical tree。
- `RECOMMENDED_GENRES`: canonical treeへの参照だけを持つ部分集合。
- `IMPLEMENTED_THEMES`: metadata、static params、sitemapへ公開してよいテーマだけ。
- `ONE_DIGIT_ADDITION_THEME`: theme ID 1、revision 3、20問、2列×10行、単元URLを所有。
- Dummy theme: `implemented:false`、`route/search/layout/problemCount`は`null`。

Rustの生成registryは`crates/drill-core/src/registry.rs`です。numeric theme IDとgenerator revisionから、skill ID、curriculum path、問題数、列・行、テーマ別重みoverrideを復元します。問題数はrequestや問題セットIDに重複保存しません。

## 4. 問題生成と再現

`crates/drill-core/src/generator.rs`の`ProblemGenerator`が共通interfaceです。通常のcandidate-pool選択に加え、answer domainを持つgeneratorは「答えを先に一様抽選し、その答えに条件づけて複数式候補を生成し、effortで式だけを選ぶ」strategyを利用できます。現行registryは一桁足し算revision 3と一次方程式(1)/(2) revision 6を持ちます。

一桁足し算の生成手順は次の通りです。

1. n=20に対し、左右1〜9の順序付き組を重複ありで8n=160件生成する。
2. 異なる式が2n=40種類未満ならpool全体を捨て、新しい160件を生成する。
3. `effort → 順序付き式 → 生成ordinal`で決定的にsortする。
4. bootstrap選択ごとに残りpoolから独立な一様indexを5個生成する。
5. 難易度iは5個のi番目に小さい値が指す候補を選ぶ。
6. 最終採用済み式と重複した場合は、poolを変えず同じbootstrap枠で再抽選する。
7. 採用時だけpoolから候補を除き、24問（n+4）を選んだ後にeffort最小2問・最大2問を捨て、残る20問を同じRNG streamでFisher-Yates shuffleする。

既定deadlineは100ms、attempt上限は10,000です。候補生成、pool再生成、重複再抽選は同じbudgetを消費し、`generation_timeout`と`generation_attempt_limit`を別エラーで返します。

Seedは1〜16文字で、`1-9`、`a-z`、`A-Z`から`I`、`l`、`O`を除いた集合です。q1空欄時だけWebが同じalphabetの4文字Seedを生成します。ID `3-1-3-Ab3Z-3`はschema 3、theme 1、revision 3、Seed `Ab3Z`、難易度3へ可逆decodeでき、保存済みrevisionから同じ20問を再生成できます。

## 5. 共通問題・解答境界

Worksheetは次を持ちます。

- schema versionとproblem-set ID
- decode済み`ProblemSetIdentity`
- registry由来のskill ID、curriculum path、layout
- 20個の`Problem`

Problemはschema version、通し番号、numeric theme ID、typed prompt、`answer_schema`、直交した`input_interface`、canonical AnswerNode、SolutionGraph、27成分OperationVector、解決済みeffortを持ちます。

`AnswerNode`は次のvariantを定義済みです。

- empty
- integer
- exact decimal（十進coefficientとscale）
- nan_error（boundedな数値化不能raw text。表示・編集可能で、数値へcoerceせず常に不正解）
- fraction
- mixed fraction
- root
- negative
- plus-minus
- tuple
- variable

数学値はbinary floating-pointへ変換しません。Rust内部のinteger/coefficient/answer-schema limitは`i64`、BigNum magnitudeは`u64`ですが、JSON/WASMではcanonical decimal stringとしてserializeします。例えば18桁`999999999999999999`は文字列のままWebへ渡り、JavaScript `number`を経由しません。Floatを許可するのは`log10`で得るoperation quantityと最終effortだけです。採点時は整数・有限小数・分数・帯分数・negativeを既約有理数へexact変換するため、`2/4 = 0.5 = 1/2`、`4.0 = 4`として比較します。

`AnswerNode::size()`が全variant共通の表示サイズ契約です。integerは十進桁数、compositeは親1と全childの合計で、`frac(num(12),num(42))`は5です。入力検証ではこれとは別に構造node数も最大18とし、19個目のnodeで短絡して拒否します。入力/display treeと採点用normalized treeは`AnswerRepresentation`で分離できます。

structured editorは分数・帯分数・root・negative・plusminusを選択slotへ挿入し、commaをtop-level tupleとして追加します。小数点は数式templateではなく通常の10キー最下段に独立した`.`キーとして置き、表示文字列をFloatへ通さず十進coefficientとscaleを直接更新します。整数・有限小数・分数・帯分数・negative・tuple childはexactに正規化されます。rootとplusminusは構文入力と表示までで、数値評価は後続単元の責務です。回答枠は固定高ではなく、分数・帯分数・root等の描画内容に応じて縦へ伸び、問題行の中央に配置して上下を切り取りません。

## 6. Effortモデル

`crates/drill-core/src/effort.rs`は次を分離します。

1. typed operationをnodeに持つ標準解法graph
2. 未使用成分も0として保持する固定27成分operation vector
3. base weight
4. grade、theme、masteryの倍率layer

解決値は`base × grade × theme × mastery`です。alpha 1.1の各layerは1.0ですが、theme registryのoverrideだけを変更して同じgraph/vectorを再評価できます。

一桁足し算graphは`BigNum(left+right)`と`BasePlus`を持ち、和が10以上なら`Increment`と`OverheadCarryPlus`を別nodeで加えます。BigNumはoperandでなく正解ASTから導出します。分数41/57はBigNum(41)+BigNum(57)、小数0.57はBigNum(57)です。

`OverheadNegative`は負数operandを含む演算ごとに1回です。唯一の一般例外は、正のa,bに対する`a + (-b)`で、aとbの大小関係を問わず0回です。operand順序は区別し、`(-b)+a`と`a-(-b)`には1回加えます。単独の負数literalは演算ではないため、`-0.57`はBigNum(57)だけです。

全27成分と重みは[`effort-model.md`](effort-model.md)に固定しています。特にTransposition=2、Carry Plus/Minus/Mult=0.5です。

## 7. Web/WASM/PDF境界

`apps/web/src/domain/wasm-adapter.ts`が唯一の本番JSON境界です。schema、identityとIDの一致、Web theme registryと一致するtheme/revision/layout/problem count、問題ID一意性、typed prompt/answer、input-interface capabilities、表示sizeと構造node予算、27成分vector、exact decimal stringをfail-closedで検証します。editor state/candidateとgrade expected/actualも選択interfaceへ照合し、v2、version欠落、legacy action aliasは受け付けません。

公開WASM exportは次の7個です。

| export | 役割 |
|---|---|
| `generate_problem` | registryから1問生成 |
| `generate_worksheet` | registry所定問題数を難易度付き生成 |
| `regenerate_problem_set` | 可逆IDから再生成 |
| `apply_editor_action` | AnswerNodeとcursorを編集 |
| `normalize_answer` | canonical treeを返す |
| `grade_answer` | typed AnswerNodeを採点 |
| `calculate_effort` | graph/vector/重みからeffort算出 |

Webは画面状態、入力FIFO、タイマー、表示、PDF描画だけを担当します。生成規則、正規化、採点、effort計算をTypeScriptへ複製しません。

PDFは`apps/web/src/domain/layout.ts`のA4 geometryをq2と共有します。1ページ目は問題と空枠、2ページ目は解答で180度回転し、両ページ右下に小さく生成日とSeedを記録します。q1/q2は同じ`openWorksheetPdf`を使い、q1は非同期生成前に空タブを開いてpopup blockを避けます。

## 8. 主なファイル

```text
apps/web/src/domain/theme-registry.ts   # 実装themeのWeb側single registry
apps/web/src/domain/curriculum.ts       # registryから9学年/recommended/Dummyを投影
apps/web/src/domain/problem-format.ts   # typed promptの共有expression formatter
apps/web/src/domain/drill-engine.ts     # schema-v3 TypeScript DTO/interface
apps/web/src/domain/wasm-adapter.ts     # registry照合付きstrict JSON/WASM adapter
apps/web/src/components/AutoDrillApp.tsx# q1/q2 UIと状態遷移
apps/web/src/pdf/worksheet-pdf.ts       # 共有layoutから2ページPDF
crates/drill-core/src/answer.rs         # typed exact AnswerNodeとsize
crates/drill-core/src/exact.rs          # i64/u64 canonical decimal JSON serde
crates/drill-core/src/generator.rs      # registry generatorと8n trimmed-bootstrap difficulty sampling
crates/drill-core/src/identity.rs       # Seed、difficulty、可逆problem-set ID
crates/drill-core/src/registry.rs       # revision付きtheme/generator登録
crates/drill-core/src/effort.rs         # graph、27-vector、weights、negative/carry semantics
crates/drill-wasm/src/lib.rs            # 薄いwasm-bindgen JSON envelope
```

## 9. 既存alpha受入証拠（2026-08-02）

- repository bridge: healthy。
- Rust: `cargo fmt --all -- --check`、`cargo check --workspace --all-targets`、Clippy warning-as-error、`cargo test --workspace --all-targets`成功。core 22件、WASM 5件。
- Web: lint、typecheck、65 tests、production build成功。
- Next output分離: clean状態からdev serverを起動し、production build中に44回assetを継続取得し、build後のproduction server配信まで成功。
- static build: `/`、`/drills/grade-1/one-digit-addition`、`/sitemap.xml`を生成。Dummy routeは生成しない。
- actual wasm-bindgen/Node: `3-1-3-Ab3Z-3`、20式一意、canonical answer/answer schema/BigNumのdecimal string、18桁`999999999999999999`の完全往復を確認。約分・通常/分数内の冗長なマイナス・余計な小数点warningも生成済みWASM実体で確認。
- 実ブラウザ/q1: おすすめ2段select、学年3段select、grade-1〜9、中1=`grade-7`、Dummy選択時の警告と生成・印刷無効、難易度1〜5・既定3を確認。
- 実ブラウザ/q2: 左1〜10・右11〜20、初期入力パネル非表示、選択後の固定数式パレット・10キー、物理入力、Enterで次問と83pxスクロール、固定リボンを確認。分数`1/2`、帯分数`1 1/2`、小数`0.5`、`√9`、`−3`、`±4`、tuple `1, 2`を生成済みWASMで入力・表示確認。
- 実ブラウザ/採点: 20誤答枠、20正答表示、3つの採点後操作、タイマー停止を確認。
- 実ブラウザ/q3: q2印刷が別Blob PDFタブを開くことを確認。ブラウザ安全制限によりBlob内容の取得・撮影は拒否されたため、今回の最新差分ではPDF bytes/geometryの自動テスト4件を受入証拠とした。alpha 1.0時点では同一layoutの2ページをPoppler描画し目視確認済み。

### 2026-08-09 schema-v3/input-interface/nan_error recovery acceptance

- v3 identity `3-1-3-Ab3Z-3` round-trips through generation and regeneration; explicit v2 and missing-version requests/IDs fail closed.
- `simple_numeric` is enforced for the current one-digit theme; structured interfaces remain typed and capability-driven for future themes. Rust owns the reducer and validates structure actions; Web keypad and physical-key mappings use the same projection. `allow_negative` controls both the physical minus key and the visible minus control.
- Non-Clear editor actions validate the incoming active path/cursor before clone or mutation. `SelectSlot` requires an explicit in-range path/cursor; no malformed value is repaired. `Clear` remains unconditional recovery.
- Every typed editor state/candidate and grade answer is checked against its supplied input interface. `nan_error` remains bounded raw text for recovery and never promotes to a disallowed decimal, negative, or structure node.
- Web grade DTO status is restricted to `correct`/`incorrect`/`unanswered` and is checked against `is_correct` plus whether actual is the typed `empty` answer.
- `nan_error` preserves bounded raw text such as `3.1.4.5` through editing and grading, never enters exact arithmetic, and is always incorrect. Exact decimal, AST-size, structural-node, and extreme-scale protections remain active.
- Rust workspace formatting, Clippy warning-as-error, and all-target tests passed (drill-core 37 tests, drill-wasm 10 tests); Web also passed 8 files / 90 tests, typecheck, lint, production build, and `git diff --check`.
- Focused UI tests cover simple-numeric decimal/negative capability projection (including allowed/disallowed negative controls and physical-minus mapping) and structured-math templates/slot editing; focused adapter tests cover v2 rejection, interface mismatch, node-budget rejection, malformed paths, strict grade status consistency, and raw nan_error projection.
- `./scripts/build-wasm.sh` regenerated the actual wasm-pack package. Headless Chrome against the generated package confirmed the current one-digit theme shows digits-only `simple_numeric`, physical digit input works, disallowed physical `-` is ignored, and both ribbon/input panel are fixed. A temporary injected-engine browser fixture (removed after verification) confirmed `allow_negative=true` exposes the visible minus control with the same result as physical `-`, and `structured_math` exposes the six rich structure controls plus an independent decimal key; on-screen fraction and physical `/` both create a vertically stacked fraction. Runtime console warning/error count was zero in these interaction checks. Direct generated-WASM editing also preserved `3.1.4.5` as recoverable `nan_error` raw text.

### 2026-08-09 一次方程式(1)/(2) acceptance

- Rust generator registryにはtheme ID 2/revision 6「一次方程式(1)」とtheme ID 3/revision 6「一次方程式(2)」を登録する。両者は同じ`LinearEquationGenerator` trait実装をmode違いで共有し、generator dispatchは`REGISTERED_GENERATORS`からregistration identityで解決する。
- Webは`theme-registry.ts`をsingle sourceとして、grade tree、Recommended grouping、routes/sitemap、16問2列8行layout、prompt kind、answer-schema kind、rich input capabilityを導出する。Recommendedは`足し算と引き算 / 一桁の足し算`と`方程式 / 一次方程式(1),(2)`を保持し、gradeは`中学1年生 / 一次方程式 / 一次方程式(1),(2)`。
- revision 6では128候補それぞれでcanonical answerをanswer domainから独立一様に復元抽出し、その答えを固定したまま式を条件付き生成する。同じ答えの重複は自然な抽選結果として許容する。128式は答えごとに分けず1つの共通poolでeffort比較し、既存5-way order statisticでdifficultyを適用する。(1)のanswer domainは整数`-15..=15`。(2)はそれに、既約形で分母2・`|分子|<=20`、または分母3..12・`|分子|<=15`の非整数有理数を加えた集合。
- 式は `ax+b=0` / `ax+b=d` / `ax+b=cx` / `ax+b=cx+d` の4形だけを生成し、選んだ答えを満たす係数へ条件づける。係数domainは従来どおり、(1)は絶対値15以下の整数、(2)はそれに`|分子|+分母<=10`の既約非整数分数を加える。`b=0`は許可して`+0`を表示しないが、`ax=cx`へ退化する候補は棄却する。全answer domainを自動走査し、各答えから少なくとも1つ有効な式を生成できることをRust testで確認する。
- (2)では非零解に対し候補生成の3/4で、移項後`Ax=B`の`B/A`が共通因子`k>=2`を持つよう逆構成を優先する。難易度3の6 seed/96問で94問が実際に最終約分を必要とし、回帰条件は75%以上に固定した。
- 負定数の表示は候補ごとに1/2で`a−b` / `a+(−b)`を選ぶ。共有`problem-format.ts`は`0x`を出さず、係数1/-1を`x`/`−x`へ整形する。minusは独立math tokenで、Webは数学用`−`、PDFは横線を直接描画するためASCIIハイフンへ退化しない。Web/PDFの分数も同じtyped math token列から縦型に描画する。
- 一次方程式の標準effort graphは`ax+b=cx+d -> Ax=B -> x=B/A`に従い、`OverheadLinear`、実際の移項、係数/定数整理、分母整理、最終除算、必要なGCD/negative/BigNumをoperation vectorへ記録する。
- (2)の非整数解は`require_reduced_fraction_form` policyを持つ。未約分通常分数は`fraction_not_reduced` warning付き不正解、同値な帯分数・有限小数・繁分数等は`fraction_form_required` warning付き正解。整数解を`−1/1`や`√0`のような別表現で答えた場合も数学的同値性をexactに判定し、`integer_form_required` warning付き正解として表示する。warningは問題横の可視バッジとして表示する。
- Web worksheetは16問、式をセル左上、`x = [ ]`を右下へ配置する。rich keyboardは分数・帯分数・平方根・マイナス・plusminus・tupleと独立小数点をすべて表示し、左右矢印は隣接する。初回選択でも回答欄が固定keyboardに隠れないようviewport safety scrollを適用する。PDFも同じ2×8 layoutとexpression formatterを利用する。
- 生成済みWASMを直接Nodeから呼び、両themeの16問生成、rich interface、exact canonical answer、schema-aware gradeを確認。(2)の実分数解に対して未約分を送ると`incorrect + fraction_not_reduced`を返した。
- production buildをHeadless Chromeで実行し、(1)/(2)とも16問、instruction、全16個の`x =`、`0x/1x`非表示、全rich key、小数点、`← →`隣接、fixed input panel、console/runtime issues 0を確認。回答欄選択後は実測でanswer bottom 234px、keyboard top 258pxとなり重なりなし。
- 最終検証: `cargo fmt --all -- --check`、Clippy `--workspace --all-targets -D warnings`、Rust all-target tests（drill-core 41 / drill-wasm 10）、Web full Vitest（9 files / 100 tests）、TypeScript typecheck、ESLint warning 0、Next production build、`git diff --check`を実施。
- q1のnative selectを廃止し、候補行にもsemantic rubyを描画できる共通custom combobox/listboxへ変更。q2の数式templateは文字組みではなくTeX風のinline SVG glyphへ変更した。空のstructured slotでBackspaceされた場合はRust reducerがそのslotを含む最も浅いAST nodeを削除し、実WASM/ブラウザでもfraction→emptyを確認した。

### 2026-08-10 structured-math rendering / equation sampling hardening

- 数式template SVGのplaceholderを拡大し、`±`/`−`との視覚重量を揃えた。帯分数は整数部と分数部の水平間隔を縮め、1つの数として読める比率へ調整した。
- nested fractionの表示CSSを`grid-template-rows: 1fr 1fr`から`auto auto`へ変更。親分数が子分数の高さを上下両段へ複製して指数的に肥大する再帰を除去した。
- Web editorはRustが返したgrowth candidate（digit/structure insertion）をoff-screenで同じ`StructuredAnswer`として1回renderし、実DOM寸法をproblem cellの許容幅/高さと比較する。超過候補はstateへcommitせずnopとし、Backspace/Delete等の縮小操作は常に通す。これはfractionだけでなくroot/mixed-fraction/tuple/negative等の全ASTへ共通適用する。
- Headless Chrome実測では繁分数の表示高さはempty約19px → 1段約48px → 2段約69pxとなり、3段目以降はsize guardによりnop。7回連打後も2段・約69pxのままで、103px高のproblem cell内に収まった。
- revision 6では各candidateで答えを一様抽選してから4形の係数を逆算し、式生成失敗時も同じ答えを保持する。128候補全体を`SolutionGraph`由来effortで比較するため、答え自体の計算負荷もdifficultyに反映される。(2)は最終除算で約分が発生する整数共通因子付き`A,B`を優先する。
- revision 6 validation: Rust workspaceはdrill-core 48 tests / drill-wasm 10 tests、`cargo fmt --check`、Clippy `-D warnings`が成功。WebはTypeScript typecheck、ESLint warning 0、関連Vitest 82 testsが成功し、actual wasm-packを再生成した。生成済みWASMの一次方程式(2)を8 seedで測定した平均effortはdifficulty 1..5で`17.57, 21.02, 23.61, 25.82, 28.31`と単調増加した。候補poolは8n、bootstrapはn+4問を選択し、effort最小2問・最大2問をtrimして最終n問とする。

## 10. 既知の制約と次の拡張点

- 実装済みgeneratorは一桁足し算、一次方程式(1)、一次方程式(2)。ほかの学年・テーマは意図的にDummy。
- 新themeはRustの`ThemeRegistration + ProblemGenerator`とWebの`ThemeDefinition`を追加し、prompt variant/formatterが新種ならそこだけ拡張する。grade tree、Recommended、routes、sitemap、layout validationへ個別theme-ID分岐を追加しない。
- 旧generator revisionを永続的に再現するには、revision実装を削除せずregistryへ残す。
- `NEXT_PUBLIC_SITE_URL`未指定時、sitemap originは開発用`http://localhost:3000`。配布時は正規URLを指定する。
- PDFは標準Helveticaで数字・ASCII footerだけを描画する。日本語タイトルはWebリボンに留める。
- 実プリンター固有の余白、両面印刷向き、別ブラウザ・別OSフォントは未検証。
- 配布用CSP/HTTP headerとtransitive依存licenseの全監査は未実施。

## 11. 再現コマンド

```bash
../../scripts/init-project.sh --check AutoDrill
pnpm install --frozen-lockfile
./scripts/build-wasm.sh
pnpm lint
pnpm typecheck
pnpm test
pnpm build
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
pre-commit run --all-files
pnpm dev
```

`./scripts/build-wasm.sh`はRust targetやCLIを自動installしません。生成物はGit管理外の`apps/web/public/wasm/pkg/`へ置かれます。`next dev`は`.next-dev`、production build/startは`.next`を使い、同時実行でassetを上書きしません。
