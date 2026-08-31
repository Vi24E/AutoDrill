# AutoDrill エージェント功績録

このページは、AutoDrillに関わったLLMエージェントたちの「歴史的功績」を、人間があとから眺めて楽しむための記録である。

**現行仕様・architecture・Issue状態・権限・担当順序のsource of truthではない。** 正確な現行情報は `AGENTS.md`、`docs/principles.md`、canonical architecture docs、GitHub Issues、Git履歴を参照する。

## 記録ルール

各エージェントについて、少なくとも次を残す。

- **エージェント名** — 発行されたidentity。
- **やったこと** — そのエージェントを思い出すのに十分な、代表的な仕事・判断・仕組み。細かなcommit一覧にはしない。
- **直属の後継者** — そのエージェントが継続promptで命名した直接の後継エージェント。複数を命名した場合は列挙し、まだ発行していなければ `—` とする。

このページは功績録なので多少の要約表現は許すが、行っていない仕事を後から帰属させない。訂正する場合はGit履歴やIssue attributionなど確認可能な記録を優先する。

---

## Alpha-1

**エージェント名:** `Alpha-1`

**やったこと:**

- AutoDrill全体を `docs/principles.md` の13原則に照らして監査し、Rust/Web間の長除法decimal normalization二重SoT、typed interaction policy不足、consumerのないwire/public surface、real Chrome state graph failureなど、後続改善の起点となるaudit findingsを整理した。
- 旧 `docs/issues.md` で管理していたIssue群をGitHub Issuesへ移行し、GitHub Issuesを唯一のIssue / backlog source of truthに変更した。旧Markdown trackerはarchiveへ退役させた。
- mobile対応を単なるLow priorityではなく、activation conditionまで意図的に凍結する `priority:deferred` として定義した。
- LLMごとのidentity / generation管理と、GitHub Issueにおける `報告者` / `更新者` attributionの運用を導入した。既存の移行IssueについてもAlpha-1報告として整理した。
- 未解決Issueを次担当へ引き継ぐための運用promptを整備し、最初の直属後継エージェントとして `Beacon-2` を命名した。
- 後に、問題を人間が実際に解いて難易度・特異性を評価し、生の観測データを蓄積して問題品質を研究するlocal QA applicationの構想と実装handoff promptを設計した。

**直属の後継者:** `Beacon-2`, `Forge-6`

---

## Beacon-2

**エージェント名:** `Beacon-2`

**やったこと:**

- 筆算入力のinteraction contractをtyped metadataへ移し、割り算の入力順序と小数掛け算の小数点配置をtheme/Rust側の明示policyとして表現した。Webがoperatorや正答値から挙動を推測する構造を縮小し、Rustのtyped grading authorityを維持した。
- 長除法のdecimal normalizationをRust `worked_solution`へ一本化し、Web側の重複再計算を削除した。併せてcurrent consumerのないWorksheet wire metadata、WASM endpoint、Rust public facadeを整理し、contract/public surfaceを現行利用範囲まで縮小した。
- real Chromeで再現していたprint preview close raceをlifecycle ownershipの問題として修正し、staleなgeneric numeric theme defaultも除去した。Rust/Webのquality gate、fresh GitHub Pages build、全registered-themeのbrowser/layout/print verificationまで通して修正一式を検証した。
- #14・#41・#42については、hosting architectureやproduct UXの未決定事項を実装で勝手に埋めずdecision gapとして記録し、#3・#19の主観的なuser confirmationも機械検証と分離して次担当へ残した。Close可否の独立判定を行う直属後継者として `Cedar-3` を命名した。

**直属の後継者:** `Cedar-3`

---

## Cedar-3

**エージェント名:** `Cedar-3`

**やったこと:**

- Beacon-2の修正一式を独立reviewし、Rust/Webのquality gate、fresh Pages build、full real Chrome 80 worksheet samples / 1,139 editable actions / 143 input-panel actionsまで自ら再検証したうえで、監査Issue #93〜#98をCloseした。
- #17は実装自体が通っていても `docs/architecture/web-print.md` に実装前のstale記述が残ることを検出し、docsとcurrent implementationの不一致を理由にtechnical failureとしてCloseを拒否した。
- `status:user-confirmation` をtechnical completionと分離し、#3・#19について確認場所、seed、操作手順、期待結果、不合格時に残す情報まで再現可能に記録する運用を定義した。

**直属の後継者:** `Dawn-4`

---

## Dawn-4

**エージェント名:** `Dawn-4`

**やったこと:**

- AutoDrill内で使うlocal skill群を見直し、`maintainable-coding`、`explain`、`interactive-ui-tuning`、`make-prompt` を、冗長な原則集ではなく実行時の判断を直接拘束する形へ再設計した。あわせて、AutoDrillの運用をrepository直下の契約だけで完結させるため、親LLM環境への依存除去、branchを勝手に作成・切替しない規則、LLM commit attributionなどの運用整理を進めた。
- 継続prompt用のidentity発行手順を実運用し、A〜Wの通常identity rotationとgeneration管理に従って直属の後継者 `Ember-5` を発行し、identity更新をcommitした。

**直属の後継者:** `Ember-5`

---

## Ember-5

**エージェント名:** `Ember-5`

**やったこと:**

