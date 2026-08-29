> **Historical document:** 2026-08-30 に `docs/issues.md` から GitHub Issues へ移行する直前のbacklog snapshotです。現行Issueのsource of truthではありません。

# AutoDrill — Open Issues

この文書を、現時点の未解決 Issue の canonical backlog とする。

- dated audit 文書は発見時点の証拠・調査記録であり、現在の backlog の source of truth にはしない。
- mobile はalphaではsupport対象外。mobile対応は`roadmap.md`のbeta移行条件とし、発見したmobile UX問題は **Low / alpha非blocking** としてbacklogへ記録する。
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

**状態:** Closed (2026-08-27 user-directed UX fix + browser verification)

`.input-panel-inner` 自体は中央寄せだが、3 column 幅が非対称であるため数字keypad/主要操作の視覚重心がworksheet中心からずれる。

**再現条件**

1. Chromeをdesktop幅1440×1000（1024×768でも再現）にし、難易度`ふつう`・Seed `A1b2`で`1けたのたしざん(1)`を生成する。
2. 任意の解答欄を選択してinput panelを開く。数字のみのsimple keypadでは左側trackが空のまま、右側に移動/delete/clear/確定が残り、panel全体の視覚質量が右へ偏る。
3. 同じ難易度`ふつう`・Seed `A1b2`で`すうじはひとりぼっち`を生成し、問題1の任意の空cellを選択すると、数字keyが`1 2 3 / 4`の3+1配置となり、2段目右側に大きな空白が生じる。
4. 同じ難易度`ふつう`・Seed `A1b2`で`分数の足し算`を生成し、問題1の解答欄を選択すると、左側の`分数`/`帯分数`2buttonがtrack高さいっぱいまで縦長にstretchする。

magic-number offsetではなくgrid構造を整理し、主要入力部を中央へ合わせる。


**実装試行 (2026-08-16 / 検証待ち)**

入力panelの3列を左右対称`minmax(... ) / 210px / minmax(...)`へ変更し、numeric keypadを幾何学中心trackへ置いた。実Chrome probeでも中心deltaが許容範囲内であることを確認した。ユーザー視覚確認前なのでOpen。

**ユーザーUX確認 (2026-08-16)**

幾何学的中央性は改善しているが、`+ / − / ±`の演算子keyが構造入力keyと同じ大きな占有面積を持ち、余白が過剰で視覚的に重い。構造keyと演算子keyを同一サイズで扱わず、演算子keyを数字keyに近いcompact controlとして再配置する。修正後に再度ユーザー確認する。


**技術的修正完了・ユーザー再確認待ち (2026-08-16)**

分数・平方根・複数解のstructure keyは64pxを維持する一方、`+ / − / ±`は42pxのcompact operator rowへ分離した。numeric keypadの幾何学中心は維持し、browser regressionではstructure row / operator rowそれぞれの高さ一致と、operator rowがstructure rowより十分小さいことを検証している。最終的な視覚バランスのみユーザー再確認待ち。

**2026-08-26 visual UX audit追補**

中学生full keypadは、structure / numeric / operator / controlの4群が画面上で概ね釣り合って見える。一方、数字のみ・筆算等のsimple keypadではnumeric track自体はworksheet中心へ来ているが、左側trackが空で右側controlだけが残るため、**panel全体の視覚質量は右寄り**に見える。幾何学的なnumeric centerだけではM-002の「視覚的中央」のClose条件として不十分であり、simple shellを含めたユーザー視覚確認までOpenを維持する。

**2026-08-26 frontend visual UX audit再確認**

1440pxだけでなく1024px幅でも、数字のみ・筆算・Mini Sudokuのsimple input panelは左trackが空いたまま右側control群だけを保持するため、入力UI全体が明確に右へ偏って見える。Mini Sudokuではさらに数字key `1 2 3 / 4` が3+1配置となり、2段目左端に4だけが残って大きな空白を作る。1〜4の有限keypadは2×2等の対称配置にし、simple shell自体もpanel中心に対して視覚質量が左右で釣り合う構造へ整理する。magic-number offsetで補正しない。

また小学生の分数入力では、structure keyが「分数」「帯分数」の2個だけになると各buttonが左trackの高さいっぱいまで縦長にstretchし、glyph/labelに対して過剰な空白を持つ。中学生の2×2 structure shellと同じcontrol familyとして寸法規則が連続していない。利用可能key数が少ない場合もbutton自体を不必要に巨大化させず、共通のkey size / grid rhythmを維持する。

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


**2026-08-27 解決確認**

ユーザー方針に従い、simple keypadは空の左trackを廃止して「数字群 + 編集control群」の2列自体をpanel中央へ配置した。Mini Sudokuは1〜4を2×2配置とした。小学生分数の`分数`/`帯分数`は縦長自体を許容し、unusedな3列目をなくして2列で幅を使い切る。1440×1000実Chromeで通常計算・分数・Mini Sudokuを再確認し、視覚重心と不要空白の問題が解消した。

小学生分数keyについて過去に記載した「button自体を巨大化させない」という要求はsupersedeする。縦方向の伸びは許容し、問題だったunused空白だけを削減する。

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

**状態:** Closed (2026-08-27 user-directed UX fix + browser/integration verification)
**対象:** Print/PDF UX

現在の印刷プレビューでは解答ページを上下反転して表示する方式が固定されている。利用者・印刷用途によっては通常向きの解答ページが望ましいため、反転有無を選択可能にする。

**再現条件**

1. desktop Chrome（1440×1000程度）で、難易度`ふつう`・Seed `A1b2`の`負の数の計算(1)`を生成する。
2. `印刷`からin-appの印刷プレビューを開く。
3. 2ページ目の解答ページまでscrollする。
4. 解答ページ全体が180°反転しているが、preview内に「両面印刷向けの意図的な反転」である説明も、通常向きへ切り替えるcontrolもない。画面だけでは意図的仕様か描画不具合か判断できない。

**方針**

- 印刷設定に「解答を反転する / 通常向き」の明示optionを追加する。
- 初期値は現行挙動との互換性と教材用途を踏まえて決める。
- Web print previewと実PDFで同じ設定を使用し、CSSだけの偶発的なtransform分岐にしない。

**Close条件**

- preview上で反転有無を選べる。
- 生成PDFの解答ページorientationが選択と一致する。
- 問題ページには影響しない。
- browser/PDF regressionを追加する。

**2026-08-26 visual UX audit追補**

解答pageの180°反転自体は既存の両面印刷仕様だが、print preview画面にはその意図を説明する表示がない。監査者が画面だけでは「意図的な反転」か「描画不具合」か判定できず、docsを読んで初めて仕様と確認できた。したがってM-011はorientation選択だけでなく、**preview単体で反転の意味が理解できること**もClose条件へ含める。


**2026-08-27 解決確認**

印刷プレビューの`印刷する`横に、初期ONの`解答を逆さにする`checkboxを追加した。OFFにするとpreviewとnative print対象DOMの解答ページから180°回転classが外れる。実Chromeでtoolbar配置を目視確認し、integration testでもON/OFFに応じたanswer page class切替を固定した。

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

### M-032 Mini Sudokuの2×2境界と外周が均一な罫線階層になっていない

**状態:** Closed (2026-08-27 topology-owned grid rules + browser verification)
**発見契機:** 2026-08-26 frontend visual UX audit / user confirmation
**対象:** Mini Sudoku Web / print grid presentation

「すうじはひとりぼっち」では、4×4外周と2×2 block境界を強い罫線、通常cell境界を細い罫線として読む必要がある。しかし現画面では同じ意味の境界でも黒い太線と薄い灰色線がsegmentごとに混在し、外周も一続きの同一太さに見えない。Web問題面・入力中・採点後・print問題面・print解答面で再現する。ユーザー確認により既知の実Issueであり、answer cellの作成方法が不適切なことに由来すると判定された。

**再現条件**

1. desktop Chrome（1440×1000程度）で、難易度`ふつう`・Seed `A1b2`の`すうじはひとりぼっち`を生成する。この固定条件で再現確認済み。
2. 4つの4×4盤面について、外周・中央の縦境界・中央の横境界を目視比較する。
3. 同じ2×2 block境界であるにもかかわらず、segmentごとに黒い太線と薄い灰色線が混在し、外周も同一strokeとして連続して見えない。
4. 空cellを選択した入力中状態、未回答または誤答で採点した状態でも同じ不均一が残る。
5. `印刷`→print previewの問題ページ/解答ページでも同じ罫線階層の不均一を確認できる。

**方針**

- given / editable / correctionの状態からborderを個別生成せず、4×4 grid topologyから外周・2×2境界・通常cell境界を一意に導出する。
- 同じsemantic boundaryは全segmentで同一のstroke width / colorにする。
- Web / grading / printで同じgrid presentation primitiveを共有する。

**Close条件**

- 4×4外周が全周で同じ太さ・濃さに見える。
- 2×2境界が縦横とも途切れず同じ太さに見える。
- 通常cell境界はそれより明確に細い。
- given / empty / selected / gradedの状態変更でborder hierarchyが変わらない。


**2026-08-27 解決確認**

罫線を各cellの`button`/`span`から完全に分離し、`.mini-sudoku-grid`自身のoverlayが4×4 topologyを一括描画する構造へ変更した。外周と2×2境界は2px、通常cell境界は1pxで、given / empty / selected / gradedの要素種別に依存しない。実Chromeの通常表示・入力中・採点後で全segmentが連続・均一であることを画像で確認した。printも同じ`MiniSudokuGrid` primitiveを使用する。

---

### M-033 可変長の通常計算で解答欄の横位置が行ごとに揃わない

**状態:** Closed (2026-08-27 accepted by user; no code change)
**発見契機:** 2026-08-26 frontend visual UX audit
**対象:** Fraction / decimal inline-expression worksheet layout

分数・小数の通常計算では、answer boxを式の直後へinline配置しているため、operandや分母の長さに応じて同一column内の解答欄が左右へ蛇行する。特に小数の割り算では短い`2 ÷ 4`と長い小数式でanswer box位置が大きく異なり、二列worksheetの垂直な視覚軸が崩れる。整数の基本計算・方程式では固定answer laneがあり、この不均一は同系列UI間でも目立つ。

**再現条件**

1. desktop Chrome（1440×1000程度）で、難易度`ふつう`・Seed `A1b2`の`小数の割り算`を生成する。
2. 左column先頭4問を見る。このSeedでは順に`0.008 ÷ 0.08`、`0.025 ÷ 0.5`、`0.09 ÷ 0.1`、`0.0021 ÷ 0.07`が生成される。
3. 4つの解答boxの左端が同じ垂直laneへ揃わず、実Chrome計測では約`274.3 / 263.8 / 253.3 / 284.8px`となり、最大約31.5px蛇行する。
4. 左右columnを比較してもanswer boxの対応位置が鏡像的な一貫した軸にならない。

**最終判断**

式幅に追従するinline answer boxは現行worksheetでは許容範囲とする。固定answer laneへの変更は行わない。監査時に記載した方針/Close条件はこの判断でsupersedeする。

**2026-08-27 ユーザー判断**

式長に応じてinline解答欄の位置が変わる現状は許容範囲と判断された。固定answer laneへの変更は行わずCloseする。

---

### M-034 Mini Sudoku採点後の訂正数字と×印が盤面へ自然に対応しない

**状態:** Closed (2026-08-27 user-directed graded presentation + browser verification)
**発見契機:** 2026-08-26 frontend visual UX audit
**対象:** Mini Sudoku graded presentation

不正解採点後、canonical correctionの赤数字が通常数字より大幅に小さく、空cellの右下隅へ寄って表示される。また問題全体の`×`が盤面から離れた左下へ置かれ、どの盤面に対するmarkか視覚的な結び付きが弱い。corrected gridを一目で読み取る用途として不自然である。

**再現条件**

1. desktop Chrome（1440×1000程度）で、難易度`ふつう`・Seed `A1b2`の`すうじはひとりぼっち`を生成する。
2. 1問以上を未回答のまま、または誤った数字を入力して`採点`する。
3. 空/誤答cellに表示される赤い正答数字をgiven数字と比較すると、かなり小さく、cell中央ではなく右下隅へ寄っている。
4. 各problemの`×`が盤面に隣接せず左下側へ離れて表示され、どの盤面への評価markか視線だけでは対応づけにくい。
5. 4問を見比べるとcorrectionとmarkのrelative placementが教材gridの中心軸と整合していない。

**最終方針 / Close条件**

- 不正解problem全体の`×`は表示しない。赤いcanonical correctionがあれば誤りは判別できる。
- canonical correctionは従来より十分大きくし、cell内で読み取れること。user/given digitとの差は赤色で表現する。
- 正解problemの`○`は維持してよい。
- graded stateでもMini Sudokuのgrid geometry / border hierarchyを変えない。

**2026-08-27 解決確認**

ユーザー判断により、不正解problem全体へ付けていた`×`は削除した。誤りはcell内の赤いcanonical correctionだけで示し、その文字サイズを従来より大きくした。実Chromeで、赤字だけで訂正箇所を識別でき、離れた`×`が消えていることを確認した。

---

### M-035 `うそつきだれだ`の選択UIと採点後correctionの対応関係が画面だけでは分かりにくい

**状態:** Closed (2026-08-27 user-directed UX fix + browser verification)
**発見契機:** 2026-08-26 frontend visual UX audit
**対象:** Liar puzzle answer / graded presentation

