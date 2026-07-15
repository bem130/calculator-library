# Issue 99: Replace bitwise integer nth-root search

## Problem

`floor_nth_root_nonnegative` doubles a `BigInt` bound and then bisects the full
input bit width, raising every candidate to the index. On current `main`, the
algebraic cube-root representative allocates 82,154 bytes / 3,133 blocks, and
DHAT attributes its largest allocation sites to 128 iterations of this search.
Issue 76 shared the exact-point lower/upper search but did not change the
remaining single-search algorithm.

## Requirements

- Derive an exact upper estimate from input bit length and index, then use an
  integer convergence algorithm without floats or input-specific branches.
- Preserve floor roots for zero, one, perfect powers, adjacent non-powers,
  indices across ordinary and boundary values, and large inputs.
- Preserve signed odd-root callers, directed interval bounds, termination,
  deterministic logical-work/resource accounting, no-float, and protocol.
- Record native allocation/timing for the algebraic representative and sqrt /
  general-power controls, logical work, and Wasm/npm boundary behavior.
- Complete repository gates and diff, whole-system, and merge-granularity
  reviews before one integration into `main`.
