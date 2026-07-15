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

## Profiling result

Shift-then-multiply was exact against the Rational recurrence but moved general
power from 101,425 bytes / 696 blocks to 101,657 / 697, with the same
6,223-byte / 43-block peak. `exp(1)` and `exp(-10000)` were exactly unchanged
at 8,173 / 295 / 1,423 and 502,372 / 1,460 / 16,047 bytes / blocks / peak.
Tiny dyadic exp moved only from 21,259 to 21,243 bytes with the same 395 blocks
and 4,154 / 38 peak.

The 16-byte tiny-input saving does not justify a general-power regression at
the dominant path. No implementation change is retained. The shift allocation
is intrinsic to materializing the aligned sum before adding the next term;
further improvement requires retaining a structured shift across the addition,
not reordering the same two operations.