通常画面ではA/B/C/Dがstatement群から大きく右下へ離れたplain textとして置かれ、clickable controlとしてのaffordanceと「この問題の選択肢」という結び付きが弱い。採点後はuser側A/B/C/Dの直後に、赤字・黒丸を含むもう一組のA/B/C/Dが無labelで並び、画面だけではどちらが自分の回答でどちらが正答か確定できない。実装やsourceを読まないと意味を確定できない状態自体をUI defectとみなす。

**再現条件**

1. desktop Chrome（1440×1000程度）で、難易度`ふつう`・Seed `A1b2`の`うそつきだれだ`を生成する。
2. 採点前の各problemを見ると、A/B/C/Dがstatement blockから大きく右側・下側へ離れ、通常時はplain textに見えるためclickable controlであることが弱い。
3. 任意の人物を1人以上選択して`採点`する（未回答のまま採点してもcorrection表示は確認可能）。
4. 採点後、元のA/B/C/D列の直後に赤字/黒丸を含む別のA/B/C/D列が無labelで追加される。
5. `自分の回答`/`正答`等のlegendがないため、画面だけでは二組の役割を一意に決められず、sourceを読まないと意味を確定できない。

**最終方針 / Close条件**

- A/B/C/Dの回答rowを従来より若干左上へ寄せ、statementとの視覚距離を縮める。
- 採点前のuser selectionは従来どおり黒いovalを使う。
- 採点後のcanonical selectionは赤いovalで示し、user selection（黒）と一目で区別できるようにする。
- 色分けで役割が明確になるため、`自分の回答`/`正答`labelの追加は必須としない。

**2026-08-27 解決確認**

回答rowを従来より若干左上へ移動した。採点後はユーザー自身の選択を黒、canonical answerの選択ovalを赤で表示することで、無labelでも二つの役割を色で識別できるようにした。実Chromeで誤答状態を確認し、黒いuser selectionと赤いcanonical selectionが明確に分離して見えることを確認した。

---

---

### M-036 ふりがなONでもworksheet本文の主要テキストにふりがなが付かない

**状態:** Closed (2026-08-27 worksheet ruby integration + browser/test verification)
**発見契機:** 2026-08-26 frontend visual UX audit
**対象:** Worksheet title / instruction / problem text furigana presentation

設定画面で「ふりがな」をONにした状態でも、worksheet headerの学年や採点・印刷等にはふりがなが付く一方、worksheetの主題であるタイトル（例: `分数の足し算`）やinstruction（例: `次の計算をしなさい。...`）、文章系problem本文にはふりがなが付かない。画面上で同じON状態の中にruby付き/なしが混在し、ふりがなtoggleの適用範囲をユーザーが予測できない。詳細設定内ではふりがななし、という既存仕様は維持する。

**再現条件**

1. desktop Chrome（1440×1000程度）の設定画面で`ふりがな`をONにする。
2. 難易度`ふつう`・Seed `A1b2`で`分数の足し算`を生成し、worksheet header・title・instructionを見比べる。学年や`採点`/`印刷`等にはふりがながある一方、`分数の足し算`やinstruction本文には付かない。
3. `うそつきだれだ`を生成すると、statement等の教材本文にもふりがなが付かない。方程式themeでもtitle/instructionの扱いは同じ。
4. TOPへ戻り`ふりがな`をOFFにして同じfamilyを再生成すると、もともとrubyが付いていたheader/controlからは消えるため、toggle自体は動いているがworksheet主要本文だけ適用対象から抜けていることを確認できる。

**最終方針 / Close条件**

- worksheet titleとinstructionを共通`RubyMessage`経路へ通し、ふりがなONなら教材上必要な漢字へrubyを付ける。
- 分数・方程式・Mini Sudoku等のinstructionを代表確認する。
- `うそつきだれだ`は小学1年生で理解できる漢字しか使っていないというユーザー判断により、instruction / statementへの追加rubyは不要とする。
- 数式・変数・grade tag等には不要なrubyを付けない。
- OFFではruby/rtを描画せず、詳細設定内は従来どおりふりがななし。
- ON/OFFでpaperの問題配置やanswer laneを動かさない。

**2026-08-27 解決確認**

worksheet titleとinstructionを設定画面と同じ`RubyMessage`経路へ通し、分数・方程式・Mini Sudoku等の教材文へふりがなを適用した。`うそつきだれだ`は小学1年生で読める漢字のみというユーザー判断によりinstruction/statementを例外として現状維持する。詳細設定内は引き続きふりがななし。分数worksheetを実Chromeで確認し、長いinstructionにもrubyが表示されることを確認した。

---

### M-037 詳細設定に隠れたSeedがTOP復帰後も再生成を固定してしまう

**状態:** Closed (2026-08-27 user-directed state semantics + tests)
**発見契機:** 2026-08-27 user report
**対象:** Settings / Seed generation semantics

手入力Seedでworksheetを生成して`TOPに戻る`と、Seed文字列自体はsettings stateに残る一方、詳細設定は閉じた初期状態へ戻る。そのまま`問題生成`すると見えないSeedを再利用して同じ問題が生成されるため、ユーザーからは「再生成されない原因」が分からない。

**再現条件**

1. TOPで`詳細設定`を開き、Seedに`repeatMe`を入力して問題を生成する。
2. worksheetから`TOPに戻る`。詳細設定は閉じているが内部のSeed値は`repeatMe`のまま残る。
3. 詳細設定を一度も開かずに`問題生成`すると、修正前は`repeatMe`が再利用され同一worksheetになる。

**確定仕様**

- 現在のTOP画面に入ってから詳細設定を一度も開いていなければ、保存されているSeed文字列があっても自動Seedを使用する。
- 現在のTOP画面で詳細設定を一度でも開けば、その後詳細設定を閉じてもSeed欄を設定値として扱う。Seedが空欄なら従来どおり自動Seed。
- worksheetから再び`TOPに戻る`と「このTOP滞在で詳細設定を開いた」という状態はリセットする。
- 同じ規則を`問題生成`とQ1の`印刷`の両方へ適用する。

**解決確認**

SettingsScreen mountごとに`advancedSettingsOpened=false`から開始し、summaryを一度でも操作するとtrueへ単調に遷移する。生成/印刷時は`advancedSettingsOpened && seed !== ''`の場合だけ手入力Seedを採用する。testで「開く→Seed入力→閉じる→生成/印刷」は手入力Seed、「TOP復帰→詳細設定を開かず生成」は自動Seedになることを固定した。


---

### M-038 筆算の描画座標がworksheet方眼と完全には一致していない

**状態:** Closed (2026-08-29 current Chrome / full regression verified)
**発見契機:** 2026-08-27 user report / column-arithmetic visual audit
**対象:** 全筆算themeのWeb通常表示・入力・採点・print問題面/解答面

筆算worksheetは背景方眼を教材上の座標系として使っているが、筆算本体・横線・解答cell・割り算記号の一部がその方眼と異なる位置へ描画されている。数pxのずれでも方眼教材では強い違和感になるため、単なるcosmetic差として扱わない。

**再現条件**

共通条件はdesktop Chrome 1440×1000、難易度`ふつう`、Seed `A1b2`とする。ずれは問題値に依存しないため他Seedでも再現する。

1. `二桁の足し算の筆算`を生成する。背景方眼に対し、各数字cellの中心・筆算本体・横線が約`6.4px`下へずれる。解答cell上端は横線がnormal flowで消費する約1pxを加えて約`7.4px`下へずれる。
2. `二桁の引き算の筆算`、`三・四桁の足し算の筆算`、`三・四桁の引き算の筆算`でも同じ縦ずれを確認できる。
3. `一桁をかける掛け算の筆算`、`二桁をかける掛け算の筆算`でも筆算本体/横線は約`6.4px`下、解答cellは約`7.4px`下となる。二桁乗算で途中計算用の空白rowを持っても、このoffsetは保持されたまま下へ伝播する。
4. `小数の足し算と引き算の筆算`、`小数と整数の掛け算の筆算`、`小数の掛け算の筆算`でも同じ縦ずれを確認できる。decimal markerだけの問題ではなく、lane全体の座標ずれである。
5. `一桁で割る割り算の筆算`、`小数と整数の割り算の筆算`、`小数の割り算の筆算`では、商cellと数字cellは方眼へほぼ一致する一方、割り算記号のSVGだけずれる。修正前は上横線がCSSの`top: -1.6px`により方眼線より約`1.6px`上へ描かれ、曲線下端も同じだけ持ち上がって次の方眼線まで届かない。また曲線の左への張り出しが約`5.2px`あり、2026-08-27のユーザー拡大確認では約`3px`右へ寄せる必要があると判定された。
6. 幅の広いlaneでは「problem cell内へ収める」制約がgrid snapより優先されるため、lane全体が縦方眼から横へずれる。全問題走査では`三・四桁の足し算の筆算`・`二桁をかける掛け算の筆算`・`小数の掛け算の筆算`・`小数の割り算の筆算`で最大約`3.7px`、`二桁で割る割り算の筆算`では最大約`6.3px`を確認した。
7. `印刷`からpreviewを開き、問題ページと解答ページを見る。加減乗算ではWeb通常画面と同じ縦ずれが残り、完成筆算の解答ページにも伝播する。さらに完成筆算では横線要素がnormal flow上で約1pxずつ高さを消費するため、加減算の答えrowは横線後に約1px追加で下へずれ、二桁乗算では最初の横線後のpartial rowsと最終横線後のanswer rowでずれが累積する。割り算の完成手順でも各`.column-division-solution-rule`が約1pxのflow heightを持ち、後続の途中計算rowを段階的に押し下げる。

**監査で確認した13 theme**

- `二桁の足し算の筆算`
- `二桁の引き算の筆算`
- `三・四桁の足し算の筆算`
- `三・四桁の引き算の筆算`
- `一桁をかける掛け算の筆算`
- `二桁をかける掛け算の筆算`
- `一桁で割る割り算の筆算`
- `二桁で割る割り算の筆算`
- `小数の足し算と引き算の筆算`
- `小数と整数の掛け算の筆算`
- `小数と整数の割り算の筆算`
- `小数の掛け算の筆算`
- `小数の割り算の筆算`

**根本原因**

- 非divisionの`.expression` wrapper自体は方眼線へ正しくsnapしている。しかし子の`.column-arithmetic`が`inline-flex`のためinline formatting contextのbaseline/line box leadingが入り、実際の筆算本体だけ約`6.4px`下へ配置される。
- `.column-arithmetic-rule` / `.column-arithmetic-final-rule`をnormal flow上の`border-top`として描いているため、線幅約1pxがlayout heightを消費し、後続の解答rowをさらに約1px押し下げる。完成二桁乗算では2本のruleにより最終answer rowまで累積する。division solutionの`.column-division-solution-rule`も同じくflex column内で1pxのmain-sizeを持ち、途中計算の後続rowへ累積する。
- divisionは`.column-arithmetic-division { display: block; }`なので上記baselineずれを受けないが、修正前の`.column-division-bracket-mark`は`top: -1.6px`でSVG全体を上へ持ち上げ、上横線と曲線下端の双方を方眼から約1.6pxずらしていた。さらに`left: calc(-.22 * var(--worksheet-grid-cell))`が標準表示で約5.2pxの過剰な左overhangを作り、曲線が約3px左へ寄って見えていた。
- lane右端snapは、laneがproblem cell内へ収まらない場合`minimumRight`を優先するため、4〜5桁の加算・二桁乗算・小数乗除算・二桁除算などの幅広laneで方眼snapを破棄する経路がある。

**2026-08-27 割り算記号の追加確認**

ユーザー提供の拡大画像で、上横線だけでなくcurve自体のgeometryも不適切と確認した。標準1440px表示では修正前の左overhangは約5.2pxで、curveを約3px右へ寄せるのが自然だった。また`top:-1.6px`によりcurve下端も約1.6px上へ浮いていた。個別pixel translateは追加せず、SVG boxを方眼1cell高へ正しく合わせた上でoverhang比率を縮小する修正を適用した。M-038全体はnon-divisionの6.4px baselineずれ等が残るためOpenを維持する。

**監査結果の数値**

1440×1000実Chromeで方眼1マスは約`23.58px`。全問題走査では、通常幅laneの水平位置誤差は概ね`0.2px`未満だが、幅広laneでは最大約`6.3px`まで外れる。非division 16問worksheetでは筆算本体の縦ずれが行により約`6.4〜6.6px`、解答cellは約`7.4〜7.6px`で一貫して再現した。division系の商/数字cellは縦誤差最大約`0.16px`で、bracket上横線だけ約`1.4〜1.6px`上へ外れる。 入力中と未回答採点後も代表的な加算/除算で確認し、selection borderやcanonical correctionによる別のgeometry driftはなく、通常表示の座標系をそのまま引き継いでいる。

**方針**

- 方眼を唯一のcanonical coordinate systemとし、筆算側に第二のbaseline/margin座標系を作らない。
- non-division筆算をinline baselineに依存しないblock/grid primitiveへする。
- 横線はgrid line上へoverlayとして描画し、線幅をlayout heightへ加算しない。
- division bracketはSVG boxの上端/下端を隣接する方眼線へ一致させ、上横線のstroke centerと曲線下端をそれぞれ方眼線上へ置く。左overhangもcurveの必要幅だけに限定し、過剰な張り出しを作らない。
- 幅広laneでもcell境界優先でgrid snapを捨てない。必要ならworksheet cell geometry側から4-column配置とgrid pitchの整合を解く。
- theme個別の`margin-top`/`translate`補正は禁止し、Web/graded/printで同じpresentation primitiveを共有する。

