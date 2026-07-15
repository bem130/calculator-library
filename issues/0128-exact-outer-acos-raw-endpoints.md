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

