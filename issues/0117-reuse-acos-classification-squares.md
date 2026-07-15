# Issue 117: Reuse outer-acos classification squares

## Problem

Non-degenerate `acos` classifies each selected rational endpoint by comparing
`2*n^2` with `d^2`. An outer endpoint then immediately constructs
`1-x^2=(d^2-n^2)/d^2`, recomputing both arbitrary-precision squares. The
classification and complement are one logical operation but currently discard
their shared structural work.

## Requirements

- Represent outer-region classification so its exact numerator and denominator
  squares can be consumed by direct positive and negative acos evaluation.
- Keep the allocation-free bit-length proof for clearly central values and do
  not add work to central, unit-series, special, or exact-point paths.
- Preserve the exact `2*n^2 < d^2` boundary decision, canonical complement,
  positive/negative directed rounding, antitone endpoint selection, and lazy
  shared-pi construction.
- Preserve determinism, stopping, logical-work/resource accounting, no-float
  policy, public protocol, and error precedence. Do not special-case a source
  expression or raise limits.
- Add focused equivalence and boundary regressions. Measure deterministic
  allocation/peak, native timing, logical work, Wasm/npm, CLI and browser
  controls before and after.
- Complete diff, whole-system, and merge-granularity reviews plus repository
  gates before one ff-only integration into `main`.
