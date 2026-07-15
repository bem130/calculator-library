# Issue 126: Round mid-transform asin from the raw atan endpoint

## Problem

For `1/2<|x|<1/sqrt(2)`, asin uses
`atan(x/sqrt(1-x^2))`. The selected directed atan series is normalized into a
canonical Rational and immediately converted to a directed dyadic, retaining a
large GCD/division boundary that the unit, high-transform, and public atan
paths already avoid.

## Requirements

- Keep the mid-transform unit atan endpoint raw through its final directed
  dyadic rounding, with odd-function direction reversal for negative inputs.
- Preserve monotonicity, sign, precision, endpoint ordering, sqrt direction,
  exact/special/unit/high-transform and coarse-ratio fallbacks, cancellation,
  logical-work/resource accounting, determinism, no-float, and protocol.
- Add canonical oracle and public dispatch regressions across both signs and
  region boundaries, reproducible allocation/native/logical/Wasm/npm/CLI/UI
  evidence, all repository gates, and three reviews.

## Resolution

Positive mid-transform endpoints now retain the selected unit atan recurrence
as raw numerator/denominator parts through final directed dyadic rounding.
Negative endpoints use the positive magnitude in the opposite direction and
negate only the dyadic result; a coarse ratio outside the unit range falls back
to the canonical route.

Against main `9d4e4fb`, `asin((1+sin(1))/3)` moved from 626,612 bytes / 1,797
blocks to 560,500 / 1,757, with unchanged peak 28,517 / 81. Native ranges moved
from 7.790--10.102 ms to 0.998--1.105 ms. Full logical-work output, including
the new case, remained byte-identical at SHA-256
`cf0253618aa3db0f6f42135ac47cbe71dd4623c2f60d4404f10cae6283775be2`.
