# Issue 94: Construct integral scientific literals without rational normalization

## Problem

The general decimal converter sends values whose final decimal scale is negative
through `Rational::new(product, 1)`, invoking GCD and exact division even though
the result is known to be an integer. A zero mantissa with a large positive scale
still constructs `10^scale` and normalizes `0/10^scale`, although every such
literal is canonically zero. This creates work and allocation unrelated to the
result and can unnecessarily restrict valid zero literals.

## Requirements

- After fully validating mantissa and exponent, return canonical zero before
  constructing a scale power when the parsed mantissa is zero.
- For negative final scale, multiply by the exact power of ten and construct the
  denominator-one Rational directly.
- Preserve decimal/exponent grammar and typed errors, signed zero, exact values,
  input/resource limits, no-float policy, and public protocol.
- Cover positive/negative signs, fractional mantissas that become integers,
  exponent boundaries, very large zero scales, and fractional controls.
- Record native allocation/timing/logical-work and Wasm/npm boundary evidence,
  run repository gates, and complete diff, consistency, and merge reviews.
