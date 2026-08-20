# 2026-08-19 Autonomous QA / Convergence Audit

## Scope

Current worktreeをsource of truthとして、`AUDIT-M-001` / `AUDIT-M-002`のClose再判定、全38 themeのlarge-sample generation QA、input/grading public boundary、Web/print shared presentation、external Rust API、test/repository hygieneを確認した。

この監査ではdifficulty sampler自身が利用する`effort`のmeanを「実際の難しさ」の独立証拠として扱わない。effort orderingはsampler contractのsanity checkに限定し、難易度の実体確認にはcarry/borrow、桁数、演算子数、異分母率、係数規模、liar人数、Sudoku空欄数等の**effortから独立したprompt構造指標**を用いる。

## Targeted Close re-audit

### AUDIT-M-001 — PASS / Close YES

Web `wasm-adapter.ts`のAnswerNode / schema / input-interface / grade DTO検証は、known tag、required property、canonical integer encoding、primitive width、structural node budget等のwire integrityに限定されている。display-size、normalization、input capability validity、mathematical validity、grading semanticsはRust/WASMがauthorityである。

`wasm-adapter.test.ts`には、structural-node overflowを拒否する一方でRust-owned display-size/schema/grade semanticsをWebが再判定しないregressionがある。full Vitestも通過した。

### AUDIT-M-002 — PASS / Close YES

Random difficultyは最初の抽選前に`ProblemKey`単位でcandidate populationをcollapseし、distinct semantic candidate集合から`swap_remove`で一様非復元抽出する。raw bootstrap multiplicityは選択確率へ入らない。

40 distinct candidateについて一候補だけ121回重複させた160-candidate poolと重複なしpoolが、同じSeedで同じRandom selection sequenceを返すregressionを確認した。capacity不足はtyped sampling error、capacity十分なduplicate-rich poolはfinite progressする。

## Autonomous large-sample QA

Audit-only test `generator/autonomous_qa.rs` を追加した。通常CIを重くしないよう`#[ignore]`とし、明示実行時だけ全registryを広くprobeする。

- registered themes: 38
- difficulties: 4
- seeds / theme / difficulty: 100
- worksheets: 15,200
- problems: 242,400
- generation failures: 0
- invariant violations: 0
- layered worksheets checked: 1,600

確認対象はproblem count、duplicate prompt / semantic `ProblemKey`、curriculum guard、determinism、problem-set regeneration、layer quota、finite effort、generation error/pathological seedの記録である。最初の3 Seedは同一request再生成と`problem_set_id`再生成も比較する。

### Independent structural difficulty probes

代表的なeasy / normal / hardの結果:

- 一桁加算: carry 0.09 / 0.59 / 0.99、answer magnitude 7.03 / 9.97 / 12.98
- 二桁加算: carry 0.44 / 0.92 / 1.81
- 九九: answer magnitude 9.30 / 21.62 / 40.24
- 負の数(1): operators/problem 1.15 / 2.03 / 2.81
- 負の数(2): operators/problem 1.22 / 2.10 / 2.51
- 分数加算: unlike-denominator rate 0.70 / 0.98 / 1.00
- 分数減算: unlike-denominator rate 0.16 / 0.77 / 0.95
- 二次方程式(2): coefficient magnitude 5.28 / 10.66 / 16.10
- うそつきだれだ: people count 3.14 / 3.58 / 3.95
- 二桁加算の筆算: carry 0.43 / 0.94 / 1.83
- 二桁引算の筆算: borrow 0.00 / 0.31 / 0.95
- 三・四桁加算の筆算: carry 0.91 / 1.64 / 2.48
- 三・四桁引算の筆算: borrow 0.64 / 1.29 / 2.11
- 小数加減筆算: carry/borrow 0.30 / 0.73 / 1.48
- Mini Sudoku: blanks 5.82 / 7.48 / 9.35

