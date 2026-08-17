# AutoDrill — Open Issues

この文書を、現時点の未解決 Issue の canonical backlog とする。

- dated audit 文書は発見時点の証拠・調査記録であり、現在の backlog の source of truth にはしない。
- mobile はalphaではsupport対象外。mobile対応は`roadmap.md`のbeta移行条件として管理し、alphaのopen Issueには含めない。
- 症状だけでなく、共通原因・設計上の問題がある場合はそちらを Issue の単位とする。

## Pre-release compatibility policy

AutoDrillは現在ユーザのいない開発段階であり、**過去のgenerator revision / schema / problem-set IDとの後方互換性は要件としない**。

- production公開前は破壊的なschema / generator revision更新を許可する。
- active implementationから置き換えられた旧generator revisionを、historic regenerationのためだけに保持しない。
- schema v4等の旧wire shape、historic generator registry、legacy-only compatibility adapter、旧problem-set ID fingerprint testは、現行実装に不要なら削除する。
- 互換性維持のためにactive codeの設計を複雑化しない。必要になった時点（実ユーザ・保存済みworksheet・公開済みURL等が存在した後）からversion compatibility policyを導入する。
- 下記Closed issueの過去の実装記録に「旧revisionを保持する」「historic再生成を維持する」と書かれていても、それは当時の判断記録であり、**現在の要件ではない**。

この方針により、旧revisionを安全にfreezeすること自体ではなく、**不要なlegacy compatibility machineryを削除して現行仕様を単純化すること**を優先する。

---

Severity:

- **Critical** — 正常利用を阻害する。公開前に必須。
- **High** — 教材品質、正確性、UX、保守性に大きな問題。全単元化・本格公開前に必須。
- **Medium** — 明確な実害または保守コストがある。
- **Low** — 現在のサポート範囲では低優先度、または予定済み改善。

---

## Critical

### C-001 正常な `−` / 分数入力が「式が大きすぎます！」として拒否される

**状態:** Closed (2026-08-16 implementation + independent verification)
**対象:** MathLive input / overflow validation

負数を許可する問題で `−` を入力した場合、および分数を許可する問題で分数テンプレートを入力した場合に、正常な入力でも「式が大きすぎます！」として拒否される。

現行 `AutoDrillApp.tsx` の size validator は MathLive が実際に描画した glyph bounds ではなく、外側 `.answer-box` の矩形を problem cell と比較している。MathLive の構造入力で layout が変化すると false positive が起こり得る。

M-001 の「巨大な小数が実際にはみ出しても検出されない」問題も同じ subsystem の false negative である。

**方針**

- outer frame ではなく、MathLive の実際の painted content bounds を測る共通 validator にする。
- `negative` / `fraction` 等、input interface が許可する構造自体を理由に拒否しない。
- overflow 時だけ直前の accepted LaTeX に戻す。
- browser test で正常な負数・分数と、実際にoverflowする長大入力の両方を検証する。


**実装試行 (2026-08-16 / 検証待ち)**

- `.answer-box` 外枠ではなくMathLive shadow DOMのpainted content boundsを測るvalidatorへ変更した。
- parse/AST size limitとvisual overflow判定を分離し、visual overflow時だけ直前のaccepted LaTeXへ戻す。
- JSDOM回帰testに加え、実Chromeのquadratic input probeで通常の`-1`・`\frac{1}{2}`・root・±入力がsize noticeなしでpaint bounds内に収まることを確認した。ユーザー確認前なのでOpenを維持する。

**独立監査 (2026-08-16)**

painted-content validator自体と正常な`-1` / 分数 / root / ±の実Chrome挙動は確認できた。ただし既存の18桁→19桁回帰はAST size limit経路であり、parseには成功するが実paintだけoverflowする経路を直接検証していない。visual overflow→直前accepted LaTeXへのrollbackを実Chrome regressionで固定するまでOpenを維持する。


**解決確認 (2026-08-16)**

実Chrome regressionを強化し、Rust parserが受理する単純値に対してMathLive paint geometryだけを一時的に巨大化することで、AST size limitとは独立した実visual overflowを発生させた。painted boundsがproblem cellを越えた状態を確認後、production validatorが直前accepted valueへrollbackし、「式が大きすぎます！」を表示し、geometryを戻した後は再びcell内へ収まることを検証した。通常の負数・分数・root・±等が誤拒否されない既存probeも同時に通過したためCloseする。

**再発報告と追加解決 (2026-08-17)**

ユーザー実機で「負の数の計算」の`クリア`時に、入力が消えず「式が大きすぎます！」になる回帰を確認した。負の数theme固有実装ではなく、共通MathLive pathで`deleteAll`後の空answerにもpaint overflow validatorを適用し、caret/placeholder geometryを式本体と誤認していたことが原因だった。theme特例は追加せず、Rust parserが`Empty`を返した入力はvisual overflow判定の対象外とする共通修正へ変更した。JSDOMでは意図的にframeをcell外へ置いた状態でもclearが成功する回帰testを追加し、実Chromeの`負の数の計算(1)`でも`1 -> クリア -> empty`、size noticeなしを確認したためClosedを維持する。

---

## High

### H-001 finite-domain generator が母集団を毎回全列挙し、500ms wall-clock timeout に対して不安定

**状態:** Closed (2026-08-16 implementation + independent verification)
**対象:** Rust generator / WASM generation budget

#### 原因

「静的サイトであり、処理はすべてclient上で完結する」という認識は正しい。サーバ処理や通信が原因ではない。

ただし、現在の finite-domain generator は想定される「16問なら 8N ≒ 128候補程度を抽選する」実装ではない。

`generate_with_generator()` は `finite_distinct_candidate_count()` がある theme では別分岐に入り、**domain の全 index を毎 worksheet で列挙し、それぞれについて Problem / canonical answer / solution graph / effort を構築する**。

現在の例:

- 分数の足し算: 1,071候補
- 分数の引き算: 244候補
- 分数の掛け算: 291候補
- 分数の割り算: 750候補
- 分数総まとめ: 7,756候補

さらに finite loop 内では wall-clock timeout を確認せず、全列挙後に初めて `check_timeout()` する。

したがって、静的サイトにもかかわらず timeout する理由は、**client/WASM 上で必要以上の候補を同期的に構築し、その wall-clock を 500ms budget で判定しているため**である。browser main thread scheduling、CPU throttling、host load の揺らぎもこの 500ms に含まれる。

つまり、認識と異なっていた実装は「client-side かどうか」ではなく、**有限母集団を必要数だけsampleせず、全Problem化している点**である。

**方針**

- finite domain を毎回全列挙してから抽選する方式を廃止する。
- theme が定義する**適切な一様分布**から、Seed に対して deterministic に bootstrap 候補を直接生成する。有限index domainを持つ場合は一様なunique index samplingを使い、構成的に一様生成できるdomainでは全domain materializeを要求しない。
- 原則 `CANDIDATE_POOL_MULTIPLIER * problem_count` を中心にし、layer/diversity/difficulty trim に必要な分だけ追加生成する。
- layered themeは各layerについて同じ原則で必要数だけ生成し、quotaを満たすために全母集団を構築しない。
- domain size を増やしても生成時間が母集団全体にほぼ線形比例しない構成にする。
- sampling distributionはtheme側の明示policyとして所有し、共通samplerは「一様に1候補を引く」「重複排除」「difficulty selection」等の汎用mechanismだけを担当する。
- generator behaviorが変わる場合はgenerator revisionを上げ、旧revisionの再生成可能性を壊さない。
- deterministic work budget と wall-clock watchdog を分離する。
- 長いloopを残す場合はloop内でもbudget確認を行う。
- CPU throttling を含む browser test を追加する。


**実装試行 (2026-08-16 / ユーザー確認待ち)**

- active fraction revisionは有限候補vectorを構築せず、theme側のoperand spaceから一様なpair/indexを直接抽選し、domain条件に対するrejection samplingでbootstrap候補を構成するよう変更した。旧revisionは`ExhaustiveLegacy`として凍結した。
- `分数総まとめ(仮分数)`はlayerごとの成立率差でaccepted populationが偏らないよう、`ProblemGenerator::draw_candidate_for_layer`を使って加減乗除のbootstrap quotaを直接満たす。4layer合計で原則`8N`候補とし、`8N × layer数`の過剰生成を廃止した。
- active constructive fraction poolはprompt単位でdedupしてからdifficulty selectionへ渡し、easy側の同一低effort promptがattemptを消費し続ける経路を除去した。Q6r7 / difficulty 1を含む全registered themeのbroad-seed testが通過している。
- 1,071候補を持つ旧分数足し算について、active revisionは`max_attempts=300`でも生成でき、旧rev4は旧仕様どおりAttemptLimitになる回帰testを維持している。
- `max_attempts=10,000`をprimary deterministic work budgetとし、wall-clockは15,000msのemergency watchdogへ分離した。WASM→JS clock crossingを毎attempt行わず64attemptごと＋phase境界で確認する。
- Chromeで`分数総まとめ`をWASM直呼びし、通常CPUで約1.16秒、6×CPU throttlingで約9.27秒でも15秒watchdog内で生成成功することを確認した。Issueはユーザー確認前なのでOpenのまま維持する。

**独立監査 (2026-08-16)**

productionのactive fraction samplerは全domain materializationを避ける構造へ改善済みで、現行schema v5へ直した監査probeでは分数総まとめを6×CPU throttling下でも約8.1秒で生成できた。一方、repositoryの`verify-browser-layout.mjs`のCPU-throttle probeは`schema_version: 4`をhard-codeしており現行新規生成を検証できない。また通常CIではthrottle probe自体を有効化していない。現行generated contractからschemaを取得し、CIで代表高コストthemeのthrottle regressionを実行するまでOpenを維持する。


**解決確認 (2026-08-16)**

browser generation probeの`schema_version: 4` hard-codeを廃止し、productionがgenerated contractから使う`DRILL_SCHEMA_VERSION`をbrowser runtimeへ公開してprobeも同じcurrent schemaを読むようにした。GitHub Pages CIのbrowser verificationで`AUTODRILL_CPU_THROTTLE_RATE=6` / theme 23を常時実行するようworkflowへ追加した。current schema v5の6×CPU probeはfull browser監査で約7.46秒で成功し、その後78 worksheet samplesとprint probesも完走したためCloseする。

---

### H-002 theme 固有 generator 実装が巨大な共通 `generator.rs` に集中している

**状態:** Closed (2026-08-16 independent audit)
**対象:** Architecture / maintainability

`crates/drill-core/src/generator.rs` は約5,800行あり、多数の theme 固有 domain、candidate construction、archetype、特殊 sampling が同一ファイルへ集中している。

さらに共通 sampler 自体にも、

- `THEME_ID_LIAR_PUZZLE` なら 3人/4人を半々にする
- `THEME_ID_MULTIPLICATION_TABLE` だけ commutative canonicalization を変える

という numeric theme ID 特例が存在する。

**原則**

> **themeに関する情報は、原則としてそのthemeのmodule/fileへ構造化して集約し、共通frameworkから統一APIで参照する。**

単にgenerator本体だけをtheme fileへ移すのでは不十分である。各theme file（または密接なtheme family file）は、可能な限り1つの構造化されたtheme definition/APIとして次を所有する。

- registration / generator revision
- curriculum / safety metadata
- operand / answer domain
- candidate generation
- archetype / layer definition
- rejection条件
- sampling distribution / sampling policy
- dedup semantics
- presentation上Rust/Web双方で共有すべきcapability
- 真にtheme固有なeffort例外

共通generator/sampler側はnumeric theme IDや表示名を見て特例分岐せず、この統一APIのみを通してtheme policyを取得する。新theme追加時に巨大matchや複数registryを何箇所も編集しない構造を目標とする。

共通 layer に残すもの:

- deterministic RNG
- generic candidate sampling / selection
- difficulty selection
- dedup framework
- layer/archetype quota の汎用mechanism
- timeout / work budget
- 共通 arithmetic/domain helper
- generator trait/interface
- 共通 effort primitive / standard solution helper

各 theme が所有するもの:

- operand / answer domain
- archetype
- candidate construction
- theme 固有 rejection 条件
- layer quota 宣言
- theme 固有 sampling policy
- 真にtheme固有な effort 例外

また、fraction / quadratic 等が単に同じ `16問 / 2x8` であるため `LINEAR_EQUATION_*` layout constant を借用している箇所も、意味的に中立な layout profile へ整理する。


**実装試行 (2026-08-16 / ユーザー確認待ち)**

- `ThemeRegistration`と`ProblemGenerator`を統一theme APIとして整備し、basic arithmetic / fractions / decimals / equations / column arithmetic / liar puzzleのregistration・domain・candidate generation・sampling layer/policy・theme固有effortを各theme/family moduleへ移した。
- 共通`generator.rs`のproduction部分は約5,800行規模のtheme混在状態から約1,050行まで縮小し、deterministic RNG、bootstrap/difficulty selection、dedup、layer quota、budget等のmechanism中心にした（残りは主に共通回帰test）。
- numeric theme ID / generator revision / skill ID / curriculum path / layout compatibility定数の**定義元**も各theme/family moduleへ移し、`model.rs`は旧参照向けcompatibility re-exportだけにした。
- 各family moduleがactive/historicを含む`GENERATORS`集合まで所有し、`GeneratorEntry::active` / `historic`を明示する。中央のactive registry・historic match・generator registryの二重管理を廃止し、中央は6 familyのgenerator sliceをchainするだけとした。active revisionをrevision番号の最大値から推測せず、この明示metadataから取得する。既存familyへthemeを追加する場合、Rust側のtheme情報はそのfamily moduleだけで完結する。
- layered theme向けの`draw_candidate_for_layer`、bootstrap multiplier、dedup policy等も統一APIのcapabilityとして追加し、numeric theme ID特例を共通samplerへ戻さない構造にした。
- 複数themeで意味まで同一のprimitiveだけ`generator_support.rs`へ残した。Issueはユーザー確認前なのでOpenのまま維持する。

**独立監査によるClose根拠 (2026-08-16)**

共通`generator.rs`のproduction責務はgeneric sampling / difficulty / dedup / layer quota / budget中心へ縮小され、theme固有domain・candidate construction・sampling policy・revision集合はfamily moduleが所有している。共通samplerに教材固有numeric theme ID特例も残っていないことを確認した。

---

### H-003 metadata / tag を使うべき性質を、theme ID・slug・prompt内容・表示path等から逆算している

**状態:** Closed (2026-08-16 independent audit)
**対象:** Theme metadata architecture / Rust-Web contract

