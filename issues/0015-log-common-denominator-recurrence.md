# Issue 15: log正項級数を共通分母recurrenceで評価する

## 背景と問題

`log(x)=2*sum(z^(2k+1)/(2k+1))`のreduced seriesは各項をRationalとして乗算・除算し、
部分和へ加えるたび汎用GCDで既約化する。exp recurrence改善後、`ln(2)`はgeneral powerの主要コストである。

## 方針

- `z=a/b`に対し、`b^(2k+1)`と奇数分母の積を共通分母としてBigInt recurrenceを構築する。
- `z^(2k+1)`と奇数積を増分更新し、loop中のRational canonicalizationを除く。
- lowerと最初の未加算項を含むupperだけを最終的にcanonical Rational化する。
- 既存の幾何tail、最小項数、directed enclosure、no-float、logical-workを維持する。

## 受入条件

- 旧Rational定義と複数の`z`・項数でlower/upperがexact一致する。
- `ln(1)`、`ln(2)`、非退化log、general power、domain/resource回帰を通す。
- allocation、timing、logical-work、Wasm境界をbefore/after記録する。
- 全gateとsubagent差分・全体整合・merge粒度review後にmainへ一度だけ統合する。
