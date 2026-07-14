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
- Preserve source AST limits, canonical rewrite/logical-work accounting, internal
  expression limits, deterministic exact output, no-float policy, and public
  protocol. If eliminating unreachable nodes changes a limit boundary, prove that
  the removed nodes were not observable work or weaken resource protection.

## Acceptance

- Positive, negative, decimal, mixed literal/symbolic, nested add/subtract, and
  invalid/limited input regressions preserve results and error precedence.
- Wide-add scaling retains exact outputs and the existing logical-work boundary.
- Native timing/allocation/stages and Wasm/npm boundary measurements show the
  attributable effect; exact rational, symbolic, algebraic, and approximate
  controls do not regress.
- Focused tests, package/example build, browser E2E, full repository gates, diff
  review, whole-system consistency review, and merge-granularity review complete
  before one integration into `main`.
