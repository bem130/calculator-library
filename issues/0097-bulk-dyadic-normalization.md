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

