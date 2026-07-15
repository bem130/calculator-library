# Issue 104: Move shared logarithm-two bounds into endpoints

## Problem

Non-degenerate logarithm intervals with two nonzero binary range exponents
compute one shared raw `ln(2)` lower/upper pair, but endpoint selection borrows
that pair and clones both BigInts for each endpoint. When the endpoints select
different sides, both raw bounds can be moved directly. Mixed exponent signs
select the same side and require only one clone, not two independent clones.

## Requirements

- Measure non-degenerate log allocation, peak, time, logical work, and Wasm/npm
  boundary before and after the ownership change.
- Consume a shared raw `ln(2)` pair into lower/upper endpoint composition;
  move distinct selected sides and clone exactly once only when both endpoints
  require the same side.
- Preserve zero/single-exponent behavior, positive/negative/mixed exponent
  direction selection, range reduction, exact raw composition, directed rounding,
  interval ordering, precision, stopping limits, and logical-work accounting.
- Cover both-positive, both-negative, both mixed-sign orders, one-zero, and
  different-magnitude exponents against independent directed endpoints.
- Preserve no-float policy and public Rust/Wasm/npm protocol.
- Complete focused and repository gates, diff and whole-system reviews, and
  merge-granularity review before one integration into `main`.
