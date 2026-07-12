# Issue 30: non-degenerate acos endpointでπ enclosureを共有する

非dyadic入力のacosは上下Rational endpointを別評価する必要があるが、入力非依存の
同一π boundsまで各endpointで再構築する。πだけを一度共有し、endpoint固有asin、
反単調方向、special endpoint、logical workを維持する。`acos(1/3)`で測定し、
全gate/review後に一度だけ統合する。
