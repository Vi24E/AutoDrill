# AutoDrill alpha 1.2 実装状況

更新日: 2026-08-15

この文書は現在のコードを対象とした実装概要です。過去の移行履歴ではなく、現行architectureと受入条件を記録します。数学・学習内容の基準は[`curriculum.md`](../curriculum.md)、境界schemaは[`problem-schema.md`](problem-schema.md)、Answer ASTは[`answer-ast.md`](answer-ast.md)、effortは[`effort-model.md`](effort-model.md)、Web/PDFは[`web-pdf.md`](web-pdf.md)を参照してください。

## 実装済みテーマ

公開Web registryには37テーマを実装しています。numeric theme IDは互換性識別子であり連番性は要求しません。

| ID | rev | テーマ | 学年 | 問題数 |
|---:|---:|---|---|---:|
| 1 | 5 | 一桁の足し算 | 小1 | 20 |
| 2 | 8 | 一次方程式(1) | 中1 | 16 |
| 3 | 8 | 一次方程式(2) | 中1 | 16 |
| 4 | 3 | 一桁の引き算 | 小1 | 20 |
| 5 | 3 | 二桁の足し算 | 小2 | 20 |
| 6 | 3 | 九九 | 小2 | 20 |
| 7 | 3 | 負の数の計算(1) | 中1 | 20 |
| 8 | 3 | 負の数の計算(2) | 中1 | 20 |
| 9 | 4 | 分数の足し算 | 小5 | 16 |
| 10 | 4 | 分数の掛け算 | 小6 | 16 |
| 11 | 4 | 分数の引き算 | 小5 | 16 |
| 12 | 4 | 分数の割り算 | 小6 | 16 |
| 13 | 3 | 割り算(1) | 小3 | 20 |
| 14 | 3 | 二次方程式(1) | 中3 | 16 |
| 15 | 4 | 二次方程式(2) | 中3 | 16 |
| 16 | 3 | 二次方程式(3) | 中3 | 16 |
| 17 | 5 | 小数の足し算と引き算 | 小4 | 20 |
| 18 | 6 | 小数の掛け算 | 小5 | 20 |
| 19 | 3 | 連立方程式(1) | 中2 | 12 |
| 20 | 4 | うそつきだれだ | おまけ | 6 |
| 21 | 1 | 分数と整数の掛け算 | 小6 | 16 |
| 22 | 1 | 分数と整数の割り算 | 小6 | 16 |
| 23 | 1 | 分数総まとめ(仮分数) | 小6 | 16 |
| 24 | 1 | 小数の割り算 | 小5 | 20 |
| 25 | 1 | 二桁の足し算の筆算 | 小2 | 16 |
| 26 | 1 | 二桁の引き算の筆算 | 小2 | 16 |
| 27 | 1 | 三・四桁の足し算の筆算 | 小3 | 16 |
| 28 | 1 | 三・四桁の引き算の筆算 | 小3 | 16 |
| 29 | 1 | 一桁をかける掛け算の筆算 | 小3 | 16 |
| 30 | 1 | 二桁をかける掛け算の筆算 | 小3 | 16 |
| 31 | 1 | 一桁で割る割り算の筆算 | 小3 | 12 |
| 32 | 1 | 二桁で割る割り算の筆算 | 小4 | 12 |
| 33 | 1 | 小数の足し算と引き算の筆算 | 小4 | 16 |
| 34 | 1 | 小数と整数の掛け算の筆算 | 小4 | 16 |
| 35 | 1 | 小数と整数の割り算の筆算 | 小4 | 12 |
| 36 | 1 | 小数の掛け算の筆算 | 小5 | 16 |
| 37 | 1 | 小数の割り算の筆算 | 小5 | 12 |

ID 18 revision 5の旧「小数の掛け算と割り算」は公開registryから外していますが、historical registration/generatorを残しているため既存`problem_set_id`から決定的に再生成できます。ID 18 revision 6は「小数の掛け算」、ID 24 revision 1は「小数の割り算」です。

