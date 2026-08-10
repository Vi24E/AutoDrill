# AutoDrill alpha 1.1 実装状況

更新日: 2026-08-10

この文書は、現在の作業ツリーを後続の開発者が再現・調査・拡張するための実装記録です。製品仕様の正典はリポジトリ直下の[`goal.txt`](../goal.txt)、学習内容とeffort定義は[`curriculum.md`](../curriculum.md)です。公開境界の詳細は[`problem-schema.md`](problem-schema.md)、[`answer-ast.md`](answer-ast.md)、[`effort-model.md`](effort-model.md)、Web/PDFの責務は[`web-pdf.md`](web-pdf.md)を参照してください。

## 1. 現在の到達点

alpha 1.0のq1設定、q2回答・採点、q3別タブPDFを維持しながら、alpha 1.1のカリキュラム選択、難易度生成、共通問題形式、共通解答AST、努力量計算、単元URLを実装しました。

- q1上部に`おすすめ`（既定）と`学年から選ぶ`の切替を持つ。
- `おすすめ`は`ジャンル → テーマ`の2段階、`学年から選ぶ`は`学年 → ジャンル → テーマ`の3段階select。
- 学年は小学1年生から中学3年生までで、slugは`grade-1`から`grade-9`。中1・中2・中3は`grade-7`・`grade-8`・`grade-9`。
- 実装済みテーマはnumeric theme ID 1〜11の11テーマ。一桁の足し算、一次方程式(1)/(2)、一桁の引き算、二桁の足し算、九九、負の数の計算(1)/(2)、分数の足し算、分数の引き算、分数の掛け算を登録し、おすすめと学年選択は同じregistry object・generatorを参照する。
- 各学年の未実装枝は`Dummy1`で表し、選択中は生成・印刷を無効化して`このテーマはまだ利用できません`と表示する。DummyはURL、metadata、sitemapへ出さない。
- 難易度1〜5を実装し、既定3。UI、生成request、問題セットIDは同じ値を使う。
- 一桁足し算は候補8n件、2n種類の多様性条件、effort order statistic、最終重複排除、決定的shuffleで固定20問を生成する。
- 共通`AnswerNode`、標準解法`SolutionGraph`、固定27成分`OperationVector`、分離された重みlayerをRust coreに実装した。
- 問題セットIDは`{schema_version}-{numeric_theme_id}-{generator_revision}-{seed}-{difficulty}`。例は`3-1-3-Ab3Z-3`。
- 実装済み11テーマはそれぞれcanonical単元URLを持ち、metadata/static params/sitemapも同じtheme registryから導出する。`/`はTOPのまま。
- Rust/WASM境界はschema version 3。TypeScript側に数学規則のfallbackを置かない。
- Problemの`input_interface`は`answer_schema`と直交する。整数算術テーマは必要に応じてnegativeを許可する`simple_numeric`、分数テーマと一次方程式は`structured_math`を使い、`allowed_structures`だけをパレットと物理キーへ投影する。

## 2. 画面と遷移

| 状態 | URL/入口 | 表示・操作 | 遷移 |
|---|---|---|---|
| q1 設定 | `/`または実装済み単元URL | ふりがな、選択モード、カリキュラムselect、難易度、Seed、問題生成、印刷 | 生成→q2、印刷→q3 |
| q2 編集中 | q1で生成、または採点後に問題へ戻る | 問題選択、MathLive回答欄、入力パネル、タイマー、採点、印刷 | 採点→`grading`、TOP→q1 |
| q2 採点中 | 編集中に採点 | 入力・問題選択をロック、タイマー停止、採点ボタン黒転・無効 | 成功→`graded`、失敗→`editing` |
| q2 採点済み | 採点成功 | 誤答赤枠、赤字正答、全回答readonly、採点ボタン黒転・無効、採点後操作 | 問題に戻る/再挑戦→`editing`、別Seed→`replacing` |
| q2 別問題生成中 | 採点済みから別Seed生成 | 現在の採点結果を保持したまま結果操作・TOP・印刷をロック | 成功→新しい`editing`、失敗→元の`graded` |
| q3 印刷 | q1またはq2の印刷 | Blob URLの2ページPDF | ブラウザのタブ操作 |

