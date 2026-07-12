# Issue 31: inverse trigの±1判定を構造比較する

asin/acos endpointは特殊値判定のたびに±1 Rationalを構築する。canonical Rationalの
符号・分子絶対値・正分母を直接比較し、中間allocationを除く。旧exact比較、domain、
方向付きbound、logical workを維持し、測定・全gate/review後に統合する。
