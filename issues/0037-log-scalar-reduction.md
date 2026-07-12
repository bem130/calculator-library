# Issue 37: log range reductionの2倍・半減をprimitive化する

`log` は正引数を `[1,2)` へ移す各stepで、Rational `1` / `2`との汎用比較と、
Rational `2`による汎用除算または乗算を行う。canonical正Rationalのparts比較と
既存primitive positive-scalar helperなら、中間Rational operandと汎用dispatchを除き、
各stepを一度のcanonical化で実行できる。

- 正負のbinary exponent、複数step、非整数引数で旧汎用演算とexact一致させる。
- `[1,2)`の境界選択、最大step数、exponent accounting、log2合成を維持する。
- `ln(2)`と代表general-power/log複合経路をbefore/after測定する。
- directed bounds、logical work、resource accounting、no-float、公開protocolを変えない。
- 全gateとsubagentの差分・全体整合・merge粒度review後にmainへ一度だけ統合する。
