# Issue 107: Profile dyadic exponential factor application order

## Problem

DHAT attributes about 40 KB of the general-power path to repeated left shifts
and about 38 KB to adjacent BigInt multiplication in the dyadic exponential
Taylor recurrence. The recurrence currently multiplies its growing sum by the
primitive term index before applying the fixed denominator shift.

## Requirements

- Compare the current multiply-then-shift order with shift-then-multiply on the
  measured general-power, ordinary exp, tiny-dyadic exp, and large-exp paths.
- Retain a change only if deterministic allocation/peak and timing do not trade
  away correctness or another representative path.
- Preserve the exact recurrence, structured final denominator, directed bounds,
  logical-work/resource accounting, no-float policy, and public protocol.
- Record rejected variants and complete the appropriate reviews and gates
  before one integration into `main`.