これは単なる `curriculum_path` の日本語文字列依存の問題ではない。**theme/category/capability を明示metadata/tagとして持つべきなのに、別の特徴量から推測している箇所全体**を対象とする。

#### 確認できた具体例

1. **Rust側には theme tag 自体がない**

   `ThemeRegistration` / `WebThemeContract` には `numeric_theme_id`, `skill_id`, `curriculum_path`, layout, effort override しかなく、Web側の `tags` と共通の machine-readable taxonomy/capability が存在しない。

2. **小学生の負数禁止を `curriculum_path` の `starts_with("小学")` から推定**

   safety policy が表示・分類文字列に依存している。

3. **対象学年 tag を保存せず `grade.slug` から生成している**

   `derivedGradeTag()` は `grade-3` 等をparseして `grade_3` 等を後付け生成する。これは「対象学年もタグで管理する」という方針と逆である。

4. **学年帯 typography も `grade.slug` を正規表現でparseして推定**

   `worksheetGradeBand()` は `grade-1` 等から early/late elementary / junior-high を逆算している。

5. **画面の `小3` / `中1` tag 表示も `grade.slug` をsliceして生成**

   theme の明示tagを読んでいない。

6. **学年情報が複数箇所へ重複**

   各themeには `grade`, `route.gradeSlug` があり、さらに `drill-engine.ts` で `jp-grade-3` 等を curriculum path ID として手入力している。1つのcanonical metadataから派生していない。

7. **帯分数表示対象を numeric theme ID のhard-coded setで判定**

   `problem-format.ts` に `MIXED_FRACTION_THEME_IDS = new Set([9, 10, 11, 12, 21, 22])` がある。これは典型的なfeature flagの逆算であり、theme tag / presentation metadata にすべきである。

8. **筆算worksheetかどうかを generated problem 全件の `prompt.kind` から推定**

   `layout.ts`, `AutoDrillApp.tsx`, `worksheet-pdf.tsx` の3か所で、

   `worksheet.problems.every(problem => problem.prompt.kind === 'column_arithmetic')`

   によって row-major 配置、divider抑制、CSS class等を決めている。

   しかし `column_arithmetic` tag は既に全筆算themeへ付与済みであり、production側がそれを使っていない。なお PDF test 側では実際に `definition.tags.includes('column_arithmetic')` を使っており、productionとtestの設計が不一致である。

9. **equation用cell stylingを prompt kind のunionから推定**

   Web/PDFで `linear_equation || quadratic_equation || simultaneous_equation` をまとめて `isLinearEquation` とし、layout/CSS挙動を決めている。theme側には既に `equations` tag があるため、worksheet/theme-level presentationならtagまたは明示presentation metadataを使うべきである。

10. **共通generatorが numeric theme ID から theme 固有sampling性質を推定**

    `liar_puzzle` の population balance や multiplication-table の dedup semantics が共通 sampler の ID 特例になっている。これは H-002 と重なる。taxonomy tag である必要はないが、theme側の明示policy/capabilityとして所有すべきである。

#### 重要な区別

`prompt.kind` による分岐をすべて禁止するわけではない。

例えば `ProblemExpression` が `column_arithmetic` prompt を long-division renderer へ渡す、`liar_puzzle` prompt の固有fieldsを描画する、といった**problem variantそのものを処理する分岐は正当**である。

問題なのは、

> worksheet/theme 全体の taxonomy、対象学年、layout policy、presentation policy、safety policy を、生成済みproblemやslug/ID/stringから逆算すること

である。

#### 必須方針

- 対象学年を含む theme taxonomy は explicit tag として持つ。
- safety/capability/presentation のように型を分ける価値があるものは、単一の雑多な `tags: string[]` に詰め込まず typed metadata/tag namespaces にしてよい。
- Rust と Web で共有すべき metadata は Rust registry / generated contract 等、1つのcanonical sourceから投影する。
- route slug、表示label、日本語curriculum path、numeric theme ID、生成済みpromptの集合を feature detection に使わない。
- 同じ事実を `grade`, `route.gradeSlug`, `jp-grade-N`, derived grade tag のように複数手入力しない。
- test で「変更した表示名やroute slugによってgeneration/safety/layoutが変わらない」ことを保証する。


**実装試行 (2026-08-16 / 検証待ち)**

- Rust `ThemeRegistration`へtyped grade/taxonomy/safety/presentation/dedup/answer contractを追加し、generated Web contractへ投影した。
- Web theme fileからgrade/tags/promptKind/answerSchemaKind/inputInterface等の重複手入力を除去し、route/SEO/UI copy中心へ縮小した。
- mixed fraction、column worksheet ordering、equation layout、elementary safety等のfeature detectionを明示metadataへ置換した。production grepで既知の逆推定patternが消えたことは確認済みだが、独立再監査前なのでOpenを維持する。

**独立監査によるClose根拠 (2026-08-16)**

Rust `ThemeRegistration`のtyped grade/tags/safety/presentation/dedup/answer contractがgenerated Web contractへ投影されている。旧来のmixed-fraction ID set、`小学`文字列safety判定、generated problem集合からのcolumn worksheet判定等のfeature inferenceがproductionから除去されていることを確認した。

---

### H-004 筆算の数字・解答欄・あまり欄がページ方眼の座標系へ正しく揃っていない

**状態:** User confirmation pending (technical verification passed 2026-08-17)
**対象:** Column arithmetic Web/PDF layout

確認済み症状:

- 入力した解答欄の数字が見えない場合がある。
- 数字glyphが方眼cellの中心に見えない。
- 割り算の「あまり」解答欄が方眼からずれる。
- 解答欄とページ全体の方眼が単一の座標系に乗っていない。

長除法 remainder には flexible column と `0.25 * grid-cell` の gap があり、ページ方眼へ整数cell単位でsnapする保証がない。

**方針**

- 筆算ではページ方眼を唯一の座標系とする。
- operand digit / operator / quotient / answer digit / remainder field / solution steps を `--worksheet-grid-cell` の明示cell laneで配置する。
- borderだけでなく、描画された数字glyphの視覚中心が対応cell中心へ一致することを検証する。
- Web問題面、print問題面、print解答面で同じ coordinate model を使う。


**実装試行 (2026-08-16 / 検証待ち)**

- 筆算lane右端を0.35cell等のvisual insetではなくページ方眼のgrid lineへ直接snapする座標modelへ変更した。
- answer spacer、partial/final rule、remainder gap等のfractional-cell補正を整数cell単位へ整理した。
- 実Chromeで二桁割り算2 Seedとも`crossings=0`, `gridMismatches=0`、小数加減・二桁乗算・二桁除算のprintでもcrossing 0、実PDFはいずれも2 pagesを確認した。ユーザー視覚確認前なのでOpenを維持する。

**独立監査・ユーザー実画面確認 (2026-08-16)**

機械監査ではcolumn crossing / grid mismatchの多くは解消していたが、full browser verifierは二桁筆算足し算で旧`2–12px` gap assertionと新しいgrid modelが不一致のままFAILしていた。さらにユーザー実画面確認で、自動testが拾えていなかった次の問題を確認した。

- answer inputの下辺が方眼cellから外れて見える。MathLiveの連続数値fieldを方眼へ載せる設計自体が不自然。
- 長除法の曲線と上横線が別primitive（SVG + CSS border）で、割り算記号が視覚的につながっていない。
- 採点時に筆算だけuser answer fieldをDOMから消してcompleted solutionへ置換しているため、各問題の○×・自分の誤答・正答の対応が分からない。
- 加減乗で通常の文字列入力を使うと、`4`→`2`のような筆算として自然な入力（1の位4、10の位2）を表現できない。

**追加必須方針**

- 筆算の最終解答はMathLiveの1個の連続fieldではなく、**方眼1桁ごとの独立editable slot**として実装する。
- 加算・減算・乗算、および余りは一の位slotから開始し、digit入力ごとに右→左へ進む。長除法の商は計算順に合わせ左→右へ進む。各slotはpointerで直接選択可能にする。
- digit slotの値から通常のtyped AnswerNodeを構成し、数学的gradingは従来どおりRustをsource of truthとする。桁editorはWeb interaction/presentationだけを所有する。
- 採点後はuser inputを消さずread-onlyで残し、問題ごとの○/×を明示する。不正解は正答を別表示する。worked solutionでuser answerを置換しない。
- 長除法の括弧は曲線と横線を単一の連続rendererとして描画する。
- Web問題面・採点後・print問題面・print解答面で同じgrid coordinate invariantを使い、magic-number gap assertionを廃止する。
- 実Chrome screenshotを含む最終UX確認を必須とする。


**技術的修正完了・ユーザー再確認待ち (2026-08-16)**

- Web筆算の最終解答をMathLiveの連続fieldから、方眼1cell=1digitの独立editable slotへ置換した。slotは同じ`--worksheet-grid-cell`を直接使用し、下辺・左右境界もpage gridと同一座標系に乗る。
- 加減乗とあまりは一の位から右→左、長除法の商はRust `worked_solution`から求めた最初の商位置から左→右へfocusが進む。各桁はpointerで独立選択でき、物理keyboardでもdigit / 矢印 / Backspace / Delete / Enterを操作できる。 Enter/確定では4列worksheetのrow-major順に次の問題へ進み、次問題の自然な開始桁（加減乗なら一の位、割り算なら最初の商位置）を選択する。
- 桁draftはWeb interaction stateとして保持し、完成/未完成の状態からtyped `AnswerNode`を構成する。数学的gradingは従来どおりRustがsource of truthで、Web側に答え判定を複製していない。
- 採点後もuser input slotをread-onlyで残し、各問題に明示的な`○` / `×`を表示する。不正解では`正: ...`を別表示し、interactive worksheetをworked solutionへ置換しない。print解答面のworked solutionは従来どおり維持した。
- 長除法記号はCSS borderとSVG curveの分離を廃止し、曲線から横線まで1本のSVG pathで描画するようにした。
- browser verifierの旧`2–12px` gap magic numberを廃止し、slotのtop/right grid alignment、隣接gap=0、cell正方形、font/grid consistencyを検証するようにした。
- full Chrome監査で78 worksheet samplesすべてcrossing 0 / grid mismatch 0。筆算加算・割り算の専用interaction probeと、4系統のprint/PDF probeも通過した。

技術的Close条件は満たしたと判断するが、筆算の視覚品質・操作感はUX Issueなので最終Closeはユーザーの実画面確認後とする。

**ユーザー再確認による再修正 (2026-08-17)**

ユーザー実画面で、加減乗のdigit selectionとは別に旧DOM focus ringが元の桁へ残り「boxが正しく移動しない」ように見えること、answer glyphがcell境界へ食い込むこと、長除法記号の曲線形状、余り入力・採点後表示が不自然であることを確認した。

- `selectedDigit`変更時に実DOM focusも同じslotへ移し、focus ringとselection boxを単一化した。物理keyboard入力でも`document.activeElement`とselected slotが一致するbrowser regressionを追加した。
- answer digit fontをgrid cellの約78%へ縮小し、slot自体を`overflow:hidden`とした。glyphが方眼からはみ出す構造を禁止し、slot size / font scaleをbrowser regressionで固定した。
- 長除法の左曲線を、上下端が同じxで中央だけ右へ膨らむ`)`型のBezier `M 0 28 C 11 21 11 7 0 0`へ変更し、そのまま上横線へ接続した。
- 商は従来どおり独立digit slotで左→右入力するが、最終商digitを入力した時点で**自動的に余り欄へ移動**する。
- 余りは筆算grid内部ではないためdigit-slot editorから外し、既存の共通MathLive numeric fieldを再利用する通常big-endian入力へ戻した。`2`,`1`で`21`になることを実Chromeで確認した。
- 採点時の`正: ...`別表示を廃止した。不正解ではcanonical answerを**同じanswer gridの中へ赤字で描画**し、割り算の余りも同じ余りfield内を赤字canonical valueへ切り替える。別correct-answer文字列はDOMに生成しない。

実Chromeの加算・除算probeはともにpassし、加算ではfocus追従・grid alignment・red in-grid correction、除算では商→余り自動遷移・big-endian余り・`)`型記号・red correctionを確認済み。最終的な教材としての視覚評価のみユーザー再確認待ちとする。

**ユーザー再確認による仕様訂正 (2026-08-17)**

採点後に正答を同じslotへ置換すると、ユーザーが実際に何を入力したか消えてしまうため不可とされた。採点後表示を次の不変条件へ変更する。

- user answer / column draftは採点後もread-onlyでそのまま保持し、canonical answerで上書きしない。
- 不正解時のcanonical answerは**別の赤字correction rowとして同じ方眼座標系内**へ描画する。user rowとcorrection rowをstate上もDOM上も分離する。
- 長除法も商・あまりのuser answerを保持し、correct quotient/remainderはそれぞれ隣接する赤字correctionとして表示する。
- 長除法記号は現在の`)`型を維持しつつ、ユーザー提示の標準的な筆算表記へ近づけるため曲線の張り出しを右へ寄せ、Bezierの曲率を弱める。
- browser regressionでは採点前後でuser valueが同値であることと、canonical correctionが別要素として赤字表示されることを同時に検証する。

この変更もUX最終確認前なのでH-004はCloseしない。


**ユーザー再確認・共通grid規格の補強 (2026-08-17)**

- 1桁掛け算でanswer slotが背景方眼から1cellずれて見える再発を確認した。原因は座標primitiveそのものではなく、掛け算rendererだけが常に1cellの`work-space`を追加しており、operator固有DOMの高さがanswer row位置へ漏れていたこと。
- `columnArithmeticWorkingRows(problem)`を共通presentation policyとして追加し、作業行数をgrid cell整数単位で一元管理する。1桁掛け算は0行、複数桁掛け算だけ必要な作業行を確保する。operator rendererが独自pixel/gapでanswer位置を変えないことをregressionで固定する。
- 長除法のworked solutionで各減算段に`−`を表示していたが、日本の一般的な筆算表記では不要なため削除する。減算を示す横線と桁配置だけを表示する。
- 今後H-004をCloseするには、加減乗除すべてについて「operand / rule / answer / worked-solutionの各rowが共通worksheet gridの整数cell座標だけから決まる」ことをbrowser invariantとして確認する。

---

### H-005 筆算の途中計算・solution stepsをWeb側が再計算している

**状態:** Closed (2026-08-16 independent audit)
**対象:** Rust/Web domain boundary / column arithmetic solution presentation

