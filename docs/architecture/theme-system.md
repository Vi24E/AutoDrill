# Theme System / Metadata Architecture

Theme追加時に、分類・安全性・表示・generator policyを場当たり的に推測しないための設計方針。

## Core principle

**Themeの性質はtyped metadata / capability / policyとして明示する。**

次からfeature detectionしない。

- numeric theme ID
- route slug
- 日本語label / curriculum path
- worksheetに生成されたproblemの内容
- unrelated answer schema
- unrelated layout dimensions

`prompt.kind`でそのProblem variantをrenderする等、型そのものを処理する分岐はこの禁止に含まれない。

## Metadata categories

すべてを1個の`string[]`へ押し込む必要はない。意味ごとにtyped namespaceを分ける。

### Curriculum taxonomy

例:

- `grade_1` ... `grade_6`
- `junior_high_1` ... `junior_high_3`
- addition / subtraction / multiplication / division
- fractions / decimals / negative_numbers
- linear / simultaneous / quadratic equation
- bonus

対象学年は`grade.slug`をparseして後付け生成するのではなく、canonical machine-readable metadataとして保持する。

### Presentation / usage

例:

- column arithmetic
- print recommended
- mixed-number presentation policy
- answer placement
- worksheet ordering / layout profile

### Safety / capability

例:

- elementary curriculum safety
- whether negative values are permitted by the curriculum
- supported answer structures
- domain-specific validation constraints

safety policyを表示labelから推測しない。

### Generator / sampling policy

例:

- layered archetype definition
- layer quota
- commutative dedup semantics
- balanced population policy
- finite-domain sampling strategy
- exhaustive finite domainでのcanonical ordering / random-only shuffle policy

これらはtaxonomy tagである必要はないが、theme moduleが明示的に所有するpolicyとして表現する。九九の各段はこの最後のpolicyを使い、family-owned finite domain `N×1 ... N×9` の9件すべてをworksheetへ出す。d1〜d3はそのsource順を保持し、d4だけをdeterministicにshuffleするため、通常のdifficulty effort rankingは適用しない。


## Unified theme API

Rustのtheme/family moduleは、可能な限り1つの構造化されたdefinitionとしてregistrationとgenerator policyを同じ場所で所有する。現行では `themes/basic_arithmetic.rs`, `fractions.rs`, `decimals.rs`, `equations.rs`, `column_arithmetic.rs`, `liar_puzzle.rs`, `mini_sudoku.rs` がこの境界である。

各moduleが所有するもの:

- registration / current generator revision
- grade / taxonomy / safety / presentation / input capability
- operand・answer domain
- candidate construction / rejection
- archetype / sampling layer / quota
- finite-domain sampling policy / dedup policy
- 真にtheme固有なeffort例外

共通 `generator.rs` は `ProblemGenerator` API、deterministic RNG、bootstrap/difficulty selection、dedup framework、timeout/work budget等のmechanismだけを扱い、numeric theme IDによる教材固有special caseを持たない。複数themeで意味まで同一の式AST構築・exact decimal変換等だけを `generator_support.rs` に置く。

各theme/family moduleは**現行generatorだけ**を`GENERATORS`集合として所有する。registration constructionではnonzeroを保証する`ThemeId` / `GeneratorRevision`を使い、完成した`ThemeRegistration`もprivate fieldとしてそのtyped identityを保持する。外部利用者はgetter経由でのみraw numeric projectionを得るため、validated registrationを任意のpublic field集合へ戻さない。

family集合からのregistry constructionは`LazyLock<Result<Registry, RegistryError>>`として一度だけ実行する。duplicate current theme IDは`RegistryError::DuplicateThemeId`となり、`registration()` / `active_registration()` / generator lookup / Web contract / generation boundaryがfallible resultとして伝播させる。最初のregistry accessで`expect`/`assert` panicして初めて設定不整合を発見する設計にはしない。別の中央matchや二重registryも持たない。既存familyへthemeを追加するとき、numeric ID・revision・metadata・generatorを別の中央表へ追記してはならない。

未使用のtheme-level weight override fieldは持たず、必要な拡張は実際のconsumerが生じた時点でtyped APIとして設計する。sampling policyはvalidated `SamplingStrategy` constructorで宣言し、answer-conditioned domainのnon-empty、layer setのnon-empty/quota整合、constructive multiplierのnonzero、classifierのbounded `LayerIndex`変換を共通frameworkが保証する。callbackがrequested answer/layerを無視した場合も共通frameworkが返却Problemを検証し、`SamplingError`として即時fail closedする。candidate callback自体は`Result<Option<Problem>, GenerationError>`で、通常のrejectionとdomain-contract violationを型で分離する。

