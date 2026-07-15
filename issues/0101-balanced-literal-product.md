# Issue 101: Balance multiplicative literal products

## Problem

Numeric-only multiplication chains are flattened during source lowering, but
their Rational coefficient is still accumulated strictly left to right. For
`1*2*...*128`, each step multiplies the already-growing arbitrary-precision
integer by one more operand. Existing scaling measurements identify this
increasing BigInt multiplication as the remaining dominant cost.

## Requirements

- Measure the current wide-product time, allocation, peak allocation, and
  logical work before changing the algorithm.
- Combine literal factors with a deterministic balanced product plan so similarly
  sized intermediates are multiplied together instead of repeatedly extending
  one accumulator.
- Reserve conservative structural logical work before each multiplication and
  preserve the latched resource-limit fallback: after a limit is reached,
  lowering must not silently resume folding later literals.
- Preserve exact Rational canonical form, source parsing and error precedence,
  unary signs, decimal fractions, zero behavior, nonliteral/domain-sensitive
  factors, source/DAG limits, no-float policy, and public protocol.
- Cover ordinary, signed, fractional, zero, non-power-of-two, large, and
  resource-boundary products, and record native plus Wasm/npm evidence.
- Complete focused and repository gates, diff and whole-system reviews, and
  merge-granularity review before one integration into `main`.
