# Issue 108: Convert exact exponential points once

## Problem

`exp` converts both certified dyadic endpoints to canonical Rational values
before checking equality. Exact-point inputs therefore repeat the same BigInt
shift/Rational construction even though all subsequent point paths use one
value and already share their series or binary-scaling plan.

## Requirements

- Detect canonical exact dyadic points before Rational conversion and convert
  the point once for ordinary, directly rounded, and binary-scaled exp paths.
- Preserve non-degenerate endpoint conversion and evaluation, error precedence,
  directed bounds, precision, logical-work/resource accounting, no-float policy,
  and public protocol.
- Measure ordinary, tiny-dyadic, large positive/negative, and general-power
  controls; add focused exact-point/non-degenerate regression coverage.
- Complete diff/whole-system reviews, repository gates, and merge-granularity
  review before one integration into `main`.

## Resolution

`exp` now detects equal canonical dyadic endpoints before conversion and sends
one Rational point through the existing ordinary, direct-dyadic, or shared
binary-scaling plan. Non-degenerate endpoints retain their independent
conversions and directed evaluation.

Against base `321a665`, one-call allocation moved `exp(1)` from 8,173 bytes /
295 blocks (1,423 / 26 peak) to 8,157 / 293 (1,407 / 24), tiny dyadic exp from
21,259 / 395 (4,154 / 38) to 21,211 / 391 (4,122 / 36), `exp(-10000)` from
502,372 / 1,460 to 502,356 / 1,458, and `exp(10000)` from 480,026 / 1,407 to
480,010 / 1,405. Large-exp peak was non-regressed. The non-degenerate general
power control was identical at 101,425 / 696 / 6,223. Logical-work output was
byte-identical at SHA-256 `a925d3238a37ac073ae380a8c0200c9c654944a71f9a3e573660740d55d6fbd7`.
Ten-sample `exp(1)` measured 8.630--8.974 us; this unpaired run is not a timing
claim.

The optimized Wasm artifact moved from 830,189 bytes to 830,350 bytes
(`45c4bdd0fcfdcf089c1654ea21a50b0d622ce986b752c8e48e73bd73dbd66a76`),
below budget. Ten-iteration/two-warmup boundary runs retained 1,824- and
1,794-byte payloads for tiny dyadic and negative-10000 exp and measured 0.323
and 3.682 ms/iteration; these short runs establish integration, not timing.

Repository validation passed formatting, workspace/all-feature and wasm-target
Clippy, no-default check/test, 381 core tests, 37 native Wasm tests, 23 Node
Wasm tests, doctests, generated/protocol/no-float checks, `cargo deny`, DTO
regeneration, package check, example build, arithmetic oracle, package-size
budget, browser E2E, and rustdoc. The added focused regression covers ordinary,
direct-dyadic, positive/negative binary-scaled points and a non-degenerate
interval against independent endpoint routes. Both pnpm audits remain blocked
by the registry's retired endpoint returning HTTP 410; manifests and lockfiles
are unchanged.
