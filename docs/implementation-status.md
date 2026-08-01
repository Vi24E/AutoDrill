# AutoDrill alpha 1.1 実装状況

更新日: 2026-08-02

この文書は、現在の作業ツリーを後続の開発者が再現・調査・拡張するための実装記録です。製品仕様の正典はリポジトリ直下の[`goal.txt`](../goal.txt)、学習内容とeffort定義は[`curriculum.md`](../curriculum.md)です。公開境界の詳細は[`problem-schema.md`](problem-schema.md)、[`answer-ast.md`](answer-ast.md)、[`effort-model.md`](effort-model.md)、Web/PDFの責務は[`web-pdf.md`](web-pdf.md)を参照してください。

## 1. 現在の到達点

alpha 1.0のq1設定、q2回答・採点、q3別タブPDFを維持しながら、alpha 1.1のカリキュラム選択、難易度生成、共通問題形式、共通解答AST、努力量計算、単元URLを実装しました。

- q1上部に`おすすめ`（既定）と`学年から選ぶ`の切替を持つ。
- `おすすめ`は`ジャンル → テーマ`の2段階、`学年から選ぶ`は`学年 → ジャンル → テーマ`の3段階select。
- 学年は小学1年生から中学3年生までで、slugは`grade-1`から`grade-9`。中1・中2・中3は`grade-7`・`grade-8`・`grade-9`。
- 実装済みテーマはnumeric theme ID 1の`一桁の足し算`。おすすめと学年選択は同じregistry object・generatorを参照する。
- 各学年の未実装枝は`Dummy1`で表し、選択中は生成・印刷を無効化して`このテーマはまだ利用できません`と表示する。DummyはURL、metadata、sitemapへ出さない。
- 難易度1〜5を実装し、既定3。UI、生成request、問題セットIDは同じ値を使う。
- 一桁足し算は候補5n件、2n種類の多様性条件、effort order statistic、最終重複排除、決定的shuffleで固定20問を生成する。
- 共通`AnswerNode`、標準解法`SolutionGraph`、固定27成分`OperationVector`、分離された重みlayerをRust coreに実装した。
- 問題セットIDは`{schema_version}-{numeric_theme_id}-{generator_revision}-{seed}-{difficulty}`。例は`2-1-2-Ab3Z-3`。
- 実装済み単元URLは`/drills/grade-1/one-digit-addition`。`/`はTOPのまま。
- Rust/WASM境界はschema version 2。TypeScript側に数学規則のfallbackを置かない。

## 2. 画面と遷移

| 状態 | URL/入口 | 表示・操作 | 遷移 |
|---|---|---|---|
| q1 設定 | `/`または実装済み単元URL | ふりがな、選択モード、カリキュラムselect、難易度、Seed、問題生成、印刷 | 生成→q2、印刷→q3 |
| q2 回答 | q1で生成 | 2列×10行、固定リボン、固定10キー、物理キー、タイマー、採点、印刷 | 印刷→q3、TOP→q1 |
| q2 採点後 | q2で採点 | 誤答赤枠、赤字正答、問題に戻る、同一問題再挑戦、別Seed生成 | 各操作に応じq2へ |
| q3 印刷 | q1またはq2の印刷 | Blob URLの2ページPDF | ブラウザのタブ操作 |

q2は共有A4 geometryを使い、左列を1〜10、右列を11〜20の縦順で表示します。回答欄選択時だけ画面下の10キーを表示し、リボンと10キーはviewport固定です。物理数字、Backspace、Delete、左右矢印、Enterを受け付け、Enterは確定して次問へ移動し、同列の1行分を自動スクロールします。

回答は18 AST-sizeまでです。選択中は実カーソルを表示し、11pxを下限に回答枠を右方向へ広げます。上限を超える入力は状態を変更せず、リボン付近へ`式が大きすぎます！`を表示します。

採点ボタンは入力FIFOを排出してから最新回答を読み、タイマーを停止します。数学的に正解でも表記に冗長性がある場合は、正解扱いのまま問題横へ`約分`、`冗長なマイナス`、`余計な小数点`のwarningを複数表示できます。`問題に戻る`は回答を保持してタイマーを再開、`もう一回問題を解く`は同じ問題セットで回答と時間を初期化、`別の問題を解く`は同じテーマ・難易度の新しい自動Seedを生成します。

