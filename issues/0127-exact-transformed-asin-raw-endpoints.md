# Issue 127: Use raw directed endpoints for exact transformed asin

## Problem

Exact dyadic asin points above one half bypass the raw mid/high endpoint
implementations and build paired canonical Rational bounds before converting
them to dyadics. This preserves a duplicate normalization boundary after the
non-degenerate paths have removed it.

## Requirements

- Route exact transformed points through the directed raw endpoint boundary,
  sharing the paired pi enclosure only when the high transform needs it.
- Preserve paired enclosure ordering, odd symmetry, special `±1`, unit/mid/high
  boundaries, coarse fallback, precision, logical-work/resource accounting,
  determinism, no-float, and public protocol.
- Add exact mid/high positive/negative oracle and public regressions,
  reproducible native/allocation/logical/Wasm/npm/CLI/UI evidence, all gates,
  and three reviews.

## Resolution

Exact transformed points now construct lower and upper directed dyadic
endpoints through the same raw mid/high helpers as non-degenerate intervals.
High points share one paired pi enclosure; unit points and special `±1` retain
their established routes.

Against main `09c9fe7`, `asin(3/4)` moved from 348,323 bytes / 1,230 blocks
(27,023 / 59 peak) to 272,523 / 1,181 (16,935 / 47 peak). Native ranges moved
from 3.074--3.145 ms to 0.489--0.561 ms. The added case uses 31 logical-work
units and the full base/candidate output remained byte-identical at SHA-256
`2eeaaa8c6c9ff77ea963864915de12c839189dc037195a10ec49617f313ea97e`.
Optimized Wasm moved from 839,928 to 839,620 bytes; candidate SHA-256 is
`d77d0ae88e00a6ddb0440bdbfa9ab33d1be87319413d127d185132349e25b990`.
The v26 npm case moved from 18.796 to 2.755 ms/iteration with the same
1,772-byte payload. CLI retains `asin(3/4)` and browser E2E checks its positive
enclosure below two radians.