`apps/web/src/components/ProblemExpression.tsx` では、筆算の解答表示時にTypeScriptが数学的な途中計算を再構築している。

確認例:

- 掛け算の部分積をWeb側で再計算する。
- 長除法について、桁を下ろす、商の桁を決める、掛ける、引く、余りを得る、という各stepをWeb側で再計算する。

これは単なるpixel/layout処理ではなく、**数学的solution semantics**である。canonical answerやgradingをRustが所有していても、解答ページの途中式だけWeb独自計算になっているため、Rust/Webのsource of truthが分裂している。

**方針**

- 筆算のworked solutionをRust coreのtyped semantic dataとして生成する。
- 例として `ColumnArithmeticSolution` / typed solution trace等をProblem DTOまたはsolution DTOから取得できる構造にする。
- Rustは途中計算の数学的値・step順を所有する。
- Webはそのtyped dataを同一のcolumn coordinate model上へ配置するだけにする。
- CSS/pixel geometryはRustへ持ち込まない。
- Web問題面 / print問題面 / print解答面で同じsemantic solution dataを使う。


**実装試行 (2026-08-16 / 検証待ち)**

- Rust `Problem`へoptional typed `worked_solution`を追加し、掛け算の部分積と長除法のsemantic stepsを`themes/column_arithmetic.rs`で生成するようにした。
- Web `ProblemExpression`から部分積・長除法の数学的再計算を削除し、Rust DTOを配置するrendererへ変更した。pixel/CSS geometryはWeb側に残した。
- active筆算themeをrev2へ上げ、rev1は`worked_solution`なしのhistoric generatorとして保持した。実Chrome printで二桁乗算のpartial products 32、二桁除算のsolution steps 24をRust DTO経由で描画できることを確認した。ユーザー確認前なのでOpenを維持する。

**独立監査によるClose根拠 (2026-08-16)**

掛け算partial productsと長除法semantic stepsはRust `worked_solution`が所有し、Webはtyped DTOのgrid配置だけを行う。active筆算rev2 / historic rev1のrevision分離とhistoric fingerprintも確認した。

---


### H-006 開発段階では不要なhistoric generator / legacy schema互換コードを削除する

**状態:** Closed (2026-08-17 post-fix verification)
**対象:** Rust generator registry / schema compatibility / WASM contract / tests / docs

現在のrepositoryは、過去のgenerator revisionやschema v4 problem-set IDを再生成するために、active generatorとは別にhistoric generatorを保持している。具体的にはfraction / decimal / column arithmetic等で旧revision registration、legacy ruleset、schema v4 operation-vector projection、historic fingerprint regression等が存在する。

これは公開後の再現性保証としては合理的だが、現在はユーザがおらず開発段階であるため、後方互換性を維持する必要はない。現状ではむしろ、active generatorを変更するたびにhistoric pathも考慮する必要があり、domain / schema / effort / registry / testを複雑化している。

**方針**

- 現行active schema / generator revisionだけをproduction contractとして残す。
- historic generator entry、旧revision専用registration / mode / ruleset、legacy-only regeneration pathを削除する。
- schema v4をhistoric再生成だけのためにsupportしている場合はsupport対象から外し、現行schemaのみへ単純化する。
- `OperationVector`のhistoric wire長projection等、旧schema維持だけを目的とする分岐を削除できるか確認する。
- historic serialized fingerprint / legacy regeneration testは削除する。現行generatorのdeterminism testは維持する。
- Web/WASM側の`regenerate_problem_set`等も、現行product requirementとして使用しないなら削除する。将来必要になった場合は、その時点からversion compatibility policyを設計する。
- closed issueやarchitecture docsに残る「旧revisionを必ず保持する」という旧方針を、pre-release compatibility policyに合わせて整理する。

**注意**

単に旧registrationを消すだけではなく、historic compatibilityのために導入されたschema分岐・helper・test・docsまで追跡して削除する。逆に、現行generationでも使用するexact arithmetic utilityやdeterministic RNG等はlegacy扱いして削除しない。

**Close条件**

- generator registryに現行active implementation以外のhistoric entryがない。
- 現行productionが旧schema / 旧revision再生成を前提としない。
- legacy-only compatibility code / fingerprint fixture / testが残っていない。
- Rust/WASM/Web contractとcanonical architecture docsが現行schemaだけをsource of truthとして一致する。
- current generator determinism・grading・Web/PDF等の回帰testが通る。

**解決確認 (2026-08-17)**

pre-release compatibility policyに従い、fraction / decimal / column arithmeticのhistoric generator、旧revision registration/mode、schema v4 support、31次元OperationVector projection、historic fingerprint test、legacy-only finite-domain materializationを削除した。generator registryはnumeric theme IDごとにcurrent generator 1件だけを持つ構造へ単純化し、この原則を`docs/principles.md`にも明記した。現行problem-set IDのdeterministic regenerationは維持するが、旧revision再生成はproduction requirementとしない。Rust 103 tests + WASM 10 tests、Clippy `-D warnings`がpassし、legacy symbol scanでも旧compatibility pathが残っていないことを確認した。

---

### H-007 Rust/WASM wire DTOをTypeScriptで手書き再定義している

**状態:** Closed (2026-08-17 post-fix verification)
**対象:** Rust-Web contract / generated types / WASM adapter

Rust側がcanonical ownerであるにもかかわらず、Web側で`AnswerNode`, `ProblemPrompt`, `ArithmeticExpression`, `AnswerSchema`, `AnswerInputInterface`, `WorkedSolution`, `ProblemDto`, `WorksheetDto`, editor state/action等のwire shapeを手書きTypeScript unionとして再定義している。さらに`wasm-adapter.ts`が同じshapeをruntime validationとしてもう一度手書きしている。

runtime validation自体は必要だが、schema定義を複数言語で人手同期する構造は、新しいanswer/prompt/worked-solution variant追加時にsilent mismatchを起こしやすい。

**方針**

- Rustのcanonical wire schemaからTypeScript型・discriminant・可能ならvalidator metadataをgenerated artifactとして投影する。
- Web固有のsemantic validationはgenerated schemaの上に置く。
- `input structure` / prompt kind / answer schema kind等のenum値も手書き二重管理を減らす。
- generated codeを直接手編集しない。

**Close条件**

- Rust wire variant追加時に、Web側で同じunion/discriminant一覧を別途手入力する必要がない。
- WASM境界のfail-closed validationは維持される。
- schema generation driftをCIで検出できる。

**解決確認 (2026-08-17)**

canonical Rust wire DTOへ`wire-types` feature時だけ`ts-rs` deriveを付与し、`export_web_wire_types` + `scripts/sync-web-wire-types.mjs`から`apps/web/src/generated/wire/`を生成するようにした。`ts-rs`はoptional tooling dependencyであり、通常のcore/WASM dependency treeには入らない。Webの`AnswerNode` / `ArithmeticExpression` / `ProblemPrompt` / `AnswerSchema` / `WorkedSolution` / `SolutionGraph`等はgenerated Rust typeのalias/projectionとなり、Rust variant一覧をTypeScriptで再入力しない。decimal-stringとしてserializeするi64/u64はRust側の`#[ts(type = ...)]`でwire typeまでcanonical化した。WASM境界のruntime validatorはfail-closed semantic validationとして意図的に残す。contract drift check、TypeScript、ESLint、Web 168 testsがpassした。

---

### H-008 `うそつきだれだ`を誤ってlayered themeとして扱っている

**状態:** Closed (2026-08-17 post-fix verification)
**対象:** Liar puzzle sampling / difficulty semantics

`うそつきだれだ`はlayered themeではなく、easyで3人、hardで4人へ自然に偏ることを許容する仕様である。しかし現行generatorは3人/4人を`SamplingLayerSpec`として宣言し、6問を各3問程度へquota固定するため、difficultyによる人数分布の変化をgeneric layered samplerが打ち消している。

**方針**

- liar puzzleから3人/4人のsampling layer宣言を外す。
- 通常のglobal effort-based difficulty selectionへ戻す。
- 「liar puzzleはlayered themeではない」ことを直接testする。
- generic sampler側へliar puzzle固有theme ID特例は追加しない。

**Close条件**

- liar puzzleにlayer quotaが適用されない。
- easy/hardで3人/4人の比率がeffortに応じて変化できる。
- 既存の問題品質・determinism testが通る。

**解決確認 (2026-08-17)**

`うそつきだれだ`から3人/4人の`SamplingLayerSpec`宣言とlayer判定を削除し、通常のglobal effort-based selectionへ戻した。`liar_puzzle_is_not_a_layered_theme` regressionを追加し、generic layered samplerへtheme ID特例は追加していない。Rust test suiteで問題生成・determinismを含めpassした。

---

## Medium

### M-001 大きすぎる小数入力が枠からはみ出しても拒否されない

**状態:** Closed (2026-08-16 implementation + independent verification)

C-001 と同じ overflow validator の false negative。outer `.answer-box` ではなく実際の MathLive painted content bounds を測る共通validatorへ統合する。


**実装試行 (2026-08-16 / 検証待ち)**

C-001と同じpainted-content validatorへ統合した。18桁を保持し次の桁で安定して拒否するWeb testが通過し、実Chromeでも正常な構造入力が誤拒否されないことを確認した。長大overflowの追加ユーザー確認前なのでOpen。

**独立監査 (2026-08-16)**

C-001と同じ理由で、現在の回帰testはAST size rejectionと正常入力containmentを検証する一方、parse成功後の純粋なvisual overflowを直接固定していない。実paint overflow regression追加までOpenを維持する。


**解決確認 (2026-08-16)**

C-001と同じ実Chrome visual-overflow regressionで、parse成功ケースのpaint overflow→rollback経路を直接固定した。18桁入力保持/次の桁のsize-limit rejectionという既存境界testも引き続き通過しているため、false positive/false negative両側の再発防止が揃ったと判断してCloseする。

---

### M-002 入力キーボードの主要key群が視覚的に中央からずれている

**状態:** User confirmation pending (technical verification passed 2026-08-17)

`.input-panel-inner` 自体は中央寄せだが、3 column 幅が非対称であるため数字keypad/主要操作の視覚重心がworksheet中心からずれる。

magic-number offsetではなくgrid構造を整理し、主要入力部を中央へ合わせる。


**実装試行 (2026-08-16 / 検証待ち)**

入力panelの3列を左右対称`minmax(... ) / 210px / minmax(...)`へ変更し、numeric keypadを幾何学中心trackへ置いた。実Chrome probeでも中心deltaが許容範囲内であることを確認した。ユーザー視覚確認前なのでOpen。

**ユーザーUX確認 (2026-08-16)**

幾何学的中央性は改善しているが、`+ / − / ±`の演算子keyが構造入力keyと同じ大きな占有面積を持ち、余白が過剰で視覚的に重い。構造keyと演算子keyを同一サイズで扱わず、演算子keyを数字keyに近いcompact controlとして再配置する。修正後に再度ユーザー確認する。


**技術的修正完了・ユーザー再確認待ち (2026-08-16)**

分数・平方根・複数解のstructure keyは64pxを維持する一方、`+ / − / ±`は42pxのcompact operator rowへ分離した。numeric keypadの幾何学中心は維持し、browser regressionではstructure row / operator rowそれぞれの高さ一致と、operator rowがstructure rowより十分小さいことを検証している。最終的な視覚バランスのみユーザー再確認待ち。

**ユーザー再確認による再修正 (2026-08-17)**

42pxのoperator rowをstructure keyの下へ置く案も余白が不自然との指摘を受け、layoutを再設計した。`+ / − / ±`はformula-keypadから外し、**数字0と同じ最下段**へ統合した。数字領域を6 subcolumnsとして、上3段の各digitは2 span、0は3 span、`+ / − / ±`は各1 spanとする。分数・平方根・複数解のstructure keyは3個を横並びにし、それぞれの高さを`7 / 4 / 1`の3数字row合計（3×digit height + 2×gap、現状138px）へ一致させた。実Chrome regressionでstructure heightと3row合計の差2px以内、operatorと0のtop一致、operator heightとdigit height一致を検証している。最終視覚確認待ち。

**ユーザー再確認による再設計 (2026-08-17)**

Desmos型の安定した操作配置を参考にし、中学生範囲はthemeごとにkeyの位置を大きく変えず、原則として同一のfull keypad shellへ固定する方針とする。

- 左: 分数 / 帯分数 / 平方根 / 複数解等のstructure key。中学生themeでは同じ位置へ固定し、当該input contractで許可されないkeyはdisabled表示にする。
- 中央: `7 8 9 / 4 5 6 / 1 2 3 / 0 .` の数字領域。小数点も位置は固定し、非対応themeではdisabledとする。
- **数字と制御の間**: `+ / − / ±`を数字keyと同程度のcompactな縦stripとして配置する。
- 右: 左右移動 / delete / clear / commit等のcontrol。
- Web UIがRust input capabilityを無視して不正なAnswerNodeを作らないよう、固定shellと「keyが有効か」は分離する。

代表として負の数・一次方程式・連立方程式・二次方程式で同じshellが描画されるbrowser regressionを持つ。最終視覚確認前なのでM-002はOpen扱いを維持する。


**ユーザー再確認 (2026-08-17)**

- 中学生範囲ではテーマごとに構造keyをdisableして見た目を変えず、原則として同じfull keypad shellを保つ。
- 構造keyは **2×2**（分数 / 帯分数 / 平方根 / 複数解）で固定する。帯分数だけをテーマ都合で灰色化しない。
- `+ / − / ±`は数字とcontrolの間のcompact縦列を維持する。
- 最終Closeは、中1負の数・一次方程式、中2連立方程式、中3二次方程式で同じ2×2 shellと列順が維持されることを実画面確認後とする。

---

### M-003 入力キーボードを明示的に閉じることができない

**状態:** Closed (2026-08-17 user confirmation)

問題選択後にinput panelをしまう操作がない。入力内容を保持したまま閉じられる明示controlを設ける。`Esc` 等のkeyboard操作も検討する。


**実装試行 (2026-08-16 / 検証待ち)**

入力内容を保持したまま閉じる明示buttonと`Escape`操作を追加し、active MathLiveをblurしてselectionだけ閉じるようにした。Web testsと実Chromeのclose/Escape probeが通過。ユーザー確認前なのでOpen。

**ユーザーUX確認 (2026-08-16)**

close/Escapeの機能自体は動作確認済み。ただし文字の「閉じる」はpanel controlとして野暮ったいため、下向きchevron等の収納アイコンへ変更する。accessibility上の`aria-label=入力パネルを閉じる`とEscape動作は維持する。


