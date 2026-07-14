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

## Resolution

Canonical negation now flips only the signed numerator and clones the positive
denominator. Mixed subtraction allocation moved from 9,128 bytes / 404 blocks to
9,064 / 396, while its 2,047 / 43 peak and logical work remained unchanged. A
negative-literal addition control improved from 9,045 / 392 to 9,013 / 388 and
the other representative controls were unchanged. Criterion moved from
`[23.304,29.411]` to `[17.212,18.618]` us. Applying Wasm benchmark definition
v20 to both artifacts measured 0.719 ms at base and 0.726 ms at candidate, which
is treated as noise rather than a speedup; payload remained 1,805 bytes. The
825,518-byte artifact SHA-256 is
`8520a6f4bb9c2a7ba5de32d238659377f3f430e919e9ac4bffa96ada6f587fcf`.

Focused regressions, native and wasm-target lints, no-default/workspace/doc/Wasm
tests, generated DTO/protocol/no-float/dependency gates, package check, example
production build, external rational oracle, package-size gate, browser E2E, and
Rust documentation build all passed on the final artifact.
