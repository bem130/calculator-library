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

## Resolution

The lexer now returns the scanned end offset and classifies
`&source[start..end]` directly. No identifier text is copied or retained; known
tokens and unknown-identifier spans preserve the existing grammar and errors.

At base `20cf7fc`, deterministic allocation moved from 51,603 bytes / 1,485
blocks to 51,588 / 1,480 for the five-identifier exact-symbolic case, from
165,260 / 2,695 to 165,251 / 2,692 for the three-identifier approximate case,
and from 8,282 / 310 to 8,279 / 309 for `sin(1)`. These reductions exactly match
the identifier lengths and counts; peaks were unchanged. Approximate logical
work remains 400,447 units.

Alternating 20-sample parse-only ranges overlapped at 1.457--1.786 us for base
and 1.506--1.617 us for candidate, so no native timing claim is made. The public
Wasm/npm smoke retained its 1,780-byte payload and 3,640-byte JS heap delta;
short samples moved from 1.169 to 0.776 ms/iteration and are boundary evidence
only. The artifact moved from 828,720 bytes
(`c4e863ac13c094a1147943165843e1725900bfda009090d64bbd4075f840a91a`)
to 828,671 bytes
(`7b8fbbe10259480120c58105088d0218a29097b608edc2da802363428288324d`).

The final CI-equivalent gates passed formatting; native and Wasm clippy;
no-default checks and 373 core tests; workspace 373 core, 37 native-Wasm, and 2
CLI tests; doc tests; 23 Node-Wasm tests; generated/protocol/no-float checks;
dependency policy and audits; package type/presentation checks; package and
example builds; the external oracle; package-size budget; browser E2E; and
documentation generation. The final example build retained the candidate
SHA-256 and 828,671-byte artifact recorded above.
