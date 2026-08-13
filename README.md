# AutoDrill

日本の小中学校カリキュラムに沿った計算ドリルを、決定的に生成・回答・採点・PDF出力するWebアプリです。

このprojectは`LLM/projects/`内の独立Git repositoryです。共通agent環境、Board、Dashboard、Environment Doctorは2階層上の`LLM/`から共有し、このrepositoryへコピーしません。

## 現在のバージョン

**AutoDrill alpha 1.2**

alpha 1.2では、alpha 1.1以降に追加した多数のテーマとMathLiveによる構造化数式入力を正式な現行構成として扱います。実装済みテーマは19個です。

- 一桁の足し算
- 一桁の引き算
- 二桁の足し算
- 九九
- 割り算(1)
- 小数の足し算と引き算
- 小数の掛け算と割り算
- 分数の足し算
- 分数の引き算
- 分数の掛け算
- 分数の割り算
- 負の数の計算(1)
- 負の数の計算(2)
- 一次方程式(1)
- 一次方程式(2)
- 連立方程式(1)
- 二次方程式(1)
- 二次方程式(2)
- 二次方程式(3)

小学1年生〜中学3年生の全学年に実装済み単元があり、学年選択、`おすすめ`、5段階難易度、Seed再現、単元別URLを備えています。

## セットアップと開発

Node.js **20.9以上**（`.nvmrc`は22.14.0）、pnpm 10.20.0、Rust stableを用意し、repository直下で実行します。

```bash
pnpm install --frozen-lockfile
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
pnpm lint
pnpm typecheck
pnpm test
pnpm build
pnpm contract:check
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
git diff --check
```

## Rust / WASM boundary

問題生成・正規化・採点・effort計算・MathLive入力の`AnswerNode`化はRustがsource of truthです。Webはschema-v3 JSON DTO経由でWASMを呼び出し、数学的correctnessを再実装しません。

各Problemは`answer_schema`とは独立したtyped `input_interface`を返します。整数テーマは`simple_numeric`、分数テーマは`fraction`のみを許可する`structured_math`、一次方程式は全MathLive構造を許可するrich inputを使用します。不正・巨大な入力はRust境界でfail closedに扱います。

```bash
./scripts/build-wasm.sh
```

生成packageはGit管理外の`apps/web/public/wasm/pkg/`へ出力されます。

## Theme architecture

Webの各実装テーマは`apps/web/src/domain/themes/`で**1テーマ1ファイル**として定義します。各definitionがroute、学年・ジャンル、worksheet表示、input capability、Rust compatibility identityをまとめて所有します。`theme-registry.ts`はdefinitionの列挙とlookupだけを担当します。

数学的generator本体とrevision registryはRust側にあり、Web theme definitionへ数学ロジックを持ち込みません。この分離により、テーマ追加時のmetadata同期箇所を局所化しつつ、Rustをsource of truthとして維持します。

## Web / MathLive / PDF

Web問題式はMathLive、解答入力は`math-field`を使用します。MathLiveのLaTeX snapshotはRustの`parse_mathlive_answer`を通過して初めて採点可能な`AnswerNode`になります。

印刷/PDFは独自の数式rendererを持ちません。印刷ボタンではまず同じA4 DOMの2ページプレビューを表示し、プレビュー内の「印刷する」からブラウザ標準の印刷/PDFエンジンへ進みます。問題式はWebと同じ`ProblemExpression`、解答は同じ`MathLiveStatic`で組版します。そのため分数・根号・指数などを追加してもPDF専用の座標計算やglyph実装は不要です。日本語fontもWebと同じNoto Sans JPを使用し、`pdf-lib`やPDF専用font bundleはありません。印刷moduleは操作時だけdynamic importします。MathLiveのeditor/static renderer・数式template・問題式rendererはq2用dynamic chunkへ分離し、q1のfirst paintをblockしません。idle時と問題生成開始時にpreloadし、worksheetへ切り替える前にchunk readyを待つため、数式だけ後から表示される遅延を避けます。

詳細は[`docs/web-pdf.md`](docs/web-pdf.md)、schemaは[`docs/problem-schema.md`](docs/problem-schema.md)、Answer ASTは[`docs/answer-ast.md`](docs/answer-ast.md)、effortは[`docs/effort-model.md`](docs/effort-model.md)、現行実装概要は[`docs/implementation-status.md`](docs/implementation-status.md)を参照してください。
