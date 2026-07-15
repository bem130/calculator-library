# Issue 100: Reserve exact lexer token capacity

## Problem

The lexer pushes large owned `Token` values into a zero-capacity `Vec`. At the
256-term wide-add boundary, geometric growth accounts for 32,640 allocated
bytes across eight blocks, almost half of the complete public calculation's
68,577 bytes. Reserving from source byte length would over-allocate long single
numeric literals and is not a valid token-count model.

## Requirements

- Perform an allocation-free lexical preflight using the same number,
  identifier, Unicode, whitespace, punctuation, span, and error rules.
- Allocate the token vector once at the exact validated token count, without
  changing token payload ownership or parser/error precedence.
- Preserve malformed-number and unknown/unexpected-token spans, UTF-8 behavior,
  implicit multiplication, source limits, logical work, and public protocol.
- Record wide-add and representative exact/approximate allocation and timing,
  logical work, and Wasm/npm boundary evidence.
- Complete repository gates and diff, whole-system, and merge-granularity
  reviews before one integration into `main`.
