# Issue 60: expの項数探索で得たfactorialを再利用する

expのTaylor項数探索はtail条件を判定するためfactorialを構築するが、探索後に
その値を捨て、共通分母`b^N*N!`の構築時に同じ積を再計算している。precisionから
項数と`N!`を一度だけ計画し、exact point、非退化endpoint、binary scalingの
各recurrenceへ渡す。

- 値依存の分母冪、部分和、tail、方向付き丸めは共有しない。
- 異なるendpoint分母ではfactorialだけを共有し、各共通分母は独立に構築する。
- 項数、保証区間、停止性、logical work、resource accountingを変更しない。
- exact/非退化/large expとgeneral powerを回帰・測定し、全gateとreview後に統合する。
