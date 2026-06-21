# calculator-library

Exact calculator library implemented as a Rust core with Wasm, npm, CLI, and web example adapters.

Author: `bem130`  
License: `MIT`

## Current Status

The implementation is following [doc/design.md](doc/design.md). The current working surface includes Phase 1 exact rational arithmetic plus selected Phase 2 and Phase 3 exact/certified behavior:

- `calculator-core` parses and evaluates rational expressions without `f32` / `f64`.
- Rational evaluation carries an internal exact-dyadic certified interval and can produce exact significant-digit scientific notation.
- Integer powers and rational powers follow the `RealPrincipal` semantics: perfect roots such as `(-8)^(1/3)` and `(-8)^(2/3)` are exact, non-perfect roots such as `2^(1/2)` return certified enclosures with `Partial`, and negative bases with non-rational exponents report `NonRealPower`.
- `sqrt` preserves perfect-square rational results exactly, recognizes simple radicals such as `sqrt(72) = 6sqrt(2)`, `sqrt(6962) = 59sqrt(2)`, `sqrt(1/2) = sqrt(2)/2`, and `2^(1/2) = sqrt(2)`, and reduces simple-radical products, quotients, and like terms such as `sqrt(2) * sqrt(2) = 2`, `sqrt(2) * sqrt(3) = sqrt(6)`, and `sqrt(8) / sqrt(2) = 2`.
- `pi`, `pi/6`, and other rational multiples of `pi` are recognized structurally as exact `RationalPiMultiple` values and return certified enclosures with `Partial` until requested decimal digits are confirmed.
- The `e` constant returns a certified enclosure with `Partial` until requested decimal digits are confirmed.
- `exp` / `log` identities are exact when proven over rational values, including `exp(0)`, `log(1)`, `exp(log(x))` for proven positive rational `x`, and `log(exp(x))` for rational `x`; `exp(1)` returns the certified enclosure for `e`.
- Rational and simple-radical special angles are exact when the DAG proves the argument is a supported rational multiple of `pi`: examples include `sin(pi/6) = 1/2`, `cos(pi/3) = 1/2`, `sin(pi/4) = sqrt(2)/2`, `tan(pi/3) = sqrt(3)`, and `tan(pi/2)` as `domain.tangentPole`.
- Forward trigonometric functions lower degree and gradian inputs to exact radian expressions before evaluation, so `sin(30)` in degree mode is exact `1/2`.
- Inverse trigonometric known values are exact for supported rational arguments: examples include `asin(1/2) = pi/6` in radian mode, `asin(1/2) = 30` in degree mode, `acos(-1) = pi`, and `atan(1) = pi/4`.
- `calculator-cli` evaluates exact expressions such as `0.1 + 0.2`.
- `calculator-wasm` exposes DTO-based calculation through `wasm-bindgen`.
- `packages/calculator` provides TypeScript facades for calculation and headless session dispatch over the Wasm module.
- `examples/vanilla-web` is a browser example using the public npm facade.

Later phases in the design document, including broader transcendental interval evaluation, symbolic simplification, full square-free factorization, cyclotomic exact trig, and algebraic numbers, are still in progress.

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

## Vanilla Web Example

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
guarded `exp` / `log` identities, exact rational power semantics, and rational
`pi` multiple output, rational and radical special-angle output, inverse
trigonometric known values, simple radical output and algebra, and `tan` pole
errors.

## Verification

Common local checks:

```sh
cargo fmt --all --check
cargo test --workspace
cargo clippy --all-targets --all-features -- -D warnings
cargo check -p calculator-core --no-default-features
cargo test -p calculator-core --no-default-features
cargo deny check
cargo xtask generate-types
cargo xtask check-generated
cargo xtask check-no-floats
git diff --exit-code
corepack pnpm --dir packages/calculator run check
corepack pnpm --dir examples/vanilla-web run build
corepack pnpm --dir examples/vanilla-web run test:e2e
cargo doc --workspace --no-deps
```
