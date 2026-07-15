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

## Resolution

paired boundsとdirected single-boundは、ceil reductionが1なら入力をborrowしてseriesへ
渡し、1より大きい場合だけ`divide_rational_by_positive_u32`で既存分母へprimitive
divisorを掛ける。旧汎用divisionとのexact一致を符号・分母・divisorの組合せで固定し、
zero divisorのtyped errorも維持する。

base `a4f5c78`で2つのexp callerだけを一時的にowned integer Rationalと汎用divisionへ
戻すと、`exp(2)`は8,801 bytes / 313 blocks (1,567 / 26 peak)を使用した。scalar
helperは8,761 / 311でpeak不変、40 bytes / 2 blocksを削減する。general power、
`exp(sqrt(2)*ln(2))`、`exp(-10000)`はそれぞれ101,393 / 692、125,556 / 1,828、
502,356 / 1,458でbyte/block/peak完全一致し、direct reduction対象外のcontrolである。

10-sample `exp(2)` Criterionはgeneric 20.092--22.182 us、restored scalar
24.443--37.298 usで、このunpaired runではscalar側が遅かった。allocation削減との
因果や安定したtimingを確定するpaired/interleaved測定ではないため、timing改善は
主張しない。generic runtime差分は完全に撤去した。directed bounds、reciprocal、
integer power、logical work、resource accounting、no-float、公開protocolは変更しない。
