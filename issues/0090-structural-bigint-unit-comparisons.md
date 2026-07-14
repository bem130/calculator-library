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

## Resolution

Comparison-only BigInt constants were replaced with structural predicates.
Exact-rational allocation moved 12,590 / 530 to 12,582 / 529, algebraic
99,251 / 3,730 to 99,235 / 3,728, wide product 72,802 / 2,648 to 72,794 /
2,647, and approximate 172,264 / 2,725 to 172,232 / 2,721. Peaks and logical
work were unchanged. The final 825,509-byte Wasm artifact SHA-256 is
`48096c109f8483bf0b6e9f190f30ded5b4d5ef6f23f9166cc43f5d091754a077`.
