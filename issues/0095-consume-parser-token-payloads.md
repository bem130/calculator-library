# Issue 95: Move owned token payloads into the source AST

## Problem

The lexer owns number and identifier payloads in its token vector. During
primary-expression parsing, the parser clones the current token before advancing,
so every number literal `String` is duplicated into the source AST while the
original remains live in the token vector until parsing finishes. Wide expressions
therefore allocate and retain storage proportional to their literal payload twice.

## Requirements

- Consume the current token and move owned payloads into the source AST without
  cloning them.
- Preserve lookahead, precedence, associativity, implicit multiplication,
  function parsing, byte spans, and parse-error precedence and payloads.
- Preserve source AST node/depth limits, exact semantics, logical-work charging,
  no-float policy, and every public protocol surface.
- Cover successful number/function/parenthesized parsing and representative
  errors around the consumed token boundary.
- Record deterministic parse/public-path allocation, scaling timing, logical
  work, and Wasm/npm boundary evidence with unchanged-result controls.
- Complete focused tests, repository gates, diff and whole-system reviews, and
  merge-granularity review before one integration into `main`.

