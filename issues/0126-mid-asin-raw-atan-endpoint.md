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

