# AutoDrill alpha 1.0 実装状況

更新日: 2026-07-30

この文書は、現在の作業ツリーを後続の開発者が再現・再開するための実装記録です。
製品の外部仕様はリポジトリ直下の [`goal.txt`](../goal.txt) を正典とし、Web/PDFの
境界メモは [`web-pdf.md`](web-pdf.md) にまとめています。この文書は実装と検証の
状態を記録するもので、未検証のWASMブラウザ動作を成功扱いにしません。

## 1. 現在の到達点

- q1（設定）・q2（回答）・q3（別タブのPDFビューアー）の画面遷移をNext.jsで実装済み。
- q1はゲームロビーを着想源にした中央カード、控えめなグリッド・幾何学背景、押下感のある操作ボタンで構成する。カリキュラム選択は型付きの木データから`学年 → 領域 → 単元`の3連動selectへ投影し、q2の白紙調ワークシートには装飾を持ち込まない。
- alpha 1.0の一桁足し算は、1〜9の順序付き組から重複なしで20問を決定的に生成する。
- q1だけにSeed入力欄を置き、空欄は押下ごとにcrypto優先の4文字自動Seed、非空欄は入力文字列そのものを使う。正しい手入力は許可文字（`1-9`、`a-z`、`A-Z`から`I`/`l`/`O`を除外）の1〜16文字で、同じ指定Seedは同じ20問を再現する。
- 実際に使ったSeedとローカル生成日（`YYYY-MM-DD`）は`WorksheetMetadata`としてUIとPDFへ引き継ぎ、q2とq3の各問題ページ右下へ小さく表示する。q2にはSeed入力欄を表示しない。
- q2は`src/domain/layout.ts`の共有A4レイアウト（余白・上部/下部予約領域・2列×10行）をCSSの百分率座標へ変換して表示する。上部リボンと、回答欄クリック後に現れる通常10キー順の入力パネルをviewportへ固定する。Enter確定後は次問を選択してパネルを継続し、最終行も固定パネルの上までスクロールできる。
- 数字キー、物理キーボードの数字、Backspace/Delete/左右矢印、Enter（確定して次問へ）を受け付ける。
- WASMの非同期編集呼出しはFIFOキューと最新の回答refで直列化し、連続した数字入力とEnterの順序を保持する。
- 採点開始時は表示時間を押下時刻で凍結し、同じFIFOキューを排出してから最新の回答refを読む。誤答・未回答は`#e01010`の太枠と正答を表示する。
- 選択中の回答は`integer` ASTの桁配列として表示し、採点は全20問をRust/WASMへ1問ずつ委譲する。
- 回答ASTはRustの`AnswerNode::size()`を唯一の構造サイズ契約とし、整数は十進数字1文字をsize 1、空回答を0として数える。alpha 1.0の上限は18で、19桁目の入力は状態を変えず`answer_ast_size_limit`としてWebへ返す。
- q1の「印刷」とq2の印刷アイコンは同じ`openWorksheetPdf`パイプラインを使う。q1では非同期処理前に空タブを開き、ポップアップ阻止を避ける。
- PDFは問題ページと解答ページの2ページを生成し、解答ページを180度回転する。
- Rust nativeテスト、Web lint/typecheck/test/buildは現行ソースで成功している。
- Rust native crateに加え、`wasm32-unknown-unknown`向け生成とブラウザでの実WASMロードを検証済み。WASMの時計はwasm32では`performance.now()`由来の`BrowserClock`、nativeでは`SystemClock`を使い分ける。

## 2. 画面・状態と責務

| 状態 | 入口 | 主な表示・操作 | 次の状態 |
|---|---|---|---|
| q1 設定 | 初期表示、q2のTOPに戻る | 学年・領域・単元の3連動select、難易度placeholder、Seed入力、問題生成、印刷 | 問題生成→q2、印刷→q3 |
| q2 回答 | q1の問題生成 | 20問、2列×10行、固定リボン、回答時間、生成日/Seedフッター、初期は非表示の固定10キー（欄クリックで表示）、採点、印刷、TOPに戻る | 印刷→q3、TOPに戻る→q1 |
| q3 PDF | q1またはq2の印刷 | ブラウザが表示する実PDF（問題ページ＋180度回転した解答ページ） | ブラウザのタブ操作に委ねる |

