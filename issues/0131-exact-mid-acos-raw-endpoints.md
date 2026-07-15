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
