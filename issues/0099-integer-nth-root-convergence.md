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

## Resolution

The general floor nth root now derives the upper estimate
`2^ceil(value_bits/index)` and applies exact integer Newton iteration. Index two
uses the specialized convergent square root, while indices at least the value's
bit length return the mathematically forced root one. A former bitwise-search
oracle checks indices 1, 2, 3, 5, 31, 32, 33, and 127 over zero, one, perfect
cubes and adjacent values, and 31--1,000-bit input boundaries.

Same-host one-iteration algebraic allocation moved from 82,154 bytes / 3,133
blocks to 43,402 / 1,842, with the same 4,884-byte / 104-block peak. The
`sqrt(2)` control remained 9,085 / 337 and general power 101,809 / 698.
Concurrent 50-sample algebraic timing improved from 147.56--187.97 us to
107.68--131.57 us. The complete logical-work output stayed byte-identical at
SHA-256 `a925d3238a37ac073ae380a8c0200c9c654944a71f9a3e573660740d55d6fbd7`.

The ten-iteration/two-warmup Wasm/npm algebraic path retained its 1,792-byte
payload and measured 1.073 ms/iteration base versus 0.596 ms candidate; this
short run is boundary evidence rather than a timing claim. The optimized
artifact moved from 829,184 bytes
(`c89a824645735bcbd10e266dc8bbb191f8431a9b44be847b5b3acee2fbda93ac`)
to 829,230 bytes
(`5cea3933bbd249eca968400337cb48b9b2dcbd79785d12d6a6c896ce409a44bd`)
and remains below budget.