**Close条件**

- 全13筆算themeで、各digit cellの中心が方眼cell中心へ一致する。
- 筆算の横線・最終横線・割り算記号の上横線が方眼線上へ一致する。割り算記号の曲線は被除数境界から不自然に左へ離れず、下端が次の方眼線まで届く。
- 解答cellの全辺が方眼線へ一致し、横線の線幅によって次rowがずれない。
- 二桁除算を含む全laneの縦辺が方眼線へ一致する。
- Web通常表示、入力中、採点後、print問題ページ、print解答ページの全状態で同じgeometryを維持する。
- 実Chromeで代表geometry誤差をsubpixel rounding程度（目安`0.25px`未満）まで確認し、画像でも1px級の線/中心ずれが見えないことを目視確認する。
- 重複するtheme別CSS補正を増やさず、browser geometry regressionは共通primitiveに対して最小限追加する。

**解決確認 (2026-08-29)**

current worktreeを前担当者の報告から独立に再検証した。`ROUTE_FILTER=column-` の実Chrome verifierは **13 worksheet route(s)** を実際に選択し、Seed `A1b2` で全13筆算themeを走査した。全themeで `crossings=0`, `gridMismatches=0`。方眼geometryの最大誤差は Web `0.045px`、print問題面 `0.069px`、print解答面 `0.124px`で、Close目安の`0.25px`未満を満たした。加減乗除のdigit center / operator / rule / answer slot / completed solution / division quotient / remainder / bracketを同じpage-grid基準で測定している。

割り算記号は単一SVG path `M 0 28 C 7 21 7 7 0 0 L 100 0`を使用し、旧CSS `border-top`は存在しない。実Chrome screenshotでもcurveが過度に左へ離れる・下端が次方眼線へ届かない再発は確認されなかった。Webとprintはいずれも`columnArithmeticGridVariables(problem, position)`を使用し、theme別pixel補正ではなく共通presentation geometryから座標を供給する。full `pnpm layout:verify`でも **38 routes / 80 worksheet samples** を実走査して成功したためCloseする。

---

### M-039 割り算の筆算で正常な「あまり」入力がvisual overflowとしてrollbackされる

**状態:** Closed (2026-08-29 current Chrome / regression verified)
**発見契機:** 2026-08-29 user report
**対象:** `一桁で割る割り算の筆算`, `二桁で割る割り算の筆算` のWeb入力

**再現条件**

Seed `A1b2`、難易度`ふつう`で次を行う。

1. 割り算の筆算を生成する。
2. 商を入力し、最終商digitから「あまり」へ自動focus移動する。別経路として「あまり」欄を直接clickしても再現する。
3. あまりへ通常の整数、特に複数桁の`21`を入力する。
4. 修正前は入力がrollbackされ、`式が大きすぎます！`が表示される。右端列のremainder fieldで特に確実に再現する。

**期待値**

- あまりは普通のscalar整数入力として保持され、`2`→`1`は`21`になる。
- 商とあまりは別answer coordinateとして保持する。
- 正常入力でsize-limit noticeを出さず、そのまま採点できる。
- 採点後、および`問題に戻る`後もuser inputを保持する。
- 一方、通常の構造化MathLive式に対する本当のvisual/AST size guardは維持する。

**根本原因**

M-038で筆算laneを背景方眼へ正しくsnapした結果、右端列のremainder fieldはlogical `problem-cell`境界を意図的に跨ぎ得る。共通MathLive visual overflow guardは入力内容の意味ではなくpaintの絶対座標をlogical `problem-cell`へcontainment比較していたため、正常なremainder paintをoverflowと誤認してaccepted valueへrollbackしていた。さらに前修正で導入されたpresentation policyは`WorksheetAnswerField`から渡されていたものの、`AutoDrillApp → WorksheetScreen`の手書きcallback wrapperが第5引数を転送せず、productionではdefault policyへ戻っていた。

MathLive shadow DOMを実Chromeで計測すると、正常な`21`でもfocus/caret用内部layout boxはfieldの`clientWidth`を超え、paint widthも2方眼cellと一致するため、固定scalar fieldを構造化数式と同じpaint-containment規則で判定すること自体が不安定だった。

**修正**

- MathLive入力のvisual-size責務をtyped presentation policy `intrinsic-expression | fixed-scalar`で分離した。
- 通常の可変長・構造化数式は従来どおり`intrinsic-expression`としてvisual overflow guardを通す。
- remainderのように固定方眼slotへ置くscalar整数は`fixed-scalar`としてparser/AST validationを通しつつ、構造化数式向けpaint-containment guardを適用しない。slot名やtheme IDをgeneric validator内で分岐するbypassは追加しない。
- callbackを`onMathInput={updateMathLiveAnswer}`として直接forwardし、presentation policyが中間wrapperで欠落する経路を除去した。
- browser verifierも「logical problem-cellを跨ぐこと」自体を失敗条件にせず、page overflow / 隣接lane overlap / grid alignmentという実際のlayout invariantを検証する。

**対応テスト / Close条件**

- `AutoDrillApp.test.tsx`: 問題生成 → 商入力 → remainderへfocus移動 → `21`入力 → noticeなし → grade requestで商/あまりが別tuple coordinate → 採点後も値保持、をbehavior testで固定。
- 実Chrome: 一桁・二桁除算の双方で、自動focus・直接click・物理keyboard `2`→`1`を確認し、`21`順序保持・noticeなし・採点成功を確認。
- 実Chrome: 採点後に`問題に戻る`を押しても商・remainder・直接入力したremainderが保持され、再編集可能であることを確認。
- 既存AST size-limit regressionも維持し、`pnpm test`で20 files / 167 testsが成功。
- structured MathLiveのbrowser regressionでは、parserが受理する値を一度acceptedにした後にfieldを`400px`へ拡大して純粋なpaint overflowを発生させ、直前値へのrollback・`式が大きすぎます！`表示・rollback後containmentを確認する。固定scalar policy追加後もこの真のvisual size guardは成功している。
- `ROUTE_FILTER=column-division`およびfull browser verifierが成功し、入力修正でM-038の方眼geometryを壊していないことを確認。

以上を満たしたためCloseする。

---

### M-040 user-visible state / 1-step interaction のbrowser coverageが網羅的でない

**状態:** Closed (2026-08-29 exhaustive interaction sweep verified)
**発見契機:** M-039 retrospective / user request
**対象:** Settings / grading settings / worksheet editing・graded / input panel / print preview / 全active theme

**問題**

M-039は「あまり欄へ普通の整数を入力する」という自明な1-step操作だけで再現したにもかかわらず、従来のbrowser verifierは代表routeごとの手書きprobe中心であり、UIに存在する全操作や全answer位置を機械的に列挙していなかった。このため、特定theme・特定位置・新規button等が既存probeに選ばれなければ、ユーザが1回操作するだけで発生する回帰でもCIを通過できた。

**修正方針**

- user-visible UIを有限状態グラフとして扱い、canonical stateごとに現在enabledな `button / input / summary / MathLive` actionをcensusする。未知のenabled actionがmanifestへ追加されただけで未検証のまま通らないようfailureにする。
- Settingsのoptionは代表値だけでなく、難易度4件・学年9件・active theme 38件を実際に選択して到達可能性を確認する。
- worksheetは全38 active themeについて先頭Seedで**全editable affordance**を実操作する。通常MathLive、連立方程式の各x/y coordinate、筆算digit slot、長除法remainder、Mini Sudokuのeditable cell、liar choice等をproblem位置ごとに網羅する。MathLiveは1文字だけでなく`2`→`1`の複数文字保持も確認する。
- input panelは出現するdistinct action surfaceをsignature化し、各surfaceでenabledな全buttonをcanonical baselineから1-stepずつ実行する。action間でstateを持ち越さず、必要ならfresh worksheet / 別answer coordinateへ分離する。
- `worksheetSampleCount`だけでなくeditable affordance/action countも0ならfailureにし、filter誤りによるsilent successを禁止する。
- transientな`grading` lock/failure復帰はdeferred-engine unit testで補完し、browserで人工的raceを作る代わりに状態機械の同期lockを直接検証する。

**状態グラフ簡素化**

採点後の`別の問題を解く`shortcutは削除した。対応する`generateDifferentWorksheet` callback、`replacing` phase、専用testsも最後のconsumerとともに削除し、別worksheetへ進む場合は`TOPに戻る`を経由する。graded stateのaction censusは`問題に戻る`・`もう一回問題を解く`・`印刷`・`TOPに戻る`だけを許可し、shortcutが意図せず復活してもCIで検出する。

**包括化によって実際に検出した既存欠陥**

1. `.advanced-settings-body { display: grid }` がnative `<details>` の閉状態非表示を上書きし、詳細設定を閉じてもSeed/採点設定がlayout上は残る状態をcensusが検出した。`.advanced-settings:not([open]) > .advanced-settings-body { display: none; }`で閉状態を明示した。
2. 連立方程式ではfull keypadの分数・平方根・小数・`±`等がenabledなのに、Web側`isCoordinateAnswer`がinteger/negative以外をrollbackしていた。Rustの`OrderedPair` gradingは外側2要素tupleのshapeを所有しており、各coordinate内容の数学的評価はRustへ委譲できるため、このWeb独自filterを削除した。UI/editor contractが許可したAnswerNodeを保持し、正誤判定はRustをSoTとする。

**Close条件 / regression**

- 実Chrome state graph: **8 canonical user-visible states / 37 one-step edges** を成功。`input-panel-open`も独立stateとしてcensusし、そこからの閉じる/Escape/印刷/採点/TOPを含む。
- Settings option sweep: **4 difficulties / 9 grades / 38 active themes** を成功。
- 全active-theme deterministic Seed `A1b2` interaction sweep: **42 worksheet samples / 1,139 editable-affordance actions / 126 distinct input-panel actions** を成功（signed arithmeticの追加Seed regressionを含む）。Seedは詳細設定を実UIで開いて入力し、生成後footerが要求Seedと一致することまでassertする。
- 連立方程式単独では **24 coordinate affordances / 24 enabled keypad actions** をすべて成功。
- CI必須の`pnpm layout:verify`へ同じ検証を統合し、手動専用scriptにはしない。
- 最新full `pnpm layout:verify`（Seed `A1b2,M7x9` + signed arithmetic追加Seed + native print probes）で **38 routes / 80 worksheet samples / 1,139 editable-affordance actions / 126 distinct input-panel actions** を実走査し、actual Chrome PDF生成を含めて成功。
- keypadのcursorはcaret/selected slotの移動、`確定`は次coordinate/problemへのselection遷移までeffect assertionし、単なる「clickしてexceptionなし」では成功扱いしない。CustomSelectのArrowDown/ArrowUp/Home/End/Escape/Space/Enterとdigit-gridの数字/左右/Backspace/Delete/Enterもunit behavior testで全明示keyboard branchを固定。

以上を満たし、full複数Seed・print runでも同じcontractが成功したためCloseする。

---

## Low

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

### L-008 mobile worksheet / input panel layoutが狭幅viewportへ適応していない

**状態:** Open (alpha non-blocking)
**発見契機:** 2026-08-26 frontend visual UX audit
**対象:** Mobile / responsive worksheet UX

mobileはalpha support対象外のためLow。390px幅の実画面で、共通input panelがviewportより横長となり、数字key・矢印・clear・commit等の主要操作が画面外へ出る。通常計算、分数、中学数式、筆算、Mini Sudokuで同系統を確認した。また長い二次方程式ではexpressionと`x =` answer boxが衝突し、`うそつきだれだ`ではstatementとA/B/C/D choicesが重なる。個別themeのpatchではなく、worksheet / keypad / answer placementのresponsive layout全体としてbeta移行時に解決する。

**再現条件**

1. Chromeを390×844のmobile viewportにする。
2. 難易度`ふつう`・Seed `A1b2`で`1けたのたしざん(1)`を生成し、問題1の解答欄を選択する。input panelがviewportより横長で、数字key/移動/delete/clear/確定の一部が画面外へ出る。
3. `分数の足し算`、筆算、Mini Sudoku、中学数式でも解答欄を開くと同系統の横overflowを確認できる。
4. `二次方程式(3)`では長い式の右端と`x =`/answer boxが衝突する問題が複数出る。
5. `うそつきだれだ`ではstatement本文とA/B/C/D choiceが同じ領域へ入り込み、文字が重なる。

alphaではsupport対象外のため、上記はすべてLow / alpha非blockingとして扱う。

**Close条件**

- 主要input controlが横scrollなしで狭幅viewport内へ収まる。
- 長い数式とanswer fieldが重ならない。
- liar choicesがstatementへ重ならず、対応問題との関係を維持する。
- 代表familyを実mobile browserで目視確認する。

---

### L-009 分数系instructionが日本語の語中で不自然に改行される

**状態:** Closed (2026-08-27 accepted by user; no code change)
**発見契機:** 2026-08-26 frontend visual UX audit
**対象:** Worksheet instruction typography

分数の足し算・引き算・掛け算・割り算等で、instruction末尾の「答えなさい。」が行端にかかると`答` / `えなさい。`の間で改行される。機能上の実害は小さいが、教材画面として日本語組版の違和感が強い。themeごとの手動改行ではなく、instruction copy / inline grouping / available widthの共通規則で解消する。

**再現条件**

