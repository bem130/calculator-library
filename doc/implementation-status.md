# Current Implementation Status

この文書は現行実装の状態を記録する。利用者向けの公開契約は [`public-contract.md`](public-contract.md) を正本とし、この文書の内部構造・アルゴリズム名・テスト構成は公開契約ではない。

## Implemented Surface

現行実装は Rust core、Wasm adapter、npm facade、vanilla web example、CLI、WASI crate を持つ。core は `#![no_std] + alloc` で、浮動小数点型 `f32` / `f64` を使わない。

主な実装済み領域:

* 有理数の exact parse、四則演算、整数累乗、percent lowering。
* `0.1 + 0.2 = 3/10` などの decimal lossless evaluation。
* rational exact output の format preference。有限小数として正確に表せる値は `finiteDecimal` で表示でき、improper rational は `mixedFraction` で帯分数表示できる。正確に表せない finite decimal request は rational 表示へ戻す。
* real principal power semantics に基づく rational power の exact root / domain error / symbolic fallback。
* exact dyadic certified interval、decimal scientific certified interval presentation、adaptive scientific rounding。scientific output は exact rational だけでなく、certified interval の上下端を同じ有効桁数・丸めモードで丸めて一致する場合にも confirmed digits として返す。scientific output と decimal scientific enclosure はどちらも `x.xxx × 10^n` 形式の presentation tree を返す。既定の request は 5 significant digits の scientific output と decimal scientific enclosure を要求する。
* `pi`、`e`、exp/ln/base-explicit log と逆三角関数合成の証明可能な恒等式。`exp(ln(x))` は正性を証明できる supported rational/radical/radical-linear/algebraic `x` で exact にし、`ln(exp(x))` は supported exact `x` で exact にする。`log(argument, base)` は有理数の整数べき、同じ正の有理数 basis の rational power、basis 同士が整数べき関係にある rational powerでexactにする。さらに底変換 `ln(x)/ln(b)`、同一底・同一真数の対数比、順序・結合に依存しない連鎖積、同一底の和差を、真数・底・非零分母の定義域が証明できる場合にexact reductionまたはcanonical symbolic normalizationへ接続する。`ln(argument)` は底 `e` の自然対数として受け、底を省略した `log(argument)` は受けない。`sin(asin(x))` と `cos(acos(x))` は `x in [-1, 1]` を証明できる supported exact `x` で exact にし、`cos(asin(x))` と `sin(acos(x))` は `sqrt(1 - x^2)` を supported exact 値として構築できる場合に exact にし、`tan(atan(x))` は supported exact real `x` で exact にする。bounded rational/dyadic endpoint に対する exp/log/asin/acos/atan、rational point trigonometric range reduction、周期的な sin/cos extrema、tan pole-aware branch、正の底が証明できる一般実数指数 `x^y` の certified interval。
* `abs`、`floor`、postfix `!` / `fact`、`root(argument,index)`、`perm`、`comb`、`mod`、`gcd`、`lcm` / `lcd` alias。整数専用関数は任意精度整数で exact に評価し、巨大な factorial / permutation / combination は logical work limit で止める。`root(argument,index)` は既存の rational power / real algebraic / interval 経路へ lower する。
* `sinh`、`cosh`、`tanh`、`asinh`、`acosh`、`atanh` は source-level で受け付け、内部 DAG では exp/log/sqrt の組み合わせへ lower する。四則演算と非負整数冪のcanonical constructorは、有理係数と整数multiplicity付きfactorからなるbounded sparse polynomial formを使い、平坦化、決定的順序、同類項、分配同値、重複factorを統合する。`e^x`、`exp(x)`、`exp(x,e)`を同じ内部表現にし、自然指数因子の積と商も引数の和差へ統合する。exact reduction後はstored exact valueをfactor keyにして親のcanonical viewを再構築する。これにより `sinh(0)=0`、`cosh(0)=1`、`cosh(100)-sinh(100)=exp(-100)`、`cosh(1000)-sinh(1000)-e^(-1000)=0`、`sin(pi/2)*exp(1)-exp(1)=0` のようなexact simplificationが数値評価より前に行われる。相殺と零倍は消える部分式のdomainが証明済みの場合だけ適用する。
* GeneralSymbolic exact presentation と条件付き exact evaluation における安全な対数積・商・冪、証明済み非ゼロの self-division、証明済み非負の `sqrt(x^2)`、奇偶性、整数 `pi` シフト、`sin` / `cos` の半整数 `pi` cofunction shift、`tan` の半整数 `pi` reciprocal shift の正規化。有理算術は対数規則より先にcanonical化するため`ln(2*3) = ln(6)`となり、非有理の証明済み正因子では`ln(pi*e) = ln(pi)+ln(e)`のように展開する。ほかに`ln(2/3) = ln(2)-ln(3)`、`ln(sqrt(2)) = 1/2*ln(2)`、`exp(2)/exp(2) = 1`、`sqrt(exp(2)^2) = exp(2)`、`sin(-1) = -sin(1)`、`cos(-1) = cos(1)`、`sin(pi+1/10) = -sin(1/10)`、`cos(pi/2+1/10) = -sin(1/10)`、`tan(pi/2+1/10) = -1/tan(1/10)`、`exp(sin(pi/2+1/10)) = exp(cos(1/10))`。
* rational pi multiple recognition。
* simple radical と radical linear combination の exact presentation、積・商・整数累乗の bounded reduction。recognized exact radical/radical-linear/algebraic 値の certified enclosure は、再度元式全体を区間評価せず、証明済み exact 値から構築する。
* rational/simple-radical special angles と inverse trigonometric known values。
* bounded real algebraic recognition for supported polynomial operations、整数累乗、符号を証明できる real algebraic の主 n 乗根と有理指数、cyclotomic trigonometric cases、degree-one algebraic result の rational collapse と後続の代数的演算への伝播。
* parser/session/DTO/native-Wasm/browser conformance tests。
* resource limit enforcement before or during expensive evaluation paths。再帰的exact normalizationは共有logical-work budgetを使い、代数演算はresultant・factorization・root isolation、canonical polynomialはterm積・pairwise merge・factor scan・sort・materializationの保守的上界を開始前に累積予約する。rewrite/logical-work limit到達後はstored exact符号等のstructural domain検証だけを行い、新しいexact/interval探索を開始せずtyped partialへ戻る。source加減算chainは単一n-ary Addへlowerし、canonical interningはcollision確認付きhash bucketを使う。presentation tree と出力文字列にも `max_presentation_nodes` / `max_output_bytes` を適用し、`calculate`、`present`、`presentInput`、生成済みpartial certified enclosureで巨大な出力を返さない。保証区間自体を生成できないpartial outcomeはtyped unavailableと`certifiedEnclosure: null`を返す。
* npm facade の `presentInput` による、評価とは独立した入力式 presentation tree。presentation tree は `renderPlainText`、`renderMathMl`、`renderLatex` で、result relation は `renderResultRelationPlainText`、`renderResultRelationMathMl`、`renderResultRelationLatex` で利用側の表示形式へ変換できる。
* vanilla web example は native textarea の value / selection / composition / scroll を編集中の正本とし、ブラウザ標準の mouse・touch・IME・keyboard editing を保つ expression editor、Shift layer keypad、settings popover、常時表示の exact/scientific/certified interval panes を持つ。画面keypadは確定済みDOM selectionへ同じ編集commandを適用し、タッチデバイスの app key 操作では soft keyboard を出さない。入力previewは公開 `presentInput` facade、結果表示は公開 worker `calculate` DTO と relation/presentation rendererを表示の正本とし、UI独自の数値formatを持たない。