製品仕様上の「生成・編集・正規化・採点・努力量計算」はRustが所有します。React/TypeScriptは画面状態、入力イベントの順序付け、表示、PDF描画だけを担当し、正しさや生成規則を再実装しません。

## 3. ディレクトリとモジュール

```text
AutoDrill/
├── apps/web/
│   ├── src/app/                 # Next.jsのページ、global CSS
│   ├── src/components/          # q1/q2とタイマー・入力イベント
│   ├── src/domain/drill-engine.ts# TS側のversioned DTOとengine interface
│   ├── src/domain/curriculum.ts  # 学年・領域・単元の型付きカリキュラム木
│   ├── src/domain/wasm-adapter.ts# JSON envelopeの唯一の本番境界
│   ├── src/domain/layout.ts      # A4の余白・上部予約領域と2×10の共有レイアウト（Web/PDF共用）
│   ├── src/domain/seed.ts        # 許可alphabetの4文字自動Seedと注入可能なfallback
│   ├── src/domain/worksheet-metadata.ts # 生成日/実SeedのUI・PDFメタデータ
│   ├── src/pdf/worksheet-pdf.ts  # pdf-libによる2ページ生成・Blob遷移
│   ├── src/wasm/load-generated.ts # 生成済みwasm-pack web packageの動的import seam
│   └── public/wasm/pkg/          # build時だけ生成するGit管理外のWASM成果物
├── crates/drill-core/src/
│   ├── generator.rs              # seed、重複、100ms、試行上限、Worksheet
│   ├── model.rs                  # AnswerNode、Problem、Worksheet、DTO向け型
│   ├── editor.rs                 # editor actionとInteger ASTの更新
│   ├── normalize.rs              # canonical answer拡張点
│   ├── grade.rs                  # correct/incorrect/unanswered
│   ├── effort.rs                 # operation_countsとeffort
│   ├── error.rs                  # typed generation/editor errors
│   └── rng.rs                    # seedからの再現可能SplitMix64
├── crates/drill-wasm/src/lib.rs  # wasm-bindgen関数、JSON DTO、envelope変換
├── scripts/build-wasm.sh         # target/toolingを自動変更しないWASM生成script
├── docs/web-pdf.md               # Web/PDF境界、依存、ライセンス
├── goal.txt                      # 製品仕様の正典
└── AGENTS.md / README.md         # 開発・検証コマンドと運用注意
```

## 4. Rust/WASM APIとDTO

### 共通 envelope

すべての公開関数は、JSON文字列を受け取り、次の形のJSON文字列を返します。

```json
{
  "schema_version": 1,
  "ok": true,
  "data": {},
  "error": null
}
```

失敗時は`ok:false`、`data:null`、`error:{code,message,details?}`です。schema versionが異なる、JSONが壊れている、DTOの型が合わない場合は`invalid_request`または`unsupported_schema_version`になります。

### 必須export

`crates/drill-wasm`のwasm-bindgen exportは次の6個です。

| export | 入力の要点 | 成功データ |
|---|---|---|
| `generate_problem` | `{schema_version,seed}` | 一つのProblem DTO |
| `generate_worksheet` | `{schema_version,seed,problem_count?,timeout_ms?,max_attempts?}` | 固定メタデータを含むWorksheet DTO |
| `apply_editor_action` | `{schema_version,state,action}` | `EditorState` |
| `normalize_answer` | `{schema_version,answer}`（raw answerの互換入力も受付） | canonical answer |
| `grade_answer` | `{schema_version,expected,actual}` | `{schema_version,status,is_correct,expected,actual}` |
| `calculate_effort` | `{schema_version,problem,weights?}` | `{value,operation_counts}` |

