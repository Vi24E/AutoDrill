> **Historical document:** この文書は履歴保存用であり、現行仕様のsource of truthではありません。現在の設計は `docs/principles.md` / `docs/architecture/`、未解決事項は `docs/issues.md` を参照してください。

# Theme taxonomy / tag設計

更新日: 2026-08-15

## 目的

Themeの分類情報をUIごとに重複入力せず、theme追加時に必要なmetadataを1か所で把握できるようにする。Webのsource of truthは`apps/web/src/domain/themes/<theme>.ts`の`grade`と型付き`tags`であり、Rustは数学・互換性・worksheet layoutのsource of truthを維持する。

## Stored metadata

`ThemeTag`は`theme-definition.ts`のliteral unionからのみ選べる。任意stringを許さないためtypoはTypeScriptで検出される。

現行tagは次の分類だけを持つ。

- 教材カテゴリ: `addition`, `subtraction`, `multiplication`, `division`, `fractions`, `decimals`, `negative_numbers`, `equations`
- 方程式subtype: `linear_equation`, `simultaneous_equation`, `quadratic_equation`
- 特殊分類: `bonus`
- presentation / usage: `column_arithmetic`, `print_recommended`

不要な将来tagを先行定義しない。`interactive_friendly`, `review`, `special`等が実際に必要になった時点でunionと導出規則へ追加する。

## Derived metadata

学年は既存`grade`がsource of truthである。`taxonomyTags(theme)`はそこから`grade_1`〜`grade_6`, `junior_high_1`〜`junior_high_3`を導出するため、themeファイルへ学年を二重記述しない。

既存callerとの互換性のため`ThemeDefinition`には`gradeGenre` / `recommendedGenre` projectionを残すが、themeファイルでは指定しない。`defineTheme()`がtagsから中央集約的に導出する。

導出優先順位は次の通り。

- 学年別genre: fractions → decimals → negative numbers → equation subtype → addition/subtraction → multiplication/division
- おすすめgenre: bonus → equations → negative numbers → fractions → decimals → addition/subtraction → multiplication/division

例えば`tags: ['decimals', 'addition', 'subtraction']`は学年別・おすすめとも「小数」へ入り、`tags: ['addition']`は「足し算と引き算」へ入る。表示順そのものはregistry順を維持し、taxonomy導入によって既存UIの順序を変更しない。

## 筆算

全筆算themeは演算カテゴリtagに加えて次を持つ。

- `column_arithmetic`: 縦式presentationを使う
- `print_recommended`: 印刷して紙へ途中計算を書き込む利用を推奨する

小数筆算はさらに`decimals`を持つ。設定画面の

`この問題は紙に印刷して解くことをおすすめします。`

は`print_recommended`だけで判定し、theme ID/nameのhard-coded listを持たない。

## 責務境界

- Rust registration: numeric theme ID, generator revision, skill ID, curriculum path, problem count, rows/columns
- Rust Problem DTO: 数学的prompt、canonical answer、grading schema、effort
- Web ThemeDefinition: route/search copy、表示名、型付きtaxonomy tag、grade
- UI curriculum projection: ThemeDefinitionのderived metadataを利用

同じ意味をRust/Web双方へ重複させない。Rustのgrade/curriculum pathをWeb contractから参照できる情報はcompatibility fieldへ投影し、UI taxonomyはWeb theme definitionで一元化する。
