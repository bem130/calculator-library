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

- Recognize a sign followed only by ASCII decimal digits before the general
  decimal/exponent path.
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
