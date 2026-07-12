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
