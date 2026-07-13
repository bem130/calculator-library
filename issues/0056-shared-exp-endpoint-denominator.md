# Issue 56: 非退化exp endpointの共通分母積を共有する

expのTaylor recurrenceは、部分和に必要な一時BigInt `b*n` と別に、最終的にしか
使わない共通分母まで各項で増大乗算している。dyadic公開経路の2冪分母では
`b^N*N!`をfactorialのshiftで構築し、一般分母はexactなpow fallbackを使う。
同一分母の非退化endpointでは最終値も共有する。

- 分母が異なるendpointとbinary scalingは同じexact構築を独立に行う。
- exact pointの単一recurrence、tail、directed bounds、reciprocal、range reductionを維持する。
- 共有版と独立版のexact一致を正負、reduction、同分母・異分母で回帰する。
- logical work、resource accounting、no-float、公開protocolを変更しない。
- general powerと累積expのallocation/timingをbefore/after測定し、全gateとsubagent review後にmainへ一度だけ統合する。