q2はregistryの行数を使う共有A4 geometryで、20問テーマは左1〜10・右11〜20、16問テーマは左1〜8・右9〜16の縦順で表示します。回答欄選択時だけ画面下のinterface投影による入力パネルを表示し、リボンと入力パネルはviewport固定です。一桁足し算の`simple_numeric`は10個の数字キーだけ、`structured_math`は許可された構造キーと共通の数字・編集キーを表示します。物理キーとボタンは同じcapability projectionを使い、未許可の構造キーは無視します。小数点は許可された場合だけ10キー最下段に表示します。Enterは確定して次問へ移動し、同列の1行分を自動スクロールします。

回答は表示sizeと構造node数をそれぞれ18以下にします。Webのcaret・selection・分子/分母placeholder移動はMathLiveだけが所有し、production回答stateと採点requestはRustが受理した`AnswerNode`を直接保持します。旧`EditorState.active_path`/`cursor`は`apply_editor_action`互換境界に限定します。回答枠はMathLiveの実DOM寸法でgrowth guardを掛け、分数・帯分数等の必要な高さは自動的に確保します。上限を超える入力はRust回答stateへ反映せず、`式が大きすぎます！`を表示します。空構造をBackspaceで削除した場合も削除後のMathLive値を明示的に再parseし、表示と`AnswerNode`を同期します。

q2の排他的な`WorksheetPhase`は`editing / grading / graded / replacing`の4状態です。採点開始時は最初の`await`より前に同期的に`grading`へ遷移し、全MathLive fieldをreadonly化して入力FIFOを排出してから最新回答を採点します。`grading`/`graded`/`replacing`では採点ボタンを黒地・白字のpressed表示にして再押下を禁止します。採点失敗だけ`editing`へ戻り、採点成功後の`問題に戻る`は回答を保持してタイマーを再開、`もう一回問題を解く`は同じ問題セットで回答と時間を初期化します。`別の問題を解く`は`replacing`で同じテーマ・難易度の新Seedを生成し、成功時のみ新しい`editing`へ入り、失敗時は元の`graded`へ戻ります。数学的に正解でも表記に冗長性がある場合は、正解扱いのまま問題横へwarningを複数表示できます。

## 3. カリキュラムregistry

Webの正規registryは`apps/web/src/domain/curriculum.ts`です。

- `CURRICULUM_TREE`: 9学年のcanonical tree。
- `RECOMMENDED_GENRES`: canonical treeへの参照だけを持つ部分集合。
- `IMPLEMENTED_THEMES`: metadata、static params、sitemapへ公開してよいテーマだけ。
- `IMPLEMENTED_THEMES`はtheme ID 1〜11を保持する。20問テーマは2列×10行、一次方程式と分数テーマは16問・2列×8行で、各theme definitionが単元URLと表示metadataを所有する。
- Dummy theme: `implemented:false`、`route/search/layout/problemCount`は`null`。

Rustの生成registryは`crates/drill-core/src/registry.rs`です。numeric theme IDとgenerator revisionから、skill ID、curriculum path、問題数、列・行、テーマ別重みoverrideを復元します。問題数はrequestや問題セットIDに重複保存しません。

## 4. 問題生成と再現

`crates/drill-core/src/generator.rs`の`ProblemGenerator`が共通interfaceです。通常のcandidate-pool選択に加え、answer domainを持つgeneratorは「答えを先に一様抽選し、その答えに条件づけて複数式候補を生成し、effortで式だけを選ぶ」strategyを利用できます。現行registryはtheme ID 1〜11を持ち、一桁足し算はrevision 3、一次方程式(1)/(2)はrevision 6、theme ID 4〜11の8算術テーマはrevision 1です。

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
- registry所定の16個または20個の`Problem`

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

