# Issue 120: Round positive outer-acos atan endpoints from raw fractions

## Problem

Issue 82 kept inverse-trigonometric internal atan consumers in canonical
Rational form. Current-main DHAT for `acos((2+sin(1))/3)` now attributes about
20 KB / 16 blocks to four `Rational::new` normalizations immediately after
binary-split atan bounds. Positive direct outer-acos endpoints return that atan
value unchanged and then convert it to a directed dyadic, so the GCD and exact
division are representation-only work.

## Requirements

- Route positive direct outer-acos endpoints through the existing raw atan
  fraction to directed-dyadic boundary without canonical Rational construction.
- Preserve negative outer `pi-atan`, central `pi/2-asin`, exact-point, special
  values, antitone endpoint selection, sqrt direction, and mixed endpoint paths.
- Preserve exact enclosure, positivity, monotonicity, precision/refinement,
  error precedence, logical-work/resource accounting, no-float, and protocol.
- Add a direct-vs-canonical oracle across directions, precision, classification
  boundary, and non-degenerate endpoint ordering.
- Measure allocation/peak, native timing, logical work, Wasm/npm, CLI, example
  and browser paths plus controls. Complete diff, whole-system, merge-granularity
  reviews and all repository gates before one ff-only integration.
