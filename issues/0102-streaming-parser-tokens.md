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

## Resolution

Parsing now retains the source cursor, the last materialized token end, and one
owned lookahead token. The allocation-free first scan still validates the entire
lexical language before parser construction; the second scan materializes each
number or operator only when recursive descent requests it. Consumed number
payloads move directly into the source AST, while trailing whitespace does not
change the unexpected-end offset.

Against base `1f3c267`, one-call `wide_add_256` allocation moved from 52,289
bytes / 2,241 blocks to 35,937 / 2,240, with peak allocation from 37,412 bytes /
767 blocks to 24,014 / 812. `wide_multiply_128` moved from 42,918 / 1,361 to
34,758 / 1,360, with peak from 18,596 / 383 to 13,228 / 428. The higher peak
block counts reflect shorter-lived AST/token overlap while peak bytes decreased.
Exact rational moved 11,915 / 494 to 11,691 / 493, algebraic 42,346 / 1,839 to
41,482 / 1,838, and approximate 115,467 / 1,137 to 114,955 / 1,136; their peak
values were unchanged.

Logical work stayed at 261 units for wide add and 1,339 for wide multiply.
Concurrent 20-sample wide-product ranges moved from 92.23--99.04 us to
79.41--87.02 us. Candidate wide-add measured 136.82--147.08 us and exact
rational 32.85--37.50 us; their earlier samples were taken under different host
load, so no comparative timing claim is made for them.

The ten-iteration/two-warmup Wasm/npm wide-add path retained its 1,728-byte
payload and measured 0.863 ms/iteration at the base versus 1.267 ms candidate;
the short boundary run is not used for a timing claim. The optimized artifact
moved from 829,554 bytes
(`4b507aaf91b9237f46981672fe0b846aaf832e22f799630ee00f858caad58793`)
to 829,645 bytes
(`10bea59a6984261b3fe247e3cb3a493d3c4ddb0ca39d6ff437c7a90fd2816b9f`)
and remains below budget.

Repository validation passed formatting, workspace/all-feature and wasm-target
Clippy, no-default check/test, 379 core tests, 37 native Wasm tests, 23 Node
Wasm tests, doctests, generated/protocol/no-float checks, `cargo deny`, DTO
regeneration, package check, example build, arithmetic oracle, package-size
budget, browser E2E, and rustdoc. Both pnpm audit commands remain blocked by
the registry's retired legacy endpoint returning HTTP 410; registry-error
tolerance allowed the remaining gates to run, and dependency manifests and
lockfiles are unchanged.
