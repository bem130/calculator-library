# calculator-library

Exact calculator library implemented as a Rust core with Wasm, npm, CLI, and web example adapters.

Author: `bem130`  
License: `MIT`

## Current Status

The implementation is following [doc/design.md](doc/design.md). The current working surface includes exact rational arithmetic, certified interval evaluation, selected symbolic simplification, special values, bounded real algebraic recognition, Wasm/npm facades, and a deployed browser example:

- `calculator-core` parses and evaluates rational expressions without `f32` / `f64`.
- Rational evaluation carries an internal exact-dyadic certified interval and can produce exact significant-digit scientific notation.
- Rational exact output honors display preferences: finite decimal output is used only when it is exact, and improper rationals can be rendered as mixed fractions.
- Integer powers and rational powers follow the `RealPrincipal` semantics: perfect roots such as `(-8)^(1/3)` and `(-8)^(2/3)` are exact, non-perfect roots such as `2^(1/2)` return certified enclosures with `Partial`, and negative bases with non-rational exponents report `NonRealPower`.
- General real powers with a certified positive base use interval composition through `exp(y ln(x))`, so expressions such as `2^sqrt(2)` and `sqrt(2)^sqrt(2)` retain exact symbolic output while returning certified enclosures.
- `sqrt` preserves perfect-square rational results exactly, recognizes simple radicals such as `sqrt(72) = 6*sqrt(2)`, `sqrt(6962) = 59*sqrt(2)`, `sqrt(1/2) = sqrt(2)/2`, and `2^(1/2) = sqrt(2)`, and reduces simple-radical products, quotients, and like terms such as `sqrt(2) * sqrt(2) = 2`, `sqrt(2) * sqrt(3) = sqrt(6)`, and `sqrt(8) / sqrt(2) = 2`.
- Radical exact output supports linear combinations of rational and simple-radical terms, such as `sin(pi/6) + sqrt(2) = 1/2 + sqrt(2)` and `sqrt(3) + sqrt(2) = sqrt(2) + sqrt(3)`.
- `pi`, `pi/6`, and other rational multiples of `pi` are recognized structurally as exact `RationalPiMultiple` values and return certified enclosures with `Partial` until requested decimal digits are confirmed.
- The `e` constant returns a certified enclosure with `Partial` until requested decimal digits are confirmed.
- `exp` / `ln` / base-explicit `log` identities are exact when proven over supported exact values, including `exp(0)`, `ln(1)`, `log(8,2) = 3`, `log(2^(1/3),2) = 1/3`, `log(8,sqrt(2)) = 6`, `log(sqrt(2),8) = 1/6`, `ln(8)/ln(2) = 3`, `log(8,3)/log(2,3) = 3`, `log(8,7)*log(7,3)*log(3,2) = 3`, `log(2,10)+log(5,10) = 1`, `ln(e) = 1`, `exp(3,2) = 8`, `exp(ln(x))` for proven positive rational/radical/radical-linear/algebraic `x`, and `ln(exp(x))` for supported exact `x`; guarded inverse-trig identities include direct forms such as `sin(asin(x))` and cofunction forms such as `cos(asin(sqrt(2)/3)) = sqrt(7)/3`; `exp(1)` returns the certified enclosure for `e`.
- General symbolic exact output normalizes safe logarithm products and powers, proven nonzero self-division, proven nonnegative square roots of squares, odd/even function presentation, integer-`pi` trigonometric shifts, `sin` / `cos` half-`pi` cofunction shifts, and `tan` half-`pi` reciprocal shifts, such as `ln(2*3) = ln(2)+ln(3)`, `ln(sqrt(2)) = 1/2*ln(2)`, `exp(2)/exp(2) = 1`, `sqrt(exp(2)^2) = exp(2)`, `sin(-1) = -sin(1)`, `cos(-1) = cos(1)`, `sin(pi+1/10) = -sin(1/10)`, `cos(pi/2+1/10) = -sin(1/10)`, `tan(pi/2+1/10) = -1/tan(1/10)`, and nested cases like `exp(sin(pi/2+1/10)) = exp(cos(1/10))`.
- Rational and simple-radical special angles are exact when the DAG proves the argument is a supported rational multiple of `pi`: examples include `sin(pi/6) = 1/2`, `cos(pi/3) = 1/2`, `sin(pi/4) = sqrt(2)/2`, `sin(pi/12) = sqrt(6)/4 - sqrt(2)/4`, `tan(pi/12) = 2 - sqrt(3)`, and `tan(pi/2)` as `domain.tangentPole`.
- Forward trigonometric functions lower degree and gradian inputs to exact radian expressions before evaluation, so `sin(30)` in degree mode is exact `1/2`.
- Inverse trigonometric known values are exact for supported rational and simple-radical arguments: examples include `asin(1/2) = pi/6`, `asin(sqrt(2)/2) = pi/4`, `atan(sqrt(3)) = pi/3` in radian mode, and `asin(1/2) = 30` / `atan(sqrt(3)) = 60` in degree mode.
- Extended calculator functions include `root`, `abs`, `floor`, postfix `!` / `fact`, `perm`, `comb`, `mod`, `gcd`, `lcm` / `lcd`, and hyperbolic / inverse-hyperbolic functions. Integer combinatorics and divisibility functions reduce exact integer cases directly; hyperbolic functions lower through `exp`, `ln`, and `sqrt` so existing exact simplification also applies inside larger expressions such as `exp(sinh(0)) = 1` and `ln(cosh(0)) = 0`.
- Certified interval evaluation covers constants, supported elementary functions, rational point trigonometric range reduction, periodic `sin` / `cos` extrema, `tan` pole-aware monotone branches, and positive-base general powers.
- Bounded real algebraic recognition covers supported prime rational powers, algebraic sums/products/quotients/integer powers, cyclotomic exact trigonometric values such as `sin(pi/5)`, `cos(pi/5)`, and `tan(pi/5)`, and rational collapse of degree-one algebraic results such as `2^(1/3)-2^(1/3) = 0` and `2^(1/3)/2^(1/3) = 1`. Nested rational collapses are propagated into later algebraic operations, so `((2^(1/3)-2^(1/3))+2)^(1/3)` is reduced to `2^(1/3)` before its parent is evaluated.
- `calculator-cli` evaluates exact expressions such as `0.1 + 0.2`.
- `calculator-wasm` exposes DTO-based calculation and input presentation through `wasm-bindgen`.
- `packages/calculator` provides TypeScript facades for calculation, input presentation, presentation and result-relation rendering to plain text / MathML / LaTeX, and headless session dispatch over the Wasm module.
- `examples/vanilla-web` is a browser example using the public npm facade.

