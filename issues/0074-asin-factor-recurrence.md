# Issue 74: asin recurrenceの係数BigInt構築を除く

asinの共通分母Taylor recurrenceは各項で
`numerator_squared * (2k-1) * (2k-1)`を一時BigIntとして構築し、直後に growing
term numeratorへ乗算して破棄する。directed endpointでもpaired exact pointでも
同じ短命係数が反復される。

- owned term numeratorへ`numerator_squared`と2つのprimitive odd factorを順に
  in-place乗算し、numerator係数BigIntをmaterializeしない。
- denominator factor、部分和、最初の未加算項を2倍するtail、paired/directed bound、
  負値の奇関数性とacos変換を変更しない。
- 旧係数積 recurrenceとのexact oracleをunit/nonunit numerator、zero、小plan、
  64/128/256-bit plan、overflow境界で比較する。
- `asin(1/3)`、非退化unit-series asin、変換域asin、acos controlのnative allocation、
  Criterion、logical work、Wasm/npm境界をbefore/after測定し、全gateとsubagentの
  差分・全体整合・merge粒度review後にmainへ一度だけ統合する。
