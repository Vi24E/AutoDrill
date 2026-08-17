# AutoDrill — Design Principles

この文書は AutoDrill の実装判断における最上位の設計原則である。

AIエージェントを含む実装者は、production codeを変更する前にこの文書を確認すること。個別仕様・古いaudit・過去のplanと本書が衝突する場合は、本書と最新のユーザー指示を優先する。

設計上の判断が本書から一意に定まらない場合、または場当たり的な例外実装が必要に見える場合は、勝手に補完せずユーザーへ設計を相談する。

## 1. Maintainability first

AutoDrillは使い捨てのprototypeではない。小1〜中3の多数のthemeへ拡張し、長期的に保守する前提で設計する。

- 既存の再利用可能な実装資源がある場合は再利用する。
- 共通化する価値がある新しい処理は、再利用可能性を考えて実装する。
- ただし将来の可能性だけを理由に過剰な抽象化を先行しない。
- 同じ意味・規則・metadataを複数箇所へ手入力しない。
- theme固有の処理はtheme側へ置き、共通frameworkへnumeric theme ID等の特例を蓄積しない。
- 共通primitive、共通renderer、共通validation、共通sampling mechanism等は中央に置き、themeから再利用する。
- magic number、表示名、slug、numeric ID、生成済みproblemの特徴から、本来metadataとして明示すべき性質を逆算しない。
- 場当たり的な例外処理が必要になった場合は、実装前に設計を相談する。

## 2. Static / local-first architecture

alphaの間はPCを対象とし、基本的に静的Webアプリとして設計する。

- 問題生成、入力、正規化、採点、印刷は原則クライアント端末内で完結させる。
- 初期betaまでは、広告を除き、ユーザーの学習処理を完全にローカルで完結させる予定とする。
- この方針はserver運用コストだけでなく、認証情報・個人データ・server-side attack surfaceを持たないことでsecurity riskを低減する目的も持つ。
- client-onlyであることを理由に、CSP・dependency hygiene・input validation・WASM boundary validation等のsecurity hardeningを省略しない。
- serverを導入しないために不自然なarchitectureへ歪める必要が出た場合は相談する。
- 将来的にlogin、端末間同期、学習履歴、premium等のためserver/client modelを導入する可能性は高い。その場合もRust domain logicをserver/clientで重複実装しない。

### Mobile scope

- alphaではmobileをsupport対象にしない。
- mobile responsive問題はalphaのrelease blockerにしない。
- **betaへ移行する段階でmobile対応を解決し、mobile UXを改めて監査する。**

## 3. UX quality is a functional requirement

AutoDrillではUXの細部を品質の一部として扱う。

- 数学的に正しいだけでは十分ではない。
- 数式、入力欄、方眼、筆算、button、panel等の位置・中心・baseline・余白を視覚的に自然に揃える。
- 「数pxずれているだけ」を無条件にcosmeticとして扱わない。教材では数字の桁位置や方眼alignmentが理解・記入のしやすさへ直結する。
- Web上での回答だけでなく、A4へ印刷したときの読みやすさ・書き込みやすさを実物相当で確認する。
- UI変更はDOM上のoverflow testだけで完了とせず、必要に応じて実Chrome screenshot / print preview / PDFで目視する。
- 子どもが説明なしでも操作できることを重視する。
- 物理keyboardは利用できるようにしてよいが、基本操作をkeyboard必須にはしない。

## 4. UI should be intuitive and restrained

華美さではなく、直感性・明快さ・教材としての落ち着きを優先する。

- 装飾を増やすことで完成度を演出しない。
- hierarchy、spacing、alignment、typography、interaction feedbackで品質を作る。
- 問題用紙・印刷面は教材として自然な簡潔さを保つ。
- 意味のないanimation、過剰な色、装飾枠、視認性を下げる意匠を避ける。
- UI上の情報量は必要十分にし、advanced settingは必要に応じて段階的に開示する。

## 5. Domain ownership

数学的な意味論はRust coreをsource of truthとする。

Rust coreが所有するものの例:

- deterministic problem generation
- canonical answer
- normalization / grading
- typed answer semantics
- effort operation graph / operation counts
- mathematical validation
- generator revision / reproducibility

Webが所有するものの例:

- DOM / React state
- focus / pointer / keyboard interaction
- MathLive integration
- visual presentation
- worksheet / print layout
- accessibility
- route / SEO presentation metadata
- localStorage等のbrowser-local persistence