structured math入力はMathLiveへ分数・帯分数・root・negative・plusminus・tupleのLaTeX templateを挿入し、各`input` snapshotを`parse_mathlive_answer`経由でRust `AnswerNode`へ戻します。小数点は通常の10キー最下段に独立した`.`キーとして置きます。整数・有限小数・分数・帯分数・negative・tuple childはRustでexactに正規化され、rootとplusminusの数値評価は後続単元の責務です。回答枠は固定高にせずMathLive内容に応じて伸びるため、分数・帯分数・rootを上下で切り取りません。

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
- q1のnative selectを廃止し、候補行にもsemantic rubyを描画できる共通custom combobox/listboxへ変更。q2の旧inline SVG/独自slot rendererは2026-08-10にMathLiveへ置換済みで、Web production pathには残さない。

### 2026-08-10 MathLive migration / equation sampling hardening

- Web数式renderer/editorをMathLive 0.110.0へ統一した。問題式・採点後canonical answer・数式palette previewは`math-span`、editable answerは`math-field`を使う。旧native MathML helper、MathML JSX宣言、CSS/SVG fraction/root/caret rendererは削除し、Web内でrendererを混在させない。PDFは従来どおりpdf-libの独立rendererを使う。
- MathLiveはlayout、caret、placeholder navigationを担当し、Rustは数学authorityのまま維持する。`math-field.value`は`parse_mathlive_answer` WASM adapterを通して`AnswerNode`へ変換し、normalize/grade/effortは既存Rust coreが担当する。MathLiveが返す`\frac72`/`\sqrt2`等のcanonical TeXもRustで解釈する。
- 空structured placeholderのBackspaceはMathLiveの公開range/selection/command APIだけで最小の空構造を削除する。独自caret/slot overlayは持たない。growth guardはMathLive実DOMを測定し、セル外へ伸びるgrowthだけをnopにする。
- 実Chromeで整数、caret中間挿入、分数`7/2`、帯分数`1 1/2`、平方根`√2`、nested fraction、空分数Backspace、canonical answerを確認。instructionは15px、一次方程式は17pxで、problem/canonical/paletteはMathLive static renderer、answerはMathLive editorとなり旧MathML/SVG要素は0件だった。
- MathLive既定placeholder記号`▢` (U+25A2) はChromeでfraction numeratorへ縮小された際に上辺が欠ける。`□` (U+25A1) は欠けないが空分数の視覚中心が約2px上へ寄ったため、公開`placeholderSymbol`は`☐` (U+2610)を使う。20pxの実answer-boxを2倍Rasterで画素測定し、空分数の黒線包含矩形と外枠中心の差が約0.25pxまで縮むことを確認した。serialized LaTeXは引き続き`\placeholder{}`のままRust/WASMへ渡す。
- editable answerの外枠とMathLive rendererを分離した。外側`answer-box`だけがborder/min-size/paddingを持ち、内側`answer-mathfield`は自然サイズで描画する。外枠はflexで内側を水平・垂直中央配置し、MathLiveの公開`container`/`content` partの既定paddingと強制widthを除去する。実Chrome計測では整数・空分数・空帯分数の外枠中心と公開`content`中心がすべて`Δx=0, Δy=0`、実描画baseとの差も縦約0.10pxだった。growth guardは分離後も外枠込みの実footprintを測定する。
- 静的palette previewと編集templateを分離した。previewは`\square`を使って入力位置を常に可視化し、編集時だけMathLiveの`\placeholder{}`を使う。実Chromeでは分数・帯分数previewのMathLive glyph全体がボタン内に収まり、上端余白8px、回答欄の空/入力済み分数・帯分数はいずれも上下overflow 0pxだった。
- 採点状態を`editing / grading / graded / replacing`で明示した。`grading`は同期lockで二重採点を防ぎ、`graded`ではfield click/物理キー/入力panel再表示を禁止する。`replacing`は別問題生成の非同期競合を遮断し、失敗時に`graded`へ戻る。採点中・採点済み・置換中の採点ボタンは`aria-pressed=true`、黒地白字、disabledとなる。実Chromeで採点済み値`9`へ物理キー`7`を送っても値不変、`問題に戻る`後だけreadonly解除・入力panel復帰を確認した。
- この修正後のWeb full Vitestは9 files / 108 tests、TypeScript typecheck、ESLint warning 0、contract check、Next production build、`git diff --check`が成功した。
- revision 6では各candidateで答えを一様抽選してから4形の係数を逆算し、式生成失敗時も同じ答えを保持する。128候補全体を`SolutionGraph`由来effortで比較するため、答え自体の計算負荷もdifficultyに反映される。(2)は最終除算で約分が発生する整数共通因子付き`A,B`を優先する。
- revision 6 validation: Rust workspaceはdrill-core 48 tests / drill-wasm 10 tests、`cargo fmt --check`、Clippy `-D warnings`が成功。WebはTypeScript typecheck、ESLint warning 0、関連Vitest 82 testsが成功し、actual wasm-packを再生成した。生成済みWASMの一次方程式(2)を8 seedで測定した平均effortはdifficulty 1..5で`17.57, 21.02, 23.61, 25.82, 28.31`と単調増加した。候補poolは8n、bootstrapはn+4問を選択し、effort最小2問・最大2問をtrimして最終n問とする。

