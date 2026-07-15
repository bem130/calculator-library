# Issue 129: Use raw directed endpoints for exact central acos

## Problem

Exact nonzero acos points with `|x|<=1/2` still normalize paired canonical
Rational bounds before dyadic rounding, despite existing positive and negative
central raw endpoint implementations.

## Requirements

- Route exact positive and negative central points through raw directed
  endpoints with one shared pi enclosure.
- Preserve antitonic ordering, zero and transform-region fallbacks, precision,
  logical-work/resource accounting, determinism, no-float, and protocol.
- Add exact dispatcher oracle, allocation/native/logical/Wasm/npm/CLI/browser
  evidence, all gates, and three reviews.

## Resolution

Exact nonzero central dyadic points now build both directed bounds with the
existing raw central endpoint helpers and one shared paired pi enclosure. Zero,
special angles, outer and mid-transform points retain their prior routes. The
dispatcher regression checks both signs, all regions and multiple precisions;
it requires the new interval to contain the prior canonical paired result while
the endpoint oracle tests retain exact directed-bound equality.

Against main `21b3ddb`, `acos(3/8)` moved from 41,237 bytes / 800 blocks
(5,383 / 49 peak) to 31,973 / 805 (4,567 / 49 peak). Thus total and peak bytes
fell 22.5% and 15.2%; the five additional short-lived allocations do not raise
the peak block count. Same-host native ranges moved from 498.76--536.51 us to
285.70--301.08 us. Full logical-work output remained byte-identical at SHA-256
`86667e251f7da693d5551e16c99c870f4d44f44f3168184baec4f0ebddfc8404`.
Optimized Wasm moved from 840,383 to 840,987 bytes and remains within budget;
candidate SHA-256 is
`273307c9f45c3014c769954c88358a4ba8be89a6c2aef0c9d258fd9413e14782`.
The v26 npm case moved from 3.174 to 2.155 ms/iteration with the same 1,766-byte
payload. CLI retains `acos(3/8)` and browser E2E checks its positive enclosure
below two radians.
