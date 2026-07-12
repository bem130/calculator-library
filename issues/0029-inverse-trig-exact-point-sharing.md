# Issue 29: inverse trig exact pointのendpoint評価を共有する

atan/asin/acosはlower==upperのexact pointでも同じRational endpoint boundsを2回評価し、
級数・π変換を重複構築する。exact pointでは一度のpaired boundsを共有し、非退化intervalの
単調/反単調endpoint評価、方向付き保証、domain/pole、logical workを維持する。
既存native/Wasm benchmarkで測定し、全gate/review後に一度だけ統合する。
