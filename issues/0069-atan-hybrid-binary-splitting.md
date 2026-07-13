# Issue 69: nonunit atan級数をhybrid binary splittingで評価する

unit rangeへ縮小したnonunit `atan` の交代級数は、増大する部分和へ各項の
分母factorを逐次乗算する。公開 `atan(2+sin(1))` 経路ではこのrecurrenceが
時間とallocationを支配し、同じbackendを使うtransformed `asin` / `acos`にも
コストが波及する。

- `z=a/b` と項比 `p_k=-a^2(2k-1)`, `q_k=b^2(2k+1)` について、連続区間の
  積 `P/Q` と累積積和 `T/Q` をexactに結合する。
- 小さい連続区間は逐次leafで評価し、十分大きいnonunit planだけをbalanced
  mergeする。zero、unit numerator、小さいplanは既存recurrenceを維持する。
- paired / lower / upperは現行の交代級数parityとsum/adjacent方向を保ち、
  adjacentを使わないdirected endpointではrootの不要な最終積を作らない。
- threshold近傍と64/128/256-bit planをlegacy recurrenceとのexact oracleで
  比較する。
- `atan`、reciprocal変換、Machin π共有、transformed `asin` / `acos`、
  cancellation、logical work、resource limit、no-float、公開protocolを変更しない。
- leaf幅とfull-balanced案はallocationとCriterionの両方で比較し、時間を
  退行させる案や単なるlimit変更は採用しない。

現行main `19ea116` の1 calculation allocationは
`atan(2+sin(1))` が710,074 bytes / 1,983 blocks、transformed `asin` が
1,473,476 / 4,073、transformed `acos` が1,641,406 / 4,166である。
