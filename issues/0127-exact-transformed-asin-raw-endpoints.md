# Issue 127: Use raw directed endpoints for exact transformed asin

## Problem

Exact dyadic asin points above one half bypass the raw mid/high endpoint
implementations and build paired canonical Rational bounds before converting
them to dyadics. This preserves a duplicate normalization boundary after the
non-degenerate paths have removed it.

## Requirements

- Route exact transformed points through the directed raw endpoint boundary,
  sharing the paired pi enclosure only when the high transform needs it.
- Preserve paired enclosure ordering, odd symmetry, special `±1`, unit/mid/high
  boundaries, coarse fallback, precision, logical-work/resource accounting,
  determinism, no-float, and public protocol.
- Add exact mid/high positive/negative oracle and public regressions,
  reproducible native/allocation/logical/Wasm/npm/CLI/UI evidence, all gates,
  and three reviews.

