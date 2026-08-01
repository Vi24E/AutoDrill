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
可逆な問題セットID、単元別URLを実装しています。実際に生成できるテーマは
`一桁の足し算`だけで、未実装枝は`Dummy1`として生成・印刷を無効化します。

問題生成・編集・正規化・採点・努力量計算はRustが所有し、ブラウザはschema-v2 JSON DTOの
WASM境界だけを呼び出します。生成パッケージはGit管理外の
`apps/web/public/wasm/pkg/`へ出力します。

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
