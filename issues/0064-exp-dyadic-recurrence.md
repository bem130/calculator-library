# Issue 64: dyadic exp recurrenceの分母積をshiftへ分解する

exp Taylor recurrenceはdyadic入力の分母`b=2^k`に対しても各項でBigInt
`b*n`を構築し、増大する部分和へ一般多倍長乗算している。2冪shiftをstate構築時に
一度だけ検出し、数学的に等価な`sum *= n; sum <<= k`へ分解する。

- `b`が2冪でない場合は従来の`sum *= b*n`を維持する。
- common denominatorの既存2冪判定もallocationしない同じhelperへ統合する。
- 旧一般積recurrenceとのlower/upper exact一致を整数、dyadic、非dyadic分母で回帰する。
- tail補正への適用はloop-onlyとのA/Bでallocationとpeakを比較し、全代表経路で
  独立した改善にならなければこのsliceへ含めない。
- directed bounds、項数、logical work、公開protocolを変更しない。
