# Issue 24: trig range compositionのidentity乗算を一般に除く

`sin_cos_rational`のbinary angle compositionはidentity pairから開始するため、
divisor>1でも最初のset bitで結果を変えない4区間積・加減算・clampを行う。

- range-reduced factorを初期resultとし、残り`divisor-1`乗だけをbinary compositionする。
- 従来のcompositionとのexact interval一致、方向付き保証、tan pole semanticsを維持する。
- `sin(2)` / `cos(2)` / `tan(2)`をnative、logical work、Wasmでbefore/after測定する。
- resource limit、no-float、公開protocolを変えず、全gate/review後に一度だけ統合する。