1. desktop Chrome（1440×1000程度）で、難易度`ふつう`・Seed `A1b2`の`分数の足し算`を生成する。instructionはSeed非依存なので同themeなら他Seedでも再現する。
2. paper上部のinstructionが2行へwrapしたケースを見る。
3. 行末付近の`答えなさい。`が日本語の語単位で保持されず、`答`だけが前行に残り`えなさい。`が次行へ送られる等の不自然な語中分割が発生する。
4. viewport全体では十分な余白があり、theme固有のmanual改行ではなくinstructionの共通組版規則で再現する。

**最終判断**

この語中改行は現状許容とし、修正対象から外す。従来のClose条件はsupersedeする。

**2026-08-27 ユーザー判断**

この改行差は修正不要と判断されたため、現状を許容してCloseする。

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
**状態:** Closed (2026-08-18 independent verification)
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

**2026-08-18 再監査対応メモ**

2026-08-18再監査追補: `ProblemSetIdentity`は`from_parts`を唯一のvalidation pathとし、`new` / `FromStr` / custom `Deserialize`の全経路でschema・seed・nonzero theme/revisionを同一に検証する。`Problem`内部は生の`AnswerSchema` / `AnswerNode`ではなくprivate `ValidatedAnswerSchema` / `CanonicalAnswer`を保持し、`Problem::generated`でschema自体の構造条件、canonical answerの具体的range/shape、DigitGridのtuple length/digit域を一括検証する。Mini Sudokuは`MINI_SUDOKU_GRID_SPEC`をgivens validation・registration・answer contractの単一SoTとする。native Rust / WASM grading境界のexternal `AnswerSchema`もstructural/canonical-answer validationを通し、`grade_answer_with_schema`は不正schemaを`GradeError`として返す。MathLive parseのnative/WASM境界も`AnswerInputInterface`の構造条件を先に検証する。

同日追加対応: `WorkedSolution`をgeneratorが任意field/enum literalとして構築するAPIを廃止し、`Problem::generated`がcolumn promptとcanonical answerからprivate worked-solution domain valueを一意に導出する。partial product / long-division stepのwire shapeは`wire.rs`だけがDTOとして所有するため、promptと無関係なdivisor/product/offsetをsafe generator APIから注入できない。Liar Puzzle canonical answerにもpeople-domain contractを追加し、昇順・重複なし・`1..=people_count`・空/all-liar除外をaggregate boundaryで保証する。candidate sourceは`Result<Option<Problem>, GenerationError>`となり、`ProblemInvariantError` / `WorksheetInvariantError`は`invalid_generated_problem` / `invalid_generated_worksheet`としてWASM/Webまで伝播し、production generatorの`Problem::generated(...).expect(...)`およびWorksheet invariantの`AttemptLimit`偽装を廃止した。独立再確認待ちのためPending。

同日最終追補: 再監査で残った「schema上合法でもpromptの数学的正解ではないcanonical answerを`Problem::generated`へ渡せる」穴を閉じた。`semantics.rs`をgeneratorから独立した数学的authorityとして追加し、`Problem::generated`の`CanonicalAnswer` constructionがpromptごとのexact semantic verificationを必ず通す。Addition / Arithmetic / ColumnArithmeticはexact rational評価、Linear / Simultaneousは方程式への代入と一意性、Quadraticは解集合の個数とexact quadratic-number代入、Liar Puzzleはstatement truthから求めた唯一解mask、Mini Sudokuはgivensを保つ唯一の合法盤面との一致を検証する。既存`generator_support::evaluate_expression`、Liar solver、Mini Sudoku solverも同じ`semantics.rs`実装へ寄せ、validator専用の第二数学実装を作らない。`1+1=3`、`12×3=35`を含む全prompt familyのsemantic mismatch regression testを追加した。独立再確認待ちのためPending。

2026-08-18独立再監査でH-009の全Close条件をPASS。prompt/canonical answer semantic consistencyを含め、safe Rust APIから不正aggregateを構築できないことを確認したためClosed。


---

### H-010 特殊effortを`Option<f64>`で逃がし、graph/vector/scalar/scoreを複数SoTとして保持している

**severity:** High
**状態:** Closed (2026-08-18 independent verification)
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

**2026-08-18 再監査対応メモ**

2026-08-18再監査追補: `OperationEffort`のcached scoreを削除し、通常modelは`EffortModel::Operations(OperationPlan)`としてprimitive operation列だけをevidenceに保持する。operation vectorとweighted scoreはplanから都度導出する。repository全体を確認してprerequisite edgeのproduct consumerが存在しなかったため、`SolutionGraph` / `SolutionStep` / `depends_on` DAG abstraction自体をcore・wire・Webから削除した。Web DTOは`operation_plan` / `operation_vector` / `theme_specific_effort` / `effort`を1つのmodelから投影する。将来DAGが必要なら実consumerが生じた時点で再設計する。最上位current文書`docs/principles.md`に残っていた`effort operation graph`表記も`effort operation plan`へ同期した。独立再確認待ちのためPending。

2026-08-18独立再監査でH-010の全Close条件をPASS。その後のcleanupで意味を追加しない一field `OperationEffort` wrapperも削除し、`EffortModel::Operations(OperationPlan)`へ直接単純化した。


---

### H-011 `ProblemGenerator` traitがoptional/default methodの組合せでcapability protocolを表現している

**severity:** High
**状態:** Closed (2026-08-18 independent verification)
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

**2026-08-18 再監査対応メモ**

2026-08-18再監査追補: method組合せだけでなくstrategy valueのinvariantもconstruction時に検証する。`SamplingStrategy`のvariantはprivate化し、validated constructorだけをtheme側へ公開した。`AnswerDomain`はnon-empty、`SamplingLayers`はnon-emptyかつminimum合計がworksheet problem count以下、constructive multiplierは`NonZeroUsize`、classifierの生`usize`はbounded `LayerIndex`へ変換する。空domain/layerや範囲外classifierは`SamplingError`としてgeneration boundaryへ伝播し、`next_bounded(0)` panicや`AttemptLimit`への偽装を行わない。

同日追加対応: answer-conditioned callbackの返却Problemはrequested canonical answerと即照合し、不一致は`SamplingError::AnswerConditionMismatch`とする。constructive-layered callbackもrequested layerと実classifier結果を照合し、不一致は`RequestedLayerMismatch`とする。single-problem / worksheet両経路で同じvalidationを使い、違反をretryや`AttemptLimit`へ変換しない。WASMの`invalid_sampling_strategy` / `invalid_registry`およびaggregate contract errorはWeb `DrillEngineErrorKind`でも独立kindを保持し、`invalid_dto`へ潰さない。独立再確認待ちのためPending。


---

### H-012 `ThemeRegistrationSpec`がnamed fieldsになっただけで、theme contractのinvalid combinationを許している

**severity:** High
**状態:** Closed (2026-08-18 independent verification)
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

**2026-08-18 再監査対応メモ**

2026-08-18再監査追補: 完成後の`ThemeRegistration`も全fieldをprivate化し、内部identityを`ThemeId` / `GeneratorRevision`のまま保持する。raw `u32`はgetterでのみprojectionする。registryは`LazyLock<Result<Registry, RegistryError>>`として構築結果を保持し、duplicate theme IDを最初のaccess時の`expect` panicにせず、registration/generator/Web contract/generation boundaryへ明示的に伝播する。Mini Sudokuの1/4/16 domain factは`MINI_SUDOKU_GRID_SPEC`へ一元化し、grid validation・input contract・answer tuple validationに加えてsolverのdigit iterationも同じspecを参照する。Web boundaryでも`invalid_registry`を`invalid_dto`へ潰さず独立kindとして保持する。独立再確認待ちのためPending。


---

### M-027 `うそつきだれだ`の論理式長を`Operation::Identity` node数へ偽装している

**severity:** Medium
**状態:** Closed (2026-08-18 independent verification)
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

**2026-08-17 実装後メモ**

2026-08-17対応: `Operation::Identity` node数によるformula-length偽装を削除し、うそつきだれだの式長を`EffortModel::ThemeSpecific`として明示した。無関係primitive weightへの依存は消滅した。独立確認待ちのためPending。


---

### M-028 MathLive移行後も旧`EditorState` / `apply_editor_action` state machineをcompatibility目的で保持している

**severity:** Medium
**状態:** Closed (2026-08-18 independent verification)
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

**2026-08-17 実装後メモ**

2026-08-17対応: 旧`EditorState` / `EditorAction` / `apply_editor_action` state machine、WASM endpoint、wire export、Web adapterの未使用compatibility pathと対応testを削除した。MathLiveが必要とするinput capability validationだけを`input.rs`へ分離し、canonical architecture docsも現行pathへ更新した。独立確認待ちのためPending。


---

### M-029 theme capabilityが`ThemeTag`と`ThemePresentationPolicy`へ二重登録されている

**severity:** Medium
**状態:** Closed (2026-08-18 independent verification)
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

**2026-08-17 実装後メモ**

2026-08-17対応: `ColumnArithmetic` / `PrintRecommended`をtaxonomy tagから削除し、behavioral capabilityは`ThemePresentationPolicy`のみをSoTとした。Webの印刷推奨表示・筆算分類・PDF testも`presentation`から導出するよう変更した。独立確認待ちのためPending。


---

### M-030 `problem_key()`がdedup keyとして`ProblemPrompt`全体を所有cloneし、sort比較中にも再構築している

**severity:** Medium
**状態:** Closed (2026-08-18 independent verification)
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

**2026-08-17 実装後メモ**

2026-08-17対応: dedup identityを専用`ProblemKey`へ分離し、`Candidate`生成時に一度だけcanonical keyを構築するよう変更した。sort comparatorはAST/Vecを含む`ProblemPrompt`を比較ごとにclone/rebuildしない。

2026-08-18追加対応: 再監査で指摘されたdedup用HashSetへのowned `ProblemKey::clone()`も除去した。`Candidate.key`を`Rc<ProblemKey>`とし、AST / `Vec<LiarStatement>` / MiniSudoku gridを含むsemantic key本体はCandidate生成時の1 allocationだけを所有する。temporary dedup setへのinsertは`Rc` clone（参照カウント増加）のみで、heap-owning semantic keyを複製しない。collision-proneなhash-only identityには落としていない。独立再確認待ちのためPending。


---

### M-031 theme固有regression testが共通`generator.rs`へ集中し、theme追加の変更局所性を壊している

**severity:** Medium
**状態:** Closed (2026-08-18 independent verification)
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

**2026-08-18 再監査対応メモ**

2026-08-18再監査追補: 再監査で中央に残っていた九九/逆九九の具体的effort testも削除した。`multiplication_table.rs`が`log10(answer)`のtheme-specific modelを、`division_table.rs`が3回のinverse-table probe planをそれぞれlocal testとして所有する。中央`generator.rs` testはregistry-wide invariantとsampling frameworkのvalue invariantだけを扱い、九九・分数・二次方程式・筆算等のtheme固有semanticsを参照しない。独立再確認待ちのためPending。


---

### L-006 現worktreeのRust sourceが`cargo fmt --all -- --check`を通らない

**severity:** Low
**状態:** Closed (2026-08-18 independent verification)
**該当:** 現在の未コミットRust差分（`generator.rs`, `generator_support.rs`, `model.rs`, `theme.rs`, `themes/basic_arithmetic.rs`, `themes/mini_sudoku.rs`, `themes/mod.rs`, `themes/multiplication_table.rs`等）

**具体的なコード上の証拠**

2026-08-17監査時に`cargo fmt --all -- --check`がexit 1となり、複数Rust fileにrustfmt差分を報告した。一方`cargo check` / test / Clippyはpassしている。監査ではproduction codeを変更しない指示のため、rustfmt自体は実行していない。

**なぜRustとして不適切か / 保守性上問題か**

formatting自体は意味論問題ではないが、Rust repositoryのcanonical formattingから外れた差分はreview noiseを増やし、CIでfmtを必須化した場合にmergeを止める。

**よりRustらしい修正方向**

現在の価値ある未コミット差分を破壊しない状態で、実装担当が意図した変更を確定した後に`cargo fmt --all`を適用し、差分をreviewする。

**Close条件**

- `cargo fmt --all -- --check`がcleanにpassする。

**2026-08-17 実装後メモ**

2026-08-17対応: `cargo fmt --all`を適用し、後続のClippy/workspace testまで通過した。独立確認待ちのためPending。


---

### L-007 `SamplingLayerSpec.key`が宣言されるだけで一度も読まれていない

**severity:** Low
**状態:** Closed (2026-08-18 independent verification)
**該当:** `crates/drill-core/src/theme.rs::SamplingLayerSpec`, layered theme declarations

**具体的なコード上の証拠**

`SamplingLayerSpec`は`key: &'static str`, `weight`, `minimum`を持つが、`crates/drill-core/src`を横断検索しても`.key`のconsumerが存在しない。layered samplerは配列indexだけでlayerを識別している。各themeは意味のある文字列keyを記述しているがproduction semanticsには影響しない。

**なぜRustとして不適切か / 保守性上問題か**

未使用metadataは「将来使うかもしれない」schema residueであり、AI実装者にkeyの一意性・安定性・wire意味が必要だと誤認させる。`principles.md`の一時/試験実装の残骸を残さない方針にも反する。

**よりRustらしい修正方向**

現在不要ならfieldを削除する。diagnostics/telemetry等でstable layer identityが本当に必要なら、そのconsumerと型上の意味を同時に導入する。

**Close条件**

- `key`を削除するか、現行product requirementに基づく明確なconsumerが存在する。

**2026-08-17 実装後メモ**

