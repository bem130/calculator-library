# Issue 45: `1-x^2`の証明済み既約性を利用する

canonical `x=n/d`では`gcd(n,d)=1`なので、非zeroの`(d^2-n^2)/d^2`も
`gcd(d^2-n^2,d^2)=1`である。Issue 44のdirect partsを汎用Rational constructorへ
戻さず直接canonical構造にし、±1のzeroだけ`0/1`へ正規化する。

- 正負、zero、±1、half、1近傍で汎用constructor結果とexact一致させる。
- denominator positivity、sqrt domain、directed asin/acos、resource契約を維持する。
- allocationを測定し、全gateとsubagent review後にmainへ統合する。
