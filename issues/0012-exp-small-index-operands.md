# Issue 12: exp recurrenceの小整数operandをheap BigInt化しない

## 背景

expのTaylor recurrenceは既約Rationalを項ごとに構築せず、BigIntの共通分母・部分和・現在項を
更新する。general powerの最終非退化expを含む代表経路では、このseries stateが主要コストである。

## 問題

各項の`n`とtailの`N+1`、定数2は`u32`/`u8`に収まることが停止条件で保証されているが、
現在は毎回`BigInt::from`で多倍長operandを生成してからBigInt積へ渡す。大きくなる部分和・分母は
多倍長である必要がある一方、loop indexまでheap所有BigIntにする必要はない。

## 方針

- num-bigintが提供するprimitive整数との乗算を使い、小整数operandを直接渡す。
- 共通分母recurrence、tail上界、項数、directed roundingを変更しない。
- exact enclosure、停止性、logical-work、no-float、公開protocolを維持する。

## 受入条件

- 旧BigInt operand recurrenceとlower/upperが複数precision・有理入力で一致する。
- exact point、非退化endpoint、正負exp、binary-scaled大指数、general powerを回帰確認する。
- allocation/timing/logical-work/Wasm境界をbefore/afterで記録する。
- focused test、repository全gate、subagent差分・全体整合・merge粒度review後にmainへ一度だけ統合する。
