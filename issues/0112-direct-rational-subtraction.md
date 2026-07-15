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

The corresponding before/after peak bytes / blocks were 2,047 / 43 for exact
mixed subtraction, 1,407 / 24 for `exp(1)`, 1,560 / 19 for `ln(2)`, 6,223 / 43
for general power, 6,316 / 36 for the cumulative exp-power-log path, and 12,590
/ 40 for non-degenerate log. Because deterministic public allocation, block
count, peak, and logical work were all unchanged, the candidate failed the
early acceptance criterion; native timing and Wasm/npm measurements were not
run for code that would not be retained.

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

Repository gates passed with 383 core tests, 37 native Wasm tests, and 23
wasm32 tests, plus formatting, native/Wasm clippy, no-default-feature, doctest,
generated contract, protocol snapshot, no-float, dependency policy,
package/example frozen install, TypeScript/package checks, example build,
external oracle, package-size, browser E2E, and rustdoc gates. Exact pnpm audit
requests were unavailable because the registry endpoint returned HTTP 410; the
paired `--ignore-registry-errors` checks completed, and manifests and lockfiles
were unchanged.
