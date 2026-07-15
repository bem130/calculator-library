# Issue 128: Use raw directed endpoints for exact outer acos

## Problem

Exact non-special outer acos points still build paired canonical Rational
bounds, although non-degenerate positive and negative outer endpoints already
have raw directed implementations.

## Requirements

- Route exact positive and negative outer points through their raw directed
  endpoints, sharing pi only for the negative transform.
- Preserve antitonic ordering, sign, precision, `±1`, central/mid fallback,
  coarse-ratio fallback, logical-work/resource accounting, determinism,
  no-float, and protocol.
- Add exact dispatcher oracle, allocation/native/logical/Wasm/npm/CLI/browser
  evidence, all gates, and three reviews.

## Resolution

Exact non-special outer points now build lower and upper dyadic bounds through
the existing directed raw endpoint helpers. Negative points share one paired pi
enclosure; central, mid and `±1` points retain the canonical route.

Against main `ad4de2e`, `acos(3/4)` moved from 408,819 bytes / 1,273 blocks
(28,567 / 48 peak) to 242,659 / 904 (13,287 / 50 peak). The two-block peak
increase accompanies a 53.5% peak-byte reduction and 29.0% fewer total blocks.
Native ranges moved from 4.056--4.136 ms to 0.171--0.186 ms. Full logical-work
output remained byte-identical at SHA-256
`2eeaaa8c6c9ff77ea963864915de12c839189dc037195a10ec49617f313ea97e`.
Optimized Wasm moved from 839,620 to 840,383 bytes and remains within budget;
candidate SHA-256 is
`a3e1a8374df58b3229d2a37814cd868f0d3583df13a1b2411539b0d1a26d754f`.
The v26 npm case moved from 27.559 to 1.033 ms/iteration with the same
1,772-byte payload. CLI retains `acos(3/4)` and browser E2E checks its positive
enclosure below two radians.