**技術的修正完了・ユーザー再確認待ち (2026-08-16)**

文字の「閉じる」を削除し、panel右上を下向きchevron SVGの収納iconへ変更した。button textが空でSVGを持つこと、`aria-label=入力パネルを閉じる`、Escapeで閉じることを実Chrome regressionで検証済み。最終的なicon位置・自然さのみユーザー再確認待ち。

**ユーザー確認 (2026-08-17)**

下向きchevronの収納buttonについて「良い」と確認されたためCloseする。`aria-label=入力パネルを閉じる`とEscape操作は維持する。

---

### M-004 分数割り算 `x / x` を独立した effort primitive として扱っていない

**状態:** Closed (2026-08-16 independent audit)
**対象:** effort model

同じ分数を同じ分数で割る `x / x` は、一般の分数割り算と同じ primitive sequence として重く評価すべきではない。

**方針**

- `x / x` を独立した operation basis として扱う。
- weight は **1** とする。
- fraction division の hard selection にこの形が不当に押し上げられないことを統計testする。


**実装試行 (2026-08-16 / 検証待ち)**

active fraction division/summaryの`x/x`へ独立`FractionSelfDivision` primitive（weight 1）を追加し、OperationVectorをschema v5で32次元へ拡張した。既存index 0〜30は不変で、新basisは末尾index 31である。保守性上の根本問題だった「現行vector shapeとhistoric wire shapeの同一視」を解消するため`schema.rs`へschema→次元mappingを集約し、内部は現行32次元、serialize時はidentity schemaへlossless projectionする構造へ変更した。新規生成はv5、historic v4 problem-set IDは31次元のまま再生成可能で、既存fingerprint testも一致している。分数割り算はrev6、分数総まとめ(仮分数)はrev3へ上げ、直前rev5/rev2はv4 historic generatorとして保持した。Rust/WASM/Webの全自動確認後もユーザー確認まではOpenを維持する。

**独立監査によるClose根拠 (2026-08-16)**

`FractionSelfDivision`はIdentityと独立、weight=1、schema v5 32次元のindex 31である。schema v4は31次元のhistoric wireを保持し、non-zero新basisをv4へlossy projectionする操作は拒否される。fraction division/summary revision分離、WASM/Web contract、historic fingerprint、実生成worksheetの`x/x` vectorまで確認した。

---

### M-005 GitHub Pages公開版ではNext.jsのsecurity response headersが適用されない

**状態:** Open
**対象:** Deployment security

`apps/web/next.config.mjs` には CSP, `X-Content-Type-Options`, `X-Frame-Options`, `Referrer-Policy`, `Permissions-Policy` が定義されている。

しかし `GITHUB_PAGES=true` の static export では `headers()` を意図的に無効化している。GitHub Pages は任意のNext response headersを配信できないため、**公開版にはこれらのheaderが存在しない**。

client-only設計により server secret / user database 等のattack surfaceは小さいが、clickjacking、script injection時の被害制限、MIME hardening等の defense-in-depth が公開版で欠ける。

**方針**

- GitHub Pagesを本番hostとして継続する場合、静的hostで実現可能なhardeningと不可能なheaderを明示する。
- 独自ドメイン取得とhosting機能を混同しない。独自ドメインをGitHub Pagesへ向けるだけではheader制御能力は増えない。betaで独自ドメインを導入する際は、同時にheader設定可能なstatic host/CDNを選ぶかを判断する。
- 強いresponse-header policyを必須とする段階では、Cloudflare Pages / Netlify / Vercel等、header設定可能なstatic hostへの移行を比較する。
- CSPをmeta tagで代替できる範囲とできない範囲を混同しない。特に `frame-ancestors` はHTTP headerが必要。
- 本番URLに対するdeployment verificationを用意する。


**実装状況 (2026-08-16)**

GitHub Pages自体が任意HTTP response headerを配信できないため、production codeだけで完全修正することはできない。hosting migrationは今回勝手に行わず、Openのまま維持する。
---

### M-006 MathLive parserのrecursion budgetが全recursive pathをcoverしていない

**状態:** Closed (2026-08-16 independent audit)
**対象:** Rust MathLive parser / robustness

現行parserは raw LaTeX 4,096 bytes、`{}` / `[]` nesting、`\frac` / `\sqrt` / `\placeholder` 個数についてpreflight limitを持つ。これは改善されているが、recursive descentには別経路が残る。

例:

- `parse_unary()` による `-----1` / `\pm\pm...` の再帰
- `(...)` から `parse_expression()` への再帰

これらはparse完了後のAnswer AST size checkより前にnative/WASM stackを消費する。

**方針**

- parser自身に明示的なrecursion/nesting budgetを持たせ、すべてのrecursive descent pathで共通に消費する。
- raw byte limitは二次防御として維持する。
- `{}`だけでなくparentheses、unary chain、root/fraction/groupの混合deep nestingをregression testする。


**実装試行 (2026-08-16 / 検証待ち)**

parser自身に共通recursion/nesting budgetを追加し、unary chain・parentheses・group/root/fraction等のrecursive descentで消費するようにした。pathological unary/parentheses regression testを追加しRust testsは通過。独立再監査前なのでOpen。

**独立監査によるClose根拠 (2026-08-16)**

parser共通nesting budgetがunary / parentheses / group / root / fraction等のrecursive descentをcoverし、pathological unary・parentheses・nested structuresのregressionが通過している。

---

### M-007 legacy generator revisionが現行mutable helperを共有し、将来の再生成保証が弱い

**状態:** Superseded by H-006 (2026-08-17 pre-release compatibility policy)
**対象:** Determinism / generator revisioning

**方針変更:** 当初は旧revisionをfreezeして再生成保証を維持する方針だったが、現在はpre-releaseで実ユーザがいないため後方互換性を要件としない。旧revisionを安全に保持する方向ではなく、H-006でhistoric generator / legacy schema互換コード自体を削除する。以下は当時の問題認識・実装記録として残す。

legacy fraction / decimal generatorはrevision番号としてregistryに残っているが、domain構築・candidate construction等の一部を現行helperと共有している。

そのため、現行revision向けhelperを将来refactorすると、**過去revisionの同じproblem-set IDの出力まで変わる可能性**がある。現状testが通ることは、将来のimmutable reproducibilityを保証しない。

**方針**

- revision依存のdomain / sampling policy / construction ruleはrevision-specific implementationとして凍結する。
- 本当にrevision-independentな数学utilityのみ共有する。
- historical representative Seedについてworksheetまたはstable fingerprintのgolden regressionを保持し、旧revisionが変化したら検出する。
- H-001等でsampling algorithmを変更する際は必ず新revisionを発行し、旧revisionをそのまま再生成可能にする。


**実装試行 (2026-08-16 / ユーザー確認待ち)**

H-001/H-005の変更では旧revision generatorを別registration/policyとして保持した。fraction domain/construction、decimal construction、column constructionはそれぞれ`*_v1`の凍結rulesetとして明示し、将来の変更は既存helperを編集せずv2を追加する方針にした。さらにfraction/decimal/columnの代表historic problem-set IDについてserialized Worksheetの長さ+FNV-1a fingerprintを固定する回帰testを維持している。全Rust testでhistoric fingerprintが一致している。Issueはユーザー確認前なのでOpen。

**独立監査によるClose根拠 (2026-08-16)**

fraction / decimal / column arithmeticのrevision依存rulesetは`*_v1`等として凍結され、変更時は新ruleset/revisionを追加する構造になっている。historic serialized fingerprint regressionも全件一致した。

---

### M-008 canonical docsの一部が現行production implementationと一致していない

**状態:** Closed (2026-08-16 implementation + independent verification)
**対象:** README / canonical architecture docs

監査時点で次の不一致を確認した。

- root `README.md` が「5段階難易度」と記載しているが、productionは `1=かんたん / 2=ふつう / 3=むずかしい / 4=ランダム` の4段階。
- `docs/architecture/problem-schema.md` のgenerator revision一覧に古いrevisionが残り、同文書内の別記述とも不整合がある。
- elementary safetyを日本語curriculum path文字列から推測する現行/旧設計の記述が、`principles.md` / `theme-system.md` のtarget architectureと混在している。

canonical docsの不一致はAIエージェントが誤ったrevision追加・互換性変更を行う直接的な原因になる。

**方針**

- **現行production implementationをsourceとしてcanonical docsを同期する。**
- generator revision、schema version、theme一覧等の機械的事実は可能ならgenerated contractまたは検証scriptから生成/検証する。
- target architectureとcurrent implementationを区別して明記する。
- archive文書はcurrent designへの導線に使わない。


**実装試行 (2026-08-16 / ユーザー確認待ち)**

READMEの難易度を4段階へ修正し、`problem-schema.md`のactive revision tableを現行Rust `web_contract()`に同期した。typed safety、constructive/uniform bootstrap、15秒emergency watchdog、worked solution、historic frozen rulesetの説明も現行実装へ更新した。`theme-system.md` / `generator.md`にはfamily-owned generator集合と統一theme APIを反映した。generated Web contractも現行Rustから再同期済み。Issueはユーザー確認前なのでOpen。

**独立監査 (2026-08-16)**

現行productionはschema v5だが、root `README.md` と `docs/architecture/web-print.md` に「schema-v4 JSON DTO」というcurrent boundary記述が残っている。またbrowser verification scriptにもcurrent schemaの手書き`4`が残っていた。canonical docsと検証scriptの機械的schema事実を現行contractへ同期するまでOpenを維持する。


**解決確認 (2026-08-16)**

root `README.md` と `docs/architecture/web-print.md` のcurrent boundaryをschema v5へ同期した。browser verifierのcurrent generation schemaも手書き数値を廃止し、productionと同じ`DRILL_SCHEMA_VERSION`から取得する。canonical docs/scriptsを再検索し、schema v4はhistoric compatibilityの説明としてのみ残っていることを確認したためCloseする。


---

### M-009 割り算の入力順序policyを選択できない

**状態:** Open
**対象:** Division input interaction / typed presentation metadata

現在の割り算系UIでは、長除法の商を計算順に左→右へ入力する「筆算として自然な入力」と、通常の数値欄として常にmost-significant digitから入力するbig-endian方式のどちらを採用するかが実装に固定されている。

今後、教材・端末・利用者の期待によって適切な方式が異なる可能性があるため、挙動をcomponent内部の暗黙ルールにしない。

**方針**

- 割り算系theme/presentation metadataにtypedなinput-order policyを持たせる。
- 少なくとも次を区別できるようにする。
  - `natural_division_flow`: 長除法の商は実際の計算順に左→右等、筆算に即したslot/focus遷移を使う。
  - `big_endian`: 通常の数値入力と同様、常にmost-significant digitから文字列として入力する。
- quotient / remainder / 小数商等の各answer partについて、どのpolicyが適用されるかを明示できる構造にする。
- Web側がtheme IDやprompt値から入力方向を推測せず、typed metadataだけを読む。
- mathematical gradingは入力順序に依存させず、最終的なtyped `AnswerNode`をRustへ渡す既存責務を維持する。
- 代表的な整数除法・小数除法についてbrowser regressionを持つ。

**Close条件**

- policyがtyped metadataとして定義され、themeごとに選択可能。
- natural / big-endianの双方に自動testがある。
- 既存の長除法UXをpolicy選択後も再現でき、grading / print / historic worksheet identityへ不要な影響を与えない。


---


---

### M-010 小数の掛け算の筆算で解答欄の小数点位置が固定されている

**状態:** Open
**対象:** Column decimal multiplication input model / UX

小数の掛け算では、計算結果の小数点位置はoperandの小数桁数から解答時に決める対象であり、解答欄にあらかじめ小数点を固定表示すると学習上のヒントになり、入力モデルとしても不自然である。

**方針**

- 小数点を固定grid markerとして与える現行方式を見直す。
- digit slotとdecimal-point placementを別のeditable semanticとして扱うか、筆算専用editor全体の入力モデルを再設計する。
- 小数の加減算（小数点を揃える）と掛け算（結果の小数点位置を決める）を同一UI規則で無理に扱わない。
- 根本的なinteraction変更を伴うため、今回のH-004修正とは分離して実装する。

**Close条件**

- 小数掛け算で解答前に正答小数点位置が露出しない。
- 小数点位置を自然に入力・修正できる。
- gradingはRustのtyped answerをsource of truthとする。
- Web/printの教材意図が一致し、browser regressionとユーザーUX確認を通す。

---

### M-011 印刷時の解答ページ反転を選択できない

**状態:** Open
**対象:** Print/PDF UX

現在の印刷プレビューでは解答ページを上下反転して表示する方式が固定されている。利用者・印刷用途によっては通常向きの解答ページが望ましいため、反転有無を選択可能にする。

**方針**

- 印刷設定に「解答を反転する / 通常向き」の明示optionを追加する。
- 初期値は現行挙動との互換性と教材用途を踏まえて決める。
- Web print previewと実PDFで同じ設定を使用し、CSSだけの偶発的なtransform分岐にしない。

**Close条件**

- preview上で反転有無を選べる。
- 生成PDFの解答ページorientationが選択と一致する。
- 問題ページには影響しない。
- browser/PDF regressionを追加する。

---


### M-012 input profileの意味をRustとTypeScriptで二重管理している

**状態:** Closed (2026-08-17 post-fix verification)
**対象:** Theme input capability / generated contract

Rustの`ThemeInputProfile`から実際の`AnswerInputInterface`への対応と、Webの`inputInterfaceFromProfile()`が別実装になっている。現にRustには`ImproperFraction` profileがある一方、Web側switchには対応caseがなく、active contractで現在踏まないため顕在化していない。

**方針**

- generated contractにはprofile名だけでなく、Rust側で解決済みのcanonical input interface/capabilityを出力する。
- Webはprofile名からcapabilityを再構築しない。
- 新input capability追加時にRustだけをcanonical変更点とする。

**修正規模:** Medium
**優先度:** 次のinput profile追加前。

**解決確認 (2026-08-17)**

Rust `WebThemeContract`が`input_interface` / `editor_input_interface`を解決済みの`AnswerInputInterface`としてexportするように変更し、Webの`inputInterfaceFromProfile()`を削除した。新しいinput capabilityの意味はRust `generator_support::input_interface`だけで定義される。generated contract drift checkとWeb test suiteがpassした。

---

### M-013 Web/PDFでanswer presentation semanticsが重複している

**状態:** Closed (2026-08-17 post-fix verification)
**対象:** Web/PDF answer rendering

