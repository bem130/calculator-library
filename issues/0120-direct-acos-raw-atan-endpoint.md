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

## Resolution

Positive non-degenerate outer-acos intervals now route both antitone-selected
endpoints through the existing raw atan fraction and round only at the directed
dyadic boundary. Negative outer endpoints retain canonical `pi-atan`; central,
mixed, special, and exact-point inputs retain their prior Rational paths. The
old exact input-order rejection is performed after endpoint evaluation so
coarse dyadic rounding cannot hide a reversed interval.

Against base `65a3717`, one `acos((2+sin(1))/3)` calculation moved from
813,282 bytes / 1,998 blocks (58,069 / 81 peak) to 557,146 / 1,541
(26,869 / 78 peak): 31.5% fewer bytes, 459 fewer blocks, and 53.7% lower peak
bytes. Base-identical controls include negative outer acos at 962,132 / 2,305
(62,086 / 86), negative central acos at 1,019,757 / 1,793 (48,790 / 74), exact
`acos(3/4)` at 408,819 / 1,273 (28,567 / 48), exact `acos(5/8)` at 397,398 /
1,319 (29,183 / 49), transformed asin at 733,736 / 1,874, and non-degenerate
atan at 285,789 / 1,163.

Two powered interleaved 30-sample native pairs measured base at
12.067--12.919 and 11.980--13.689 ms versus candidate at
0.944--1.137 and 0.932--1.071 ms. Logical-work output remained byte-identical
at SHA-256 `7342dcca027f7a801364ddc8624fba95d88617161fbfc32dec27e63ea11c4773`.
The optimized Wasm artifact moved from 833,365 bytes
(`7dce68b0b7c4c9ef1fb5b063627fc954a328b3ba0e452a71e63c21b97c48834c`)
to 834,404 bytes
(`a599b933dbaab4d7d23b157dd32698fa0acffc654df9fac067e8be1f0377cec4`),
remaining within budget. A 100-iteration/10-warmup npm public-path smoke was
87.647 ms/iteration at base versus 5.876 ms/iteration for the candidate, with
the same 1,794-byte payload.

The final CLI retained canonical source `acos(1/3*sin(1)+2/3)`. Package type and
presentation checks, example build, and browser E2E passed with the existing
scientific/enclosure display contract. Repository gates passed formatting,
native and wasm32 clippy, no-default check and 394 core tests, 37 native Wasm
tests, 23 wasm32 tests, doc tests/build, generated DTO regeneration, protocol
snapshot, no-float, `cargo deny`, external arithmetic oracle, and package-size.
Both pnpm audit commands reached the registry's retired endpoint and returned
HTTP 410 rather than an advisory result.
