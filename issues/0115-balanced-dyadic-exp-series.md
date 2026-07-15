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

## Resolution

The prototype was rejected and no runtime code is retained. At base `f13b268`,
the deterministic `approximate_general_power` case used 101,393 bytes / 692
blocks with a 6,223-byte / 43-block peak. A fully balanced one-term-leaf tree
reduced total bytes to 82,033, but increased blocks to 1,039 and peak to 7,311 /
52. Hybrid leaves of 8, 16, and 32 terms produced respectively 72,737 / 807,
73,825 / 783, and 84,345 / 758 bytes/blocks, while all retained a roughly
7.3-KB / 52-block peak.

The structured denominator avoided repeated giant powers of two, but each tree
merge still had to hold left and right products plus a scaled partial sum. That
simultaneous live state caused the peak and block regressions. A reverse nested
evaluation removed the tree, but moved the growing shift to a materialized
denominator and measured 110,585 / 716 with a 6,511 / 47 peak. It therefore
regressed total bytes, blocks, and peak.

All variants preserved exact regression results, but none met the stated
bytes/blocks/peak adoption gate. The branch restores the original recurrence;
future work should not retry leaf-width tuning or denominator materialization.
It requires a representation that can add a scaled partial sum without keeping
both child products live.
