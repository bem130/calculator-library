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

## Resolution

The lexer now scans validated lexemes once without constructing numeric payloads,
counts exactly the non-whitespace tokens, then allocates the token vector at
that capacity before the materializing pass. Both passes share `scan_lexeme`,
including number/exponent, identifier, Unicode pi, punctuation, span, and error
construction, so the preflight cannot accept a different lexical language.

Same-host one-iteration allocation moved wide add from 68,577 bytes / 2,248
blocks to 52,289 / 2,241, with peak live allocation 37,444 / 767 versus
37,412 / 767. Exact rational moved 12,075 / 495 to 11,915 / 494, algebraic
43,402 / 1,842 to 42,346 / 1,839, and the combined approximate path 115,851 /
1,139 to 115,467 / 1,137. Concurrent 50-sample ranges overlapped for wide add
(151.05--176.53 us base, 142.86--176.32 us candidate) and exact rational
(31.90--33.07 us base, 31.86--33.80 us candidate), so no timing claim is made.

The complete logical-work output stayed byte-identical at SHA-256
`a925d3238a37ac073ae380a8c0200c9c654944a71f9a3e573660740d55d6fbd7`.
The ten-iteration/two-warmup Wasm/npm wide-add path retained its 1,728-byte
payload and measured 0.741 ms/iteration base versus 0.863 ms candidate; the
short run is boundary evidence, not a timing claim. The optimized artifact
moved from 829,230 bytes
(`5cea3933bbd249eca968400337cb48b9b2dcbd79785d12d6a6c896ce409a44bd`)
to 829,554 bytes
(`4b507aaf91b9237f46981672fe0b846aaf832e22f799630ee00f858caad58793`)
and remains below budget.

Repository validation passed formatting, workspace/all-feature and wasm-target
Clippy, no-default check/test, 377 core tests, 37 native Wasm tests, 23 Node
Wasm tests, doctests, generated/protocol/no-float checks, `cargo deny`, DTO
regeneration, package check, example build, arithmetic oracle, package-size
budget, browser E2E, and rustdoc. Both pnpm audit commands are currently
blocked by the registry's retired legacy audit endpoint returning HTTP 410;
the same commands completed with `--ignore-registry-errors`, and dependency
manifests and lockfiles are unchanged by this slice.