WASMは薄い境界とし、Rust/Webのどちらにも同じ数学ロジックを重複実装しない。

## 6. Explicit metadata, not feature inference

Themeやworksheetの性質は、可能な限りtyped metadata / capability / policyとして明示する。

本来metadataである性質の例:

- 対象学年
- taxonomy / genre
- 筆算themeであること
- 印刷推奨
- mixed-number presentation policy
- safety constraints
- sampling/archetype policy
- worksheet layout policy

これらを次の情報から逆算しない。

- numeric theme ID
- route slug
- 日本語label / curriculum path
- generated problemの集合
- unrelated answer schema
- unrelated layout dimensions

ただし、`ProblemPrompt`のvariantをrenderするために`prompt.kind`で分岐する等、**その型そのものを処理する分岐**は正当である。

Rust/Webの双方で必要なmetadataは、原則1つのcanonical sourceからgenerated contract等で投影する。

## 7. Reuse the mathematical primitives

Effortを含む数学処理は、標準解法を再利用可能なprimitiveへ分解する。

- effortは原則 `operation_counts · operation_weights` とする。
- 共通の加減乗除、GCD/LCM、逆数、約分、小数点移動、因数探索等をthemeごとに再実装しない。
- 真にtheme固有の例外だけをtheme側に置く。
- 「難しそうだから定数を足す」式のad-hoc scoringは避ける。

詳細は `architecture/effort-model.md` を参照する。

## 8. Determinism and versioning

- 同じ**現行**problem-set identityから同じworksheetを再生成できることを重視する。
- pre-releaseで実ユーザー・保存済みworksheet・公開済みproblem-set IDとの互換性が不要な間は、**各themeのregistryには現行generatorを1つだけ登録する**。
- generator変更でrevisionを更新しても、置き換えられた旧revisionをhistoric generatorとしてproduction codeへ残さない。旧revision専用registration、schema projection、compatibility adapter、fingerprint fixture等も、現行仕様に不要なら削除する。
- 互換性維持のためにactive generator / registry / schemaを複雑化しない。過去revisionをGit履歴以外のproduction implementationとして保存しない。
- 実ユーザー、保存済みworksheet、外部に公開されたproblem-set ID等が存在し、後方互換性がproduct requirementになった時点で、version compatibility policyを改めて設計する。
- performance改善を理由に**現行revision内の**determinismを壊さない。
- Seed / worksheet identityの将来拡張は `roadmap.md` と `issues.md` の現行方針に従う。

## 9. Web / print parity

Webと印刷で数学的内容を別実装しない。

- 同じProblem DTOからWeb/PDFを構成する。
- 数式rendererやpresentation logicを不要に二重化しない。
- 筆算の数字・answer・solution stepsは共通coordinate modelへ乗せる。
- Webだけ、PDFだけにしか存在しない数学的special caseを作らない。

## 10. Repository hygiene and backups

AIエージェントの一時作業物をrepositoryやDesktopへ蓄積しない。

- 一次テスト、検証用script、temporary fixture、screenshot、browser profile、生成したdebug artifact等は、目的を終えたら削除する。
- regression testや正式なfixtureとして価値があるものだけを、意図を明示してrepositoryへ残す。
- 「念のため」でproject directoryやrepository全体をfile copyしてbackupしない。
- repositoryのbackup / rollback pointは**Git commit**で作る。履歴として価値のある状態はcommitによって保存する。
- 既存の未コミット差分を保護する必要がある場合、内容を確認し、ユーザーの許可範囲でcommitする。勝手に差分を破棄しない。
- commitできない事情があり、copy backupが必要に見える場合は、先にユーザーへ相談する。
- `/tmp`、Desktop、repository外に作成した一時物も作業終了時にcleanupする。ただしAutoDrill以外のファイルを推測で削除しない。

## 11. When to ask the user

以下に該当する場合、実装前に質問する。

- 複数の自然なarchitecture案があり、長期的なsource of truthが変わる。
- 既存の設計原則と要求が衝突する。
- ad-hoc exceptionを追加しないと実装できない。
- serverを追加する、external serviceへ依存する、永続dataを外部送信する等、local-first方針を変更する。
- theme metadata / tagging / ownershipの境界が曖昧である。
- 数学的標準解法や日本の教材表記について仕様が一意でない。
- UI上の挙動に複数の妥当な解釈があり、その差が継続的なUXへ影響する。
