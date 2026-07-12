# Issue 16: 負exp boundの逆数で重複GCDを除く

負のexp endpointは正値のcanonical Rational boundを計算後、`1 / bound`を汎用Rational除算で
構築する。正値canonical Rationalの分子・分母は既に正かつ互いに素なので、交換後もcanonicalであり、
再GCDは不要である。

- 正値が証明された内部exp boundだけでcanonical reciprocalを構築する。
- zero防御、符号、lower/upper反転、directed enclosure、logical-workを維持する。
- 正負通常値、巨大正負指数、非退化endpointで回帰し、allocation/timing/Wasm境界を記録する。
- 全gateとsubagent review後にmainへ一度だけ統合する。
