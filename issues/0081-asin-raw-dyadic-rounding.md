# Issue 81: Round unit arcsine raw fractions directly to dyadics

## Problem

The `|x| <= 1/2` arcsine Taylor recurrence ends with exact common-denominator
numerator/denominator parts. Public certified interval evaluation canonicalizes
both bounds into Rational values with large GCD/division work, then immediately
converts those Rationals back to directed dyadic endpoints.

On 2026-07-15 with `rustc 1.97.0`, one public `asin(1/3)` calculation allocates
426,740 bytes in 1,031 blocks with peak live allocation of 17,079 bytes. This is
the largest clear redundant normalize-then-round path in the current ordinary
approximate benchmarks.

## Requirements

- Expose raw lower and upper-tail fraction parts from the unit arcsine recurrence
  and round them directly with exact directed floor/ceil.
- Apply to exact-point and directed non-degenerate endpoints in `|x| <= 1/2`.
- Preserve zero, signs/odd symmetry, boundary `1/2`, positive-series tail,
  monotonicity, certified enclosure, precision, and typed errors.
- Preserve transformed `|x| > 1/2`, acos composition, logical-work accounting,
  resource limits, no-float policy, and public protocol.
- Compare raw and legacy canonical Rational bounds across signs, precisions, term
  counts, exact/nonexact divisions, and overflow boundaries.
- Measure `asin(1/3)`, `asin(sin(1)/3)`, transformed controls, native timing,
  logical work, Wasm/npm facade, and example path before integration.
- Complete diff, consistency, merge-granularity reviews and all repository gates.