interactive Webの`WorksheetAnswerField`とprint側の`PrintAnswer`が、liar puzzle / column division / ordered pair / simultaneous equation / 通常answer等の意味的な分解を別々に行っている。expression renderingは共有が進んでいるが、answer presentationは二重実装である。

**方針**

- React componentを無理に共通化せず、`Problem -> AnswerPresentationPlan`のpure domain projectionを1つ作る。
- Web rendererとPDF rendererは同じpresentation planを別表示へ投影する。
- 新answer type追加時のWeb/PDF変更箇所を局所化する。

**修正規模:** Medium
**優先度:** 新answer type追加前。

**解決確認 (2026-08-17)**

`Problem -> AnswerPresentationPlan`を`apps/web/src/domain/answer-presentation.ts`へ抽出し、interactive WebとPDFがliar / column division / column arithmetic / simultaneous equation / standard answerの意味的分解を共有するようにした。renderer自体は用途別のまま維持した。専用testとWeb/PDF全体testがpassした。

---

### M-014 筆算grid geometryにCSS側の第二のsource of truthが残っている

**状態:** Closed (2026-08-17 post-fix verification)
**対象:** Column arithmetic layout / CSS geometry

TypeScript側にA4/grid geometryのcanonical値がある一方、CSSにも`--worksheet-grid-cell`相当の導出済み数値やpercentage位置が直接埋め込まれている。A4 margin / header / grid cell変更時にWeb input、worked solution、print背景が別々にずれる危険がある。

**方針**

- canonical geometryからCSS custom propertiesを生成・設定する。
- CSSは`var(--worksheet-grid-cell)`等の意味付き変数だけを参照する。
- glyph optical correctionのような純粋な視覚補正まで無理にdomain化しない。

**修正規模:** Small / Medium
**優先度:** H-004のgrid系修正が落ち着いた段階で統合する。

**解決確認 (2026-08-17)**

`apps/web/src/domain/column-arithmetic-presentation.ts`をcanonical geometry projectionとし、Web/PDFの筆算gridに必要なCSS custom propertiesをそこから設定するよう統合した。CSS側に導出済みgrid寸法・位置を第二のsource of truthとして保持しない構造へ寄せ、純粋なglyph optical correctionはpresentation-localのまま残した。fresh WASM + Next static exportを作り直したうえでbrowser verifierを実行し、78 worksheet samplesすべてで`crossings=0` / `gridMismatches=0`、筆算digit input、掛け算partial products、割り算worked solution、native print PDFもpassした。

---

### M-015 theme追加時のWeb central boilerplateが多い

**状態:** Closed (2026-08-17 post-fix verification)
**対象:** Web theme registry

`theme-registry.ts`では新theme追加時にexport / import / `THEME_DEFINITIONS`列挙の複数箇所を編集し、`drill-engine.ts`にもthemeごとのID/revision/layout/curriculum projection exportが多数存在する。値そのものはgenerated contract由来でありfalse SoTではないが、theme数増加に対する変更波及が大きい。

**方針**

- 明示registry自体は維持してよい。filesystem magic discoveryは導入しない。
- export/import/listの重複と個別derived constantを減らし、typed accessor/data-driven projectionへ寄せる。
- 1 theme追加で無関係な中央switch/matchを複数修正しない構造を目標とする。

**修正規模:** Medium
**優先度:** theme数がさらに増える前。

**解決確認 (2026-08-17)**

`theme-registry.ts`は明示的なtheme列挙・lookupだけに限定し、per-theme re-exportを削除した。さらに`drill-engine.ts`の37 theme分の`*_CORE_CONTRACT` / ID / revision / skill / curriculum / layout中継定数を削除した。各Web theme定義が手入力するcross-language情報はgenerated contractで型付けされた`numeric_theme_id` foreign keyだけで、`defineTheme`がrevision・skill ID・curriculum・layout・capabilityを解決/検証する。theme追加時に無関係な中央projection群を編集する必要はない。TypeScript / ESLint / Web 168 testsがpassした。

---

### M-016 `ThemeRegistration::new()`が多数の位置引数を要求する

**状態:** Closed (2026-08-17 post-fix verification)
**対象:** Rust theme registration API

`ThemeRegistration::new()`がgrade / tags / safety / presentation / dedup / answer contract / layout等を11個程度の位置引数として受け取り、各theme familyで多数回呼ばれている。metadata追加時の位置引数事故やreview負荷が高い。

**方針**

- 過剰なbuilder hierarchyは作らず、named struct literalまたはrequired identityだけのconstructorへ寄せる。
- AIエージェントが各fieldの意味を局所的に読める形を優先する。

**修正規模:** Medium
**優先度:** 次にThemeRegistration metadataを追加・変更するとき。

**解決確認 (2026-08-17)**

11個の位置引数を持つ`ThemeRegistration::new(...)`を`ThemeRegistrationSpec { named_fields... }`へ変更し、全theme registrationを名前付きmetadataへ移行した。layoutも`problem_count / columns / rows`の分解保持をやめ`WorksheetLayoutProfile`として保持する。Clippy `-D warnings`を含むRust全検証がpassした。

---

### M-017 Web test suiteにstale selector / stale fixtureが残っている

**状態:** Closed (2026-08-17 post-fix verification)
**対象:** Web tests / fixtures

保守性監査時点のVitestは164件中158件pass / 6件fail。5件は削除済みの「数式テンプレート」UIを参照するstale selector/期待値、1件はWASM adapter fixtureのinput capabilityがcanonical ThemeDefinitionと不一致だった。PDF testにもtheme ID 1〜37の巨大な手作業representative fixtureがあり、fixture自身がproduction contractを再実装している。

**方針**

- stale selector/fixtureを現行UI/contractへ同期する。
- valid identity / Problem / Worksheet fixture builderを少数のcanonical helperへ集約する。
- 可能ならRust/WASMのdeterministic outputをfixture sourceとして利用する。
- UI testはsemantic roleを優先し、geometry/private DOM selectorへの依存を必要箇所へ限定する。

**Close条件**

- Web test suiteが全passする。
- fixtureが現行schema/identity contractに違反したDTOを正規fixtureとして生成しない。
- theme追加時に37-case switch等へ必ず追記する必要がない。

**解決確認 (2026-08-17)**

stale selector / capability fixtureを現行UI/contractへ更新し、fixtureはThemeDefinition由来のcanonical metadata/capabilityを使うようにした。PDFのtheme ID 1〜37 `switch`も削除し、prompt kind / tags / answer schema等からfamily-level representative fixtureを構成するため、新theme追加時にID別case表への追記を要求しない。fixtureのproblem-set IDも現行schemaから生成する。最終Web suiteは18 files / 168 testsすべてpassした。

---

### M-018 generator最終順序についてcanonical docsと実装が一致していない

**状態:** Closed (2026-08-17 post-fix verification)
**対象:** Difficulty / docs consistency

architecture docsは「最終n問をdeterministic Fisher–Yates shuffle」と記述しているが、現行実装はeasy / normalをeffort昇順に並べ、hard / randomだけshuffleする。実装にはworksheet内で難易度を徐々に上げるという教育上の明示理由と回帰testがあるため、コードが誤りとは断定しない。

**方針**

- intended behaviorを確定し、code / tests / canonical docsを一致させる。
- easy/normalのdifficulty rampを正とするならdocsだけを修正する。

**修正規模:** Small
**優先度:** 今。

**解決確認 (2026-08-17)**

既存実装と回帰testの教育的意図を正として、`かんたん` / `ふつう`はeffort非減少順、`むずかしい` / `ランダム`はdeterministic shuffleとcanonical architecture docsへ明記した。code / tests / docsを一致させた。

---

### M-019 `AutoDrillApp.tsx`がinput state machineを含む多数責務を抱えている

**状態:** Closed (2026-08-17 post-fix verification)
**対象:** Web orchestration / input controller

`AutoDrillApp.tsx`はgeneration, grading, MathLive, simultaneous x/y focus, column digit input, liar selection, correction, print, settings等を一つの大きなcomponentで制御している。単純な行数の問題ではなく、新input mode追加時にrender branch / keyboard dispatch / focus / answer state / grading transitionを離れた箇所で同時変更する必要がある。

**方針**

- top-level orchestrationは残す。
- answer/input state transitionを`useWorksheetAnswerController`等のbehavior単位へ抽出する。
- 細粒度componentを大量生成する過剰分割は避ける。

**修正規模:** Medium / Large
**優先度:** 次の新input type追加前。全面rewriteはしない。

**解決確認 (2026-08-17)**

`useWorksheetAnswerController`を追加し、answer state、selected problem/slot、column draft、same-tick mirror refs、MathLive action queue/accepted LaTeX、mathfield registryを`AutoDrillApp`からbehavior単位で抽出した。generation / grading / routing / printのtop-level orchestrationはAppに残し、全面rewriteや細粒度component乱造は行っていない。controller専用testを追加し、既存`AutoDrillApp` 61 testsを含むWeb 168 tests、TypeScript、ESLintがpassした。

---

### M-020 Web theme identity / route metadataに冗長な手入力がある

**状態:** Closed (2026-08-17 post-fix verification)
**対象:** ThemeDefinition / routing metadata

Web `ThemeDefinition`には`themeKey`, Rust `skill_id`, `gradeSlug`, `themeSlug`, `pathname`等があり、一部は同一事実または導出可能な情報である。`themeKey`と`skill_id`が別概念である必要があるかも型上明確でなく、`pathname`もslugから導出できる。

**方針**

- `themeKey`がcanonical skill IDと同一ならRust contractからderiveする。別概念なら意味を明示する。
- `pathname`等のpure derived metadataは手入力しない。
- route表示名等のWeb-owned presentation metadataは引き続きWebに置く。

**修正規模:** Small
**優先度:** 近いうち。

**解決確認 (2026-08-17)**

`themeKey`はRust `skill_id`、`gradeSlug`はcanonical grade、`pathname`はgrade/theme slugから`defineTheme`内でderiveするよう変更した。各theme fileから重複するthemeKey / gradeSlug / pathname入力を削除し、Web-owned情報はlabel / route themeSlug / search / worksheet presentationに限定した。

---

## Low


### L-004 小規模なdead/stale residueを整理する

**状態:** Closed (2026-08-17 post-fix verification)
**対象:** Repository hygiene / comments / dead exports

保守性監査で次の小規模residueを確認した。

- `theme-definition.ts`の未使用input-interface constants。
- `column-arithmetic-presentation.ts`の未使用`COLUMN_ARITHMETIC_GRID_CQW`。
- `model.rs`の`MAX_ANSWER_AST_SIZE`直上に、long division layoutについての無関係なstale comment。
- `wasm-adapter.ts`のerror textに現schema v5と不一致な`schema-v3`表記。
- `model.rs`の大量legacy re-export facadeは移行互換の可能性があるため、参照確認なしに削除しない。H-006でhistoric compatibility削除を行う際に必要性を再判定する。

**方針**

- 実参照を確認してdead export/commentだけを削除する。
- 単に古そうという理由でcompatibility facadeを削除しない。H-006の現行-only方針と合わせて判定する。

**修正規模:** Small
**優先度:** cleanup時。

**解決確認 (2026-08-17)**

未使用input-interface constants、未使用grid CQW export、stale `MAX_ANSWER_AST_SIZE` comment、schema-v3文言、historic compatibility re-export residueを削除した。pre-release方針により未参照の旧`decimal-multiply-divide.ts` source-compatibility aliasも削除した。legacy/stale symbol scanで対象残骸がないことを確認した。

---

### L-005 小規模なexact arithmetic utility重複

**状態:** Closed (2026-08-17 post-fix verification)
**対象:** Rust exact arithmetic helpers

Euclidean GCD等の小さな数学utilityが複数moduleに若干異なる形で存在する。Rust標準libraryに直接GCDがなく、このためだけに外部dependencyを増やす価値は低い。

**方針**

- 必要なら小さな内部`exact_arithmetic` / `number_theory` utilityへ`gcd_u64`等を寄せる。
- effort model内のGCD探索操作数は教材意味論であり数学utilityとは別物なので共通化しない。
- dependency追加による置換は行わない。

**修正規模:** Small
**優先度:** 将来でよい。

**解決確認 (2026-08-17)**

exact arithmetic用Euclidean GCDを`exact.rs`の`gcd_u64` / `gcd_u128`へ集約し、model / normalize / equation exact arithmetic等から再利用するようにした。人間の計算操作数を表すeffort-model上のGCD探索 semanticsとは分離したまま維持し、GCD共通化のための外部dependencyは追加していない。ClippyとRust testsがpassした。

---

### L-003 repository内に `.DS_Store` が残っている

**状態:** Closed (2026-08-16 independent audit)
**対象:** Repository hygiene

監査時点で `./.DS_Store` と `./research/.DS_Store` を確認した。Git ignore対象でproductionへの実害はほぼないが、正式assetではない。

**方針**

- 次回repository cleanup時にAutoDrill配下の該当ファイルだけを削除する。
- `.DS_Store` を生成物としてignoreし続ける。
- AutoDrill外の同名fileを推測で削除しない。


**実装試行 (2026-08-16 / ユーザー確認待ち)**

AutoDrill repository内を再確認し、監査時に見つかった`.DS_Store`は現在0件である。AutoDrill外のfileは削除していない。Issueはユーザー確認前なのでOpenのまま維持する。

**独立監査によるClose根拠 (2026-08-16)**

repository内`find . -name .DS_Store`とtracked fileの双方が0件で、`.gitignore`にも`.DS_Store`が存在することを確認した。

---

### L-001 採点時Warningの表示方法を変更する

**状態:** Planned

Warning semanticsそのものではなく、ユーザーへの提示方法を再設計する予定。

---

### L-002 Seedをworksheet設定全体の一意な再現identifierへ拡張する

**状態:** Planned / potentially important

現在のSeedだけでなく、詳細設定・difficulty等も復元できる形にし、「Seedからドリルが一意に定まる」ようにする。

互換性とversioningを考慮して設計する。

---


---

## Observations — 現時点では修正Issueにしないもの

### 正負の数(2) hard の割り算比率 42.5%

少し高いが、現時点では憂慮するほどの偏りとは判断しない。直ちに修正しない。将来effort変更時にdistribution regressionとして再確認する。

---


---

## 独立Close判定 (2026-08-17)

対象: H-006, H-007, H-008, M-012〜M-020, L-004, L-005。既存の「解決確認」は証拠として用いず、current worktreeをコード・test・fresh build・browser verifierから再検証した。FAILがあるため、対象Issueの状態は `Verification pending` のままとし、Close / commit / pushは行わない。

