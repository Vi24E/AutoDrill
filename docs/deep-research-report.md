# AutoDrill の structural singularity 推定：分野横断で実装可能な設計

## 結論と設計原則

本調査の結論は、現在の中心仮説

> **「低複雑度の関係が、偶然にしては目立つ形で成立しているほど singularity が高い」**

は、AutoDrill の中核仮説としてかなり有望です。ただし、そのままでは一つ重要な要素が欠けています。より妥当な作業仮説は、

> **structural singularity ≈「表現上アクセスしやすい低記述長の関係」が「その文脈では統計的に珍しく」成立し、しかも「問題中で役割的に目立つ」度合い**

です。

「単純なパターンが偶然にしては出来すぎて見える」という考えには、認知科学でかなり直接的な先行研究があります。Feldman は、単純なパターンの「驚き」を、無作為な背景の下でその単純さがどの程度異例かという形で定量化しています。これは本件の「simple relation × rarity」に非常に近い発想です。Chater–Vitányi 系の simplicity principle や、Griffiths–Tenenbaum の coincidence のベイズ的分析も、単純な代替説明が低い事前確率を克服するほどデータに支持されると「偶然以上」に感じられる、という構図を支持します。citeturn14search0turn14search1turn14search4

さらに、surprise は注意を引きつけます。Itti–Baldi の実験では Bayesian surprise の高い位置に視線が偏り、反復・対称性・prototypicality などは processing fluency と関連することが報告されています。数についても、数値推定時に round number へ回答が集中する round-number bias が実験的に観察されています。したがって、`100`、同じ桁、対称性、単純比などに特別な「知覚・表記上のアクセスしやすさ」を与えることには心理学的な根拠があります。citeturn14search2turn19search1turn19search0turn19search2

ただし、**「低 Kolmogorov complexity なら singularity」ではありません**。何が短い記述なのかは採用した表現言語によって変わるからです。`x-y≡0 (mod 100)` は十進表記では心理的に自然ですが、`x-y≡0 (mod 97)` を同じ複雑度として扱うのは不自然でしょう。したがって AutoDrill では、万能な algorithmic complexity を目指すのではなく、**十進表記・算数教育・式の役割構造に grounded した有限 DSL の description length** を使うべきです。MDL の基本思想である「モデル自体の複雑さを含めて短い説明を好む」という方向は適合しますが、実際の符号長は AutoDrill 用に設計する必要があります。citeturn18search0turn18search12

また、**k=2 から始める方針は妥当ですが、「二項関係だけ」に固定するのは推奨しません**。正確には、

> **unary + binary を core language とし、意味的に正当化された derived node を許す。k=3 は shadow language に置き、残差から必要性を検証する**

のがよいです。NIST の combinatorial testing 研究でも、実際の故障は少数パラメータの interaction で生じるものが多いという経験則があり、低次数から探索することには一般的な計算上の合理性がありますが、pairwise が完全だという主張ではありません。citeturn16search5turn16search9turn16search17

最終的な推奨構成を一行で表すと、

**typed object graph → 有限 unary/binary relation DSL → MDL prior × conditional rarity → 解釈可能な raw structural score → 弱教師で軽く calibration → residual/contrast QA loop**

です。プログラム検証の観点では、QuickCheck の「生成して性質を試す」、Daikon の「有限な候補 invariant family を観測データに照合する」、CEGIS の「候補→counterexample→refinement」、metamorphic testing の「性質だけを変えた follow-up case」という四つの考え方を組み合わせた構成に近くなります。citeturn18search2turn16search0turn13search2turn18search11

重要なのは、最初から「singularity の真の値」を一本の教師ありモデルで学ぼうとしないことです。実装上は少なくとも次の三層を分けるべきです。

| 層 | 意味 | User QA 依存 |
|---|---|---:|
| `S_struct` | 単純・珍しい・目立つ構造がどの程度あるか | ほぼ無し |
| `S_user` | 現在の User がそれをどの程度 singular と評価するか | 有り |
| `S_population` | 将来の想定児童・生徒母集団での知覚 | 現時点では識別不能 |

この分離が重要なのは、単一の非代表的評価者しかいない現在のデータから、評価者固有の感度と母集団の latent singularity を統計だけで分離することはできないためです。Many-Facet Rasch や Dawid–Skene は複数評価者の bias/error を扱う強力な枠組みですが、一人しかいない評価者から「母集団平均との差」を復元する魔法にはなりません。citeturn15search9turn15search2


## 中心仮説と k≤2 の射程

AutoDrill の singularity は、difficulty と違って「解答に必要な認知操作量」ではなく、**問題を一瞥したときに見つかる偶然らしくない構造**に近いと考えるのがよいでしょう。Feldman の simple-pattern surprise はまさに、「単純な記述を持つ規則性がランダム背景の下ではどれだけ異例か」を扱います。一方、Griffiths–Tenenbaum は coincidence を「現在の説明より別の構造仮説を支持する証拠」としてベイズ的に捉えています。したがって singularity を単なる frequency や単なる description length のどちらか一方に還元するより、**simple alternative と null probability の比較**として扱う方が自然です。citeturn14search0turn14search1turn14search9

ただし、AutoDrill 用にはさらに「relation accessibility」が必要です。人間はすべての数学的真理を同じ速度で列挙しているわけではありません。反復、対称性、類似性、十進的な丸い数などは表現上特にアクセスしやすい一方、同程度の記号長を持つ人工的な合同式や巨大な素数関係は通常それほど目立ちません。Gestalt の perceptual organization、processing fluency、round-number bias は、この「関係言語の prior が均一ではない」ことを支持します。citeturn19search1turn19search2turn19search0

したがって中心仮説は、次の四成分に分解すると扱いやすくなります。

\[
\text{singularity}
\approx
f(
\underbrace{\text{simplicity}}_{\text{短く記述できる}},
\underbrace{\text{rarity}}_{\text{通常は起こりにくい}},
\underbrace{\text{accessibility}}_{\text{人が気づきやすい}},
\underbrace{\text{role prominence}}_{\text{問題中で重要な位置にある}}
).
\]

