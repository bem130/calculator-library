# Issue 117: Reuse outer-acos classification squares

## Problem

Non-degenerate `acos` classifies each selected rational endpoint by comparing
`2*n^2` with `d^2`. An outer endpoint then immediately constructs
`1-x^2=(d^2-n^2)/d^2`, recomputing both arbitrary-precision squares. The
classification and complement are one logical operation but currently discard
their shared structural work.

## Requirements

- Represent outer-region classification so its exact numerator and denominator
  squares can be consumed by direct positive and negative acos evaluation.
- Keep the allocation-free bit-length proof for clearly central values and do
  not add work to central, unit-series, special, or exact-point paths.
- Preserve the exact `2*n^2 < d^2` boundary decision, canonical complement,
  positive/negative directed rounding, antitone endpoint selection, and lazy
  shared-pi construction.
- Preserve determinism, stopping, logical-work/resource accounting, no-float
  policy, public protocol, and error precedence. Do not special-case a source
  expression or raise limits.
- Add focused equivalence and boundary regressions. Measure deterministic
  allocation/peak, native timing, logical work, Wasm/npm, CLI and browser
  controls before and after.
- Complete diff, whole-system, and merge-granularity reviews plus repository
  gates before one ff-only integration into `main`.

## Resolution

Rejected after measurement. A prototype returned an owned classification plan
containing `n²` and `d²`, moved those squares through the directed endpoint
evaluator, and constructed the canonical complement without recomputing them.
Focused positive/negative boundary, containment, antitone, and complement
equivalence tests passed, and logical-work output remained byte-identical.

Against base `461a8d4`, the positive non-degenerate target moved only from
958,986 bytes / 2,470 blocks to 958,826 / 2,468, with the same 58,069 / 81
peak. A negative outer control moved from 1,102,868 / 2,742 to 1,102,820 /
2,741 with the same 62,086 / 86 peak. The negative central control remained
exactly 1,019,757 / 1,793 (48,790 / 74 peak).

Native samples were unstable and did not justify a timing claim: candidate
ranges moved from 17.771--20.167 ms to 14.753--15.897 ms across consecutive
30-sample runs, while the base range was 12.636--13.525 ms. The optimized Wasm
artifact grew from 833,423 to 833,565 bytes (+142). The small transient
allocation reduction does not justify the ownership enum, wider signatures,
and artifact growth, so commits `61e347e`/`f559cef` retain the audited
experiment while restoring runtime and benchmark code exactly.

Three-iteration/one-warmup npm public-path smokes were 98.84 ms/iteration at
base and 99.48 ms for the prototype with the same 1,794-byte payload; these
short runs support no timing claim. The restored tip passed formatting,
clippy, no-default-feature, 390 core, 37 native Wasm, 23 wasm32, doc, generated
DTO, protocol, no-float, dependency-policy, package/example build, browser E2E,
and package-size gates. Both exact pnpm audits reached the registry's retired
endpoint and returned HTTP 410 rather than an advisory result.
The prototype artifact SHA-256 was
`6cdae4fa3d7371e565c68b83eb8566812bcaa2f884776fe9deb87f0f5d775e75`;
the restored 833,423-byte artifact returned to base SHA-256
`469f6df75aa63ffac27e3b7099456a6238ca89c0d1950637b292564eba44781b`.
At both base/restored and prototype commits, CLI controls returned the identical
canonical sources `acos(1/3*sin(1)+2/3)` and `acos(1/7*sin(1)-6/7)`.
The prototype package/example build and browser E2E also passed, as did the
restored-tip browser gate, so no display or public-path state changed.

Do not repeat this cache-lifetime approach without evidence that the squared
operands are materially larger or a representation that reduces code size and
does not extend their live range. The remaining dominant work is sqrt/atan and
the enclosing public evaluation pipeline, not these two short-lived squares.
