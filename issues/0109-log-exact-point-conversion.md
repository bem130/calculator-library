# Issue 109: Convert exact logarithm points once

## Problem

`log` converts both certified dyadic endpoints to Rational before domain and
equality classification. Canonical exact points therefore repeat an identical
BigInt shift and Rational construction before entering the paired raw-log path.

## Requirements

- Detect canonical exact dyadic points before conversion and convert once,
  while preserving logarithm domain/error classification.
- Preserve structurally different/noncanonical equal endpoints through the
  existing post-conversion equality fallback and keep non-degenerate directed
  evaluation unchanged.
- Measure `ln(2)`, large positive log, non-degenerate log, logical work, and
  Wasm/npm boundary; add focused point/domain/non-degenerate regressions.
- Preserve precision, range reduction, resource accounting, no-float policy,
  and public protocol; complete reviews and repository gates before one
  ff-only integration into `main`.

## Resolution

Canonical equal dyadic endpoints are now classified before conversion. One
Rational point performs the existing nonpositive-domain check and paired raw
log evaluation. Structurally unequal endpoints still use the former two
conversions, ordering validation, and post-conversion equality fallback.

Against base `08999e3`, `ln(2)` allocation moved from 10,022 bytes / 406 blocks
(1,574 / 29 peak) to 9,990 / 402 (1,560 / 19), and the large-positive-log case
from 95,980 / 938 (12,146 / 27) to 95,900 / 934 (12,114 / 25). The
non-degenerate log control was identical at 158,393 / 942 / 12,590. Logical
work output retained SHA-256
`a925d3238a37ac073ae380a8c0200c9c654944a71f9a3e573660740d55d6fbd7`.
A ten-sample unpaired `ln(2)` run measured 20.29--25.83 us and is not a timing
claim.

The optimized Wasm artifact is 830,527 bytes at
`73ac6c23478b12ec306f7fed400349f1175e739f7f9265bf4b23f37d42448b1a`,
within budget. A ten-iteration/two-warmup large-log boundary run retained its
1,834-byte payload and measured 0.635 ms/iteration; it establishes integration,
not a timing claim.

Repository validation passed formatting, workspace/all-feature and wasm-target
Clippy, no-default check/test, 382 core tests, 37 native Wasm tests, 23 Node
Wasm tests, doctests, generated/protocol/no-float checks, `cargo deny`, DTO
regeneration, package check, example build, arithmetic oracle, package-size
budget, browser E2E, and rustdoc. Focused coverage includes positive points,
nonpositive domain errors, a true non-degenerate interval, and structurally
different but Rational-equal dyadic endpoints through the legacy fallback.
Both pnpm audits remain blocked by the retired registry endpoint returning HTTP
410; manifests and lockfiles are unchanged.
