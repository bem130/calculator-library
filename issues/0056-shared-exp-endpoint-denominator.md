# Issue 56: 非退化exp endpointの共通分母積を共有する

expのTaylor recurrenceは共通分母の更新ごとに一時BigInt `b*n` を構築している。
所有する増大分母へcanonical入力分母とprimitive項番号を順にin-place乗算し、
一時多倍長operandを除く。同一分母の非退化endpointでは最終値も共有する。

- 分母が異なるendpointとbinary scalingは同じin-place更新を独立に行う。
- exact pointの単一recurrence、tail、directed bounds、reciprocal、range reductionを維持する。
- 共有版と独立版のexact一致を正負、reduction、同分母・異分母で回帰する。
- logical work、resource accounting、no-float、公開protocolを変更しない。
- general powerと累積expのallocation/timingをbefore/after測定し、全gateとsubagent review後にmainへ一度だけ統合する。
