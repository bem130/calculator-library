# Issue 65: unit-numerator log recurrenceをspecializeする

reduced-logの級数変数`z=a/b`で`a=1`の場合も、各項で`term *= a^2`と
`term * odd_product`を実行している。正のunit numeratorをloop外で一度判定し、
専用recurrenceではtermが常に1であることから`sum += odd_product`へ置き換える。

- `z=1/b`全般を対象とし、`ln(2)`固有の分岐にはしない。
- nonunit numeratorとzeroは既存loopを維持する。
- upper tail、canonicalization、term-count/error preflight、directed boundsを変更しない。
- 旧一般loopおよび旧Rational recurrenceとのexact一致をunit/nonunit/zero、複数の
  term countとprecisionで回帰する。
- native allocationとCriterionをunit-log利用経路、non-degenerate log、large exp、
  通常exp controlで比較し、全gateとreview後に統合する。