同様に、ID 9〜12 revision 3の旧・仮分数ベース単独テーマも公開registryから外したhistorical generatorとして保持します。active revision 4は通常カリキュラム向けの帯分数版ですが、既存revision 3の`problem_set_id`は旧input interface（Fraction/Decimal）・旧仮分数canonical answer・旧finite domainで再生成できます。`分数総まとめ(仮分数)`はこの互換層とは別のactive復習テーマです。

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
- `crates/drill-wasm/src/lib.rs`: schema-v4 WASM envelope

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
  fraction-integer-multiplication.ts
  fraction-integer-division.ts
  fraction-summary-improper.ts
  division-1.ts
  decimal-add-subtract.ts
  decimal-multiplication.ts
  decimal-division.ts
  signed-arithmetic-1.ts
  signed-arithmetic-2.ts
  linear-equation-1.ts
  linear-equation-2.ts
  quadratic-equation-1.ts
  quadratic-equation-2.ts
  quadratic-equation-3.ts
  simultaneous-equation-1.ts
  column-add-two-digit.ts
  column-subtract-two-digit.ts
  column-add-three-four-digit.ts
  column-subtract-three-four-digit.ts
  column-multiply-one-digit.ts
  column-multiply-two-digit.ts
  column-divide-one-digit.ts
  column-divide-two-digit.ts
  column-decimal-add-subtract.ts
  column-decimal-multiply-integer.ts
  column-decimal-divide-integer.ts
  column-decimal-multiplication.ts
  column-decimal-division.ts
