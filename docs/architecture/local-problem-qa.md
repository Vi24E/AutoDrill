# Local Problem QA Architecture

AutoDrillの問題品質をUser本人が観測し、将来の統計解析へ使うdatasetを作るlocal-only QA applicationの現行architecture。最上位原則は[`../principles.md`](../principles.md)を参照する。

## Deployment boundary

`apps/qa`はlocalhostにだけbindする独立Node workspace packageである。公開用`apps/web`、Next.js static export、GitHub Pages buildには含めない。rootのproduction `build` / `dev` scriptsは引き続き`@autodrill/web`だけを対象とする。

QA applicationのlocal serverはdurable SQLite persistenceを提供するためのもので、network service、cloud DB、login、analyticsを持たない。既定databaseはOSのapplication-data directoryへ置き、`AUTODRILL_QA_DB_PATH`で明示変更できる。

通常起動はrepository直下のmacOS `AutoDrill Problem QA.app`を使う。app bundleはQA runtime、Rust/WASM generator、versioned Rust contractを`Contents/Resources`へ同梱し、Finder起動時にDesktop配下のrepositoryへaccessしない。desktop launcherはOS割当のephemeral loopback portでserverを開始し、独立temporary profileのChromium app-mode windowを開く。Terminal、address bar、browser tabを表示せず、window processの終了をserver lifecycleへ結びつける。profileは終了時に削除し、durable dataはSQLiteだけへ残す。固定portの`src/server.mjs`起動は開発・debug用途である。

## Source of truth

raw SQLite recordsをcanonical dataとする。

- `qa_sessions`: evaluator、timezone、app/schema/Git version、開始・終了
- `items` / `item_revisions`: source identity、content hash、問題、単元、canonical answer、lossless original payload、変更履歴
- `attempts`: repeated exposureを独立observationとして保持するlifecycle snapshot、observation mode、answer、explicit outcome、correctness、timing、provenance
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

database constraintも現行scaleのvalid rangeを防衛する。将来scaleを変更するときはdefinition versionとmigrationを更新し、既存ratingの意味を変えない。

## Rating-only state graph

```text
queue -> rating (problem + answer shown) -> complete -> queue
rating -> explicit abandon
```

通常flowは`observation_mode=rating_only_answer_shown`で開始し、問題とcanonical answerを同時に表示する。回答欄、submit、採点はなく、Userはdifficulty × singularityだけを入力する。`raw_user_answer` / `normalized_user_answer` / `submitted_at`はnull、`correctness=ungraded`、`grading_method=not_collected_assumed_solved_v1`とし、「全問正解」を観測済みcorrectnessとして捏造しない。ratingは答え表示後なので`pre_answer_reveal=0`である。

旧`answer_then_rating` recordとinternal APIは既存datasetの再解析互換性のため残す。旧flowの回答・correctnessをmigrationで消したり新方式へ偽装したりしない。active attempt中はhistory、problem detail、exportをserver側でlockする。

## Timing and recovery

server UTC timestampとbrowser monotonic timeを併記する。attemptはshown、first interaction、answer start、submit、rating start/submit、reveal、completeのwall-clock timestampとelapsed millisecondsを持つ。visibility/focus eventをraw保存し、active-time補正は後から再計算する。

rating selection eventと確定処理はSQLiteへ保存する。reload、browser restart、server restart後はopen ratingをresumeできる。旧flowの未完了answer attemptは初回表示時に履歴を残したままrating-onlyへ移行する。継続しない場合はphysical deleteせず`abandoned`として完了させる。

## Schema evolution and exports

`src/migrations/`の連番SQLを`BEGIN IMMEDIATE` transactionで適用し、`schema_migrations`とSQLite `user_version`へ記録する。開発中もdatabase delete/recreateをmigrationの代用にしない。

Full JSON exportはmanifestと全raw/projection tableを含む。Analysis CSVはattemptごとのflat rowで、schema/export version、export timestamp、provenance、answer、outcome、timing、latest ratingを含む。どちらもactive blind rating中はlockする。

## AutoDrill integration

default flowはsession開始・problem登録・queue選択を自動化する。QA serverがRust `drill-core`の既存WASM boundaryを呼び、canonical web contractのうち`simple_numeric` inputと対応promptを持つthemeから一様に選択する。requested difficultyは`ランダム`、worksheet内problemも一様に選び、selection seed、candidate source、filter、propensityを保存する。theme IDのhard-codeや数学generatorの再実装は行わない。

各item snapshotはtheme / skill / curriculum metadata、generation request、worksheet identity、generator revision、seed、Problem DTO、prompt、answer schema、worked solution、layout、worksheet全体をlossless JSONとして保持する。通常flowは回答を収集・採点せず、canonical answerだけをrating前から表示する。過去分布はrating前に表示しない。

manual/import APIと既存recordはdata correction・compatibilityのため内部に保持するが、通常UIには表示しない。Bayesian modelは実装せず、将来追加しても`model_runs` / `derived_results`へ別projectionとして保存する。
