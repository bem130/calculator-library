# Issue 40: unit asin endpointを方向付き評価する

非退化asin intervalは各endpointでpaired boundsを作り片側を捨てる。`|x|<=1/2`の
正項級数ではlowerは部分和、upperはtail付き値と方向が固定なので、必要側だけを
canonical化する。負値は奇関数として方向を反転する。

- exact pointはpaired boundsを共有する。
- unit域のlower/upper endpointは必要方向だけを構築する。
- ±1/2境界、zero、負値、旧paired boundsとのexact一致を回帰する。
- `|x|>1/2`の既存sqrt/atan/π変換はこのsliceではpaired fallbackを維持する。
- logical work、resource accounting、no-float、公開protocolを変えない。
- allocation/timing、全gateとsubagent review後にmainへ統合する。
