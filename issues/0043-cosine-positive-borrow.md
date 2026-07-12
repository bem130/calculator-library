# Issue 43: 正unit cosine入力を借用する

`cos_unit_rational_bounds`は偶関数の負入力を正規化するため、正入力でもRational全体を
cloneしてから級数へ渡す。負入力だけowned negateを保持し、非負入力は借用して不要な
BigInt allocationを除く。

- 正負、zero、unit境界で旧owned経路とexact一致させる。
- cosine偶関数性、級数tail、range reduction、logical workを維持する。
- allocationをbefore/after測定し、全gateとsubagent review後に統合する。
