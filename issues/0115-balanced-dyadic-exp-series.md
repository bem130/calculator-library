# Issue 115: Balance the direct dyadic exponential finite sum

## Problem

For non-degenerate dyadic exponential endpoints, the direct Taylor recurrence
repeatedly shifts the already-growing sum numerator and multiplies the growing
term numerator. In the `2^sqrt(2)` public path, four endpoint callsites account
for about 78 KB of the 101 KB deterministic allocation. Earlier work optimized
factor order, buffers, the final denominator, and exact-point term selection,
but did not change the linear finite-sum construction.

## Requirements

- Prototype an exact balanced segment representation for the dyadic Taylor
  ratios. Preserve the factorial base plus checked binary shift representation;
  do not materialize a giant power-of-two denominator at each merge.
- Keep term count, tail proof, directed rounding, positivity, monotonicity,
  refinement, stopping, logical-work/resource accounting, no-float policy, and
  the public protocol unchanged.
- Initially restrict dispatch to sufficiently long nonnegative direct dyadic
  series. Preserve zero/short, general Rational, negative reciprocal,
  binary-scaled, and value-aware tiny paths until independently measured.
- Compare the balanced state exactly with the legacy linear recurrence at zero,
  one, threshold boundaries, ordinary and large dyadic numerators, paired and
  directed bounds, and shared-denominator endpoint paths.
- Adopt only if deterministic bytes, blocks, peak allocation, and native timing
  are all non-regressing for general power and composite controls. Remove the
  prototype and record a negative result otherwise.
- Record logical work, Wasm/npm, CLI/example/browser, and repository gates;
  complete diff, consistency, and merge-granularity reviews before one
  integration into `main`.

## Resolution

The prototype was rejected and no runtime code is retained. At base `f13b268`,
the deterministic `approximate_general_power` case used 101,393 bytes / 692
blocks with a 6,223-byte / 43-block peak. A fully balanced one-term-leaf tree
reduced total bytes to 82,033, but increased blocks to 1,039 and peak to 7,311 /
52. Hybrid leaves of 8, 16, and 32 terms produced respectively 72,737 / 807,
73,825 / 783, and 84,345 / 758 bytes/blocks, while all retained a roughly
7.3-KB / 52-block peak.

The structured denominator avoided repeated giant powers of two, but each tree
merge still had to hold left and right products plus a scaled partial sum. That
simultaneous live state caused the peak and block regressions. A reverse nested
evaluation removed the tree, but moved the growing shift to a materialized
denominator and measured 110,585 / 716 with a 6,511 / 47 peak. It therefore
regressed total bytes, blocks, and peak.

All variants preserved exact regression results, but none met the stated
bytes/blocks/peak adoption gate. The branch restores the original recurrence;
future work should not retry leaf-width tuning or denominator materialization.
It requires a representation that can add a scaled partial sum without keeping
both child products live.

### Reproduction record

Measurements used `rustc 1.97.0 (2d8144b78 2026-07-07)`, Cargo 1.97.0, on
x86_64 Linux 6.18.33.2 under WSL2. Each allocation sample was one cold public
calculation:

```sh
CALCULATOR_ALLOCATION_ITERATIONS=1 \
  cargo run --profile bench -p calculator-core --features std \
    --example allocation_baseline -- approximate_general_power
cargo test -p calculator-core --features std exp_series -- --nocapture
```

The prototypes replaced the numerator loop in both
`exp_series_state_with_plan` and
`exp_series_state_with_common_denominator` only when the denominator was a
power of two and `term_count >= 16`. A segment for ratios
`p_k=a`, `q_k=2^s*k` stored `(P,Q_base,Q_shift,T)` and merged adjacent segments
as `P=P_l*P_r`, `Q_base=Q_l*Q_r`, `Q_shift=s_l+s_r`, and
`T=T_l*(Q_r<<s_r)+P_l*T_r`; the root sum numerator was
`(Q_base<<Q_shift)+T`. Full-balanced used one ratio per leaf. Hybrid variants
used the existing linear recurrence inside leaves of 8, 16, or 32 ratios before
the same merge. Reverse nested evaluation used, for `k=N..1`,
`sum=sum*a; denominator=denominator*k<<s; sum+=denominator`, then constructed
`a^N` once for the unchanged upper tail. These formulas and dispatch boundaries
are sufficient to reconstruct each measured variant without retaining rejected
runtime code in the repository.

The deterministic allocation gate ran before native timing, logical-work,
Wasm/npm, CLI, and browser candidate gates. Because every prototype already
failed the required blocks and peak criteria, those downstream candidate
measurements were intentionally not run. After restoring the original runtime,
the focused exact test and allocation baseline above passed, followed by the
complete repository Rust, contract, package/example, oracle, browser, size, and
documentation gates. No public artifact or protocol changed.
