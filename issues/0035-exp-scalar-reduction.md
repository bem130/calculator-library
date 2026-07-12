# Issue 35: exp range reductionをprimitive scalar除算する

`exp(x)` の `x > 1` range reductionはceilで得た正の`u32` divisorをRationalへ
変換し、汎用Rational除算へ渡す。divisorを既存分母へ直接掛けて一度だけcanonical化する
既存helperを使えば、中間Rationalと汎用division dispatchを除ける。directed boundの
reduction=1経路も入力をcloneせず借用できる。

- paired boundsと非退化intervalのdirected single-boundの両経路を同じhelperへ接続する。
- reduction=1のdirect path、正負exp、reciprocal方向、整数冪を維持する。
- 旧汎用除算とのexact一致を複数の符号・分母・divisorで固定する。
- directed bounds、logical work、resource accounting、no-float、公開protocolを変えない。
- `exp(2)`、general powerと明示exp複合経路をbefore/after測定し、全gateとsubagent review後に統合する。
