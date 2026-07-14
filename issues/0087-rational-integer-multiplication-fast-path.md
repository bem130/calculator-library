# Issue 87: Avoid normalization for canonical integer products

## Problem

`Rational::multiply` always constructs numerator and denominator products and
passes them through `Rational::new`, including when both canonical operands are
integers. Their denominators are both one, so the denominator product and
GCD/exact-division normalization cannot change the result. After Issue 86 removes
intermediate DAG normalization, this redundant Rational normalization remains in
every step of a wide integer product.

At base commit `772e594`, `1*2*...*128` allocates 86,818 bytes in 3,212 blocks,
peaks at 18,904 bytes / 511 blocks, and consumes 6,377 logical-work units.

## Required change

- Return canonical zero and multiplicative identities without general Rational
  normalization.
- For integer/integer operands, multiply only the numerators and construct the
  canonical integer result directly. Preserve arbitrary precision and signs.
- Keep fraction/fraction and integer/fraction products on the existing general
  normalization path in this slice; an integer may share factors with a
  fractional denominator, so direct mixed construction is not generally valid.
- Update structural logical-work accounting to charge the operations actually
  retained by each path. Do not make large integer multiplication free.
- Preserve canonical zero, positive denominator, exact results, determinism,
  resource-limit behavior, no-float policy, and the public protocol.

## Acceptance

- Zero/one, both signs, multi-limb integers, mixed and fractional controls, and
  canonical-form invariants have focused regressions.
- Wide-product allocation, logical-work, native scaling, and Wasm/npm measurements
  demonstrate the effect without regressing wide-add, exact-rational, symbolic,
  algebraic, or approximate controls.
- Focused tests, package/example build, browser E2E, full repository gates, diff
  review, whole-system consistency review, and merge-granularity review complete
  before one integration into `main`.
