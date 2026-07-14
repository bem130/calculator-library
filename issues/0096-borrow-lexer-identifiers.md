# Issue 96: Classify lexer identifiers from borrowed source slices

## Problem

The lexer copies every ASCII identifier into a temporary `String`, classifies
that string into a payload-free `Constant` or `Function` token, and immediately
drops the allocation. The token and source AST never retain the identifier text,
so this allocation is unrelated to the result and scales with function-heavy
input.

## Requirements

- Scan the identifier boundary once and classify the borrowed source slice
  without allocating an owned string.
- Preserve the accepted identifier set, reserved/unknown identifier behavior,
  byte spans, expected-token payloads, function parsing, and implicit
  multiplication.
- Preserve source/input limits, exact semantics, logical-work accounting,
  no-float policy, and public protocol.
- Cover known constants/functions, alphanumeric/underscore unknown identifiers,
  adjacent implicit multiplication, and UTF-8 boundary behavior.
- Record native parse/public allocation and timing, logical work, Wasm/npm
  boundary evidence, and unchanged-result controls.
- Complete repository gates and subagent diff, whole-system, and merge-granularity
  reviews before one integration into `main`.

