# Issue 48: reciprocal atan endpointでpi enclosureを共有する

非退化intervalの`|x|>1` atanはlower/upper endpointを方向付きに評価するが、
入力非依存のMachin piを各endpointで再計算する。両endpointがreciprocal域なら
同一精度のpaired pi enclosureを一度構築し、endpoint固有のreciprocal atanは
独立に保ったまま共有できる。

- 正負、unit、reciprocal、unit/reciprocal混在intervalで旧directed boundとexact一致させる。
- 混在intervalでは不要なpaired piを作らず、従来の片方向pi boundを維持する。
- 保証区間、奇関数方向、logical work、resource、protocol契約を維持する。
- `atan(2+sin(1))`のnative allocation・timing・logical workと公開境界を測定し、
  全gateとsubagent review後に統合する。
