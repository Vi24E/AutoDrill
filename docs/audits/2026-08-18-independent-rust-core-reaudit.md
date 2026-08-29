# 2026-08-18 Independent Rust Core Re-audit

この文書は、fresh audit後の未コミット修正worktreeに対して別担当が実施した独立再監査の記録である。**現行Issueのsource of truthはGitHub Issues (https://github.com/Vi24E/AutoDrill/issues) であり、この監査結果はその後の修正前時点の証拠である。**

## Verdict at audit time

`VERDICT: NOT OK`

PHASE A:

- `AUDIT-H-001`: PASS / Close YES。external safe Rust APIからrecursive prompt ASTへ到達不能。
- `AUDIT-M-001`: FAIL / Close NO。既知driftは直ったが、WebにRust semantic policyの手書き再実装が残存。
- `AUDIT-M-002`: FAIL / Close NO。zero-bound panicは塞がったが、distinct不足がouter retryで`AttemptLimit`へ変換される。
- `AUDIT-L-001`: PASS / Close YES。`print_recommended`はtyped presentation policyとしてcode/docs一致。

## New findings

- Medium: crate root public Rust APIがcurrent product consumerに対して広すぎる。effort/registry/theme internalsとselective theme facadeがsemver surfaceへ露出。
- Low: `docs/architecture/answer-ast.md`が削除済み`AnswerRepresentation`をcurrent architectureとして記述。
- Low: `draw_problem_v1`, `operand_domain_v1`, `summary_operand_domain_v1`等のcurrent-only naming residue。

補助的な単純化候補として、Webの手書き`THEME_TAG_VALUES`、`ALL_MATH_STRUCTURES`、unused semantic equality helper、`print_recommended tag`という古いtest名称も指摘された。

## Quality gates at audit time

Rust fmt/check/clippy/test、generated contract/wire check、`git diff --check`、Web tsc/eslint/vitestはすべてPASS。Rust 124 tests、Web 165 testsだった。

## Architecture grades at audit time

- Domain model: A-
- Ownership / borrowing: A-
- Error design / panic safety: B+
- Exact arithmetic: A
- Generator architecture: B
- Effort architecture: A-
- Theme extensibility: B+
- Public API design: C+
- WASM boundary: A-
- Web/Rust SoT boundary: C
- Test architecture: A-
- Module organization: B+
- Documentation consistency: B
- 仕様変更耐性: 7/10
- Rust idiomaticity: 8/10
- 保守性: 7/10

この監査後、`AUDIT-M-001` / `AUDIT-M-002`と新規findingsは再修正された。Close判定は次の独立再監査へ委ねる。

## 2026-08-19 remediation verification

この独立再監査でClose不可だった`AUDIT-M-001` / `AUDIT-M-002`と、新規`REAUDIT-M-001` / `REAUDIT-L-001` / `REAUDIT-L-002`に対して追加修正を実施した。修正担当はこのverification記録の作成者と同一なので、canonical issueは独立再監査まで`修正済み / 独立再監査待ち`を維持する。

主な変更:

- Web WASM adapterをSerde/wire integrity validationへ縮小し、seed/difficulty/input capability/theme-prompt/schema/layout/effort/grade-result等のRust-owned semantic policy再実装を除去。input capability validationはgrade requestにも`input_interface`を渡しRust core/WASMへ委譲。
- candidate poolのlow-diversity outer retryを廃止し、raw pool完成後にglobal/layer distinct capacityをtyped `InsufficientDistinctCandidates`として確定的に返す。pool完成前にwork budgetが尽きた場合だけ`AttemptLimit`とするproperty testも追加。
- crate rootからcurrent product consumerのないeffort/registry/theme internalsとselective theme facadeを削除。repository外consumer compile probeで`Operation`, `ThemeRegistration`, `active_registrations`, `ONE_DIGIT_ADDITION_REGISTRATION`がimport不能であることを確認。
- `AnswerRepresentation` docs drift、current-only `_v1` naming、Webの手書きtheme-tag/editor-structure列挙、古い`print_recommended tag` test名を整理。
- 作業中にproptestが生成した`crates/drill-core/proptest-regressions/generator.txt`は、旧error contractを期待するstale propertyが原因だったためpropertyを修正し、生成残骸を削除。再実行で再生成されないことを確認。

最終quality gates:

- `cargo fmt --all -- --check`: PASS
- `cargo check --workspace --all-targets --all-features --offline`: PASS
- `cargo clippy --workspace --all-targets --all-features --offline -- -D warnings`: PASS
- `cargo test --workspace --all-features --offline`: PASS — drill-core 117 / drill-wasm 8, total 125
- generated Rust→Web contract/wire freshness checks: PASS
- Web `tsc --noEmit`: PASS
- Web ESLint `--max-warnings=0`: PASS
- Web Vitest: PASS — 20 files / 165 tests
- external Rust public-API compile probe: PASS (internal facade imports fail as intended)

Vite CJS Node API deprecation warningは依存/toolchain由来であり、test failureではない。