## Hardened Gates

Phase 5 の堅牢化として、次を CI に入れている。

* Rust formatting、Clippy、wasm target Clippy。
* core no-default check/test、workspace test、Rust doctest、wasm-pack test。
* generated DTO check、protocol snapshot check、DTO regeneration diff check。
* `calculator-core` 内の `f32` / `f64` 禁止。
* `cargo deny check` と pnpm audit。
* TypeScript package check、vanilla example build、browser e2e。
* external arithmetic oracle。
* package size budget。
* public enum match exhaustiveness lint。

性能調査用には、native coreのexact/approximate/algebraic/large-expression/session経路を測るCriterion harness、native allocationとlogical-work境界のrunner、Wasm/npm facadeの対応経路を境界cost込みでJSON出力するrunnerを持つ。再現条件と比較手順は [`performance-baselines.md`](performance-baselines.md) に記録する。測定値は機種依存の診断情報であり、公開契約や固定CI閾値ではない。
最初のprofilingではapproximate複合経路の時間がevaluationに集中することを確認し、interval endpoint比較でdyadicを既約rationalへ変換していた不要なGCD/divisionを、2冪指数を整列した係数比較へ置き換えた。logical-work課金と結果契約は維持する。
続くexp/log profilingでは、lowerとupperが同じexact dyadic pointでも同じTaylor boundsを2回構築していたため、directed lower/upper pairを一度だけ計算して共有する。非退化intervalのendpoint別評価、保証区間、refinement上限、logical-work課金は変更しない。
対数の範囲縮小後に生じる`log(1)`は、精度分の零Taylor termを構築せず、恒等式のexact directed pair `[0, 0]`を返す。これにより`ln(2)`とgeneral power内の対数経路から不要な有理数演算・allocationを除き、精度、refinement上限、logical-work課金は維持する。
指数の範囲縮小係数が1となる正のunit-range引数は、引数の1除算とdirected boundsの1乗を行わず、small exponential seriesの保証区間を直接使用する。unit rangeを超える引数の範囲縮小、有理数冪、精度、logical-work課金は変更しない。
approximate component benchmarkは`exp(1)`、`ln(2)`、`2^sqrt(2)`、`sin(1)`を同じevaluation境界で分離し、対応する公開`calculate`境界のallocation caseを持つ。現行baselineではgeneral powerが時間とallocationを支配し、次の調査対象は非退化指数intervalのexp bounds構築である。
expのTaylor級数はrange reduction後の`0 <= x <= 1`と次項以降の剩余が次項の2倍以下であることを使い、`(N + 1)! >= 2^(precision_bits + 1)`を満たす最小項数を整数演算で求める。これにより剩余幅を`2^-precision_bits`以下に保ったまま、general powerの不要な有理数項を除く。directed rounding、refinement、logical-work課金、公開精度契約は変更しない。
expの項更新`term * x / n`は、中間積と除算結果を別々に既約化せず、分子積と`denominator(term) * denominator(x) * n`から1回だけcanonical Rationalを構築する。exact値・停止性・方向付き保証とlogical-work課金を維持したまま、中間allocationと重複GCDを除く。
expのTaylor部分和と現在項は、range reduction後の`x = a/b`に対して共通分母`b^n * n!`上のBigInt recurrenceで保持する。loop内で毎回Rationalを既約化せず、最終lowerと最初の未加算項の2倍を含むupperのみcanonical化する。旧Rational recurrenceとのexact一致、directed bounds、停止性、logical-work課金を維持する。
極端な内部精度では、loop中に既約化しない共通分母の一時BigInt sizeを継続してprofilingする。現在の公開precision/resource上限と固定反復による停止性は維持されるが、代表経路の改善と極端入力のpeak memoryを別々に監視する。
unit-range sin/cosは交代Taylor級数の剩余が最初の未加算項以下であることを使い、より厳しいcos側の`(2N + 2)! >= 2^precision_bits`を満たす最小項数を整数演算で求める。sinの次項はさらに大きいfactorial除数を持つため同じ幅保証を満たす。directed enclosure、range reduction、logical-work課金は変更しない。

## Deliberately Not Contract

次は実装詳細であり、公開契約として扱わない。

* 内部 module 構成。
* exact expression DAG の node storage layout。
* polynomial factorization / resultant / root isolation の具体的な探索順。
* special angle recognition の内部 dispatch 順。
* cache の有無、warm/cold の内部挙動。
* package size budget の数値そのもの。ただし CI gate としての存在は公開品質基準である。

## In Progress

`design.md` の最終目標には、さらに広い transcendental interval evaluation、一般実代数的数の完全な演算閉包、より広い symbolic simplification、最終的な 1.0 release hardening が含まれる。現行実装はこれらをすべて完了したとは扱わない。

未対応または制限超過の領域では、厳密式を破壊して近似値へ落とさず、`Partial`、`unsupportedFeature`、`computationLimit`、または `inputLimit` として返す。
