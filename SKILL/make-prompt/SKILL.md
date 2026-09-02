# Make Prompt

AutoDrillの作業を新しいChatGPTへ引き継ぐための専用skill。

目的は会話の要約ではない。次のChatGPTが過去chatを読まなくても、正しいtool、session、repository、identity、Git状態、Issue状態、ユーザー指示、検証状態へ到達し、そのまま作業を再開できる実行用promptを作る。

## 最上位原則

**次のChatGPTを正しい環境へ到達させるためのbootstrap命令は、重複していても省略しない。**

AutoDrillでは、handoff promptの冒頭に必ず次を含める。

- `@local-mcp` という文字列を明記し、最初にlocal-mcpを使わせる
- 実際に確認したexact `session_id`
- `CURRENT_IDENTITY: <next identity>` を明記し、そのsession中は不変であること、他identityはすべて第三者であることを明記する
- repositoryの絶対パス
  `/Users/koharayu/Desktop/LLM/projects/AutoDrill`
- 従うのはrepository直下の `AGENTS.md` **のみ**
- `LLM/AGENTS.md`、`../../AGENTS.md`、その他の親 `AGENTS.md` は読まず、適用しない
- subagent、role-routing、delegation protocolなどの委譲機構を使わない
- `docs/principles.md` を必ず読み、AutoDrillの最上位の設計原則として扱う
- 必要なSKILLはChatGPT内蔵機能ではなく、repository直下の `SKILL/<name>/SKILL.md` を直接読む
- **Work modeへの切替を提案しない。** AutoDrillでは現在のconversationと指定されたlocal-mcp等のtoolで継続し、この制約をhandoff promptの拘束的なユーザー指示として必ず引き継ぐ
- ユーザーの明示指示なしにbranchを作成せず、既存branchへも切り替えず、現在branchを維持する

これらはrepositoryを読んだ後で再取得できる一般説明ではなく、repositoryと正しい規則へ到達するための起動条件なので削らない。

## 発動条件

ユーザーが、新しいChatGPT / LLM / 担当者へ実際に渡す継続promptを求めたときに使う。

単なるprompt添削や、まだ引き継がないpreviewではidentityを発行しない。previewでは `AGENTS.md` を更新せず、generationも消費しない。

## 1. 最初に実環境を確認する

handoff作成の最初のtool actionとしてlocal-mcpを使う。最初のuser-visible updateにも `@local-mcp` と使用するexact `session_id` を明記する。

`session_id` はSKILL内の固定値を使わず、その時点のsession情報から実値を確認する。推測しない。

repositoryの絶対パスが実在し、対象がAutoDrillであることを確認する。

repository直下の `AGENTS.md` と `docs/principles.md` を読む。親 `AGENTS.md`、ROLES、中央Boardなどへ遡らない。

現在のtaskに必要なSKILLを特定し、repository直下から直接読む。無関係なSKILLを機械的に全部読まない。

## 2. identityを発行する

identityの発行規則は `AGENTS.md` を正とする。

LLMが発行してよい通常identityの頭文字は **A〜W** だけで、Wの次はAへ戻る。X / Y / Zは特殊用途の予約文字であり、LLMは発行・管理しない。

新しいidentityを実際に発行する直前に、`AGENTS.md` の次を読む。

- `latest issued identity`
- `generation max`

現在担当者自身のgenerationから次generationを推測してはいけない。次generationは必ず `generation max + 1` とする。次の頭文字は `latest issued identity` の通常rotationから決める。

条件を満たす、人間が区別しやすい一般名を選び、`<name>-<generation>` とする。

発行したら、同じ作業で `AGENTS.md` の `latest issued identity` と `generation max` を新identityへ更新する。更新後に再読し、handoff promptのidentityと一致することを確認する。

### current identityはsession中不変

現在のagent identityと、handoffで発行する次agent identityを混同しない。

- `CURRENT_IDENTITY` は、そのsessionで最初に割り当てられた現在agent自身だけを表す。
- `CURRENT_IDENTITY` はsession終了まで不変であり、途中で変更・継承・再解釈しない。
- handoffで新identityを発行しても、現在agentのidentityは変わらない。新identityは**次の別session / 別agent**のidentityである。
- 過去context、handoff、Git history、Issue、docsに現れる他のidentityは、すべて現在agentから見た第三者として扱う。
- 過去agentの文章中の「私」「自分」「we」等を、現在agent自身への一人称として読み替えない。
- 他agentの作業を知っていること、引き継いでいること、current worktreeに存在することを理由に、その作業を `CURRENT_IDENTITY` の実績として帰属しない。

handoff promptの冒頭では、次agentに対して次の意味を明示する。

```text
CURRENT_IDENTITY: <next identity>

CURRENT_IDENTITY is the only current actor for this session and is immutable until the session ends.
All other agent identities appearing in prior context, Git history, Issues, docs, or handoff material are third parties.
```

この規則は文体上の演出ではなく、監査・review・Issue attribution・Git attributionで自他を混同しないためのprovenance境界として扱う。

### next identityの発行記録はcommitする

実際にhandoff identityを発行するときは、`AGENTS.md` の `latest issued identity` / `generation max` という次agentの発行記録を未コミットのまま残さずcommitする。現在agent自身のidentityを変更する操作ではない。

commit messageの末尾には、そのcommitを作成した**現在担当者**のidentityを `[Dawn-4]` のような形式で必ず付ける。関係のない未コミット変更をhandoff commitへ勝手に混ぜない。

pushは自動では行わない。ユーザーの指示またはAutoDrillの既存方針に従う。handoff promptにはpush済みか未pushかを実値で書く。

