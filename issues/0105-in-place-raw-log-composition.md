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

## Profiling result

Two ownership variants were measured and rejected before integration. Moving
the owned reduced numerator/denominator through in-place multiplication left
`ln(2+sin(1))` unchanged at 158,393 bytes / 942 blocks; consuming the raw
numerator during final dyadic scaling also left total allocation unchanged and
reduced peak bytes only by trading 40 peak blocks for 51. A zero-reduced-term
composition fast path likewise left `ln(2)`, the large-positive-log control,
and the non-degenerate target byte/block/peak totals exactly unchanged at
10,022 / 406 / 1,574, 95,980 / 938 / 12,146, and 158,393 / 942 / 12,590.

`num-bigint` must grow these buffers for the following product or precision
shift, so syntactic ownership does not remove an allocator operation at this
boundary. No implementation change is retained. Further work should profile
inside the endpoint-specific binary-split recurrence rather than add a
branch or ownership rewrite at raw composition/rounding.
