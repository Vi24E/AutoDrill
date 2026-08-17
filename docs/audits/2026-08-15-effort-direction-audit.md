> **Historical document:** この文書は履歴保存用であり、現行仕様のsource of truthではありません。現在の設計は `docs/principles.md` / `docs/architecture/`、未解決事項は `docs/issues.md` を参照してください。

# Effort direction / difficulty sampling audit — 2026-08-15

## 0. Scope and conclusion

This document records the independent audit of how scalar `effort = w · x` difficulty selection changes the distribution of the underlying operation vector `x`.

The original audit did **not** modify production code. Instrumentation was placed only in a repository-external copy. The implementation status below was added after the fixes in this same development cycle; Sections 1–14 retain the pre-fix measurements as historical baseline where useful.

### Main conclusion

The originally suspected catastrophic loss of operation-vector diversity was **not** observed as a general problem.

- `normal` and especially `hard` usually concentrate operation-vector directions more than `random`, because selecting by scalar effort necessarily favors a narrower family of operation compositions.
- `easy` does not consistently concentrate directions; in several themes it becomes more directionally diverse because there are several different cheap strategies.
- This concentration is often educationally intended: e.g. carry/borrow, reduction, factor search, or longer arithmetic legitimately becomes more common at higher difficulty.
- Therefore a generic vector-diversity optimizer is **not currently justified**.

The practical problems found are narrower and are addressed by:

1. fixing generator populations where the source population is biased or too small;
2. stratifying pedagogically distinct problem archetypes before applying scalar effort selection;
3. splitting themes that are actually separate curriculum units.

### Post-audit implementation status

| Finding | Status | Current implementation |
|---|---|---|
| Generic operation-vector diversity | **intentionally unchanged** | No cosine/PCA/farthest-point/vector-diversity sampler was added. Difficulty remains scalar effort. |
| Decimal multiplication vs division | **fixed by curriculum split** | ID 18 revision 6 is multiplication; ID 24 revision 1 is division. ID 18 revision 5 remains regenerable as the historical combined theme. |
| Decimal addition/subtraction coverage | **fixed** | Layered sampling gives 10 addition + 10 subtraction in a 20-problem worksheet at every difficulty, including random. |
| Quadratic equation (2) archetype loss | **fixed** | Layered sampling gives 2 difference-of-squares + 2 perfect-square + 12 general problems in the current 16-problem worksheet (2/2/16 for 20 problems). Repeated roots are restored and redundant outer scale is fixed at 1. |
| Fraction arithmetic curriculum design | **superseded by curriculum redesign** | Standard fraction units use mixed-number presentation/answers when improper; fraction×integer and fraction÷integer are separate grade-6 themes; `分数総まとめ(仮分数)` preserves improper-fraction practice as a four-layer theme. Historical IDs 9–12 revision 3 remain regenerable as hidden improper-fraction generators. |
| SignedArithmetic2 division starvation | **fixed** | Bounded rational intermediate/final values are allowed. Raw division-node share rose from 8.5% to 25.22% in the post-fix 5,000-candidate measurement. |
| Quadratic equation (3) embedded `±` grading | **fixed** | Embedded `PlusMinus` is expanded exactly into a bounded solution set and compared with explicit Tuple roots without floating-point approximation. |
| Liar-puzzle 3/4-person difficulty skew | **intentionally unchanged; not an Issue** | The theme is not layered. Easy favoring 3-person and hard favoring 4-person is intended scalar-difficulty behavior. |

The remainder of this audit should be read with this table as the current status. Numeric distributions in Sections 1–14 describe the pre-fix baseline unless a section explicitly says otherwise.

---

## 1. Audit method

The production working tree was copied to:

`/Users/koharayu/Desktop/AutoDrill-effort-audit-tmp`

Only that copy was instrumented to expose the candidate population immediately before difficulty selection.

Measured sample:

- 20 themes
- 4 difficulties: easy / normal / hard / random
- 300 seeds per theme/difficulty
- 410,400 final worksheet problems
- 668,395 raw candidate problems

For each problem, the following directions were analyzed:

- raw operation direction: `u = x / ||x||₂`
- weighted contribution direction: `v = (w ⊙ x) / ||w ⊙ x||₂`
- versions excluding `BigNum`
- auxiliary core-arithmetic versions excluding `BigNum` and overhead-family dimensions

Metrics included:

