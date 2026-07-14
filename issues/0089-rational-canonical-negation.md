# Issue 89: Negate canonical Rationals without renormalization

## Problem

`Rational::negate` clones the signed numerator and canonical positive denominator,
then sends both through `Rational::new`. Negation cannot change the numerator and
denominator GCD or denominator sign, so the constructor repeats a GCD and two
exact divisions for an already canonical value. Subtraction and symbolic
normalization use this primitive repeatedly.

At base commit `6dff484`, the public `-5/6 - 7` allocation case uses 9,128 bytes
in 404 blocks, versus 9,045 / 392 for the comparable addition case; both peak at
2,047 bytes / 43 blocks.

## Required change

- Negate only the owned numerator and clone the canonical positive denominator.
- Preserve unique zero, positive denominator, reduced form, exact semantics,
  deterministic work/resource behavior, no-float policy, and public protocol.
- Keep subtraction work accounting conservative unless the implementation's
  reserved operations can be reduced without undercharging another path.
- Cover zero, signs, large integers, proper/improper fractions, subtraction, and
  public exact/symbolic controls.

## Acceptance

- Focused canonical and arithmetic regressions pass.
- Native allocation/timing/logical-work and Wasm/npm measurements record the
  generalized effect and non-regressing controls.
- Package/example build, browser E2E, repository gates, diff review,
  whole-system consistency review, and merge-granularity review complete before
  one integration into `main`.
