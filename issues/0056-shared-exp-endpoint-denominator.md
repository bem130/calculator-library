# Issue 56: 非退化exp endpointの共通分母積を共有する

expのTaylor recurrenceは、値依存の部分和を更新する傍ら、最終的にしか使わない
共通分母 `b^N * N!` まで各項で増大BigInt乗算している。共通分母を分母冪と
factorialから直接構築し、同一分母の非退化endpointではその最終値も共有する。

- 分母が異なるendpointとbinary scalingは同じ直接構築を独立に行う。
- exact pointの単一recurrence、tail、directed bounds、reciprocal、range reductionを維持する。
- 共有版と独立版のexact一致を正負、reduction、同分母・異分母で回帰する。
- logical work、resource accounting、no-float、公開protocolを変更しない。
- general powerと累積expのallocation/timingをbefore/after測定し、全gateとsubagent review後にmainへ一度だけ統合する。
