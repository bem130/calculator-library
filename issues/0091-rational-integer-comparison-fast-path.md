# Issue 91: Compare canonical Rational integers structurally

## Problem

`Rational::compare` always cross-multiplies both numerators by the opposite
denominator. Canonical integers have denominator one, so integer/integer
comparisons allocate two mathematically redundant BigInt products. Comparisons
against the integer domain boundaries zero, one, and minus one are common in
exact normalization, algebraic interval isolation, and transcendental domain
validation.

At main commit `d9b003f`, one public calculation allocates 12,582 bytes / 529
blocks for the exact-rational case, 99,957 / 4,424 for the 256-term wide sum,
172,232 / 2,721 for the approximate composite, and 99,235 / 3,728 for the
algebraic case. Their logical-work boundaries are respectively 231, 261,
400,447, and 400,229 units.

## Required change

- Compare zero structurally from the other canonical numerator's sign, and
  compare two canonical integers directly by numerator.
- When exactly one operand is an integer, scale only that integer numerator by
  the fractional operand's positive denominator.
- Retain exact cross multiplication for two non-integer operands.
- Do not use floating point, weaken resource accounting, or change public
  protocol and ordering semantics.
- Cover signs, zero, equal and unequal large integers, both mixed operand
  orders, and non-integer controls against the general cross-product oracle.

## Acceptance

- Focused and workspace tests preserve exact ordering and canonical invariants.
- Native allocation, timing, logical work, and the Wasm/npm boundary are
  recorded before and after for representative affected paths.
- Package/example builds, browser E2E, repository gates, and diff,
  whole-system, and merge-granularity reviews have no blocker before a single
  integration into `main`.

## Resolution

`Rational::compare` now handles zero from the canonical numerator sign, compares
two integers directly, and performs only the necessary single scaling when one
operand is fractional. The two-fraction cross-product path is unchanged. An
exhaustive matrix over zero, signs, 4096-bit integers, proper and improper
fractions, and both operand orders agrees with the former cross-product oracle.

At candidate commit (to be recorded by the branch history), deterministic
one-calculation allocation for the algebraic representative moved from 99,235
bytes / 3,728 blocks to 99,035 / 3,703. Peak remained 4,884 bytes / 104 blocks
and logical work remained 400,229 units. Exact rational, approximate composite,
and 256-term wide-add controls were byte-, block-, peak-, and work-identical.
A same-host ten-sample Criterion comparison detected no timing change: candidate
confidence interval 276.31--324.75 us versus base 278.24--299.31 us. This slice
therefore claims deterministic allocation reduction only.
