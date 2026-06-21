# calculator-library

Exact calculator library implemented as a Rust core with Wasm, npm, CLI, and web example adapters.

Author: `bem130`  
License: `MIT`

## Current Status

The implementation is following [doc/design.md](doc/design.md). The current working surface is Phase 1 exact rational arithmetic:

- `calculator-core` parses and evaluates rational expressions without `f32` / `f64`.
- `calculator-cli` evaluates exact expressions such as `0.1 + 0.2`.
- `calculator-wasm` exposes DTO-based calculation through `wasm-bindgen`.
- `packages/calculator` provides a TypeScript facade over the Wasm module.
- `examples/vanilla-web` is a browser example using the public npm facade.

Later phases in the design document, including certified arbitrary-precision approximation, symbolic simplification, special angles, and algebraic numbers, are still in progress.

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

## Verification

Common local checks:

```sh
cargo fmt --all --check
cargo test --workspace
cargo clippy --all-targets --all-features -- -D warnings
cargo check -p calculator-core --no-default-features
cargo test -p calculator-core --no-default-features
cargo xtask check-generated
cargo xtask check-no-floats
corepack pnpm --dir packages/calculator run check
corepack pnpm --dir examples/vanilla-web run build
cargo doc --workspace --no-deps
```