2026-08-17対応: consumerのない`SamplingLayerSpec.key`を削除し、layer identityは現行samplerが実際に使用するindexだけに単純化した。独立確認待ちのためPending。

---

## 2026-08-18 post-audit cleanup

### NEW-M-001 exact rational primitiveが`normalize.rs`と`semantics.rs`へ二重実装されている

**severity:** Medium
**状態:** Closed (2026-08-18 independent re-verification)
**該当:** `crates/drill-core/src/exact.rs`, `normalize.rs`, `semantics.rs`

独立監査で、exact rationalのcanonicalization・checked四則・符号反転が`normalize.rs`と`semantics.rs`のprivate `ExactRational`へ重複していることを確認した。prompt semantics自体は一元化されていたためH-009はClosedできるが、共有数学primitiveのSoT分裂として別Issueにする。

**Close条件**

- exact rationalのcanonicalization/checked arithmeticを1つのcrate-private primitiveへ統合する。
- normalizationとprompt semanticsは責務を分離したまま同じprimitiveを再利用する。

**2026-08-18 実装後メモ**

`exact.rs::ExactRational`へcanonicalization、checked add/subtract/multiply/divide/negate、exact square rootを統合した。`normalize.rs`はAnswer AST→exact rational変換とAST projection、`semantics.rs`はprompt/equation/puzzle semanticsだけを所有し、同じexact arithmetic primitiveを共有する。2026-08-18再監査と修正後のrepository-wide確認でClose条件を満たすことを確認しClosed。

**2026-08-18 再監査追補**

再監査で`effort.rs`にchecked rational addの手書き再実装、`themes/equations.rs`にAnswerNode→RationalCoefficientの独自projectionが残っていることを確認して再Openした。修正では、加算を`RationalCoefficient::checked_add`へ直接委譲し、AnswerNode→coefficient projectionを`exact_value.rs::rational_coefficient_from_answer`へ統合した。

**2026-08-18 検証**

`rational_add` / theme-local AnswerNode rational projectionの残存がないことをrepository-wideで確認し、Rust fmt/check/clippy/testを通過したためClose。

**2026-08-18 再々監査対応**

独立再監査で、平方根簡約semanticが`effort.rs::square_root_decomposition`と`themes/equations.rs::simplify_square_root`へ重複していることを確認して再Openした。`exact.rs::square_free_sqrt_decomposition`へcheckedな平方根分解を集約し、equation constructionとquadratic-formula effortの双方が同じprimitiveを利用するよう修正した。人間が平方因子を探索するoperation-count生成はeffort固有semanticなので`effort.rs`へ残す。Rust全gate通過。独立再確認待ち。

**2026-08-18 独立再判定**

独立再判定で、平方根square-free分解が`exact.rs::square_free_sqrt_decomposition`へ一元化され、`effort.rs` / `themes/equations.rs`が共有primitiveを利用していることをcurrent codeから再確認した。Close可判定。

### NEW-M-002 canonical architecture docsがcurrent codeと不一致

**severity:** Medium
**状態:** Closed
**該当:** `docs/architecture/problem-schema.md`, `answer-ast.md`, `theme-system.md`

独立監査で、Mini Sudoku registrationの記載漏れ、九九effortの旧`BigNum`説明、root/plus-minusの「数値評価未定義」という旧説明、theme family一覧のMini Sudoku漏れを確認した。

**Close条件**

- canonical architecture docsがcurrent registry / EffortModel / exact radical・plus-minus semantics / theme modulesと一致する。

**2026-08-18 実装後メモ**

Problem schemaのcurrent registration表へID 38 Mini Sudokuを追加し、九九を`EffortModel::ThemeSpecific(log10(c))`として記述した。Answer ASTはexact square-root normalization、quadratic radical semantics、plus-minus solution-set expansionを現行実装どおり記述し、Theme Systemのfamily一覧へ`mini_sudoku.rs`を追加した。2026-08-18再監査と修正後のrepository-wide確認でClose条件を満たすことを確認しClosed。

### NEW-L-001 alpha 1.1互換引数がproduction PDF APIに残存

**severity:** Low
**状態:** Closed
**該当:** `apps/web/src/pdf/worksheet-pdf.tsx::openWorksheetPdf`, `apps/web/src/components/AutoDrillApp.tsx::openWorksheetPdfLazy`

`targetWindow`はコメント上alpha 1.1 call-site compatibility専用で、current callerはすべて`undefined`を渡していた。pre-releaseではhistoric compatibilityをproduction codeへ保持しない原則に反する。

**Close条件**

- `targetWindow`互換引数と対応cleanupをproduction API/call site/testから削除する。

**2026-08-18 実装後メモ**

`openWorksheetPdf` / lazy wrapperから`targetWindow`を削除し、current signatureを`(worksheet, metadata?)`へ単純化した。call siteとtestsを同期し、旧`window.open`前提のtest residueも削除した。2026-08-18再監査と修正後のrepository-wide確認でClose条件を満たすことを確認しClosed。

### NEW-H-001 effort plan builderがsafe public API上のhidden preconditionをpanicへ逃がす

**severity:** High
**状態:** Closed (2026-08-18 re-audit verification)
**該当:** `crates/drill-core/src/effort.rs`, `themes/basic_arithmetic.rs`, `themes/equations.rs`, `themes/column_arithmetic.rs`

独立監査で、generator-specific effort builderが外部public APIとして公開され、zero divisor・singular system・overflow・one-digit domain等の条件を`debug_assert` / `expect("bounded ...")`へ依存していた。

**対応**

- generator-specific builderをcrate内部へ縮小した。
- 一次・連立・二次方程式の失敗可能なplan構築を`Option<OperationPlan>`へ変更し、zero/singular/overflow/domain mismatchを明示的に失敗させる。
- 一桁加減・二桁加算・余り付き整数除算も入力domainを入口で検証する。
- theme callerは失敗をcandidate rejectionまたは`InvalidGeneratedProblem`として処理し、semantic validation前のpanicへ逃がさない。

**Close条件**

- safe external APIからgenerator固有のhidden preconditionを踏めない。
- failure可能な方程式effort constructionがpanic conventionではなく型で失敗を表す。

2026-08-18対応・検証済みのためClosed。

**2026-08-18 再監査追補**

再監査で`RationalCoefficient::new(i64::MIN, 3)`同士の除算effortがreciprocal denominator変換の`expect`へ到達できる反例を確認して再Openした。rational add/sub/mul/div、LCM/reduction周辺の失敗可能なoperation-plan構築を`Option`へ伝播し、巨大中間値はpanicではなくplan構築失敗として返す。さらに、整数乗算modelを呼ぶ前にchecked resultを確定し、private helperのbounded前提を呼出箇所で機械的に保証する。極値回帰testを追加した。

**2026-08-18 検証**

`bounded rational` / `bounded reciprocal` panic residueがないことを確認し、`i64::MIN/3`同士のdivision effortがpanicせず`None`を返す回帰testを追加した。Rust全gate通過のためClose。

### NEW-M-003 Answer AST limitをRustとWebで二重管理している

**severity:** Medium
**状態:** Closed (2026-08-18 independent re-verification)
**該当:** `crates/drill-core/src/contract.rs`, `apps/web/src/domain/wasm-adapter.ts`

Webが`MAX_ANSWER_AST_SIZE = 18`を手入力しており、Rust側のlimit変更時にgenerated freshnessがPASSしたままdriftできた。

**対応**

`WebContract.max_answer_ast_size`をRust `MAX_ANSWER_AST_SIZE`から生成し、Web adapterはgenerated contract値を使用する。

**Close条件**

- AST size limitの数値SoTがRustに1つだけ存在する。

2026-08-18 generated contract freshnessまで確認しClosed。

**2026-08-18 再監査追補**

production adapterはRust generated contractを参照していたが、Web testに`18`の手書き境界値が残っていたため再Openした。`wasm-adapter.test.ts`と`AutoDrillApp.test.tsx`も`DRILL_CORE_CONTRACT.max_answer_ast_size`から境界を導出するよう修正した。

**2026-08-18 検証**

WebのAST-limit testがgenerated `DRILL_CORE_CONTRACT.max_answer_ast_size`を参照し、手書き`repeat(18)` / `length: 18`が残っていないことを確認した。generated freshnessとWeb全test通過のためClose。

**2026-08-18 再々監査対応**

独立再監査で`apps/web/src/test/fixtures.ts`のmock MathLive parserに`latex.length > 18`が残っていることを確認して再Openした。fixtureもgenerated `DRILL_CORE_CONTRACT.max_answer_ast_size`を参照するよう修正し、Web source全体にAST-limit由来の手書き`18`が残っていないことを検索確認した。generated freshness・TypeScript・ESLint・Web全test通過。独立再確認待ち。

**2026-08-18 独立再判定**

独立再判定で、Web production/test/fixtureがすべてgenerated `DRILL_CORE_CONTRACT.max_answer_ast_size`へ追随し、AST limitの手書き18が残っていないことを確認した。Close可判定。

### NEW-M-004 consumerのないWeightProfile / WeightMultipliers / resolved_weightsが残る

**severity:** Medium
**状態:** Closed
**該当:** `crates/drill-core/src/effort.rs`, `registry.rs`

現行productでgrade/theme/mastery multiplier layerのconsumerがなく、`resolved_weights(registration)`もregistrationを無視してdefault weightを返していた。

**対応**

`WeightProfile`, `WeightMultipliers`, `EffortWeights` alias, `resolved_weights`とalpha-era commentを削除し、現行generatorは`OperationWeights::default()`を直接使用する。将来必要になった時点でconsumerとともに再導入する。

**Close条件**

- future-only weight abstractionがproduction surfaceに存在しない。

2026-08-18 repository-wide検索で残存なしを確認しClosed。

### NEW-M-005 ProblemPrompt / worked solutionのi64をJS numberへ無条件に落とせる

**severity:** Medium
**状態:** Closed
**該当:** `crates/drill-core/src/model.rs`, `apps/web/src/domain/wasm-adapter.ts`

AnswerNode等はexact integerをdecimal string化していた一方、prompt/worked-solutionの`i64`はJavaScript `number`へ投影され、domain拡張時にsilent precision lossが可能だった。

**対応**

- `Problem::generated`がProblemPromptとderived WorkedSolutionのnumber-wire整数をJavaScript safe-integer範囲に制約する。
- Web adapterも`Number.isSafeInteger`で検証する。
- exact 64-bit rangeが必要なAnswerNode / AnswerSchema / BigNum等は従来どおりdecimal stringを使う。
- safe-integer外promptを拒否するnegative testを追加した。

**Close条件**

- Rustが`number`としてwireへ出す整数についてexact JS表現可能性をdomain boundaryで保証する。

2026-08-18検証済みのためClosed。

### NEW-M-006 GradeErrorのsemantic codeがWASM/Web境界でgeneric errorへ潰れる

**severity:** Medium
**状態:** Closed
**該当:** `crates/drill-core/src/grade.rs`, `crates/drill-wasm/src/lib.rs`, `apps/web/src/domain/wasm-adapter.ts`

`InvalidAnswerSchema`と`ExpectedAnswerOutsideSchema`がWASMで`invalid_request`へ統合され、Webではさらに`invalid_dto`へ落ちていた。

**対応**

core `GradeError::code()`から`invalid_answer_schema` / `expected_answer_outside_schema`を定義し、WASM envelopeとWeb `DrillEngineErrorKind`まで保持する。Rust/WASMとWeb双方へ境界testを追加した。

**Close条件**

- grade invariant failureのsemantic codeがRust→WASM→Webで保存される。

2026-08-18検証済みのためClosed。

### NEW-H-002 public `DeterministicRng::next_bounded`がzero boundでpanicする

**severity:** High
**状態:** Closed (2026-08-18 independent re-verification)
**該当:** `crates/drill-core/src/rng.rs`, `crates/drill-core/src/lib.rs`

`DeterministicRng`がcrate外へpublic exportされている一方、`next_bounded(0)`は`assert!`でpanicするhidden preconditionを持っていた。現product consumerはdrill-core内部だけなので、pre-releaseで不要なexternal API surfaceを閉じる。

**対応**

- `DeterministicRng`とgeneration用methodを`pub(crate)`へ縮小する。
- `lib.rs`から`DeterministicRng`のpublic re-exportを削除する。
- external current consumerのないseed conversion helperもcrate-privateに保つ。

**Close条件**

- crate外safe APIから`next_bounded(0)`へ到達できない。

**2026-08-18 検証**

`DeterministicRng` / `next_bounded`はcrate-privateであることを確認した。

**2026-08-18 再々監査対応**

独立再監査で`seed_to_u64`にもcrate外current consumerが存在しないことを確認して再Openした。`seed_to_u64`を`pub(crate)`へ縮小し、`lib.rs`からre-exportを削除した。repository-wideでRNG関連の不要public surfaceがないこととRust全gate通過を確認した。独立再確認待ち。

**2026-08-18 独立再判定**

独立再判定で、`DeterministicRng` / `next_bounded` / `seed_to_u64`がcrate-privateとなり、`lib.rs`の不要public re-exportも消えていることを確認した。Close可判定。

### NEW-M-007 generatorが生成しない`LiarStatement::Implication`がcurrent contract全体に残る

**severity:** Medium
**状態:** Closed (2026-08-18 independent re-verification)
**該当:** `model.rs`, `themes/liar_puzzle.rs`, generated wire, Web formatter / adapter / fixtures

