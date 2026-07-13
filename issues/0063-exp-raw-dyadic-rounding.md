# Issue 63: exp Taylor結果をraw fractionからdirected dyadicへ丸める

expの共通分母Taylor recurrenceは最終的な分子・分母を得た直後にcanonical
Rationalへ既約化するが、公開intervalへ返す経路では続けて同じ比をdyadicへ
directed roundingする。途中のGCDと除算を避け、rawの正分母fractionを直接丸める。

- `0 <= x <= 1`でTaylor結果が直ちにdyadic化されるexact、非退化endpoint、
  binary-scaled residualだけを対象とする。
- range reduction後の整数冪、負値のreciprocal、その他のRational利用経路は
  canonicalizationを維持する。
- 旧canonical経路とのexact dyadic一致を通常値、非退化interval、正負large expで
  回帰し、directed enclosure、precision、logical work、公開protocolを変更しない。
- native allocationと代表benchmarkを測定し、全gateとreview後に統合する。
