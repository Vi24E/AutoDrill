# Web / PDF boundary (alpha 1.2)

## Theme registry

Webの実装済みthemeは`apps/web/src/domain/themes/`で1テーマ1ファイルとして定義します。各`ThemeDefinition`がroute、学年・ジャンル、worksheet表示、input capability、Rust compatibility identityを所有し、`src/domain/theme-registry.ts`は列挙とlookupだけを担当します。

`curriculum.ts`、単元route、sitemap、worksheet copy、layout、WASM response validationは同じdefinitionを参照します。全学年の公開カリキュラムは実装済みthemeだけで構成します。

## q1 / q2 / q3

- q1: 設定画面。おすすめまたは学年からthemeを選択し、difficulty/Seedを指定する。
- q2: Web回答画面。MathLiveで入力し、Rust/WASMへ解析・採点を委譲する。
- q3: browser内で生成したPDFを別tabへ開く。

空Seedはbutton click時に自動生成され、q2/PDF metadataは同じresolved Seedを保持します。

## WASM adapter

`src/domain/wasm-adapter.ts`がproductionの数学境界です。schema-v3 JSON DTOを使い、以下をRust/WASMへ委譲します。

- worksheet generation
- MathLive LaTeX → AnswerNode
- legacy typed editor action
- grading

Webはnormalization、正誤判定、effort、generator条件を再実装しません。

`i64`/`u64`のexact payloadはcanonical decimal stringでJSON越境し、JavaScript `number`へ落としません。effort/vectorの評価値のみ有限`number`を許可します。

## MathLive

MathLiveはcaret、placeholder、fraction/root layout、editable renderingを所有します。各input snapshotはFIFO queueでRustの`parse_mathlive_answer`へ送り、Rustが承認したAnswerNodeのみstateへ保存します。

Rust parserはAnswer AST sizeに加え、raw LaTeX長・structure nestingをparse前に検査します。過度に深いinputはrecursive parserへ入る前に`answer_ast_size_limit`として拒否します。

## Responsive worksheet

PDFは固定A4 geometryを使いますが、Web回答画面は固定720px canvasを要求しません。

```css
.paper {
  width: min(720px, 100%);
  min-width: 0;
}
```

cellはshared A4 modelをpercentage座標へ変換するため、viewportが狭い場合も同じ問題順を保ったまま縮みます。入力panelはfixed bottomのままsafe-areaを考慮します。

## Shared math formatting

数式のsourceはRust DTOから`mathlive-format.ts`が作るLaTeXです。Web表示と印刷/PDFは別々の数式rendererを持たず、どちらもMathLive 0.110.0へ同じLaTeXを渡します。

Webでは`ProblemExpression` / `MathLiveStatic`が`<math-span>`を使います。印刷用DOMでも**同じReact componentをそのまま再利用**します。したがってfraction、root、exponent、括弧、operator spacing、baseline等はMathLiveが一元的に所有し、PDF側にfraction lineやminus記号の座標実装はありません。将来expression ASTが複雑化しても、MathLiveが扱えるLaTeXなら印刷側の追加実装は不要です。

`problem-format.ts`のsemantic tokenはaccessible plain text等には残しますが、数式のvisual renderingには使いません。

## PDF / 印刷

`src/pdf/worksheet-pdf.tsx`はPDF primitiveを描画せず、Webと同じshared layout/theme metadataから印刷専用の2page A4 React DOMを作ります。

- page 1: 問題、Webと同系統のanswer box
- page 2: 解答。既存の両面印刷仕様により180°rotation
- 20問theme: 2列×10行
- 16問theme: 2列×8行
- title/instruction: Web ThemeDefinitionと同一
- footer: date / Seed
- 数式: `ProblemExpression` / `MathLiveStatic`をWebと共有

MathLive custom elementのrender完了と`document.fonts.ready`を待った後、`window.print()`でブラウザ標準の印刷ダイアログを開きます。PDF保存はブラウザの「PDFに保存」を利用します。この経路により、Chrome等の印刷エンジンが実際のWeb DOM/CSS/fontをPDFへ変換します。

旧`pdf-lib`、`@pdf-lib/fontkit`、`src/pdf/math-renderer.ts`、PDF専用Noto Sans JP shard/mapは削除しました。日本語もWebが通常利用している`@fontsource/noto-sans-jp`をそのまま印刷します。外部font/CDN通信はありません。

印刷moduleは操作時のみdynamic importするため、設定画面のfirst paintへ印刷コードを載せません。印刷操作は直接`window.print()`へ進まず、同じ2page A4 DOMを使うin-app previewを先に表示し、preview内の「印刷する」でnative print/PDFへ進みます。

## Tests

unit testは全19 registered themeについて2page print DOMを生成し、各問題式・解答が`math-span`（Webと同じMathLive static element）へ投影されることを確認します。fraction代表例は`1/3 + 1/4`がslash textへflattenされずLaTeXのままMathLiveへ渡ることも確認します。browser acceptanceではactual Chromeで`Page.printToPDF`を実行し、2page PDF、全MathLive shadow render完了、数式とanswer boxのoverlap/clippingなしを確認します。

印刷UIのintegration testはPDF moduleをmockせず、主要な到達経路を状態遷移ごとに通します。現在の必須経路は、設定画面→印刷preview、preview→native印刷、worksheet editing→preview→戻る、answer input選択中→preview→戻る、graded worksheet→preview→戻るです。各経路でpreview表示前に`window.print()`が呼ばれないこと、戻った後に元のworksheet状態が維持されることも確認します。

## Security boundary

Next responseにはCSPと基本security headersを設定します。production CSPはresource originを`self`中心に制限し、object/frame/base injectionをdenyします。MathLive/Next hydration/WASMに必要なinline script/styleと`wasm-unsafe-eval`だけを許可します。

app codeはuser textを`dangerouslySetInnerHTML`へ渡しません。MathLiveのraw stringは採点authorityではなく、Rust parser/capability validationを通過したAnswerNodeだけが採点へ進みます。
