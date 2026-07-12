# Issue 14: transcendental series境界の小整数BigInt化を除く

## 背景

exp recurrence本体の改善後、general powerではlogとseries境界計算の比率が相対的に増えた。
exp/log/trig/atanの項数・tail補助は停止性を整数だけで判定する。

## 問題

項番号、奇数分母、factorial index、固定係数9などは`u32`/`u8`で有界だが、境界loop内で
毎回`BigInt::from`してから多倍長値へ乗算している。比較対象やfactorial自体はBigIntである必要が
あるが、小整数operandを所有多倍長値にする必要はない。

## 方針と受入条件

- num-bigintのprimitive scalar乗算をexp/log/trig/atanの境界補助へ適用する。
- checked index arithmetic、項数、tail不等式、directed bounds、logical-workを変更しない。
- 最小項数の直接不等式検証と代表transcendental/general-power回帰を通す。
- allocation/timing/logical-work/Wasm境界を記録する。
- 全gate、subagent差分・全体整合・merge粒度review後にmainへ一度だけ統合する。