### Historic revision rulesets

pre-releaseではhistoric generatorをproduction codeへ残さない。generator revision更新時は旧entry・旧ruleset・旧schema compatibilityを削除し、必要な履歴はGitに任せる。後方互換性がproduct requirementになった時点で、初めてversioned ruleset / compatibility policyを導入する。

## Source of truth

### Rust-owned metadata

生成・validation・reproducibility・safetyに必要なmetadataはRust側をcanonical sourceとする。

理由:

- generatorとsafety validationがRust core内で実行される。
- Web labelやrouteが変わっても数学的挙動を変えてはいけない。
- Rust→Web contractは既にgenerated projectionを持っている。

RustからWebへ必要なmachine-readable curriculum metadataをgenerated contractとして投影する。

### Web-owned metadata

Webにしか意味のないものはWeb theme definitionが所有する。

例:

- route path
- SEO title / description
- Web copy
- purely visual presentation metadata

ただしWeb-onlyと思っていた性質をRustも参照する必要が出た場合、二重記述せずcanonical ownershipを見直す。

## Grade handling

現行implementationではRust `ThemeRegistration.grade` をcanonical numeric gradeとし、generated contract経由でWebへ投影する。次の旧方式へ戻してはいけない。

- `grade.slug = "grade-3"`をparseしてgrade metadataを生成する
- `grade.slug`から数学的/safety policyを推定する
- `curriculum_path`の`"小学"`文字列からelementaryか判定する
- 同じgradeをRust/Webへ別々に手入力する

route slugや`小3` / `中1`の表示文字列、`early-elementary / late-elementary / junior-high`のvisual typography bandは、この1つのtyped gradeから純粋にprojectionする。

## Column arithmetic

筆算themeは明示的な`column_arithmetic` presentation capabilityを持つ。

worksheet全体が筆算かどうかを

```ts
worksheet.problems.every(problem => problem.prompt.kind === 'column_arithmetic')
```

から逆算しない。

`worksheet_grid`をpage-wide方眼・row-major配置・divider suppressionの独立presentation capabilityとし、`column_arithmetic`はその利用者の一つとする。grid puzzleが方眼を再利用するために筆算themeへ偽装してはいけない。

column arithmetic capabilityは筆算固有のanswer placement / work geometry / print recommendationだけを表し、page grid自体のsource of truthにはしない。

個々のProblemを実際にrenderする際は`ProblemPrompt::ColumnArithmetic`を型として処理する。


## Fixed digit-grid input

固定cellへ有限digitを入れるthemeはRust `DigitGridSpec { min_digit, max_digit, cell_count }`をdomain parameterのcanonical sourceとし、`ThemeAnswerContract::DigitGrid(spec)`から`AnswerInputInterface::DigitGrid`へ投影する。Webはこのcontractからkeypad範囲・cell数を取得し、tuple-only MathLive等へ偽装しない。Mini Sudokuは`MINI_SUDOKU_GRID_SPEC`を1箇所だけ定義し、`MiniSudokuGrid`のdigit validation、registration、canonical answer tuple長/digit域のvalidation、solverが試すdigit rangeのすべてが同じspecを参照する。数独固有の2×2 block規則は`ProblemPrompt::MiniSudoku`のdomain semanticsであり、generic input capabilityへ混ぜない。

## Fraction presentation

mixed-number表示対象をnumeric theme ID一覧で管理しない。

Theme presentation metadataとして、例えば次のようなpolicyを持たせる。

- `mixed_number_when_improper`
- `keep_improper_fraction`

具体的な命名・型を実装する際は、既存fraction theme全体との整合を見て決める。

## Theme module ownership

Webは原則1 theme 1 definition fileを維持する。

Rust generatorもthemeまたは密接なtheme familyごとのmoduleへ分離する。

Theme moduleが所有する:

- domain
- candidate construction
- archetype
- rejection rules
- sampling policy declaration
- true theme-specific effort exception

共通frameworkが所有する:

- deterministic RNG
- generic sampling
- generic layer/quota mechanism
- dedup framework
- work/time budget
- common arithmetic/domain helpers
- shared effort primitives

## Adding a new theme

新theme追加時は最低限、次を確認する。

1. canonical curriculum grade / taxonomyは何か
2. Rust/Webで共有すべきmetadataは何か
3. presentation capabilityは何か
4. safety constraintsは何か
5. generator固有policyを共通samplerへ漏らしていないか
6. 既存helper / effort primitiveを再利用できないか
7. Web/printで同じsemantic dataを使っているか
8. ID / slug / labelによるhard-coded feature detectionを追加していないか

この分類自体が曖昧な場合は実装前にユーザーへ確認する。
