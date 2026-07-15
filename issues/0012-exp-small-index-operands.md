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

## Resolution

finite-sum recurrenceは各term indexとtail indexを停止条件で保証された`u32`のまま
`BigInt`へ乗算し、tail定数2も`u8`のprimitive operandとして渡す。増大するsum、term、
denominatorだけを多倍長で保持する。dyadic/general denominatorとlower/upper tailを
旧general product recurrenceへ比較するfocused oracleを保持する。

base `8336d2a`でindexと定数2を毎回owned `BigInt`へ戻した一時variantとの比較では、
general powerが103,633 bytes / 762 blocks (6,223 / 43 peak)から101,393 / 692へ、
非退化unit expが76,353 / 1,040 (6,220 / 59)から74,113 / 970へ改善した。
`exp(1)`は9,309 / 329 (1,407 / 24)から8,157 / 293、`exp(-10000)`は
504,788 / 1,534 (16,047 / 32)から502,356 / 1,458へ改善し、全caseのpeakは不変だった。

10-sample general-power CriterionはBigInt operand 77.835--84.420 us、primitive
77.714--98.069 usで、restored runでは有意差を検出しなかったためtiming改善は
主張しない。BigInt-operand runtime差分は完全に撤去した。recurrence、tail保証、
logical work、停止性、no-float、公開protocolは変更しない。
