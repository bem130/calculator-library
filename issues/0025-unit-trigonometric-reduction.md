# Issue 25: unit paired trigの1除算を除く

`sin_cos_rational`はrange-reduction divisorが1でも`value / 1`を汎用Rational除算し、
既にcanonicalな入力を再度GCD正規化している。

- divisor=1では入力Rationalを直接unit級数へ渡す。
- divisor>1のrange reduction、方向付きbound、tan pole semanticsを維持する。
- 旧1除算とdirect入力のexact一致を正負・零・境界で回帰する。
- `tan(1)`のallocation/timing/logical work/Wasmを記録し、全gate/review後に一度だけ統合する。