## 3. カリキュラムregistry

Webの正規registryは`apps/web/src/domain/curriculum.ts`です。

- `CURRICULUM_TREE`: 9学年のcanonical tree。
- `RECOMMENDED_GENRES`: canonical treeへの参照だけを持つ部分集合。
- `IMPLEMENTED_THEMES`: metadata、static params、sitemapへ公開してよいテーマだけ。
- `ONE_DIGIT_ADDITION_THEME`: theme ID 1、revision 2、20問、2列×10行、単元URLを所有。
- Dummy theme: `implemented:false`、`route/search/layout/problemCount`は`null`。

Rustの生成registryは`crates/drill-core/src/registry.rs`です。numeric theme IDとgenerator revisionから、skill ID、curriculum path、問題数、列・行、テーマ別重みoverrideを復元します。問題数はrequestや問題セットIDに重複保存しません。

## 4. 問題生成と再現

`crates/drill-core/src/generator.rs`の`ProblemGenerator`が共通interfaceです。現行registryには`OneDigitAdditionGenerator` revision 2だけがあります。

一桁足し算の生成手順は次の通りです。

1. n=20に対し、左右1〜9の順序付き組を重複ありで5n=100件生成する。
2. 異なる式が2n=40種類未満ならpool全体を捨て、新しい100件を生成する。
3. `effort → 順序付き式 → 生成ordinal`で決定的にsortする。
4. k問目の残りpool `[1,5n-k+1]`から独立な一様乱数を5個生成する。
5. 難易度iは5個のi番目に小さい値が指す候補を選ぶ。
6. 最終採用済み式と重複した場合は、poolを変えず同じkで再抽選する。
7. 採用時だけpoolから候補を除き、20問後に同じRNG streamでFisher-Yates shuffleする。

既定deadlineは100ms、attempt上限は10,000です。候補生成、pool再生成、重複再抽選は同じbudgetを消費し、`generation_timeout`と`generation_attempt_limit`を別エラーで返します。

Seedは1〜16文字で、`1-9`、`a-z`、`A-Z`から`I`、`l`、`O`を除いた集合です。q1空欄時だけWebが同じalphabetの4文字Seedを生成します。ID `2-1-2-Ab3Z-3`はschema 2、theme 1、revision 2、Seed `Ab3Z`、難易度3へ可逆decodeでき、保存済みrevisionから同じ20問を再生成できます。

## 5. 共通問題・解答境界

Worksheetは次を持ちます。

- schema versionとproblem-set ID
- decode済み`ProblemSetIdentity`
- registry由来のskill ID、curriculum path、layout
- 20個の`Problem`

Problemはschema version、通し番号、numeric theme ID、typed prompt、answer schema、canonical AnswerNode、SolutionGraph、27成分OperationVector、解決済みeffortを持ちます。

`AnswerNode`は次のvariantを定義済みです。

- empty
- integer
- exact decimal（十進coefficientとscale）
- fraction
- mixed fraction
- root
- negative
- plus-minus
- tuple
- variable

数学値はbinary floating-pointへ変換しません。Rust内部のinteger/coefficient/answer-schema limitは`i64`、BigNum magnitudeは`u64`ですが、JSON/WASMではcanonical decimal stringとしてserializeします。例えば18桁`999999999999999999`は文字列のままWebへ渡り、JavaScript `number`を経由しません。Floatを許可するのは`log10`で得るoperation quantityと最終effortだけです。採点時は整数・有限小数・分数・帯分数・negativeを既約有理数へexact変換するため、`2/4 = 0.5 = 1/2`、`4.0 = 4`として比較します。

`AnswerNode::size()`が全variant共通の構造サイズ契約です。integerは十進桁数、compositeは親1と全childの合計で、`frac(num(12),num(42))`は5です。入力/display treeと採点用normalized treeは`AnswerRepresentation`で分離できます。

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

`apps/web/src/domain/wasm-adapter.ts`が唯一の本番JSON境界です。schema、identityとIDの一致、theme/revision、20問layout、問題ID一意性、typed prompt/answer、27成分vector、exact decimal stringをfail-closedで検証します。

