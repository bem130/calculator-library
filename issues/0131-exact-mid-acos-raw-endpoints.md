# Issue 131: Use raw directed endpoints for exact mid-transform acos

## Problem

Exact non-special acos points between the central and outer regions still build
paired canonical Rational bounds before dyadic rounding. Existing directed
mid-transform asin endpoints already compute the required atan recurrence as a
raw fraction.

At main `83022f9`, `acos(5/8)` uses 397,422 bytes / 1,322 blocks
(29,183 / 49 peak) and 5.4962--6.0768 ms in a same-host ten-sample Criterion
run, making this exact dispatcher region materially slower than the raw central
and outer regions.

## Requirements

- Route exact positive and negative mid-transform acos points through raw
  directed endpoints with one shared pi enclosure.
- Preserve antitonic ordering, region boundaries, zero/special/central/outer
  fallbacks, precision, determinism and logical-work/resource accounting.
- Avoid input-specific branches, relaxed precision and protocol changes.
- Add exact dispatcher/oracle, allocation/native/logical/Wasm/npm/CLI/browser
  evidence, all repository gates and three reviews.

## Resolution

Exact positive and negative mid-transform points now reuse a raw fraction
producer shared with the existing directed mid-asin path. Acos composes that
fraction directly with one paired pi enclosure and rounds only the final ratio.
If coarse sqrt precision cannot prove a nonzero unit ratio, both asin and acos
retain their canonical fallback.

Against main `83022f9`, `acos(5/8)` moved from 397,422 bytes / 1,322 blocks
(29,183 / 49 peak) to 278,878 / 1,190 (15,023 / 40 peak). Total bytes fell
29.8%, total blocks 10.0%, peak bytes 48.5% and peak blocks 18.4%. Same-host
native ranges moved from 5.4962--6.0768 ms to 446.53--472.00 us. Full
logical-work output remained byte-identical at SHA-256
`86667e251f7da693d5551e16c99c870f4d44f44f3168184baec4f0ebddfc8404`.

The v26 npm/Wasm public path moved from 50.811 to 3.083 ms/iteration with the
same 1,772-byte payload. Optimized Wasm moved from 840,987 to 842,134 bytes and
remains within budget; candidate SHA-256 is
`fd7638ce53428e53dfd8d3c625314ad80b6c489b8c9c45dcb205a79a6ec345f9`.
CLI retains `acos(5/8)` and browser E2E checks its positive enclosure below two
radians.
