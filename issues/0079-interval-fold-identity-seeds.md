# Issue 79: Remove interval fold identity seeds

## Problem

Certified interval evaluation initializes every n-ary addition with `[0, 0]` and
every n-ary multiplication with `[1, 1]`, then combines that identity with the
first real child. Production expression lists are non-empty, so this performs an
unnecessary directed dyadic operation for every composite Add or Multiply node.

For addition, aligning a nonzero first child with exponent-zero identity storage
can also create transient integer data that normalization immediately discards.
For multiplication, multiplying both endpoints by one unnecessarily clones and
allocates bigint storage.

## Requirements

- Seed non-empty Add and Multiply folds from their first evaluated child.
- Preserve left-to-right child evaluation and error precedence.
- Preserve explicit zero/one results for defensive empty-list handling.
- Preserve certified endpoints, determinism, logical-work accounting, resource
  limits, no-float policy, and the public protocol.
- Add focused empty/single/multiple/sign/zero interval regressions and public-path
  regression coverage.
- Record deterministic allocation and native/Wasm timing evidence for existing
  composite approximate benchmarks; do not integrate if allocation is unchanged.
- Complete diff, overall-consistency, and merge-granularity reviews plus repository
  gates before one integration into `main`.

## Baseline

On 2026-07-14 with `rustc 1.97.0`, one public `sqrt(2)*ln(2)` calculation allocates
90,818 bytes in 3,422 blocks with peak live allocation of 3,967 bytes in 74 blocks.
The existing allocation case is `approximate_power_log_product`.
