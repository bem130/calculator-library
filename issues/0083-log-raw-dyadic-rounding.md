# Issue 83: Round public logarithm endpoints from raw fractions

## Problem

The public certified logarithm path canonicalizes reduced-series bounds as
`Rational`, optionally scales and adds another canonical `ln(2)` bound after
binary range reduction, and immediately converts the result back to a directed
dyadic endpoint. The repeated GCD and exact-division work is representation cost,
not part of the enclosure proof.

At `main` commit `1f659b0`, one native calculation allocates 20,165 bytes / 724
blocks for `ln(2)`, 278,420 / 1,708 for `ln(2+sin(1))`, and 170,153 / 1,415 for a
large positive logarithm. The non-degenerate case peaks at 16,982 bytes / 43
blocks.

## Required change

- Expose raw numerator/positive-denominator lower, upper, and directed outputs
  from recurrence and hybrid binary-split log series states.
- Limit the new path to public `log()` endpoints. Keep Rational helpers used by
  exponential planning and other internal consumers.
- For reduced bound `a/b`, selected `ln(2)` bound `c/d`, and signed binary
  exponent `k`, compose exactly as `(a*d + k*c*b)/(b*d)` and directed-round once.
- Preserve exact-point paired evaluations, non-degenerate directed endpoints,
  shared `ln(2)` evaluation, positive-domain error precedence, input and final
  output ordering, range-reduction limits, overflow preflights, logical work,
  determinism, no-float policy, and public protocol.

## Acceptance

- Raw fractions and final dyadic endpoints match the canonical Rational oracle
  across recurrence/binary-split, zero/unit, positive and negative binary
  exponents, signs of the logarithm, and coarse/normal precisions.
- Reversed positive inputs fail before series work; existing nonpositive domain
  and unsupported classifications retain precedence.
- Native allocation/timing/logical work, Wasm/npm, package/example build, browser
  E2E, and all repository gates are recorded.
- Diff, whole-system consistency, and merge-granularity reviews have no blocker
  before a single integration into `main`.
