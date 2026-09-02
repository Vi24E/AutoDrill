# Local Problem QA Architecture

AutoDrillの問題品質をUser本人が観測し、将来の統計解析へ使うdatasetを作るlocal-only QA applicationの現行architecture。最上位原則は[`../principles.md`](../principles.md)を参照する。

## Deployment boundary

`apps/qa`はlocalhostにだけbindする独立Node workspace packageである。公開用`apps/web`、Next.js static export、GitHub Pages buildには含めない。rootのproduction `build` / `dev` scriptsは引き続き`@autodrill/web`だけを対象とする。

QA applicationのlocal serverはdurable SQLite persistenceを提供するためのもので、network service、cloud DB、login、analyticsを持たない。既定databaseはOSのapplication-data directoryへ置き、`AUTODRILL_QA_DB_PATH`で明示変更できる。

通常起動はrepository直下のmacOS `AutoDrill Problem QA.app`を使う。app bundleはQA runtime、Rust/WASM generator、versioned Rust contractを`Contents/Resources`へ同梱し、Finder起動時にDesktop配下のrepositoryへaccessしない。desktop launcherはOS割当のephemeral loopback portでserverを開始し、独立temporary profileのChromium app-mode windowを開く。Terminal、address bar、browser tabを表示せず、window processの終了をserver lifecycleへ結びつける。profileは終了時に削除し、durable dataはSQLiteだけへ残す。固定portの`src/server.mjs`起動は開発・debug用途である。

## Source of truth

raw SQLite recordsをcanonical dataとする。

- `qa_sessions`: evaluator、timezone、app/schema/Git versionとGit worktree state、開始・終了
- `items` / `item_revisions`: source identity、content hash、問題、単元、canonical answer、lossless original payload、変更履歴
- `attempts`: repeated exposureを独立observationとして保持するlifecycle snapshot、observation mode、answer、explicit outcome、correctness、timing、Git SHAとclean/dirtyを含むprovenance
- `selection_events`: candidate集合、selection policy、filter、random seed。将来adaptive selectionを使う場合のmodel/probability fieldも持つ
- `input_events`: answer change、focus/visibility、submit、rating、reveal等の意味のあるevent chronology
- `evaluations`: difficulty × singularityの全revision。旧ratingをoverwriteしない
- `change_audit`: item等の分析影響fieldのbefore/after
- `model_runs` / `derived_results`: raw dataから再生成可能なversioned projection

同一content hashはduplicate候補として可視化できるが、itemを自動削除しない。同じitemの2回目以降も`exposure_count`を持つ別attemptとして必ず保存する。

## Rating contract

rating scaleの唯一の定義元は`apps/qa/src/constants.mjs::RATING_SCALE`である。現行scale version 1は両axis 1〜7。

- Difficulty: 1 非常に易しい / 4 中程度 / 7 非常に難しい
- Singularity: 1 非常に典型的 / 4 やや特徴的 / 7 非常に珍しい・特異

UIはcolor picker型の2D selection planeとして表示し、横軸をDifficulty（左が易しい、右が難しい）、縦軸をSingularity（下が典型的、上が珍しい）とする。cursorはcell centerへsnapせず連続的に移動する。両軸の正確なraw positionを0〜1の`difficulty_position` / `singularity_position`として保存し、scale midpointを表示上の原点 `(0, 0)`、両軸を`-3〜+3`として示す。1〜7のordinal ratingは既存分析との互換用にcontinuous positionから導出する。database constraintも両表現のvalid rangeと対応を防衛する。将来scaleを変更するときはdefinition versionとmigrationを更新し、既存ratingの意味を変えない。

## Rating-only state graph

```text
unit choice -> rating (problem + answer shown) -> complete -> same-unit next rating
rating -> explicit abandon
rating -> explicit return -> unit choice
rating -> explicit abandon -> new unit rating
```

通常flowはUserがcanonical skill identityで単元を選んでから`observation_mode=rating_only_answer_shown`で開始し、問題とcanonical answerを同時に表示する。回答欄、submit、採点はなく、Userはdifficulty × singularityだけを入力する。評価確定後は完了画面や保存通知を挟まず同じ単元の次問題へ進む。単元変更時は現在のattemptを`abandoned`として残してから切り替える。評価画面から単元選択へ明示的に戻る場合も、表示済みproblemをphysical deleteせず`abandoned`として残してからchooserへ戻る。`raw_user_answer` / `normalized_user_answer` / `submitted_at`はnull、`correctness=ungraded`、`grading_method=not_collected_assumed_solved_v1`とし、「全問正解」を観測済みcorrectnessとして捏造しない。ratingは答え表示後なので`pre_answer_reveal=0`である。

単元選択肢の件数は、canonical skill identityごとの「invalidatedされていない完了attemptかつevaluationが1件以上あるobservation数」である。rating revision数やabandoned attempt数を水増しに使わない。確定直後はclient表示を増分し、reload時はSQLite集計を再取得する。

旧`answer_then_rating` recordとinternal APIは既存datasetの再解析互換性のため残す。旧flowの回答・correctnessをmigrationで消したり新方式へ偽装したりしない。active attempt中はhistory、problem detail、exportをserver側でlockする。履歴へ移動するときは、表示中の問題を確認付きで`abandoned`として保存してからlockを解除し、rating前に過去分布を見せない。

## Timing and recovery

server UTC timestampとbrowser monotonic timeを併記する。attemptはshown、first interaction、answer start、submit、rating start/submit、reveal、completeのwall-clock timestampとelapsed millisecondsを持つ。visibility/focus eventをraw保存し、active-time補正は後から再計算する。

