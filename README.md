# AutoDrill

日本の小中学校カリキュラムに沿った計算ドリルを、決定的に生成・回答・採点・PDF出力するWebアプリです。

このprojectは`LLM/projects/`内の独立Git repositoryです。共通agent環境、Board、Dashboard、Environment Doctorは2階層上の`LLM/`から共有し、このrepositoryへコピーしません。

## 現在のバージョン

**AutoDrill alpha 1.2**

alpha 1.2系の現行実装では、通常計算、分数、小数、正負の数、方程式、筆算、「うそつきだれだ」を含む **38個のactive theme** を登録しています。個々の学年配置と教材内容は[`curriculum.md`](curriculum.md)を参照してください。

小学1年生〜中学3年生の全学年に実装済み単元があり、学年選択、`おすすめ`、4段階難易度、Seed再現、単元別URLを備えています。

## セットアップと開発

Node.js は `.nvmrc`、pnpm は root `package.json` の `packageManager`、Rust は `rust-toolchain.toml` の各canonical pinに従って用意し、repository直下で実行します。

```bash
./scripts/install-wasm-tools.sh
pnpm install --frozen-lockfile
pnpm toolchain:check
pnpm dev              # http://localhost:3000
```

macOSではrepository直下の`AutoDrillを起動.command`をダブルクリックしても起動できます。

## Alpha公開

alpha版はGitHub Pagesのproject siteとして `https://vi24e.github.io/AutoDrill/` に公開します。`main`へのpushで`.github/workflows/pages.yml`がRust/WASM・Webの検証、Next.js static export、Pages成果物検証、deployを順に実行します。Pages buildだけ`/AutoDrill`のbase pathを使用し、通常のlocal buildには影響しません。alphaは将来の独自ドメインSEOへ影響を残さないよう`noindex, nofollow`で公開します。

Pages成果物だけを検証する場合は、GitHub Pages用環境変数で`pnpm build`した後に次を実行します。

```bash
pnpm pages:verify
```

主要な検証コマンド:

```bash
pnpm toolchain:check
pnpm lint
pnpm typecheck
pnpm test
pnpm build
pnpm contract:check
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
git diff --check
```

## Rust / WASM boundary

問題生成・正規化・採点・effort計算・MathLive入力の`AnswerNode`化はRustがsource of truthです。Webはgenerated Rust Web contractのcurrent schema（現行v7）JSON DTO経由でWASMを呼び出し、数学的correctnessを再実装しません。

各Problemは`answer_schema`とは独立したtyped `input_interface`を返し、themeごとに許可する数値・構造入力を明示します。Webはこのcapabilityから入力UIを構成し、不正・巨大な入力はRust境界でfail closedに扱います。

```bash
./scripts/build-wasm.sh
```

生成packageはGit管理外の`apps/web/public/wasm/pkg/`へ出力されます。

## Theme architecture

Webの各実装テーマは`apps/web/src/domain/themes/`で**1テーマ1ファイル**として定義します。各definitionがroute、学年・ジャンル、worksheet表示、input capability、Rust compatibility identityをまとめて所有します。`theme-registry.ts`はdefinitionの列挙とlookupだけを担当します。

数学的generator本体とrevision registryはRust側にあり、Web theme definitionへ数学ロジックを持ち込みません。この分離により、テーマ追加時のmetadata同期箇所を局所化しつつ、Rustをsource of truthとして維持します。

## Web / MathLive / Print

Web問題式はMathLive、解答入力は`math-field`を使用します。MathLiveのLaTeX snapshotはRustの`parse_mathlive_answer`を通過して初めて採点可能な`AnswerNode`になります。

印刷/PDFは独自の数式rendererを持ちません。印刷ボタンではまず同じA4 DOMの2ページプレビューを表示し、プレビュー内の「印刷する」からブラウザ標準の印刷/PDFエンジンへ進みます。問題式はWebと同じ`ProblemExpression`、解答は同じ`MathLiveStatic`で組版します。そのため分数・根号・指数などを追加してもPDF専用の座標計算やglyph実装は不要です。日本語fontもWebと同じNoto Sans JPを使用し、`pdf-lib`やPDF専用font bundleはありません。印刷moduleは操作時だけdynamic importします。MathLiveのeditor/static renderer・数式template・問題式rendererはq2用dynamic chunkへ分離し、q1のfirst paintをblockしません。idle時と問題生成開始時にpreloadし、worksheetへ切り替える前にchunk readyを待つため、数式だけ後から表示される遅延を避けます。

設計思想と現行文書の入口は[`docs/README.md`](docs/README.md)です。AIエージェントを含む実装者は、まず[`docs/principles.md`](docs/principles.md)を確認し、対象領域の[`docs/architecture/`](docs/architecture/)を参照してください。未解決事項は[`docs/issues.md`](docs/issues.md)、support scopeと将来計画は[`docs/roadmap.md`](docs/roadmap.md)で管理します。
