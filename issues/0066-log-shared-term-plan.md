# Issue 66: non-degenerate logのprecision-only term planを共有する

非退化log intervalはlower/upper endpointを別々の方向で評価するが、級数項数は
endpoint値や方向ではなく要求precisionだけで決まる。従来はendpointごとの
reduced seriesと、必要な`ln(2)` compositionがそれぞれ同じterm-count探索を
繰り返していた。

- range reduction後にprecision-only term countを一度だけ計画し、lower/upperの
  directed reduced seriesと、shared/single `ln(2)` compositionへ渡す。
- endpoint固有のseries state、方向、binary exponent compositionは独立に保つ。
- exact pointのpaired evaluatorはこのsliceでは変更しない。
- 両endpointがreduced 1かつbinary exponent 0なら従来どおりprecision preflightを
  行わず`[0, 0]`を返す。それ以外は従来と同じtyped precision errorを返す。
- 旧独立directed evaluatorとのexact一致を負・零・正・mixed exponent、unit
  reduced endpoint、複数precision、error順で回帰する。
- allocation、Criterion、logical workを対象経路と通常controlで比較し、全gateと
  review後に統合する。
