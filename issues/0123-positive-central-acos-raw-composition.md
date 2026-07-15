# Issue 123: Compose positive central-acos endpoints before dyadic rounding

## Problem

Uniformly positive non-degenerate central acos still canonicalizes the unit
asin series and `pi/2-asin(x)` before immediate directed dyadic rounding. The
negative central path now proves that raw final-boundary composition is viable,
but the subtraction directions and antitone endpoint selection require their
own oracle and measurements.

## Requirements

- Compose shared `pi/2` minus the raw unit-asin fraction exactly for uniformly
  positive endpoints with `|x|<=1/2`, then round once to the directed dyadic.
- Preserve antitone endpoint selection, bound directions, positivity,
  monotonicity, precision, error precedence, resource accounting, determinism,
  no-float, and protocol.
- Retain existing routes for negative, mixed, outer, exact, zero, and special
  endpoints. Add focused oracle/regression, representative benchmarks, logical
  work, Wasm/npm/CLI/browser evidence, all gates, and three reviews.

## Resolution

For uniformly positive non-degenerate central endpoints, the evaluator now
keeps the oppositely directed unit-asin recurrence as raw numerator/denominator
parts, composes `pi/2-asin(x)` exactly with shared pi, and rounds once to the
directed dyadic boundary. Negative, mixed, outer, exact, zero, and special
routes retain their established dispatch; focused fallback tests cover central
to outer, zero-containing, and sign-mixed intervals.

Against base `b8eea33`, `acos((1+sin(1))/4)` moved from 514,966 bytes / 1,636
blocks (25,277 / 69 peak) to 436,950 / 1,567 (17,517 / 62 peak): 15.2% fewer
bytes, 69 fewer blocks, and 30.7% lower peak bytes. Native 20-sample ranges
moved from 3.17--3.53 ms to 0.459--0.482 ms. With the target included at
200,448 minimum logical-work units, base/candidate runner output matched at
SHA-256 `c26664ee5491163d519586976c98f433a1e45c8aed35ded9effa4d0b81d50e30`.

Optimized Wasm moved from 836,732 bytes
(`75c301f61fe9993392711c88ce34fc3f9c337503dd5d91647f7d80693f110ee2`)
to 837,564 bytes
(`6c708a2d469d1c1ad783a3b0c2c770e0c606fade0b2ca84835439fbc2c9348e4`),
within budget. A 100-iteration/10-warmup npm run moved from 20.086 to
2.732 ms/iteration with the same 1,788-byte payload and definition v26.
