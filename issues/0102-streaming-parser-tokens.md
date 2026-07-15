# Issue 102: Stream validated tokens into the parser

## Problem

Issue 100 removed geometric token-vector growth by requesting an exact capacity,
but the parser still retains every large owned `Token` until parsing finishes.
For wide exact expressions this storage is now the largest avoidable live lexer
allocation even though the recursive-descent parser needs only one-token
lookahead and moves number payloads directly into the source AST.

## Requirements

- Keep the allocation-free whole-input lexical preflight so malformed numbers,
  unknown identifiers, unexpected Unicode, and their byte spans are reported
  before any parser error exactly as today.
- After preflight, scan tokens lazily with a single owned lookahead slot; do not
  materialize or retain a token vector.
- Preserve number payload ownership, whitespace and UTF-8 cursor behavior,
  unexpected-end offsets (including trailing whitespace), precedence,
  associativity, implicit multiplication, function/base syntax, and all public
  parse errors.
- Preserve source byte/node/depth limits, deterministic behavior, no-float
  policy, public protocol, and Wasm/npm behavior.
- Add focused equivalence and wide-input regressions; record native allocation,
  peak allocation, timing, logical work, and Wasm/npm boundary evidence.
- Complete repository gates and diff, whole-system, and merge-granularity
  reviews before one integration into `main`.
