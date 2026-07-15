# Issue 119: Retain dyadic alignment across exponential recurrence additions

## Problem

On current main, one `2^sqrt(2)` calculation allocates 101,393 bytes in 692
blocks. DHAT attributes about 78 KB across the two directed exponential
endpoints to the growing Taylor numerator: each step multiplies the sum by the
bounded term index, materializes the fixed denominator left shift, and only
then adds the next numerator power. Reordering those two operations was rejected
in Issue 107, while balanced and reverse finite-sum trees were rejected in
Issue 115 because they increased blocks or peak live state.

## Requirements

- Investigate a representation that retains the dyadic denominator alignment
  across addition rather than merely reordering the same multiply and shift.
- Preserve the exact finite Taylor sum, upper-tail proof, directed lower/upper
  rounding, exact-point and non-degenerate endpoints, binary scaling, general
  non-dyadic Rational fallback, and all stopping thresholds.
- Do not raise limits, reduce precision, approximate with floats, or specialize
  a public source expression. Preserve deterministic logical-work/resource
  accounting, typed errors, no-float policy, and public protocol.
- Compare against current main for general power, ordinary/tiny/large positive
  and negative exp, and non-degenerate exp. Reject the representation if bytes,
  blocks, peak live state, timing, or Wasm size trade away a representative
  path without a justified general benefit.
- Add an independent exact recurrence oracle for all retained representations.
  Record native allocation/timing, logical work, Wasm/npm, CLI, package/example,
  and browser evidence. Complete diff, whole-system, and merge-granularity
  reviews plus repository gates before one ff-only integration into `main`.