## 3. 現在状態は必ず実値から取得する

canonical sourceから取得できるものを、過去chatや記憶から推測しない。

少なくとも次をhandoff直前に確認する。

### Git

- repository root
- current branch
- full HEAD SHA
- upstream
- local/remoteの差
- worktree status
- push済みか未pushか

handoff作成のためにbranchを作ったり切り替えたりしない。

commit、amend、rebaseなどでHEADが変わった後は、出力直前にもう一度full SHAを取得する。

### Issues

GitHub Issuesを現在のbacklogの正典として、**現在の作業と未完了事項に関係するIssueだけ**をlive stateから確認する。

必要に応じて、number、title、state、labels、本文、最新commentを読む。archiveや古い会話要約をcurrent stateの代わりにしない。

deferred、user confirmation待ち、product decision待ち、technical failureなどを混同しない。

### ユーザー指示

現在の会話で最後に確定した指示を優先する。古いworkflowを復活させない。

特に、commit / push / Closeの可否、branch制約、確認待ち、触ってはいけない範囲など、次担当者が誤ると作業結果が変わる指示を残す。

AutoDrillでは **Work modeへの切替を提案しない** というユーザー指示も毎回handoffへ残す。これは省略可能な会話上の好みではなく、次担当者の実行環境選択を拘束する継続条件として扱う。

### 検証

「テスト済み」のようにまとめず、次担当者が判断に使う重要な検証は、実行したcommandと結果を具体的に残す。

自動testのPASSと、ユーザーによる主観確認、第三者review、product decisionを同じものとして扱わない。

### 一時状態

実行中job、残す必要のあるtemporary file、未コミット変更がある場合は、その具体的な状態と次担当者が最初に何をすべきかを書く。

## 4. 何をpromptへ入れるか

次担当者が知らなければ誤った作業をする情報を優先する。

必ず入れるもの:

- 次担当者identity
- `CURRENT_IDENTITY` が次担当者identityと一致し、session中不変であるという明示的なidentity boundary
- 前担当者identity
- bootstrap命令
- repositoryとsession
- current branch / HEAD / push / worktree
- 現在のmission
- 関係するlive Issueと未完了状態
- 最新の拘束的なユーザー指示
- 現在の判断に必要な完了済み作業
- 重要な検証結果
- 必要なSKILL
- 最初に行う具体的な操作

完全に終了し、今後の判断に影響しない古いIssueや作業履歴を毎回列挙しない。

repositoryを正しく起動した後にcanonical docsから安全に取得できる一般的なarchitecture説明は、現在の作業に直接必要でなければ大量に転載しない。

## 5. 出力

最終出力は、次のChatGPTへそのままコピー&ペーストできる**一つのコードブロック**にする。

コードブロック内の冒頭で、bootstrap命令を先に書く。その後の構造はtaskに合わせてよいが、通常は次の情報を読みやすく整理する。

- Identity
- Mission
- Execution environment
- Mandatory startup
- Git state
- Relevant live Issues
- Binding user instructions
- Relevant completed work
- Verification evidence
- Open / deferred / decision items
- First actions
- Completion / reporting constraints

固定テンプレートを満たすためだけに空のsectionを作らない。

コードブロック外には、原則として説明を付けない。

## 発行前audit

出力直前に確認する。

- `@local-mcp` が冒頭にある
- exact `session_id` を実際に確認した
- repository pathが正しい
- root `AGENTS.md` のみを使う指示がある
- 親AGENTSと委譲機構を使わない指示がある
- `docs/principles.md` を最上位原則として読む指示がある
- 必要なrepository内SKILLが具体名で指定されている
- branchを作成・切替しない指示がある
- identityがA〜Wのrotationと `generation max + 1` に従う
- handoff prompt冒頭に `CURRENT_IDENTITY` があり、session中不変・他identityは第三者という規則が明記されている
- `AGENTS.md` のnext identity発行記録（`latest issued identity` / `generation max`）を更新しcommit済みである
- next identity発行記録のcommit後に最新HEADを再取得した
- Git / push / worktree状態が実値と一致する
- live Issue状態を古い記録から推測していない
- 最新のユーザー指示が反映されている
- Work modeへの切替を提案しない制約がhandoff promptに明記されている
- 実行していない検証をPASSと書いていない
- 次担当者が「sessionは？」「repositoryは？」「何を読めばよい？」「どのbranch？」「何をすべき？」と追加質問せず開始できる

一つでも取得可能なのに未確認の重要情報があれば、handoff promptを完成扱いしない。

## 最後に自分のagent-historyを残す

上のhandoff必須作業をすべて終え、identity・Git・Issue・bootstrap情報とprompt本文が固まった**作業の末尾**で、`docs/agent-history.md` に現在担当エージェント自身の功績を記録する。これはhandoffの正確性より重要度が低い。identity発行、実環境確認、live Issue確認、handoff prompt作成を遅らせてまで先に行わない。

記録するのは既存形式に合わせた次の3点だけとする。

- **エージェント名** — 現在担当者の発行済みidentity。
- **やったこと** — Git履歴、GitHub Issue、実際の作業内容から確認できる代表的な功績を簡潔に要約する。細かなcommit一覧にせず、行っていない仕事を推測で帰属させない。
- **直属の後継者** — このhandoffで自分が命名した次のエージェント。まだ命名していない場合は `—`。

同じエージェントのentryがすでに存在する場合は重複追加しない。必要な訂正だけを行う。更新する場合は、関係のない未コミット差分を混ぜず、原則として `docs/agent-history.md` だけの独立commitにする。commit attributionはrepository直下 `AGENTS.md` の現在規則に従う。
