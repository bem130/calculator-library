# Issue 103: Eliminate temporary log leaf factors

## Problem

The hybrid binary-split logarithm evaluates each leaf recurrence by first
materializing `numerator_squared * odd_before` and
`denominator_squared * odd_after` as temporary arbitrary-precision integers.
Those values are immediately multiplied into the running product and sum.
Non-degenerate endpoint evaluation repeats this value-dependent work for each
directed endpoint and remains the dominant measured logarithm cost.

## Requirements

- Measure non-degenerate log time, allocation, peak allocation, logical work,
  and the Wasm/npm public boundary before and after the change.
- Update leaf products and scaled sums directly by the shared BigInt square
  factor and bounded primitive odd factor, without constructing combined
  temporary factors.
- Preserve the exact binary-split `P/Q` and `T/Q` recurrence, term parity,
  lower/upper tail, paired and directed endpoints, range reduction, shared
  `ln(2)`, precision, stopping limits, and logical-work accounting.
- Cover zero, unit-numerator, nonunit, leaf-threshold, multi-leaf, paired, lower,
  and upper paths against the existing incremental recurrence.
- Preserve no-float policy and public Rust/Wasm/npm protocol.
- Complete focused and repository gates, diff and whole-system reviews, and
  merge-granularity review before one integration into `main`.
