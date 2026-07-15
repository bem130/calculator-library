# Issue 109: Convert exact logarithm points once

## Problem

`log` converts both certified dyadic endpoints to Rational before domain and
equality classification. Canonical exact points therefore repeat an identical
BigInt shift and Rational construction before entering the paired raw-log path.

## Requirements

- Detect canonical exact dyadic points before conversion and convert once,
  while preserving logarithm domain/error classification.
- Preserve structurally different/noncanonical equal endpoints through the
  existing post-conversion equality fallback and keep non-degenerate directed
  evaluation unchanged.
- Measure `ln(2)`, large positive log, non-degenerate log, logical work, and
  Wasm/npm boundary; add focused point/domain/non-degenerate regressions.
- Preserve precision, range reduction, resource accounting, no-float policy,
  and public protocol; complete reviews and repository gates before one
  ff-only integration into `main`.
