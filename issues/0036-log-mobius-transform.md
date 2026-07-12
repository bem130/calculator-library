# Issue 36: log級数変数をcanonical partsから直接構築する

range-reduced `log(x)` は `z=(x-1)/(x+1)` を、Rational subtraction、addition、
divisionとして順に評価し、3回canonical化する。canonical正値 `x=n/d` では
`z=(n-d)/(n+d)` なので、BigInt partsを構築して最後に一度だけcanonical化できる。

- `1 <= x <= 2`の境界、整数、非整数、約分が必要なpartsで旧式とexact一致させる。
- `log(1)=0` fast pathとrange reduction、正項級数tailを維持する。
- `ln(2)`、general power、log積、明示exp複合経路をbefore/after測定する。
- directed bounds、logical work、resource accounting、no-float、公開protocolを変えない。
- 全gateとsubagentの差分・全体整合・merge粒度review後にmainへ一度だけ統合する。
