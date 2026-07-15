# Issue 108: Convert exact exponential points once

## Problem

`exp` converts both certified dyadic endpoints to canonical Rational values
before checking equality. Exact-point inputs therefore repeat the same BigInt
shift/Rational construction even though all subsequent point paths use one
value and already share their series or binary-scaling plan.

## Requirements

- Detect canonical exact dyadic points before Rational conversion and convert
  the point once for ordinary, directly rounded, and binary-scaled exp paths.
- Preserve non-degenerate endpoint conversion and evaluation, error precedence,
  directed bounds, precision, logical-work/resource accounting, no-float policy,
  and public protocol.
- Measure ordinary, tiny-dyadic, large positive/negative, and general-power
  controls; add focused exact-point/non-degenerate regression coverage.
- Complete diff/whole-system reviews, repository gates, and merge-granularity
  review before one integration into `main`.