```

各ThemeDefinitionはroute、学年・ジャンル、worksheet title/instruction、input interface、answer schema kind、Rust compatibility identityを1ファイルに集約します。`theme-registry.ts`は列挙とlookupのみです。class継承ではなくimmutable typed dataを採用し、theme metadataへlifecycleや数学ロジックを持ち込みません。

Theme分類のsource of truthは型付き`tags`と`grade`です。`gradeGenre` / `recommendedGenre`はthemeファイルへ手入力せず、`theme-definition.ts`でtagsから導出するread-only projectionです。学年tag（`grade_1`〜`grade_6`, `junior_high_1`〜`junior_high_3`）も`grade`から導出します。筆算themeは演算/小数tagに加えて`column_arithmetic`と`print_recommended`を持ち、UIの印刷推奨表示は後者だけを参照します。詳細は[`theme-taxonomy.md`](theme-taxonomy.md)を参照してください。

## 問題生成

全generatorはSeedとdifficultyに対して決定的です。candidate selectionにはbounded attempt/time budgetを持たせ、Rust/WASM境界でtyped errorにします。

小学生registrationは共通境界で負数をfail closedに拒否します。分数テーマは正の値だけを生成し、共通operand母集団は既約な正の非整数分数`numerator + denominator <= 15`です。通常の分数単元では仮分数値を帯分数として表示・canonical answer化し、数学的に同値な仮分数入力は正解のまま`MixedFractionFormRequired` warningを返します。分数×分数・分数÷分数と、分数と整数の乗除は別単元です。`分数総まとめ(仮分数)`だけは仮分数表記を維持し、乗除では整数operandも自然に混在させます。有限domainは足し算1,071、引き算244、掛け算291、標準分数÷分数750、分数×整数226、分数/整数除算318、総まとめ7,756候補です。

小数は二進浮動小数点を使わずExactDecimalで生成します。小4の加減は1〜3有効数字・小数第1〜3位、減法は非負で、20問をAddition/Subtractionの10問ずつにlayered samplingします。小5の掛け算と割り算は独立テーマです。旧ID 18 rev5のcombined generatorはhistorical regeneration専用で残します。除法は有限小数の商から逆生成して循環小数を出しません。

筆算はID 25〜37の13テーマです。加減算・掛け算は16問・4×4、完成した長除法を問題側と同じ可読サイズで載せる必要がある割り算4テーマ（ID 31, 32, 35, 37）は12問・4×3で、同じ計算を横式で解く既存themeとは別registrationです。整数加減は小2の2桁、小3の3〜4桁へ分離し、乗法は1桁/2桁の乗数、除法は1桁/2桁の除数で分離します。整数除法は商と余りから被除数を逆生成し、最終答案を常に`OrderedPair(商, 余り)`として採点します。小数は小4の加減・小数×整数・小数÷整数と、小5の小数×小数・小数÷小数へ分離します。小数÷小数の筆算は有限小数を逆生成し、除数が小数第1位なら商も小数第1位（最大2有効数字）、除数が小数第2位なら商を1桁整数にして、元の縦式が6方眼以内に収まる母集団を直接生成します。紙面へ途中計算を書けることを主目的とし、Webでは最終答案だけを入力します。

負の数の計算(2)は2〜4 leafのASTを使い、中間値をbounded rationalとして許容します。各中間結果は`|numerator| <= 729, denominator <= 81`、最終答えは`|numerator| <= 200, denominator <= 36`です。5,000 accepted candidatesの実測ではdivision node比率が旧8.5%から25.22%へ改善し、整数答えと分数答えの両方を生成します。

一次方程式は答え先行で候補を構成し、full effortでdifficultyを選択します。二次方程式は平方根帰着・因数分解・解の公式の3テーマを分離します。(1)は解を先に選び、整数解は1〜16、根号解は`√a`（非平方数`2≤a≤30`）から生成します。(2)は平方差・完全平方・一般因数分解をlayer化し、16問では2/2/12（20問なら2/2/16）を保証します。重解を生成し、教材上冗長だった方程式全体のscaleは1へ固定しました。(3)は分数係数の分母払いと`±`/根号を含むexact Answer ASTを扱い、embedded `PlusMinus`を最大4 branchのexact solution setへ展開できるため、canonical `±`と明示Tupleの2解を同値採点できます。浮動小数比較は使いません。

## Layered sampling

layered themeは、教材上同じworksheet内で各アーキタイプを一定量保証したい場合だけ使用します。一般のvector diversity最大化ではありません。

- theme/generatorは内部`SamplingLayerSpec { key, weight, minimum }`を宣言する
- quotaは各layerの`minimum`を先に確保し、残数をweightのlargest-remainder方式で配分する
- candidateをlayerへ分類し、各layer内部で既存scalar effort samplerをそのまま適用する
- randomもquotaを守り、layer内部だけrandom samplingする
- layer metadataは公開Worksheet schemaへ露出しない
- non-layered themeのsampling挙動は変更しない

現行layered themeは、小数の足し算/引き算、小数の足し算と引き算の筆算、分数総まとめ(仮分数)、二次方程式(2)です。`うそつきだれだ`はlayered themeではなく、easyで3人、hardで4人へ偏る既存挙動を意図的に維持します。

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
- 筆算themeは行優先で配置し、加減算・掛け算は16問を4列×4行、割り算は12問を4列×3行とする。セル間の縦区切り線は表示しない。Web/PDF共通の`ColumnArithmeticExpression`で縦式・小数点位置・割り算の長除法枠を表示
- 全筆算themeは共通の**ページ方眼**を使う。問題文より下の書き込み領域全体へA4比率に連動する正方形gridを薄く表示し、operand・演算記号・途中計算・Web入力・印刷問題・印刷解答を同じ方眼座標へsnapする。`ch`やproblemごとの独立grid、個別の`top/right/%`微調整は使わない。小数点は1cellを消費せず、隣接する桁cellの境界（グリッド交点）へ黒点として描画する
- 問題pageの筆算cellは数字と主線、Web答案位置をページ方眼へ一致させる。掛け算の未解答問題は乗数直下の主線1本だけとし、余計な最終横線を置かない。印刷問題pageでは余計な答案枠（割り算の「あまり」欄を含む）を置かず、ページ全面の方眼へ自由に途中計算を書き込める
- 印刷解答pageは最終答だけを別枠表示せず、完成した筆算を表示する。加減算は横線下の結果、二桁掛け算は部分積と最終結果、割り算は商・途中の掛け算/引き算・桁下ろし・最終余りまでを同じ桁グリッド内に描画する
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
- 印刷/PDF: **全37公開テーマ**をshared `ProblemExpression`で2page print DOMへ投影することを確認。通常数式はMathLive、筆算はWeb/PDF共通の筆算presentationを使う
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
