# Issue 93: Keep dyadic exponential denominators structured through rounding

## Problem

For a Taylor input denominator `q = 2^k`, exponential evaluation materializes
the final common denominator as `N! << (k*N)` and then divides a similarly large
scaled numerator by it. The recurrence already treats `q` as a shift, but loses
that structure at the final directed-dyadic rounding boundary.

## Requirements

- Represent the dyadic final denominator as a factorial base plus checked binary
  shift through lower, upper-tail, exact-point, directed, and shared-endpoint paths.
- Compute exact directed floor/ceil without materializing the shifted denominator;
  cover precision smaller than, equal to, and larger than the denominator shift.
- Preserve signed floor/ceil semantics, upper-tail enclosure, positivity,
  monotonicity, determinism, stopping, cancellation, and all resource errors.
- Preserve the general non-power-of-two denominator path and rational-only test
  helpers.
- Do not change precision, Taylor term selection, logical-work accounting,
  no-float policy, or public protocol.
- Measure `exp(2^-100)` and an amplified `exp(2^-1000)` path plus ordinary and
  general-power controls;
  reject the implementation if no material allocation or timing benefit appears.
- Add exact equivalence/property regressions and complete native/Wasm/package/
  example gates plus diff, consistency, and merge-granularity review.

## Resolution

The exponential series denominator now remains either materialized for a general
input denominator or structured as its factorial base plus a checked binary shift
for a power-of-two input denominator. Directed dyadic rounding consumes that
shift on the numerator/quotient side. When the shift exceeds the requested
precision, arithmetic right shift plus exact remainder tests implement signed
floor/ceil without constructing the power-of-two divisor. Rational-only helpers
still materialize the denominator at their existing boundary.

At base `45f49da`, one public `exp(2^-100)` calculation allocated 24,702 bytes /
477 blocks and peaked at 5,074 / 39. The candidate allocated 24,270 / 481 and
peaked at 4,154 / 38. The amplified `exp(2^-1000)` moved from 40,289 / 704,
peak 5,595 / 41, to 39,177 / 707, peak 4,411 / 41. The general-power path
`2^sqrt(2)` moved from 149,543 / 1,991, peak 8,551 / 44, to 142,863 / 1,993,
peak 6,247 / 43. `exp(1)` remained exactly 16,497 / 554, peak 1,423 / 26;
`exp(-10000)` was effectively unchanged at 510,964 versus 510,956 bytes and
the same 1,720 blocks and 16,047 / 32 peak.

Same-host ten-sample Criterion ranges moved `exp(2^-100)` from
25.64--35.34 us to 18.76--22.28 us. General power measured 157.79--172.44 us
at base and 143.46--179.10 us in the candidate, so no general-power timing claim
is made. The complete logical-work baseline was byte-for-byte unchanged.

A three-iteration/one-warmup Wasm/npm boundary smoke measured the focused case at
0.564 ms/iteration in the base and 0.630 ms/iteration at final tip while its
1,824-byte payload was unchanged. This short cold-path diagnostic is too noisy
for a Wasm timing claim. The base
artifact was 826,091 bytes with SHA-256
`45265a3e54ea365a7daaf6cd062dbcc81587378a08b58f79e6c132b6ade0416c`;
the candidate is 829,165 bytes with SHA-256
`7d5cc154557d057903760c1a9096062e5b4cf75ca8c5cd549561598024521a32`,
below the package budget. Native allocation and Criterion are the primary
evidence; the short Wasm run verifies the public boundary and payload.

The final gate run passed Rust formatting; native and Wasm Clippy; core
no-default check and 370 tests; workspace tests (370 core, 37 native Wasm, and
2 CLI); documentation tests; 23 Node Wasm tests; generated DTO, protocol,
regeneration-diff, no-float, dependency, oracle, and package-size checks;
package checking; example build; browser E2E; and workspace documentation.
