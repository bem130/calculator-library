# Issue 53: exact sqrtの上下boundで整数平方根探索を共有する

exact-point sqrtは同じscaled rationalに対してlower用のfloor square rootと、
upper用のceil square rootを独立に探索している。scaled lower/upperは高々1だけ異なるため、
lowerのfloor rootを一度求め、その平方とscaled upperの比較だけでupperが同じrootか次の整数かを決定できる。

- 浮動小数点や近似的な座標補正を使わず、整数比較だけで上下boundを決定する。
- exact/non-square、perfect square、zero、複数precisionで従来boundとのexact一致を回帰する。
- non-degenerate endpoint評価、logical work、resource、公開protocolを変更しない。
- `sqrt(2)`と`2^sqrt(2)`のnative allocation/timingおよびWasm/npm境界を測定する。
- 全gateとsubagent review後にmainへ一度だけ統合する。
