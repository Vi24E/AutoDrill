# AutoDrill alpha 1.2 実装状況

更新日: 2026-08-13

この文書は現在のコードを対象とした実装概要です。過去の移行履歴ではなく、現行architectureと受入条件を記録します。数学・学習内容の基準は[`curriculum.md`](../curriculum.md)、境界schemaは[`problem-schema.md`](problem-schema.md)、Answer ASTは[`answer-ast.md`](answer-ast.md)、effortは[`effort-model.md`](effort-model.md)、Web/PDFは[`web-pdf.md`](web-pdf.md)を参照してください。

## 実装済みテーマ

numeric theme ID 1〜19の19テーマを実装しています。

| ID | テーマ | 学年 | 問題数 |
|---:|---|---|---:|
| 1 | 一桁の足し算 | 小1 | 20 |
| 2 | 一次方程式(1) | 中1 | 16 |
| 3 | 一次方程式(2) | 中1 | 16 |
| 4 | 一桁の引き算 | 小1 | 20 |
| 5 | 二桁の足し算 | 小2 | 20 |
| 6 | 九九 | 小2 | 20 |
| 7 | 負の数の計算(1) | 中1 | 20 |
| 8 | 負の数の計算(2) | 中1 | 20 |
| 9 | 分数の足し算 | 小5 | 16 |
| 10 | 分数の掛け算 | 小6 | 16 |
| 11 | 分数の引き算 | 小5 | 16 |
| 12 | 分数の割り算 | 小6 | 16 |
| 13 | 割り算(1) | 小3 | 20 |
| 14 | 二次方程式(1) | 中3 | 16 |
| 15 | 二次方程式(2) | 中3 | 16 |
| 16 | 二次方程式(3) | 中3 | 16 |
| 17 | 小数の足し算と引き算 | 小4 | 20 |
| 18 | 小数の掛け算と割り算 | 小5 | 20 |
| 19 | 連立方程式(1) | 中2 | 12 |

現行`CURRICULUM_TREE`は小学1年生〜中学3年生の全9学年を実装済みthemeだけで構成し、placeholder単元は公開しません。

## Source of truth

数学的なsource of truthはRustです。

- `crates/drill-core/src/registry.rs`: numeric theme ID / generator revision / Rust compatibility contract
- `crates/drill-core/src/generator.rs`: generator interfaceと現行generator family
- `crates/drill-core/src/answer.rs`: exact AnswerNode
- `crates/drill-core/src/mathlive_input.rs`: MathLive LaTeX → AnswerNode
- `crates/drill-core/src/normalize.rs`: exact normalization
- `crates/drill-core/src/grade.rs`: grading
- `crates/drill-core/src/effort.rs`: solution graph / operation vector / effort
- `crates/drill-wasm/src/lib.rs`: schema-v3 WASM envelope

Webは数学を再計算せず、WASM DTOをvalidationして表示・入力・PDFへ投影します。

## Web theme構造

テーマ追加時のmetadata同期範囲を狭めるため、alpha 1.2ではWeb themeを`apps/web/src/domain/themes/`へ分割しました。

```text
apps/web/src/domain/themes/
  theme-definition.ts
  one-digit-addition.ts
  one-digit-subtraction.ts
  two-digit-addition.ts
  multiplication-table.ts
  fraction-addition.ts
  fraction-subtraction.ts
  fraction-multiplication.ts
  fraction-division.ts
  division-1.ts
  decimal-add-subtract.ts
  decimal-multiply-divide.ts
  signed-arithmetic-1.ts
  signed-arithmetic-2.ts
  linear-equation-1.ts
  linear-equation-2.ts
  quadratic-equation-1.ts
  quadratic-equation-2.ts
  quadratic-equation-3.ts
  simultaneous-equation-1.ts
```

各ThemeDefinitionはroute、学年・ジャンル、worksheet title/instruction、input interface、answer schema kind、Rust compatibility identityを1ファイルに集約します。`theme-registry.ts`は列挙とlookupのみです。class継承ではなくimmutable typed dataを採用し、theme metadataへlifecycleや数学ロジックを持ち込みません。

## 問題生成

全generatorはSeedとdifficultyに対して決定的です。candidate selectionにはbounded attempt/time budgetを持たせ、Rust/WASM境界でtyped errorにします。

小学生registrationは共通境界で負数をfail closedに拒否します。分数四則は正の値だけを生成します。分数割り算は第2オペランドを逆数にして掛けるモデルで、整数との組合せと整数解も許容します。小数は二進浮動小数点を使わずExactDecimalで生成します。小4の加減は1〜3有効数字・小数第1〜3位、減法は非負。小5の乗除は小数側を1〜2有効数字・小数第1〜2位とし、小4の小数と整数の乗除と、小5の小数同士の乗除を混在させます。除法は有限小数の商から逆生成して循環小数を出しません。

一次方程式は答え先行で候補を構成し、full effortでdifficultyを選択します。二次方程式は平方根帰着・因数分解・解の公式の3テーマを分離します。(1)は解を先に選び、整数解は1〜16、根号解は`√a`（非平方数`2≤a≤30`）から生成し、整数16個と非平方数の根号25個からなる41要素のanswer domainを同等に扱い、難易度差は後段のeffort選抜だけで付けます。(3)は分数係数の分母払いと`±`/根号を含むexact Answer ASTを扱い、簡約後の根号内は99以下です。分数算術は有限domain全候補を直接構築するため、coupon-collector型のSeed依存遅延を起こしません。

