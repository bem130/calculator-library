# Issue 67: log range reductionをcanonical parityでscaleする

logの`[1,2)` range reductionはcanonical Rationalを2倍または半減するたびに
汎用constructorへ戻り、既約性が構造的に分かっている値へGCDと除算を
繰り返している。大きなbinary exponentではこの正規化がstep数に比例する。

- canonical `n/d`の半減は、`n`が偶数なら`(n/2)/d`、奇数なら`n/(2d)`を
  直接構築する。
- 2倍は、`d`が偶数なら`n/(d/2)`、奇数なら`(2n)/d`を直接構築する。
- 正分母、既約性、符号、zero、range-reduction step limit、binary exponent、
  logical work、no-float、directed enclosure、公開protocolを変更しない。
- 汎用Rational演算とのexact一致をparity組合せとmulti-step入力で回帰する。
- 128-stepのlarge positive logをnative allocation、Criterion、logical work、
  Wasm/npm benchmarkへ追加し、既存log/exp経路をcontrolとして測定する。

最終共通分母をloop後のpowで構築する別案は、non-degenerate logのallocationを
約33 KB削減した一方、保存baselineに対するCriterion中央値を約12%悪化させた
ため採用しない。
