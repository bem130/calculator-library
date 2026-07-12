# Issue 38: 非退化log endpointを方向付き評価する

非退化intervalのlogはlower endpointからpaired boundsを作ってupperを捨て、upper
endpointでもpaired boundsを作ってlowerを捨てる。級数state、tail、range-reduction後の
log2合成を方向付きにし、必要なcanonical Rationalだけを構築する。

- exact pointは従来どおり単一stateからpaired boundsを共有する。
- 非退化endpointはlowerからlower、upperからupperだけを構築する。
- 正負binary exponentでlog2 endpointの方向選択を維持する。
- 旧paired boundsとのexact一致、domain、tail、range reduction、停止性を回帰する。
- logical work、resource accounting、no-float、公開protocolを変えない。
- native allocation/timingとWasm境界を測定し、全gateとsubagent review後に統合する。
