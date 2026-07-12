# Issue 19: sin/cos交代級数を共通分母recurrenceで評価する

unit-range `sin` / `cos` はTaylor項をRationalとして保持し、各反復の冪乗算、
factorial係数除算、部分和加減算でGCD正規化を繰り返している。

- `x=a/b` の冪、factorial係数、共通分母をBigInt recurrenceで更新する。
- sin/cosを同じ内部helperへ統合し、最終部分和と最初の未加算項だけをcanonical化する。
- 旧Rational定義とのexact一致、交代級数の方向付きbound、符号・偶奇性を維持する。
- `sin(1)` / `cos(1)` のnative allocation・timing・logical workとWasm公開経路をbefore/afterで記録する。
- resource limit、no-float、公開protocolを変更せず、全gateとsubagent review後に一度だけ統合する。
