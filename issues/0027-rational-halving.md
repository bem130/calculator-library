# Issue 27: interval Rationalのhalvingをprimitive scalar化する

`halve_rational`は定数2のRationalを毎回構築して汎用除算へ渡す。π/2および
asin/acos/atan変換で反復されるため、既存の正primitive scalar除算へ接続する。
旧汎用除算とのexact一致、符号、方向付きbound、logical workを維持し、影響経路の
allocationを測定して全gate/review後に一度だけ統合する。
