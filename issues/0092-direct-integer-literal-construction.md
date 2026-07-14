# Issue 92: Construct integer literals without decimal normalization

## Problem

`Rational::from_decimal_literal` routes source integers such as `123` through
the general decimal mantissa/scale path. It copies every digit into a new String,
constructs `10^0`, and invokes Rational GCD normalization and exact division even
though the canonical result is known to have denominator one. The 256-term wide
integer sum repeats this work for every literal after additive DAG folding has
already removed intermediate expression nodes.

At main commit `2718120`, one public 256-term wide-add calculation allocates
99,957 bytes / 4,424 blocks, peaks at 38,104 bytes / 1,023 blocks, and consumes
261 logical-work units. Allocation stacks attribute repeated digit copying,
`10^0`, GCD, and division to 251 parsed integer literals.

## Required change

- In the Rational converter, recognize its optional sign followed only by ASCII
  decimal digits before the general decimal/exponent path. Source grammar keeps
  treating `+` and `-` as unary operators.
- Parse those digits once into BigInt, apply the sign, and construct the
  canonical Rational integer directly.
- Preserve leading-zero and signed-zero normalization and all existing error
  classifications.
- Keep literals containing a decimal point or exponent on the general exact
  decimal path; do not change grammar, resource accounting, no-float policy, or
  public protocol.

## Acceptance

- Unsigned, signed, leading-zero, zero, and multi-thousand-digit integers match
  the general canonical constructor; decimal and exponent controls are unchanged.
- Wide-add scaling retains exact output and logical-work/resource behavior.
- Native allocation/timing, Wasm/npm boundary, package/example build, browser
  E2E, and repository gates are recorded before integration.
- Diff, whole-system consistency, and merge-granularity reviews have no blocker.

## Resolution

The integer-only path now parses the original unsigned digit slice directly,
applies the source sign, and constructs the canonical denominator-one Rational.
The decimal-point and exponent path is unchanged. Regression coverage compares
zero, signs, leading zeroes, and a 4096-digit integer with the general canonical
constructor and retains exact decimal/exponent controls.

At implementation commit `05dc2cf`, deterministic one-calculation allocation
for the 256-term wide sum moved from 99,957 bytes / 4,424 blocks to 81,525 /
2,888. Peak remained 38,104 bytes / 1,023 blocks and logical work remained 261
units. The exact-rational control moved from 12,582 / 529 to 12,182 / 501 while
its logical work remained 231 units.

A same-host ten-sample Criterion comparison measured the candidate wide-add at
185.11--223.92 us versus base at 305.03--367.42 us. Exact rational measured
32.54--41.31 us versus 36.70--47.42 us with no detected change. The
three-iteration/one-warmup Wasm/npm smoke moved wide-add from 1.558 ms/iteration
to 0.928 ms/iteration while payload stayed 1,728 bytes and retained heap stayed
3,528 bytes. Base artifact
`5cc8d57bef0a9e22aaa29ac2824508d32903f225ce4203649ddf2db527740134`
was 825,746 bytes; candidate
`45265a3e54ea365a7daaf6cd062dbcc81587378a08b58f79e6c132b6ade0416c`
is 826,091 bytes and remains below the package budget. Native Criterion and
deterministic allocation are the primary evidence; the short Wasm run confirms
the public boundary, output payload, and direction of the focused improvement.

The final gate run passed Rust formatting; native and Wasm Clippy; core
no-default check and tests; workspace and documentation tests; Node Wasm tests;
generated DTO, protocol snapshot, regeneration-diff, no-float, dependency,
oracle, package-size, and public-enum checks; package checking; example build;
browser E2E; and workspace documentation generation. The candidate Wasm
artifact hash and size remained the values recorded above.