## Effort

operation vectorは31次元です（既存29成分の末尾にBaseFractionCancel/BaseRootSquareCancelを追加）。解決weightは`base × grade × theme × mastery`です。

一般のeffort計算は`crates/drill-core/src/effort.rs`へ集約しています。整数四則、分数、GCD/LCM/PF、小数、方程式は共通builderを再利用し、九九・九九逆算・うそつきだれだだけを`crates/drill-core/src/themes/`の各theme moduleへ置く明示的例外とします。GCDは両数の素因数分解と共通因子比較で評価し、旧`OverheadGcdDivisible` shortcutは削除済みです。

可換な単純算術では、`a+b`と`b+a`、`a×b`と`b×a`をworksheet内の同一問題として扱い、両方を同時に選びません。一桁足し算と九九は母集団が小さいため順序違いを別問題として許可します。

3項計算は現時点では導入していません。2項domainでの再測定結果を優先し、それでも5段階の分離が不十分な場合にgenerator domain拡張として検討します。

## MathLive入力

Webのeditable inputはMathLive 0.110.0です。MathLiveがcaret・placeholder・fraction/root layoutを所有し、ReactはMathLive valueをRustへ順序付きで送ります。Rustが承認したAnswerNodeだけを採点stateへ保存します。

`parse_mathlive_answer`はAnswer AST limitに加えて、raw LaTeX長・structure nestingをparse前にbounded checkします。極端に深いpaste/programmatic inputがrecursive parserのstack limitへ到達する前に`answer_ast_size_limit`として拒否します。

## PDF

印刷/PDFには独自の数式rendererを持ちません。`src/pdf/worksheet-pdf.tsx`は印刷用2page A4 DOMを組み、問題式をWebと同じ`ProblemExpression`、canonical answerを同じ`MathLiveStatic`へ渡します。最終PDF化はブラウザ標準のprint/PDF engineが担当します。

- fraction/root/exponent/operator spacing/baselineはMathLiveがWebと同じ規則で組版
- PDF専用のfraction line、minus glyph、baseline、font fittingは存在しない
- 単元title・instructionはWeb ThemeDefinitionと共通
- 日本語はWebと同じNoto Sans JPを使用し、PDF専用font shardやfont embedding codeは持たない
- `pdf-lib` / `@pdf-lib/fontkit`への依存なし
- 印刷ボタンは同じMathLive/A4 DOMの2page previewを先に表示し、「印刷する」でnative print/PDFへ進む
- 20問/16問ともWebと同じshared A4 layout modelをpercentage座標へ投影
- 解答pageの180°rotationは両面印刷向けの既存仕様としてCSSで維持
- 印刷moduleは操作時だけdynamic importし、設定画面のinitial bundleへ入れない
- MathLive editor/static renderer・problem renderer・template iconはq2用dynamic chunkとし、q1のfirst paintをblockしない。productionではidle時と問題生成開始時にpreloadし、worksheetへ切り替える前にchunk readyを待つため、数式だけ遅れて表示されない

## Responsive UI

A4 geometryはPDF側に維持しつつ、Web worksheetの`.paper`は`min(720px, 100%)`へ変更しました。モバイルではviewport幅へ縮み、720px固定canvasの横スクロールを要求しません。内部cell位置はshared A4 modelからpercentageへ変換するため、desktop/PDFの順序は維持します。

## Security

Next.js / ReactはNext 16.3.0 / React 19.2.8へ更新しました。Next responseから`X-Powered-By`を無効化し、以下を全routeへ設定します。

- Content-Security-Policy
- `X-Content-Type-Options: nosniff`
- `X-Frame-Options: DENY`
- strict referrer policy
- camera/microphone/geolocation/payment/USBを無効にするPermissions-Policy

CSPはMathLive/WASM/Next hydrationに必要なlocal inline script/styleとWASM compileを許可しつつ、外部origin、object、frame、base injectionをdenyします。production runtimeはthird-party CDNを必要としません。

## Dependency / build

- Node >=20.9（`.nvmrc`: 22.14.0。現行Macの22.2.0でも起動可能）
- pnpm 10.20.0
- Next.js 16.3.0
- React / React DOM 19.2.8
- ESLint 9 flat config
- MathLive 0.110.0

Next development outputは`.next-dev`、productionは`.next`へ分離します。 dev起動時にはsource/config signatureを比較し、前回起動後にsourceが変更されていれば`.next-dev`だけを自動破棄します。これにより、削除・rename済みmoduleのTurbopack chunk（旧PDF rendererなど）が再起動後も残ることを防ぎます。sourceが変わっていなければcacheは保持します。

## Test policy

少数でも境界価値の高いtestを優先します。

- Rust generator/property: 全themeのdomain、Seed reproducibility、difficulty、timeout/attempt
- MathLive: fraction/nesting/Backspace/11÷1相当/noncanonical/size-limit
- WASM adapter: schema/version/DTO/capability/fail-closed
- 印刷/PDF: **全11テーマ**を同じMathLive static componentで2page print DOMへ投影することを確認
- browser acceptanceではChrome native `Page.printToPDF`でactual 2page PDFを生成し、MathLive shadow render完了・cell clipping/overlapなしを確認
- Web: curriculum、furigana、grading state、mobile layoutはbrowser acceptanceでも確認

## 公開前の標準検証

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
pnpm lint
pnpm typecheck
pnpm test
pnpm contract:check
pnpm build
git diff --check
```
