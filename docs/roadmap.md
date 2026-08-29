# AutoDrill Roadmap

この文書は、現時点で確定しているsupport scopeと将来方向を管理する。

具体的な未解決bugはcanonical GitHub Issues（[`issues.md`](issues.md)から参照）、実装判断の上位原則は[`principles.md`](principles.md)を参照する。

## Current alpha

### Product scope

- 日本の小1〜中3の算数・数学ドリルを段階的に拡充する。
- Web上で回答・採点できることと、紙へ印刷して使えることの両方を重視する。
- 対応単元数よりgenerator品質、入力UX、採点の正確性、印刷品質を優先する。
- alphaの対象deviceは**PC**とする。
- alphaではmobile supportをrelease requirementにしない。

### Runtime / infrastructure

- static Web appを基本とする。
- generation / input / grading / printはclient上で完結する。
- backend / user databaseを前提にしない。
- alpha公開は現在GitHub Pagesを使用している。
- security hardeningはclient-onlyであることとは別に継続する。

### Development priorities

- 小1〜中3のtheme coverage拡大
- theme generatorのmodule化と再利用可能な共通基盤
- typed metadata / tag architecture
- effort primitive / standard solution modelの改善
- MathLive inputの正確性・堅牢性・操作性
- Web / printの視覚品質
- Seed / reproducibility設計

## Beta transition requirements

alphaからbetaへ進める段階で、少なくとも次を改めて扱う。

### Mobile support

**mobile対応はbetaへの移行時に解決する。**

その段階で、mobileを正式support対象として再設計・再監査する。

確認対象:

- worksheetのresponsive layout
- input panel / MathLive keyboard
- dropdown / modal
- touch target
- scrolling / viewport overlap
- print導線
- 320 / 375 / 390 / 430px等の代表viewport
- 実機または実機相当browserでのUX

alpha中に発見したmobile-only overflow等は、原則としてGitHub Issuesで **`priority:deferred`** として保持する。これはLow severity / 低優先度を意味せず、beta移行まで実装を意図的に凍結するpriority classである。beta移行時にこのclassのmobile Issueを再開し、beta transition requirementとして扱う。

### Public-quality review

- major Critical / High issueの解消
- generator / grading / printの大規模再監査
- deployment securityの再確認
- dependency / browser supportの再確認
- SEO / route metadataの公開品質確認

## Initial beta runtime policy

初期betaまでは、**広告を除き学習処理を完全にlocal device内で完結させる予定**とする。

- answer / history等をserverへ送信することを前提にしない。
- server componentが必要になった場合、local-first/security方針との整合を先に確認する。

広告・analytics等を導入する際は、送信data、privacy、CSP、third-party scriptを別途設計・監査する。

## Future server/client model

将来的には次の機能のためserver/client architectureを採用する可能性が高い。

- login / account
- device sync
- learning history
- weakness analysis / personalization
- premium plan
- teacher / student management

導入時も、Rustの数学的domain logicをclient/serverへ重複実装しない。

具体的backend technologyは、その時点の要件に基づいて決定する。過去のlegacy planに書かれた特定stackを、現在の確定仕様とはみなさない。

## Seed / worksheet identity

将来、Seedからdifficulty・詳細設定等を含むworksheet条件全体を復元できるようにし、共有identifierとして一意性を高める予定。

これは互換性・generator revision・URL/share UXへ影響するため、ad-hocに変更せずversioned designとして実施する。

## Later product directions

現時点では確定実装仕様ではないが、長期候補として次がある。

- 文章題を含む全単元coverage
- learning analytics / personalization
- teacher向け複数worksheet生成
- user account / sync
- premium機能
- 多言語 / 他国curriculum

これらは現在のarchitectureを不必要に複雑化する理由にはしない。実装時に要件を再確認する。
