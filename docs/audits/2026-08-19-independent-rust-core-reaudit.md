# 2026-08-19 Independent Rust Core Re-audit

この文書は、2026-08-19時点の未コミットremediation worktreeに対して別担当が行った独立Close再判定 + fresh Rust auditの記録である。**現行Issueのsource of truthは`docs/issues.md`であり、この監査結果はその後の追加修正前時点の証拠である。**

## Verdict at audit time

`VERDICT: NOT OK`

PHASE A:

- `AUDIT-M-001`: FAIL / Close NO。Webのdomain semantic duplicationはほぼ解消したが、`operation_plan.operations`がRust `Operation` tagged unionの未知discriminant・variant固有payload幅をfail-closedに検証していなかった。
- `AUDIT-M-002`: FAIL / Close NO。pool完成後のdistinct capacity不足はtyped errorになったが、ranked selectorが採用済みsemantic keyのduplicateをpoolに残すため、capacity十分なduplicate-rich poolでも`AttemptLimit`へ落ち得た。
- `REAUDIT-M-001`: FAIL / Close NO。root re-exportは縮小したが、`Problem`のpublic effort accessors経由でinternal effort型を操作でき、`generate_problem(seed)` / `generate_worksheet(seed)`とrequest defaultがtheme 1固定のgeneric-looking facadeとして残っていた。
- `REAUDIT-L-001`: PASS / Close YES。`answer-ast.md`はcurrent ownership/normalization architectureと一致。
- `REAUDIT-L-002`: PASS / Close YES。current-only `_v1` residueは消滅。

## New Low findings

- `BootstrapDedup`という名称が、現在実際にdedup policyを適用するselection phaseと一致していない。
- public `AnswerNode::Binary`に含まれる`AnswerBinaryOperator`がcrate root exportされず、public AST APIとして非対称。
- cross-language worksheet/layout DTOがtarget-sized `usize`で、Web側もfixed-width wire contractとして検証できない。

新規High / MediumはPHASE Aの3件以外になし。

## Quality gates at audit time

監査時点ではRust fmt/check/clippy/test、generated contract/wire checks、Web tsc/eslint/vitest、`git diff --check`はPASS。drill-core 117、drill-wasm 8、Web 165 tests。

## Architecture grades at audit time

- Domain model: A-
- Ownership / borrowing: A-
- Error design / panic safety: B
- Exact arithmetic: A
- Generator architecture: C+
- Effort architecture: A-
- Theme extensibility: B+
- Public API design: C
- WASM boundary: A-
- Web/Rust SoT boundary: B
- Test architecture: B+
- Module organization: B+
- Documentation consistency: A-
- 仕様変更耐性: 7/10
- Rust idiomaticity: 8/10
- 保守性: 7/10

監査時のcommit recommendationはNO。理由はLowではなく、上記3 Mediumのstructural defect。

## Remediation verification — 2026-08-19

この監査後、Close不可だった3件と新規Low 3件を追加修正した。修正担当はこのverification記録の作成者と同一なので、`AUDIT-M-001` / `AUDIT-M-002` / `REAUDIT-M-001` / `REAUDIT2-L-*`は次の独立再監査まで`修正済み / 独立再監査待ち`とする。監査自身がPASSした`REAUDIT-L-001` / `REAUDIT-L-002`のみClosedへ更新した。

追加修正:

- Webにpure wire-level `assertOperation()`を追加。known variant、`Count.amount: u32`、`TimeTen.exponent: u32`、`OverheadDistribution.terms: u32`、`BigNum.magnitude: canonical u64 decimal string`を検証し、effort semantic consistencyはRustへ残す。
- selectorは1 semantic keyを採用した時点でそのkeyの全duplicateをselection populationから除去。capacity十分ならsuccessful selectionごとに未選択distinct keyが単調減少する。監査反例と同じ160 candidates / 40 distinct / 121 duplicate poolをregression化し、difficulty 1–4で完走を確認。
- `Problem`のworked-solution/effort internal accessorsをcrate-privateへ縮小。external scalar `effort()`だけを維持。coreのtheme 1 shortcut generation APIとrequest defaultsを削除し、requestはexplicit theme IDを必須化。
- `BootstrapDedup`を`SelectionDedup`へ改名。
- `AnswerBinaryOperator`をpublic AST構成要素としてroot export。
- layout cross-language contractを`u32`へ固定。

外部temporary crate probeでは、旧shortcut / internal effort type imports / internal accessorsはcompile failure、explicit request generationとscalar effortはcompile successを確認し、temporary directoryは削除した。

最終quality gates:

- `cargo fmt --all -- --check`: PASS
- `cargo check --workspace --all-targets --all-features --offline`: PASS
- `cargo clippy --workspace --all-targets --all-features --offline -- -D warnings`: PASS
- `cargo test --workspace --all-features --offline`: PASS — drill-core 118 / drill-wasm 8
- generated Rust→Web contract freshness: PASS
- generated Rust→Web wire types freshness: PASS
- Web `tsc --noEmit`: PASS
- Web ESLint `--max-warnings=0`: PASS
- Web Vitest: PASS — 20 files / 172 tests

Vite CJS Node API deprecation warningは既存toolchain warningでありtest failureではない。
