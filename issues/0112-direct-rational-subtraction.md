# Issue 112: Subtract rationals without a negated temporary

## Problem

`Rational::subtract` clones and negates the complete right operand, then passes
that temporary Rational to `add`. This allocates an avoidable signed numerator
and cloned denominator before doing the actual cross-products. Subtraction is
used by public exact arithmetic and throughout exp/log range reduction and
certified endpoint construction.

## Requirements

- Implement subtraction directly for zero, integer/integer, mixed integer, and
  general rational operands without constructing a negated Rational temporary.
- Preserve canonical positive denominators, reduction, zero uniqueness, signs,
  alias-safe borrowed operands, and exact arithmetic semantics.
- Add branch-complete tests including large operands, equal cancellation, mixed
  forms, and comparison with canonical construction.
- Measure exact subtraction, exp/log, general-power, and non-degenerate controls
  for allocation/peak, logical work, native timing, and Wasm/npm behavior.
- Preserve resource accounting, determinism, no-float policy, and public
  protocol; complete reviews and all gates before one ff-only integration.

## Resolution

Rejected after public-path measurement. A branch-complete direct subtraction
prototype passed focused canonical-form tests, but every representative public
allocation measurement was byte-for-byte and block-for-block unchanged:
`-5/6-7` remained 8,453 / 372, `exp(1)` 8,157 / 293, `ln(2)` 9,990 / 402,
`2^sqrt(2)` 101,393 / 692, `exp(sqrt(2)*ln(2))` 125,556 / 1,828, and
`ln(2+sin(1))` 158,393 / 942. Their peak measurements were also identical.
Logical-work output retained SHA-256
`a925d3238a37ac073ae380a8c0200c9c654944a71f9a3e573660740d55d6fbd7`.

The exact expression uses canonical lowering rather than this arithmetic method,
and the formerly Rational-heavy exp/log recurrences now retain raw fraction or
structured denominator state. Direct subtraction therefore did not remove work
from the measured public paths. The prototype and its test were removed; no
runtime or protocol change is retained. Future selection must use allocation
call stacks rather than treating a broadly used primitive as a measured hot
site.

Candidate DHAT call stacks instead attribute the dominant general-power
allocations to both directed exponential recurrences: about 21.0 KB and 19.0 KB
per endpoint at `term_numerator *= value_numerator` and
`sum_numerator *= index; sum_numerator <<= denominator_shift`. These are growing
BigInt recurrence states, not Rational subtraction. Reordering the adjacent
multiply and shift was already measured and rejected in Issue 107; repeating
that experiment is out of scope. Removing this cost requires a different exact
recurrence representation that can retain alignment across addition, with peak
size and all exp controls measured explicitly.
