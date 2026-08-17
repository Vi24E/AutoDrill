# Architecture Documents

現行の実装方針を領域別に記録する。

最上位の判断原則は[`../principles.md`](../principles.md)。architecture文書と設計原則が衝突する場合は設計原則を優先し、判断が曖昧ならユーザーへ確認する。

- [`overview.md`](overview.md) — 全体構成と責務境界
- [`theme-system.md`](theme-system.md) — theme ownership / metadata / tags
- [`generator.md`](generator.md) — generator / sampling / determinism
- [`problem-schema.md`](problem-schema.md) — Problem / worksheet schema
- [`answer-ast.md`](answer-ast.md) — Answer AST / structured input
- [`effort-model.md`](effort-model.md) — effort primitive / solution graph
- [`web-print.md`](web-print.md) — Web / MathLive / print presentation
- [`deployment-security.md`](deployment-security.md) — static deployment / security

未解決の設計欠陥はarchitecture文書へ「実装済み」のように書き換えず、[`../issues.md`](../issues.md)で追跡する。