現行「うそつきだれだ」は6種類のnon-implication statementだけを生成するのに、`Implication` variantのsemantics・effort・wire・Web表示・validationを維持していた。current consumerのないpre-release residueであり、Rust/Web間の余分なsemantic policyも生んでいた。

**対応**

- `LiarStatement::Implication`をdomain enumから削除する。
- truth/effort/test branch、Web formatter/validator/fixtureを削除する。
- generated TypeScript wire typeをRustから再生成する。

**Close条件**

- production/generated contractに`implication` variantが存在しない。
- current 6 statement variantsのgeneration/format/boundary testが通る。

**2026-08-18 検証**

Rust production、generated wire、Web production/testから`Implication` / `implication`が消えていることを確認した。liar puzzleの6 statement variants testとWeb 164 testsが通過したためClose。

**2026-08-18 再々監査対応**

独立再監査で、残存6variantについてRustは`PeopleCount=3..=4`なのにWebが5人を許容し、`ExactLiarCount`の許容域もRust/Webで不一致であることを確認して再Openした。Rust側は`LiarCount`を1以上`MAX_LIAR_PUZZLE_PEOPLE`未満へ型で制約し、statement validationも`count < people_count`を要求する。Web boundaryはRust domain policyの数値再実装をやめ、current variantとwire field shapeのみ検証する。Rust invariant test・Web全test通過。独立再確認待ち。

**2026-08-18 独立再判定**

独立再判定で、liar puzzleのpeople/count invariantがRust側へ集約され、Webの独自range policyと`Implication` residueがないことを確認した。Close可判定。

### NEW-M-008 `TimeTen` operation countにdefault weight由来のmagic `+5`が埋め込まれている

**severity:** Medium
**状態:** Closed (2026-08-18 verification)
**該当:** `crates/drill-core/src/effort.rs`, `docs/architecture/effort-model.md`, `curriculum.md`

`TimeTen(n)`を`TimeTen`座標へ`n+5`として入れ、default weight 0.2との積で`1+0.2n`を作っていた。これはoperation countとweightの責務を混ぜ、weight override時に固定overheadまで連動して変化する。

**対応**

- vectorを`TimeTen × 1 + Count × n`へ分解する。
- `TimeTen` default weightを1.0、既存`Count` weight 0.2として従来の`1+0.2n`を再現する。
- weight変更時の固定/反復cost分離をtestで固定する。

**Close条件**

- operation vectorにdefault weightを逆算したmagic offsetがない。
- default scoreは従来式を保ち、weight overrideが意味論どおり独立する。

**2026-08-18 検証**

`TimeTen`の`n + 5` hackとdefault weight 0.2直結が消え、vectorが`TimeTen=1`, `Count=n`になる回帰test、およびweight override時の独立性testが通過したためClose。

### NEW-M-009 current README / curriculumに38 themes移行前・SolutionGraph移行前の記述が残る

**severity:** Medium
**状態:** Closed (2026-08-18 independent re-verification)
**該当:** `README.md`, `curriculum.md`

canonical architecture docsはcurrent codeと一致していたが、current READMEはactive themeを37個と記載し、`curriculum.md`は連立方程式effortをobsoleteな`SolutionGraph`構築として記述していた。`effort.rs`自身がcurriculumを現行仕様として参照するため、archiveではなくcurrent docs driftである。

**対応**

- READMEを38 active themesへ更新する。
- 連立方程式の記述をcurrent `OperationPlan` strategy比較へ更新する。

**Close条件**

- current docsから37-theme / SolutionGraph production architectureを推論できない。

**2026-08-18 検証**

current source/docs範囲に`37個のactive theme` / `SolutionGraph`記述が残っていないことを確認した。READMEは38 themes、curriculumは`OperationPlan`比較へ同期したためClose。

**2026-08-18 再々監査対応**

独立再監査でREADMEにcurrent schemaを`現行v5`とするstale記述が残っていることを確認して再Openした。Rust SoTの`SCHEMA_VERSION = 7`に合わせ`現行v7`へ同期し、README/curriculum/canonical architectureのcurrent範囲にv5・37 themes・SolutionGraph residueがないことを検索確認した。独立再確認待ち。


**2026-08-18 独立再判定**

独立再判定で、current schema v7 / 38 themes / OperationPlan記述がREADME・curriculum・current architectureと一致することを確認した。Close可判定。

### NEW-H-003 public raw `AnswerNode` semantic entrypointがAST structural limitを強制せずstack overflow可能

**severity:** High
**状態:** Closed (2026-08-18 independent re-verification)
**該当:** `crates/drill-core/src/answer.rs`, `normalize.rs`, `grade.rs`, `model.rs`

独立再監査で、crate外から任意深度の`AnswerNode`をsafe Rustだけで構築でき、`size` / `normalize_answer` / `grade_answer`等が無制限再帰へ入れることを確認した。WASM boundaryにはsize guardがあったがnative public semantic APIには同等のstructural guardがなかった。

**対応**

- `AnswerNode::size()`をexplicit stackによるiterative traversalへ変更する。
- semantic recursion専用に、display-size policyと分離したiterative `is_within_structural_node_limit()`を導入する。
- `normalize_answer` / `grade_answer` / `grade_answer_with_schema` / canonical-answer schema validationはrecursive semantic処理へ入る前にstructural budgetを検証する。
- `grade_answer_with_schema`は超過時`GradeError::AnswerAstSizeLimit`を返す。
- `exact_integer_magnitudes`もcrate-privateかつiterative traversalへ変更する。
- 100,000階層のraw external treeを使うnative回帰testを追加する。
- shallowだがdisplay sizeが大きい`i64::MIN`やextreme exact decimalは合法exact valueなので、structural recursion budgetとinteractive display-size limitを混同しない。

**Close条件**

- public semantic entrypointがunvalidated deep ASTをそのままrecursive traversalへ渡さない。
- `size()`自体がdeep raw ASTでrecursive stackを消費しない。
- existing exact-value normalization semanticsを壊さない。

Rust 115 core tests + 7 wasm tests、fmt/check/clippyを通過。独立再確認待ち。

**2026-08-18 独立再判定後の再修正**

独立再判定で、semantic entrypointはboundedになった一方、public recursive enumのderive `Clone` と通常Dropが100,000階層でstack overflowする反例が実測され、回帰test自身も`mem::forget(deep)`でDropを回避していたため再修正。`AnswerNode`のDrop/Clone/Eq/Ord/Debugをexplicit-stack実装へ変更し、Serializeはstructural limit超過をrecursion前にerror、Deserializeはdepth guard + structural node checkでbounded化した。回帰testは`mem::forget`を削除し、100,000階層についてClone/Eq/Ord/Debug/Serialize rejection/Deserialize rejection/通常Dropを実行する。

Rust/Web/generatedの全gate通過後も、状態は独立再確認待ちのままとする。

### NEW-M-010 generator helperのpreconditionが型ではなく`debug_assert` / caller conventionに依存する

**severity:** Medium
**状態:** Closed (2026-08-18 independent re-verification)
**該当:** `crates/drill-core/src/generator_support.rs`, current theme callers

`draw_decimal_coefficient(0)`、`draw_decimal_operand(..., 0)`、`draw_signed_integer(..., 0)`、empty sliceの`ensure_negative_term` / `draw_bounded_rational_arithmetic_ast`等が`next_bounded(0)`や`len()-1`へ到達できるhidden preconditionを持っていた。

**対応**

- failure可能なhelperを`Option`化し、zero/negative/empty boundを入口でrejectする。
- decimal digit power/range計算も`checked_pow` / checked arithmetic / checked conversionへ変更し、巨大boundでもpanicしない。
- theme callerは`?`または`Option<Vec<_>>`でcandidate rejectionへ伝播する。
- zero/negative/empty/`u32::MAX` boundの回帰testを追加する。

**Close条件**

- caller conventionや`debug_assert`をcorrectness条件に使わない。
- helperの型で受け取れる異常boundがRNG panic/underflow/overflowへ到達しない。

Rust全gate通過。独立再確認待ち。

**2026-08-18 独立再判定後の再修正**

独立再判定で、`generator_support.rs`の既知helperは直っていたが、`themes/column_arithmetic.rs::draw_integer_with_digits` / `draw_column_remainder`に同型の`debug_assert`・unchecked `pow`・zero-bound RNG preconditionが残っていたため再修正。両helperを`Option`化し、zero/invalid boundをreject、`checked_pow` / checked range conversionを使用し、callerは`?`でcandidate rejectionへ伝播する。0 / 1 / `i64::MIN` / `u32::MAX`の回帰testを追加。

Rust/Web/generatedの全gate通過後も、状態は独立再確認待ちのままとする。

### NEW-M-011 Web runtime validatorがRust domain policyを再実装している

**severity:** Medium
**状態:** Closed (2026-08-18 independent re-verification)
**該当:** `apps/web/src/domain/wasm-adapter.ts`

独立再監査で、liar people/count bounds、Mini Sudokuの`16` / `1..=4`、Arithmetic ExactDecimalの`scale <= 6`等、Rust domain側のsemantic policyをWeb runtime validatorが別実装していることを確認した。

**対応**

- Arithmetic ExactDecimalはwire shape (`coefficient` safe integer / `scale` u32)だけを検証し、Rustに存在しない`scale <= 6` ruleを削除する。
- liar statementはcurrent discriminantとfield shapeを検証し、people/count/person domain policyはRust SoTへ委ねる。
- Mini Sudokuのcell count/min/maxはhandwritten値ではなくgenerated theme input interfaceから取得する。
- RationalCoefficientもWeb独自のsemantic canonicality判定を持たずwire field shapeに限定する。

**Close条件**

- auditで確認されたRust semantic constants/policiesをWebへ手入力しない。
- theme-dependent grid boundsはgenerated contract由来である。

TypeScript・ESLint・Web 164 tests・generated freshness通過。独立再確認待ち。

**2026-08-18 独立再判定**

独立再判定で、Web validatorがwire shape validationへ縮小され、Mini Sudoku boundsはgenerated contract由来、decimal `scale <= 6` / liar range policyの再実装が消えていることを確認した。Close可判定。

### NEW-M-012 current consumerのないpublic Rust facadeが残っている

**severity:** Medium
**状態:** Closed (2026-08-18 independent re-verification)
**該当:** `crates/drill-core/src/lib.rs`, `generator.rs`, `effort.rs`, `answer.rs`

独立再監査で、`StepClock`、`calculate_plan_effort`、`big_num_operations`がcrate外current consumerなしでpublic exportされていた。

**対応**

- `StepClock`を`#[cfg(test)] pub(crate)`へ縮小する。
- `calculate_plan_effort`をtest-only crate-private helperへ縮小する。
- `big_num_operations`とそのsupport traversalをcrate-privateへ縮小する。
- `lib.rs`のpublic re-exportから削除する。

**Close条件**

- current external product consumerのない対象facadeがcrate public surfaceに存在しない。

repository-wide public-surface scanとRust clippy `-D warnings`を通過。独立再確認待ち。

**2026-08-18 独立再判定**

独立再判定で、`StepClock` / `calculate_plan_effort` / `big_num_operations`がcrate外public surfaceから消え、current internal/test consumerだけになっていることを確認した。Close可判定。


---

## 2026-08-18 fresh Rust core / WASM audit follow-up

既存IssueのClose表記や過去の監査結果を根拠にせず、commit `964df53 Refactor Rust core around validated domain boundaries` のcurrent codeを新規projectとして全面監査して追加したIssue。監査記録は [`audits/2026-08-18-fresh-rust-core-audit.md`](audits/2026-08-18-fresh-rust-core-audit.md) を参照する。

今回の実装担当は監査担当と同一であるため、以下は **修正済み / 独立再監査待ち** とする。自分自身の修正説明だけを根拠にClosedへはしない。

### AUDIT-H-001 public recursive `ArithmeticExpression` がsafe external consumerをstack overflowでabortできる

**severity:** High
**状態:** Closed (2026-08-18 independent re-audit)
**該当:** `crates/drill-core/src/model.rs`, `lib.rs`, `wire.rs`

#### 監査時の問題

`ArithmeticExpression`は`Box<ArithmeticExpression>`を持つpublic recursive enumで、通常のrecursive derive `Drop` / `Clone` / `Debug` / `Eq` / `Ord` / Serdeを持ち、crate rootからpublic re-exportされていた。外部一時crateからsafe Rustだけで100,000階層のtreeを構築し、`drop(expr)`だけでstack overflow → process abortを実測した。

current generatorが浅い式だけを生成することはpublic APIの安全性保証にはならない。

#### 修正

current external product consumerがraw prompt ASTを構築する必要はないため、`AnswerNode`と同等の巨大custom trait implementationをもう一組導入するのではなく、raw prompt syntaxをcrate内部実装へ戻した。

- `ArithmeticExpression`, `ArithmeticOperator`, `ProblemPrompt`, `RationalCoefficient`のcrate-root public re-exportを削除。
- `Problem::prompt()`を`pub(crate)`へ縮小。
- `ProblemWire`等のnested wire DTOをcrate-private化。
- `WorksheetWire`は`ts-rs` export rootとしてのみpublicに残し、fieldをprivate化してraw prompt construction surfaceにしない。
- これによりexternal consumerはarbitrary-depth `ArithmeticExpression`をsafe public APIから構築できない。

#### 検証

repository外のtemporary crateで`use drill_core::ArithmeticExpression;`をcompileし、現在は

`error[E0432]: unresolved import drill_core::ArithmeticExpression`

