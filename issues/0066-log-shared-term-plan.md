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

## Resolution

非退化endpoint dispatcherはrange reductionとzero shortcutを終えた後、precision-only
`log_series_terms`を一度だけ実行する。同じterm countをlower/upper directed seriesと、
必要なshared/single `ln(2)` evaluationへ渡す。両reduced endpointが1かつbinary
exponent 0ならplanning前にexact zeroを返すため、従来のerror precedenceも維持する。
負・零・正・mixed binary exponent、unit reduced endpoint、複数precisionを独立
directed endpoint oracleと比較する回帰を保持する。

base `466d3ed`でlower、upper、必要時log2を独立planningへ戻した一時variantは、
`ln(2+sin(1))`で159,673 bytes / 1,032 blocks (12,590 / 40 peak)を使用した。
shared planは158,393 / 942でpeakは完全一致し、1,280 bytes / 90 blocksを削減する。
exact-point dispatcherを使うlarge-positive log、`ln(2)`、general powerはそれぞれ
95,900 / 934、9,990 / 402、101,393 / 692でbyte/block/peak完全一致し、このsliceが
exact-point paired evaluatorを変更しないというscopeを確認した。

10-sample non-degenerate Criterionはindependent 187.39--190.10 us、shared
186.81--214.34 usで、Criterionは有意差を検出しなかったためtiming改善は主張しない。
independent-planning runtime差分は完全に撤去した。logical work、typed limits、
no-float、公開protocolは変更しない。
