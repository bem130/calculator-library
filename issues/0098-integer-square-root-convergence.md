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
