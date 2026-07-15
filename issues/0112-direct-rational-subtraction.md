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