- pairwise cosine similarity distribution
- centroid concentration `||mean(u)||₂`
- centered covariance participation ratio / effective rank / principal-component share
- per-operation prevalence and mean count
- operation-count correlation with scalar effort
- generator-format prevalence
- raw population vs random
- exact-same-effort and narrow-effort-band direction diversity
- paired seed-block bootstrap confidence intervals

`cargo test -p drill-core` in the external copy: **93 passed, 0 failed**.

---

## 2. Current difficulty sampler

The sampler remains scalar-effort based. Non-layered themes retain the original behavior. A theme may additionally declare pedagogical sampling layers; this metadata is internal to generation and is not exposed in the Worksheet schema.

For a non-layered N-problem worksheet:

- ordinary generators build an `8N` candidate pool;
- finite generators enumerate their finite candidate domain;
- `random` samples uniformly from the unsorted candidate pool, rejecting only duplicate prompts;
- easy/normal/hard sort by scalar effort;
- five candidate indices are sampled repeatedly (minimum / median / maximum for easy / normal / hard);
- `N + 4` candidates are selected, then the two lowest and two highest selected efforts are trimmed.

For a layered theme, the common sampler first allocates quota using per-layer minimums plus weighted largest-remainder allocation, partitions the candidate population by layer, applies the same scalar sampler independently inside each layer, and merges the results. Random also respects quota and samples randomly inside each layer. Thus difficulty is conditional on pedagogical archetype, not a vector-diversity objective.

The random mode is a good empirical proxy for the raw generator population. Across themes, the difference between raw and random centroid concentration was at most about 0.006 in this audit.

---

## 3. Direction-diversity result

Ignoring the three explicit special models (multiplication table, inverse multiplication-table division, liar puzzle), 17 ordinary vector-model themes remain.

Raw-direction centroid concentration relative to random:

- normal increased in **17/17** themes;
- hard increased in **16/17** themes;
- easy increased in only **7/17** themes.

Weighted-contribution direction:

- hard increased in **17/17** themes.

Thus the hypothesis "all selected difficulties are less diverse than random" is false in that simple form. A more accurate statement is:

> Scalar selection tends to make normal/hard operation composition more directional, but easy may either concentrate or disperse. The important question is not direction concentration itself, but whether a pedagogically important problem archetype disappears from a difficulty level.

This distinction is important. For example, an easy worksheet having fewer carry/reduction operations is intended; an easy worksheet becoming almost entirely one arithmetic operator when the theme is supposed to teach two operators is not.

---

## 4. Major observed distribution effects (pre-fix baseline)

### Signed arithmetic (2)

The raw generator population already underproduces division because every AST node must evaluate to an integer.

Approximate raw operator-node shares:

- addition: 30.7%
- subtraction: 30.3%
- multiplication: 30.5%
- division: **8.5%**

This is primarily a **generator population** issue, not a sampler issue.

### Fraction multiplication

The finite source population is very small and strongly constrained. Unit fractions are already common in the raw domain and easy selection amplifies them heavily.

Integer operands are absent entirely.

This is primarily a **generator-domain** issue.

### Fraction subtraction

The finite source population is even smaller. Same-denominator prevalence changes approximately as follows:

- random: 47.7%
- easy: 78.4%
- normal: 45.0%
- hard: 12.0%

The global effort ordering is correctly recognizing unlike-denominator work as more expensive, but because the source domain is tiny, the difficulty label also becomes close to a denominator-format classifier.

### Quadratic equation (2)

The effort graph itself has been repaired and now models PF/factor-pair search and the special factorization strategies. Remaining coverage issues include:

- repeated roots are not generated;
- the external scale is greater than one about 80% of the time despite being mostly redundant for the solution strategy;
- difference-of-squares problems exist in the raw population but become approximately 30% of easy and 0% of normal/hard.

The special form being easier is correct; complete disappearance from other difficulty levels is a coverage concern.

### Decimal addition/subtraction

The raw source population is almost exactly 50:50 addition/subtraction, but scalar selection changes the mix strongly:

- easy: about 73.9% addition / 26.1% subtraction
- normal: about 54.2% / 45.8%
- hard: about 22.2% / 77.8%

This is a clear example where **selection**, rather than generation, makes the difficulty label partially synonymous with the archetype.

### Liar puzzle

31-dimensional direction analysis is not meaningful because the explicit special effort model is just a formula-length number represented as repeated `Identity` operations.

The raw candidate pool is intentionally balanced 3-person:4-person = 1:1, while scalar difficulty selection changes it to approximately:

- easy: 90.4% 3-person
- normal: 50.3% 3-person
- hard: 92.6% 4-person