Problem DTOの意味上のフィールドは`problem_id`、`prompt:{kind,left,right}`、`answer_schema`、`canonical_answer`、`operation_counts`です。alpha 1.0では`prompt.kind`と`answer_schema.kind`は`addition`/`integer`、答えの範囲は1〜18です。Worksheet DTOは`skill_id`、`curriculum_path`、`generator_version`、文字列seed、`layout:{problem_count:20,columns:2,rows:10}`、20個のProblem DTOを持ちます。

Rust Worksheet DTOの`seed`は生成器へ渡した文字列を保持します。Web側はこれに加えて`WorksheetMetadata`（`generated_date`と同じ実Seed）を生成時に作り、DTOを変更せずq2表示とq1/q2共通のPDF呼出しへ渡します。q1のSeed入力が空なら許可alphabetから4文字自動Seedを解決してからRustへ渡すため、fixture/runtimeが返すseedが異なってもUIとフッターは実際の使用値を示します。正しい手入力の1〜16文字はそのまま渡します。17文字以上・許可外文字の入力拒否、sanitize、エラー表示、ボタン無効化は仕様未決定のため未実装です。

EditorStateのWeb境界表現は次の通りです。

```json
{
  "schema_version": 1,
  "node": {"kind": "integer", "digits": [1, 2]},
  "cursor": 2,
  "committed": false
}
```

`grade_answer`には期待値として`problem.canonical_answer`、実値として`editorState.node`だけを渡します。Web adapterは20問分の呼出順序と集計だけを担当します。

### 生成エラー

生成開始から既定100msを超えた場合と、既定最大10,000回の試行を使い切った場合を別形式で返します。

| 状況 | `error.code` | `details` |
|---|---|---|
| タイムアウト | `generation_timeout` | `{timeout_ms}` |
| 試行上限 | `generation_attempt_limit` | `{attempts,max_attempts}` |
| 81問を超える要求 | `invalid_problem_count` | `{requested}` |

TypeScriptの`DrillEngineError.kind`も`generation_timeout`と`generation_attempt_limit`を区別し、画面文言を分けています。

### 回答AST sizeと編集エラー

`MAX_ANSWER_AST_SIZE`は18です。現在の整数ASTでは十進桁数がそのままsizeになり、将来の複合ノードは親ノード1と子ノードのsizeを合算します。したがって将来の`frac(num(12), num(42))`は5として扱う契約です。18桁の整数へさらに数字を挿入すると、Rust editorは入力前のimmutable stateを変更せず、WASM envelopeで`answer_ast_size_limit`と`details:{max_size:18}`を返します。外部から直接渡されたsize超過stateは通常の編集操作ではないため`invalid_request`です。

## 5. PDF戦略と依存・ライセンス

`src/domain/layout.ts`がA4（595.28×841.89pt）の余白・上部予約領域と2列×10行のセル座標を計算し、Web表示はそのtop-origin座標をページ比率へ変換、PDFは同じセルをbottom-originへ変換して描画します。したがってq2の順序・行位置・中央区切り・紙面寸法に独立したCSS grid定義はありません。入力イベントは`AutoDrillApp`内のFIFO action queueで直列化し、遅延するWASMでも回答桁と確定の順序を維持します。採点もqueueのtailをawaitしてから`answersRef.current`をスナップショットするため、入力直後の採点がReactの古いstateを参照しません。`seed.ts`はWeb Crypto `getRandomValues`を優先し、58文字alphabetのrejection samplingで4文字を作り、テスト注入可能なdistinct fallbackも同じalphabet/長さを守ります。`WorksheetMetadata`はRust DTOを変更せず、q1で解決した実Seedとローカル生成日をq2表示・q1再訪時の前回表示・PDFへ渡します。紙面には中央の縦区切りだけを描き、問題ページの右下へフッターを描きます。解答ページは180度回転後も物理右下かつ正立で読めるよう、回転ページでは上端の未回転座標へフッターを置き、文字自体を180度逆回転してから描きます。`pdf-lib`でクライアント内にPDF bytesを生成し、Blob URLをq3タブへ設定します。問題ページには問題番号と空の回答枠、解答ページには問題番号と答えを描き、後者を`180°`回転します。PDFは標準Helveticaだけを使うため、実行時のフォント取得やネットワークサービスはありません。