たとえば `183 - 83` の suffix 一致は十進文字列上アクセスしやすく、`100` という正答も目立ちます。逆に、同じ二数がある巨大な素数 \(q\) を法としてたまたま特殊な関係を持っていても、relation language 内で高コストならほぼ寄与させません。これは「成立している数学的関係を全部探す」のではなく、**人間が発見しそうな短い説明の候補だけを見る**という設計です。これは MDL/simplicity の考え方と、認知的な pattern salience の知見を合わせた本報告の提案です。citeturn14search4turn18search0

**k=2 の評価については、ユーザー仮説を少し修正すべきです。** `33` は二つの `Digit` object に対する `EQ(d₁,d₂)`、`183` と `83` は `COMMON_SUFFIX(x,y,2)` または `CONGRUENT(x,y,100)`、`3x-15=-3x+15` は左右 coefficient の `NEG_EQ` と constant の `NEG_EQ` として、確かに binary relation でうまく表せます。さらに answer や normalized coefficient を object として作れば、「答えが表示 literal と同じ」「答えがその負数」といった現象も binary です。

一方、**unary predicate は最初から明示的に許すべきです**。`100` が round、`0` が特別、`777` が repeated-digit、`1221` が palindrome であることを、無理に「特殊定数との二項関係」へ変換しても理論的な利点はありません。むしろ relation language の記述長が歪みます。したがって core は厳密には **arity ≤ 2** とするのがよいです。

また、k=3 が本当に必要になる代表例は存在します。

| パターン | primitive object 間の本質的 arity | k=2 で扱える条件 |
|---|---:|---|
| `14, 17, 20` が等差数列 | 3 | `midpoint(14,20)=17` を derived node として許せば 2 |
| `a+b=c` | 3 | `a+b` が問題 AST の実在 node なら、その node と `c` の binary relation にできる |
| `3,4,5` の Pythagorean relation | 3 | `a²+b²` を意味的 derived node にしてよい場合のみ |
| 三つの係数が `a+b+c=0` | 3 | 任意 pair closure を許さないなら本質的に 3 |
| 三角形三角の和が `180°` | 3 | 幾何意味論として「angle sum」node が標準なら 2 に compile 可 |

ここで最重要なのは、**「derived quantity を自由に作れば、どんな k=3 も k=2 にできる」という逃げ道を禁止すること**です。任意の pair \((x,y)\) ごとに \(x+y, x-y, xy,\ldots\) を再帰的に作れば、形式上は binary でも探索空間は高 arity と同じように爆発し、spurious relation が大量に発生します。

したがって derived quantity は、

**問題の意味・解法正規形・既存 AST から一意にライセンスされるものだけ**

に制限します。たとえば一次方程式なら「正規化した LHS coefficient」「RHS constant」「solution」は自然ですが、「任意の二 literal の積」は、そのテーマで意味を持たなければ core object にしません。有限 template から likely invariant を検査する Daikon や、syntax template に探索空間を制限する program synthesis の考え方は、この種の探索空間設計が有効であることを示すよい類似例です。citeturn16search32turn13search22

したがって **k=2 から始めること自体は支持しますが、「k=2 仮説を真と仮定する」のではなく「k=3 を残差で検定する」設計にすべき**です。これは combinatorial testing の「低次数 interaction を先に調べ、必要に応じて t を上げる」という発想とも整合します。citeturn16search9turn16search17

教育研究からも、surface と structure の双方を保持すべき理由があります。Chi–Feltovich–Glaser の古典的研究では、専門家と初学者で問題の分類に用いる特徴が異なり、専門家は深い原理、初学者は literal/surface feature により依存する傾向が観察されています。これは物理問題についての結果であり AutoDrill singularity を直接測ったものではありませんが、**単一の「数学的構造表現」だけでは児童・生徒の salience を代用できない**ことを示唆します。数値としての `183` と文字列 `"183"` を同時に保持するというユーザー案は、この点で非常に妥当です。citeturn14search7turn14search19


## 関係言語の具体案

最初に実装すべき representation は、単なる「数の袋」ではなく **typed, role-aware object graph** です。

各問題 \(p\) から、

\[
V(p)=V_{\text{surface}}\cup V_{\text{semantic}}
\]

を作ります。

**`surface` 側**には、表示数値、符号付き数値 token、numeral string、digit token、分数の numerator/denominator、decimal digits などを置きます。**`semantic` 側**には、正答、正規化係数、定数項、既存 AST subexpression の値、問題意味から一意に定まる主要量だけを置きます。同じ `183` について `Num(183)` と `DigitSeq("183")` を両方持つことは意図的です。一方、「183 の十進 digits が `"183"`」という定義的変換自体には singularity point を与えません。

推奨型は最低限、次です。

| 型 | 例 | 備考 |
|---|---|---|
| `ExactNum` | `183`, `-3`, `1/4`, `0.25` | 内部は exact rational。float は避ける |
| `Digit` | `'1'`, `'8'`, `'3'` | occurrence を区別 |
| `DigitSeq` | `"183"` | leading/trailing zero 等の surface 情報保持 |
| `SemanticNum` | solution, normalized coefficient | provenance を必須にする |
| `Role` | `lhs_coeff`, `rhs_const`, `answer` | 値ではなく metadata |
| `ASTNode` | `3x-15` 等 | 全テーマ共通の少数 structural metadata のみ |

このような type restriction を使うと、候補 relation の大半を静的に排除できます。Daikon でも変数型・適用可能な invariant family に基づいて候補を絞るという考え方が採られており、AutoDrill でも「無限 symbolic search」ではなく「typed finite catalog」が適しています。citeturn16search0turn16search28

**core unary family** は、k=2 仮説への例外として最初から入れるべきです。

| family | 例 | 初期 complexity |
|---|---|---|
| `IS_ZERO(x)` | answer = 0 | 最小 |
| `IS_UNIT(x)` | ±1 | 小 |
| `IS_SMALL_INT(x;q)` | 2,3,… | q の符号長を課す |
| `IS_POWER10(x)` | 10,100,1000 | 小 |
| `IS_ROUND_DECIMAL(x,d)` | 100, 2500 等 | 小〜中 |
| `REPEATED_DIGIT(s)` | `"33"`, `"777"` | 小 |
| `PALINDROME(s)` | `"121"` | 小〜中 |
| `CONSECUTIVE_DIGITS(s)` | `"123"` | 中 |
| `IS_SQUARE(x)` | 25, 49 | 中 |

