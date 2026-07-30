# AGENTS.md — AutoDrill

`managed_project_contract: llm-managed-project-v1`

## セッション開始時の必須動線

このrepositoryは`LLM/projects/`内の独立product repositoryである。作業前に必ず次を行う。

1. `../../AGENTS.md`を読み、共通行動規範として適用する
2. `../../ROLES.md`、`../../board/README.md`、自分のrole contract、使用engineのadapterを必要範囲で読む
3. `../../scripts/init-project.sh --check AutoDrill`でbridgeを検証する
4. Git rootとbranchを確認し、material taskだけ中央`../../board/`へproject名付きで記録する

共通の行動・権限・委譲・gate・Git・通知規則は`../../AGENTS.md`が正典である。本ファイルは
このproduct固有の目的、command、architecture、禁止範囲だけを追加する。

## このprojectについて

日本の計算ドリルを決定的に生成・回答・採点・印刷する静的Webアプリ。

## ディレクトリ構成

| パス | 役割 |
|---|---|
| `apps/web/` | Next.js 14のq1設定・q2回答画面、WASM adapter、PDF生成 |
| `crates/drill-core/` | Rustの決定的生成・編集・正規化・採点・努力量計算 |
| `crates/drill-wasm/` | Rust coreをJSON/WASM DTOへ変換する薄い公開境界 |
| `docs/web-pdf.md` | Web/WASM/PDF境界と依存ライセンスの設計メモ |
| `docs/implementation-status.md` | 現在の実装状況・検証証拠・制約・再開手順 |
| `scripts/build-wasm.sh` | Rust/WASM targetを変更せずに行うローカル生成パイプライン |
| `README.md` | project概要、インストール、開発、検証コマンド |

## コマンド

- install: `pnpm install --frozen-lockfile`
- dev: `pnpm dev`
- build: `pnpm build`
- lint: `pnpm lint`（補助: `pre-commit run --all-files`）
- typecheck: `pnpm typecheck`
- web test: `pnpm test`
- Rust test: `cargo test --workspace --all-targets`
- WASM package: `./scripts/build-wasm.sh`（target/toolingが既にある場合だけ）
- bridge check: `../../scripts/init-project.sh --check AutoDrill`

`scripts/build-wasm.sh` は `wasm32-unknown-unknown` と `wasm-bindgen` CLIを
自動インストールしない。初回環境でtargetが無い場合は、スクリプトのエラーを
確認し、開発者が自分のRust toolchainへ前提を用意してから再実行する。

ブラウザ向けWASMを生成した後は、Next.jsのdev/buildで`apps/web/public/wasm/pkg/`
を配信し、`apps/web/src/wasm/load-generated.ts`から動的にロードできる。

## 規約・注意

- Board、Dashboard、Doctor、role contract、provider adapterをこのrepositoryへコピーしない
- 中央環境は`../..`、中央Boardは`../../board/`を使う
- product codeとproduct固有の設定だけをこのrepositoryで管理する
- 認証情報、token、秘密鍵をcommitしない
