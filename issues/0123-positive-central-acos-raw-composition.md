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

Pending measurement and implementation.
