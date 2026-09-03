# Web / MathLive / Print Architecture

上位原則は[`../principles.md`](../principles.md)を参照する。未解決の既知問題は [GitHub Issues](https://github.com/Vi24E/AutoDrill/issues) で追跡する。

## Theme registry

Webの実装済みthemeは`apps/web/src/domain/themes/`で1テーマ1ファイルとして定義します。各`ThemeDefinition`がroute、学年・ジャンル、worksheet表示、input capability、Rust compatibility identityを所有し、`src/domain/theme-registry.ts`は列挙とlookupだけを担当します。

`curriculum.ts`、単元route、sitemap、worksheet copy、layout、WASM response validationは同じdefinitionを参照します。全学年の公開カリキュラムは実装済みthemeだけで構成します。

## Settings / Worksheet / Print flow

- q1: 設定画面。おすすめまたは学年からthemeを選択し、difficultyを指定する。詳細設定の`Seed`欄はraw RNG seedではなく、schema/theme/revision/raw-seed/difficultyを含むfull problem-set IDの入力・共有欄である。
- q2: Web回答画面。通常themeはMathLive、筆算themeは方眼1桁ごとの独立digit slotで入力し、最終的なtyped AnswerNodeの採点はRust/WASMへ委譲する。
- 採点後: `問題に戻る`で入力を保持してeditingへ戻るか、`もう一回問題を解く`で同じworksheetを初期化する。別worksheet生成のshortcutは持たず、別の問題へ進む場合は`TOPに戻る`を経由してtheme/difficultyを再確認して生成する。
- 印刷: worksheetと同じdataからin-app print previewを開き、そこからbrowser標準の印刷/PDF保存へ進む。

Seed欄が空ならbutton click時にraw generation seedを内部生成して通常requestを行い、Rustが返したcanonical `problem_set_id`をSeed欄・q2/PDF metadata・現在routeの`?seed=` queryへ投影します。Seed欄または`?seed=`にfull IDがある場合は、その文字列をWASM replay endpointへopaqueに渡します。Rustがidentityをparseして同じworksheetを再生成し、WebはID構文を重複実装しません。Userがtheme/difficultyを手動変更した場合は古いreplay ID/queryをclearし、次の生成を新しい設定から行います。

## WASM adapter

`src/domain/wasm-adapter.ts`がproductionの数学境界です。generated Rust Web contractのcurrent schema（現行v7）JSON DTOを使い、公開WASM endpointはcurrent productが実際に消費する `generate_worksheet` / `generate_problem_set` / `parse_mathlive_answer` / `grade_answer` とします。これらを通じて以下をRust/WASMへ委譲します。

- current settingsからのworksheet generation
- full problem-set IDからのdeterministic replay
- MathLive LaTeX → AnswerNode
- AnswerNode/input capability validation
- grading

単一problem生成とstandalone normalizationはcurrent Web consumerを持たないためWASM/public facadeへ公開しません。problem-set ID replayはWeb consumerが存在するため公開しますが、ID parse/validationはRust coreだけが所有します。normalizationはgrading等から使うRust core内部primitiveとして保持します。

Webはnormalization、正誤判定、effort、generator条件を再実装しません。

AnswerNodeのinteger/coefficientとAnswerSchemaの`i64` boundsなど、64-bit exactnessが必要なwire payloadはcanonical decimal stringでJSON越境します。ProblemPrompt / worked solutionで`number`として越境する整数は`Problem::generated`がJavaScript safe-integer範囲をinvariantとして検証し、Web adapterも`Number.isSafeInteger`で境界検査します。`EffortModel` / `OperationPlan` / `OperationVector` / `BigNum`はRust内部実装であり、現行Problem wireへ公開しません。Web側にeffort semanticsやそのvalidatorを持たせません。

## MathLive

通常themeではMathLiveがcaret、placeholder、fraction/root layout、editable renderingを所有します。各input snapshotはFIFO queueでRustの`parse_mathlive_answer`へ送り、Rustが承認したAnswerNodeのみstateへ保存します。中学生（grade 7〜9）の方程式・数値系の通常数式入力はRustのtyped `JuniorHighFull` input profileを使い、themeごとにkey位置を変えず、分数・帯分数・平方根・複数解の2×2 structure grid、数字、小数点、`+ / − / ±`、編集controlからなるfull keypad shellを共通利用します。一方、式そのものを答案にする中1一次式themeは`LinearExpression` input profileを使い、`x`、数字、`+ / −`と編集controlだけをcapabilityから投影する。`variable`をJuniorHighFullへ無条件追加せず、見た目だけenabledなのにRust parserが拒否する状態や、不要なthemeへ変数入力を広げる状態を作りません。

筆算themeのうち、加減乗の最終答案と長除法の商はMathLiveの連続文字列fieldを使わず、ページ方眼と同じ1cell=1digitの独立slotをWeb interaction layerが所有します。加減乗は一の位から右→左、商は標準筆算順に左→右へ進み、selected slotと実DOM focusを常に一致させます。長除法の余りは筆算grid内部ではないため独立digit slotにはせず、通常themeと同じMathLive numeric fieldを再利用してbig-endian入力します。slot draftからinteger / exact decimal / quotient partのtyped AnswerNodeを構成しますが、正誤判定・canonical answer・worked solutionの数学的意味論はRustから移しません。割り算の入力順はRust theme presentation policyのtyped `column_input` metadataでanswer partごとに指定し、Webはその解決済みpolicyだけをinteractionへ投影します。

Rust parserはAnswer AST sizeに加え、raw LaTeX長・structure nestingをparse前に検査します。過度に深いinputはrecursive parserへ入る前に`answer_ast_size_limit`として拒否します。

## Viewport scope

worksheet本体はA4 geometryを基準にWebへ投影します。

**alphaの正式support対象はPCであり、mobile responsiveはrelease requirementではありません。** betaへ移行する段階で、worksheet縮小だけでなくinput panel、MathLive、touch target、scrollingまで含めてmobile UXを再設計・再監査します。詳細は[`../roadmap.md`](../roadmap.md)を参照してください。

狭いviewportでも偶発的に利用できることと、mobileを正式supportすることは区別します。alpha中はmobileのためにPC UXやA4 coordinate modelを場当たり的に歪めません。

## Shared math formatting

通常数式のsourceはRust DTOから`mathlive-format.ts`が作るLaTeXです。Web表示と印刷/PDFは別々の数式rendererを持たず、どちらもMathLive 0.110.0へ同じLaTeXを渡します。筆算だけはfraction/root等の数式組版ではなく桁位置そのものが教材内容なので、`ProblemExpression`内の共通`ColumnArithmeticExpression`が縦式DOMを所有し、Web/PDFの両方で同じcomponentを再利用します。

Webでは通常数式を`ProblemExpression` / `MathLiveStatic`の`<math-span>`で表示し、印刷用DOMでも**同じReact componentをそのまま再利用**します。fraction、root、exponent、括弧、operator spacing、baseline等はMathLiveが一元的に所有し、PDF側にfraction lineやminus記号の座標実装はありません。筆算の縦式・横線・長除法枠・小数点位置もPDF専用実装ではなく同じ`ColumnArithmeticExpression` / CSSを共有します。 解答pageでは最終値だけを別欄へ出さず、加減算は結果まで、二桁乗算は部分積まで、除算は商・各部分積・引き算・最終余りまでを含む「完成した筆算」として同じ桁グリッドへ描画します。割り算記号は日本の教材に合わせて、左側を直線ではなく丸く立ち上がる長除法記号として表示します。

問題単位のouter presentationは`WorksheetProblemCell`をWeb/printで共有します。logical A4 cellの`left/top/width/height`投影、`problem-cell-*` modifier classの合成、問題番号をexpression内外のどちらへ置くか、Mini Sudokuと通常expressionの分岐はこのcomponentだけが所有します。Webはinteractive answer/grade mark、printはstatic answer/solutionをslotとして渡し、同じpresentation familyを各rendererが独自に再分類・再配置しません。
採点後のinteractive筆算では、**ユーザが入力した答案をcanonical answerで置換しません**。user digit slots / 余りfieldをread-onlyでそのまま保持し、不正解時のcanonical answerは同じ方眼座標系のcorrection row（余りは対応するcorrection位置）へ赤字で別レイヤ表示します。正誤markはanswer欄の位置に依存させず、全worksheet theme共通で問題番号の直上へ置き、正解`○`・不正解のcheck-shaped markとも赤く、問題番号より明確に大きく表示します。print解答面のworked solutionとは役割を分離します。

`worksheet_grid` capabilityのpage座標は`worksheet-grid-presentation.ts`が唯一のauthorityです。方眼pitchだけでなくpage-grid origin、grid line indexへの丸め、line座標への投影も同moduleのprimitiveを使います。Mini Sudokuは1 puzzle cell = 2 worksheet-grid cellsを保ちつつ、4×4盤面外周と問題番号cellの原点を整数page-grid lineへ配置します。logical problem cellは盤面を置く領域の選択にだけ使い、`1.1 cell`や`.45 cell`のようなfractional padding/marginでpage-grid alignmentを再現しません。

筆算の桁位置は全theme共通のページ方眼で管理します。方眼1辺はA4幅に対する一定比率（実寸約19.5pt）で、Web previewとnative printで同じ物理比率になります。問題文より下の書き込み領域全体へ方眼を薄く表示し、表示数字は文字列を方眼cellへ分解して配置します。operand、operator、rule、answer、worked-solutionの各rowはすべてこのgridの**整数cell座標**だけから決めます。operator別rendererが独自のpx offsetや暗黙の余白でanswer rowを動かしてはいけません。interactiveな加減乗のanswer slot数はcanonical answerの桁数から決めず、表示operandとoperatorから求める最大必要桁数を確保します。小数加減算では、小数点を縦に揃えた後のoperand textと占有cell数を`column-arithmetic-presentation.ts`で一度だけ決め、rendererとlane sizingが同じ結果を使います。整数部幅と小数部幅を別layerで再計算しません。掛け算ではanswer rowをrule直下へ置き、未解答画面に部分積用の空行を予約しません。加算・減算・乗算の演算記号は右端から`max(operandの表示digit数) + 1` cell目に固定し、短いoperandの桁数によって左右へ動かしません。問題番号も別の割合座標系を持たず、方眼へsnap済みの筆算lane自身の子要素として1×1 cellで描画します。加減乗では演算子列の真上、長除法ではlane左端の除数列の真上を番号cellとし、文字はその1×1 cell中央へ置きます。これにより番号のためにlogical problem cellから座標を再計算する経路が不要になり、番号の桁数、operand幅、problem cellのfractional寸法に依存せず全問で筆算本体との相対位置が同一になります。

小数点はcellを1つ使う文字ではなく、2つの桁cellの境界に置く0幅の黒点として描画します。Web digit editorの入力順序と小数点policyはRust Problem wireの`column_input`だけをauthorityとし、加減算等の`fixed(scale)`、整数系の`none`、小数掛け算の`editable`を区別します。`editable`では未入力時にcanonical小数点位置を表示せず、数字slotとは別semanticとして`.`キーまたは物理`.`で境界を配置・再配置します。割り算の商は`natural_division_flow`、余りは`big_endian`等、answer partごとにtyped policyを指定でき、Webはoperator/schema/theme IDから方向を推測しません。印刷問題/解答も同じ座標系を使うため、児童は問題cellに限定されずページ上の方眼へ自由に途中計算を書き込めます。長除法のdecimal normalizationはRust `worked_solution`が唯一のauthorityであり、`dividend_coefficient` / `dividend_scale` / `quotient_trailing_cells`を確定してwireへ渡します。Webはoperand scaleから正規化後の小数桁や追加0を再計算せず、これらの確定値を方眼cell幅・小数点位置・trailing cellへ投影するだけです。長除法は除数と被除数の実桁数に応じたcompact laneを使い、除数・曲線・被除数・商を同じ方眼へ揃えます。割り算記号の曲線と上横線は別borderへ分割せず、1本の連続SVG pathとして描画します。長除法のworked solutionでは日本の一般的な表記に合わせ、各減算段へ余計な`−`記号を付けず、桁配置と横線だけで減算stepを表現します。

`ProblemPrompt::LinearExpression`も共有`LinearExpression` formatterを通り、Web worksheetとprintで同じ括弧・係数・変数surfaceを使う。`problem-format.ts`のsemantic tokenはaccessible plain text等には残しますが、数式のvisual renderingには使いません。

## PDF / 印刷

`src/pdf/worksheet-pdf.tsx`はPDF primitiveを描画せず、Webと同じshared layout/theme metadataから印刷専用の2page A4 React DOMを作ります。

- page 1: 問題。筆算themeでは問題文より下を全面方眼とし、印刷用answer boxは表示せず自由記入
- page 2: 解答。現状は既存の両面印刷仕様により180°rotation。通常向きとの選択可能化は`M-011`で追跡する
- 20問theme: 2列×10行
- 通常の16問theme: 2列×8行
- 加減算・掛け算の筆算16問theme: 4列×4行
- 割り算の筆算12問theme: 4列×3行
- title/instruction: Web ThemeDefinitionと同一
- footer: date / Seed（canonical full `problem_set_id`）
- 数式: `ProblemExpression` / `MathLiveStatic`をWebと共有

MathLive custom elementのrender完了と`document.fonts.ready`を待った後、`window.print()`でブラウザ標準の印刷ダイアログを開きます。PDF保存はブラウザの「PDFに保存」を利用します。この経路により、Chrome等の印刷エンジンが実際のWeb DOM/CSS/fontをPDFへ変換します。

旧`pdf-lib`、`@pdf-lib/fontkit`、`src/pdf/math-renderer.ts`、PDF専用Noto Sans JP shard/mapは削除しました。日本語もWebが通常利用している`@fontsource/noto-sans-jp`をそのまま印刷します。外部font/CDN通信はありません。

印刷moduleは操作時のみdynamic importするため、設定画面のfirst paintへ印刷コードを載せません。印刷操作は直接`window.print()`へ進まず、同じ2page A4 DOMを使うin-app previewを先に表示し、preview内の「印刷する」でnative print/PDFへ進みます。

## Tests

unit testはgenerated contractが列挙する全active themeについて2page print DOMを生成します。筆算ではページ全面方眼classと各問題の方眼lane変数も検証します。通常数式は`math-span`（Webと同じMathLive static element）、筆算は同じ`ColumnArithmeticExpression`へ投影されることを確認します。4×4筆算では16 cell・行優先の問題順・縦区切りなし・筆算本体と答案位置の整合を検証します。解答pageでは加減算の最終結果、掛け算の部分積、割り算の商と途中の掛け算/引き算/余りまでを含む完成した筆算を同じpresentationから描画します。browser acceptanceはsitemapの全routeを複数Seedで生成し、通常worksheetでは各logical cell境界に対するclipping/overlapを確認します。筆算はpage-wide gridが座標authorityなので、不可視のlogical problem cellを越えたこと自体はfailureにせず、page外へのoverflowと同じrowの隣接筆算laneとの実overlapをproblem numberを含む描画範囲で検証します。`worksheet_grid` presentationでは背景方眼の実paint origin/pitchをChromeから取得し、筆算のdigit/rule/answer/problem-numberとMini Sudokuの盤面・全digit cell・problem-numberのedgeが同じgrid lineへ一致することをWeb/print問題面/print解答面で直接検証します。加えて、全active themeの先頭Seedでは各problemの**全editable affordance**（MathLive coordinate、筆算digit slot、長除法remainder、Mini Sudoku cell、liar choice等）を実操作し、MathLiveは複数桁入力まで保持されることを確認します。出現するdistinct input-panel surfaceはenabledな全buttonを自動列挙して実行し、`variable` keyを含む未知のactionを無検証で追加できないようにします。設定・詳細設定・採点設定modal・worksheet editing/graded・print previewは状態グラフとしてaction censusを持ち、設定全option、TOP、採点、問題に戻る、もう一回、print/back/Escape等の全1-step edgeを実Chromeで固定します。非同期gradingのlock/failure復帰はdeferred-engine unit testで補完します。筆算では実際の`.column-digit-answer`を検査対象にし、rule右端との水平整合だけでなく、`answer.top = rule.bottom + workingRows × gridCell`という縦方向のgrid invariantも検証します。staleな旧input selectorで検査がskipされないよう、digit-slot DOMを直接対象にします。さらに筆算previewからactual Chrome `Page.printToPDF`を実行し、2page PDFが生成されることを確認します。

印刷UIのintegration testはPDF moduleをmockせず、主要な到達経路を状態遷移ごとに通します。現在の必須経路は、設定画面→印刷preview、preview→native印刷、worksheet editing→preview→戻る、answer input選択中→preview→戻る、graded worksheet→preview→戻るです。各経路でpreview表示前に`window.print()`が呼ばれないこと、戻った後に元のworksheet状態が維持されることも確認します。

## Security boundary

app codeはuser textを`dangerouslySetInnerHTML`へ渡しません。MathLiveのraw stringは採点authorityではなく、Rust parser/capability validationを通過したAnswerNodeだけが採点へ進みます。

通常のNext deployment向けsecurity headersは設定されていますが、現在のGitHub Pages static exportではNext `headers()`が配信されません。このdeployment差分を含むsecurity方針は[`deployment-security.md`](deployment-security.md)を正とし、既知課題は [GitHub Issues](https://github.com/Vi24E/AutoDrill/issues) で追跡します。
