# Issue 115: Balance the direct dyadic exponential finite sum

## Problem

For non-degenerate dyadic exponential endpoints, the direct Taylor recurrence
repeatedly shifts the already-growing sum numerator and multiplies the growing
term numerator. In the `2^sqrt(2)` public path, four endpoint callsites account
for about 78 KB of the 101 KB deterministic allocation. Earlier work optimized
factor order, buffers, the final denominator, and exact-point term selection,
but did not change the linear finite-sum construction.

## Requirements

- Prototype an exact balanced segment representation for the dyadic Taylor
  ratios. Preserve the factorial base plus checked binary shift representation;
  do not materialize a giant power-of-two denominator at each merge.
- Keep term count, tail proof, directed rounding, positivity, monotonicity,
  refinement, stopping, logical-work/resource accounting, no-float policy, and
  the public protocol unchanged.
- Initially restrict dispatch to sufficiently long nonnegative direct dyadic
  series. Preserve zero/short, general Rational, negative reciprocal,
  binary-scaled, and value-aware tiny paths until independently measured.
- Compare the balanced state exactly with the legacy linear recurrence at zero,
  one, threshold boundaries, ordinary and large dyadic numerators, paired and
  directed bounds, and shared-denominator endpoint paths.
- Adopt only if deterministic bytes, blocks, peak allocation, and native timing
  are all non-regressing for general power and composite controls. Remove the
  prototype and record a negative result otherwise.
- Record logical work, Wasm/npm, CLI/example/browser, and repository gates;
  complete diff, consistency, and merge-granularity reviews before one
  integration into `main`.
