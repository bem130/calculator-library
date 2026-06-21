# calculator-library

Exact calculator library implemented as a Rust core with Wasm, npm, CLI, and web example adapters.

Author: `bem130`  
License: `MIT`

## Current Status

The implementation is following [doc/design.md](doc/design.md). The current working surface is Phase 1 exact rational arithmetic:

- `calculator-core` parses and evaluates rational expressions without `f32` / `f64`.
- Rational evaluation carries an internal exact-dyadic certified interval and can produce exact significant-digit scientific notation.
- `sqrt` preserves perfect-square rational results exactly and returns a certified enclosure with `Partial` for irrational rational inputs such as `sqrt(2)`.
- `e` and `pi` constants return certified enclosures with `Partial` until requested decimal digits are confirmed.
- `exp` / `log` identities are exact when proven over rational values, including `exp(0)`, `log(1)`, `exp(log(x))` for proven positive rational `x`, and `log(exp(x))` for rational `x`; `exp(1)` returns the certified enclosure for `e`.
- `calculator-cli` evaluates exact expressions such as `0.1 + 0.2`.
- `calculator-wasm` exposes DTO-based calculation through `wasm-bindgen`.
- `packages/calculator` provides TypeScript facades for calculation and headless session dispatch over the Wasm module.
- `examples/vanilla-web` is a browser example using the public npm facade.

Later phases in the design document, including transcendental interval evaluation, symbolic simplification, special angles, and algebraic numbers, are still in progress.

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
clipboard copy, worker cancellation, rational scientific/enclosure output, and
guarded `exp` / `log` identities.

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
