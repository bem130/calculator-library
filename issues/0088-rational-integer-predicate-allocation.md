# Issue 88: Avoid allocating the Rational integer predicate constant

## Problem

`Rational::is_integer` compares its canonical denominator with a newly
constructed `BigInt::one()`. The predicate is used repeatedly by arithmetic
dispatch, and Issue 87's integer multiplication fast path calls it several times
per product. For `1*2*...*128`, the faster path reduced allocated bytes and
elapsed time but increased allocation count from 3,212 to 3,664 blocks.

The denominator is already stored as a positive canonical BigInt. Testing its
existing value for one does not require constructing another arbitrary-precision
integer.

## Required change

- Implement the integer predicate using the existing denominator's structural
  one test without allocating a comparison operand.
- Keep the public Rational representation, canonicality, exact arithmetic,
  logical-work accounting, resource limits, no-float policy, and protocol
  unchanged.
- Demonstrate that the Issue 87 wide integer product's block-count regression is
  removed and that exact rational, mixed arithmetic, approximate, symbolic, and
  algebraic controls do not regress.

## Acceptance

- Focused tests cover integer and fractional canonical values, including large
  signed integers and denominators.
- Native allocation, Criterion, logical-work, and Wasm/npm measurements record
  before/after values and remaining bottlenecks.
- Focused tests, package/example build, browser E2E, repository gates, diff
  review, whole-system consistency review, and merge-granularity review complete
  before one integration into `main`.

## Resolution

The predicate now calls `is_one()` on the stored canonical denominator. For
`wide_multiply_128`, allocation fell from 80,930 bytes / 3,664 blocks to
72,802 / 2,648 with unchanged 18,904 / 511 peak and 1,339 work units. The same
change reduced wide-add and every measured exact/approximate control. Criterion
moved from `[183.60,212.70]` to `[165.64,205.12]` us. The v19 Wasm smoke moved
from 1.379 to 1.304 ms with its 2,162-byte payload unchanged; final artifact
SHA-256 is `091ea180ff8623ceba34aeabfd5cd810daeb062d396a2a8ff9e455bde7a439b9`
at 825,688 bytes.
