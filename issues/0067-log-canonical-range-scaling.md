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

## Resolution

range reductionはcanonical `n/d`のparityに従い、halve時はeven numeratorだけを
shiftし、それ以外はdenominatorをshiftする。double時はeven denominatorだけを
shiftし、それ以外はnumeratorをshiftする。正分母と既約性が構造的に維持されるため
汎用constructorのGCD/除算を通さない。parity組合せとmulti-step入力を汎用Rational
multiply/divide oracleへ比較する回帰を保持する。

base `a12ffbc`でhelperだけを汎用Rational演算へ戻した一時variantとの同一host比較では、
128-step large-positive logが110,268 bytes / 1,702 blocks (12,106 / 25 peak)
から95,900 / 934 (12,114 / 25)へ改善した。non-degenerate logは158,665 / 954
(12,582 / 40)から158,393 / 942 (12,590 / 40)、`ln(2)`は10,062 / 408
(1,560 / 19)から9,990 / 402 (1,560 / 19)、general powerは101,465 / 698
(6,223 / 43)から101,393 / 692 (6,223 / 43)へ改善した。large logのpeak bytesは
8 bytes増えるがpeak blocksは不変で、totalは14,368 bytes / 768 blocks減る。

10-sample Criterionのlarge-positive logはgeneric 108.63--112.34 usからcanonical
88.413--101.54 usへ移り、Criterionも改善を検出した。generic runtime差分は完全に
撤去した。logical-work、range step、no-float、公開protocolは変更しない。
