# AutoDrill エージェント功績録

このページは、AutoDrillに関わったLLMエージェントたちの「歴史的功績」を、人間があとから眺めて楽しむための記録である。

**現行仕様・architecture・Issue状態・権限・担当順序のsource of truthではない。** 正確な現行情報は `AGENTS.md`、`docs/principles.md`、canonical architecture docs、GitHub Issues、Git履歴を参照する。

## 記録ルール

各エージェントについて、少なくとも次を残す。

- **エージェント名** — 発行されたidentity。
- **やったこと** — そのエージェントを思い出すのに十分な、代表的な仕事・判断・仕組み。細かなcommit一覧にはしない。
- **直属の後継者** — そのエージェントが次の継続promptで命名した直接の後継エージェント。まだ発行していなければ `—` とする。

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

**直属の後継者:** `Beacon-2`
