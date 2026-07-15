# Issue 121: Compose negative outer-acos endpoints before dyadic rounding

## Problem

Issue 120 removed canonical Rational construction from positive non-degenerate
outer-acos endpoints, but negative outer endpoints still form `pi-atan` as a
canonical Rational before immediately rounding the result to a directed dyadic.
The retained negative control allocates substantially more than the positive
path, so the raw composition boundary must be measured and reviewed rather than
assuming the positive optimization generalizes automatically.

## Requirements

- Profile the negative outer-acos endpoint from sqrt/atan through shared-pi
  composition and directed dyadic rounding; identify normalization, allocation,
  logical-work, and presentation costs separately.
- If representation-only normalization is dominant, compose exact raw pi and
  atan fractions and round once without constructing an enormous decimal or
  weakening the certified enclosure.
- Preserve antitone endpoint selection, `pi-atan` directions, strict positivity,
  monotonicity, input-order error precedence, exact/special/central/mixed paths,
  precision refinement, cancellation, resource accounting, determinism,
  no-float policy, and public protocol compatibility.
- Cover negative outer endpoints around the classification boundary, ordinary
  negative values, mixed intervals, reversed bounds, and positive/exact controls.
- Record allocation/peak, native timing, logical work, Wasm/npm, CLI, example,
  and browser evidence. Complete focused and repository gates, diff and
  whole-system reviews, and merge-granularity review before one ff-only merge.

## Resolution

When both antitone-selected endpoints are negative and outer, the evaluator
keeps the directed atan series as raw numerator/denominator parts, composes it
exactly with the selected shared-pi bound, and rounds once to the requested
dyadic boundary. If coarse directed sqrt rounding makes the ratio exceed one,
the existing reciprocal/canonical route remains the explicit fallback.

Against base `2cc3b53`, `acos((-6+sin(1))/7)` moved from 962,132 bytes /
2,305 blocks (62,086 / 86 peak) to 613,188 / 1,811 (37,174 / 78 peak):
36.3% fewer bytes, 494 fewer blocks, and 40.1% lower peak bytes. Positive outer,
negative central, transformed asin, and atan controls remained byte-identical.
A 20-sample native run measured base at 29.18--32.52 ms and candidate at
1.50--1.80 ms. Logical-work output remained byte-identical at SHA-256
`7342dcca027f7a801364ddc8624fba95d88617161fbfc32dec27e63ea11c4773`.

The optimized Wasm artifact moved from 834,404 bytes
(`a599b933dbaab4d7d23b157dd32698fa0acffc654df9fac067e8be1f0377cec4`)
to 835,889 bytes
(`8f0c0c618cd609b2b319114263dfa7558fb7cc77ecd31aa7792a7be2080bbd89`),
remaining within budget. A 100-iteration/10-warmup npm public-path run moved
from 183.828 to 8.117 ms/iteration with the same 1,788-byte payload. CLI kept
`acos(1/7*sin(1)-6/7)` and browser E2E checks its enclosure remains between
two and four radians.