となることを確認。temporary crateは検証後削除した。

**独立再監査 (2026-08-18):** `wire-types`有効のrepository外consumer crateから`ArithmeticExpression` / `ProblemPrompt` / `RationalCoefficient` / `ProblemWire`へ到達できず、`Problem::prompt()`と`WorksheetWire::problems`もprivateであることを実compile probeで確認。別のpublic recursive typeである`AnswerNode`も100,000階層のsafe-Rust treeでsize/Clone/Eq/Debug/Serialize rejection/Dropを完走した。Closeする。

---

### AUDIT-M-001 Web runtime validatorがRust semantic policyを再実装し、current codeですでに判定driftしている

**severity:** Medium
**状態:** Closed (2026-08-19 autonomous QA re-audit)
**該当:** `apps/web/src/domain/wasm-adapter.ts`, `drill-engine.ts`

#### 監査時の問題

current Rust/Webで少なくとも次の不一致が存在した。

- Rustは`AnswerSchema::Decimal { max_scale: 0 }`をstructurally validとするがWebは拒否。
- Rustは`Rational { max_abs_numerator: 0, max_denominator > 0, ... }`を合法とするがWebは拒否。
- Webがgeneric digit-gridへRust domainにない`max_digit <= 9`条件を追加。
- Rust/WASMの`unsupported_schema_version`, `unknown_theme`, `unknown_generator_revision`, identity/input-interface系errorがWebでgeneric `invalid_dto`へ潰れる経路がある。

#### 修正

- Webの`max_scale == 0`独自拒否を削除。
- Webの`max_abs_numerator == 0`独自拒否を削除。
- generic digit-gridのWeb独自`max_digit <= 9`制約を削除。
- safe integer / u32 / JSON shape / discriminant等のwire integrity validationは維持。
- `unsupported_schema_version`, `unknown_theme`, `unknown_generator_revision`, `invalid_problem_set_identity`, `input_structure_not_allowed`, `input_interface_violation`をWeb `DrillEngineErrorKind`まで保持する。
- Rust-valid zero numerator boundをWebが拒否しないregressionと、semantic error-code preservation testを追加。

**独立再監査 (2026-08-18):** 既知の3 driftとerror-code collapseは解消したが、`assertInputInterface` / input capability traversal / prompt-schema-identity-layout-grade-result等のsemantic policyがWebに残り、特にseed alphabet/rangeがRustとWebで二重SoTだったためClose不可。

**追加修正 (2026-08-19):**

- `wasm-adapter.ts`をSerde/wire shape validationへ縮小し、seed alphabet、difficulty domain、input capability、prompt↔theme、schema semantic bounds、worksheet registry/layout/curriculum/problem-count、effort-model exclusivity、grade status/warning canonicalityの再判定を削除。
- enum/discriminant、primitive wire幅、current envelope schema、exact integer string、AnswerNode structural budget等、untrusted WASM DTOのwire integrity checkは維持。
- grade requestへ`input_interface`を渡し、`AnswerInputInterface::validate_answer()`をRust coreに追加してexpected/actualのinput capability semanticsをWASM側で検証する。
- grade/request schema semanticsはWebで先回りせずRustへ渡し、Rust/WASM error codeを保持する。
- Web testsも「semantic mismatchをmock responseで再判定しない」「unknown wire variantは拒否する」を分離した。

**2026-08-19 独立再監査:** semantic engine化はほぼ解消したが、`operation_plan.operations` が `kind: string` と `BigNum` だけしか検証せず、Rust `Operation` tagged unionの未知discriminant・variant固有payload・u32幅を通すためFAIL。これはdomain semanticsではなくSerde/wire integrityなのでClose不可。

**2026-08-19 再修正:** `assertOperation()`を追加し、current Rust `Operation`のknown `kind`、`Count.amount: u32`、`TimeTen.exponent: u32`、`OverheadDistribution.terms: u32`、`BigNum.magnitude: canonical u64 decimal string`をfail-closedに検証する。operation→vector整合やeffort意味論はWebへ再実装しない。malformed variant 6ケースとSerde-valid上限値のWeb regressionを追加。

**Close条件:** 独立再監査で、Web runtime validationがwire integrityに限定され、Rust-owned domain rule変更時に同じsemantic constant/ruleをTypeScriptへ手入力する必要がないことを確認する。

**2026-08-19 独立Close再判定:** Rust `Operation`全variantとWeb `assertOperation()`を照合し、unknown discriminant、variant payload、u32幅、canonical u64 decimal stringをwire integrityとしてfail-closedに検証しつつ、operation/vector/effort等のsemantic整合はRustに残っていることを確認。targeted Web test 40件もPASS。Closeする。

**2026-08-19 全面監査で再発:** `wasm-adapter.ts`の`assertAnswerNode()`がRust `AnswerNode::bounded_input_size()`とほぼ同じdisplay-size計算をTypeScriptへ再実装していた。structural node budgetはboundary wire防御として妥当だが、integer digit count、decimal scale、fraction/root/tuple parent cost等のdisplay semanticsはRust-owned ruleであり、Webへ複製しない。

**再修正:** WebのAnswerNode validatorからdisplay-size計算を完全に削除し、known discriminant、required payload shape、canonical i64 string、u32 width、binary operator、structural node budgetだけをfail-closedに検証する。mock WASM responseについて、structural node超過は拒否する一方、Rust側display-size policy相当のsemantic mismatchをWebが再判定しないregressionを追加した。

**2026-08-19 autonomous QA再判定:** current `wasm-adapter.ts`を改めて追跡し、AnswerNode / AnswerSchema / input interface / grade resultのWeb検証がknown wire tag、required property、canonical integer encoding、primitive width、structural node budget等のwire integrityへ限定されていることを確認した。display-size、normalization、capability validity、mathematical validity、grading semanticsはRust/WASM側に残る。`wasm-adapter.test.ts`のwire-vs-semantics regressionを含むfull VitestもPASS。Closeする。

---

### AUDIT-M-002 candidate selectorがdistinct-capacityを保証せずzero-bound RNG panicへ到達可能

**severity:** Medium
**状態:** Closed (2026-08-19 autonomous QA re-audit)
**該当:** `crates/drill-core/src/generator.rs`, `error.rs`

#### 監査時の問題

bootstrap poolはgeneric framework上`AllowDuplicates`を選択可能だが、final worksheet selectionはunique `ProblemKey`を要求する。旧実装はraw pool lengthだけを見ていたため、要素数は十分でもdistinct keyが不足するpoolを受け入れられた。

random difficultyではduplicateを捨てながら`swap_remove()`を続けるため、poolが空になっても必要件数へ到達しない場合に`rng.next_bounded(0)`へ到達しpanicし得た。

#### 修正

`SamplingError::InsufficientDistinctCandidates { required, available }`を追加。`select_candidates_from_pool()`はselection開始前にdistinct `ProblemKey`数を検証し、

- random: `count`
- ranked difficulty: `count + 2 * EFFORT_TRIM_PER_SIDE`

を満たさなければtyped sampling errorを返す。raw `layer_pool.len()`による偽のcapacity checkは削除した。

8 candidate / 1 distinct keyから2 unique問を要求する回帰testを追加し、panicではなく`InvalidSampling(InsufficientDistinctCandidates { required: 2, available: 1 })`になることを確認した。

**独立再監査 (2026-08-18):** selector内部のzero-bound panicは解消したが、non-constructive Random / AnswerConditioned / Layeredではfull poolのdistinct数が不足するとpool全体を捨てて再生成し、selectorのtyped errorへ到達せず最終的に`AttemptLimit`になる経路が残るためClose不可。

**追加修正 (2026-08-19):**

- full poolを作った後にlow-diversity poolを捨てて再生成するouter retryを削除。
- raw poolは有限target sizeまで構築し、その後global distinct capacityを一度検査し、不足なら即`InsufficientDistinctCandidates`を返す。
- layered poolもlayerごとのdistinct quotaを`ensure_layered_pool_capacity()`で検証しtyped errorへ統一。
- ConstructiveLayeredでもbootstrap段階でduplicateを延々排除せずraw poolを構築し、distinctnessは同じfinal selection/capacity contractへ集約。
- constant-key sourceの回帰testを`AttemptLimit`期待から`InsufficientDistinctCandidates`期待へ変更。

**2026-08-19 独立再監査:** distinct capacity自体はtyped errorへなったが、difficulty 1–3 selectorはselected keyのduplicateをpoolに残したまま`continue`していた。160 candidates / 40 distinct / 121 copies of one keyというcapacity十分なpoolで10,000 attemptsを使い切る反例がありFAIL。

**2026-08-19 再修正:** selectorが1 semantic keyを採用した時点で、そのkeyの残duplicateをselection populationから全て除去する。これにより各successful selectionごとに未選択distinct key数が単調減少し、事前capacityが十分ならranked/randomとも有限回で完了する。監査反例と同じ160/40/121 poolをregression化し、difficulty 1–4・複数seedで20 unique selectionとattempt数20/24を確認。`BootstrapDedup`は現在の責務に合わせ`SelectionDedup`へ改名。

**Close条件:** 独立再監査でRandom / AnswerConditioned / Layered / ConstructiveLayered、random/ranked difficulty、AllowDuplicates/Deduplicateの各経路でdistinct不足がAttemptLimitやzero-bound RNGへ化けずtyped sampling errorへ着地し、capacity十分なduplicate-rich poolではselection progressが保証されることを確認する。

**2026-08-19 独立Close再判定:** selector実装を全strategy/difficulty/dedup policyから再追跡し、採用keyの残duplicateがselection populationから除去されることを確認。160 raw / 40 distinct / 121 duplicateのpathological poolをlow/median/high effortへ集中させた独自probeでもdifficulty 1–4が有限progressし、selection-only AttemptLimitや`next_bounded(0)`を再現できなかった。Closeする。

**2026-08-19 全面監査で再発:** Random difficultyでは採用後に同じkeyの残duplicateを除去していたため最終worksheetはuniqueだったが、最初の抽選自体はraw bootstrap poolから行っていた。したがって同じsemantic candidateが偶然多く生成されるほど選択確率が高くなり、canonicalな「distinct candidate集合から一様非復元抽出」と不一致だった。

**再修正:** difficulty=Randomでは`SelectionDedup`設定にかかわらずselection前に`ProblemKey`単位でpopulationをcollapseし、そのdistinct populationから一様な`swap_remove`非復元抽出を行う。ranked difficultyの既存sampling semanticsは変更しない。同じ40 distinct candidate集合について、1 candidateだけを121重複させたpoolと重複なしpoolが同一Seedで同一Random selectionを返すregressionを追加した。

**2026-08-19 autonomous QA再判定:** Random pathは最初の抽選前に必ず`ProblemKey`でpopulationをcollapseし、そのdistinct集合から一様な`swap_remove`非復元抽出を行うcurrent codeを再確認した。40 distinct candidateに対し一候補だけ121重複させたpoolと重複なしpoolが同一Seedで同一selection sequenceを返すregressionもPASS。15,200 worksheet / 242,400 problemのlarge-sample probeでもduplicate key・generation failureは0。Closeする。

---

### AUDIT-L-001 `curriculum.md`が`print_recommended`をtaxonomy tagとして記述している

**severity:** Low
**状態:** Closed (2026-08-18 independent re-audit)
**該当:** `curriculum.md`

#### 監査時の問題

current Rustではbehavioral capabilityである`print_recommended`を`ThemePresentationPolicy`がcanonicalに所有し、taxonomy tagとの二重登録は既に削除されている。一方`curriculum.md`だけが「`print_recommended` taxonomy tagを持つ」と旧architectureを記述していた。

#### 修正

`curriculum.md`をcurrent codeに同期し、typed `ThemePresentationPolicy`から`print_recommended` capabilityを導出しtaxonomy tagへ二重登録しないことを明記した。

**独立再監査 (2026-08-18):** `ThemeTag`はclassification、`ThemePresentationPolicy::print_recommended()`はbehavior、Webはgenerated contractの`presentation.print_recommended`を使用するcurrent code/docs一致を確認。Closeする。

---

## 2026-08-18 independent re-auditで追加されたIssue

独立再監査記録は [`audits/2026-08-18-independent-rust-core-reaudit.md`](audits/2026-08-18-independent-rust-core-reaudit.md) を参照する。既存の`NEW-M-001`等とIDが衝突するため、このfollow-upでは`REAUDIT-*`をcanonical IDとする。修正担当と再監査担当は別であるが、以下の追加修正はこの文書更新と同じ担当が実装したため独立再確認まではClosedにしない。

### REAUDIT-M-001 crate root public API surfaceがcurrent product boundaryより広い

**severity:** Medium
**状態:** Closed (2026-08-19 independent re-audit)
**該当:** `crates/drill-core/src/lib.rs`, `effort.rs`, `theme.rs`, theme modules

独立監査ではeffort primitives、registry internals、`ThemeRegistration`/policy型、selective theme-specific constants/registrationsがcrate rootから大量にpublic re-exportされる一方、current external product consumerがないことを確認した。

**修正:** crate rootをgeneration / grading / parser / identity / boundary model / generated exporterに必要なAPIへ縮小。effort・registry・theme registration/policyのpublic facadeとselective theme constantsを削除し、公開を外した結果deadになった`calculate_effort` / `default_effort` / registration facade methods / stale layout aliasesも削除またはtest-only化した。

