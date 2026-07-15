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

