# Toolchain architecture

AutoDrill の開発・CI toolchain は、再現性と更新容易性を優先して管理する。

## JavaScript runtime / package manager

- Node.js の project pin は repository root の `.nvmrc` を canonical とする。
- `package.json` の `devEngines.runtime.version` と `engines.node` は `.nvmrc` と一致させる。
- pnpm の version は root `package.json` の `packageManager` を canonical とする。
- `pnpm toolchain:check` は、実行中 Node/pnpm とこれらの宣言が一致することを検証する。
- GitHub Actions は `pnpm/setup` を使用し、root `package.json` の `packageManager` / `devEngines.runtime` を読む。CI workflow に Node/pnpm version を重複して hard-code しない。
- macOS の `AutoDrillを起動.command` は `.nvmrc` の exact version に対応する asdf install を直接使用する。「インストール済みの最新版」を選んではならない。

asdf を使う開発環境では `legacy_version_file = yes` により `.nvmrc` を読ませる。project root に `.tool-versions` を追加して同じ Node version を二重管理しない。

## Rust toolchain

- Rust の project pin は `rust-toolchain.toml` を canonical とする。
- `channel` に current stable patch release を固定し、`rustfmt` / `clippy` と `wasm32-unknown-unknown` target も同じfileで宣言する。
- rustup の directory toolchain discovery / auto-install を利用し、CI workflowへRust versionやtargetを重複hard-codeしない。
- CIは `rustup show active-toolchain` でproject toolchainをmaterializeしてから、WASM補助CLIだけを導入する。

## WASM build tools

- `wasm-pack` のversionは `scripts/wasm-toolchain.sh` の `WASM_PACK_VERSION` をcanonicalとする。
- `wasm-bindgen-cli` はRust crateの `wasm-bindgen` とexact一致が必要なためversionを別途hard-codeせず、`Cargo.lock` から自動導出する。
- `scripts/install-wasm-tools.sh` が必要なCLIを導入し、`scripts/verify-toolchain.sh` / `scripts/build-wasm.sh` がexact versionを検証する。CIも同じhelperを使用し、workflowへversionを重複記述しない。
- `scripts/build-wasm.sh` は出力directoryを毎回cleanにしてから `wasm-pack --no-pack --no-typescript` でbrowser用artifactだけを生成する。WASM出力はnpm packageではなく生成物であり、Git管理しない。

## Next.js generated types

- `apps/web/next-env.d.ts` はNext.js自身が生成するartifactでありGit管理しない。
- typecheckは `next typegen` を先に実行してから `tsc --noEmit` を行う。fresh checkoutやcache削除後でも型検査が成立することを要件とする。
- developmentとproductionで異なる `distDir` を使っても、最後に実行したNext commandによってtracked fileが書き換わる構成を作らない。

## Dependency update policy

「npm 上の latest」よりも、peer dependency が完全に成立する最新の安定組み合わせを優先する。major update は、関連 toolchain 全体の peer range と test suite が通る場合にのみ採用する。

2026-08-21 時点では以下を意図的に current major より手前に固定している。

- `@types/node`: Node 24 runtime と一致させるため 24.x。
- `eslint`: current Next.js dependency tree の `eslint-plugin-import` / `eslint-plugin-jsx-a11y` / `eslint-plugin-react` が ESLint 10 をまだ peer range に含めないため 9.x。
- `typescript`: current TypeScript-ESLint dependency tree が TypeScript 7 をまだ peer range に含めないため 5.9.x。

これらは upstream の peer range が更新された時点で再評価する。peer mismatch を `allowedVersions` 等で握りつぶして先行 major を導入しない。

## pnpm compatibility overrides

`pnpm-workspace.yaml` の override は、理由と削除条件を隣接 comment に記載する。

現在 `@napi-rs/wasm-runtime` を 1.1.6 に固定しているのは、`unrs-resolver 1.12.2` の WASI binding が `@emnapi/core` / `@emnapi/runtime` 1.10.0 を固定する一方、semver で選ばれる wasm-runtime 1.2.0 が emnapi 2 alpha を要求するためである。upstream binding の dependency set が整合したら override を削除する。

Security override は patched version へ固定し、`pnpm audit --prod` で production dependency に既知脆弱性がないことを確認する。
