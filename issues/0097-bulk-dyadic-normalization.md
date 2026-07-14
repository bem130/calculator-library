# Issue 97: Remove dyadic powers of two in one normalization step

## Problem

`normalize_dyadic` removes an even coefficient's powers of two one bit at a
time. Every iteration shifts the coefficient and adds one to a `BigInt`
exponent. At 128-bit presentation precision, current general-power profiling
shows several call sites each allocating 128 primitive-add temporaries, although
the complete trailing-zero count is available structurally.

## Requirements

- Determine the nonzero coefficient's trailing-zero count once, shift by that
  count once, and add the same exact count to the binary exponent once.
- Preserve negative coefficients, zero canonicalization, odd coefficients,
  arbitrary exponent signs, and exact lower/upper directed bounds.
- Preserve precision, termination, logical-work/resource accounting, no-float
  policy, and public protocol.
- Compare the bulk path with the former bitwise oracle across signs, zero/odd,
  limb boundaries, large shifts, and representative sqrt/exp/general-power
  public paths.
- Record native allocation/timing/logical-work and Wasm/npm boundary evidence.
- Complete repository gates and subagent diff, whole-system, and merge-granularity
  reviews before one integration into `main`.

## Resolution

`BigInt::trailing_zeros` now determines the complete removable power of two.
Normalization performs one shift and one exact exponent addition instead of
allocating an exponent temporary for every zero bit. A bitwise oracle regression
covers zero, both coefficient signs, odd values, limb boundaries, and a
1,000-bit shift.

On the same host with one allocation iteration, general power moved from
142,857 bytes / 1,990 blocks to 118,249 / 1,221, `sqrt(2)` from 41,845 / 1,370
to 25,525 / 860, `exp(-10000)` from 510,948 / 1,718 to 502,756 / 1,462, and
`exp(1)` from 16,493 / 552 to 8,301 / 296. Peak live allocation was unchanged
within 24 bytes for all four controls. Twenty-sample Criterion ranges improved
from 241.98--289.63 us to 179.71--216.93 us for general power and from
84.89--105.62 us to 65.66--75.14 us for `sqrt(2)`. Concurrent 30-sample
`exp(-10000)` controls overlapped at 365.43--409.44 us base and
344.51--376.58 us candidate, so no timing improvement is claimed there.

The complete logical-work baseline remained byte-identical (SHA-256
`a925d3238a37ac073ae380a8c0200c9c654944a71f9a3e573660740d55d6fbd7`).
The focused Wasm/npm `exp(-10000)` smoke retained the 1,794-byte payload and
measured 6.672 ms/iteration base versus 6.657 ms candidate over ten iterations;
the short run is evidence of boundary compatibility, not a timing claim. The
optimized artifact moved from 828,671 bytes
(`7b8fbbe10259480120c58105088d0218a29097b608edc2da802363428288324d`)
to 829,237 bytes
(`9f80e03f47ebbfae9ff417689a6f9f01447d09d0ac1732eaf54c6b68f33f3108`)
and remains below the package budget.