**Correction:** this is intended difficulty behavior, not a coverage Issue. `うそつきだれだ` is not a layered theme and no 3/4-person quota should be imposed. The candidate-population balance is retained, while easy/hard are allowed to prefer shorter/longer SAT formulas and therefore 3/4-person problems.

---

## 5. Recommended solution for layered / archetypal themes

### Principle

Do **not** optimize generic vector diversity directly.

Instead, distinguish between:

1. **separate curriculum units** — split into separate themes;
2. **archetypes that belong in the same worksheet** — stratify by archetype, then use the existing scalar effort sampler inside each stratum.

Decimal multiplication vs decimal division is case (1): split the theme.

Decimal addition vs subtraction and quadratic factorization forms are case (2). Liar population size is explicitly **not** case (2); its difficulty-dependent 3/4-person skew is intended.

### Proposed model: archetype-stratified effort sampling

A theme may optionally declare a small finite set of pedagogical sampling strata, e.g.:

- decimal add/sub: `Addition`, `Subtraction`
- quadratic factoring: `DifferenceOfSquares`, `PerfectSquare`, `General`

The common sampler should then:

1. determine the target number of final worksheet problems for each stratum;
2. build enough candidate population **inside each stratum**;
3. apply the existing easy/normal/hard scalar order-statistic sampler independently inside each stratum;
4. for random, sample uniformly inside each stratum;
5. merge the selected strata and apply the usual presentation ordering/shuffle.

This makes difficulty conditional on archetype:

> "easy general factorization" vs "hard general factorization",
> rather than globally allowing every difference-of-squares problem to consume the easy tail and disappear from hard.

### Why this is preferable to cosine/PCA diversity selection

- keeps `effort = w · x` as the sole difficulty metric;
- is transparent and testable;
- encodes curriculum intent directly;
- does not require thresholds in 31-dimensional vector space;
- does not force unrelated operation directions merely to maximize geometric spread;
- handles special-model themes such as liar puzzle where vector direction is meaningless.

### Suggested implementation shape

The exact Rust API can be chosen later, but conceptually `ProblemGenerator` should optionally expose something equivalent to:

- a finite list of required sampling strata;
- `sampling_stratum(problem) -> key`;
- optional quota weights for those strata.

This should remain **internal sampler metadata**, not part of the public worksheet schema unless UI/explanation later needs it.

Default policy should be equal coverage when no stronger curriculum ratio is specified. For N problems and K strata, distribute `N / K` approximately equally and deterministically distribute the remainder.

Do not create a Cartesian product of every observable feature. A theme should have one primary pedagogical stratification axis unless there is a strong curriculum reason otherwise. Otherwise the number of required cells explodes.

### Important limitation

Stratification cannot create a missing archetype. For example, quadratic(2) currently generates no repeated-root / perfect-square cases. The generator must first be expanded before that archetype can receive a quota.

Likewise, if an archetype has almost no internal effort range, easy/normal/hard within that archetype may legitimately look nearly identical. Coverage should take precedence over manufacturing artificial difficulty.

### Tests that should accompany this mechanism

For every stratified theme:

- every required archetype appears at every difficulty;
- final per-archetype counts match the declared quota;
- duplicate-prompt guarantees remain intact;
- deterministic regeneration remains exact;
- easy/normal/hard effort separation is checked **within each archetype**;
- raw source imbalance cannot starve a required stratum;
- finite domains fail clearly if a stratum does not contain enough distinct problems.

---

## 6. Current fraction generator population

### 6.1 Operand domain

`fraction_arithmetic_operand_domain()` currently constructs positive non-integer reduced fractions using:

- denominator `2..=9`
- numerator `1..=(10 - denominator)`
- integer-valued reductions removed
- then sort/dedup

This yields only **22 distinct fraction operands**:

`1/9, 1/8, 1/7, 1/6, 1/5, 1/4, 2/7, 1/3, 2/5, 3/7, 1/2, 3/5, 2/3, 3/4, 4/5, 5/4, 4/3, 3/2, 5/3, 7/3, 5/2, 7/2`

Composition:

- 8/22 = **36.4% unit fractions**
- 15 proper fractions
- 7 improper fractions

The constraint is effectively `numerator + denominator <= 10` before reduction. This is a small coefficient-style domain and is much more restrictive than necessary for a general drill population.

### 6.2 Current finite population sizes

For addition/subtraction/multiplication the generator begins with `22 × 22 = 484` ordered operand pairs.

