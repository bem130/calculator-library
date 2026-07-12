# Issue 20: 自然対数の底eの級数を共通分母recurrenceで評価する

定数 `e` の保証区間は `sum(1/n!)` の各項をRational化し、部分和へ加えるたびに
GCD正規化している。指数関数 `exp(1)` のTaylor recurrenceとは別経路なので、直近の
exp改善では解消されていない。

- factorialを共通分母として部分和分子をBigIntで更新し、最終lower/upperだけをcanonical化する。
- 従来の `2/(N+1)!` tail、項数、方向付き保証、決定性、停止性を維持する。
- 旧Rational定義とのexact一致を回帰し、`e` と `exp(1)` を混同しないbenchmarkを追加する。
- native allocation・timing・logical work、Wasm/npm公開経路をbefore/afterで記録する。
- resource limit、no-float、公開protocolを変えず、全gateとsubagent review後に一度だけ統合する。
