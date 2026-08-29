# 2026-08-18 Fresh Rust Core / WASM Architecture Audit

## Status

- Audit target: `964df53 Refactor Rust core around validated domain boundaries`
- Initial worktree: clean (`main...origin/main [ahead 2]`)
- Initial audit verdict: **NOT OK**
- Initial findings: **High 1 / Medium 2 / Low 1**
- Remediation performed after the audit: **yes**
- Post-fix automated verification: **all required gates pass**
- Formal issue closure: **independent re-audit pending**

This document is a dated audit record. The canonical backlog is [GitHub Issues](https://github.com/Vi24E/AutoDrill/issues), and the current architecture remains [`../architecture/`](../architecture/).

---

## 1. Executive summary

The current Rust codebase is not a “Better C” codebase. The core makes substantial use of Rust domain modeling: private aggregate fields, validated newtypes, checked exact arithmetic, typed generator strategies, immutable `Problem` / `Worksheet` aggregates, and a thin WASM adapter. `AnswerNode` in particular has been deliberately made stack-safe for hostile native trees; a 1,000,000-level external tree was exercised across its major custom traits during the audit without reproducing the old stack-overflow class.

The largest fresh defect was elsewhere: `ArithmeticExpression` remained a crate-public recursive raw enum with ordinary recursive derived traits. An external safe-Rust consumer could build a 100,000-level expression and abort the process merely by dropping it. This was a genuine public API safety defect and the sole High-severity commit blocker.

Two structural Medium findings remained. First, the Web adapter still reimplemented parts of Rust semantic policy and already disagreed with Rust about legal answer-schema values while also collapsing several semantic Rust/WASM errors into `invalid_dto`. Second, candidate selection assumed sufficient distinct capacity even when a bootstrap pool was allowed to contain duplicates, so a valid generic strategy configuration could eventually reach `next_bounded(0)`.

A Low documentation drift was also present: `curriculum.md` described `print_recommended` as a taxonomy tag although the current architecture intentionally owns it in typed presentation policy.

The fixes below address all four formal findings without changing the product’s mathematical behavior or reintroducing legacy compatibility machinery. Large module splitting was deliberately not mixed into this remediation.

---

## 2. Original formal findings

### AUDIT-H-001 — Public recursive `ArithmeticExpression` can abort a safe external consumer

**Severity:** High
**Initial blocker:** YES

### Evidence

At audit time:

- `crates/drill-core/src/model.rs` defined `ArithmeticExpression` as a public recursive enum containing `Box<ArithmeticExpression>`.
- It derived ordinary recursive `Clone`, `Debug`, `Deserialize`, `Eq`, `Hash`, `Ord`, `PartialEq`, `PartialOrd`, and `Serialize` implementations.
- `crates/drill-core/src/lib.rs` re-exported it from the crate root.
- `ProblemPrompt` was also public and `Problem::prompt()` exposed the raw prompt tree.

An external temporary crate, using only safe Rust and the current `drill-core` as a path dependency, constructed a 100,000-level left-deep expression. Calling `drop(expr)` produced a native stack overflow followed by process abort.

This differed materially from `AnswerNode`: the latter already had explicit-stack `Drop`, `Clone`, equality/order, bounded debug/serialization paths, and structural guards.

### Why it mattered

The defect required no malformed JSON, unsafe code, private API access, or generator bug. It was reachable directly from the public Rust surface. “Current generators only produce shallow expressions” was therefore not a sufficient invariant.

---

### AUDIT-M-001 — Web adapter duplicates Rust semantic policy and can drift

**Severity:** Medium

### Evidence

Concrete current mismatches found during the audit included:

- Rust accepts `AnswerSchema::Decimal { max_scale: 0 }`; Web rejected `max_scale === 0`.
- Rust permits `Rational { max_abs_numerator: 0, max_denominator > 0, ... }`; Web rejected zero maximum numerator.
- Web added a `max_digit <= 9` digit-grid rule that is not part of the generic Rust `AnswerInputInterface` structural invariant.
- Rust/WASM preserved distinct semantic errors such as `unsupported_schema_version`, `unknown_theme`, `unknown_generator_revision`, invalid identity, and input-interface violations, but Web mapped several of them to generic `invalid_dto`.

The generated Rust contract and generated TS wire types were fresh; the drift existed in handwritten runtime validation and error mapping.

### Why it mattered

A future Rust-valid schema could serialize successfully through WASM and still be rejected only by the Web client. That is a direct violation of the intended “Rust is mathematical/domain source of truth; Web validates wire integrity” boundary.

---

### AUDIT-M-002 — Candidate selection has a hidden distinct-capacity precondition

**Severity:** Medium

### Evidence

The generic sampling framework permits bootstrap pools containing duplicate semantic candidate keys. Final selection, however, always requires unique problem keys.

The old selection code checked raw pool length but did not establish that the pool contained enough distinct keys. In the random-difficulty path it repeatedly used `swap_remove()` while accepting only previously unseen keys. A pool with enough elements but too few distinct keys could therefore be exhausted while `selected.len() < count`, after which `rng.next_bounded(pool.len() as u64)` would receive zero and panic.

Current production constructive-layered themes happened to use deduplicated bootstrap pools, so this was not observed in the active 38 themes. It was nevertheless a real generic-framework hidden precondition.

---

### AUDIT-L-001 — `curriculum.md` describes `print_recommended` using the old metadata model

**Severity:** Low

`curriculum.md` stated that column-arithmetic themes carry a `print_recommended` taxonomy tag. Current Rust intentionally removed behavioral `ColumnArithmetic` / `PrintRecommended` tags and derives the capability from `ThemePresentationPolicy` instead.

The code was the more coherent architecture; the current documentation sentence was stale.

---

## 3. Remediation

### 3.1 AUDIT-H-001 remediation — close the raw prompt AST from the crate public API

The fix deliberately does **not** add a second copy of `AnswerNode`’s large custom stack-safe trait implementation. There is no current external product consumer that needs to construct arbitrary `ArithmeticExpression` or `ProblemPrompt` values.

Changes:

- Removed `ArithmeticExpression`, `ArithmeticOperator`, `ProblemPrompt`, and `RationalCoefficient` from the `drill-core` crate-root public re-export surface.
- Changed `Problem::prompt()` to `pub(crate)`.
- Made nested wire DTOs (`ProblemWire`, worked-solution wire internals, operation-plan wire internals) crate-private.
- Kept `WorksheetWire` public only as the tooling root required by the `ts-rs` export binary, but made its fields private so it cannot be used as an external raw-domain construction surface.
- Kept generated TypeScript output unchanged in shape: `ts-rs::export_all_to` still recursively exports the nested canonical Rust wire dependencies.
- Removed unused `MiniSudokuGrid` getters exposed only by the previously broad public surface.

### External-consumer verification

A new temporary external crate attempted:

```rust
use drill_core::ArithmeticExpression;
```

The current crate correctly fails to compile it:

```text
error[E0432]: unresolved import `drill_core::ArithmeticExpression`
no `ArithmeticExpression` in the root
```

The temporary crate was removed after the probe. No repository file was used for the external exploit test.

This changes the invariant from “external consumers may construct an arbitrarily deep raw prompt AST, and every recursive trait must defend itself” to “raw generated prompt syntax is an internal implementation detail; external consumers receive validated aggregate/wire output only.” For a pre-release crate with no external prompt-construction consumer, that is the simpler and safer Rust API.

---

### 3.2 AUDIT-M-001 remediation — separate wire integrity from Rust semantic authority

Changes in `apps/web/src/domain/wasm-adapter.ts` and `drill-engine.ts`:

- Removed the Web-only rejection of `Decimal.max_scale === 0`.
- Removed the Web-only rejection of `Rational.max_abs_numerator === 0`.
- Removed the Web-only generic `digit_grid.max_digit <= 9` restriction.
- Preserved safe-number/u32/shape/discriminant validation at the Web boundary.
- Added first-class Web error kinds for:
  - `unsupported_schema_version`
  - `unknown_theme`
  - `unknown_generator_revision`
  - `invalid_problem_set_identity`
  - `input_structure_not_allowed`
  - `input_interface_violation`
- These codes are no longer collapsed into `invalid_dto`.

Regression coverage was added to ensure a Rust-valid zero maximum rational numerator is not rejected by Web and to ensure the semantic Rust/WASM error codes survive to `DrillEngineErrorKind`.

---

### 3.3 AUDIT-M-002 remediation — validate distinct capacity before selection

Added:

```text
SamplingError::InsufficientDistinctCandidates { required, available }
```

`select_candidates_from_pool()` now computes the number of distinct `ProblemKey`s before any selection loop. It requires:

- random difficulty: at least `count` distinct candidates;
- ranked difficulties: at least `count + 2 * EFFORT_TRIM_PER_SIDE` distinct candidates.

If the requirement is not met, selection returns the typed sampling error before any RNG call can see an empty pool. The old raw `layer_pool.len()` check was removed because raw length did not establish the actual invariant.

A regression test constructs eight candidate objects with one shared semantic key and verifies that requesting two unique random candidates returns:

```text
InvalidSampling(InsufficientDistinctCandidates { required: 2, available: 1 })
```

rather than reaching a zero-bound RNG panic.

---

### 3.4 AUDIT-L-001 remediation — synchronize curriculum wording

`curriculum.md` now states that column-arithmetic themes derive `print_recommended` from typed `ThemePresentationPolicy`, and that this capability is not duplicated as a taxonomy tag.

---

## 4. Additional cleanup performed during remediation

These were audit observations below formal-Issue severity, not new formal issues:

- Removed unused public `AnswerRepresentation`; it had no current consumer and allowed independent `display` / `normalized` fields to be constructed inconsistently.
- Removed an orphaned “Backward-compatible name retained…” comment in `problem-format.ts` that no longer described any compatibility alias.
- Renamed Rust-only decimal multiplication symbols from the obsolete `*_DECIMAL_MULTIPLY_DIVIDE` naming to `*_DECIMAL_MULTIPLICATION`. Numeric theme identity, skill ID, curriculum path, generator behavior, and wire contract are unchanged.

Large-file decomposition (`effort.rs`, `generator.rs`, `model.rs`) was **not** performed merely to reduce line count. Those were maintenance observations, not demonstrated correctness defects, and mixing a module rewrite into the four targeted fixes would increase review risk.

---

## 5. Post-fix quality gates

Final verified state before writing this audit document:

| Gate | Result |
|---|---|
| `cargo fmt --all -- --check` | PASS after canonical formatting |
| `cargo check --workspace --all-targets --all-features --offline` | PASS |
| `cargo clippy --workspace --all-targets --all-features --offline -- -D warnings` | PASS |
| `cargo test --workspace --all-features --offline` | PASS — `drill-core` 117 + `drill-wasm` 7 = **124 tests** |
| `node scripts/sync-web-contract.mjs` | PASS, check-only |
| `node scripts/sync-web-wire-types.mjs` | PASS, check-only |
| `apps/web ./node_modules/.bin/tsc --noEmit` | PASS |
| `apps/web ./node_modules/.bin/eslint . --max-warnings=0` | PASS |
| `apps/web ./node_modules/.bin/vitest run` | PASS — **20 files / 165 tests** |
| external Rust public-API probe | PASS — old `ArithmeticExpression` import fails to compile |
| `git diff --check` | PASS before documentation write; rerun at final handoff |

Vitest emits the Vite CJS Node API deprecation warning twice. This is a dependency/toolchain warning, not a project-code lint/test failure.

---

## 6. Architecture assessment after remediation

The original audit grades were:

| Area | Initial grade | Post-fix assessment |
|---|---:|---|
| Domain model | C | **B** — validated aggregates remain strong, and the unsafe raw prompt AST is no longer external construction surface |
| Ownership / borrowing | B | **B** |
| Error design / panic safety | C | **B** — external prompt abort and zero-bound selector path are removed |
| Exact arithmetic | A | **A** |
| Generator architecture | C | **B** — distinct-capacity invariant is now explicit and typed |
| Effort architecture | B | **B** |
| Theme extensibility | B | **B** |
| Public API design | C | **B** — raw implementation types and unused facade surface are reduced |
| WASM boundary | A | **A** |
| Web/Rust SoT boundary | C | **B** — known semantic-policy disagreements and error-code collapse are removed |
| Test architecture | B | **B** |
| Module organization | B | **B** |
| Documentation consistency | B | **B** |

The architecture still has large modules and necessarily cross-cutting contracts (`AnswerNode`, schema version, operation-vector dimension), but those are not reasons for a rewrite. The codebase is a reasonable foundation for continued theme development once these changes receive an independent Close re-audit.

---

## 7. Changeability after remediation

1. **Adding an ordinary math theme:** still relatively local to a family module plus explicit Web registration; no new central semantic switch was introduced by these fixes.
2. **Adding a new Answer representation:** remains cross-cutting because of `AnswerNode`’s wire/input/grading/rendering role; this is the largest intentional change surface.
3. **Adding an effort primitive:** structurally clear, but changes the schema-visible vector dimension and therefore legitimately requires contract regeneration/schema consideration.
4. **Changing schema version:** generated Rust→Web contract remains strong; the repaired Web boundary is less likely to reject a Rust-valid new value through stale semantic assumptions.
5. **Changing sampling policy:** safer because final unique-selection capacity is now an explicit checked invariant rather than a caller convention.
6. **Changing Web UI:** remains comparatively isolated from Rust mathematical semantics.
7. **Changing Rust core:** generated contract/wire freshness checks remain the primary anti-drift mechanism; handwritten Web validation is now closer to boundary-shape validation rather than a second domain engine.

---

## 8. Final remediation verdict

**Implementation/gate verdict: OK.**

The single High blocker identified by the fresh audit is no longer reachable from the external safe Rust API, and both Medium structural defects have dedicated regression coverage. The Low current-doc drift is corrected.

Because the same agent performed both this audit and the remediation, the canonical issues are recorded as **修正済み / 独立再監査待ち**, not self-Closed. A fresh independent auditor should verify the current diff and decide final closure.
