# Issue 84: Avoid rational normalization for integer addition

## Problem

`Rational::add` always cross-multiplies both numerators, multiplies denominators,
and calls the GCD-normalizing constructor. When either canonical denominator is
one, the exact result is already canonical without GCD normalization. In
particular, for canonical `a/b` and integer `n`,
`gcd(a + n*b, b) = gcd(a, b) = 1`; the symmetric case follows identically.
Wide integer sums and mixed integer/fraction sums repeatedly pay unnecessary
products, GCD, and exact divisions.

At `main` commit `03c76bf`, the representative 256-term integer sum allocates
284,027 bytes / 10,858 blocks, peaks at 109,443 bytes / 1,891 blocks, and uses
932 logical-work units. Existing historical Criterion and Wasm snapshots are
about 705 us and 10.49 ms respectively.

## Required change

- Add canonical zero-identity, integer/integer, and one-integer-operand paths to
  general `Rational::add`; do not add a wide-expression or syntax special case.
- Construct the result through the invariant-preserving integer representation,
  including zero and negative sums, without GCD normalization.
- Keep addition where both operands are non-integer on the existing exact
  Rational path.
- Preserve public arithmetic results, deterministic normalization, logical-work
  accounting, resource limits, no-float policy, and protocol.
- Add reproducible wide-expression scaling and stage measurements so the claimed
  bottleneck and remaining costs are explicit.

## Acceptance

- Fast-path results match the general canonical constructor for zero, signs,
  cancellation, large BigInts, both mixed operand orders, and subtraction;
  non-integer/non-integer controls are unchanged.
- Wide sums at multiple sizes retain exact output and existing work/error behavior.
- Native allocation/timing/stages/logical work, Wasm/npm, package/example build,
  browser E2E, and repository gates are recorded before/after.
- Diff, whole-system consistency, and merge-granularity reviews have no blocker
  before a single integration into `main`.