主要依存は次のライセンスです（正確な推移依存一覧はpnpm lockfileと各パッケージのlicense metadataを参照）。

- Next.js、React、Vitest、Testing Library: MIT
- TypeScript: Apache-2.0
- `pdf-lib`: MIT
- `@fontsource/noto-sans-jp`: SIL Open Font License 1.1
- Rust cratesのworkspace license宣言: MIT（`Cargo.toml`）。依存crateの個別条件はCargo metadataで確認する。

## 6. 再現コマンド

```bash
../../scripts/init-project.sh --check AutoDrill
pnpm install --frozen-lockfile
pnpm dev
pnpm lint
pnpm typecheck
pnpm test
pnpm build
cargo test --workspace --all-targets
pre-commit run --all-files
```

`next dev`は`.next-dev`、`pnpm build`と`next start`は`.next`を出力先に使います。開発サーバーを起動したまま本番buildを実行しても、開発中タブが参照するchunkを上書きしません。

WASMパッケージを生成する場合は、targetとmatching CLIが既に存在する環境で次を実行します。

```bash
./scripts/build-wasm.sh
```

このscriptは`rustup target add`、`cargo install`、`wasm-pack`の自動installを行いません。生成物は`apps/web/public/wasm/pkg/`へ置かれ、クライアント側で`load-generated.ts`を通じて自動ロードされます。生成物が無い場合は、q1のエラー欄にWASMパッケージ生成を促す日本語メッセージを表示します。

## 7. 検証証拠（2026-07-30）

この統合作業で確認できた結果は次の通りです。

- `../../scripts/init-project.sh --check AutoDrill`: `managed project: healthy`。
- 共通role/runtime検証: `verify_roles.py`で189項目、Codex commander・`gpt-5.6-luna`・effort `max`・実効permission `disabled`を検証。adapterのworkspace-write意図との差は警告として出るが、より制限の強い実効経路を採用。
- `cargo test --workspace --all-targets`: drill-core 9 tests、drill-wasm 5 tests、計14 tests passed。AST size 0/1/18、19桁目の型付き拒否、削除後の再入力、WASM error code/detailsを含む。`cargo fmt --all --check`と全targetのClippy（warningをerror扱い）もpassed。
- 実WASM時計修正: Rust/Cargo 1.97.1、`wasm32-unknown-unknown` target、`wasm-pack 0.13.1`、`wasm-bindgen 0.2.126`で`./scripts/build-wasm.sh`を実行して`apps/web/public/wasm/pkg/`を生成した。wasm32経路は`performance.now()`を使う`BrowserClock`へ切り替え、throw・non-finite値・時間の逆行・初回読み取り失敗をfailed latchし、いずれも生成側で`generation_timeout` envelopeへ変換する（Reviewer re-review accepted）。生成runtimeのブラウザロードとq1→q2生成成功を確認し、`std::time::Instant`由来の時計panicを再現しないことを確認した。生成物はGit管理外。
- 手動実ブラウザ確認（localhost dev、生成WASM）: q1→q2生成、回答欄クリック後の固定10キー、物理数字キー入力とEnterでの次問移動を確認した。18桁入力は式へ侵入せず右へ伸び、19桁目は状態を変えずリボン付近に`式が大きすぎます！`を表示した。最終20問目へスクロールしても上下の固定UIを操作できた。採点押下後に`00:27`が1秒以上変化しないこと、誤答・未回答枠が赤い太枠になり右隣へ赤い正答を表示することを確認した。q1/q2印刷は同じBlob PDF処理を使用する。
- `pnpm lint`: ESLint passed（warning/errorなし）。
- `pnpm typecheck`: TypeScript passed。
- `pnpm test`: 5 test files、32 tests passed（3連動select、固定10キー順、物理編集キー、遅延engineのEnter直後入力先、18桁表示/19桁目通知、採点時刻凍結、誤答枠/正答、PDF回答枠位置、Seed、共有A4レイアウト、PDF回転・フッター、Next phase別出力を含む）。
- `pnpm build`: Next.js static production build passed。
- phase-safe出力分離: fresh `next dev`でHTML参照assetを確認し、production build前後とも`layout.css`、`webpack.js`、`main-app.js`、`app-pages-internals.js`、`app/page.js`がHTTP 200だった。build後の再読込でもq1の問題生成がq2へ遷移した。
- PDF visual QA: 最新bytesをA4・2ページとして`pdfinfo`で検査し、Popplerで両ページをPNG化した。問題ページは回答枠が等号の6pt後へ揃い、解答ページは180度回転、中央区切り・番号・フッターに欠けや重なりがないことを目視確認した。
- `pre-commit run --all-files`: managed-project hookは対象ファイルなしでskip（失敗なし）。

