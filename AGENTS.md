# AGENTS.md — AutoDrill

このファイルはAutoDrill repository内で作業するLLM向けの正典である。親directoryの `AGENTS.md`、`ROLES.md`、中央Board、委譲・role-routing protocolはAutoDrillの契約として使用しない。

## セッション開始時の必須動線

作業前に必ず次を行う。

1. **repository直下のこの `AGENTS.md` のみに従う。** `../../AGENTS.md`、`LLM/AGENTS.md`、その他の親 `AGENTS.md` は読まず、適用しない。
2. **`docs/principles.md` を読み、最上位の設計原則として適用する。**
3. `docs/README.md` を読み、現在のcanonical docsの入口を確認する。
4. Git root、現在branch、worktreeを確認する。ユーザーの明示指示なしにbranchを作成せず、既存branchへも切り替えない。
5. 変更対象に対応する `docs/architecture/` 文書と [GitHub Issues](https://github.com/Vi24E/AutoDrill/issues) を必要範囲で読む。
6. taskに必要なSKILLがある場合、ChatGPT内蔵機能ではなくrepository直下の `SKILL/<name>/SKILL.md` を直接読む。
7. ローカル作業にlocal-mcpを使う場合は、conversationで指定されたsessionを使い、session idを推測しない。

subagent、role-routing、delegation protocolなどの委譲機構は使用しない。

## LLM identity / generation management

このsectionのidentity情報は、複数のLLMを人間が区別し、名前やgenerationの重複を防ぐための発行メモである。権限、管理者、branchの優先順位、発言の正しさを表さない。

- **latest issued identity:** `Pine-16`
- **generation max:** `16`
- 通常のLLM identityは `<一般名>-<generation>` とする。一般名は人間が認識しやすい一般語・一般的な名称を使う。
- LLMが発行してよい通常identityの頭文字は `A -> B -> ... -> W -> A ...` の順で循環させる。
- `X`、`Y`、`Z` は特殊用途の予約文字とし、LLMは発行・管理しない。
- generationは通常identity全体で単調増加させる。新しいidentityは必ず、このsectionの `generation max + 1` を使う。
- 次に通常identityを発行するときは、`latest issued identity` の頭文字からrotationを進める。現在は `Pine-16` なので、次は **Qで始まる一般名**とgeneration `17`を使う。
- 現在作業しているLLM自身のgenerationから次generationを推測しない。必ずこのsectionを発行直前に読み直す。
- 同一LLM instanceが通常の会話や同一taskを継続するだけでは新generationを発行しない。**新しいLLMへ実際に渡すhandoff promptを発行するとき**だけ新identityを発行する。
- 新identityを発行するときは、handoff promptを返す前に `latest issued identity` と `generation max` を更新し、その更新をcommitする。previewだけを作る場合は更新しない。
- identity更新後は `AGENTS.md` を再読し、handoff prompt内のidentityと一致することを確認する。

## Git規則

- ユーザーの明示指示なしに新しいbranchを作成しない。
- ユーザーの明示指示なしに既存branchへ切り替えない。現在branchを維持する。
- 既存の未コミット差分を勝手に破棄、reset、checkout、stashしない。
- LLMが作成する**すべてのcommit message**は、末尾にそのcommitを作成した担当identityを `[Dawn-4]` の形式で付ける。
- commitに関係のない既存差分を勝手に混ぜない。
- pushは自動の既定動作としない。ユーザーの指示または現在の作業で明示された方針に従う。
- backup目的でrepositoryやfileのcopyを作らない。rollback pointはGit commitで保存する。commitできない事情がある場合は、copyを作る前にユーザーへ相談する。

### GitHub Issue reporter / updater attribution

- Issue / backlogのSoTはGitHub Issuesである。Issue本文の先頭に `**報告者:** <name>` を明記する。
- LLMが報告する場合の`<name>`はそのLLM identity（例: `Alpha-1`）とする。ユーザー自身が報告した内容をIssue化する場合は `**報告者:** User` とする。
- Issueの`報告者`はIssueを起票した情報源を表し、後続の更新で書き換えない。
- LLMが既存Issueへ本文追記・修正・comment・follow-up・verification結果を追加する場合、**その更新部分**に `**更新者:** <identity>` を残す。ユーザーの指示内容をユーザー自身の更新として記録する場合は `**更新者:** User` とする。
- 既存文章のattributionを後続更新者へ上書きしない。誰が書いたかを区別できる単位で報告者/更新者を残す。
- 2026-08-30にMarkdown backlogからGitHubへ移行したIssue、および同日に作成したaudit Issueは、ユーザー指示により過去記録を含めて `**報告者:** Alpha-1` とする。
- Issueを読むLLMは、GitHub accountの投稿者だけでなくこの報告者/更新者表記を見て、自分・他agent・Userの記録を区別する。

#### Issue本文の標準形式

新規Issueは、既存の[#1](https://github.com/Vi24E/AutoDrill/issues/1)を基準に、本文冒頭を次の順で統一する。

```text
**報告者:** <User または LLM identity>

**状態:** <Open / Planned / Deferred / User confirmation pending / Closed>
**対象:** <subsystem / feature / architecture boundary>
```

- 冒頭の後に、観測された問題または達成したい目的を文章で記す。原因・再現条件・evidenceが確認済みなら続けて記す。
- 解決方針が決まっている場合は `**方針**`、客観的な完了条件を定義できる場合は `**Close条件**` を使う。未決定のproduct decisionを埋めるために推測で方針やClose条件を作らない。
- `状態` はGitHubのOpen/Closed stateと `status:*` / `priority:deferred` labelの人間向けprojectionであり、本文だけを独立した状態SoTにしない。
- 既存Issueの履歴記述は削除・再解釈せず、後続更新は原則commentへ `**更新者:** <identity>` とともに追加する。本文を整理する場合も、意味を変えずattributionを残す。
- Issue titleにはseverity・status・source・独自連番を新規に埋め込まない。GitHubの `#<number>` を唯一のIssue identifierとし、severity / status / source / priorityはlabelをsource of truthにする。既存migration Issueの `C-001` / `H-004` / `AUDIT-*` 等は履歴としてrenameを強制しない。

### `status:user-confirmation` の運用

- `status:user-confirmation` は、実装と客観的なtechnical verificationが完了し、残る受入条件が見栄え、読みやすさ、操作感、教材としての自然さ等の**人間による主観的確認**だけであるIssueに使う。product/architectureの仕様決定、technical failure、未完了testの代用には使わない。
- labelを付与する**前**に、Issue本文または最新commentへ `**更新者:** <identity>` とともに `### User confirmation procedure` を記載する。単に「ユーザー確認待ち」「見栄えを確認」だけでは不十分で、少なくとも次を再現可能に明記する。
  1. **確認場所**: stableなroute/theme/画面。必要なら公開URLまたはlocal起動commandも記す。
  2. **再現条件**: seed、difficulty、viewport、設定値、事前状態など、結果へ影響する条件。決定的に固定できる値は固定する。
  3. **操作手順**: どのcontrolをどの順序で操作するか。問題番号や入力対象を固定できる場合は固定する。
  4. **確認項目と期待結果**: どこを見て、何なら合格かを具体的に書く。複数観点がある場合は分ける。
  5. **不合格時に残す情報**: 再現step、problem番号/表示内容、期待との差。見た目の問題ではscreenshotが有用なら依頼してよい。
- User confirmation手順はagent固有のtemporary file、消えるbrowser state、未共有のlocal dataへ依存させない。可能な限りcanonical route、deterministic seed、repositoryに残る設定/手順を使い、別sessionのLLMやUserが同じ状態を再現できるようにする。
- ChatでUserへ確認を依頼するときも、**どこで何を確認するか**を具体的に再掲する。Issue番号だけを示して「確認してください」と依頼してはならない。Issueに記録したroute/seed/操作/期待結果とChatの依頼内容を一致させる。
- 再現可能な確認手順を定義できない場合は `status:user-confirmation` を付けない。要求自体が未確定ならproduct/architecture decisionとして、客観条件が未達ならtechnical failureとして扱う。
- Userが確認結果を返したら、その結果と確認条件をIssueへ記録する。Userの回答をそのままIssueへ転記する場合は `**更新者:** User` とし、不合格ならlabelを外して新しいtechnical/product workへ戻す。

## このprojectについて

日本の計算ドリルを決定的に生成・回答・採点・印刷する静的Webアプリ。

## ディレクトリ構成

| パス | 役割 |
|---|---|
| `apps/web/` | Next.js 16の設定・回答画面、WASM adapter、印刷UI |
| `crates/drill-core/` | Rustの決定的生成・編集・正規化・採点・努力量計算 |
| `crates/drill-wasm/` | Rust coreをJSON/WASM DTOへ変換する薄い公開境界 |
| `docs/principles.md` | 最上位の設計思想。実装前に必読 |
| `docs/README.md` | canonical docsへの入口 |
| `docs/architecture/` | 現行architecture / implementation policy |
| [GitHub Issues](https://github.com/Vi24E/AutoDrill/issues) | Issue / backlog の唯一のsource of truth |
| `docs/roadmap.md` | support scope / 将来計画 |
| `scripts/build-wasm.sh` | Rust/WASM targetを変更せずに行うローカル生成パイプライン |
| `README.md` | project概要、インストール、開発、検証コマンド |
| `SKILL/` | AutoDrillで使用するlocal skill。必要なものだけ直接読む |

## AutoDrill固有の実装原則

- `docs/principles.md`を最上位の設計判断として扱う。
- Issue / backlogは [GitHub Issues](https://github.com/Vi24E/AutoDrill/issues) を唯一のsource of truthとし、repository内へcurrent backlogのMarkdown mirrorを作らない。
- 設計思想が不明な場合、または場当たり的な例外が必要な場合は、実装前にユーザーへ質問する。
- theme固有generatorを共通巨大fileへ追加しない。共通資源は再利用し、theme固有処理はtheme moduleへ置く。
- 本来metadataである性質をnumeric theme ID、slug、表示文言、生成結果等から推測しない。
- alphaはPC対象。mobile対応Issueは`priority:deferred`としてbeta移行まで凍結し、beta移行時に再開する。
- 一次テスト・debug用に作成した一時script、fixture、screenshot、browser profile等は、正式な資産として残す理由がなければ作業終了時に削除する。
- 既存の未コミット差分を勝手に破棄しない。

## コマンド

- install: `pnpm install --frozen-lockfile`
- dev: `pnpm dev`
- build: `pnpm build`
- lint: `pnpm lint`（補助: `pre-commit run --all-files`）
- typecheck: `pnpm typecheck`
- web test: `pnpm test`
- Rust test: `cargo test --workspace --all-targets`
- WASM package: `./scripts/build-wasm.sh`（target/toolingが既にある場合だけ）

`scripts/build-wasm.sh` は `wasm32-unknown-unknown` と `wasm-bindgen` CLIを自動インストールしない。初回環境でtargetが無い場合は、スクリプトのエラーを確認し、開発者が自分のRust toolchainへ前提を用意してから再実行する。

ブラウザ向けWASMを生成した後は、Next.jsのdev/buildで`apps/web/public/wasm/pkg/`を配信し、`apps/web/src/wasm/load-generated.ts`から動的にロードできる。

## 規約・注意

- product codeとproduct固有の設定だけをこのrepositoryで管理する。
- 親LLM環境のBoard、Dashboard、Doctor、role contract、provider adapter、委譲設定をこのrepositoryへコピーしない。
- 認証情報、token、秘密鍵をcommitしない。
