# Issue 133: Borrow the shared nondegenerate exp denominator

## Problem

When nondegenerate exp endpoints have the same Rational denominator, the
dispatcher constructs the final series denominator once but clones its large
BigInt-backed representation into the lower endpoint state. The lower path
does not mutate that denominator; only the upper path extends it with the tail
factor.

At main `8901472`, public `2^sqrt(2)` uses 101,393 bytes / 692 blocks
(6,223 / 43 peak). DHAT attributes the largest groups to the two endpoint
series states and their large shift/multiply buffers.

## Requirements

- Let the lower directed endpoint round against a borrowed shared denominator
  and transfer ownership only to the upper endpoint.
- Preserve endpoint-specific numerator recurrences, tail construction,
  directed bounds, precision, determinism and logical-work/resource accounting.
- Keep exact-point, unequal-denominator, binary-scaling and Rational-returning
  paths unchanged.
- Add exact oracle, allocation/native/logical/Wasm/npm evidence, all gates and
  three reviews; reject the change if deterministic allocation/peak regresses.