0、反復、対称性、round number を低コスト側に置くことには、processing fluency、Gestalt grouping、round-number bias から一定の心理学的根拠があります。ただし「児童の singularity rating がそのままこれらに比例する」とまでは先行研究は言っていないため、family weight の最終調整は QA で行うべきです。citeturn19search0turn19search1turn19search2

**core binary numeric family** は以下から始めることを推奨します。

| relation | 例 |
|---|---|
| `EQ(x,y)` | `3,3` |
| `NEG_EQ(x,y)` | `3,-3` |
| `ABS_EQ(x,y)` | `3,-3` |
| `DIFF(x,y;q)` | \(x-y=q\), q は低複雑度定数のみ |
| `RATIO(x,y;q)` | \(x/y=q\), q は小整数・単純有理数 |
| `DIVIDES(x,y)` | \(x\mid y\) |
| `SAME_PARITY(x,y)` | 必要なテーマのみ |
| `SQUARE_OF(x,y)` | \(x=y^2\) |
| `POWER10_RATIO(x,y;d)` | \(x/y=10^d\) |
| `CONGRUENT10(x,y;d)` | \(x\equiv y\pmod{10^d}\) |
| `ANSWER_EQ_LITERAL(a,x)` | 答えが表示値に一致 |
| `ANSWER_NEG_LITERAL(a,x)` | 答えが表示値の負数 |

特に `CONGRUENT10` は一般の `CONGRUENT_MOD_m` より prior を大きくすべきです。十進 notation から直接生まれるからです。一般 modulus を core に入れる場合でも、`m=10^d` と `m=97` に同じ code length を与えるべきではありません。

**binary string/digit family** は、数値 relation と別 family にします。

| relation | 例 |
|---|---|
| `DIGIT_EQ(d_i,d_j)` | `33` |
| `SEQ_EQ(s,t)` | 同じ numeral sequence |
| `PREFIX(s,t,k)` | 先頭 k 桁一致 |
| `SUFFIX(s,t,k)` | `183`, `83` |
| `IS_PREFIX_OF(s,t)` | `"18"` と `"183"` |
| `IS_SUFFIX_OF(s,t)` | `"83"` と `"183"` |
| `REVERSE(s,t)` | `"12"` と `"21"` |
| `HAMMING_SMALL(s,t,k)` | 必要性が出てから shadow → core |

`183` と `83` は数値側で `CONGRUENT10(...,2)`、文字列側で `SUFFIX(...,2)` の両方に引っかかります。これは本来同じ十進構造を二重に見ているため、**両方を独立証拠として足し算してはいけません**。relation graph に「semantic equivalence / implication」を持たせ、同一 evidence family に束ねる必要があります。

同様に、

\[
\text{suffix}_3 \Rightarrow \text{suffix}_2 \Rightarrow \text{suffix}_1
\]

なので三つを全部数えると singularity を過大評価します。実装上は、各 implication chain について

*最も specific な成立 relation だけ採用する*

か、

\[
-\log P(\text{suffix}_2 \mid \text{suffix}_1)
\]

のような **incremental evidence** にするのがよいです。最小実装では前者で十分です。

一次方程式などでは **role-aware family** が重要です。たとえば

\[
3x-15=-3x+15
\]

について単に全 literal pair を見るだけでなく、

\[
R((\text{lhs\_coeff},3),(\text{rhs\_coeff},-3))
\]

と

\[
R((\text{lhs\_const},-15),(\text{rhs\_const},15))
\]

を区別します。同じ `3,-3` でも、左右対称 role に置かれている方がはるかに構造的です。Chi らの expert/novice 研究を踏まえても surface と structural role を別々に保持し、後からどちらが評価に効くかを見る方が安全です。citeturn14search19

**定数生成規則**は singularity system の成否を左右します。最も危険なのは、観測した pair から都合のよい定数を後付けすることです。たとえば任意の \(x,y\) に対し \(c=x-y\) を無料で作れば、すべての pair が `x-y=c` を満たし、relation discovery が無意味になります。

したがって定数は三種類に限定します。

1. **universal constants**：`0, ±1`、ごく小さい整数、単純分数など。
2. **notation constants**：`10^d`、decimal position など、表示体系から自然に生じるもの。
3. **theme-licensed constants**：180°、60分、100% など、そのテーマ意味論が明示的に許すもの。

それ以外のパラメータ \(q\) を使う relation には、その整数・有理数自身の description cost を必ず課します。MDL 的な complexity penalty を使う理由はここにあります。citeturn18search0

relation の複雑度は、理想化すると

\[
L(r)=
L_{\text{operator}}
+L_{\text{parameter}}
+L_{\text{representation}}
+L_{\text{role}}
+L_{\text{composition}}
\]

とします。

たとえば `EQ(Digit,Digit)` は非常に短く、`SUFFIX(...,2)` も短い。`mod 100` は decimal representation に自然なので短め、`mod 97` は parameter cost が大きい。numeric → digit string 変換には小さな conversion cost を付け、複数変換や二段 composition にはさらに cost を足します。

最初からこれらの bit cost を User QA で細かく学習するのは推奨しません。100件ではパラメータ自由度が高すぎます。**最初は 5～10 段階程度の手設計 code-length class** にし、User QA は family group の補正程度に使う方が安定します。

探索アルゴリズム自体は単純で構いません。問題あたり object 数を \(m\)、relation template 数を \(R\) とすれば、素朴な typed evaluation でもおおむね

\[
O(Rm^2)
\]

です。対象が数十 object、数百 template 程度なら、symbolic regression を毎問回す必要はありません。Daikon の有限 invariant family や template-based synthesis は、まさに「言語を適切に制限すれば exhaustive-ish な検査が実用的になる」という設計例です。citeturn16search28turn13search22

一方、Schmidt–Lipson 型の symbolic regression は自由形式の自然法則をデータから発見できる強力な方法ですが、本件の core engine としては探索自由度が過大です。**残差ケースに対する shadow search** に限定して使う方がよいでしょう。citeturn18search1turn18search5


## singularity 統計量と推奨アーキテクチャ

単純さと珍しさを統合する最もよい第一案は、**MDL prior を持つ mixture likelihood-ratio / Bayes-factor-like score** です。

問題 \(p\) に対する relation instance を

\[
h=(r,i,j)
\]

とします。\(r\) は relation、\(i,j\) は適用対象 object です。

relation の prior mass を

\[
\pi(r)\propto 2^{-L(r)}
\]