| Issue | 判定 | 独立検証結果 |
|---|---|---|
| H-006 | **FAIL** | registryとschema自体はcurrent-onlyで、旧revision generator / schema v4 projectionは見つからずdeterministic regenerationもPASSした。一方、`model.rs`に旧配置を温存する大量の`#[allow(unused_imports)] pub use ...` facadeが残り、`column_arithmetic.rs`には「Historic generators」「ruleset v1をfreezeして将来別rulesetを追加」とする現pre-release方針と矛盾する残骸、`architecture/overview.md`にも`generator revision compatibility`が残る。legacy互換維持用re-export/residueなし、というClose条件を満たさない。 |
| H-007 | **FAIL** | Rust→ts-rs generated wire typesとdrift check、exact integerのstring型、fail-closed validator、runtimeからのts-rs非侵入は確認した。しかしWebに手書き`EditorAction` 9-variant union、`mapEditorAction`/`assertEditorAction`のdiscriminant switch、`INPUT_STRUCTURES`のvariant一覧が残る。Rust wire variant追加時にWeb側の同じvariant inventory更新が必要で、二重定義を解消できていない。 |
| H-008 | **PASS** | `うそつきだれだ`に`sampling_layers`はなくgeneric global effort selectionを通る。generic samplerにtheme-ID special caseなし。直接regression `liar_puzzle_is_not_a_layered_theme`もPASS。 |
| M-012 | **PASS** | input capabilityはRust generated Web contractの`input_interface` / `editor_input_interface`がSoT。Webに`ThemeInputProfile`意味変換の二重実装なし。 |
| M-013 | **PASS** | `answer-presentation.ts`のpure projectionをWeb/PDF双方が利用し、renderer component自体は不自然に共通化していない。 |
| M-014 | **FAIL** | fresh WASM + fresh static exportのbrowser verifierでは全筆算sampleが`crossings=0 / gridMismatches=0`で、digit input / partial products / division solution / print PDFも正常。ただし`globals.css`に`--column-digit-width: calc(5 * var(--worksheet-grid-cell))`、`calc(3 * ...)`、`calc(2 * ...)`等の導出済みgeometry fallback/magic numberが残り、TypeScript canonical geometryとは別の第二SoTを保持している。 |
| M-015 | **PASS** | 新theme追加時のcross-language identityはWeb theme fileのtyped numeric IDが中心で、revision/skill/curriculum/layout/input等はRust contractから解決。巨大なtheme別中継switchやfilesystem magic discoveryはない。 |
| M-016 | **FAIL** | `ThemeRegistrationSpec { ... }`自体はnamed fieldsになったが、筆算familyが`column(id, revision, skill_id, curriculum_path, grade, tags, answer_contract)` / `column_division(...)`という7個の位置引数へmetadata指定を再包装している。grade/tags/answer contractまで位置依存であり、意味付きfield化のClose条件を満たさない。 |
| M-017 | **PASS** | Web Vitest 18 files / 168 tests全PASS。stale selector/invalid fixtureによるfailureなし。PDF fixtureはschema/prompt familyで分岐し、theme ID 1〜37の巨大switchや新themeごとのfixture追記要求は見つからない。 |
| M-018 | **PASS** | code/test/docsでeasy/normalはfinal effort nondecreasing、hard/randomはdeterministic shuffleという同一仕様。回帰testもPASS。 |
| M-019 | **PASS** | answer/input session stateと同期ref/action queueは`useWorksheetAnswerController`にまとまり、same-tick同期test、即時採点、grading lock二重実行防止等のAutoDrillApp regressionもPASS。 |
| M-020 | **PASS** | Web theme definitionはRust-owned identity/metadataをgenerated contractから導出し、pathname等のderived metadataをthemeごとに手入力していない。新theme追加で無関係な巨大switch更新は不要。 |
| L-004 | **FAIL** | `ProblemMathToken = MathToken`のunused backward-compatible alias、`dividerX` legacy field（production利用なし）、`model.rs`のre-export facade、historic generator方針のobsolete comment等が残る。 |
| L-005 | **FAIL** | productionのpure exact GCDは`exact.rs`へ概ね集約され、pedagogical `gcd_search_operations`は適切に分離されている。しかし`crates/drill-core/src/lib.rs`のtest内に独立したEuclidean `gcd`実装が残り、`exact::gcd_u64`と重複している。不要dependency追加はない。 |

主要検証: `cargo test --workspace`（drill-core 103 + drill-wasm 10 PASS）、workspace Clippy `-D warnings`、drill-core `wire-types` featureのcheck/clippy、generated Web contract/wire drift check、TypeScript typecheck、ESLint `--max-warnings=0`、Vitest 168 tests、fresh release WASM生成、production Next static build、Pages export 38 routes、`git diff --check`をPASS。browser verifierは78 worksheet samplesすべてcell boundary内、全筆算sample `crossings=0 / gridMismatches=0`。column digit input、掛け算partial products、割り算worked solution、Chrome print/PDFを確認し、column PDF 342376 bytes / 2 pages、division PDF 325952 bytes / 2 pagesでPASSした。sandboxでは`wasm-opt`実行権限とlocalhost bindが拒否されたため、WASMは同じrelease buildを`--no-opt`でfresh生成し、browser verifierはhost側で実行した。

---

## 保守Issue 修正後Close再検証 (2026-08-17)

前回の独立Close判定でFAILだった6件を修正し、対象15件をcurrent worktreeから再検証した。既存の「解決確認」は証拠として用いず、コード検索・contract generation・Rust/Web test・fresh WASM・fresh production export・実Chrome browser verifierを根拠とする。

| Issue | 判定 | 修正後の主要根拠 |
|---|---|---|
| H-006 | **PASS** | `model.rs`のtheme compatibility re-export facadeを削除し、利用側をcanonical theme/schema moduleへ直接接続した。historic generator / old revision registration / schema-v4 projection / legacy-only re-exportはproduction scanで0件。旧ruleset保持を示すobsolete comment/docも削除し、current generator determinismを維持。 |
| H-007 | **PASS** | Web `EditorAction`はgenerated `RustEditorAction`そのものを使用し、手書き9-variant unionと1対1 `mapEditorAction`を削除。editor action discriminantとinput structure inventoryもRust declarationからgenerated Web contractへ投影し、runtime validatorはgenerated inventory上でfail-closedを維持。exact i64 payloadはgenerated TSでもdecimal `string`。contract/wire drift check PASS、通常runtime dependency treeに`ts-rs`なし。 |
| H-008 | **PASS** | liar puzzleはlayerなし・global effort selectionのまま。直接regression PASS。 |
| M-012 | **PASS** | input capabilityはRust generated contractがSoTで、WebにThemeInputProfile semantic mappingなし。 |
| M-013 | **PASS** | Web/PDFは共通pure answer presentation projectionを使用。 |
| M-014 | **PASS** | CSSからdigit lane 5-cell、division work 3-row、remainder 2-cell等の第二SoTを除去し、TS canonical geometryからCSS custom propertiesを供給。fresh Chrome verifierで全78 samples `crossings=0 / gridMismatches=0`、digit input・partial products・division worked solution・実PDF 2ページを確認。 |
| M-015 | **PASS** | 新theme追加時のRust-owned metadataはgenerated contractから解決し、巨大中継switch / filesystem magic discoveryなし。 |
| M-016 | **PASS** | 筆算registrationの7位置引数helperを削除。各themeはcanonical `ThemeRegistrationSpec { ... }`のnamed fieldsで直接宣言し、追加builder/metadata wrapperも持たない。 |
| M-017 | **PASS** | Web 168 tests PASS。stale selector / theme-ID巨大fixture switchなし。 |
| M-018 | **PASS** | easy/normal=effort昇順、hard/random=deterministic shuffleでcode/test/docs一致。 |
| M-019 | **PASS** | answer/input session ownershipはcontrollerへ集約されたまま。AutoDrillApp regression 61 testsを含むWeb suite PASS。 |
| M-020 | **PASS** | Web theme identity/route metadataの重複は再発していない。 |
| L-004 | **PASS** | `ProblemMathToken` compatibility alias、legacy `dividerX` field、obsolete wasm output `.gitignore`、model re-export facade、stale legacy wordingを削除。指定residue scan 0件。 |
| L-005 | **PASS** | test-local GCDに加え`grade.rs`の純粋Euclidean GCD重複も発見・削除し、`exact::gcd_u64/u128`へ集約。pedagogical `gcd_search_operations`は意味が異なるため分離維持。 |

修正後の主要検証: `cargo test --workspace`、workspace Clippy `-D warnings`、`drill-core --features wire-types` Clippy、generated Web contract / wire drift check、TypeScript typecheck、ESLint `--max-warnings=0`、Vitest 168 tests、fresh release WASM、production Next build、Pages export 38 routes、browser layout verifier、`git diff --check`。最終commit前に同一check群を再実行する。

---

### M-021 page-wide worksheet grid capabilityが`column_arithmetic`へ結合している

**状態:** Closed (2026-08-17 implementation verification)
**発見契機:** 新単元「すうじはひとりぼっち」実装

筆算で使っているA4全体の方眼、row-major配置、divider suppressionが`presentation.column_arithmetic`からしか取得できず、非筆算のgrid puzzleから再利用するには「数独を筆算扱いする」かWeb側でtheme固有special caseを入れる必要がある。

**方針:** page-wide worksheet gridを独立したpresentation capabilityへ分離し、column arithmeticはそのcapabilityを利用する1利用者とする。


**解決確認 (2026-08-17)**

Rust presentation metadataへ独立`worksheet_grid` capabilityを追加し、筆算は`worksheet_grid=true, column_arithmetic=true`、数独は`worksheet_grid=true, column_arithmetic=false`とした。row-major配置・page方眼・divider suppressionは`worksheet_grid`だけを見る。共通A4 grid geometryは`worksheet-grid-presentation.ts`へ分離し、fresh Chrome全80 samplesでcrossing 0を確認した。

---

### M-022 固定セルdigit-grid入力のtyped capabilityがない

**状態:** Closed (2026-08-17 implementation verification)
**発見契機:** 新単元「すうじはひとりぼっち」実装

現行`AnswerInputInterface`はscalar numeric / structured mathのみで、4x4 gridの各cellへ1〜4を入力する能力を正しく表現できない。`tuple_only`等へ偽装するとWebが入力意味論を再解釈する必要がある。

**方針:** Rust contractへfixed digit-grid input capabilityを追加し、cell countとdigit rangeをgenerated Web contractへ投影する。


**解決確認 (2026-08-17)**

`AnswerInputInterface::DigitGrid { min_digit, max_digit, cell_count }`をRust canonical contractへ追加し、generated TSからWeb keypad/cell数を取得する。数独は1〜4・16cellをcontractで宣言し、WASM/Web validatorもshape/rangeをfail-closed検証する。input/controller/component testと実Chrome入力probeがpassした。

---

### M-023 2要素以外の順序付きtuple answer schemaがない

**状態:** Closed (2026-08-17 implementation verification)
**発見契機:** 新単元「すうじはひとりぼっち」実装

16cellの完成盤面は順序付きtupleとして採点したいが、`OrderedPair`は2要素専用の意味名であり、`Algebraic`ではtupleを解集合として順序正規化する。

**方針:** fixed-length ordered tuple schemaを追加し、長さと順序をRust gradingでfail-closedに検証する。


**解決確認 (2026-08-17)**

`AnswerSchema::OrderedTuple { length }`を追加し、Rust gradingで長さと順序を厳密比較する。solution-set semanticsを持つ`Algebraic` tupleとは分離した。16cell数独answerとWeb/WASM validation、ordered tuple regressionがpassした。

---

### M-024 九九の特殊effortを`BigNum` primitiveへ偽装している

**状態:** Closed (2026-08-17 implementation verification)
**発見契機:** 新単元「すうじはひとりぼっち」の特殊effort設計見直し

九九は仕様上`log(answer)`というtheme固有effort例外だが、現実装は`Operation::BigNum`の通常primitiveを借用して同じ数値を得ている。数値が一致してもprimitiveの意味が異なり、`principles.md`の「場当たり的な実装をしない」「真にtheme固有の例外だけをtheme側へ置く」に反する。

**方針:** 通常のoperation-vector effortとは別に、theme固有effortを明示的なoptional経路として表現する。九九と「すうじはひとりぼっち」はこの経路を利用し、既存primitiveの意味を流用しない。通常themeは従来どおりoperation vectorをsource of truthとする。


**解決確認 (2026-08-17)**

`Problem.theme_specific_effort: Option<f64>`を通常operation-vector modelとは独立した明示的経路として追加した。九九は`Some(log10(answer))`へ移行し`BigNum`偽装を削除、数独は`Some(nontrivial + 0.3*trivial)`を使用する。特殊effort problemはgraph/vectorを空にし、generic Rust regressionとWeb wire validatorが通常modelとの混在を拒否する。

**再監査注記 (2026-08-17 / Rust architecture audit)**

元の`BigNum` primitive偽装そのものは解消されているため、本Issueの元症状はClosedのままとする。ただし、代替として導入された`theme_specific_effort: Option<f64>`は最終architectureとしては不適切と判定した。`solution_graph` / `operation_vector` / `theme_specific_effort` / `effort`を独立fieldとして同居させ、「Someなら他2つは空/zero」というruntime invariantへ逃がしているためである。このより広い設計問題は **H-010** で追跡する。

---

### M-025 Pages verifierがtheme数を固定値で保持している

**状態:** Closed (2026-08-17 implementation verification)
**発見契機:** 新単元「すうじはひとりぼっち」のfresh Pages export検証

`scripts/verify-pages-export.mjs`がsitemap URL数を`38`と直書きしており、正常にthemeを1つ追加しただけでverificationが失敗する。theme追加のたびにverifierの無関係なmagic number更新が必要で、M-015/M-017で避けるべき中央boilerplateの一種である。

**方針:** 固定件数を削除し、fresh export内のdrill `index.html`集合とsitemap URL集合を相互照合する。root routeも明示的に要求し、sitemap欠落・余計なrouteの双方を検出する。


**解決確認 (2026-08-17)**

固定`38`件を削除し、fresh export内の`drills/**/index.html`集合とsitemap URL集合を相互照合するよう変更した。新theme追加後のalpha Pages exportは39 routesでpassした。

---

### M-026 route/sitemap testsが全theme routeを手書き列挙している

**状態:** Closed (2026-08-17 implementation verification)
**発見契機:** 新単元「すうじはひとりぼっち」のroute追加

