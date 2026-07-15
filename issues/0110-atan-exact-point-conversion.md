# Issue 110: Convert exact arctangent points once

## Problem

`atan` converts both certified dyadic endpoints to Rational before ordered/equal
classification. Canonical exact points repeat the same BigInt shift and
Rational construction before entering the paired unit or reciprocal path.

## Requirements

- Detect canonical exact dyadic points before conversion and convert once for
  zero, unit-series, and reciprocal atan evaluation.
- Preserve noncanonical Rational-equal fallback, non-degenerate directed
  endpoints, ordering/error precedence, shared pi, and pole-free atan semantics.
- Measure unit, reciprocal, and non-degenerate atan allocation/peak, logical
  work, native timing, and Wasm/npm boundary; add focused regressions.
- Preserve precision, resource accounting, no-float policy, and public protocol;
  complete reviews and repository gates before one ff-only main integration.

## Resolution

Canonical equal dyadic endpoints are now converted once and passed to the
existing paired atan evaluator. Structurally unequal endpoints retain ordered
two-endpoint conversion and their Rational-equality fallback.

Against base `23d6190`, unit `atan(1/2)` moved from 10,524 bytes / 433 blocks
(2,143 / 37 peak) to 10,484 / 428 (2,127 / 35), reciprocal `atan(2)` from
27,441 / 610 (4,296 / 45) to 27,409 / 606 (4,280 / 43), while non-degenerate
atan remained identical at 357,013 / 1,558 / 17,384. Logical-work output
retained SHA-256
`a925d3238a37ac073ae380a8c0200c9c654944a71f9a3e573660740d55d6fbd7`.
An unpaired ten-sample unit run measured 9.485--9.966 us and is not a timing
claim.

The optimized Wasm artifact is 830,694 bytes at
`07fe324b47a1e1c7dbdf8ce8307a7947df8931e99beb1acaccdb4c5d5a2afd7b`,
within budget. The ten-iteration/two-warmup unit boundary retained its
1,772-byte payload and measured 0.364 ms/iteration; this is not a timing claim.

Repository gates passed with 383 core tests, 37 native Wasm tests, and 23
wasm32 tests, plus formatting, clippy, no-default-feature, doctest, generated
contract, protocol snapshot, no-float, dependency policy, package/example
frozen install, TypeScript/package checks, example build, external oracle,
package-size, browser E2E, and rustdoc gates. Both exact pnpm audit requests
were unavailable because the registry endpoint returned HTTP 410; the paired
`--ignore-registry-errors` checks completed, and manifests and lockfiles were
unchanged.
