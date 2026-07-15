# Issue 106: Reprofile logarithm binary-split leaf width

## Problem

DHAT attributes the largest remaining non-degenerate-log allocation groups to
the three growing BigInt products in `log_binary_split_leaf_block`. The current
32-term leaf was selected before the later in-place leaf-factor and endpoint
ownership improvements, which changed the allocation/time balance between
sequential leaves and tree merges.

## Requirements

- Re-measure representative leaf widths on current `main`, including native
  allocation/peak and Criterion timing for the non-degenerate log.
- Retain a different width only when it improves measured cost without trading
  away timing, correctness, stopping behavior, or controls.
- Preserve exact recurrence, directed/paired endpoints, tail inclusion,
  logical-work accounting, no-float policy, and public protocol.
- Record rejected variants as well as the selected result and complete the
  appropriate reviews and gates before one integration into `main`.
