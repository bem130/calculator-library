# Current Implementation Status

この文書は現行実装の状態を記録する。利用者向けの公開契約は [`public-contract.md`](public-contract.md) を正本とし、この文書の内部構造・アルゴリズム名・テスト構成は公開契約ではない。

## Implemented Surface

現行実装は Rust core、Wasm adapter、npm facade、vanilla web example、CLI、WASI crate を持つ。core は `#![no_std] + alloc` で、浮動小数点型 `f32` / `f64` を使わない。

主な実装済み領域:

* 有理数の exact parse、四則演算、整数累乗、percent lowering。
* `0.1 + 0.2 = 3/10` などの decimal lossless evaluation。
* real principal power semantics に基づく rational power の exact root / domain error / symbolic fallback。
* exact dyadic certified interval と adaptive scientific rounding。scientific output は exact rational だけでなく、certified interval の上下端を同じ有効桁数・丸めモードで丸めて一致する場合にも confirmed digits として返す。
* `pi`、`e`、exp/ln/base-explicit log と逆三角関数合成の証明可能な恒等式。`exp(ln(x))` は正性を証明できる supported rational/radical/radical-linear/algebraic `x` で exact にし、`ln(exp(x))` は supported exact `x` で exact にする。`log(argument, base)` は有理数の整数べき、同じ正の有理数 basis の rational power、basis 同士が整数べき関係にある rational power で exact にし、`ln(argument)` は底 `e` の自然対数として受ける。底を省略した `log(argument)` は受けない。`sin(asin(x))` と `cos(acos(x))` は `x in [-1, 1]` を証明できる supported exact `x` で exact にし、`cos(asin(x))` と `sin(acos(x))` は `sqrt(1 - x^2)` を supported exact 値として構築できる場合に exact にし、`tan(atan(x))` は supported exact real `x` で exact にする。bounded rational/dyadic endpoint に対する exp/log/asin/acos/atan、rational point trigonometric range reduction、周期的な sin/cos extrema、tan pole-aware branch、正の底が証明できる一般実数指数 `x^y` の certified interval。
* GeneralSymbolic exact presentation と条件付き exact evaluation における安全な対数積・商・冪、証明済み非ゼロの self-division、証明済み非負の `sqrt(x^2)`、奇偶性、整数 `pi` シフト、`sin` / `cos` の半整数 `pi` cofunction shift、`tan` の半整数 `pi` reciprocal shift の正規化。例: `ln(2*3) = ln(2)+ln(3)`、`ln(2/3) = ln(2)-ln(3)`、`ln(sqrt(2)) = 1/2*ln(2)`、`exp(2)/exp(2) = 1`、`sqrt(exp(2)^2) = exp(2)`、`sin(-1) = -sin(1)`、`cos(-1) = cos(1)`、`sin(pi+1/10) = -sin(1/10)`、`cos(pi/2+1/10) = -sin(1/10)`、`tan(pi/2+1/10) = -1/tan(1/10)`、`exp(sin(pi/2+1/10)) = exp(cos(1/10))`。
* rational pi multiple recognition。
* simple radical と radical linear combination の exact presentation、積・商・整数累乗の bounded reduction。recognized exact radical/radical-linear/algebraic 値の certified enclosure は、再度元式全体を区間評価せず、証明済み exact 値から構築する。
* rational/simple-radical special angles と inverse trigonometric known values。
* bounded real algebraic recognition for supported polynomial operations、整数累乗、符号を証明できる real algebraic の主 n 乗根と有理指数、cyclotomic trigonometric cases、degree-one algebraic result の rational collapse と後続の代数的演算への伝播。
* parser/session/DTO/native-Wasm/browser conformance tests。
* resource limit enforcement before or during expensive evaluation paths。
* npm facade の `presentInput` による、評価とは独立した入力式 presentation tree。

## Hardened Gates

Phase 5 の堅牢化として、次を CI に入れている。

* Rust formatting、Clippy、wasm target Clippy。
* core no-default check/test、workspace test、Rust doctest、wasm-pack test。
* generated DTO check、protocol snapshot check、DTO regeneration diff check。
* `calculator-core` 内の `f32` / `f64` 禁止。
* `cargo deny check` と pnpm audit。
* TypeScript package check、vanilla example build、browser e2e。
* external arithmetic oracle。
* package size budget。
* public enum match exhaustiveness lint。

## Deliberately Not Contract

次は実装詳細であり、公開契約として扱わない。

* 内部 module 構成。
* exact expression DAG の node storage layout。
* polynomial factorization / resultant / root isolation の具体的な探索順。
* special angle recognition の内部 dispatch 順。
* cache の有無、warm/cold の内部挙動。
* package size budget の数値そのもの。ただし CI gate としての存在は公開品質基準である。

## In Progress

`design.md` の最終目標には、さらに広い transcendental interval evaluation、一般実代数的数の完全な演算閉包、より広い symbolic simplification、最終的な 1.0 release hardening が含まれる。現行実装はこれらをすべて完了したとは扱わない。

未対応または制限超過の領域では、厳密式を破壊して近似値へ落とさず、`Partial`、`unsupportedFeature`、`computationLimit`、または `inputLimit` として返す。
