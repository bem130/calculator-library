# Issue 46: binary log係数をsigned primitive scalingする

log range compositionとlarge-exp residualは有界な`i64` binary exponentをRationalへ
変換し、汎用Rational乗算へ渡す。signed primitiveを既存分子へ直接掛けて一度だけ
canonical化すれば、中間Rational operandと負係数用negate、汎用dispatchを除ける。

- zero、正負、step上限、large-exp exponent範囲で旧汎用乗算とexact一致させる。
- log2 endpoint方向、exp residual、logical work、resource、protocolを維持する。
- log/large-exp allocationを測定し、全gateとsubagent review後に統合する。