Remaining design work includes broader transcendental interval evaluation beyond the current supported function set, wider symbolic simplification, algebraic operation coverage beyond the current bounded supported cases, and final 1.0 release hardening.

## Native CLI

```sh
cargo run -p calculator-cli -- "0.1 + 0.2"
```

Expected output:

```text
3/10
```

Domain errors are reported as stable code-like strings:

```sh
cargo run -p calculator-cli -- "1 / 0"
```

```text
domain.divisionByZero
```

## npm Package

TypeScript checks:

```sh
corepack pnpm --dir packages/calculator run check
```

Build the Wasm package used by the facade:

```sh
corepack pnpm --dir packages/calculator run build:wasm
```

The TypeScript facade exposes `createCalculator()` for direct expression calculation and `createSession()` for button/input workflows. Session dispatch is headless: `Evaluate` returns a calculate command, and the caller passes the result back through `applyResult()`.

For a step-by-step guide to building a custom calculator UI with the public API,
see [doc/tutorial/README.md](doc/tutorial/README.md).

## Vanilla Web Example

Public GitHub Pages deployment:

https://bem130.github.io/calculator-library/

Install dependencies:

```sh
corepack pnpm --dir examples/vanilla-web install
```

Run locally:

```sh
corepack pnpm --dir examples/vanilla-web run dev
```

Build for GitHub Pages:

```sh
corepack pnpm --dir examples/vanilla-web run build
```

The Pages workflow in [.github/workflows/pages.yml](.github/workflows/pages.yml) builds `examples/vanilla-web/dist` and deploys it from `main`.

The example e2e test covers the public worker API path, MathML rendering,
clipboard copy, worker cancellation, rational scientific/enclosure output,
guarded `exp` / `log` identities over rational, radical, and radical-linear
values, exact rational power semantics, guarded inverse-trig direct and cofunction
identities, positive-base general power intervals, symbolic
trigonometric `pi`, half-`pi` cofunction, and tangent reciprocal shift presentation, and rational
`pi` multiple output, rational and radical special-angle output, inverse
trigonometric known values, simple radical output and algebra, mixed radical
linear combinations, bounded real algebraic output, and `tan` pole errors.

## Verification

Common local checks:

```sh
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo clippy -p calculator-wasm --target wasm32-unknown-unknown --all-features -- -D warnings
cargo test --workspace
cargo check -p calculator-core --no-default-features
cargo test -p calculator-core --no-default-features
wasm-pack test --node crates/calculator-wasm
cargo deny check
cargo xtask generate-types
cargo xtask check-generated
cargo xtask check-protocol-snapshot
cargo xtask check-no-floats
node --no-warnings tools/oracle/check-rational-oracle.mjs
cargo xtask check-package-size
git diff --exit-code
corepack pnpm --dir packages/calculator audit --audit-level low
corepack pnpm --dir examples/vanilla-web audit --audit-level low
corepack pnpm --dir packages/calculator run check
corepack pnpm --dir examples/vanilla-web run build
corepack pnpm --dir examples/vanilla-web run test:e2e
cargo doc --workspace --no-deps
```