| operation | current result restriction | domain triples before common `problem_key` dedup | actually distinct selectable prompts |
|---|---|---:|---:|
| addition | positive, non-integer result; result need not be in operand domain | 448 | **233** |
| subtraction | positive result **and result must itself be one of the 22 operands** | 36 | **36** |
| multiplication | positive result **and result must itself be one of the 22 operands** | 94 | **48** |
| division | operands expanded with integers 1..9; no integer/integer; answer numerator/denominator <=72 | 878 | **878** |

Addition and multiplication are commutative-canonicalized by `problem_key`, which approximately halves the ordered population.

Fraction worksheets contain 16 problems, so the effective population-to-sheet ratios are approximately:

- addition: 233 / 16 = 14.6
- subtraction: 36 / 16 = **2.25**
- multiplication: 48 / 16 = **3.0**
- division: 878 / 16 = 54.9

The subtraction and multiplication domains are therefore genuinely too small for robust difficulty sampling.

### 6.3 Root cause

The key problem is not merely the 22-element operand set. The largest loss comes from this rule for subtraction and multiplication:

> the reduced answer must also belong to the same 22-element operand domain.

Addition has already been changed to avoid this closure constraint, which is why its domain is much healthier.

Division also uses a separate, broader answer bound and therefore has a much larger population.

Thus the four fraction generators currently use inconsistent philosophies:

- addition: bounded operands, broad valid answer space;
- subtraction/multiplication: bounded operands **and answer forced back into operand domain**;
- division: broader operands and broad answer bounds.

The subtraction/multiplication closure constraint should not be preserved unless there is an explicit curriculum reason for it.

---

## 7. Recommended fraction-generator repair

### Step 1 — decouple operand and answer domains

This should be done before enlarging the operand set.

Keep the current 22 fraction operands temporarily, but stop requiring subtraction/multiplication answers to be members of the operand domain.

With exactly the current operands:

#### subtraction

If all positive differences `left > right` are allowed:

- distinct problems increase from **36 to 231**;
- exact maximum reduced answer numerator: 61;
- exact maximum denominator: 72.

#### multiplication

If all positive products are allowed:

- commutative-distinct fraction×fraction problems increase from **48 to 253**;
- exact maximum reduced answer numerator: 49;
- exact maximum denominator: 81.

This change alone removes most of the population-size problem without changing operand pedagogy.

### Step 2 — add integer operands to fraction multiplication

The current multiplication domain contains **zero integer operands**, despite integer×fraction being a basic part of the fraction-multiplication progression.

If integers `1..=9` are added to the 22 fraction operands, while excluding pure integer×integer problems:

- there are 31 operand values total;
- commutative-distinct products with at least one fraction become **451 problems**;
- exact maximum reduced answer numerator under this domain is 63;
- maximum denominator remains 81.

This is already a comfortable finite population for a 16-problem worksheet.

The multiplication theme can then use archetype stratification if desired, e.g. fraction×integer vs fraction×fraction, so both remain present at every difficulty.

### Step 3 — reconsider the 22-value operand domain only after Step 1/2

The current `numerator + denominator <= 10` rule is still somewhat artificial and unit-fraction-heavy. However, after removing answer closure:

- subtraction would have 231 problems;
- multiplication would have 253 fraction-only or 451 including integer operands;
- addition already has 233;
- division has 878.

These may already be sufficient for alpha.

Therefore the simplest path is:

1. remove result-domain closure;
2. add integer multiplication operands;
3. remeasure distributions;
4. only then decide whether the operand range itself needs expansion.

Avoid simultaneously expanding every bound unless data still shows inadequate coverage. This keeps arithmetic sizes and effort behavior controlled.

### Answer schema consequence

The current subtraction/multiplication `AnswerSchema::Rational` bounds (`max_abs_numerator: 8`, `max_denominator: 9`) are a direct consequence of forcing answers back into the 22-value operand domain.

Removing that closure requires broadening those grading bounds. Bounds should be derived/tested against the actual finite domain rather than retaining the current values accidentally.

For the unchanged 22-value operands, exact observed maxima are:

- subtraction: numerator 61, denominator 72;
- multiplication fraction×fraction: numerator 49, denominator 81;
- multiplication including integers 1..9: numerator 63, denominator 81.

---

## 8. Implementation resolution

The audit recommendations have now been resolved as follows.