## 8. 未検証・既知の制約

- q1ボタンが無反応に見えた事象は、`next dev`稼働中に`pnpm build`が共有`.next`を上書きし、既存タブのHTMLが参照する`/_next/static/css/app/layout.css`、`main-app.js`、`app-pages-internals.js`が404になってhydrationとWASMロードが失敗したことが原因だった。`apps/web/next.config.mjs`をphase引数を受けるNext config関数にし、公式の`PHASE_DEVELOPMENT_SERVER`と一致する場合だけ開発`.next-dev`、それ以外（production build/server）は`.next`へ分離した。`next build`/`next start`が開発成果物を触らないようにし、両方の出力先は`.gitignore`へ登録、phase定数の選択は決定的config testで固定している。
- `load-generated.ts`は生成パッケージを動的にimportするseamです。生成物がない通常checkoutでは、q1にWASMパッケージ生成を促すエラーを表示します。現在の受入環境では生成済みruntimeのロードとq1→q2生成まで確認済みです。Web testsはfixture engineを明示注入して決定的UI回帰を検証し、製品のTypeScript fallbackで数学計算を行いません。
- q1/q2からBlob PDFタブが作られること、最新PDF bytesのA4 2ページ・問題ページ・180度解答ページは確認済みです。実プリンター固有の余白補正と両面印刷時の用紙向きは未確認です。
- q2は共有モデルに基づく百分率配置を使い、受入ブラウザでは固定UI・最終行・採点表示まで確認済みです。別ブラウザ・別OSフォントでの見た目は未確認です。
- 仕様どおりのSeed alphabetはASCIIかつ短いためPDFフッターは標準Helveticaで描画できます。許可外文字や17文字以上のSeedを入力した場合は、入力拒否・sanitize・エラー・ボタン無効化をまだ定めておらず、非WinAnsi文字ではPDF生成が失敗し得て、長Seedでは右下からoverflowし得ます。文字種/長さのUI挙動はUser確認待ちです。
- alpha 1.0では学年・領域・単元selectの木構造と初期値を実装済みですが、選べる枝は`小学1年生 → 数と計算 → 1けたのたしざん(1)`だけです。難易度selectも将来拡張用placeholderで、実装上は固定の一桁足し算設定を使います。負数・分数・複数演算のASTは未対応です。
- 依存crateの全transitive license監査、実ブラウザでのWASM性能の広範な実機測定（生成成功とclock panic回避は確認済み）、配布用CSP/HTTP headerは未実施です。

## 9. 次の作業候補

1. Rust出力JSONをfixtureではなく生成package経由でadapterへ渡す契約テストを追加し、Problem/Worksheet/Editor/Gradeの全フィールドを検査する。
2. 実プリンターまたはOS印刷ダイアログで両面印刷時の用紙向きを確認し、必要なら共有layoutの寸法だけを修正する。
3. 将来カリキュラム木を拡張するときは`skill_id`・`curriculum_path`・分野別layoutの公開契約を先に更新し、TypeScriptへ数学規則を複製しない。
