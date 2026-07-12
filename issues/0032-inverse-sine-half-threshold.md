# Issue 32: inverse sineの1/2閾値を構造比較する

`asin` の正値経路は series を直接使える `value <= 1/2` の判定と、その
unit-series helper の assertion で、それぞれ `Rational::one()` を2で割って
canonical `1/2` を構築する。canonical nonnegative Rationalでは
`2 * numerator` と `denominator` の整数比較だけで同じ順序を判定できるため、
分岐のためだけのRational allocationとGCD正規化を除く。

- `0`、`1/2`、その両隣、`1`未満の値で汎用Rational比較と一致させる。
- `asin(1/3)`、`acos(1/3)`、`asin`のrange-transform経路を回帰する。
- directed bounds、domain、logical work、no-float、公開protocolを変更しない。
- native allocationをbefore/after測定し、全gateとsubagentの差分・全体整合・
  merge粒度review後にmainへ一度だけ統合する。
