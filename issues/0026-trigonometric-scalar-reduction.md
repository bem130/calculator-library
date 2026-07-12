# Issue 26: trig range reductionをprimitive scalar除算する

divisor>1のtrig縮約は整数Rationalを構築して汎用Rational除算へ渡している。
正のu32 divisorを既存分母へ直接掛け、一度だけcanonical化して中間値を除く。
旧汎用除算とのexact一致、range composition、tan pole、logical workを維持し、
native allocationを測定して全gate/review後に一度だけ統合する。