とします。これは「短く記述できる relation に大きい prior」を与える実装です。Rissanen の MDL はモデルの記述長を model selection に組み込む原理を与え、Griffiths–Tenenbaum の coincidence model は「単純な代替構造と null の尤度の比較」が人間の coincidence 判断と関係することを示しています。citeturn18search0turn14search1

ただし、一つの問題に object pair が 100 個あれば、1 pair しかない問題より「何か面白い関係が偶然見つかる」確率が高くなります。そこで relation prior のほかに pair-selection prior

\[
q(i,j\mid r,p)
\]

を入れます。role のない eligible pair なら一様、左右 coefficient のような canonical role pair ならより大きい prior にしてよいでしょう。

文脈 \(c=(\text{theme},\text{difficulty},\text{roles},\text{format},\ldots)\) の通常生成下で relation が成立する確率を

\[
p_0(h\mid c)
\]

とします。

そこで一つの relation instance が成立したときの直感的 evidence は、

\[
E_h
=
-\log p_0(h\mid c)
-L(r)
-\log \frac{1}{q(i,j\mid r,p)}.
\]

つまり、

> **珍しいほど上がる、relation が複雑なほど下がる、候補 pair を大量に探索したほど下がる**

という形です。

さらに理論的にきれいな aggregate として、本報告では

\[
B(p)
=
1-\Pi
+
\sum_{r}
\pi(r)
\sum_{(i,j)\in\mathcal E_r(p)}
q(i,j\mid r,p)
\frac{\mathbf 1[r(i,j)]}
     {p_0(r(i,j)\mid c)}
\]

を推奨します。ここで \(\Pi=\sum_r\pi(r)\le1\) です。

null probability が正しく推定されていれば、

\[
\mathbb E_0[B(p)]=1
\]

になります。各 relation が独立である必要すらありません。これは「大量に候補を探せば何か当たる」という look-elsewhere effect を、relation prior と pair prior の両方で部分的に補正できます。その上で

\[
S_{\rm raw}(p)=\log B(p)
\]

を machine-level structural singularity evidence とします。これは本報告での設計提案ですが、MDL と coincidence/Bayesian surprise の考え方を直接統合したものです。citeturn18search0turn14search1turn14search30

実装をさらに簡単にしたい場合は、

\[
S_{\rm raw}
=
\operatorname{logsumexp}_{h:r(h)=1}
\left[
-\log p_0(h)-\lambda L(h)
\right]
\]

でも開始できます。ただし pair 数補正と nested relation の重複排除は必須です。

**`p0` を何から推定するかは極めて重要です。** 現在の AutoDrill 出力そのものの empirical frequency だけを使うのは危険です。たとえば生成器のバグで `100` が大量発生すると、その異常が「頻繁だから普通」と自己正当化されてしまうからです。

推奨する null は、

> **theme × difficulty × 値域 × format × unary marginal をできるだけ保持しつつ、値どうしの不要な cross-relation を壊した conditional randomization distribution**

です。

たとえば「二桁引き算」という形式、各 operand の値域、carry/borrow 条件などは保持しながら、下二桁一致のような偶然 relation は再サンプルで自由にさせます。これを大量に自動生成して relation frequency を推定します。参照教材は `p0` の絶対的 truth にはしません。

低頻度 relation の \(p_0\) には単純な生頻度ではなく、Beta–Binomial 的 smoothing や theme cluster 間の partial pooling を使うべきです。そうしないと「まだ一度も観測されなかった relation」が無限 surprisal になります。

この raw structural layer と User label layer は分離します。

最小構成では、

\[
\widehat S_{\rm user}
=
\sigma\left(
\alpha_t
+\beta S_{\rm raw}
+\sum_{g=1}^{G}w_g z_g
\right),
\qquad \beta\ge0,
\]

程度で十分です。\(z_g\) は「digit relation」「roundness」「role symmetry」など、relation family を 5～10 group に集約した residual feature です。\(w_g\) は ridge/shrinkage を強くかけます。`α_t` は時間 drift を吸収する session-level offset です。

これによって、

- core theory は User に過適合しない、
- User が `symmetry` を特に強く見るなら少量の補正が効く、
- 新分野でも `S_raw` は zero-shot で計算できる、

という三つの利点があります。

`3x-15=-3x+15` のような問題では、二つの independent-looking symmetry evidence が同時に成立します。このとき max だけでなく mixture/log-sum-exp を使うことで複数証拠を反映できます。ただし `suffix1` と `suffix2` のような論理包含関係は事前に collapse します。

推奨する全体アーキテクチャは次です。

```text
問題
 ↓
Canonical parser / semantic normalizer
 ↓
Typed object graph
 ├─ numeric values
 ├─ digit / string representation
 ├─ role metadata
 ├─ answer
 └─ licensed semantic derived nodes
 ↓
Core unary/binary relation evaluator
 ↓
Relation implication / equivalence pruning
 ↓
Conditional-null rarity lookup
 ↓
MDL × rarity mixture evidence
 ↓
S_struct
 ↓
Weak-label calibration
 ↓
S_user (0–1)

                 ┌→ residual cases
                 │
                 ├→ shadow grammar / k=3 search
                 │
                 ├→ matched counterfactual generation
                 │
                 └→ active User QA → validated relation promotion
```

この side loop は CEGIS にかなり近い構造です。CEGIS では candidate solution を検証し、失敗時の counterexample を次の synthesis に戻します。AutoDrill では「モデルが大きく外した問題」を counterexample とみなし、新しい relation hypothesis を作り、対照問題で検証して relation language を更新します。citeturn13search2turn13search8

また、property-based testing の「generator と property を分離する」発想も重要です。QuickCheck は property を明示し、それを generated input に対して試します。AutoDrill でも generator 自体をブラックボックス learner にするより、**「どの structural property が発生しているか」を別 engine で監査する**方が実装・デバッグしやすいでしょう。citeturn18search2

**残差から未知 relation を発見する手順**は次のようにします。

まず通常の in-sample residual ではなく **cross-fitted residual**

\[
e_i=y_i-\hat y_i^{(-fold(i))}
\]

を作ります。大きな正 residual は「User は特異だと感じたのに、既知 relation では説明できなかった」問題です。

次に、その問題と

- theme、
- difficulty、
- known `S_struct`、
- literal 数、
- syntax skeleton

が近い低 residual 問題を対照にします。

