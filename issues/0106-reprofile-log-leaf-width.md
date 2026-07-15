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

## Profiling result

Current-main width 32 remains the selected balance at 158,393 bytes / 942
blocks and a previously measured 171.92--178.32 us range. Width 64 increased
allocation to 263,625 / 1,020 with no peak reduction. Width 16 reduced bytes to
144,489 but increased blocks to 1,018 and regressed Criterion to
184.67--235.46 us (`p < 0.05`). Width 24 produced the same allocation as width
32 and the same tree shape for the representative term plan, so it provides no
target benefit while changing lower-plan dispatch. Reordering primitive odd
factors before the BigInt square factors also regressed allocation to 158,649
bytes / 944 blocks.

No implementation change is retained. DHAT confirms that the remaining large
allocations are the necessary growing multiplications inside leaf blocks;
changing tree granularity or multiplication order either moves cost between
bytes, blocks, and time or is neutral. A future improvement needs a different
product/sum representation or multiplication primitive rather than threshold
tuning.
