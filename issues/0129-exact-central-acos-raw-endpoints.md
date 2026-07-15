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