その小さな集合だけに、core より広い **shadow grammar** を適用します。shadow grammar には、

- k=3 relation、
- 小さな relation conjunction、
- より深い AST relation、
- extra digit transform、
- restricted symbolic expressions

を許します。

候補 \(r^\*\) は、

\[
\text{residual lift}
-
\lambda L(r^\*)
\]

が大きく、複数独立例に現れるものを優先します。自由形式 symbolic regression はこの段階に限定すれば有用です。自然法則 discovery で symbolic search が実際に複雑な関係を見つけられることは Schmidt–Lipson が示していますが、本件では search multiplicity が大きいため independent confirmation が必要です。citeturn18search1

そして最も重要なのが **metamorphic contrast** です。候補 relation 以外をできるだけ同じにして、

```text
A: 183 - 83   relation ON
B: 184 - 83   relation OFF
```

のような pair を生成します。

あるいは一次方程式なら、

```text
A: 3x - 15 = -3x + 15   symmetry ON
B: 3x - 14 = -3x + 15   symmetry OFF
```

のようにします。

metamorphic testing は、既知の input/output relation を使って follow-up test を作ることで oracle problem を軽減する方法です。AutoDrill では方向が少し違いますが、「一つの relation を制御して follow-up case を作る」という実験設計上の発想が非常に適しています。citeturn18search11

この contrast QA で relation ON–OFF の singularity 差が複数 context で再現すれば core に昇格します。再現しなければ、最初の大 residual は User noise、drift、別の confound の可能性が高くなります。

これにより、

> 大残差 → 「誤ラベル」と捨てる

ではなく、

> 大残差 → hypothesis synthesis → relation-toggle experiment → replication → language refinement

という、ユーザーが想定している循環をかなり厳密に実装できます。


## User QA、弱教師、能動学習

現在の User QA は **continuous weak label** として扱うのが正しいです。完全な ground truth とする必要はありませんが、順位情報だけに落とす必要もありません。既に時間的に離れた類似構造で 0.8 台の絶対値が再現しているなら、少なくとも「評価者内部の scale calibration」が存在する可能性があります。

ただし、単一評価者には三種類の誤差が混ざっています。

\[
y_{p,t}
=
g_t(s_p)+\epsilon_{p,t}.
\]

ここで \(s_p\) はその評価者が理想的に付ける latent score、\(g_t\) は時点による severity/scale drift、\(\epsilon\) は一時的 noise です。

Many-Facet Rasch Measurement は leniency/severity、central tendency、randomness、halo などの rater effect を扱うための枠組みです。また Dawid–Skene は複数 observer の error rate を latent truth とともに推定する代表的手法です。しかし現在のようなほぼ一人の rater では「User が母集団より対称性を +0.2 強く見る」といった population bias は識別できません。citeturn15search9turn15search2

したがって最初の目的は population correction ではなく、

> **User 自身の repeatability と temporal drift を測る**

ことです。

具体的には anchor 問題を時間を隔てて再評価します。同一 item の記憶効果が気になる場合は、literal を変えて relation pattern を保持した near-anchor も混ぜます。これによって「0.08 程度の residual は評価 noise の範囲だが、0.35 は異常」といった noise floor が得られます。

absolute rating だけでなく、少数の comparative judgment を追加する価値もあります。Attali らは SAT 数学項目の difficulty について、独立した絶対難易度推定より item 間の比較・順位判断が有望であることを報告しています。これは difficulty の研究なので singularity へ結果を直接移すことはできませんが、**評価者の scale drift を検出するために「どちらがより singular か」という pairwise check を併用する**という測定法は再利用できます。citeturn15search0

また、専門家と novice で問題表現の基準が変わりうるという教育・認知研究を考えると、現在の User を将来の児童・生徒母集団へ自動補正するのは避けるべきです。将来 5～20 人程度でも対象に近い評価者 panel が得られた段階で、User と panel の mapping を推定するのが筋です。専門家と novice の problem categorization が異なることは古典的に観察されています。citeturn14search19

**約1000 QA の使い方では、100テーマ×10問を均等評価する設計は非効率です。** relation feature がテーマ横断で共有されるという仮説を積極的に利用し、「テーマを学ぶ」のではなく「relation effect と model failure を学ぶ」べきです。

active learning では uncertainty だけを取ると同型問題が大量に集まる危険があります。BALD は information gain を使う原理的な acquisition、core-set 法は representativeness/diversity、BADGE は uncertainty と diversity を組み合わせる方法です。一方 Sugiyama の active regression 研究は、model misspecification がある場合の active selection に注意が必要であることを明示しています。AutoDrill はまさに relation dictionary が未完成で misspecified な可能性があるため、uncertainty sampling 100% は避けるべきです。citeturn17search1turn17search3turn17search21turn15search3

最小実装の acquisition は deep active learning をそのまま入れず、解釈可能な feature space 上で次を使います。

\[
A(p)=
U(p)+C(p)+D(p)+B(p)+H(p)-R(p),
\]

ここで、

- \(U\)：モデルの epistemic uncertainty、
- \(C\)：まだ QA で十分覆われていない relation の coverage gain、
- \(D\)：theme / structural cluster diversity、
- \(B\)：運用 threshold 周辺であること、
- \(H\)：residual-generated hypothesis を判別できる度合い、
- \(R\)：既選択問題との redundancy、

です。

重みを最初から精密に学習する必要はありません。**batch 20 件なら固定 quota** の方が頑健です。

| 20件 batch の初期配分 | 件数 | 目的 |
|---|---:|---|
| 高 uncertainty / high leverage | 8 | モデルパラメータを改善 |
| 未被覆 relation の最大 coverage | 6 | relation language 全体を学習 |
| residual hypothesis の ON/OFF contrast | 4 | 未知 relation 発見 |
| 無作為 sentinel | 2 | active-selection bias・model misspecification 検出 |

各枠内で farthest-first または core-set 的に距離の遠い問題を選びます。core-set active learning は batch selection を代表点選択問題として扱う考え方で、この目的に適しています。citeturn17search3

**約1000 QA に収める具体的な追加実験計画**としては、既存100件を残して追加約900 rating operations を次のように使うのが有望です。