### 2026-08-10 算術8テーマ追加・小学生負数禁止 acceptance

- theme ID 4〜11/revision 1として、一桁の引き算、二桁の足し算、九九、負の数の計算(1)/(2)、分数の足し算、分数の引き算、分数の掛け算を登録する。問題数は前5テーマが20問、分数3テーマが16問。
- `ProblemPrompt::Arithmetic`はRustのtyped `ArithmeticExpression` ASTを保持し、Web MathLiveとPDFは同じASTから表示を投影する。負の数(1)は2〜4整数項の加減、負の数(2)は2〜4整数leafの四則演算ASTで、0除算を拒否し最終値が整数になる候補だけを採用する。
- 一桁引き算は`a=b+c`として`b,c in 1..=9`から生成するため、`1<=a<=18, 1<=b,c<=9`を構成的に保証する。二桁加算は両operandを10〜99、九九は両operandを1〜9とする。九九のeffortは例外的に`BaseTimes`を使わず、正解`c`の`BigNum(c)=log10(c)`だけを持つ。
- 小学生registrationは共通generator境界で負数をfail closedに拒否する。prompt/canonical answerだけでなく、`allow_negative`、`negative`、`plus_minus`入力capabilityも許可しないため、現在の分数3テーマの入力パレットは`fraction`のみを公開する。
- 分数3テーマの有効式domainは有限（加算36、減算36、乗算94）なので、重複あり128候補を集める方式ではなく全候補を1回ずつ直接構築してdifficulty poolとする。これによりWASMでのcoupon-collector型のseed依存遅延を除去し、100ms generation budgetを安定化する。
- 一次方程式(2)の係数に使う非整数分数domainを`linear_fraction_domain()`へ共通化した。小学生の分数加算/減算/乗算はそのうち正の値だけを`positive_linear_fraction_domain()`として共有する。加算/乗算は`a,b,c>0`、減算は`a-b=c`かつ`a,b,c>0`で、いずれも`a,b,c`すべてが正の同一domainに属する候補だけを採用する。
- Recommendedは`足し算と引き算 / 掛け算と割り算 / 分数 / 負の数 / 方程式`の5ジャンルだけを持ち、theme ID 4〜11の8テーマをすべて含む。学年配置はcurriculum.mdに従い、一桁引き算=小1、二桁加算/九九=小2、分数加算/減算=小5、分数乗算=小6、負の数=中1。
- 実Chrome 151 + release生成済みWASMでおすすめ5ジャンルを実listboxから確認済み。今回さらに分数加算/減算/乗算を実routeで生成し、全operandが正、減算は`a>b>0`、採点後canonical answerも全て正、入力パレットは`分数`のみ、Runtime/WASMエラー0を確認した。Seed空欄の冷起動でも分数加算16問を100ms既定設定のまま生成できた。
- 最終検証はRust `cargo fmt --check`、Clippy `-D warnings`、workspace tests（drill-core 52 / drill-wasm 10）、Web full Vitest 112 tests、TypeScript typecheck、ESLint warning 0、Rust→Web contract check、Next production build、`git diff --check`を通過した。最終Rust sourceからrelease WASMも再生成した。

## 10. 既知の制約と次の拡張点

- 実装済みgeneratorはtheme ID 1〜11の11テーマ。未実装の残り学年・テーマ枝だけをDummyで表す。
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
