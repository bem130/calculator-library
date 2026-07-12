# Issue 18: atan交代級数を共通分母recurrenceで評価する

`atan(2)`はreciprocal identityとπのMachin公式を通じて3本のatan級数を評価する。現在は各項を
Rational化し、冪更新・奇数除算・部分和加減算ごとにGCD正規化する。

- `z=a/b`の奇数冪・奇数分母積・共通分母をBigInt recurrenceで更新する。
- 交代符号をsum numeratorへ適用し、sumと最初の未加算項による隣接boundだけをcanonical化する。
- unit atanとπのreciprocal atanを同一helperへ統合する。
- 旧Rational定義とのexact一致、停止性、directed bounds、logical-workを維持する。
- allocation/timing/Wasmを記録し、全gate/review後に一度だけ統合する。
