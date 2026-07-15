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