| 追加QA | 数 | 何を識別するか |
|---|---:|---|
| temporal anchors | 60 | User 内 noise / drift。20 anchor × 3時点 |
| relation-toggle contrasts | 240 | 約20主要 family × 6 pair × ON/OFF |
| cross-domain sentinels | 200 | 約100テーマに2件ずつ。未知の domain failure |
| adaptive batches | 300 | uncertainty + coverage + diversity |
| residual / k=3 reserve | 100 | 未知 relation と pairwise 仮説の反証 |
| **追加計** | **900** | |
| 既存 | **100** | |
| **総計** | **約1000** | |

これは統計的に最適な固定配分だという主張ではなく、**最初の実験計画として、何に QA を使ったかが明確で後から ablation できる配分**です。最初の 200～300 追加 QA の結果を見て、contrast の効果分散が大きければ contrast 枠を増やし、k=3 residual がほぼ出なければ k=3 reserve を減らすべきです。

特に relation-toggle 240件は価値が高いと考えます。ランダムに「singularity 0.67」というラベルを一件増やすより、

\[
\Delta y
=
y(\text{relation ON})
-
y(\text{relation OFF})
\]

が得られる方が、「その relation 自体が User の判断を何点動かすか」という因果的解釈に近づくからです。もちろん完全な causal experiment にするには difficulty、構文、answer などの confound を可能な限り固定する必要があります。

この設計なら、新テーマへ移るときに「そのテーマ専用の特徴を100問で再学習」する必要はありません。zero-shot で core relations を評価し、

> 既知 relation だけで説明できない高 residual / high uncertainty region

にだけ QA を追加します。これが 100 テーマに約1000件で一般化するための中心戦略です。


## 参照教材コーパスの位置づけ

参照教材は singularity の教師ではなく、**三つの別用途**に限定すべきです。

第一は **high-tail guardrail** です。

AutoDrill の目的は教材分布を再現することではないので、

\[
P_{\rm Auto}(S)\approx P_{\rm Ref}(S)
\]

を目標にしてはいけません。むしろ theme×difficulty を合わせた上で、

\[
\Pr_{\rm Auto}(S>\tau)
\]

や q90/q95 が、参照教材より不自然に大きくなっていないかを監査します。原則「AutoDrill は reference より低 singularity 側」という要件なら、下側や中央値を一致させる必要はなく、**上側 tail だけを制約する one-sided guardrail** で十分です。

たとえば運用上は、

```text
theme × difficulty cluster ごとに
AutoDrill q95(S_struct)
Reference q95(S_struct)
を比較

Auto 側の upper confidence bound が
reference guardrail を超える場合だけ生成率を抑える
```

という使い方ができます。参照教材自体に奇妙な問題が混ざっていても、それを ground truth label にはしません。

第二は **unknown-feature discovery** です。ここではユーザー案の「reference vs AutoDrill classifier」が非常に有望です。

Classifier Two-Sample Test は、二つの標本に source label を付けて classifier を学習し、held-out accuracy が chance を超えるかで分布差を検出する方法です。Lopez-Paz–Oquab は C2ST の理論・実験的性質を整理し、classifier の predictive behavior から「どこで分布が違うか」を調べられることを示しています。MMD は classifier を必要としない代表的な kernel two-sample test です。citeturn16search2turn16search22turn16search3

AutoDrill では最初に、

\[
X(p)=
[\text{core relation features},
 \text{role features},
 \text{syntax features}]
\]

を入力した **cross-fitted sparse logistic classifier** を使うのがよいです。

```text
label = 0 : reference
label = 1 : AutoDrill
```

として、係数の大きい relation を調べます。

たとえば、

```text
SUFFIX_MATCH_2       Autoで8倍多い
ANSWER_POWER10       Autoで5倍多い
MIRRORED_NEGATION    Autoで4倍多い
```

のような差が出れば、これは

> 「singularity である」

という結論ではなく、

> **「generator が reference と比べてこの relation を過剰生成している」**

という diagnostic です。

ここで classifier が高精度でも、source-specific なフォント、語尾、LaTeX formatting、topic composition を拾っているだけでは無意味です。したがって C2ST 前に theme、difficulty、問題形式を matching / stratification し、source 固有 formatting を canonicalize する必要があります。

第三は **density-ratio diagnostics** です。C2ST が「違うか」を見つけるのに対し、

\[
w(x)=\frac{p_{\rm Auto}(x)}{p_{\rm Ref}(x)}
\]

を推定すると「Auto 側に何倍過剰な region か」を定量化できます。Sugiyama らは numerator/denominator density を別々に推定せず、density ratio を直接推定する方法を研究しています。AutoDrill ではこの比を singularity score にはせず、**generator diagnosis / sampling priority** に使うべきです。citeturn17search0turn17search27

各二標本手法の位置づけは次のようになります。

| 方法 | AutoDrillでの適性 |
|---|---|
| sparse C2ST | **第一選択**。どの既知 relation が違うか説明しやすい |
| density-ratio estimation | 過剰生成 region の強度を知るのに有効 |
| MMD | 「何か差が残っているか」の omnibus detector に有効 |
| contrastive representation | core features で説明不能な residual difference 探索に有効 |
| deep C2ST | raw AST/text に未知特徴があるか探す second stage |
| GAN | 本目的には不要。二標本検定のために generator をさらに学習する理由がない |

MMD は柔軟な分布差検定ですが、「どの feature が問題か」の説明性は sparse classifier より低いので、AutoDrill の first-line diagnostic としては C2ST の方が適しています。citeturn16search3

contrastive learning / neural representation は捨てる必要はありません。ただし順序が重要です。

まず interpretable DSL で C2ST を行う。

次に既知 relation signal を除去・matching してなお raw AST classifier が source を高精度で識別できるなら、

> 「core DSL が取り逃がしている未知構造がある」

という evidence とみなします。

その classifier の high-influence example を residual discovery pipeline に戻し、近傍例・counterfactual を作って、最終的には **解釈可能な relation として core に昇格させる**のがよいです。

この使い方なら neural model は「singularity oracle」ではなく **feature-discovery microscope** になります。

frequent pattern mining / relation mining も同様に補助的です。Apriori や knowledge-base relation mining のような方法は大規模 corpus から recurrent relation を発見するのに適していますが、singularity の本質は「頻出だから重要」ではなく「問題内で単純なのに背景下では珍しい」です。したがって frequent mining は generator/template bias の診断には使えても、singularity scoring の原理にはしない方がよいでしょう。


## 評価設計と候補手法の比較