rating selection eventと確定処理はSQLiteへ保存する。reload、browser restart、server restart後はopen ratingをresumeできる。旧flowの未完了answer attemptは初回表示時に履歴を残したままrating-onlyへ移行する。継続しない場合はphysical deleteせず`abandoned`として完了させる。

## Schema evolution and exports

`src/migrations/`の連番SQLを`BEGIN IMMEDIATE` transactionで適用し、`schema_migrations`とSQLite `user_version`へ記録する。開発中もdatabase delete/recreateをmigrationの代用にしない。

Full JSON exportはmanifestと全raw/projection tableを含む。Analysis CSVはattemptごとのflat rowで、schema/export version、export timestamp、provenance、answer、outcome、timing、latest ratingを含む。どちらもactive blind rating中はlockする。

## AutoDrill integration

default flowはsession開始・problem登録・queue操作を自動化する。QA serverがRust `drill-core`の既存WASM boundaryを呼び、canonical generated contractのcurrent theme集合からQA対象を組み立てる。QA固有の除外は`curriculum_unit.key`の明示集合で所有し、一桁の足し算・引き算、九九、九九型の割り算に属するthemeを除外する。theme数やnumeric ID範囲をQA側へ固定せず、新しいsibling themeも同じcurriculum unitなら自動的に同じ除外policyへ従う。残るthemeには分数、一次・二次・連立方程式、論理問題、4×4数独などを含む。numeric theme IDや表示名から単元の性質を推測せず、数学generatorを再実装しない。

単元選択後はRust/WASM generatorでworksheet全体を一度に生成し、そのproblem indexをseed付きでshuffleしてmemoryへprefetchする。同じ単元の次問題はそのbatchから重複なしで取り出し、使い切ったときだけ新しいworksheetを生成する。各selection eventにはselected skill、worksheet seed、problem index、残候補数、selection policy、candidate source、filter、propensityを保存し、batch samplingの事実を後から再構成できるようにする。process restartでmemory batchが失われても、保存済みattemptとselection eventは失われない。

QA appには開発用の`custom sampling` toggleを持つ。OFFでは上記のrandom batch samplingをそのまま使う。ONではproductionのdifficulty semanticsを変更せず、複数の`difficulty=4` worksheetをQA候補poolとして生成し、Rust/WASMのQA専用diagnostic endpointから得たcurrent operation vectorを使って、既存観測で情報量が薄いvector方向を優先する。現行profile `operation_vector_information` v1はdifficulty予測値そのものではなく、interceptと標準化operation vectorからなるfeatureに対するridge leverage `xᵀ(XᵀX+λI)⁻¹x`で観測の薄い方向を評価するsampling utilityである。rating値はcandidate選択へ直接使わないため、difficulty / singularityのどちらを後から弱教師として回帰しても同じ実験計画を利用できる。既評価promptは候補から可能な限り除外する。theme固有scalar effortしか持たない問題ではscalar effortを1次元featureとしてfallbackする。

custom samplingは通常Worksheet wireへ`effort`や`operation_vector`を追加しない。QAという具体的consumerだけが専用endpointでscalar effort / effort model kind / current operation vector basisを取得する。selection eventにはcustom profile/version、candidate worksheet requests、candidate scoresを保存し、単純なclosed-form propensityを持たないdeterministic top-score選択では`selection_probability=null`とする。選ばれたitem snapshotにもQA sampling profile、選択時effort、operation-vector basis、vectorを保存する。basis変更後は古いsnapshot vectorを現在basisとして流用せず、現在Rustで同じpromptを再生成できる場合だけ現在basisへ再診断する。これらはraw ratingの代替ではなく、後からsampling biasを監査・再解析するためのprovenanceである。

表示中attemptのrating時間を使って次problemをserver memoryへ予約し、別のsame-origin iframeで印刷DOMと数式fontの描画まで完了させる。rating確定後に予約を正式なitem/selection/attemptとしてtransaction保存し、iframeをDOM移動・再読込せず表示位置だけvisible shellへ切り替える。予約だけでUserへ表示されなかったproblemはobservationやselection eventとして保存せず、選択bias上の「提示」と混同しない。予約IDは単元一致・単回consumeを検証し、使用後も既に描画済みframeが参照できる間だけbounded memoryに保持する。

問題表示は別実装のplain-text UIを使わず、`apps/web/src/pdf/worksheet-pdf.tsx`の`WorksheetPrintDocument`をViteでQA専用bundleにする。生成された解答ページ上の対象problem cellをiframe内で正確に切り抜き、canonical answerを含む実際の印刷/PDF DOMをrating前から表示する。これによりQA中に数式・筆算・方程式・数独等の印刷layout regressionも発見できる。QA rendererはproduction componentをsource参照するinternal consumerであり、production deploymentやwire contractを変更しない。

各item snapshotはtheme / skill / curriculum metadata、generation request、worksheet identity、generator revision、seed、Problem DTO、prompt、answer schema、worked solution、layout、worksheet全体をlossless JSONとして保持する。通常flowは回答を収集・採点せず、canonical answerだけをrating前から表示する。過去分布はrating前に表示しない。

provenanceはcommit SHAだけでなく、processが使ったsourceの`worktree_state`（clean / dirty / unknown）、porcelain status、status SHA-256、tracked diff SHA-256をsessionとattemptへJSON保存する。新規AutoDrill itemのoriginal payloadにも生成時stateをsnapshotする。macOS bundleはbuild時stateを`git-state.json`へ固定し、実行時に外側repositoryの後続変更を誤って観測しない。旧recordのprovenance訂正はchange auditへbefore/afterとactorを残す。

manual/import APIと既存recordはdata correction・compatibilityのため内部に保持するが、通常UIには表示しない。Bayesian modelは実装せず、将来追加しても`model_runs` / `derived_results`へ別projectionとして保存する。
