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

## Investigation result

The balanced product tree was prototyped and rejected. On current `main`, the
one-call `wide_multiply_128` baseline was 42,918 allocated bytes / 1,361 blocks,
18,596 peak bytes / 383 blocks, and 1,339 logical-work units. The balanced
prototype reduced charged structural work to 851 units, but increased total
allocation to 62,246 bytes / 1,370 blocks and peak allocation to 25,207 bytes /
645 blocks because it retained every parsed Rational and allocated reduction
levels. Concurrent 20-sample Criterion ranges were 92.23--99.04 us before and
96.08--116.19 us after; no improvement was detected.

The prototype was therefore removed. This issue is closed without changing the
implementation: lower logical-work accounting alone is not a performance gain,
and accepting a roughly 45% allocation increase would contradict the project's
efficiency objective. A future retry requires an ownership-reusing product
forest that demonstrates lower deterministic allocation as well as non-regressed
time before its resource-accounting changes can be considered.
