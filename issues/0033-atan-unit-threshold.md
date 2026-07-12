# Issue 33: atanのunit閾値を構造比較する

nonnegative `atan` は reciprocal identityを使う `value > 1` の判定とunit helperの
assertionで `Rational::one()` を構築し、汎用Rational比較を行う。canonical Rationalの
分子絶対値と正分母を比較する既存のunit predicateは同じ順序をallocationなしで判定できる。

- 負値、零、1未満、1、1超過で汎用比較とunit predicateの一致を固定する。
- `atan(1/2)` のunit seriesと `atan(2)` のreciprocal/π経路を回帰する。
- directed bounds、domain、logical work、no-float、公開protocolを変更しない。
- native allocationをbefore/after測定し、全gateとsubagentの差分・全体整合・
  merge粒度review後にmainへ一度だけ統合する。