通常の random CV だけでは、この研究の重要な failure mode の大半を見逃します。特に同一 relation pattern の slight variation が train/test の両方に入れば、高い CV 精度でも「分野横断一般化」や「未知 relation discovery」を何も検証していない可能性があります。

したがって少なくとも次の validation を独立して持つべきです。

**time-block validation** では、古い QA で学習して新しい QA を予測します。anchor から推定した User drift を補正した場合としない場合を比較します。これにより「モデル drift」と「評価者 drift」を区別しやすくなります。rater severity 等の変動を明示的に扱う必要性は rater-modeling 文献でも重視されています。citeturn15search9

**domain holdout** では、一つの theme を丸ごと外す leave-one-theme-out に加え、可能なら類似 theme cluster を丸ごと外します。たとえば一次方程式だけ外すより「方程式系」をまとめて外した方が、本当の cross-domain transfer を厳しく評価できます。

**relation holdout** は二種類実施します。

一つ目は、relation evaluator は残したまま、その relation を持つ QA example を supervised calibration から外す方法です。これにより MDL+rarity layer が zero-shot で方向を当てられるかを見ます。

二つ目は relation template 自体を dictionary から削除します。そして既知の高-singularity relation を「未知 relation」に見立て、

```text
template削除
→ residual発生
→ shadow grammarが候補発見
→ contrast生成
→ relation再発見
```

まで通るか試します。これは本システムにとって通常 CV より重要な end-to-end validation です。CEGIS 的 refinement loop の有効性を直接評価できます。citeturn13search2

**k=3 stress test** では、unary/binary feature を可能な限り揃えた問題 pair を人工的に作ります。

```text
A: 14, 17, 20   arithmetic progression ON
B: 14, 17, 21   OFF
```

のようなケースです。

これを複数種類作り、User が A を一貫して高 singularity とするのに k≤2 model が差を出せなければ、k=3 hypothesis は反証不能どころか積極的に支持されます。逆にこうした contrast でも差が弱ければ k=2 core を維持できます。

**counterfactual relation validation** では、global MAE より

\[
\Delta \hat S
\quad\text{vs}\quad
\Delta y
\]

を主指標にします。relation ON/OFF を正しい方向に動かせるかは、「なぜ singularity が高いのか」を学べているかを見る直接的テストです。

**difficulty leakage validation** も必要です。同じ difficulty bin 内で singularity prediction が機能するか、difficulty を match した contrast でも User score 差が残るかを確認します。response time、正答率、IRT difficulty のような difficulty proxy を singularity target feature として直接使うのは避けるべきです。

difficulty-rating 文献は測定方法の参考になりますが、construct を混同してはいけません。たとえば Attali らが研究したのは数学問題の subjective difficulty judgment であり structural singularity ではありません。その価値は「人間の絶対評価は noisy で、比較判断を補助的に使える」という測定論上の知見にあります。citeturn15search0

評価指標は一点の RMSE に集約せず、

| 層 | 指標 |
|---|---|
| User agreement | MAE, RMSE, Spearman/Kendall |
| calibration | score bin ごとの平均 User rating |
| high tail | top-decile precision、q90/q95 error、threshold exceedance |
| counterfactual | relation-toggle Δ の方向・大きさ |
| cross-domain | held-out theme performance |
| discovery | hidden relation rediscovery rate |
| drift | anchor-adjusted time-block error |
| distribution | reference/Auto tail ratio、C2ST AUC |
| uncertainty | bootstrap/posterior interval coverage |

を併記するのがよいです。

特に User repeated-anchor のばらつきを **noise floor** として報告します。たとえば同一評価者自身の再評価 RMSE が 0.10 なら、モデル RMSE 0.11 と 0.09 の差を過度に追求する価値は低くなります。

候補手法を AutoDrill の条件で比較すると次のようになります。評価は本調査からの相対評価です。

| 手法 | 理論的妥当性 | 必要QA数 | 分野横断性 | 計算量 | 実装難易度 | 弱教師への頑健性 | 解釈可能性 |
|---|---|---|---|---|---|---|---|
| **有限 typed DSL + MDL/rarity + sparse calibration** | **高** | **低〜中** | **高** | 低 | 中 | **高** | **非常に高** |
| hierarchical Bayesian relation model | 高 | 中 | 高 | 中 | 高 | **非常に高** | 高 |
| enumerative synthesis / shadow grammar | 高・発見用 | 低いラベル依存 | 高 | 中〜高 | 高 | 中 | 高 |
| symbolic regression | 中・発見用 | QAより探索データ | 高 | 高 | 高 | 低〜中 | 中 |
| sparse C2ST | **高・分布診断用** | User QA不要 | 高 | 低 | 低 | User非依存 | **高** |
| density-ratio estimation | 高・分布診断用 | User QA不要 | 高 | 中 | 中 | User非依存 | 中 |
| contrastive / neural residual detector | 中〜高・未知特徴用 | User QA少 | 高 | 高 | 高 | 中 | 低 |
| end-to-end neural singularity predictor | **現状は低** | **高** | 潜在的には高 | 高 | 中〜高 | **低** | **低** |

有限 DSL + MDL/rarity を第一選択とする根拠は、Daikon の template-based invariant discovery、MDL、低次数 interaction 探索、認知的 simplicity/surprise の四つが同じ方向を指すことにあります。citeturn16search0turn18search0turn16search9turn14search0

hierarchical Bayesian model は将来的には魅力的です。theme ごとの rarity、relation family effects、User drift、複数 rater を partial pooling できます。しかし現時点では rater がほぼ一人、QA が100件なので、多数の latent facet を最初から入れるより raw structural model を先に固定する方が識別性・デバッグ性で有利です。MFRM のような rater model は評価者が増えた段階で大きな価値を持ちます。citeturn15search9

active learning については、将来 Bayesian model が十分安定したら BALD 型 information gain を使う余地があります。ただし現在の explicit sparse relation feature では、深い gradient embedding を必要とする BADGE より、uncertainty + coverage + core-set diversity の方が実装上素直です。citeturn17search1turn17search21turn17search3

**捨てるべき、または core から外すべき案**もかなり明確です。

