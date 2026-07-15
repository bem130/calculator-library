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

## Resolution

Canonical negative exact points whose positive magnitude satisfies the Issue 113
small-argument boundary now select that magnitude's exact tail plan. The paired
positive enclosure is still constructed once and inverted as `1/upper ..
1/lower`; no zero underflow, literal special case, precision change, or resource
limit change is involved. Binary-scaled, near-one, non-degenerate, and general
Rational paths retain their previous dispatch.

At 128-bit enclosure precision, deterministic one-call allocation for
`exp(-2^-100)` changed from 30,257 bytes / 460 blocks (5,519 / 34 peak) to
11,385 / 422 (2,319 / 46 peak). The peak byte count fell even though short-lived
block concurrency increased. Positive tiny, half, near-one, non-degenerate,
`exp(-2)`, `exp(-10000)`, and general-power controls retained their preceding
allocation values. The expanded logical-work run, now including the negative
tiny case, has SHA-256
`7342dcca027f7a801364ddc8624fba95d88617161fbfc32dec27e63ea11c4773`;
all pre-existing rows remained byte-identical.

The optimized Wasm artifact is 831,928 bytes with SHA-256
`582caeda2246a9192613f683e08e531c140c8a764fdeb532b958c05691ebc7f5`.
The focused ten-iteration/two-warmup npm smoke measured 0.464 ms/iteration and
the unchanged 1,828-byte payload; this short cold-path sample is recorded only
as a public-path smoke, not a timing claim. Repository gates passed with 387
core tests, 37 native Wasm tests, and 23 wasm32 tests, plus formatting, native
and Wasm clippy, no-default-feature, doctest, generated contract, protocol
snapshot, no-float, dependency policy, package/example frozen installs,
TypeScript/package checks, example build, external oracle, package-size,
browser E2E, and rustdoc gates. Exact pnpm audit requests returned HTTP 410;
the paired `--ignore-registry-errors` checks completed, with manifests and
lockfiles unchanged.

Review found and removed an eager magnitude clone on ineligible negative
points; `exp(-2)` returned to its base 9,106 bytes / 336 blocks (1,612 / 29
peak). The CLI retains the exact symbolic text, and the added browser regression
checks that the decimal-scientific enclosure for the negative tiny point is
strictly positive and no greater than one. The npm benchmark definition was
advanced to `representative-paths-v23` for the two new cases.

Same-host 20-sample Criterion ranges for `exp(-2^-100)` moved from
182.26--206.13 us at base `dc28555` to 10.105--10.601 us at the final candidate.
The base optimized Wasm artifact was 831,200 bytes; the candidate is 831,928
bytes (+728), both within the package budget.
