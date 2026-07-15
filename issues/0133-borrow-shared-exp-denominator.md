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

## Resolution

No runtime change is retained. A prototype let the lower endpoint rebuild only
its numerator recurrence while borrowing the shared denominator, then moved the
denominator into the upper endpoint. Existing exact shared-denominator and
general-power composition oracles passed.

However, `2^sqrt(2)` does not enter the equal-denominator fast path, so its
allocation remained exactly 101,393 bytes / 692 blocks (6,223 / 43 peak).
The targeted `exp((1+sin(1))/4)` nondegenerate unit path was also exactly
unchanged at 74,113 / 970 (6,220 / 59 peak). Thus the denominator clone is not
the heap cost represented by the dominant DHAT groups on these public paths;
endpoint-specific growing numerator recurrence remains the measured cost.

The prototype was removed. Do not repeat denominator ownership changes without
a case that demonstrates clone allocation independently of numerator work.
