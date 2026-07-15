# Issue 122: Compose negative central-acos endpoints before dyadic rounding

## Problem

After the outer-acos raw endpoint work, `acos((-1+sin(1))/3)` remains the
largest inverse-trigonometric allocation control. Both selected endpoints are
negative and satisfy `|x|<=1/2`, but the evaluator canonicalizes the unit asin
series and then canonicalizes `pi/2-asin(x)` before immediately rounding it to
a directed dyadic.

## Requirements

- Compose shared `pi/2` and the existing raw unit-asin fraction exactly at the
  final directed dyadic boundary for uniformly negative central endpoints.
- Preserve `acos(-m)=pi/2+asin(m)`, antitone endpoint selection, direction,
  positivity, monotonicity, precision, ordering/error precedence, stopping,
  logical-work/resource accounting, determinism, no-float, and protocol.
- Retain canonical routes for mixed sign/classification, positive central,
  outer, exact, zero, and special endpoints.
- Add raw/canonical oracles, boundary and reversed-input regressions, native and
  npm benchmarks, browser coverage, allocation/timing/logical-work/Wasm evidence,
  all repository gates, and diff/whole-system/merge-granularity reviews.

## Resolution

Pending measurement and implementation.
