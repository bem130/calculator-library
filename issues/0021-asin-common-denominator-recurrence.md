# Issue 21: asin正項級数を共通分母recurrenceで評価する

`0 <= x <= 1/2` のasin保証区間は、各級数項をRationalとして更新し、係数乗算、
除算、部分和加算のたびにGCD正規化している。

- `x=a/b` の冪と係数積をBigIntの項分子・共通分母として更新する。
- loop内のRational正規化を除き、lowerと既存tailを含むupperだけをcanonical化する。
- 旧Rational定義とのexact一致、正項tail、負値の奇関数性、acos変換を維持する。
- special angleではない`asin(1/3)` / `acos(1/3)`をnative・logical work・Wasmで測定する。
- resource limit、no-float、公開protocolを変更せず、全gateとsubagent review後に一度だけ統合する。
