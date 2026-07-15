# Issue 118: Eliminate temporary atan leaf factors

## Problem

Current-main DHAT attributes the eight largest allocation sites in the outer
acos target to `atan_binary_split_leaf_block`. Each term first materializes
`p=-a²(2k-1)` and `q=b²(2k+1)`, then separately constructs `P*p`, although the
updated product is exactly the correction added to the scaled sum. The
logarithm leaf already avoids the analogous temporaries; atan still retains
them and dominates transformed asin/acos and non-degenerate atan.

## Requirements

- Update the owned numerator product directly by `a²`, the bounded odd factor,
  and alternating sign; add that updated product to the denominator-scaled sum.
- Update the denominator product and sum directly by `b²` and its bounded odd
  factor, without materializing combined `p`, `q`, or an extra `P*p`.
- Preserve exact `T'=Tq+Pp`, `P'=Pp`, `Q'=Qq`, alternating parity, adjacent
  bound selection, paired/directed endpoints, reciprocal atan, shared pi,
  transformed asin/acos, and all threshold paths.
- Preserve stopping, logical-work/resource accounting, determinism, no-float
  policy, error precedence, and public protocol. Do not tune limits or leaf
  width in this slice.
- Add an independent recurrence oracle around leaf and multi-leaf boundaries.
  Measure allocation/peak, native timing, logical work, Wasm/npm, CLI, browser,
  and ordinary/unit-numerator controls before and after.
- Complete diff, whole-system, and merge-granularity reviews plus repository
  gates before one ff-only integration into `main`.
