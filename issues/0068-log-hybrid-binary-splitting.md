# Issue 68: nonunit log級数をhybrid binary splittingで評価する

nonunit reduced logの正項級数は、各項で増大する部分和へ分母factorを逐次乗算
している。128-bitの公開経路では38項となり、値依存BigInt recurrenceが
非退化logの支配的な時間・allocationになっている。

- `z=a/b`, `x=a^2`, `y=b^2`について、項比
  `p_k=x(2k-1)`, `q_k=y(2k+1)`の積`P/Q`と累積和`T/Q`を保持する。
- 連続区間は`P=P_l P_r`, `Q=Q_l Q_r`,
  `T=T_l Q_r + P_l T_r`でexactに結合する。
- 32項以下は逐次chunkとして所有bufferを再利用し、32項を超えるnonunitだけを
  balanced mergeする。zero、unit numerator、小さい項数は既存recurrenceを保つ。
- lowerは`2a(Q+T)/(bQ)`、upperは最初の未加算項を現行と同じ4倍tailとして
  構築し、片側lowerではupper専用積を作らない。
- legacy incremental recurrenceとのpaired/lower/upper exact一致をzero、unit、
  nonunit、64/128/256-bit項数で回帰する。
- term count、tail保証、directed endpoint、logical work、resource limit、
  no-float、停止性、公開protocolを変更しない。

full balanced/8項leafはbytesをより削減したが、128-bit Criterionを約22%悪化
させたため採用しない。32項leafのhybridは時間とtotal bytesをともに改善する。
