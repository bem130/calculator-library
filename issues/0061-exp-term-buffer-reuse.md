# Issue 61: exp recurrenceのterm bufferを再利用する

共通分母exp recurrenceは各項で`next_term = term*a`を新しいBigIntへ構築し、直前の
term bufferを捨てている。state transitionを所有bufferへの`term *= a`として表し、
同じexact積を部分和へ加える。

- recurrence順序、共通分母、tail、directed boundsを変更しない。
- exact point、非退化endpoint、binary scaling、通常rangeへ一般に適用する。
- logical work、resource accounting、no-float、公開protocolを変更しない。
- representative exp/general powerを測定し、全gate/review後に統合する。
