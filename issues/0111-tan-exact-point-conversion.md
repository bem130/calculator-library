# Issue 111: Convert exact tangent points once

## Problem

`tan` converts both certified dyadic endpoints to Rational before its periodic
pole check. Canonical exact points therefore repeat the same BigInt shift and
Rational construction even though pole classification and tangent evaluation
need only one value.

## Requirements

- Detect canonical exact dyadic points before conversion and convert once while
  preserving the periodic half-pi pole check and its error precedence.
- Preserve noncanonical Rational-equal intervals, non-degenerate monotone
  endpoint evaluation, conservative pole detection, and public result states.
- Add focused regressions for ordinary, negative, near-pole, pole, structurally
  unequal equal-valued, and truly non-degenerate inputs.
- Measure exact-point and non-degenerate allocation/peak, logical work, native
  timing, and Wasm/npm boundary behavior before and after the change.
- Preserve precision, determinism, resource accounting, no-float policy, and
  public protocol; complete reviews and repository gates before one ff-only
  main integration.

## Resolution

Rejected after measurement. A prototype converted canonical interval points
once and evaluated them once while retaining the pole scan, Rational-equality
fallback, and non-degenerate route. Focused interval regressions passed, but it
did not change any measured public path: both `tan(1)` and `tan(2)` retained
11,271 bytes / 389 blocks and 14,539 / 528 respectively, and the non-degenerate
control retained 597,885 / 5,016. Peaks were also identical at 1,591 / 33,
2,511 / 72, and 18,922 / 68. Logical-work output retained SHA-256
`a925d3238a37ac073ae380a8c0200c9c654944a71f9a3e573660740d55d6fbd7`.

The reason is routing, not an optimizer failure: exact rational arguments use
`tan_rational` directly from expression evaluation, while `interval::tan` is
the fallback for non-rational certified intervals. Those fallback intervals
are non-degenerate in the representative public cases, so canonical point
classification adds code without removing public work. The prototype was
therefore removed; no runtime or protocol change is retained.

Repository gates passed with 383 core tests, 37 native Wasm tests, and 23
wasm32 tests, plus formatting, native/Wasm clippy, no-default-feature, doctest,
generated contract, protocol snapshot, no-float, dependency policy,
package/example frozen install, TypeScript/package checks, example build,
external oracle, package-size, browser E2E, and rustdoc gates. Exact pnpm audit
requests were unavailable because the registry endpoint returned HTTP 410; the
paired `--ignore-registry-errors` checks completed, and manifests and lockfiles
were unchanged.
