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

## Resolution

Each binary-split leaf now updates the retained numerator product first by the
shared square and bounded odd factor, scales the sum and denominator product by
their denominator factors in place, and adds the already-updated numerator
product directly. This is the same `T' = Tq + Pp`, `P' = Pp`, `Q' = Qq`
recurrence without materializing `p`, `q`, or `P*p` temporaries.

Against base `baf394f`, one-call `ln(2+sin(1))` allocation moved from 194,825
bytes / 1,236 blocks to 158,569 / 946 (18.6% fewer bytes and 23.5% fewer
blocks); peak allocation stayed 12,798 bytes / 44 blocks. Logical work remained
200,220 units and the complete logical-work output remained byte-identical at
SHA-256 `a925d3238a37ac073ae380a8c0200c9c654944a71f9a3e573660740d55d6fbd7`.
Concurrent 20-sample Criterion ranges moved from 183.56--192.05 us to
165.37--180.79 us, with Criterion detecting an improvement.

The unit-numerator `ln(2)`, large positive log, general power, and
`exp(-10000)` controls were byte/block/peak-identical at 10,022 / 406 /
1,574, 95,980 / 938 / 12,146, 101,425 / 696 / 6,223, and 502,372 / 1,460 /
16,047 respectively. Keeping only a combined denominator temporary measured
161,449 bytes / 1,018 blocks, so the fully in-place factor update was retained.

The ten-iteration/two-warmup Wasm/npm path retained its 1,772-byte payload and
measured 1.565 ms/iteration base versus 1.716 ms candidate; this short boundary
run is not a timing claim. The optimized artifact moved from 829,645 bytes
(`10bea59a6984261b3fe247e3cb3a493d3c4ddb0ca39d6ff437c7a90fd2816b9f`)
to 829,606 bytes
(`0efe50189549911e2be400ee92f860d1e5632254646a3b1a3231775a5a8ec4d3`).
