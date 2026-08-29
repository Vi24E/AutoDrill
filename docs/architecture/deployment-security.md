# Deployment / Security Architecture

AutoDrillの公開・security方針。最上位原則は[`../principles.md`](../principles.md)を参照する。

## Threat surface policy

alpha〜初期betaはstatic / local-firstで設計し、学習処理をclient端末内に閉じる。

これにより、少なくとも現段階では次を持たない。

- user credential database
- answer/historyを保持するapplication server
- server-side generator / grader API
- application secretを必要とするbackend

これはattack surfaceを小さくする意図的な設計である。

ただし「serverがない = security対策不要」ではない。

継続して扱う対象:

- dependency vulnerability
- supply-chain risk
- XSS / DOM injection
- malicious / malformed MathLive input
- WASM DTO validation
- recursion / memory / CPU DoS的入力
- CSP等のbrowser defense-in-depth
- third-party ads / analytics導入時のprivacyとscript trust

## WASM boundary

Rust/WASM境界はfail closedを原則とする。

- schema version
- theme / revision identity
- layout contract
- answer schema
- input capability
- problem count
- finite numeric values
- malformed DTO

をvalidationし、未知・不整合dataをUIへ正常worksheetとして流さない。

## User input

MathLive等のuser-controlled inputはboundedに扱う。

- raw input length
- AST size
- nesting depth
- recursion
- parse failure

に明示的な上限を持つ。

正常な構造入力を誤ってoverflow扱いすることと、長大入力を見逃すことの両方を避ける。現行の既知問題は [GitHub Issues](https://github.com/Vi24E/AutoDrill/issues) で追跡する。

## DOM / content injection

原則:

- user inputをraw HTMLとして挿入しない。
- `dangerouslySetInnerHTML`等を安易に使用しない。
- 数学表示はtrusted renderer / typed dataから構成する。
- 外部HTMLを取り込む機能を導入する場合は別途security designを行う。

## Current GitHub Pages limitation

現在のalpha deploymentはGitHub Pagesのstatic exportを使用する。

`apps/web/next.config.mjs`には通常deployment向けに次のresponse header policyがある。

- Content-Security-Policy
- X-Content-Type-Options
- X-Frame-Options
- Referrer-Policy
- Permissions-Policy

ただしGitHub Pagesのstatic exportではNext.jsの`headers()`を適用できないため、**現在のPages公開物にはこのresponse-header policyがそのまま配信されない。**

これは [GitHub Issues](https://github.com/Vi24E/AutoDrill/issues) のM-005で追跡する。

## Static-host decision

GitHub Pagesを継続するか、response headerを設定できるstatic hostへ移行するかは、security要件・独自domain・広告/analytics導入等と合わせて判断する。

host変更自体を目的にしない。一方、必要なsecurity policyが現hostでは実現できない場合は、workaroundを重ねるよりhost選定を見直す。

server-side applicationを導入する必要はなく、Cloudflare Pages / Netlify / Vercel等のstatic hostingで十分な可能性もある。その時点の要件・費用・securityを比較して決める。

## Ads / analytics

初期betaまでは学習処理はlocal-only予定だが、広告は例外となり得る。

広告・analyticsを導入する前に少なくとも次を確定する。

- third-party script一覧
- network destination
- 送信data
- cookie / localStorage usage
- privacy disclosure
- CSP policy
- failure時にdrill本体を壊さないisolation

教材のgeneration / gradingを広告scriptへ依存させない。

## Future accounts / server

login等を導入するとthreat modelは大きく変わる。その時点で別途、

- authentication / authorization
- personal data handling
- server validation
- CSRF / session security
- rate limiting
- logging / retention
- backups / disaster recovery

を設計する。

過去のlegacy planに書かれたbackend stackを自動的な確定仕様とはみなさない。server導入時はユーザーへ設計確認する。
