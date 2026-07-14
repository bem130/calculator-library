# Issue 85: Fold additive numeric literals before DAG materialization

## Problem

Source lowering flattens an additive tree, but it first materializes every numeric
literal as a Rational expression node. `add_many_linear` immediately folds those
nodes into one constant and materializes that result, leaving the per-literal
Rationals and nodes unreachable but retained in the finished DAG. Wide exact sums
therefore allocate and retain structures that cannot affect evaluation,
presentation, domain obligations, or certified intervals.

At `main` commit `a6699bf`, the 256-term integer sum allocates 277,843 bytes in
10,085 blocks, peaks at 109,443 bytes / 1,891 blocks, and consumes 932 logical-work
units. Stage profiling identifies evaluation/lowering as the dominant native
stage.

## Required change

- Accumulate plain numeric literals while flattening additive source syntax and
  materialize their canonical Rational constant at most once.
- Preserve source traversal and decimal-literal error precedence, including signs
  introduced by subtraction. Do not introduce a wide-expression or integer-only
  syntax special case.
- Continue lowering non-literal terms through the existing DAG and canonical
  polynomial path. Do not fold domain-sensitive or otherwise non-literal syntax.
- Preserve source AST limits, canonical rewrite/logical-work accounting semantics,
  internal expression limits, deterministic exact output, no-float policy, and
  public protocol. Logical work may decrease only by the intern/hash/node and
  normalization work actually eliminated; prove with limit regressions that source
  protection remains prior and that no performed canonical work becomes uncharged.

## Acceptance

- Positive, negative, decimal, mixed literal/symbolic, nested add/subtract, and
  invalid/limited input regressions preserve results and error precedence.
- Wide-add scaling retains exact outputs; any logical-work boundary reduction is
  attributable to eliminated operations and does not bypass source limits.
- Native timing/allocation/stages and Wasm/npm boundary measurements show the
  attributable effect; exact rational, symbolic, algebraic, and approximate
  controls do not regress.
- Focused tests, package/example build, browser E2E, full repository gates, diff
  review, whole-system consistency review, and merge-granularity review complete
  before one integration into `main`.
