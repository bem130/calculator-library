# Issue 110: Convert exact arctangent points once

## Problem

`atan` converts both certified dyadic endpoints to Rational before ordered/equal
classification. Canonical exact points repeat the same BigInt shift and
Rational construction before entering the paired unit or reciprocal path.

## Requirements

- Detect canonical exact dyadic points before conversion and convert once for
  zero, unit-series, and reciprocal atan evaluation.
- Preserve noncanonical Rational-equal fallback, non-degenerate directed
  endpoints, ordering/error precedence, shared pi, and pole-free atan semantics.
- Measure unit, reciprocal, and non-degenerate atan allocation/peak, logical
  work, native timing, and Wasm/npm boundary; add focused regressions.
- Preserve precision, resource accounting, no-float policy, and public protocol;
  complete reviews and repository gates before one ff-only main integration.