これらはeffort modelの自己整合性とは独立した「promptの構造がdifficultyとともに変化している」証拠である。ただし、人間の正答率・所要時間を観測していないため、教育的な体感難度をpsychometricに証明するものではない。

## Input / grading boundary

既存testはinteger、negative、decimal、fraction、mixed fraction、negative mixed fraction、Unicode minus、root、plus/minus、tuple / multi-answer、deep/oversized inputをcoverしていた。

追加regression:

- operator precedence: `2+3*4` -> 14
- malformed LaTeX: bounded `nan_error`
- WASM JSON public boundaryで representative structured inputsを parse -> normalize -> grade round-trip
- deep / oversized LaTeXは`answer_ast_size_limit`

## Web / print / mechanical QA

- Web/PDFは同一Problem DTOとshared `ProblemExpression` / column presentationを利用するcurrent architectureを確認。
- print testは全registered themeをshared presentation経由でrenderする。
- print-flow integrationはsettings -> preview -> native print、editing/selected/graded worksheetからpreview/backの状態保持をcoverする。
- production Next static build: PASS
- GitHub Pages export: PASS — 39 routes
- real Chrome/CDP layout verifier: **PASS**。通常sandboxでは`127.0.0.1` listenが`EPERM`となるためhost permissionで実行し、80 worksheet samplesすべてcrossing 0 / grid mismatch 0を確認。Mini Sudoku、符号計算、小数筆算、掛け算筆算、割り算筆算のnative `Page.printToPDF` probeも2ページPDFとしてPASS。

Browser WASMは`wasm-pack`の`wasm-opt`だけsandbox権限で拒否されたため、同一sourceを`cargo build --target wasm32-unknown-unknown --release` + Cargo.lockと一致する`wasm-bindgen 0.2.126`で生成し、Next buildへ使用した。

## External Rust API probe

Repository外temporary crateから以下をcompile/runした。

- `GenerateProblemRequest`
- `GenerateWorksheetRequest`
- `Problem::effort()`
- `grade_answer`
- `parse_mathlive_answer`
- custom `MonotonicClock`
- `ProblemSetIdentity`
- `SCHEMA_VERSION`

一方、`ArithmeticExpression`, `ProblemPrompt`, `EffortModel`, `OperationPlan`, `SystemClock`のroot importはcompile failureとなり、current consumer不要なinternal APIが外へ漏れていないことを確認した。temporary crateは削除済み。

## Test-suite maintenance

Baseline / final:

- Rust test functions: 130 -> 132
- Rust files containing tests: 20 -> 21
- Web tests: 166 -> 166
- Web test files: 20 -> 20
- Web test source lines: 3,722 -> 3,722
- Rust test-only code added: 766 lines (638-line audit harness + 29-line grammar regression + 99-line WASM boundary regression); redundant column-arithmetic effort-mean test 1件（約60行）を削除し、net test-only増加は約706行

既存test削除は1件、既存test統合は0。削除したのは13個の筆算themeだけでdifficulty別effort meanを再確認していたtestで、共通samplerの自己整合性検査と重複し、pedagogical difficultyの独立検証にもならなかった。既にregistry-wideで保証されているcontractをtheme別copy-paste testへ増殖させず、大量probeを1つのtable-driven test-only harnessへ集約した。通常CIで新たに実行されるのはinput/grading regression 2件だけで、large-sample probeは明示実行専用である。

## Repository hygiene

`.DS_Store`、proptest regression residue、`*.tmp` / `*.bak` / editor backup artifactは検出されなかった。QA harnessはproduction APIを増やさずtest-only child moduleへ分離した。MathLive helperのstaleな`compatibility helper`コメントもcurrent-only表現へ整理した。

## Convergence

Current architectureでは、theme追加後にregistry-wide invariant test + explicit large-sample probe + Rust/WASM boundary + Web shared rendering/print tests + real Chrome layout/native PDF probeまでAIが自律実行できる。残る人間QAはpedagogical difficultyの体感と最終的な美的評価に限定できる。
