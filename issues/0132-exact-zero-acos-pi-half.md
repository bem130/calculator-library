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
