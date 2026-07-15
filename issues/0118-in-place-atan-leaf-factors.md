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

## Resolution

The atan hybrid leaf now updates its owned numerator product directly by
`a²`, `2k-1`, and sign, then reuses that updated product as the `Pp`
correction. The sum and denominator product likewise consume `b²` and
`2k+1` directly. This retains the exact recurrence while removing the
combined `p`, `q`, and extra `P*p` temporaries. An independent expression-form
oracle covers single-term, parity, leaf-width, and adjacent multi-leaf ranges.

Against base `7a64e30`, deterministic one-call DHAT totals changed as follows;
peak live allocation was identical in every row.

| Case | Base bytes / blocks (peak) | Candidate bytes / blocks (peak) |
| --- | ---: | ---: |
| non-degenerate `atan` | 357,013 / 1,558 (17,384 / 44) | 285,789 / 1,163 (17,384 / 44) |
| reciprocal `atan(10/3)` | 356,885 / 1,455 (19,361 / 43) | 285,661 / 1,060 (19,361 / 43) |
| transformed `asin` | 879,440 / 2,346 (40,893 / 79) | 733,736 / 1,874 (40,893 / 79) |
| transformed `acos` | 958,986 / 2,470 (58,069 / 81) | 813,282 / 1,998 (58,069 / 81) |
| unit-numerator `atan(2)` | 27,409 / 606 (4,280 / 43) | unchanged |
| unit-series `atan(1/2)` | 10,484 / 428 (2,127 / 35) | unchanged |
| negative central `acos` | 1,019,757 / 1,793 (48,790 / 74) | unchanged |

The initial ten-sample Criterion run was host-sensitive, including an apparent
transformed-acos regression. Review therefore required three interleaved
base/candidate reruns with 30 samples and three-second measurement windows.
Their base ranges were 15.045--16.515, 15.010--16.427, and 12.716--13.297 ms;
candidate ranges were 14.107--15.721, 12.647--13.517, and 12.611--14.022 ms.
The order-correlated host drift is larger than the candidate effect, while the
candidate is faster in the first two pairs and overlaps in the third. This
rules out a repeatable regression but does not support a native timing claim;
the deterministic allocation reduction is the retained result.

The complete logical-work runner remained byte-identical at SHA-256
`7342dcca027f7a801364ddc8624fba95d88617161fbfc32dec27e63ea11c4773`.
The optimized Wasm artifact decreased from 833,423 bytes
(`469f6df75aa63ffac27e3b7099456a6238ca89c0d1950637b292564eba44781b`)
to 833,365 bytes
(`7dce68b0b7c4c9ef1fb5b063627fc954a328b3ba0e452a71e63c21b97c48834c`).
At 100 iterations after 10 warmups, the npm non-degenerate atan path was
effectively unchanged at 3.18176 versus 3.18186 ms/iteration, with the same
1,776-byte payload and 96 fewer retained JS-heap bytes. This is a public-path
compatibility check rather than a speed claim.

At the reviewed tip, CLI controls retained canonical sources `atan(10/3)`,
`atan(sin(1)+2)`, `asin(1/3*sin(1)+2/3)`, and
`acos(1/3*sin(1)+2/3)`. Package type/presentation checks, the example build,
and browser E2E passed with the existing transformed-acos display regression.
Repository gates passed formatting, all-target/all-feature native and wasm32
clippy, no-default check plus 391 core tests, 37 native Wasm tests, 23 wasm32
tests, doc tests/build, generated DTO regeneration, protocol snapshot,
no-float, `cargo deny`, external arithmetic oracle, and the 833,365-byte package
size budget. Both pnpm audit commands reached the registry's retired endpoint
and returned HTTP 410 rather than an advisory result.
