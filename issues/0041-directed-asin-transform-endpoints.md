# Issue 41: transformed asin endpointを方向付き評価する

`1/2 < |x| < 1` のasinは `pi/2-atan(sqrt(1-x^2)/x)` へ変換するが、
各endpointでsqrt・atan・piのpaired boundsを構築して片側を捨てる。
要求方向に応じてsqrt ratio endpoint、反対方向のatan、同方向のpiだけを構築する。

- lowerはsqrt upper・atan upper・pi lower、upperはsqrt lower・atan lowerを使う。
- 負値は奇関数の方向反転を維持し、±1とunit級数は既存directed経路を使う。
- 旧paired boundsとのexact一致を半境界直上、正負、1近傍で回帰する。
- domain、tail、停止性、logical work、resource accounting、no-float、protocolを変えない。
- allocation/timing、Wasm境界、全gate、subagent review後にmainへ統合する。
