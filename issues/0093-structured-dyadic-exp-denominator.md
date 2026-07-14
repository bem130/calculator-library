# Issue 93: Keep dyadic exponential denominators structured through rounding

## Problem

For a Taylor input denominator `q = 2^k`, exponential evaluation materializes
the final common denominator as `N! << (k*N)` and then divides a similarly large
scaled numerator by it. The recurrence already treats `q` as a shift, but loses
that structure at the final directed-dyadic rounding boundary.

## Requirements

- Represent the dyadic final denominator as a factorial base plus checked binary
  shift through lower, upper-tail, exact-point, directed, and shared-endpoint paths.
- Compute exact directed floor/ceil without materializing the shifted denominator;
  cover precision smaller than, equal to, and larger than the denominator shift.
- Preserve signed floor/ceil semantics, upper-tail enclosure, positivity,
  monotonicity, determinism, stopping, cancellation, and all resource errors.
- Preserve the general non-power-of-two denominator path and rational-only test
  helpers.
- Do not change precision, Taylor term selection, logical-work accounting,
  no-float policy, or public protocol.
- Measure an amplified `exp(2^-100)` path plus ordinary and general-power controls;
  reject the implementation if no material allocation or timing benefit appears.
- Add exact equivalence/property regressions and complete native/Wasm/package/
  example gates plus diff, consistency, and merge-granularity review.
