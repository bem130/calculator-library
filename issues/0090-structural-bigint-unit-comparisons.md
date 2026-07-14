# Issue 90: Use structural BigInt unit predicates

## Problem

Several hot or public-boundary predicates construct `BigInt::zero()` or
`BigInt::one()` solely to compare an existing value. Rational presentation,
finite-decimal residue classification, radical classification, and polynomial
candidate checks can inspect the existing arbitrary-precision integer directly.

## Required change

- Replace comparison-only zero/one operands with structural predicates.
- Preserve exact output, canonical classification, polynomial decisions,
  resource accounting, no-float policy, and protocol.
- Record deterministic allocation for affected public paths and controls.

## Acceptance

- Focused exact, finite-decimal, radical, and polynomial tests pass.
- Representative native/Wasm measurements and repository gates pass.
- Diff, whole-system consistency, and merge-granularity reviews approve one
  integration into `main`.
