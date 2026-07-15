# Issue 98: Replace bitwise integer square-root search

## Problem

`floor_sqrt_nonnegative` first doubles a `BigInt` until its square exceeds the
input and then performs a full-width binary search. For the 128-bit certified
`sqrt(2)` path, DHAT attributes 128 multiplication allocations to the bound
search and 129 cloned additions plus another 128 squares to bisection. This is
algorithmic work proportional to the input bit width even though an integer
square root can converge by approximately doubling the number of correct bits.

## Requirements

- Derive the initial estimate structurally from the nonnegative integer's bit
  length and use an exact integer convergence algorithm; do not use floats.
- Return the identical floor root for zero, one, perfect squares, adjacent
  nonsquares, limb boundaries, and large values.
- Preserve callers' directed sqrt/nth-root bounds, termination, logical-work
  and resource-limit contracts, deterministic output, and public protocol.
- Compare native allocation and Criterion timing for `sqrt(2)`, general power,
  algebraic controls, logical work, and the Wasm/npm boundary.
- Complete focused and repository gates plus diff, whole-system, and merge
  reviews before one integration into `main`.

## Resolution

The floor square root now starts from `2^ceil(bits/2)`, a structural upper
estimate, and applies exact integer Newton iteration until the estimate no
longer decreases. Each iteration approximately doubles the number of correct
bits and the returned fixed point is the exact floor root. Regression coverage
checks zero, one, perfect squares and both adjacent nonsquares across 32-bit,
64-bit, 128-bit, and 1,000-bit boundaries.

Same-host one-iteration allocation moved `sqrt(2)` from 25,525 bytes / 860
blocks to 9,085 / 337 and general power from 118,249 / 1,221 to 101,809 / 698.
Peak live allocation remained effectively unchanged at 1,488 versus 1,496
bytes for `sqrt(2)` and 6,223 bytes for general power. The algebraic control was
unchanged at 82,154 / 3,133. Concurrent 30-sample Criterion ranges improved
from 37.512--39.031 us to 6.921--8.185 us for `sqrt(2)` and from
112.45--117.75 us to 77.71--82.49 us for general power.

The complete logical-work output remained byte-identical at SHA-256
`a925d3238a37ac073ae380a8c0200c9c654944a71f9a3e573660740d55d6fbd7`.
The ten-iteration focused Wasm/npm combined approximate path retained its
1,812-byte payload. Its short samples were 1.068 ms/iteration base and 0.866 ms
candidate, so no Wasm timing claim is made. The optimized artifact moved from
829,237 bytes (`9f80e03f47ebbfae9ff417689a6f9f01447d09d0ac1732eaf54c6b68f33f3108`)
to 829,184 bytes (`c89a824645735bcbd10e266dc8bbb191f8431a9b44be847b5b3acee2fbda93ac`)
and remains below budget.

Repository validation passed with 375 core tests, 37 native Wasm tests, 23
Node/Wasm tests, no-default checks/tests, workspace/doc tests, native and wasm32
clippy, generated DTO/protocol/no-float checks, `cargo deny`, TypeScript checks,
example build, external rational oracle, package-size gate, browser E2E, and
rustdoc. The npm dependency graph and lockfiles are unchanged from `main`.
`pnpm audit` was attempted three times for the package and stopped at the
registry boundary because npm returned HTTP 410 for its retired legacy audit
endpoint; the same command fails on unchanged `main`, so this is recorded as an
external gate outage rather than a dependency result. Review findings removed
the unnecessary u64-to-usize narrowing on wasm32 and added direct input-bit
boundary coverage; focused tests and wasm32 clippy passed afterward.