**2026-08-19 独立再監査:** root re-exportは縮小したが、`Problem::{effort_model, operation_plan, operation_vector, theme_specific_effort}`のpublic return surfaceからinternal effort型を型推論で操作でき、さらに`generate_problem(seed)` / `generate_worksheet(seed)`とrequest `Default`がtheme 1を暗黙選択するgeneric-looking facadeとして残っていたためFAIL。

**2026-08-19 再修正:** internal effort/solution accessorを`pub(crate)`へ縮小し、外部にはscalar `Problem::effort()`だけを残す。coreの`generate_problem(seed)` / `generate_worksheet(seed)` shortcutと`Generate*Request::Default` / serde theme defaultを削除し、requestは明示的`numeric_theme_id`を必須化。`Generate*Request::new(...)`はthemeを明示するgeneric constructorとする。repository外temporary crateでshortcut/internal accessor/importがcompile不能、explicit request APIとscalar effortはcompile可能であることを確認。

**Close条件:** repository外consumer視点でcurrent product boundaryに不要なinternal architectureがpublic semver surfaceへ漏れず、generic-looking APIが特定themeを暗黙選択しないこと。

**2026-08-19 独立Close再判定:** repository外temporary crateでinternal effort/registry/generic shortcut importと`Problem` internal accessorがcompile不能、explicit request generation・scalar `Problem::effort()`・grading・MathLive・identity/schema boundaryがcompile可能であることを確認。request defaultによるtheme暗黙選択も存在しない。Closeする。

### REAUDIT-L-001 `answer-ast.md`が削除済み`AnswerRepresentation`をcurrent architectureとして記述

**severity:** Low
**状態:** Closed (2026-08-19 independent re-audit)
**該当:** `docs/architecture/answer-ast.md`

**修正:** display/input `AnswerNode`はcallerが所有し、`normalize_answer(&AnswerNode)`が必要時に別canonical treeを返すcurrent ownershipへ記述を同期。削除済みaggregateをSoTとして説明しない。

**2026-08-19 独立再監査:** current docsがdisplay/input `AnswerNode` ownership、別canonical treeを返す`normalize_answer(&AnswerNode)`、generated `Problem`内部のprivate canonical wrapperを正しく説明しておりPASS。Closeする。

### REAUDIT-L-002 current-only implementationに`_v1` naming residueが残る

**severity:** Low
**状態:** Closed (2026-08-19 independent re-audit)
**該当:** `crates/drill-core/src/themes/decimals.rs`, `fractions.rs`

**修正:** `draw_problem_v1`→`draw_problem`、`operand_domain_v1`→`operand_domain`、`summary_operand_domain_v1`→`summary_operand_domain`へ改名。historic/current併存を示唆する不要なversion suffixを削除。

**Close条件:** current-only production implementationに対応先のない`_v1` residueが残らないこと。

**2026-08-19 独立再監査:** 指定3 symbolは消滅し、広い`_vN` / legacy / historic / compatibility / deprecated scanでもhistoric implementation residueは確認されずPASS。Closeする。

---

## 2026-08-19 second independent re-auditで追加されたLow

監査記録は [`audits/2026-08-19-independent-rust-core-reaudit.md`](audits/2026-08-19-independent-rust-core-reaudit.md) を参照する。以下は今回の修正担当自身が対応したため、独立再確認まではClosedにしない。

### REAUDIT2-L-001 `BootstrapDedup` namingがcurrent selection responsibilityと不一致

**severity:** Low
**状態:** Closed (2026-08-19 independent re-audit)
**該当:** `crates/drill-core/src/generator.rs`, theme sampling declarations

**修正:** 実際の責務がfinal selection populationの重複扱いであることに合わせ、`BootstrapDedup`を`SelectionDedup`へ改名し、strategy field/accessor/callerを同じ用語へ統一した。

### REAUDIT2-L-002 public `AnswerNode::Binary` とoperator typeのvisibilityが非対称

**severity:** Low
**状態:** Closed (2026-08-19 independent re-audit)
**該当:** `crates/drill-core/src/lib.rs`, `answer.rs`

**修正:** `AnswerNode`をpublic ASTとして維持する方針に合わせ、variant構築・pattern matchに必要な`AnswerBinaryOperator`をcrate rootからpublic exportした。

### REAUDIT2-L-003 cross-language layout DTOがtarget-sized `usize`

**severity:** Low
**状態:** Closed (2026-08-19 independent re-audit)
**該当:** `crates/drill-core/src/model.rs`, `theme.rs`, `contract.rs`, Web adapter

**修正:** worksheet/layout contractをfixed-width `u32`へ変更し、internal selectionで必要な`usize`はtheme layout accessor内部でのみ変換する。Web runtime validatorもlayoutを`u32` wire widthとして検証する。

**検証 (2026-08-19):** Rust fmt/check/clippy/full tests、generated contract/wire freshness、Web tsc/eslint/full testsを通過。Rust 118 + WASM 8、Web 172 tests。

**2026-08-19 独立Close再判定:** 3 Lowはいずれもcurrent codeで責務・visibility・wire widthが修正内容と一致することを確認しPASS。Closeする。


---

## 2026-08-19 third fresh Rust auditで追加されたIssue

前段のClose再判定では`AUDIT-M-001` / `AUDIT-M-002` / `REAUDIT-M-001` / `REAUDIT2-L-*`をcurrent codeから独立にPASSとした。一方、fresh auditで以下2件を新規発見した。今回の修正担当自身が対応しているため、独立再確認まではClosedにしない。

### REAUDIT3-H-001 public `AnswerInputInterface::validate_answer()`がdeep safe-Rust ASTでstack overflowする

**severity:** High
**状態:** Closed (2026-08-19 independent re-audit)
**該当:** `crates/drill-core/src/model.rs`, `lib.rs`

**問題:** public construction可能な100,000階層`AnswerNode`を`AnswerInputInterface::validate_answer()`へ渡すと、AST budget確認なしでrecursive `ensure_capability()`へ入りprocess stack overflowを再現できた。他のnormalize/grade/Serde等のpublic raw-answer boundaryにはbudget guardがあるため、このentrypointだけ防御が漏れていた。

**修正:** `validate_answer()`自身が`answer.is_within_size_limit()`をsemantic recursionより前に検査し、超過時は`EditorError::AnswerSizeLimit { max_size: MAX_ANSWER_AST_SIZE }`を返す。既存100,000階層public-entrypoint regressionへ`validate_answer()`を追加。

**2026-08-19 独立再監査:** repository外temporary crateからpublic APIだけで100,000階層の`Negative` treeを構築し、`validate_answer()`がpanic/abortせず`AnswerSizeLimit`を返すことを実行確認。recursive `ensure_capability()`より前にbudget guardがあるためPASS。Closeする。

### REAUDIT3-M-001 MathLiveで負の帯分数`-1\frac{1}{2}`を`(-1)×(1/2)`と誤解釈する

**severity:** Medium
**状態:** Closed (2026-08-19 independent re-audit)
**該当:** `crates/drill-core/src/mathlive_input.rs`, `docs/architecture/answer-ast.md`

**問題:** unary signがmixed-number juxtapositionより先に結合するため、`-1\frac{1}{2}`が`-1/2`へ正規化され、一次方程式(2)等で正しい負の帯分数入力を受理できなかった。

**修正:** mixed-number juxtapositionをleading `Negative` / `PlusMinus` prefixの内側へ結合し、`-1\frac{1}{2}`を`Negative(MixedFraction(1,1,2))`、`\pm1\frac{1}{2}`を`PlusMinus(MixedFraction(...))`としてparseする。実際の`LINEAR_EQUATION_2_REGISTRATION`からeditor/semantic input profileを取得するregressionを追加し、semantic capability受理と`-3/2`へのexact normalizationを検証する。architecture docsもeditor grammarとfinal semantic input contractの2層を明記した。

**2026-08-19 独立再監査:** actual registration由来のeditor/semantic interfaceで正・負・Unicode minus・plus-minus・parenthesized mixed fractionをprobeし、`-1\frac{1}{2}`が`Negative(MixedFraction(...))`かつexact `-3/2`へnormalizeされることを確認。precedence regressionもPASS。Closeする。


---

## 2026-08-19 fourth independent Rust audit follow-up

直前2件のClose再判定後のfresh auditで、wire/current-only facadeに以下を確認した。

### REAUDIT4-M-001 ThemeSpecific effortがwireでdummy `OperationPlan`へ戻る

**severity:** Medium
**状態:** Closed (2026-08-19 independent re-audit)
**該当:** `crates/drill-core/src/wire.rs`, generated Web wire types, `apps/web/src/domain/wasm-adapter.ts`

**問題:** domainでは`EffortModel::Operations(OperationPlan)`と`EffortModel::ThemeSpecific`が排他的なのに、当初は`ProblemWire::from`がThemeSpecific problemへ空`OperationPlanWire`を合成していた。

**第一修正:** `operation_plan`をoptional wire fieldへ変更し、ThemeSpecificを`null`にした。

**2026-08-19 独立再監査:** fake empty plan自体は解消済みだったが、`worked_solution` / `theme_specific_effort`の`skip_serializing_if`により実Serde JSONではproperty omission、ts-rs generated typeでは必須`T | null`となる不一致を発見したためFAIL / Close NO。

**再修正:** Webにproduction consumerのないeffort内部情報 (`operation_plan`, `operation_vector`, `theme_specific_effort`, `effort`) を`ProblemWire`から完全に削除し、`WebContract.operation_kind_count`とWebのOperation validator/type surfaceも削除した。`worked_solution`は省略せず常に`null | object`としてserializeし、generated `Problem.ts`と実Serde shapeを一致させた。ordinary / ThemeSpecific / worked-solution problemのproperty presenceをRust regressionで確認する。

**2026-08-19 独立再監査:** repository外temporary crateからordinary problem、ThemeSpecific effortの九九、worked-solution付き筆算を実生成・実serializeした。3系統すべてのProblem property集合は`schema_version / id / numeric_theme_id / prompt / input_interface / answer_schema / canonical_answer / worked_solution`の8項目で一致し、effort内部4fieldはproperty自体が存在しない。九九の`Problem::effort()`はtheme-specific scalarを維持し、difficulty selectionの全Rust regressionもPASS。external compile probeで`EffortModel` / `OperationPlan` / `OperationVector`はcrate外surfaceへ漏れていないことも確認した。Closeする。

### FULLAUDIT-M-001 generated TypeScript wire typeと実Serde shapeの不一致

**severity:** Medium
**状態:** Closed (2026-08-19 independent re-audit)
**該当:** `crates/drill-core/src/wire.rs`, `apps/web/src/generated/wire/Problem.ts`, `apps/web/src/domain/drill-engine.ts`

**問題:** `serde(skip_serializing_if = "Option::is_none")`で省略されるfieldをts-rsが必須`T | null`として生成し、generated typeが実wireより強い型になっていた。

**修正:** current wireでは`worked_solution`を常に明示的`null | object`としてserializeする。未使用effort diagnosticsはwire自体から削除し、`ProblemDto`のmanual optional補正も削除した。これによりgenerated Rust wire typeをそのままWeb側で信頼できるshapeへ戻した。

**2026-08-19 独立再監査:** generated freshnessだけに依存せず、ordinary / ThemeSpecific / worked-solution有りproblemを実Serializeしてgenerated `Problem.ts`とproperty presenceまで照合した。`worked_solution`は無しなら必ず`null`、有りならobjectで、omissionはない。generated `Problem.ts`も同じ8 required propertyと`WorkedSolution | null`を表し、manual optional補正は残っていない。generated contract/wire freshness、Rust fmt/check/clippy/test、TypeScript typecheck、ESLint、full VitestもPASSしたためCloseする。

### REAUDIT4-L-001 current-only facade residue

**severity:** Low
**状態:** Closed (2026-08-19 independent full audit)
**該当:** `apps/web/src/domain/drill-engine.ts`, `apps/web/src/domain/themes/theme-definition.ts`, `crates/drill-core/src/lib.rs`, `generator.rs`

**問題:** current schemaしか扱わない`drillOperationKindCountForSchema()`、test fixture中心の`ThemeDefinition.compatibility` wrapper、外部consumerのない`SystemClock` public re-exportがcurrent-only architectureに残っていた。

**修正:** `drillOperationKindCountForSchema()`と`ThemeDefinition.compatibility`を削除し、`SystemClock`をgenerator内部型へ縮小した。今回さらにWeb effort diagnostics自体を削除したため`DRILL_OPERATION_KIND_COUNT`もWeb surfaceから消えた。consumerのない`SystemClock::Default`も削除。

**2026-08-19 独立再監査:** external temporary crateのpositive/negative compile probeで、explicit generation / grading / MathLive / scalar effort / `MonotonicClock`は利用可能、internal registry/effort/generator/SystemClock surfaceはcompile不能であることを確認。Closeする。

### Follow-up simplification

同監査の「変更を勧める: YES」について、WASM `parse_mathlive_answer()`の重複interface precheck、WASM `grade_answer()`の重複answer-size pass、`parse_additive`の不要な`Option`+`expect`を削除した。後続全面監査で指摘されたgenerate endpointの重複schema precheckも削除し、`GenerateProblemRequest` / `GenerateWorksheetRequest`のschema validationはcore generation APIへ一元化した。WASM regressionでunsupported schema error codeが維持されることを確認する。また、Web wire consumerを失った`OperationKind` / `OperationVector` / `Operation`の`ts-rs` derive/annotation residueも削除し、effort内部型がgenerated-wire責務を持つように見える状態を解消した。boundary/coreのerror precedenceはWASM regressionで固定する。