1. **Decimal multiplication/division split — fixed.** Separate active themes are registered; old combined ID 18 revision 5 is historical-regeneration only.
2. **Fraction source populations — fixed/superseded.** The compact `n+d<=15` operand domain remains, but fraction/fraction and fraction/integer curriculum units are now separated as described in Section 14.
3. **Archetype-stratified scalar sampler — fixed.** A generic optional layer mechanism is used by decimal add/sub, quadratic equation (2), and the improper-fraction summary.
4. **Liar 3/4-person skew — intentionally unchanged.** The earlier recommendation to layer by population size is withdrawn; this is intended difficulty behavior.
5. **Generic cosine/PCA/farthest-point selection — intentionally not implemented.** No evidence justified replacing scalar effort with geometric diversity optimization.

The scalar effort model therefore remains the difficulty foundation. The added abstraction is narrowly curriculum-aware archetype coverage, not generic operation-vector diversity.

## 14. Fraction-domain follow-up implementation — 2026-08-15

A follow-up experiment tested expanding the elementary fraction operand domain from reduced fractions satisfying `numerator + denominator <= 10` to `<= 15`.

### Decision

The `<= 15` operand domain is adopted. It contains 57 distinct positive non-integer reduced fractions instead of 22. This is sufficient to remove the finite-population bottleneck in subtraction and multiplication without relaxing their result closure.

The resulting finite domains are:

- addition: 1,071 commutative-distinct candidates;
- subtraction: 244 candidates;
- multiplication: 291 commutative-distinct candidates;
- division: 750 ordered fraction÷fraction candidates in the standard unit.
- fraction×integer: 226 candidates in the separate grade-6 unit.
- fraction/integer division: 318 candidates in the separate grade-6 unit.
- `分数総まとめ(仮分数)`: 7,756 candidates across its four operator layers.

Addition is intentionally **not** closed over the same `n+d<=15` operand domain. Doing so leaves only 139 commutative-distinct additions and makes 54.7% of the population equal-denominator, recreating the bias that the earlier addition fix removed. Instead addition keeps its independent result bound `numerator <= 65`, `denominator <= 72`.

### Division investigation

The previous division population became 3,873 candidates after the operand expansion because division used 57 fractions plus integers 1 through 9 as operands and accepted almost every non-integer/integer ordered pair whose result satisfied only `numerator <= 72`, `denominator <= 72`. The large count therefore came from an asymmetric and overly loose result-domain rule, not from the scalar difficulty sampler.

The curriculum redesign separates those operand archetypes instead of leaving them all in the standard fraction÷fraction unit. Standard division now requires two fraction operands and closes results over the compact 57-fraction domain plus positive one-digit integer answers, yielding 750 candidates. Fraction/integer multiplication and division are separate grade-6 units with 226 and 318 candidates respectively. The improper-fraction summary may mix fraction/fraction, fraction/integer and integer/fraction multiplication/division naturally.

For subtraction, multiplication, standard division, and the fraction/integer units, the standard compact answer domain is used where applicable. Addition retains its independent `65/72` result bound. Standard elementary themes present improper values as mixed numbers; the summary intentionally preserves improper-fraction presentation.

Commutative addition and multiplication are canonicalized while constructing the finite domain rather than enumerating both operand orders and relying on later `problem_key` deduplication. This changes no source distribution but makes `finite_distinct_candidate_count()` describe the actual distinct population and avoids redundant effort-graph construction.

Regression tests pin the operand-domain sizes, separated curriculum domains, layered quotas, deterministic generation, duplicate-prompt rejection, and result bounds. In the completed implementation `cargo test -p drill-core` passes 105/105 in the final verification.

## 15. Mixed-number curriculum handling and legacy improper-fraction summary theme — fixed

### Resolution

- Standard fraction addition/subtraction/multiplication/division now use mixed-number presentation and canonical `MixedFraction` answers whenever a positive improper result has a nonzero fractional part. Improper-fraction input remains mathematically correct but receives `MixedFractionFormRequired`, consistent with the existing form-warning policy.
- `分数と整数の掛け算` and `分数と整数の割り算` are separate grade-6 themes. Standard fraction×fraction / fraction÷fraction units no longer need integer operands for curriculum coverage.
- `分数総まとめ(仮分数)` is implemented as a four-layer theme. A 16-problem worksheet receives 4 addition + 4 subtraction + 4 multiplication + 4 division problems at every difficulty, and each layer uses its own scalar-effort selection.
- The summary explicitly retains improper-fraction presentation and permits integer operands in multiplication/division, rendering denominator-one values as integers rather than `n/1`.
- All elementary fraction generators remain nonnegative.

The compact `numerator + denominator <= 15` operand population remains the common basis. The redesign changes curriculum grouping and presentation, not the core effort model.

