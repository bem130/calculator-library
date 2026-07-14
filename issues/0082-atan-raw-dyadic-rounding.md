# Issue 82: Round public atan endpoints from raw fractions

## Problem

The public certified `atan` path builds alternating-series bounds as canonical
`Rational` values, composes reciprocal arguments with a canonical Machin-pi
bound, and then immediately converts the result back to a directed dyadic
endpoint. The intervening GCD reductions and exact divisions are representation
work rather than part of the enclosure proof. They are especially expensive for
non-degenerate reciprocal-domain inputs such as `atan(2+sin(1))`.

One-iteration native allocation baselines on `main` at `defe4a4` are:

| case | allocated bytes / blocks | peak bytes / blocks |
| --- | ---: | ---: |
| `atan(1/2)` | 20,774 / 760 | 2,167 / 37 |
| `atan(2)` | 45,586 / 965 | 4,368 / 41 |
| `atan(2+sin(1))` | 525,594 / 2,061 | 26,696 / 47 |

## Required change

- Preserve the alternating-series recurrence and binary-split proof, but expose
  the selected raw numerator and positive denominator for public atan endpoints.
- Directly round unit-domain raw fractions to lower or upper dyadics.
- For reciprocal-domain endpoints, combine the oppositely directed raw
  `atan(1/x)` fraction with the selected directed `pi/2` fraction and perform one
  final directed dyadic rounding.
- Preserve exact-point paired evaluation, non-degenerate monotone endpoint
  selection, odd symmetry, input ordering validation, term-count and overflow
  preflights, logical-work accounting, typed errors, and the public protocol.
- Keep Rational-producing helpers used by Machin pi and inverse-trigonometric
  internal compositions unless measurements justify broadening the slice.

## Acceptance

- Raw endpoint results match the legacy Rational-to-dyadic oracle across signs,
  unit and reciprocal domains, coarse and normal precisions, and recurrence and
  binary-split plans.
- Reversed exact and non-degenerate input bounds fail before coarse output
  rounding can hide the ordering error.
- Native allocation, timing, logical work, Wasm/npm boundary output, package and
  example builds, browser E2E, and repository gates are recorded before/after.
- Diff, whole-system consistency, and merge-granularity reviews have no open
  blocker before one integration into `main`.
