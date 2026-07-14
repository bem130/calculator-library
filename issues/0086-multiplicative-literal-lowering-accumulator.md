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
