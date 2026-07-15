# Issue 10: 符号確定区間の不要な端点積を除く

## 背景

`2^sqrt(2)` は正の底の一般実数冪として `exp(sqrt(2) * ln(2))` を保証区間で評価する。
commit `6e24224` の同一runでは、`sqrt(2)*ln(2)` が約264 µs、最終expまでが約645 µs、
直接general powerが約577 µsであり、この経路は代表的なapproximate componentを支配する。

## 問題

区間乗算は両区間の符号が確定していても常に4個の端点積を構築し、最小・最大を比較してcloneする。
一般冪の `ln(2)` と `sqrt(2)` はともに正区間なので、積の下端はlower同士、上端はupper同士で
単調に決まり、交差する2積と探索は不要である。負区間同士、正負の組合せにも同様の順序則がある。

## 方針

- 浮動小数点や座標的heuristicを使わず、ExactDyadic係数の確定符号で区間を分類する。
- 両区間が非負または非正なら、数学的な単調性に従う2端点積だけを構築する。
- どちらかが0を跨ぐ場合は既存の4候補比較を維持する。
- exact enclosure、決定性、停止性、logical-work、公開protocolを変更しない。

## 受入条件

- 正正、負負、正負、負正、零端点、零跨ぎの各組合せで旧4候補定義と同じ区間を返す。
- general power累積経路と通常の乗算回帰を通す。
- focused timing/allocationをbefore/afterで記録し、少なくとも不要な端点積allocationが減ることを示す。
- native、no-default、Wasm/package/exampleを含むrepository gateを通す。
- subagentの差分・全体整合・merge粒度review後、mainへ一度だけ統合する。

## Resolution

現行`multiply`は両intervalをlower/upper coefficientのexact signで分類し、正正・
負負・正負・負正では単調性で決まる2個の端点積だけを構築する。どちらかがzeroを
跨ぐ場合だけ4候補を比較する。正負・zero endpoint・zero crossingを含む表形式testと、
複数のdyadic intervalを旧4候補extrema oracleと総当たり比較するtestを保持する。

base `fe4ec77`でfast pathを一時的に旧4候補へ戻した同一host比較では、general
powerが102,297 bytes / 740 blocks (6,199 / 43 peak)から101,393 / 692
(6,223 / 43)へ、`sqrt(2)*ln(2)`が38,491 / 1,709 (3,967 / 74)から
37,587 / 1,661 (3,967 / 74)へ、その積を最終expまで評価する経路が126,460 /
1,876 (6,292 / 36)から125,556 / 1,828 (6,316 / 36)へ移る。各経路で
904 bytes / 48 blocksを削減し、peak block数は不変である。一方、peak bytesは
general powerと最終exp経路でそれぞれ24 bytes増える。general-powerの10-sample
Criterionはlegacy 75.940--88.418 us、restored fast path 89.969--104.57 usで、
このunpaired sampleではrestored側が遅かった。因果性・再現性を示すpaired測定では
ないためtiming改善は主張しない。legacy runtime差分は完全に撤去した。
