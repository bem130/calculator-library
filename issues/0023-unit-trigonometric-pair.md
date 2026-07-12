# Issue 23: unit-range paired trigからidentity compositionを除く

tanはsin/cos両成分を必要とするが、`|x| <= 1`でも`sin_cos_rational`は
identityのTrigPairを初期化し、factorとの角加算として4区間積、加減算、clampを行う。

- divisor=1では両級数の方向付きboundを直接dyadic pairへ変換する。
- divisor>1のbinary angle compositionとtanのdivision/pole semanticsを維持する。
- direct pairと従来identity compositionのexact interval一致を正負・境界で回帰する。
- `tan(1)`のnative allocation・timing・logical work・Wasm境界をbefore/afterで記録する。
- resource limit、no-float、公開protocolを変えず、全gate/review後に一度だけ統合する。
