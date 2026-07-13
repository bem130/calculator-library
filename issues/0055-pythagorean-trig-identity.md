# Issue 55: 三角関数の平方和をdomain安全にcanonical簡約する

現行のbounded sparse polynomial正規化は`x*x`と`x^2`、同類項、分配同値を
統合できるが、同じ実引数に対する`sin`と`cos`を独立したfactorとして扱うため、
`sin(x)^2 + cos(x)^2`を`1`へ簡約できない。依頼中の`sin^2+cos^2=1`は
数学上の略記であり、現行の公開構文では括弧付き関数呼び出しを使う
`sin(x)^2 + cos(x)^2 = 1`を対象とする。`sin^2(x)`を逆関数や新構文として
場当たり的に解釈しない。

- angle unit lowering後の同一canonical radian引数を持つ`sin(arg)^2`と
  `cos(arg)^2`を、Add / Multiply / nonnegative integer powerの共通canonical
  normalization内で相補関係として扱う。source順序、`x*x`対`x^2`、括弧、
  分配形に依存させない。
- 単独の固定パターンだけでなく、共通有理係数を持つ平方項を対にして
  `a*sin(arg)^2 + a*cos(arg)^2 -> a`とし、異なる係数では共通部分だけを
  確定的に縮約して残余項をcanonical順序で保持する。負係数、追加定数・項、
  複数の異なる引数を含む和でも項順に依存させない。
- 恒等式で消える`sin`、`cos`と引数が実数領域で定義済みであることを先に
  証明する。`ln(-1)`等を引数に持つ式のdomain errorを`1`で隠さず、証明が
  `Unknown`なら元のsymbolic項を保持する。簡約時は必要な`Defined(arg)`または
  元式のdomain obligationを明示的に残し、dead nodeの走査へ依存しない。
- complement認識は同じcanonical factor / stored exact valueの構造比較を使い、
  ad-hocな表示文字列比較や近似値比較を行わない。degree/radian lowering後の
  同値引数、奇偶性・period/cofunction正規化後に同じ引数となる場合も、既存の
  bottom-up exact value再構築と整合させる。
- 平方以外の冪、異なる引数、`sin(arg)^2-cos(arg)^2`、係数が対にならない式を
  誤って`1`へしない。tanの極やinverse-trig主値に規則を拡張しない。
- complement scan、factor照合、係数分割、materializationの保守的上界を操作前に
  `max_rewrite_steps`、`max_logical_work_units`、`max_expression_nodes`へ予約する。
  limit到達時は部分的な組合せ依存rewriteをせず、typed partialと元の厳密式を
  保持する。決定性、停止性、no-float、公開protocolを変更しない。
- coreで基本形、交換・結合・分配形、係数付き正負・残余、異引数、未定義引数、
  tight resource limit、radian/degreeを回帰する。CLI、Wasm/npm facade、example-uiで
  exact `1`の同一DTO・表示を確認し、package/example buildとbrowser E2Eを通す。
- focused testとlogical-work/allocationのbefore/afterを記録し、subagentの差分・
  全体整合・merge粒度reviewと全gate完了後にmainへ一度だけ統合する。
