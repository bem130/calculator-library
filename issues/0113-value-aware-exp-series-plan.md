# Issue 113: Select direct exponential terms from the exact argument

## Problem

Direct exponential Taylor evaluation selects `N` using the worst-case
`0 <= x <= 1` bound `(N+1)! >= 2^(precision+1)`. DHAT shows the two directed
general-power recurrences are the dominant allocations, while tiny exact dyadic
arguments still execute the same number of growing BigInt updates as values near
one. For exact `x=a/b`, the existing tail proof can use the tighter exact bound
`2*x^(N+1)/(N+1)! <= 2^-precision`.

## Requirements

- For arguments small enough to justify value-dependent planning, choose the
  smallest direct-series term count by exact integer comparison of the tighter
  tail bound; retain the precision-only plan near one, never use floating point,
  and never weaken the enclosure.
- Apply it to canonical exact direct points and compatible non-degenerate direct
  endpoints, choosing a plan safe for both directed endpoints.
- Preserve binary scaling and general Rational fallback until their reduced
  arguments have an independently proven value-aware plan.
- Preserve directed rounding, positivity, monotonicity, refinement, stopping,
  logical-work/resource accounting, and public protocol.
- Add boundary/oracle regressions for zero, tiny dyadics, ordinary fractions,
  near-one values, non-degenerate intervals, and unchanged fallback paths.
- Measure allocation/peak, native timing, logical work, Wasm/npm, and package/UI
  behavior; complete reviews and all gates before one ff-only integration.
