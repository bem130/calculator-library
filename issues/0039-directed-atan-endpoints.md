# Issue 39: 非退化atan endpointを方向付き評価する

非退化intervalのatanは各endpointでpaired boundsを構築し、片側を捨てる。
交代級数の次項符号から必要なsum/adjacentだけをcanonical化し、`|x|>1`の
reciprocal identityと負値の奇関数変換も方向付きに伝播する。

- exact pointは従来どおりpaired boundsを共有する。
- lower endpointはlower、upper endpointはupperだけを構築する。
- reciprocalの反転、π/2との差、負値の符号・方向反転を維持する。
- 旧paired boundsとのexact一致、zero、unit境界、正負・reciprocal域を回帰する。
- logical work、resource accounting、no-float、公開protocolを変えない。
- native allocation/timing、Wasm境界、全gateとsubagent review後に統合する。
