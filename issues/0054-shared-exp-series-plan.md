# Issue 54: 非退化exp endpointでTaylor項数計画を共有する

通常範囲の非退化expはlower endpointのlower boundとupper endpointのupper boundを
方向付き評価するが、precisionだけで決まるTaylor項数を各endpointで独立に計算している。
factorial/BigIntによる同一計画を一度だけ作り、両endpointへ共有する。

- binary-scalingはendpointごとにworking precisionが異なり得るため対象外とする。
- exact-pointの単一recurrence state共有、range reduction、負値reciprocal方向を維持する。
- 旧direction wrapperとのexact一致を正負、zero近傍、reduction 1以上で回帰する。
- logical work、resource、no-float、公開protocolを変更しない。
- general powerとnon-degenerate expのnative allocation/timing、Wasm/npm境界を測定する。
- 全gateとsubagent review後にmainへ一度だけ統合する。