「任意 expression を BFS で深く列挙する」は、無意味な偶然式を大量に生み、relation prior の設計問題を悪化させます。「symbolic regression に全部任せる」も同じ理由で core には不適切です。「reference vs Auto classifier の確率をそのまま singularity とする」は source-likeness と singularity の混同です。「-log frequency だけ」を使うと複雑な珍関係が過大評価されます。「User QA に end-to-end NN を直接 fit」するのは1000件規模・単一弱教師という条件に合いません。そして「pairwise relation だけで永久に十分」と決めるのも避けるべきです。自由形式 symbolic relation discovery が可能であること自体は知られていますが、それは本件で探索自由度を最大化すべきことを意味しません。citeturn18search1turn18search0


## 最小実装と研究判断

最初に作るべき MVP は、かなり小さくできます。

**parser / canonicalizer** は、表示 literal、digits、answer、normalized semantic values、roles を抽出します。テーマ固有コードを大量に書くのではなく、各 generator が既に知っている semantic AST から共通 interface へ値を渡します。

**relation engine** は、まず 20～40 程度の relation family から始めます。parameter 展開後には数十～数百 template になって構いません。重要なのは「family が心理的・表記的に説明可能」であることです。QuickCheck/Daikon 型に、候補 property family を先に限定してから網羅的に評価する方針が適しています。citeturn18search2turn16search0

**rarity table** は大量の User QA ではなく、自動 conditional-null generation から作ります。各 `(relation family, role pair, theme cluster, difficulty bin)` について eligible count と hit count を蓄積し、smoothing します。したがって User QA を増やさなくても、新分野で `S_struct` を計算できます。

**score engine** は relation complexity prior と rarity を統合し、nested evidence を collapse します。各問題について最終 score だけでなく、

```text
top evidence:
  1. mirrored_negation(coeff):  +2.8
  2. mirrored_negation(const):  +2.4
  3. answer_small_integer:       +0.7
raw_structural_score:            4.1
```

のような explanation trace を保存します。この trace が残差研究と generator debugging の中心データになります。

**User calibrator** は最初は非常に小さくします。`S_raw` に対する monotone bounded regression と、せいぜい数 family group の shrinkage weight、session drift だけです。既存100 QAを最大限使いつつ過学習を避けます。

**active QA manager** は「uncertainty、uncovered relation、domain diversity、residual experiment、random sentinel」を batch 内で quota selection します。active regression は model misspecification 下で選択 bias が問題になるため、ランダム sentinel を残しておくことが重要です。citeturn15search3

**reference monitor** は、reference と AutoDrill を theme/difficulty matched にして、上側 tail と sparse C2ST の二つだけをまず実装します。C2ST が known feature だけで高精度なら generator を直接修正し、known feature では分類できないのに raw representation classifier が分類できる場合だけ unknown-feature research に進みます。citeturn16search22

この MVP でまず答えるべき研究上の判定は、次の順です。

| 判定したいこと | 最小テスト |
|---|---|
| simple × rare が主要因か | relation-toggle QA で `S_raw` と Δrating を比較 |
| unary が必要か | zero/round/repeated-digit contrasts |
| k=2 が十分か | k=3 stress pairs |
| User drift が大きいか | repeated anchors |
| role が必要か | 同じ値関係を異なる role に置いた contrast |
| digit representation が必要か | numeric value を近似一致させ digit pattern だけ変える |
| reference 差に未知特徴があるか | known-DSL C2ST → residual raw-AST C2ST |
| residual loop が機能するか | 既知 relation を意図的に dictionary から削除して rediscovery |

ここで特に重要なのが最後です。**本当に未知 relation を発見できるかは、未知 relation が現れるのを待たなくても検証できます。** `SUFFIX_MATCH` や `MIRRORED_NEGATION` をわざと core から隠し、

1. prediction residual が大きくなるか、
2. shadow grammar が relation を候補に挙げるか、
3. contrast generator が診断的 pair を作れるか、
4. その QA によって relation が正しく core に戻るか、

を測ればよいのです。これは program verification の mutation/counterexample 的な考え方とよく対応します。CEGIS の candidate–counterexample–refinement cycle は、この評価設計の直接的なモデルになります。citeturn13search2

最終的には、AutoDrill generator 側で singularity を「最大値以下」にするだけでは不十分です。低 singularity 問題だけを残しても、同一の mildly-singular motif が数十問連続すればドリルは不自然になります。したがって生成集合 \(D\) については、個別 score に加え relation family の集中度も制御すべきです。

たとえば、

\[
\text{DrillPenalty}(D)
=
\sum_{p\in D}\phi(S(p))
+
\gamma
\sum_r
\psi\bigl(\operatorname{count}_D(r)\bigr)
\]

として、\(\phi\) は high-singularity tail、\(\psi\) は同一 motif の繰り返しを penalize します。これにより `183-83` 型が1問入ることと、10問連続することを区別できます。最終目的である「各 theme×difficulty の生成集合で高 singularity が過剰集中しない」を直接実装するには、個々の singularity estimator と drill-level diversity controller を分離するのがよいです。

参照教材との比較も最終的にはこの drill-level policy に使います。reference の全分布を模倣するのではなく、

\[
\text{Auto high-tail}
\le
\text{conservative reference guardrail}
\]

としつつ、relation repetition は AutoDrill 独自にさらに低く抑えることができます。これは「参照教材より安全側」という現在の目的に合います。

総合すると、AutoDrill にとって最も有望な研究路線は、

> **有限の心理・表記 grounded relation language を先に作り、relation の低記述長と conditional rarity を教師なしに統合して structural evidence を計算し、少量の弱い User QA はその evidence の校正と relation 発見にのみ使う。モデルが外れた問題は counterexample として shadow relation search と metamorphic contrast QA に回し、参照教材は high-tail calibration と two-sample feature discovery に限定して使う。**

というものです。

これは「ニューラルネットで全部学ぶ」より現在の約1000 QA 制約に合い、分野横断 transfer が可能で、`33` がなぜ高いのか、`183-83` の何が寄与したのかを説明でき、そして最も重要なことに、**k=2 や現在の relation list が間違っていた場合に、その間違いを残差から発見できる構造**を持ちます。simple-pattern surprise、MDL、low-order interaction testing、dynamic invariant discovery、CEGIS、metamorphic testing、active learning、rater modeling、classifier two-sample testing の知見を統合すると、この設計が最も AutoDrill の問題設定に整合します。citeturn14search0turn18search0turn16search9turn16search0turn13search2turn18search11turn15search3turn15search9turn16search22