# Issue 13: exp recurrenceの増大BigInt bufferを再利用する

## 背景

expのTaylor stateは共通分母形式で、部分和、現在項、分母だけをBigIntとして保持する。小整数indexの
所有BigInt化はIssue 12で除去したが、部分和更新は毎項新しい積・和を作って古いbufferを捨てる。

## 問題

`sum = sum * denominator_factor + term * numerator`の左辺はiterationを跨いで唯一所有されるため、
乗算後の桁領域を次の部分和として再利用できる。現在の式形式は新しいBigInt結果へ代入し、増大する
部分和bufferのallocation/reallocation機会を増やす。

## 方針

- 所有中の部分和へ`MulAssign`してから補正項を`AddAssign`する。
- upper tailも所有積へ補正項を加え、式の一時resultを減らす。
- recurrence、evaluation orderの数学的意味、tail保証、項数、directed roundingを維持する。

## 受入条件

- 旧Rational recurrenceとのlower/upper一致を複数値・precisionで維持する。
- exact/非退化/負exp/binary-scaled/general-power回帰を通す。
- allocation、timing、logical-work、Wasm境界をbefore/afterで記録する。
- 全gateとsubagent差分・全体整合・merge粒度review後、mainへ一度だけ統合する。

## Resolution

現行mainではfinite-sum recurrence本体が既に`sum_numerator`と
`term_numerator`を`MulAssign` / `AddAssign`で更新しており、このissueが当初
想定した式形式の一時resultは残っていなかった。lowerを保持しながらupper tailを
作る`upper_parts`には式形式が残っていたため、`sum_numerator`と
`term_numerator`を明示的にcloneしてからowned buffer上で乗算・加算するvariantを
測定した。

base `7faf5bb`とvariantは、general powerで101,393 bytes / 692 blocks
(6,223 / 43 peak)、非退化unit expで74,113 / 970 (6,220 / 59)、
`exp(-10000)`で502,356 / 1,458 (16,047 / 32)、`exp(1)`で8,157 / 293
(1,407 / 24)と完全一致した。tiny dyadic expも9,835 / 357
(2,090 / 29)で一致した。`num_bigint`の参照式は既にowned resultへ計算し、明示clone
後のassignへ変えても再利用可能なcapacityやallocation回数は増えないため、runtime
変更は保持しない。

増大するfinite-sum buffer自体のallocationを減らすには、`BigInt`のcapacityを
事前確保または再利用できるprimitive、あるいは同時live stateを増やさず桁配列を
所有したまま更新できる別表現が必要である。その条件なしにclone/assignの表記だけを
再試行しない。
