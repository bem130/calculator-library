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

Pending measurement and implementation.
