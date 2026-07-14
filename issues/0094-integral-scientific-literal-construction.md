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

## Resolution

The converter now returns canonical zero after mantissa/exponent validation but
before constructing a scale power. A nonzero literal with final scale zero uses
the parsed numerator directly; a negative final scale constructs the exact power
of ten and returns the product as a denominator-one Rational without GCD or exact
division. Positive-scale fractional values keep the general canonical path.

At base `1815cf4`, one public `0e-100000` calculation allocated 1,900,744 bytes /
3,254 blocks and peaked at 198,298 / 44. The candidate allocated 3,016 / 98 and
peaked at 1,130 / 10. `12345e100` moved from 21,488 / 685 to 21,376 / 681 with
the same 1,599 / 47 peak. The exact-rational control remained 12,182 / 501.

Same-host ten-sample Criterion ranges moved `0e-100000` from
2.167--2.576 ms to 3.451--3.772 us. The ordinary integral-scientific control
measured 58.46--67.87 us at base and 46.34--67.10 us in the candidate, so no
timing claim is made for that control. Logical work is 3 units for the zero case
and 46 for the nonzero case; the optimization does not change accounting.

A three-iteration/one-warmup Wasm/npm smoke moved `0e-100000` from 10.285 to
0.317 ms/iteration while its payload remained 1,720 bytes. The ordinary control
measured 0.691 versus 0.672 ms with a stable 1,940-byte payload. Base artifact
`7d5cc154557d057903760c1a9096062e5b4cf75ca8c5cd549561598024521a32`
was 829,165 bytes; candidate
`84e64abd994e78564aa01bd993c02cd44b9f3cfd5deb171b92c3811929b77f37`
is 829,226 bytes and remains below the package budget.

The final CI-equivalent gate run passed formatting; native and Wasm clippy;
no-default checks and tests; 371 core, 37 native-Wasm, 2 CLI, and 23 Node-Wasm
tests; doc tests; generated/protocol/no-float checks; dependency policy and
audits; package type/presentation checks; package and example builds; external
oracle; package-size budget; browser E2E; and documentation generation. The
final release Wasm retained the measured candidate SHA-256 and 829,226-byte
size above.