`sitemap.test.ts`と`page.test.tsx`が全implemented themeのrouteをexpected fixtureとして手書き列挙しており、新theme追加のたびにproduction registryとは無関係な長大listへ追記が必要だった。M-017の「新theme追加でstale fixture更新を要求しない」という意図に反する。

**方針:** expected route/static paramsは`IMPLEMENTED_THEMES`のtyped route metadataから導出する。個別themeのSEO/foreign-key等、意味のあるspot checkは残す。export completenessはPages verifierがfresh artifactとsitemapを相互照合する。

**解決確認 (2026-08-17)**

`sitemap.test.ts`とstatic params testの全route手書きfixtureを削除し、expectedを`IMPLEMENTED_THEMES`のtyped route metadataから導出した。個別theme metadataのspot checkは維持する。


---

## 2026-08-17 Rust architecture audit

以下は、既存IssueのClosed表記を前提にせず、`crates/`以下の現行Rust実装を型設計・ownership・module/API・WASM境界から再監査して追加したIssueである。テスト/Clippyの成否はIssue判定の根拠とはせず、実装上のinvariantと将来の変更容易性を根拠とする。

### H-009 `Problem`と周辺domain typeがwire DTOを兼ね、invalid stateを型で排除できない

**severity:** High
**状態:** Open
**該当:** `crates/drill-core/src/model.rs` (`Problem`, `ProblemPrompt`, `RationalCoefficient`, `AnswerInputInterface`, `AnswerSchema`, `Worksheet`, `EditorState`, `GradeResult`), `crates/drill-core/src/identity.rs` (`ProblemSetIdentity`), `crates/drill-wasm/src/lib.rs` (`CalculateEffortRequest`等)

**具体的なコード上の証拠**

- `model.rs`自身が`Versioned domain values shared by the native engine and the WASM adapter`と宣言し、core domain typeへ`Deserialize` / `Serialize` / `ts-rs`都合を直接載せている。
- `RationalCoefficient`は`new()`では0分母を拒否し約分・符号正規化する一方、`numerator` / `denominator`が`pub`でderive `Deserialize`されているため、`RationalCoefficient { denominator: 0, .. }`や未約分値をconstructorを通さず生成・deserializeできる。constructorのinvariantが型によって保護されていない。
- `Problem`は`prompt`, `input_interface`, `answer_schema`, `canonical_answer`, `worked_solution`, effort関連fieldをすべてpublicに持つ。例えばMini Sudoku promptに5cellだけ入れ、Integer schemaとscalar answerを組み合わせた`Problem`も型上合法である。
- `ProblemPrompt::MiniSudoku { givens: Vec<Option<u8>> }`は16cell・digit 1..=4を型で表さない。`LiarPuzzle` / `LiarStatement`も`people_count` / person indexを生の`u8`で保持し、0や範囲外personがrepresentableである。`liar_puzzle::statement_truth()`は`person - 1`を行うため、不正domain valueはdebug buildでpanicし得る。
- `AnswerNode`はediting draft用の`Empty` / `NanError`まで含む同じenumが`Problem.canonical_answer`の型でもあり、「canonical answerが未入力/parse error」というdomain上不正な状態を表現できる。
- `ProblemSetIdentity::new()` / `FromStr`はseed/schemaを検証する一方、struct fieldがpublicでderive `Deserialize`されるため検証を迂回できる。対照的に`Difficulty`はprivate field + custom `Deserialize`で1..=4を保証している。
- WASM `calculate_effort`はJSONを直接`Problem`へdeserializeし、`problem.schema_version`しか検証せずcoreへ渡す。Rust domain invariantをWeb validator側が補う構造になっている。
- `GradeResult`は`status`と派生値`is_correct`を両方保存し、矛盾した値を型上作れる。`Worksheet`も`problem_set_id`文字列とdecoded `identity`を同時保存する。

**なぜRustとして不適切か**

これはtyped domain modelではなく、Serdeで運びやすいpublic data bagをdomain objectとしても使っている。Rustでconstructorを用意してもpublic fields/derived Deserializeがinvariantを迂回できるなら、`Invalid states should be unrepresentable`を実現していない。wire都合とdomain意味論が同一typeへ混在している。

**なぜ保守性上問題か**

新prompt/input/schemaを追加するたびに「この組合せは合法か」を複数validator/testのruntime conventionで同期する必要がある。1箇所のvalidation漏れで、分母0、board長不正、canonical answer shape不整合、worked solution不整合などがcore内部まで到達する。Web側のruntime validationがRust domain modelの穴を埋める状態は、`drill-core`を数学的source of truthとする原則にも反する。

**よりRustらしい修正方向**

- invariantを持つcore typeはprivate field + validated constructor / `TryFrom` / custom `Deserialize`とし、wire DTOが必要ならWASM boundaryへ分離する。
- `RationalCoefficient`、grade、theme identity、fixed grid等にはvalidityを保持するnewtype/fixed-size typeを使う。Mini Sudokuなら少なくとも`[Option<Digit>; 16]`相当のdomain typeを検討する。
- edit中の`AnswerNode`と、generatorが保持するvalidated/canonical answerを同じ無制約typeとして扱わない。必要なら`CanonicalAnswer` wrapperを設ける。
- `Problem`はpublic field literalではなく、prompt/answer contract/effort evidenceの整合性を保証したaggregateとして構築する。wire serialization用のderived fieldはDTO projectionで作る。
- `GradeResult::is_correct`や`Worksheet.problem_set_id`のような純粋派生値は、core SoTとして重複保存しないか、DTO生成時だけ導出する。

**Close条件**

- coreのinvariant-bearing typeをsafe Rust API経由で不正状態に構築できない。
- WASM JSONが直接unvalidated core aggregateへdeserializeされない。
- `RationalCoefficient`等のconstructor invariantをpublic field/Serdeが迂回しない。
- prompt/input/schema/canonical answer/worked solutionの不正組合せが型または単一validated conversion boundaryで拒否される。
- draft-only answer状態とcanonical generated answerの境界が明示される。

---

### H-010 特殊effortを`Option<f64>`で逃がし、graph/vector/scalar/scoreを複数SoTとして保持している

**severity:** High
**状態:** Open
**該当:** `crates/drill-core/src/model.rs::Problem`, `crates/drill-core/src/effort.rs::{SolutionGraph, calculate_effort, calculate_graph_effort}`, `crates/drill-core/src/themes/basic_arithmetic.rs`, `crates/drill-core/src/themes/mini_sudoku.rs`, `docs/architecture/effort-model.md`, M-024

**具体的なコード上の証拠**

`Problem`は同時に次を保持する。

- `solution_graph: SolutionGraph`
- `operation_vector: OperationVector`
- `theme_specific_effort: Option<f64>`
- `effort: f64`

現行規約は「通常modelなら`theme_specific_effort=None`」「特殊modelなら`Some`かつgraphはempty/vectorはzero」である。しかしこれは型ではなくtest/Web validatorによるruntime conventionである。

`calculate_effort()`は`theme_specific_effort.unwrap_or_else(|| weights.weighted_sum(&problem.operation_vector))`だけを読み、`solution_graph`とstored `problem.effort`を無視する。従って、同じ`Problem`内のgraph/vector/special/stored effortが相互に矛盾していても関数は通常どおり値を返す。WASM `calculate_effort`もこの不整合を拒否しない。

さらに`SolutionGraph::operation_vector()`は`depends_on`を一切参照せず全nodeを1回数えるだけであり、通常builderの大半は`operations_graph()`でoperation列を単なる直列chainへ包んでいる。graph topologyを持つ型なのにeffort意味論はtopologyを使用しない。

九九・Mini Sudokuは空graph/zero vectorを格納し、別fieldのscalarへescapeする。元のM-024であった`BigNum`意味偽装は消えたが、問題を「特殊値をOptionで横に足す」設計へ移しただけである。

**なぜRustとして不適切か**

これは本質的に

`OperationModel | ThemeSpecificModel`

というsum typeである。Rustならenumでvariantとして表現すべきものを、4 independent fields + Option + empty/zero sentinelへ展開している。normal/special両方を同時に入れる、不一致vectorを入れる、stored scoreだけ変える、といったinvalid stateがすべてrepresentableである。

**なぜ保守性上問題か**

新しい特殊effortを追加するたびに、generator・tests・wire validator・docsが「どのfieldを空にするか」を共有しなければならない。将来graph semanticsやweightsを変更してもstored vector/effortが自動追従しないためsilent driftが起きる。`solution_graph`という重い抽象化も、そのedgeが意味を持つのか単なるoperation listなのか不明瞭になる。

**よりRustらしい修正方向**

例えばcoreでは次のような1つのauthorityへ寄せる。

- `EffortModel::Operation(OperationPlan)`
- `EffortModel::ThemeSpecific(ThemeSpecificEffort)`

通常modelではoperation evidenceからvectorとscoreを導出し、特殊modelはfinite/nonnegativeを保証するvalue typeまたはtheme-owned policyで表す。wireへvector/scoreを出したい場合はDTO projection時のderived dataとする。

`SolutionGraph`については、本当に依存関係が教材意味論に必要ならunique ID/dangling/cycle等をvalidated typeとして扱い、そのtopologyがconsumerで意味を持つようにする。単にprimitive列で十分なら、graphという名前・edge fieldを残さずoperation sequenceとして単純化する。

**Close条件**

- normal/special effort modelが型のvariantとして排他的に表現される。
- graph/vector/special/scalarを相互独立SoTとして保存しない。
- `calculate_effort`が唯一のeffort evidenceから導出でき、不整合`Problem`を構築できない。
- 特殊theme追加時にempty graph/zero vector sentinel規約を手動同期しない。
- `SolutionGraph.depends_on`を残すならgraph invariantと用途が明確で、残さないなら不要なgraph abstractionを削る。

---

### H-011 `ProblemGenerator` traitがoptional/default methodの組合せでcapability protocolを表現している

**severity:** High
**状態:** Open
**該当:** `crates/drill-core/src/generator.rs::ProblemGenerator`, `generate_with_generator`, layered/finite/answer-conditioned sampling;各theme generator impl

**具体的なコード上の証拠**

`ProblemGenerator`には、必須の`registration` / `draw_candidate`に加えて、次の独立したoptional/default methodが並ぶ。

- `answer_domain() -> Option<...>`
- `finite_distinct_candidate_count() -> Option<usize>`
- `draw_finite_candidate(...) -> Option<Problem>`（default `None`）
- `sampling_layers() -> Option<...>`
- `sampling_layer(...) -> Option<usize>`（default `None`）
- `bootstrap_layer_multiplier()`
- `deduplicate_bootstrap_pool() -> bool`
- `constructive_layer_sampling() -> bool`
- `draw_candidate_for_layer(...) -> Option<Problem>`（default `None`）
- `draw_candidate_for_answer(...)`（defaultは指定answerを無視して通常`draw_candidate`を呼ぶ）

このため、例えば以下がすべてcompileする。

- finite countだけ`Some`にし、finite drawはdefault `None`。
- `sampling_layers=Some`だが`sampling_layer=None`。
- `constructive_layer_sampling=true`だがlayersまたはlayer draw未実装。
- `answer_domain=Some`だが`draw_candidate_for_answer`をoverrideせず、要求answerを無視。

framework側はその矛盾を型で防げず、`expect("constructive layer sampling requires declared layers")`、`expect("layered selection requires layers")`、分類失敗によるpool不足、answer mismatchのretry等で処理している。

**なぜRustとして不適切か**

traitが「共通behavior」を抽象化する代わりに、複数の独立capability flag/Option/default methodを持つextension-pointの墓場になっている。型が実装者へ要求すべき契約を、runtime conventionへ逃がしている。dynamic dispatch自体は問題ではなく、trait contractの形が問題である。

**なぜ保守性上問題か**

新sampling strategy追加時にdefault methodがさらに増えやすく、実装者は「どのmethodの組を同時overrideすべきか」を暗黙知として覚える必要がある。AI agentがthemeを追加する運用では特に、compileするがAttemptLimitでしか壊れない実装を生みやすい。

**よりRustらしい修正方向**

`ProblemGenerator`本体はregistrationとcandidate construction等の最小共通契約へ縮め、sampling方式は整合したstrategy typeで表す。例えば`SamplingStrategy`を`Random` / `Finite` / `AnswerConditioned` / `Layered`等のenumにし、各variantが必要なcallback/dataを一括で持つ。あるいは能力ごとに小さなtrait/typeを分け、存在するcapabilityは必要methodを必須化する。

「将来拡張できるからtrait」ではなく、現在存在するstrategyの意味論がcompile-timeにcompleteになることを優先する。

**Close条件**

- finite/layered/answer-conditioned/constructiveの不完全なmethod組合せがcompile-timeに表現できない、または1つのvalidated strategy valueで拒否される。
- capability矛盾を理由にする`expect`や「defaultが指定answerを無視する」挙動がない。
- 新sampling mode追加時に無関係themeへoptional methodを増やさない。

---

### H-012 `ThemeRegistrationSpec`がnamed fieldsになっただけで、theme contractのinvalid combinationを許している

**severity:** High
**状態:** Open
**該当:** `crates/drill-core/src/theme.rs::{ThemeRegistrationSpec, ThemeRegistration, ThemeAnswerContract, ThemeInputProfile, WorksheetLayoutProfile}`, `crates/drill-core/src/registry.rs`,各`themes/*.rs` registration

**具体的なコード上の証拠**

M-016で11個の位置引数は`ThemeRegistrationSpec { ... }`へ改善されたが、現在もSpecは次の値を独立に自由組合せできる。

- `grade: Option<u8>`。コメント上は1..=9だが型は0/255も許す。
- `ThemeAnswerContract { prompt_kind, answer_schema_kind, input_profile }`。例えばMiniSudoku prompt + Integer schema + LinearEquation input profileも合法。
- `ThemeInputProfile::DigitGrid { min_digit, max_digit, cell_count }`。`min > max`や0cellも合法。
- `WorksheetLayoutProfile { problem_count, columns, rows: usize }`。0列、`columns * rows < problem_count`等も合法。
- `numeric_theme_id`と`generator_revision`はいずれも生`u32`で、registry APIも同じprimitiveを2つ受ける。

重複theme IDはregistration時に型/const構築で拒否されず、`active_registrations()`が`BTreeMap::insert`後に`assert!(previous.is_none())`して初めてpanicする。weight overrideの不正値も`resolved_weights()`内の`expect`で処理する。

