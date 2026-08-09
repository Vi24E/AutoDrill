# AutoDrill

日本の計算ドリルを決定的に生成・回答・採点・印刷する静的Webアプリ。

このprojectは`LLM/projects/`内の独立Git repositoryです。共通agent環境、Board、Dashboard、
Environment Doctorは2階層上の`LLM/`から共有し、このrepositoryへコピーしません。

作業開始時の確認:

```bash
../../scripts/init-project.sh --check AutoDrill
git status --short --branch
```

## セットアップと開発

Node.js（pnpm 11.9.0）とRust stableを用意し、リポジトリ直下で実行します。

```bash
pnpm install --frozen-lockfile
pnpm dev              # http://localhost:3000
```

macOSではrepository直下の`AutoDrillを起動.command`をダブルクリックしても起動できます。


検証コマンド:

```bash
pnpm lint
pnpm typecheck
pnpm test
pnpm build
cargo test --workspace --all-targets
pre-commit run --all-files
```

## WASMパッケージ

alpha 1.1では小学1年生から中学3年生までの学年選択、`おすすめ`、5段階難易度、
可逆な問題セットID、単元別URL、構造化解答入力を実装しています。解答欄では
分数、帯分数、厳密小数、平方根、マイナス、plusminus、複数解tupleを入力できます。現在の実装済みテーマは
`一桁の足し算`、中1の`一次方程式(1)`、`一次方程式(2)`です。未実装枝は`Dummy1`として生成・印刷を無効化します。

一次方程式は16問・2列8行で、式をセル左上、`x = [ ]`を右下へ配置します。(1)は整数係数・整数解、
(2)は制約付き分数係数・有理解を含み、どちらも全構造を表示するrich keyboardを使います。
Rust側は`ProblemGenerator` traitとrevision registry、Web側は`theme-registry.ts`とtyped prompt formatterを
共通拡張点とし、テーマ追加時に画面・WASM境界へtheme ID分岐を増殖させない設計です。

問題生成・編集・正規化・採点・努力量計算はRustが所有し、ブラウザはschema-v3 JSON DTOの
WASM境界だけを呼び出します。生成パッケージはGit管理外の
`apps/web/public/wasm/pkg/`へ出力します。

各Problemは`answer_schema`とは独立したtyped `input_interface`を返します。現在の一桁の足し算は
`simple_numeric`（digits-only keypad）で、構造テンプレート、小数点、負数キーは表示しません。
`simple_numeric.allow_negative`が有効なテーマでは、物理`-`と画面上のマイナスcontrolを同じ許可へ投影し、
無効な場合は両方を拒否・非表示にします。構造化テーマでは`structured_math.allowed_structures`だけを数式パレットと物理キーへ投影します。
解析できない入力は`nan_error`としてraw textを保持し、数値へ変換せず表示・編集・採点境界へ渡します。

```bash
./scripts/build-wasm.sh
```

このスクリプトは`wasm32-unknown-unknown` Rust targetや`wasm-bindgen` CLIを
自動インストールせず、`wasm-pack --mode no-install`で事前条件を検査します。
受入環境には`wasm32-unknown-unknown`、`wasm-pack 0.13.1`、`wasm-bindgen 0.2.126`
が用意され、実WASM生成とブラウザロード（wasm32時計は`performance.now()`）を
検証済みです。別環境でtargetが無い場合は、開発者が自身のtoolchainへ前提を
用意した後に再実行してください。
生成後のロードseamは`apps/web/src/wasm/load-generated.ts`、JSON境界の説明は
[`docs/web-pdf.md`](docs/web-pdf.md)、詳細な引き継ぎ記録は
[`docs/implementation-status.md`](docs/implementation-status.md)を参照してください。