公開WASM exportは次の7個です。

| export | 役割 |
|---|---|
| `generate_problem` | registryから1問生成 |
| `generate_worksheet` | 難易度付き20問生成 |
| `regenerate_problem_set` | 可逆IDから再生成 |
| `apply_editor_action` | AnswerNodeとcursorを編集 |
| `normalize_answer` | canonical treeを返す |
| `grade_answer` | typed AnswerNodeを採点 |
| `calculate_effort` | graph/vector/重みからeffort算出 |

Webは画面状態、入力FIFO、タイマー、表示、PDF描画だけを担当します。生成規則、正規化、採点、effort計算をTypeScriptへ複製しません。

PDFは`apps/web/src/domain/layout.ts`のA4 geometryをq2と共有します。1ページ目は問題と空枠、2ページ目は解答で180度回転し、両ページ右下に小さく生成日とSeedを記録します。q1/q2は同じ`openWorksheetPdf`を使い、q1は非同期生成前に空タブを開いてpopup blockを避けます。

## 8. 主なファイル

```text
apps/web/src/domain/curriculum.ts       # 9学年、recommended subset、Dummy、route metadata
apps/web/src/domain/drill-engine.ts     # schema-v2 TypeScript DTO/interface
apps/web/src/domain/wasm-adapter.ts     # strict JSON/WASM adapter
apps/web/src/components/AutoDrillApp.tsx# q1/q2 UIと状態遷移
apps/web/src/pdf/worksheet-pdf.ts       # 共有layoutから2ページPDF
crates/drill-core/src/answer.rs         # typed exact AnswerNodeとsize
crates/drill-core/src/exact.rs          # i64/u64 canonical decimal JSON serde
crates/drill-core/src/generator.rs      # registry generatorと5n difficulty sampling
crates/drill-core/src/identity.rs       # Seed、difficulty、可逆problem-set ID
crates/drill-core/src/registry.rs       # revision付きtheme/generator登録
crates/drill-core/src/effort.rs         # graph、27-vector、weights、negative/carry semantics
crates/drill-wasm/src/lib.rs            # 薄いwasm-bindgen JSON envelope
```

## 9. 検証証拠（2026-08-02）

- repository bridge: healthy。
- Rust: `cargo fmt --all -- --check`、`cargo check --workspace --all-targets`、Clippy warning-as-error、`cargo test --workspace --all-targets`成功。core 20件、WASM 5件。
- Web: lint、typecheck、62 tests、production build成功。
- Next output分離: clean状態からdev serverを起動し、production build中に44回assetを継続取得し、build後のproduction server配信まで成功。
- static build: `/`、`/drills/grade-1/one-digit-addition`、`/sitemap.xml`を生成。Dummy routeは生成しない。
- actual wasm-bindgen/Node: `2-1-2-Ab3Z-3`、20式一意、canonical answer/answer schema/BigNumのdecimal string、18桁`999999999999999999`の完全往復を確認。約分・通常/分数内の冗長なマイナス・余計な小数点warningも生成済みWASM実体で確認。
- 実ブラウザ/q1: おすすめ2段select、学年3段select、grade-1〜9、中1=`grade-7`、Dummy選択時の警告と生成・印刷無効、難易度1〜5・既定3を確認。
- 実ブラウザ/q2: 左1〜10・右11〜20、初期10キー非表示、選択後の固定10キー、物理数字入力、Enterで次問と83pxスクロール、固定リボンを確認。
- 実ブラウザ/採点: 20誤答枠、20正答表示、3つの採点後操作、タイマー停止を確認。
- 実ブラウザ/q3: q2印刷が別Blob PDFタブを開くことを確認。ブラウザ安全制限によりBlob内容の取得・撮影は拒否されたため、今回の最新差分ではPDF bytes/geometryの自動テスト4件を受入証拠とした。alpha 1.0時点では同一layoutの2ページをPoppler描画し目視確認済み。

## 10. 既知の制約と次の拡張点

- 実装済みgeneratorは一桁足し算だけ。ほかの学年・テーマは意図的にDummy。
- `IMPLEMENTED_THEMES`追加時は、Web registry、Rust revision registry、URL metadata/sitemap、property testを同時に更新する。
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
