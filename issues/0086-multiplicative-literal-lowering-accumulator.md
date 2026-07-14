# Issue 86: Fold multiplicative numeric literals before DAG materialization

## Problem

Source lowering handles every binary multiplication independently. A wide product
therefore materializes each numeric literal and each intermediate product, then
`multiply_many_factors` immediately folds the rational factors again. The
unreachable Rational values and expression nodes remain retained in the completed
DAG, while growing integer products are repeatedly normalized and hashed.

At base commit `ae637b6`, establish reproducible native timing, allocation,
logical-work, and Wasm/npm measurements for a representative wide exact product
and exact-rational, symbolic, algebraic, approximate, and wide-add controls before
changing the lowering algorithm.

## Required change

- Flatten source multiplication and accumulate plain numeric literals before DAG
  materialization, without an input-shape or benchmark-specific special case.
- Preserve left-to-right parsing/error precedence and unary signs. Division,
  percent, power, functions, constants, and other domain-sensitive expressions
  remain non-literal factors and continue through their existing lowering paths.
- Preserve the rule that zero may remove other factors only when every removed
  factor is proven defined. An unproven or failing factor must not be hidden by
  early literal accumulation.
- Charge conservative structural logical work before each Rational product. If a
  reservation or another canonical limit fails, latch the failure, retain later
  literals unfused after parsing them, and do not resume exact simplification.
- Preserve source byte/AST node/depth protection, internal expression limits,
  deterministic canonical ordering, exact output, no-float policy, typed limits,
  and the public protocol. A supported-range increase must come only from removed
  unreachable DAG state and repeated normalization, not a raised limit.

## Acceptance

- Integer, fractional, decimal, signed, nested, zero, mixed symbolic, and invalid
  or limited products have focused exact/error/limit regressions.
- Wide-product scaling and limit boundaries demonstrate that all performed
  multiplication work is charged and failure does not resume folding.
- Native timing/allocation/stages and Wasm/npm boundary before/after evidence is
  recorded with unchanged-result controls and remaining bottlenecks.
- Focused tests, package/example build, browser E2E, full repository gates, diff
  review, whole-system consistency review, and merge-granularity review complete
  before one integration into `main`.

## Resolution

Multiplication lowering now flattens source multiplication only and accumulates
plain signed numeric literals before DAG materialization. Each Rational product
reserves conservative numerator/denominator multiplication and normalization work
before mutation. A failed fold is latched, later source is still lowered for
error precedence, and exact folding does not resume. Divide, percent, power, and
domain-sensitive factors remain on their prior paths; zero removes factors only
when all are proven defined.

For `1*2*...*128`, allocation moved from 327,178 bytes / 11,050 blocks to
86,818 / 3,212, peak live allocation from 79,411 / 1,478 to 18,904 / 511, and
logical work from 38,176 to 6,377 units. A 6,376-unit request returns a typed
Partial with the same exact 128! expression. Wasm/npm v19 moved from 14.664 ms to
6.419 ms per iteration in the three-iteration/one-warmup smoke with the same
2,162-byte payload. The final artifact is 824,961 bytes with SHA-256
`4ad06055917fd804f2b3cb89a6ee02a9c8e4fd0937fef9bde5c0f70f3ae93c04`.

Focused signed/decimal/nested/mixed/zero-domain/division/percent/resource tests,
native allocation/scaling controls, package/example builds, browser E2E, and the
repository gates cover the acceptance criteria. Detailed reproduction commands
and the remaining BigInt scaling bottleneck are in `doc/performance-baselines.md`.
