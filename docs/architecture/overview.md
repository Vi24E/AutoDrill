# Architecture Overview

AutoDrillは、Rust domain core + thin WASM boundary + Next.js/React Web UIからなるmodular monolithとして実装する。

最上位原則は [`../principles.md`](../principles.md) を参照する。

## Runtime model

alphaではPC向けの静的Webアプリとして動作する。

- static HTML / JS / WASMを配信する。
- worksheet generation、answer parsing、normalization、grading、effort計算はclient端末上で行う。
- server APIやdatabaseを前提にしない。
- localStorage / IndexedDB等の端末内保存は必要に応じて使用できる。
- 初期betaまでは広告を除きlocal-firstを維持する予定である。
- beta移行時にmobile supportを実施する。

server/client architectureが必要になった場合でも、Rust domain coreを再利用し、client/serverで数学的ロジックを複製しない。

## Responsibility boundary

### `crates/drill-core`

数学的source of truth。

- deterministic generator
- problem / worksheet identity
- canonical answer
- typed mathematical data
- normalization
- grading
- generation validation
- effort operation plan / operation counts
- reusable arithmetic helpers
- current generator revision identity and deterministic regeneration

browser DOM、CSS、pixel position等は扱わない。

### `crates/drill-wasm`

薄いserialization / API boundary。

- Rust型とversioned JSON DTOの変換
- public WASM entry point
- boundary validation / error mapping

数学的なbusiness logicをここへ移さない。

### `apps/web`

browser presentation / interaction layer。

- Next.js static routes / SEO metadata
- React state / DOM
- MathLive integration
- pointer / keyboard / focus
- worksheet rendering
- print preview / browser PDF
- accessibility
- browser-local persistence
- Rust/WASM contract validation

数学的normalizationやgradingをTypeScriptへ再実装しない。

## Cross-language source of truth

RustとWebの両方で必要になるsemantic metadataは、同じ事実を手で二重管理しない。

- Rust-owned compatibility / generation metadataはgenerated Web contractへ投影する。
- Web-only presentation metadataはWeb theme definitionが所有する。
- 両runtimeで必要なtaxonomy / safety / presentation policyは、どちらかをcanonical sourceと決めてgenerated projectionを作る。

詳細は [`theme-system.md`](theme-system.md) を参照する。

## Theme architecture

各themeの固有ロジックはtheme moduleへ局所化する。

共通frameworkはmechanismを提供し、numeric theme IDによる特殊caseを持たないことを原則とする。

詳細は [`generator.md`](generator.md) と [`theme-system.md`](theme-system.md) を参照する。

## Input architecture

ユーザー入力は任意textを数学値として扱うのではなく、MathLiveの構造をRust側でtyped AnswerNodeへ変換する。

- input interfaceはproblemごとのcapabilityとして定義する。
- malformed / too deep / too large inputはfail closedに扱う。
- UI表示状態と採点用数学値を混同しない。

詳細は [`answer-ast.md`](answer-ast.md) と [`web-print.md`](web-print.md) を参照する。

## Printing

印刷はserver-side PDF serviceを持たず、同じworksheet dataからprint専用DOMを構成してbrowser print/PDFへ渡す。

- Webとprintで問題内容を再生成しない。
- 数学的rendererを不要に二重化しない。
- 筆算はWeb/printで共通のcoordinate modelを使用する。

## Performance policy

- correctness / determinism / maintainabilityをmicro-optimizationより優先する。
- ただし同期WASM処理がUXをblockするため、generatorは問題数に対して合理的にboundedでなければならない。
- domain全列挙等、domain sizeに不必要に比例する処理を避ける。
- algorithmic work budgetとwall-clock watchdogを区別する。
- performance問題はbrowser上でも計測する。

## Testing policy

単体testだけで品質を判断しない。

最低限:

- Rust unit/property tests
- WASM/Web contract tests
- deterministic seed regression
- multiple-seed generator audits
- browser interaction tests
- screenshot / visual verification for layout-sensitive work
- browser print/PDF verification for print-sensitive work

特にUX・筆算・MathLiveは「testがpassした」だけで完了としない。
