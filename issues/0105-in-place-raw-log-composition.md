# Issue 105: Compose raw logarithm bounds in place

## Problem

After reduced-series evaluation, logarithm range composition computes
`a/d + k*c/b` as `(a*b + k*c*d)/(d*b)`. The current expression-style BigInt
arithmetic builds fresh products for the retained reduced numerator and
denominator even though both raw fraction parts are owned and immediately
consumed. This leaves avoidable allocation in every nonzero binary-exponent
log endpoint, including non-degenerate intervals.

## Requirements

- Measure non-degenerate log allocation, peak, timing, logical work, and the
  Wasm/npm boundary before and after the ownership change.
- Reuse the owned reduced numerator and denominator buffers during exact raw
  composition; retain only the correction product that is mathematically
  necessary before addition.
- Preserve `(a*b + k*c*d)/(d*b)` exactly for positive and negative exponents,
  zero composition, directed lower/upper endpoints, paired bounds, and
  different raw denominators.
- Preserve range reduction, interval ordering, precision, stopping limits,
  logical-work accounting, no-float policy, and the public protocol.
- Cover the in-place operation against an independent expression-form oracle,
  then complete focused/repository gates, diff and whole-system reviews, and
  merge-granularity review before one integration into `main`.
