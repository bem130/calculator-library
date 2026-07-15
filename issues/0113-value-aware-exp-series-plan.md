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
- Apply it to canonical exact direct points. Preserve the shared precision-only
  plan for non-degenerate endpoints, whose public canonical denominators do not
  provide a measured value-aware win.
- Preserve binary scaling and general Rational fallback until their reduced
  arguments have an independently proven value-aware plan.
- Preserve directed rounding, positivity, monotonicity, refinement, stopping,
  logical-work/resource accounting, and public protocol.
- Add boundary/oracle regressions for zero, tiny dyadics, ordinary fractions,
  near-one values, non-degenerate intervals, and unchanged fallback paths.
- Measure allocation/peak, native timing, logical work, Wasm/npm, and package/UI
  behavior; complete reviews and all gates before one ff-only integration.

## Resolution

Small canonical exact points now use an exact value-aware tail plan when the
denominator is at least eight bits wider than the numerator. The plan returns
the smallest proven N and its N! for the existing recurrence. Arguments closer
to one retain the precision-only plan before constructing exact powers.
Non-degenerate, binary-scaled, negative/general Rational, and incompatible
endpoint routes are unchanged; an attempted endpoint-specific extension was
removed after general-power allocation regressed from 101,393 to 283,225 bytes.

Against base `879ea79`, `exp(2^-100)` moved from 21,211 bytes / 391 blocks
(4,122 / 36 peak) to 9,819 / 356 (2,090 / 29). At 128 bits its N moved from 34
to 1. Exact half, near-one, non-degenerate, general-power, cumulative exp,
ordinary exp, and large-negative controls were allocation/peak-identical.
Expanded logical-work output retained SHA-256
`104e384cad59527a87fc0ddfee4eb5e9f67c45c6506e2fb61f07d0abc57757b0`.

Ten-sample native ranges moved from 14.60--17.85 us to 7.81--9.57 us for the
tiny case; `2^-1000` overlapped. Base/candidate Wasm artifacts were 830,694 and
831,200 bytes, with candidate SHA-256
`40d24bf9d0919a09cb73337e75b67989259de548221ef5f6e8c6360941e0a6f0`.
Ten-iteration/two-warmup npm smokes measured 0.352 and 0.363 ms/iteration with
the same 1,824-byte payload, so no Wasm timing claim is made.

Repository gates passed with 385 core tests, 37 native Wasm tests, and 23
wasm32 tests, plus formatting, native/Wasm clippy, no-default-feature, doctest,
generated contract, protocol snapshot, no-float, dependency policy,
package/example frozen install, TypeScript/package checks, example build,
external oracle, package-size, browser E2E, and rustdoc gates. Exact pnpm audit
requests were unavailable because the registry endpoint returned HTTP 410; the
paired `--ignore-registry-errors` checks completed, and manifests and lockfiles
were unchanged.
