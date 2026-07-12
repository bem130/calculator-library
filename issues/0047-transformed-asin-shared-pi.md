# Issue 47: transformed asin endpointでpi enclosureを共有する

非退化intervalの`1/2 < |x| <= 1` asinはlower/upper endpointを方向付きに
評価するが、入力非依存のpi enclosureを各endpointで再計算する。acosも外側の
pi enclosureは共有する一方、内部のtransformed asinが別のpiを再構築する。
endpoint固有のcomplement sqrtとatanは独立に保ち、同一精度のpi boundsだけを
一度構築して全endpointへ渡す。

- 正負、unit、series/transform混在intervalで旧directed boundとexact一致させる。
- 正値性、単調性、保証区間、logical work、resource、protocol契約を維持する。
- transformed asin/acosのnative時間・allocationと公開境界をbefore/after測定し、
  全gateとsubagent review後に統合する。
