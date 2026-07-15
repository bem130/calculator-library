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

## Resolution

The non-degenerate endpoint path now consumes the shared raw `ln(2)` pair.
When the endpoint signs select different directed bounds, each bound moves
directly into its endpoint; when they select the same bound, exactly one raw
fraction is cloned. A zero binary exponent computes no `ln(2)` bound for that
endpoint, including the single-nonzero cases. Tests compare every sign/zero
combination, including both mixed orders, with independently directed bounds.

Against base `c5722de`, one-call `ln(2+sin(1))` allocation moved from 158,569
bytes / 946 blocks to 158,393 / 942, while peak live allocation moved from
12,798 bytes / 44 blocks to 12,590 / 40. Logical work remained 200,220 units,
and the complete logical-work output remained byte-identical at SHA-256
`a925d3238a37ac073ae380a8c0200c9c654944a71f9a3e573660740d55d6fbd7`.
Twenty-sample Criterion ranges overlapped at 165.37--180.79 us base versus
171.92--178.32 us candidate, so this slice makes no native timing claim.

The `ln(2)`, large-positive-log, general-power, and `exp(-10000)` controls were
unchanged at 10,022 / 406 / 1,574, 95,980 / 938 / 12,146, 101,425 / 696 /
6,223, and 502,372 / 1,460 / 16,047 bytes / blocks / peak bytes respectively.
The optimized Wasm artifact moved from 829,606 bytes
(`0efe50189549911e2be400ee92f860d1e5632254646a3b1a3231775a5a8ec4d3`) to
830,189 bytes
(`b1dde55f5ad549a2fb0f90879f037cf42f9bc41fa6628bc91a72cce484c3cb1e`),
remaining below budget. The ten-iteration/two-warmup npm path retained its
1,772-byte payload and measured 1.716 ms/iteration base versus 1.075 ms
candidate; the short runs are not a Wasm timing claim.
