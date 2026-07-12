# Issue 49: exact transformed acosでpi enclosureを共有する

exact pointの`1/2 < |x| < 1` acosは、内部asin変換と外側の
`pi/2-asin(x)`で同一精度のpi enclosureを二度構築する。外側で必要なpaired
piを内部asinへ渡し、complement sqrt/atanは従来どおり一度だけ評価する。

- 正負、unit/transform境界、special endpointで旧paired boundsとexact一致させる。
- directed enclosure、logical work、resource、protocolを維持する。
- exact dyadic transformを通る`acos(3/4)`のnative allocation・timing・logical workと公開境界を測定し、
  全gateとsubagent review後に統合する。
