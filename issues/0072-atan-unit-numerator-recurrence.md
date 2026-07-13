# Issue 72: unit-numerator atan recurrenceをspecializeする

`atan(1/b)`の共通分母交代級数は各iterationでnumerator squaredの1を
多倍長termへ乗算し、直後にodd productとの積を構築する。Machin πの`1/5`・
`1/239`、direct `atan(1/2)`、reciprocal atan経路でこの不要処理が反復される。

- canonical numeratorが1ならterm numeratorを更新せず、既存odd productを
  correctionとして直接加減算する。
- paired/lower/upperの交代parity、sum/adjacent、zero、小plan、nonunit hybrid、
  reciprocal変換、shared πを変更しない。
- legacy general recurrenceとのexact oracleをterm count 0/1/threshold/64/128/256で
  比較し、overflow、logical work、resource limit、no-float、公開protocolを維持する。
- `atan(1/2)`、`atan(2)`、πを使うasin/acos、non-degenerate atanをbefore/after測定し、
  timeまたはallocationを退行させる場合は採用しない。
