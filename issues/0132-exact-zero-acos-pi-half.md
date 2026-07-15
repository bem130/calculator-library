# Issue 132: Round exact zero acos directly from pi

## Problem

After exact non-special acos regions moved to raw directed endpoints, the
special identity `acos(0)=pi/2` still constructs two canonical Rational halves
of the paired pi enclosure before converting them to dyadic bounds.

## Requirements

- Measure `acos(0)` through native allocation/timing/logical-work and public
  Wasm/npm/CLI/browser paths before retaining a runtime change.
- If material, round each selected pi endpoint as the exact fraction
  `numerator/(2*denominator)` without intermediate Rational normalization.
- Preserve exact `pi/2` recognition, directed bounds, precision-zero behavior,
  logical-work/resource accounting, determinism, no-float and protocol.
- Add oracle/regression coverage, evidence, all gates and three reviews.

## Resolution

No runtime change is retained. A direct `numerator/(2*denominator)` dyadic
rounding prototype matched the canonical endpoint oracle, but the public
`acos(0)` calculation is reduced to the exact `pi/2` representation before this
interval dispatcher is relevant.

At main `6bdb503`, both base and prototype used exactly 26,771 bytes / 575
blocks (4,032 / 38 peak). Same-host ten-sample Criterion was
212.73--232.85 us at base and 226.80--228.35 us for the prototype, which is
overlapping/noise and not an improvement. The runtime prototype and its private
oracle were removed. The reproducible allocation/native/logical/npm benchmark
case remains so future changes measure the actual public exact-representation
path rather than retrying this dispatcher-only optimization.
