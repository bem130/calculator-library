# Issue 125: Compose negative high-asin atan endpoints before dyadic rounding

## Problem

Issue 124 keeps positive non-degenerate asin endpoints at `x^2>=1/2` raw
through `pi/2-atan(sqrt(1-x^2)/x)`, but negative endpoints still use the
canonical Rational transform before immediate directed dyadic rounding.  The
odd-function equivalent should not require those large normalized
intermediates.

## Requirements

- Derive negative high-transform endpoints from a positive-magnitude raw
  computation with the correct reversed rounding direction, without adding an
  input-specific branch.
- Preserve strict sign, monotonicity, directed enclosure, requested precision,
  endpoint ordering, exact/special/mid-transform/unit fallbacks, cancellation,
  logical-work/resource accounting, determinism, no-float, and public protocol.
- Cover canonical oracle values, sign/region boundaries, non-degenerate public
  dispatch with and without shared pi, allocation, native/logical/Wasm/npm/CLI
  and browser paths, and run all repository gates and reviews.

## Resolution

Negative high-transform endpoints now classify by their sign-independent
square, evaluate the positive magnitude through the Issue 124 raw composition
with the opposite directed bound, and negate the final dyadic. `-1`, coarse
ratio fallback, mid-transform and unit-series endpoints retain their canonical
paths.

Against main `70f51f0`, `asin((-2-sin(1))/3)` moved from 759,042 bytes / 2,252
blocks (41,462 / 89 peak) to 615,922 / 2,171 (37,414 / 87 peak). Native ranges
moved from 21.75--23.28 ms to 1.292--1.391 ms. The added target consumes
200,622 logical-work units; full base/candidate logical-work output remained
byte-identical at SHA-256
`5a4b4523522a0ec4d04b62587705fdde69cb53b329c608fcce33ef07b123c863`.
Optimized Wasm moved from 838,728 to 838,881 bytes and remains within budget.
The v26 npm case moved from 128.257 to 6.093 ms/iteration with the same
1,796-byte payload.
