# Issue 52: exact asinの変換方向を引数領域で選ぶ

exact dyadicの`1/2 < x < 1/sqrt(2)`で`asin(x)=pi/2-atan(sqrt(1-x^2)/x)`を使うと、
atan引数が1を超え、内部atanもpi/2 reciprocal変換を行って入力非依存piを構築・相殺する。
この領域では同値な`asin(x)=atan(x/sqrt(1-x^2))`を選び、unit atanだけで保証区間を作る。

- 浮動小数点や入力固有分岐を使わず、canonical integer partsで領域を比較する。
- 正負、half、`1/sqrt(2)`近傍、unit endpointで旧paired boundsとexact一致させる。
- logical work、resource、protocolを維持し、`acos(5/8)`をnative/公開境界で測定する。
- 全gateとsubagent review後にmainへ一度だけ統合する。
