# Issue 42: 非退化acos endpointを方向付き評価する

非退化acosは反単調endpointを正しく選ぶ一方、各endpoint内部ではpaired asin boundsを
構築して片側を捨てる。shared piとdirected asinを組み合わせ、upper入力からacos lower、
lower入力からacos upperだけをcanonical化する。

- `acos(x)=pi/2-asin(x)`の減数方向を反転する。
- ±1、zero、shared pi、反単調endpoint順を維持する。
- unit/transform両域と正負入力で旧paired boundsとのexact一致を回帰する。
- logical work、resource accounting、no-float、protocolを変えない。
- allocation/timing、全gate、subagent review後にmainへ統合する。
