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

## Resolution

`Rational::add` now returns the other canonical operand for zero, constructs an
integer directly for integer/integer, and for one integer operand computes only
`a + n*b` while retaining the fractional operand's canonical denominator. Only
fraction/fraction addition uses the general cross-products and normalizing
constructor. The direct result is canonical because `gcd(a+n*b,b)=gcd(a,b)=1`.
Large signed integers, cancellation, both mixed orders, zero identity,
fraction/fraction, and subtraction are compared with the general constructor.

At base `3680b54`, temporarily restoring the general path for every addition
moved `wide_add_256` from 35,937 bytes / 2,240 blocks (24,014 / 812 peak) to
46,169 / 3,519 with the same peak. Mixed `7 + -5/6` moved from 8,402 / 364 to
8,538 / 381, mixed subtraction from 8,453 / 372 to 8,589 / 389, and the public
fraction control from 11,691 / 493 to 11,851 / 509; their peaks were unchanged.
The fraction control includes surrounding public-pipeline additions, while the
direct unit oracle fixes fraction/fraction behavior itself.

Ten-sample `wide_add_256` Criterion ranges moved from 173.66--186.47 us on the
legacy path to 97.915--122.77 us after restoring the fast path, and Criterion
detected an improvement. The legacy runtime variant was completely removed.
Exact output and the current 261-unit wide-sum work behavior are unchanged; the
932 units in the Problem section are the historical `03c76bf` baseline rather
than the current-main value. Resource limits, no-float policy, and protocol also
remain unchanged.
