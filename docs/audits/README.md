# Audit Archive

このdirectoryは監査時点の調査記録・実験結果・証拠を保存する。

**現行仕様や未解決Issueのsource of truthではない。**

- 現在のIssue: [`../issues.md`](../issues.md)
- 現在の設計思想: [`../principles.md`](../principles.md)
- 現在のarchitecture: [`../architecture/`](../architecture/)

監査文書に「High」「未修正」等と書かれていても、その後修正されている可能性がある。実装者は必ずcanonical docsと現コードを確認する。

## Records

- [`2026-08-19-autonomous-qa-convergence.md`](2026-08-19-autonomous-qa-convergence.md) — targeted Close再判定 + 38 theme large-sample QA + input/Web/print/external API convergence audit。
- [`2026-08-19-independent-rust-core-reaudit.md`](2026-08-19-independent-rust-core-reaudit.md) — second independent Close再判定。3 Mediumを再FAIL、新規Low 3件を発見し、その後追加修正済み。
- [`2026-08-18-independent-rust-core-reaudit.md`](2026-08-18-independent-rust-core-reaudit.md) — fresh audit remediationに対する独立Close再判定とfresh follow-up findings（その後さらに修正済み）。
- [`2026-08-18-fresh-rust-core-audit.md`](2026-08-18-fresh-rust-core-audit.md) — `964df53`時点のRust core / WASM / Web boundary fresh auditと、その後のremediation verification。
- [`2026-08-15-effort-direction-audit.md`](2026-08-15-effort-direction-audit.md) — effort方向・difficulty bias調査。
- [`2026-08-14-independent-audit.md`](2026-08-14-independent-audit.md) — 独立監査記録。
