# Issue 11: ExactDyadicからRationalへの重複GCDを除く

## 背景

transcendental interval経路は、保証区間をExactDyadicで保持し、exp/log/trig/general powerの
有理数seriesへ入る境界でRationalへ変換する。general powerと最終非退化expは代表componentの
時間・allocationを引き続き支配している。

## 問題

ExactDyadicは `coefficient * 2^exponent` であり、内部演算は係数から2因子を除く正規形を作る。
それにもかかわらず負指数の変換は汎用`Rational::new`を呼び、分母が2冪だけの値へ一般GCDと
除算を再実行する。正指数でも分母1に対して同じconstructorを通る。

## 方針

- 入力ExactDyadicを局所的に正規化し、非canonicalな公開構造値も同じ値へcanonical化する。
- 負指数では正規化後の奇数係数と2冪分母、非負指数では整数としてRationalを構築する。
- 一般Rational constructorの契約は変更せず、2冪構造から互いに素性を証明できる変換だけを専用化する。
- exact equality、符号、zero canonicalization、no-float、resourceと公開protocolを維持する。

## 受入条件

- 正負係数、zero、正負指数、係数に余分な2因子を持つ非canonical値で汎用constructor結果と一致する。
- representative exact/approximate/general-power経路の結果・logical-workが変わらない。
- allocationとtimingをbefore/afterで記録し、focusedおよびrepository全gateを通す。
- subagent差分・全体整合・merge粒度review後、mainへ一度だけ統合する。