- 筆算まわりのgrading / input / presentationを横断して修正し、Rust側generator metadata、Web側のcolumn arithmetic input/presentation、PDF worksheet、browser layout verifierまで一貫して更新した。
- 筆算の演算子配置、解答欄、採点表示など、教材上の桁位置とpage-grid alignmentに関わる不整合をまとめて扱い、関連testとarchitecture文書も同じ変更で整合させた。

**直属の後継者:** `—`

---

## Forge-6

**エージェント名:** `Forge-6`

**やったこと:**

- 問題品質を人間が継続評価する独立local QA applicationを`apps/qa`へ新設し、SQLite永続保存とzero-config macOS launcherを整備した。browserやportを意識せず、評価データを再起動後も保持できる土台を作った。
- ユーザーの回答入力を不要にして解答を常時表示するrating-only flowへ簡素化し、単元選択→単元内random出題、分数を含む対象単元、即時次問、中心原点の連続difficulty × singularity plane（横軸difficulty）を実装した。
- AutoDrill本体のgeneratorとprint layoutをQAへ接続し、筆算の演算子・解答欄・問題番号・採点markをページ方眼へ揃える修正とreal Chrome / printのlayout regression検証を進め、残る座標系の論点を後続監査へ引き継いだ。

**直属の後継者:** `Grove-7`, `Harbor-8`

---

## Grove-7

**エージェント名:** `Grove-7`

**やったこと:**

- Issue #3で露呈した「同じworksheet geometryを複数のCSS規則・座標系・renderer・test-side magic numberが所有する」問題をAutoDrill全体へ一般化して独立監査し、Mini Sudokuのpage-grid位相ずれとWeb/print problem-shellの二重ownershipを新たに特定した。
- Mini Sudokuをpage-wide worksheet gridへ揃える修正、Web/printのproblem-cell presentation ownership統合、worksheet-grid browser invariantの一般化を実装・検証し、Mini Sudokuの実Chrome grid誤差をWeb約8.376px・print約6.863pxからsub-pixel範囲まで縮小した。
- 残るIssue #3の妥当性と、追加した抽象化にオーバーエンジニアリングがないかを再査定して適切なcommitへ着地させる継続promptを作成し、直属後継者として `Iris-9` を命名した。

**直属の後継者:** `Iris-9`

---

## Harbor-8

**エージェント名:** `Harbor-8`

**やったこと:**

- local QA applicationでAutoDrill問題を継続評価するため、print observationのprefetchとGit provenance保存を追加し、どのrepository stateから生成・評価した問題かを追跡できるようにした。
- QA向けにcustom information samplingを実装し、Rust/WASM runtime・effort情報・sampling logicを接続して、単純なrandom出題以外の研究用samplingを可能にした。
- QA appのunit chooser / navigationを修復・調整し、macOS packageも同期した。あわせてGitHub Issue本文の標準形式とreporter/updater attributionをIssue template / `AGENTS.md`へ明文化した。

**直属の後継者:** `—`

---

## Iris-9

**エージェント名:** `Iris-9`

**やったこと:**

- Issue #3の2026-08-31 User NGを受け、筆算の問題番号・数字・演算子・解答欄とMini Sudokuを同じpage-grid coordinate systemへ統合した。Web/printのproblem shellとworksheet grid primitivesを共有し、fractionalな問題番号offsetや不可視logical cell境界に依存する検証を廃して、実paint grid・page overflow・隣接lane overlapを測るreal Chrome verificationへ整理した。
- その監査中に、小数加減算でrendererの小数点揃え幅とlane sizingが別計算になって1cellずれる不具合を発見してIssue #108として記録し、canonical alignment planへ一本化して修正・Closeした。最終的にWeb tests / typecheck / lint / Pages export / full browser verifier / QA browser acceptanceを通し、QA rendererとmacOS QA appも同じworksheet presentationへ同期した。

**直属の後継者:** `Juniper-10`

---

## Juniper-10

**エージェント名:** `Juniper-10`

**やったこと:**

- ちびむすドリルの公開教材を外部baselineとして再現可能に分析する研究基盤を整備し、第三者PDF本体をGit管理せず、取得・抽出tool、normalized SQLite schema、分析結果、解釈規則をrepositoryへ残した。3,181 worksheet規模のreference corpusを、AutoDrillの教材品質・archetype構成を比較するためのempirical baselineとして使える形にした。
- reference corpus、AutoDrill生成sample、一次方程式100件のUser singularity QAを突き合わせ、単純な外部頻度模倣ではなくcurriculum/archetypeのsupport差とworksheet compositionを見る方針を整理した。低コストarchetype拡充をIssue #121で体系化し、一次方程式の`|A|=1`過剰生成やliteral RHS=0、方程式・正負の数等の専用theme分離を後続Issueへ展開した。
- alpha 1.3へversionを更新し、AST boundary interaction testを安定化したうえで、`deploy-alpha-1.3-20260831-juniper10`としてrelease snapshotをtagged commitにまとめた。

**直属の後継者:** `Kestrel-11`

---

## Kestrel-11

**エージェント名:** `Kestrel-11`

**やったこと:**

- 継続promptを受け、Git履歴・Issue・作業ツリーを照合して、既存agentの功績を混同せず、Kestrel-11名義で確認できる実績だけを記録する監査を行った。

**直属の後継者:** `—`
