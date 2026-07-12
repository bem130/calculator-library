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