またMini Sudokuではregistrationの`DigitGrid {1,4,16}`、generatorが返すproblemのinput interface、`OrderedTuple { length:16 }`、promptのgivens Vec長が別々に手入力される。型上はこれらが一致する保証がない。

**なぜRustとして不適切か**

named struct literalは位置引数事故を減らすだけで、domain invariantを表現しない。registrationは「themeを追加するときのcompile-time schema」に近い重要な値なのに、実態はpublic primitive fieldsを集めたdata bagである。

**なぜ保守性上問題か**

新theme/schema/input modeを追加するたびに、prompt/schema/input/layout/grade/IDの整合性をreviewとtestへ依存する。間違ったmetadataでもcompileし、場合によってはregistryを列挙した時点でpanicする。theme数が増えるほど同期面積が増える。

**よりRustらしい修正方向**

- `ThemeId`, `GeneratorRevision`, `SchoolGrade`等、混同と範囲に意味がある値はnewtype/enum化する。
- layoutはzero/mismatchを作れないconstructor/value typeにする。既知layout profileをenum化する選択も可だが、無意味な固定enum乱造は避ける。
- answer contractは「prompt kind/schema/input三つ組」を自由直積にせず、実際に存在するcontractを表すenum/typed descriptor、またはgeneratorのassociated contractへ寄せる。
- fixed gridではcell count/digit domainを1つのtyped specからprompt/input/schemaへ投影する。
- duplicate registry identityは起動後の`assert!`ではなく、構築時に一意性を保証/検証する明示的registry builder等へ寄せる。

**Close条件**

- grade/layout/digit-grid等の不正値をsafe constructorで作れない。
- prompt kind / answer schema kind / input profileの無意味な組合せを単なるpublic struct literalで作れない。
- fixed-grid等の同じdomain parameterをregistrationとproblem生成で別々に手入力しない。
- duplicate theme identityや不正weightが通常のregistry利用時panicとして初めて発見されない。

---

### M-027 `うそつきだれだ`の論理式長を`Operation::Identity` node数へ偽装している

**severity:** Medium
**状態:** Open
**該当:** `crates/drill-core/src/themes/liar_puzzle.rs::{statement_effort, solution_graph}`

**具体的なコード上の証拠**

`statement_effort()`は各論理文のformula lengthを1/2/people_countで定義している。一方`solution_graph()`は、その総長`formula_length`個の`SolutionStep`を生成し、全nodeの`operation`を`Operation::Identity`、`depends_on`を空にしている。`calculate_graph_effort`のIdentity weightが1であることを利用し、結果だけをformula lengthへ一致させている。

これは「論理式の長さ」という意味を「算術solution graph上のIdentity primitiveの反復」に偽装している。

**なぜRustとして不適切か**

型名は付いているが、variantの意味論を守っていない。`Operation::Identity`の本来の意味を変更すれば、無関係な論理パズルeffortが連動して変わる。型でsemantic distinctionを作った意味を自ら壊している。

**なぜ保守性上問題か**

primitive weight調整が別domainへ漏れ、後からコードを読んでもなぜIdentityが数十個並ぶのか説明できない。M-024で禁止した「数値が一致するprimitiveを借りる」設計と同型である。

**よりRustらしい修正方向**

H-010のeffort model整理後、論理式長が真にtheme固有modelならtheme-specific variantとして明示する。もし複数logic themeで再利用する人間操作primitiveが定義できるなら、`LogicalLiteralEvaluation`等の実際の意味を持つprimitiveを共通modelへ追加する。単に数値を合わせるため既存primitiveを借りない。

**Close条件**

- liar puzzle effortが`Identity`等の無関係primitiveのweightに依存しない。
- graph/operationを使うならnodeのoperationが実際の解法primitiveを表す。

---

### M-028 MathLive移行後も旧`EditorState` / `apply_editor_action` state machineをcompatibility目的で保持している

**severity:** Medium
**状態:** Open
**該当:** `crates/drill-core/src/editor.rs`, `model.rs::{EditorState, EditorAction}`, `lib.rs`, `crates/drill-wasm/src/lib.rs::apply_editor_action`, `export_web_wire_types.rs`, `contract.rs`, `docs/architecture/answer-ast.md`

**具体的なコード上の証拠**

`docs/architecture/answer-ast.md`は明示的に、productionのMathLive pathではselection/caretをMathLiveが所有し、旧`EditorState.active_path` / `cursor`は`apply_editor_action`互換境界だけに残す、と記載している。

production Webを検索すると`applyEditorAction`はadapter/interface定義以外に実利用callerがなく、現在の回答同期は`parse_mathlive_answer`を使用している。それにもかかわらず、Rustには約680行の旧editor state machine、wire `EditorState`/`EditorAction`、WASM endpoint、generated TS root、action discriminant contract、対応testが残る。

なお`mathlive_input.rs`は`editor::ensure_capability`を現行validationとして再利用しているため、`editor.rs`を機械的に全削除するのではなく、この現行helperは適切なmoduleへ残す必要がある。

**なぜRustとして不適切か**

コード単体の型付けより、pre-release projectで「互換性のためだけの過去state machine」をproduction APIに維持している点が問題である。`principles.md`の「legacy compatibilityを抱えない」「Gitを歴史保存層として使う」に反する。

**なぜ保守性上問題か**

AnswerNode/editor capabilityを変更するたびに、実際には使わない旧state machineとWASM contractまで同期する必要がある。さらに`EditorState { answer, cursor, active_path, committed }`自体が不正組合せを多数作れるため、不要なvalidation surfaceを残している。

**よりRustらしい修正方向**

MathLive production pathで必要なcapability validationだけを小さな現行moduleへ分離し、未使用の`EditorState` / `EditorAction` state machine、WASM endpoint、wire export、compatibility testsを削除する。将来必要ならGit履歴から再設計する。

**Close条件**

- productionで未使用の旧editor state/action APIがRust/WASM/wire contractから消える。
- MathLive parserが必要とするinput capability validationだけが現行責務として残る。
- canonical docsが「compatibility boundaryを残す」前提を持たない。

---

### M-029 theme capabilityが`ThemeTag`と`ThemePresentationPolicy`へ二重登録されている

**severity:** Medium
**状態:** Open
**該当:** `crates/drill-core/src/theme.rs::{ThemeTag, ThemePresentationPolicy}`, `crates/drill-core/src/themes/column_arithmetic.rs`, generated Web contract consumers

**具体的なコード上の証拠**

`ThemeTag`に`ColumnArithmetic`と`PrintRecommended`があり、同時に`ThemePresentationPolicy`にも`column_arithmetic: bool` / `print_recommended: bool`がある。全column themeはtagsへ両方を手入力しつつ、`Presentation::COLUMN_ARITHMETIC`で同じ2事実を再度trueにしている。

実際のWeb側も、印刷推奨badgeには`hasThemeTag(..., 'print_recommended')`を使う一方、worksheet layoutには`presentation.column_arithmetic` / `worksheet_grid`を使っているため、2つの値が食い違えばUIの一部だけが変わる。

**なぜRustとして不適切か**

typed metadataにしたにもかかわらず、同じsemantic factを別型の2 fieldへ重複保存している。bool/tagいずれかを変更してもcompilerは矛盾を検出できない。

**なぜ保守性上問題か**

新themeで片方だけ付け忘れるsilent bugを誘発し、source of truthを一つにする原則に反する。

**よりRustらしい修正方向**

taxonomyとbehavioral capabilityを明確に分ける。`print_recommended` / `column_arithmetic`がbehaviorならpresentation policyをcanonicalにし、UI badgeもそこから導出する。検索/filter用tagが必要ならcanonical policyから生成し、人手で二重登録しない。

**Close条件**

- 同じcapabilityの真偽をtagとpresentation fieldへ別々に手入力しない。
- Web/Rustの全consumerが1つのcanonical metadataから導出する。

---

### M-030 `problem_key()`がdedup keyとして`ProblemPrompt`全体を所有cloneし、sort比較中にも再構築している

**severity:** Medium
**状態:** Open
**該当:** `crates/drill-core/src/generator.rs::{canonicalize_commutative_expression, problem_key, select_candidates_from_pool, generate_with_generator}`

**具体的なコード上の証拠**

`problem_key()`の返り値は専用keyではなくowned `ProblemPrompt`である。Arithmeticでは式木をclone/canonicalizeし、ColumnArithmeticは左右式をclone、LiarPuzzleはstatements `Vec`をclone、MiniSudokuはgivens `Vec`をcloneする。LinearEquationだけはpresentation用boolをfalseへ書き換えた新Promptを作り、「promptそのもの」と「dedup identity」が異なることを関数内で吸収している。

候補poolのsort comparatorは比較のたびに`problem_key(left).cmp(&problem_key(right))`を呼ぶため、heap-owning promptでは比較ごとにclone/allocationが発生する。同じkeyはHashSet dedupでも繰り返し構築される。

**なぜRustとして不適切か**

これはmicro optimizationではなく、ownership/API設計の問題である。semantic identityを表す型がないためwire/display promptをowned keyとして流用し、borrowで済む比較までallocationしている。またpresentation-only flagをkey生成時に手で消す必要があること自体、promptとsemantic identityの混同を示す。

**なぜ保守性上問題か**

新Prompt variant追加時に「何をdedup上同一とみなすか」を巨大matchへ追記し、clone方法まで考える必要がある。display field追加がdedup semanticsへ漏れる可能性もある。

**よりRustらしい修正方向**

dedup identityを専用のborrowable/compact keyまたはtheme-owned semantic keyとして定義し、必要ならcandidate作成時に一度だけprecomputeする。sort comparator内でAST/Vecを再cloneしない。presentation-only fieldをsemantic keyへ最初から含めない。

**Close条件**

- sort比較ごとにheap-owning`ProblemPrompt`をclone/rebuildしない。
- dedup identityとwire/display promptの意味が型/API上分離される。
- 新Prompt variant追加時のdedup semanticsが明示的かつ局所的である。

---

### M-031 theme固有regression testが共通`generator.rs`へ集中し、theme追加の変更局所性を壊している

**severity:** Medium
**状態:** Open
**該当:** `crates/drill-core/src/generator.rs`の`#[cfg(test)] mod tests`（`new_arithmetic_themes_generate_with_requested_domains`, `broad_seed_effort_invariants_hold_for_every_registered_theme`, column arithmetic/layered theme tests等）

**具体的なコード上の証拠**

production generatorはH-002対応でfamily moduleへ分離されたが、`generator.rs`は現在も約2,800行あり、後半の大部分が中央testである。

- `new_arithmetic_themes_generate_with_requested_domains`は複数familyのtheme IDを手書き配列へ並べ、IDごとの巨大`match`でoperand/domain仕様を検証する。
- `broad_seed_effort_invariants_hold_for_every_registered_theme`には`repaired_themes`という過去修正対象IDの手書き`HashSet`が残る。
- column arithmetic testは13個のtheme IDを手書き列挙し、別testでも同じ13 ID配列を再度持ち、IDごとの詳細curriculum条件を中央matchで検証する。
- layered theme testも対象generatorを中央配列へ手入力している。

**なぜRustとして不適切か**

testだから多少の冗長性は許容できるが、module ownershipを反転させるほどの中央fixture/matchは問題である。productionではtheme-owned semanticsにしたのに、testでは共通generatorが各themeのprivate domain知識を再列挙している。

**なぜ保守性上問題か**

新theme追加時にfamily moduleだけで完結せず、中央`generator.rs`のID list/matchを更新する誘惑・必要性が残る。stale listはtest coverageのsilent omissionにもなる。

**よりRustらしい修正方向**

- 全registrationに共通するdeterminism、schema、effort finite、dedup等だけを`generator.rs`のgeneric testsへ残す。
- operand range、curriculum shape、theme固有archetype等は各theme/family moduleの`#[cfg(test)]`へ移す。
- capability共通testが必要なら「ID手書きリスト」ではなくregistration/generatorが公開するcapabilityを基準に走査する。
- `repaired_themes`のような歴史保存fixtureは現行contractへ置換する。

**Close条件**

- 新themeのdomain/curriculum testは原則theme/family module内で完結する。
- central generator testsにtheme IDごとの巨大match/歴史的修正対象listがない。
- universal invariant testは全registryを自動走査する。

---

### L-006 現worktreeのRust sourceが`cargo fmt --all -- --check`を通らない

**severity:** Low
**状態:** Open
**該当:** 現在の未コミットRust差分（`generator.rs`, `generator_support.rs`, `model.rs`, `theme.rs`, `themes/basic_arithmetic.rs`, `themes/mini_sudoku.rs`, `themes/mod.rs`, `themes/multiplication_table.rs`等）

**具体的なコード上の証拠**

2026-08-17監査時に`cargo fmt --all -- --check`がexit 1となり、複数Rust fileにrustfmt差分を報告した。一方`cargo check` / test / Clippyはpassしている。監査ではproduction codeを変更しない指示のため、rustfmt自体は実行していない。

**なぜRustとして不適切か / 保守性上問題か**

formatting自体は意味論問題ではないが、Rust repositoryのcanonical formattingから外れた差分はreview noiseを増やし、CIでfmtを必須化した場合にmergeを止める。

**よりRustらしい修正方向**

現在の価値ある未コミット差分を破壊しない状態で、実装担当が意図した変更を確定した後に`cargo fmt --all`を適用し、差分をreviewする。

**Close条件**

- `cargo fmt --all -- --check`がcleanにpassする。

---

### L-007 `SamplingLayerSpec.key`が宣言されるだけで一度も読まれていない

**severity:** Low
**状態:** Open
**該当:** `crates/drill-core/src/theme.rs::SamplingLayerSpec`, layered theme declarations

**具体的なコード上の証拠**

`SamplingLayerSpec`は`key: &'static str`, `weight`, `minimum`を持つが、`crates/drill-core/src`を横断検索しても`.key`のconsumerが存在しない。layered samplerは配列indexだけでlayerを識別している。各themeは意味のある文字列keyを記述しているがproduction semanticsには影響しない。

**なぜRustとして不適切か / 保守性上問題か**

未使用metadataは「将来使うかもしれない」schema residueであり、AI実装者にkeyの一意性・安定性・wire意味が必要だと誤認させる。`principles.md`の一時/試験実装の残骸を残さない方針にも反する。

**よりRustらしい修正方向**

現在不要ならfieldを削除する。diagnostics/telemetry等でstable layer identityが本当に必要なら、そのconsumerと型上の意味を同時に導入する。

**Close条件**

- `key`を削除するか、現行product requirementに基づく明確なconsumerが存在する。
