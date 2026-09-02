# Generator Architecture

問題generatorの実装・拡張時に守る共通方針。

## 1. Deterministic generation

同じproblem-set identityから同じworksheetを生成する。

identityは少なくとも次のversioned informationを持つ。

- schema version
- numeric theme ID
- generator revision
- Seed
- difficulty

product UIで「Seed」として共有する値は、この5 fieldを可逆に含む`ProblemSetIdentity`の文字列表現とする。request内の`seed` fieldはdeterministic RNGのentropy componentであり、単独ではworksheet identityではない。full IDのparse/validation/replayはRust `ProblemSetIdentity::from_str`と`generate_problem_set_from_id`だけが所有し、Webは文字列のfield分解を再実装しない。pre-releaseでは現行generator revisionだけを保持し、未知revisionのIDはfail closedする。

performance改善やmodule分割でdeterminismを壊さない。

## 2. Theme-local generator ownership

Theme固有生成ロジックを巨大な中央`generator.rs`へ蓄積しない。

Themeまたは密接なtheme familyのmoduleが所有するもの:

- operand / answer domain
- candidate construction
- archetype definition
- rejection constraints
- special sampling policy declaration
- generator-specific validation
- true theme-specific effort exception

共通generator frameworkが所有するもの:

- generator trait / interface
- deterministic RNG
- generic candidate sampling
- generic difficulty selection
- generic layered/archetype quota mechanism
- generic dedup mechanism
- attempt/work budget
- timeout watchdog
- common domain/math helpers

共通frameworkが`if theme_id == ...`を増やす構造は避ける。

## 3. Reusable mathematical helpers

次のような標準操作をthemeごとに再実装しない。

- integer arithmetic
- decimal place alignment / shift
- fraction GCD / LCM / reciprocal / cancellation
- factor search
- square-root simplification
- equation manipulation helpers

数学的helperは共通化し、各themeはdomainと組み合わせる。

## 4. Candidate population

### Normal case

必要な問題数`N`に対して、difficulty selectionに必要なbounded candidate poolを生成する。

候補数は原則として`N`の定数倍であり、domain全体の大きさへ不必要に比例させない。

### Finite / constructive domain

有限domainであっても、毎worksheetでdomain全体を`Problem`へmaterializeすることを原則としない。themeは数学的domainに適した一様なbootstrap sourceを明示する。

- cheapなindexable domainなら、Seedからdeterministicにunique indexをsampleする。
- operand pair等から構成できるdomainなら、base index/pairを一様に抽選し、domain条件に対するbounded rejection samplingで直接candidateを作る。valid candidateの全vectorを作る必要はない。
- layered themeは必要なら`draw_candidate_for_layer`を実装し、成立率の違うarchetypeを「同じ確率でattemptしたから均等」とみなさず、bootstrap quota自体を各layerから直接生成する。
- difficulty / diversity / layer quotaに必要な候補数だけ生成し、domain sizeが1,000→100,000へ増えてもworksheet generationが母集団全体へ線形比例しない構造にする。
- constructive samplerが重複を生成し得る場合、theme policyとしてbootstrap poolをprompt単位でdedupしてからdifficulty selectionへ渡せる。
- pre-releaseでは旧revisionをproduction codeへ保持しない。finite-domain samplingは現行実装だけを保守する。

詳細と検証結果は [GitHub Issues](https://github.com/Vi24E/AutoDrill/issues) のH-001を参照する。

## 5. Difficulty

基本思想は、問題の標準解法をprimitive operationへ分解したeffortを利用すること。

difficulty samplerはscalar effortだけで教材構成を破壊しない。

複数archetypeを含むthemeでは必要に応じてlayered samplingを使用する。`SamplingStrategy`はvalidated constructorからのみ構成し、answer domain/layer setを空にしたり、layer minimumの合計がworksheet problem countを超えたり、constructive bootstrap multiplierを0にした状態をsampling loopへ持ち込まない。classifierのraw indexは共通frameworkがbounded `LayerIndex`へ変換し、範囲外は`SamplingError`としてfail closedする。さらにanswer-conditioned callbackがrequested answerと異なるProblemを返した場合、constructive-layered callbackがrequested layerと異なるProblemを返した場合は、そのcandidateをretryして`AttemptLimit`へ化かさず即座にsampling-contract errorとする。single-problem generationとworksheet generationは同じ検証を通す。

基本形:

1. themeがarchetype/layerを宣言
2. generic samplerがquotaを満たす
3. 各layer内でdifficulty selection
4. `かんたん` / `ふつう`はeffort非減少順、`むずかしい` / `ランダム`はdeterministic shuffleで提示

Theme固有のquota値はtheme側が所有し、sampler自体へtheme ID特例を追加しない。

local QAの`custom sampling`はこの4段階difficultyへ第5の値を追加しない。production generatorは従来どおり`1..=4`だけを受理し、QA custom modeは通常の`difficulty=4` worksheetを複数candidate sourceとして利用する外側のdevelopment sampling layerである。したがって同一`ProblemSetIdentity`から生成されるWorksheetの意味・determinismはcustom modeによって変化しない。

## 6. Deduplication

「重複」の意味はtheme固有の数学的意味に沿って明示する。

- ordered operandsを別問題と扱うか
- commutative canonicalizationするか
- visually same / mathematically sameのどちらを除外するか

これをnumeric theme IDで中央から推測しない。theme policyとして宣言する。semantic identityは専用`ProblemKey`としてCandidate生成時に一度だけ構築する。heap-owning AST/statement keyをdedup用HashSetへ入れるたびにdeep cloneせず、Candidateと一時setは`Rc<ProblemKey>`で同じkey allocationを共有する。sort comparatorもこのprecomputed keyだけを参照する。

## 7. Correctness by construction

可能な限り不正候補を後段で大量rejectするより、domain制約・answer-first generation等で正しい問題を構成する。

それでもgeneratorから独立したvalidationを持つ。

検証例:

- canonical answerとpromptが一致する
- denominator != 0
- curriculum外valueを含まない
- expected answer schemaと一致する
- layout/problem count contractと一致する
- finite effort
- required uniqueness

## 8. Curriculum safety

「小学生themeだから負数禁止」等のpolicyを日本語pathやrouteから推測しない。

canonical typed theme metadataに基づきvalidationする。

詳細は[`theme-system.md`](theme-system.md)を参照する。

## 9. Work budget vs wall-clock timeout

この2つを別概念として扱う。

### Deterministic work budget

- max attempts
- max candidate count
- bounded recursion/depth
- bounded domain sampling

algorithmic runawayをdeterministicに止める。

### Wall-clock watchdog

ブラウザfreeze等に対する最後のguard。

browser schedulerやhost loadを含むため、algorithmic complexityの代用にしない。

「通常100ms以内」を目標とする場合でも、100ms超過を即異常と断定するのではなく、処理量自体がboundedかを先に保証する。

## 10. Tests

最低限:

- same identity => byte/semantic equivalent worksheet
- multiple Seed × difficulty generation
- canonical answer verification
- duplicate policy
- curriculum/safety constraints
- archetype quota
- effort monotonic/distribution checks where meaningful
- performance regression for large finite domains
- browser/WASM generation smoke test

不具合Seedはregression fixtureとして残す。
