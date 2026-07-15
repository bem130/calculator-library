# Issue 114: Select reciprocal exponential terms from the exact magnitude

## Problem

Issue 113 selects a value-aware Taylor plan for small positive canonical exact
points, but an equally small negative point still selects the precision-only
plan before applying the reciprocal direction. Consequently `exp(-2^-100)`
performs growing integer work that the exact tail bound for `exp(2^-100)` proves
unnecessary. The positive and negative public paths should differ only where the
reciprocal enclosure requires it, not in their Taylor truncation quality.

## Requirements

- Reproduce time, allocation, peak allocation, and logical work for small
  negative exact dyadics before changing the algorithm, with positive, ordinary,
  near-one, non-degenerate, and large-negative controls.
- Select the Taylor plan from the exact positive magnitude before applying the
  existing reciprocal direction. Do not construct `exp(|x|)` as a giant exact
  intermediate, special-case one literal, raise limits, use floating point, or
  underflow to zero.
- Preserve strict positivity, reciprocal bound direction, monotonicity,
  refinement, determinism, stopping, logical-work/resource accounting, and the
  public protocol.
- Keep the measured eligibility boundary from Issue 113: binary-scaled,
  near-one, non-degenerate, and general Rational paths remain unchanged unless
  independently measured and proven beneficial.
- Add boundary/oracle regressions for positive/negative tiny points, zero,
  ordinary negative values, refinement, and unchanged fallback paths.
- Record native, Wasm/npm, package/example, and browser evidence; complete
  focused and repository gates, reviews, and merge-granularity review before one
  integration into `main`.
