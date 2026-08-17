# AutoDrill Documentation

このdirectoryの文書は、役割ごとに明確に分離する。

## Canonical documents

実装判断では次の順に確認する。

1. [`principles.md`](principles.md) — 最上位の設計思想・判断原則
2. [`issues.md`](issues.md) — 現在の未解決Issueの唯一のbacklog
3. [`roadmap.md`](roadmap.md) — support scopeと将来計画
4. [`architecture/`](architecture/) — 現行architecture / data model / implementation policy
5. [`../curriculum.md`](../curriculum.md) — 日本のカリキュラムと数学的教材基準

AIエージェントはproduction codeを変更する前に、最低でも`principles.md`と対象領域のarchitecture文書を読むこと。

設計思想が不明、または場当たり的な例外が必要な場合は、ユーザーへ質問してから実装する。

## Architecture

- [`architecture/overview.md`](architecture/overview.md) — Rust/WASM/Web、static/local-first、責務境界
- [`architecture/theme-system.md`](architecture/theme-system.md) — theme ownership、tag/metadata、registry
- [`architecture/generator.md`](architecture/generator.md) — generator、sampling、determinism、validation
- [`architecture/problem-schema.md`](architecture/problem-schema.md) — Problem / worksheet identity / schema
- [`architecture/answer-ast.md`](architecture/answer-ast.md) — Answer AST / structured editor / grading warning
- [`architecture/effort-model.md`](architecture/effort-model.md) — effort primitiveと標準解法graph
- [`architecture/web-print.md`](architecture/web-print.md) — Web / MathLive / 印刷境界
- [`architecture/deployment-security.md`](architecture/deployment-security.md) — static deploymentとsecurity方針

## Historical records

`audits/` と `archive/` は**履歴**であり、現行仕様のsource of truthではない。

- `audits/` — 監査時点の調査結果・実験・証拠
- `archive/` — 旧plan、旧goal、旧implementation status、旧taxonomy文書
- `design/` — logo等のdesign asset / concept

古いauditに未解決Issueが書かれていても、現在の対応要否は必ず`issues.md`で確認する。
