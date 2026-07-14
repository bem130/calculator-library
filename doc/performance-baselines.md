# Performance Baselines

Performance work starts from reproducible representative-path measurements. These
benchmarks are engineering diagnostics, not part of the calculator's public API or
cross-machine pass/fail thresholds.

## Native core benchmark

Run the Criterion harness with a CPU performance governor and background workload
held as stable as practical:

```sh
cargo bench -p calculator-core --bench representative_paths
```

The harness fixes Criterion to 10 samples with one-second warm-up and measurement
windows, matching the sampling configuration used below.

The harness covers exact rational arithmetic, canonical symbolic reduction,
certified approximate evaluation, bounded algebraic recognition, a 256-term wide
expression, and a headless session action sequence. Criterion stores statistical
samples under `target/criterion`; those generated results are intentionally not
committed.

The benchmark profile uses `opt-level = 3`, one codegen unit, debug symbols for
profilers, and no LTO. This is deliberately separate from the size-oriented Wasm
release profile. Use `perf record --call-graph dwarf` against the generated bench
executable when a case identifies a regression.

## Wasm/npm boundary benchmark

Build the package, then run the public facade benchmark:

```sh
corepack pnpm --dir packages/calculator run build:wasm
corepack pnpm --silent --dir packages/calculator run benchmark > target/wasm-baseline.json
```

The JSON includes runtime identity, iteration counts, elapsed time per case, and
retained JavaScript heap change after an explicit GC when available. Retained heap
is an allocation/leak proxy, not total allocation traffic or Wasm linear-memory
usage. Compare results only on the same runtime, architecture, build profile, and
toolchain. `CALCULATOR_BENCH_ITERATIONS` and `CALCULATOR_BENCH_WARMUP` control the
run without changing case definitions.

## Logical work and correctness gates

Logical work is deterministic resource accounting, not elapsed time. Benchmark
changes must keep the existing focused resource-limit tests and the full workspace
tests green; they must not reduce work charges to improve wall-clock measurements.
Before and after an optimization, record:

1. the exact commit and toolchain (`rustc -Vv`, `node --version`, `wasm-opt --version`),
2. the benchmark case and Criterion estimate or Wasm JSON result,
3. the relevant resource-limit regression tests,
4. package size and protocol/conformance gates when Wasm-facing code changes.

The first baseline intentionally measures cold `EvaluationContext` calculations.
Warm-context/cache benchmarks should be added only with an explicit assertion that
observable results and logical-work limit decisions remain identical.

Native allocation traffic is measured separately with `dhat`, keeping allocator
instrumentation out of timing samples:

```sh
CALCULATOR_ALLOCATION_ITERATIONS=10 \
  cargo run --profile bench -p calculator-core --features std \
  --example allocation_baseline -- approximate
```

`dhat` reports total and peak live bytes/blocks and writes `dhat-heap.json` for
call-site inspection. That generated file is not committed. Wasm linear-memory
allocation is not attributed by this runner; retained JavaScript heap remains a
boundary leak/payload proxy. Source construction, the calculation request, and the
session policy are prepared before profiling so each iteration matches the timed
native operation scope.

The approximate composite also has fixed `approximate_exp_one`,
`approximate_log_two`, `approximate_general_power`, and `approximate_sin_one`
allocation cases. They use the same public `calculate` boundary as the composite;
the `approximate_components` Criterion group prepares parsed inputs and measures
only `evaluate` so parse and presentation do not obscure the component comparison.

Deterministic logical-work baselines use the smallest custom limit that produces
an outcome exactly equal to the default-limit outcome:

```sh
cargo run --profile bench -p calculator-core --features std \
  --example logical_work_baseline
```

This is an observable limit boundary rather than internal timing. Optimization
must not lower the charge merely to improve the number.

## Initial diagnostic baseline

The first run on 2026-07-11 used `rustc 1.97.0` on
`x86_64-unknown-linux-gnu`, Node `v22.23.1`, and Binaryen 130. Criterion used
10 samples with one-second warm-up and measurement windows; the Wasm runner used
10 iterations after two warm-ups with `--expose-gc`. These values establish the
local comparison point only:

| Case | Native estimate | Wasm facade ns/iteration | Wasm retained heap | Logical work |
| --- | ---: | ---: | ---: | ---: |
| exact rational | 49.9 µs | 592,318 | 4,944 B | 231 |
| exact symbolic | 3.77 ms | 21,167,098 | 11,936 B | 401,216 |
| approximate | 73.8 ms | 401,063,413 | 14,928 B | 400,447 |
| algebraic | 243 µs | 1,052,608 | 38,088 B | 400,234 |
| wide add (256 terms) | 611 µs | 1,926,770 | 9,512 B | 932 |
| session dispatch sequence | 655 ns | 187,998 | 12,808 B | not applicable |

## Post-directed-transcendental and square-root baseline

Before the later binary-scaled large-exponential slice, the logarithm, directed
exponential, trigonometric, and square-root work at commit
`1767131` was measured on 2026-07-12 with `rustc 1.97.0`, Node `v22.23.1`, and
Binaryen 130. Criterion used the standard 10-sample configuration. All unrelated
native controls were slower than their preceding stored samples during this run,
so these native values are a new local snapshot rather than regression claims.
The Wasm runner used its default 10 measured iterations after two warm-ups with
`--expose-gc`; allocation rows used one measured public calculation. Allocation
and logical-work rows are deterministic under the documented runners.

| Case | Native median estimate | Native allocated bytes / blocks | Peak bytes / blocks | Wasm ns/iteration | Wasm retained heap | Logical work |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| exact rational | 52.844 µs | 13,382 / 620 | 2,624 / 52 | 1,242,552 | 3,552 B | 231 |
| exact symbolic | 475.23 µs | 121,348 / 5,178 | 8,853 / 98 | 8,048,350 | 11,936 B | 401,216 |
| approximate composite | 1.6165 ms | 361,355 / 7,434 | 12,990 / 59 | 21,831,016 | 14,880 B | 400,447 |
| algebraic | 301.50 µs | 142,739 / 5,318 | 4,884 / 104 | 3,961,319 | 27,840 B | 400,234 |
| wide add (256 terms) | 705.06 µs | 284,091 / 10,866 | 109,443 / 1,891 | 10,488,154 | 9,464 B | 932 |
| session dispatch | 831.91 ns | 102 / 20 | 26 / 3 | 568,070 | 7,552 B | not applicable |

The same-run approximate component snapshot was 42.028 µs for `exp(1)`,
318.50 µs for `ln(2)`, 1.6010 ms for direct `2^sqrt(2)`, 280.76 µs for
`sin(1)`, 120.38 µs for `sqrt(2)`, 587.89 µs through
`sqrt(2)*ln(2)`, and 1.8150 ms through `exp(sqrt(2)*ln(2))`. The cumulative
rows are not expected to be additive because each benchmark evaluates a complete
expression with its own exact normalization. Their one-iteration allocations
were respectively 20,157, 39,949, 289,094, 37,039, 60,249, 129,386, and
323,262 bytes. General power and its final non-degenerate exponential remain the
largest component paths, so the next profiling slice should separate their
remaining series-state, rational canonicalization, and exact-normalization costs.

In the initial diagnostic run, the approximate composite was the dominant
measured path in both environments. Its Wasm facade time was roughly five times
the native estimate in that run, motivating separation of interval refinement
from DTO serialization before choosing the first optimization. A single
retained-heap delta cannot establish a
growth trend and is too coarse to identify total allocation traffic.

One-iteration native `dhat` baselines are:

| Case | Allocated bytes | Blocks | Peak live bytes | Peak blocks |
| --- | ---: | ---: | ---: | ---: |
| exact rational | 13,382 | 620 | 2,624 | 52 |
| exact symbolic | 402,444 | 10,752 | 8,853 | 98 |
| approximate | 4,916,627 | 28,370 | 38,566 | 66 |
| algebraic | 144,819 | 5,414 | 4,884 | 104 |
| wide add (256 terms) | 284,091 | 10,866 | 109,443 | 1,891 |
| session dispatch sequence | 102 | 20 | 26 | 3 |

All six cases retained zero native bytes at process exit. Use the fixed case names
shown in the table when collecting comparisons; arbitrary source strings are not
accepted by the allocation runner.

The native session row measures reducer state creation and eight actions. The Wasm
row additionally includes Wasm session construction/disposal, DTO conversion,
dispatch, and final state retrieval; the rows diagnose their respective layers
rather than identical operation scope.

## Approximate-path profiling and first optimization

The `approximate_stages` Criterion group separates the dominant composite into
public core API stages. On 2026-07-11 it measured parse at about 0.95 µs,
presentation at about 12.6 µs, and evaluation at 74.7–101 ms. Evaluation therefore
dominates; DTO/presentation work is not a plausible explanation for the native
cost.

Native `dhat` call stacks then identified repeated
`multiply -> compare_dyadic -> dyadic_to_rational -> Rational::new/gcd` traffic.
Endpoint ordering converted exact dyadics into reduced rationals even though their
coefficients can be compared directly after aligning powers of two. Direct dyadic
comparison removes those GCD/division allocations without changing interval
semantics or logical-work accounting.

With the same 10-sample harness, the composite estimate moved from the initial
73.8 ms to 70.1 ms (about 5% lower). One-iteration native allocation moved from
4,916,627 bytes in 28,370 blocks to 4,907,523 bytes in 27,984 blocks; peak live
memory and the 400,447-unit logical-work boundary were unchanged. The optimization
is deliberately kept despite the modest byte reduction because it removes an
algorithmically unnecessary canonical-rational conversion from every dyadic
endpoint comparison. Further work should profile the exp/log series and general
power refinement inside evaluation rather than target serialization first.

## Exact-point exp/log bound reuse

The next profile showed that exact dyadic points passed to `exp` and `log` still
evaluated the same rational Taylor bound routine independently for equal lower and
upper endpoints. This occurs for direct `ln(2)` and again for the exact base inside
`2^sqrt(2)`. A point has one directed lower/upper bound pair, so the interval
backend now computes that pair once; non-degenerate intervals continue to evaluate
their distinct endpoints independently.

On 2026-07-12, the same 10-sample composite moved from the post-dyadic-comparison
70.1 ms estimate to 63.4 ms (about 9.5% lower). One-iteration native allocation
moved from 4,907,523 bytes in 27,984 blocks to 4,819,171 bytes in 23,754 blocks.
Peak live memory and the 400,447-unit logical-work boundary were unchanged. The
stage group continued to place essentially all material time in evaluation; parse
remained below 1 µs and presentation near 13 µs. Further work should separate the
remaining general-power exp and log series terms before changing their algorithms.

## Reduced logarithm identity fast path

Separating the logarithm range reduction exposed an identity case inside the
remaining series work. `ln(2)` is reduced to `log(1) + log(2)`: the first term is
exactly zero, but the interval backend still built every zero Taylor term requested
by the precision before calculating `log(2)`. The reduced logarithm primitive now
returns the exact directed pair `[0, 0]` for one before entering the series. This is
the mathematical identity element of the existing range-reduction algorithm, not
a precision or iteration-limit change.

On 2026-07-12, the same 10-sample composite estimate was 60.9 ms, about 4% below
the preceding 63.4 ms run. The evaluation-only estimate moved from 70.9 ms in the
immediately preceding local run to 63.5 ms; parse and presentation remained outside
the material cost. One-iteration native allocation moved from 4,819,171 bytes in
23,754 blocks to 4,808,083 bytes in 22,740 blocks. Peak live memory and the
400,447-unit logical-work boundary remained unchanged. Timing samples on this host
show noticeable scheduler variance, while the deterministic removal of 1,014
allocation blocks provides the stronger local evidence for this fast path.

## Unit-range exponential reduction identity

Profiling the exponential half of the general-power composition found another
identity case around the Taylor series rather than inside it. For every positive
argument in `(0, 1]`, exponential range reduction chooses a factor of one. The
backend nevertheless divided the argument by one and raised each directed series
bound to the first power, canonicalizing the same large rationals three additional
times. The unit-range path now returns the small-series directed pair directly;
arguments above one retain the existing reduction and positive integer powers.

On 2026-07-12, one-iteration native allocation for the approximate composite moved
from 4,808,083 bytes in 22,740 blocks to 4,760,003 bytes in 22,684 blocks. Peak live
memory moved from 38,566 bytes in 66 blocks to 38,478 bytes in 62 blocks, and the
400,447-unit logical-work boundary was unchanged. Wall-clock comparison was
inconclusive: the after run slowed together with the parse and presentation control
groups by roughly 50%, identifying host load rather than this evaluation-only
change. The deterministic removal of division-by-one and first-power rational
canonicalization is therefore the evidence retained for this slice.

## Approximate component baseline

The general-power follow-up separates `exp(1)`, `ln(2)`, `2^sqrt(2)`, and `sin(1)`
without exposing private interval primitives to the benchmark. The preflight also
requires every case to retain a general-symbolic exact value and an available
certified enclosure, preventing a later simplifier change from silently measuring
a different path. On 2026-07-12, commit `a67ab54` with the same `rustc 1.97.0`
toolchain recorded these 10-sample evaluation-only estimates:

| Component | Estimate | Allocated bytes | Blocks | Peak bytes | Peak blocks |
| --- | ---: | ---: | ---: | ---: | ---: |
| `exp(1)` | 265.5 µs | 50,981 | 2,044 | 1,719 | 27 |
| `ln(2)` | 409.6 µs | 56,005 | 2,175 | 2,046 | 47 |
| `2^sqrt(2)` | 57.7 ms | 4,538,222 | 16,548 | 37,143 | 49 |
| `sin(1)` | 1.65 ms | 170,503 | 4,257 | 2,703 | 33 |

The allocation rows use one iteration per command:

```sh
for case in \
  approximate_exp_one \
  approximate_log_two \
  approximate_general_power \
  approximate_sin_one
do
  CALCULATOR_ALLOCATION_ITERATIONS=1 \
    cargo run --profile bench -p calculator-core --features std \
      --example allocation_baseline -- "$case"
done
```

Allocation rows use one full public calculation, so their fixed parser and output
cost differs from the evaluation-only timing boundary. Even with that conservative
boundary, general power accounts for about 95% of the composite's allocated bytes
and is over an order of magnitude slower than the other components combined. The
next optimization should therefore profile the non-degenerate exponential bounds
produced after multiplying `log(base)` by the irrational exponent; changing the
standalone exp/log series or presentation is not supported by this baseline.

## Factorial-bounded exponential series

The non-degenerate general-power profile showed that the exponential backend used
the shared heuristic `precision_bits / 3 + 16` as a Taylor term count. At 128 bits
this evaluates 58 terms even after range reduction guarantees `0 <= x <= 1`.
For that unit range, the complete tail after term `N` is at most twice term
`N + 1`, hence at most `2 / (N + 1)!`. The exponential path now chooses the
smallest `N` satisfying `(N + 1)! >= 2^(precision_bits + 1)`. This integer-only
bound is sufficient for a tail no wider than `2^-precision_bits`; it changes no
directed-rounding, refinement, or public precision contract.

On 2026-07-12, the 10-sample `2^sqrt(2)` evaluation estimate moved from 57.7 ms
to 18.35 ms (about 68% lower), while the full approximate calculation measured
20.4 ms. One-iteration general-power allocation moved from 4,538,222 bytes in
16,548 blocks to 1,803,214 bytes in 14,737 blocks, with peak live memory moving
from 37,143 to 22,567 bytes. The full approximate allocation moved from 4,760,003
bytes in 22,684 blocks to 2,024,995 bytes in 20,873 blocks, with peak live memory
moving from 38,478 to 23,902 bytes. The deterministic logical-work boundary stayed
at 400,447 units because resource charging describes the public algorithmic request
boundary rather than host-dependent internal optimization work.

## Fused exponential term update

After reducing the number of exponential terms, allocation stacks showed that each
remaining recurrence step still canonicalized `term * x`, then immediately built
and canonicalized a second rational for division by the positive integer `n`.
The recurrence now constructs `(term.numerator * x.numerator) /
(term.denominator * x.denominator * n)` once. `Rational::new` still performs the
same exact sign and GCD normalization, but the transient rational and its redundant
GCD pass are removed.

On 2026-07-12, the 10-sample `2^sqrt(2)` evaluation estimate moved from the
factorial-tail baseline of 18.35 ms to 11.80 ms (about 36% lower), and the full
approximate calculation measured 13.24 ms instead of 20.4 ms. One-iteration
general-power allocation moved from 1,803,214 bytes in 14,737 blocks to 1,548,238
bytes in 14,054 blocks. Full approximate allocation moved from 2,024,995 bytes in
20,873 blocks to 1,770,019 bytes in 20,190 blocks. Peak live memory remained
22,567 bytes for general power and 23,902 bytes for the composite; the deterministic
logical-work boundary remained 400,447 units.

## Common-denominator exponential summation

The post-fusion profile still spent most general-power time canonicalizing both the
Taylor term and accumulated sum on every iteration. For reduced `x = a/b`, the
implementation now keeps the term and partial sum over the shared denominator
`b^n * n!` using integer recurrences. It constructs canonical rationals only for
the final lower bound and the upper bound that includes twice the first omitted
term. A regression compares this representation with the former per-step Rational
recurrence for zero, interior, near-one, and unit inputs across multiple term
counts.

On 2026-07-12, `2^sqrt(2)` moved from 9.48 ms in the immediate pre-change component
run to 1.17 ms (about 88% lower). General-power allocation moved from 1,548,238
bytes in 14,054 blocks to 481,190 bytes in 12,706 blocks, and peak live memory moved
from 22,567 to 11,559 bytes. The full approximate calculation moved from 13.24 ms
to 2.72 ms; allocation moved from 1,770,019 bytes in 20,190 blocks to 702,971 bytes
in 18,842 blocks, with peak live memory moving from 23,902 to 12,894 bytes. Logical
work remained 400,447 units.

The updated evaluation-only component estimates are 18.6 µs for `exp(1)`, 301 µs
for `ln(2)`, 1.17 ms for `2^sqrt(2)`, and 1.12 ms for `sin(1)`. General power is no
longer an order-of-magnitude outlier; it and rational-point sine now form the next
timing tier. Further work should separately profile sqrt/log/multiply/exp within
general power and the trigonometric range-reduction/series path before choosing
between them.

## Factorial-bounded trigonometric series

The post-exponential profile showed rational-point sine at the same timing tier as
general power. Both unit-range sine and cosine still used the shared
`precision_bits / 3 + 16` term heuristic. Because their Taylor series alternate
with decreasing terms for `|x| <= 1`, each enclosure width is at most the first
omitted term. The shared sin/cos term count now chooses the smallest `N` satisfying
the stricter cosine condition `(2N + 2)! >= 2^precision_bits`; the following sine
term has an additional factorial divisor and therefore meets the same bound.

On 2026-07-12, the 10-sample `sin(1)` evaluation estimate moved from 1.38 ms in the
immediate pre-change run to 124 µs (about 91% lower). One-iteration allocation
moved from 170,503 bytes in 4,257 blocks to 37,039 bytes in 1,817 blocks. The full
approximate calculation moved from 2.72 ms to 1.89 ms; allocation moved from
702,971 bytes in 18,842 blocks to 569,507 bytes in 16,402 blocks. Peak composite
memory remained 12,894 bytes and logical work remained 400,447 units. General
power is again the largest measured component, so subsequent work should profile
its sqrt/log/multiply composition rather than further reducing standalone sine.

## General-power cumulative stages

The component harness now includes `sqrt(2)`, `sqrt(2)*ln(2)`, and
`exp(sqrt(2)*ln(2))` alongside direct `2^sqrt(2)`. Each untimed preflight verifies
the expected radical or general-symbolic exact classification and an available
certified enclosure. Corresponding one-iteration allocation cases are named
`approximate_sqrt_two`, `approximate_power_log_product`, and
`approximate_exp_power_log_product`.

On 2026-07-12, commit `2b7a1c2` with `rustc 1.97.0` recorded a single 10-sample
run at 263 µs, 855 µs, and 1.60 ms respectively, while direct general power
measured 1.91 ms. The same-run standalone `ln(2)` control was 482 µs; compared
with the roughly 593 µs increment from sqrt through the product, the remaining
approximately 111 µs includes multiplication, exact-expression construction, and
evaluation dispatch rather than establishing multiplication cost alone. All
existing controls slowed during this run, so absolute times are not used as a
before/after claim. The cumulative increments still identify logarithm and final
non-degenerate exponential evaluation as the primary next profiling targets.

One-iteration public-calculation allocation was 223,249 bytes in 10,130 blocks for
sqrt, 308,442 bytes in 13,627 blocks through the product, and 515,358 bytes in
14,240 blocks through exp. Sqrt remains a secondary allocation target despite its
smaller timing increment. Reproduce these rows with:

```sh
for case in \
  approximate_sqrt_two \
  approximate_power_log_product \
  approximate_exp_power_log_product
do
  CALCULATOR_ALLOCATION_ITERATIONS=1 \
    cargo run --profile bench -p calculator-core --features std \
      --example allocation_baseline -- "$case"
done
```

## Geometric-tail logarithm term bound

After range reduction, the logarithm series uses
`z = (x - 1) / (x + 1)` with `0 <= z <= 1/3`. Its positive omitted tail is at
most four times the first omitted term. The logarithm-specific term count now
chooses the smallest `N` satisfying
`4 / (3^(2N + 3) * (2N + 3)) <= 2^-precision_bits`, instead of reusing the
larger `precision_bits / 3 + 16` heuristic shared by unrelated series. The
integer-only selection preserves the directed enclosure and precision contract.

On 2026-07-12 with `rustc 1.97.0`, base commit `d0dafc8` and changed commit
`216fa08` measured the same 10-sample `ln(2)` invocation at 360.69 µs and
208.56 µs respectively (median estimates, about 42% lower). One-iteration
allocation moved from 56,005 bytes in 2,175 blocks to 39,949 bytes in 1,775
blocks. Direct `2^sqrt(2)` measured 1.22 ms after the change, and its allocation
moved from the comparable documented direct baseline of 481,190 bytes in 12,706
blocks to 465,134 bytes in 12,306 blocks. Reproduce the changed logarithm
measurements with:

```sh
cargo bench -p calculator-core --bench representative_paths --features std \
  -- approximate_components/log_two
CALCULATOR_ALLOCATION_ITERATIONS=1 \
  cargo run --profile bench -p calculator-core --features std \
    --example allocation_baseline -- approximate_log_two
```

The harness fixes this component group at 10 samples. The final non-degenerate
exponential stage remains the primary measured target for the next profiling
slice.

## Directed non-degenerate exponential endpoints

For an exact-point interval, exponential evaluation continues to build one Taylor
recurrence state and derives both directed bounds from it. For a non-degenerate
interval, monotonicity needs only the lower bound at the lower endpoint and the
upper bound at the upper endpoint. The endpoint path now canonicalizes only that
required Rational instead of constructing and discarding its opposite bound. The
same recurrence state representation is shared by paired and directed paths, and
regression tests require exact equality for positive, negative, zero, unit-range,
and range-reduced inputs.

On 2026-07-12 with `rustc 1.97.0`, base commit `1d5b44a` and changed commit
`e2119a7` measured the same 10-sample cumulative `exp(sqrt(2)*ln(2))` invocation
at 1.4096 ms and 1.2706 ms respectively (median estimates, about 9.9% lower).
Direct general power measured 1.0230 ms after the change; its immediate
pre-change timing run contained three outliers, so no timing percentage is
claimed for that row. At review-fix commit `e2c7448`, after removing endpoint
`BigInt` clones, one-iteration allocation moved from 499,302 to 486,262 bytes for
the cumulative exp stage and from 465,134 to 452,094 bytes for direct general
power. Exact-point `exp(1)` remained unchanged at 20,157 bytes in 762 blocks.
Peak live allocation increased by 96 bytes in both non-degenerate cases; this
remains below the earlier 12,894-byte composite baseline and is recorded
separately from the lower total allocation.
Reproduce the changed measurements with:

```sh
for case in exp_power_log_product general_power
do
  cargo bench -p calculator-core --bench representative_paths --features std \
    -- "approximate_components/$case"
done
for case in exp_one exp_power_log_product general_power
do
  CALCULATOR_ALLOCATION_ITERATIONS=1 \
    cargo run --profile bench -p calculator-core --features std \
      --example allocation_baseline -- "approximate_$case"
done
```

## Bounded square-factor trials and shared point sqrt scaling

Allocation stacks for `sqrt(2)` showed that simple-radical recognition continued
testing every odd square-factor candidate through 4095 after the remaining value
was already 2. Candidates are ascending, so once `factor^2` exceeds the remaining
positive integer, no later candidate can divide it as a square. Extraction now
stops at that proof boundary while retaining the existing trial ceiling and final
perfect-square check. Exact-point interval sqrt also converts and scales its dyadic
input once before deriving the same directed lower and upper roots.

On 2026-07-12 with `rustc 1.97.0`, base commit `712e9d5` and changed commit
`d50cda0` measured the same 10-sample `sqrt(2)` evaluation at 364.03 µs and
126.10 µs respectively (median estimates, about 65% lower). One-iteration
public-calculation allocation moved from 223,249 bytes in 10,130 blocks to 60,249
bytes in 1,979 blocks (about 73% fewer bytes and 80% fewer blocks). Peak live
allocation moved from 1,488 to 1,536 bytes; the total-allocation reduction and the
small peak tradeoff are tracked separately. Reproduce the changed measurements
with:

```sh
cargo bench -p calculator-core --bench representative_paths --features std \
  -- approximate_components/sqrt_two
CALCULATOR_ALLOCATION_ITERATIONS=1 \
  cargo run --profile bench -p calculator-core --features std \
    --example allocation_baseline -- approximate_sqrt_two
```

## Binary-scaled large exponential range

At base commit `1767131`, `exp(-10000)`, `e^(-10000)`, and `exp(10000)` returned
`computationLimit.PrecisionBits` through the CLI in about 0.6 seconds including
process startup. `exp(-4097)` hit the same fixed range cap, while `exp(-4096)`
entered the large Rational power path and did not finish within 30 seconds. Merely
raising the cap would construct `exp(10000)` before taking the negative reciprocal,
and the final absolute `2^-precision` grid would still round the lower endpoint to
zero.

At implementation commit `4b98a6c`, the large-range path is selected beyond the
direct Rational reduction range of 64. It computes a certified `ln(2)` interval at
guard precision, reduces `x = k*ln(2)+r`, evaluates only the bounded residual, and
adds `k` to the resulting `ExactDyadic` exponent. Both signs therefore retain an
O(precision) coefficient without a huge positive Rational power or zero underflow.
The existing scientific and decimal-scientific enclosure presentation carries the
bounded significand and exponent; no protocol change or fixed-decimal zero string
is used.

On 2026-07-12 with `rustc 1.97.0`, native evaluation measured 1.7182 ms for
`exp(-10000)` and 2.1465 ms for `exp(10000)` (10-sample medians). One public
calculation allocated 674,032 bytes / 4,574 blocks with a 16,031-byte peak for the
negative case, and 647,720 bytes / 4,487 blocks with a 17,082-byte peak for the
positive case. The unchanged `exp(1)` path remained 20,157 bytes / 762 blocks.
The logical-work equivalence runner reported 586 and 582 units respectively after
reserving the certified `ln(2)`, rational, and Taylor work before interval
evaluation; a 100-unit request returns a typed logical-work partial without an
enclosure. Binary exponent magnitude is capped at 1,000,000 before presentation so
the existing dyadic-to-decimal implementation cannot materialize an unbounded
power for inputs such as `exp(-1e9)`. A release Wasm/npm artifact of 783,704 bytes
(`sha256:6a44c38443eb21b11f7886d6bb1c7268855042ea505d02d7639bc902c7c0fca6`)
measured 14.66 ms and 13.57 ms per iteration over three iterations after one
warm-up, with retained heap changes of 20,568 and 9,344 bytes and serialized
result payloads of 1,794 and 1,786 bytes. The UI renders `exp(-10000)` as
`1.1355 × 10^-4343` and a five-digit directed enclosure without generating
thousands of zeroes. The Phase 1 CLI intentionally prints only the bounded exact
symbolic form; its successful exit verifies that numeric evaluation no longer
blocks exact-only presentation. Browser regression covers both public aliases and
their scientific/enclosure rendering; the existing worker-termination cancellation
boundary remains covered independently of calculation duration.

Reproduce the slice-specific measurements and states from the repository root:

```sh
cargo bench -p calculator-core --bench representative_paths --features std \
  -- approximate_components/exp_negative_10000
cargo bench -p calculator-core --bench representative_paths --features std \
  -- approximate_components/exp_positive_10000
for case in approximate_exp_negative_10000 approximate_exp_positive_10000 approximate_exp_one
do
  CALCULATOR_ALLOCATION_ITERATIONS=1 \
    cargo run --profile bench -p calculator-core --features std \
      --example allocation_baseline -- "$case"
done
cargo run --profile bench -p calculator-core --features std \
  --example logical_work_baseline
corepack pnpm --dir packages/calculator run build:wasm
CALCULATOR_BENCH_ITERATIONS=3 CALCULATOR_BENCH_WARMUP=1 \
  corepack pnpm --silent --dir packages/calculator run benchmark
cargo run -p calculator-cli -- 'exp(-10000)'
cargo run -p calculator-cli -- 'e^(-10000)'
corepack pnpm --dir examples/vanilla-web run test:e2e
```

The native component timing boundary is parsed `evaluate`; allocation and CLI use
the full public `calculate` boundary; the Wasm number includes facade conversion,
serialization, and presentation DTO construction. Benchmark JSON records the
serialized result payload bytes separately from retained heap. At base `1767131`,
all native/CLI/Wasm/UI public calculations failed in core with
`computationLimit.PrecisionBits`, so no result payload or UI numeric presentation
existed; the UI worker cancellation mechanism itself was already available and is
unchanged.

The direct Rational path remains in use through magnitude 64. Same-run normal
controls measured 33.596 µs for `exp(2)` and 44.637 µs for `exp(-2)`; one public
calculation allocated 20,913 bytes / 788 blocks and 21,838 bytes / 839 blocks.
`exp(1)` remained on its existing path at 20,157 bytes / 762 blocks.

## Sign-directed interval multiplication

At base commit `6e24224`, interval multiplication constructed all four endpoint
products even when both operand signs made the extrema monotone. The general-power
path `2^sqrt(2)` multiplies the positive intervals for `ln(2)` and `sqrt(2)`, so
two products, the candidate scan, and result clones were unnecessary. The signed
path selects the two extrema directly for every nonnegative/nonpositive sign
combination and retains the four-candidate definition for zero-crossing intervals.

One public calculation before/after allocated 289,094 / 288,190 bytes and
4,138 / 4,090 blocks for direct general power. The cumulative
`sqrt(2)*ln(2)` path moved from 129,386 / 5,076 to 128,266 / 5,010, and
`exp(sqrt(2)*ln(2))` moved from 323,262 / 5,672 to 322,142 / 5,606. Peak live
memory stayed within 24 bytes of the corresponding baseline, as expected because
the removed endpoint products were short-lived. Ten-sample timing medians on this
shared host varied from 567 to 645 µs around the change; a subsequent 20-sample
warm run measured 580 µs for general power and detected no significant change for
the cumulative log product. Allocation counts, exact endpoint regression cases,
and enclosure equality are therefore the evidence for this slice; the timing
samples are not treated as a speedup claim.

These measurements were taken at implementation commit `e77540e` on 2026-07-12
with `rustc 1.97.0`. Reproduce the one-iteration allocation comparisons and the
focused Criterion samples with:

```sh
for case in approximate_general_power approximate_power_log_product \
  approximate_exp_power_log_product
do
  CALCULATOR_ALLOCATION_ITERATIONS=1 \
    cargo run --profile bench -p calculator-core --features std \
      --example allocation_baseline -- "$case"
done
cargo bench -p calculator-core --bench representative_paths --features std \
  -- approximate_components/general_power --sample-size 20
cargo bench -p calculator-core --bench representative_paths --features std \
  -- approximate_components/power_log_product --sample-size 20
```

## Structural dyadic-to-rational conversion

At base commit `3c04716`, every ExactDyadic-to-Rational boundary called the
general Rational constructor. For a canonical dyadic with a negative exponent,
the coefficient is odd and the denominator is a power of two, so coprimality is
already structural; for a nonnegative exponent the result is an integer. The
specialized conversion now constructs those forms directly. It still removes
canceling factors of two from noncanonical negative-exponent inputs before direct
construction, and regression tests compare positive, negative, zero, integral,
fractional, and noncanonical cases with the general constructor.

At implementation commit `a3ecd5b` on 2026-07-12 with `rustc 1.97.0`, one public
calculation allocated the following before/after totals:

| Case | Allocated bytes |
| --- | ---: |
| `exp(1)` | 20,157 / 19,821 |
| `ln(2)` | 39,949 / 39,661 |
| `2^sqrt(2)` | 288,190 / 287,582 |
| `sin(1)` | 37,039 / 36,063 |
| `sqrt(2)` | 60,249 / 59,585 |
| `sqrt(2)*ln(2)` | 128,266 / 127,738 |
| `exp(sqrt(2)*ln(2))` | 322,142 / 321,534 |

The shared host slowed all component timings together during the focused run;
subsequent 20-sample runs detected no significant change for `exp(1)` or general
power. This slice therefore claims the deterministic allocation reduction, not a
timing speedup. Logical-work boundaries remained 231, 401216, 400447, 586, 582,
400234, and 932 for the stored representative cases. A three-iteration/one-warmup
Wasm/npm snapshot used artifact
`a4782844b7a2adc22aa1a6356d07154f607aa9bd549849a7f50003ecf928867c`
(784,565 bytes); the approximate case measured 8.96 ms/iteration with an
unchanged 1,812-byte serialized payload.

Reproduce the deterministic and boundary measurements with:

```sh
for case in approximate_exp_one approximate_log_two approximate_general_power \
  approximate_sin_one approximate_sqrt_two approximate_power_log_product \
  approximate_exp_power_log_product
do
  CALCULATOR_ALLOCATION_ITERATIONS=1 \
    cargo run --profile bench -p calculator-core --features std \
      --example allocation_baseline -- "$case"
done
cargo run --profile bench -p calculator-core --features std \
  --example logical_work_baseline
corepack pnpm --dir packages/calculator run build:wasm
CALCULATOR_BENCH_ITERATIONS=3 CALCULATOR_BENCH_WARMUP=1 \
  corepack pnpm --silent --dir packages/calculator run benchmark
```

## Rational integer-operand addition

On 2026-07-15 at base commit `03c76bf`, canonical `Rational::add` still used
two cross-products, a denominator product, GCD normalization, and exact division
when either operand was an integer. For canonical `a/b` and integer `n`, the
result `(a+n*b)/b` is already reduced because
`gcd(a+n*b,b)=gcd(a,b)=1`; integer/integer and zero identities are simpler
instances. The implementation now constructs these canonical forms directly,
while addition of two non-integer operands retains the general constructor.

Deterministic one-calculation allocation changed as follows; peak live allocation
was unchanged in every row:

| Case | Before | After |
| --- | ---: | ---: |
| 256-term integer sum | 284,027 bytes / 10,858 blocks | 277,843 / 10,085 |
| fraction + fraction control | 13,270 / 612 | 13,134 / 599 |
| `7 + -5/6` | 9,707 / 469 | 9,603 / 456 |
| `-5/6 - 7` | 10,056 / 491 | 9,936 / 476 |
| symbolic control | 54,852 / 1,893 | 54,588 / 1,860 |
| algebraic control | 100,723 / 3,942 | 100,491 / 3,913 |
| approximate control | 172,283 / 2,737 | unchanged |

Logical work remained 932 units for the wide sum; exact rational, symbolic,
algebraic, and approximate controls remained 231, 401,216, 400,234, and 400,447.
The new 16/64/128/256 scaling and parse/evaluate/present stage groups verify exact
outputs before timing. Their same-host snapshots were:

| Terms | Base | Candidate |
| ---: | ---: | ---: |
| 16 | `[90.62, 124.50]` us | `[64.04, 71.75]` us |
| 64 | `[319.18, 332.39]` us | `[201.97, 230.34]` us |
| 128 | `[588.15, 617.37]` us | `[331.43, 397.15]` us |
| 256 | `[1.199, 1.291]` ms | `[629.13, 710.31]` us |

At 256 terms the base parse/evaluate/present ranges were respectively
`[137.28,146.31]` us, `[966.56,1040.8]` us, and `[27.14,28.04]` us. The
candidate snapshots were `[67.81,71.57]` us, `[588.82,673.30]` us, and
`[13.42,17.39]` us. Evaluation remains the dominant measured stage; parse and
presentation are controls unaffected by `Rational::add`, so their movement also
shows that host load materially affected these non-alternating snapshots.
Same-host native composite samples were likewise load-sensitive: the base
256 composite was `[739.67, 803.80]` us and the candidate was
`[630.57, 808.84]` us, so this slice claims the deterministic allocation result,
not a native timing ratio. A directly alternating Wasm/npm smoke moved from
3.096 ms to 2.646 ms per iteration with unchanged 1,728-byte payload. The base
artifact was `b0e5f687e15699c4e600128f2214d6d0f7f00916e3d6713d92f0a295646d22d0`
(821,780 bytes); the candidate was
`b257482004950ab458921e76a71078846752e74eb8f2d34265bce4bb96aa3cc9`
(821,966 bytes). The cold three-iteration sample is boundary verification, not a
stable speedup claim.

Reproduce with allocation cases `wide_add_256`, `exact_rational`,
`exact_mixed_add`, `exact_mixed_subtract`, `exact_symbolic`, `algebraic`, and
`approximate`; Criterion filters `large_expression`,
`large_expression_scaling`, and `large_expression_stages`; the logical-work
runner; and `CALCULATOR_BENCH_CASE=wide_add_256` in the Wasm/npm harness.
The final repository gate passed dependency audits, formatting, native and Wasm
clippy, no-default-features and workspace/doc tests, Node Wasm tests, generated
DTO/protocol/no-float/deny checks, package checks and size budget, example build,
external rational oracle, browser E2E, and workspace documentation.

## Additive literal lowering accumulator

On 2026-07-15 at base commit `a6699bf`, additive source lowering flattened the
syntax tree but first interned every numeric literal as a Rational and expression
node. `add_many_linear` immediately folded those values into one constant, leaving
the per-literal nodes unreachable but retained. Numeric literals, including unary
`+`/`-` wrappers and subtraction signs, are now parsed and accumulated before DAG
materialization. Non-literal and domain-sensitive terms retain the existing DAG
path and source traversal/error order.

The accumulator reserves structural logical work before every Rational addition.
Integer addition uses the maximum signed-limb width; mixed and fractional paths
cover products, numerator addition, GCD normalization and exact divisions. Limit
failure is latched: later literals are parsed for error precedence but retained
unfolded, and exact simplification does not resume. The 256-term sum now succeeds
at 261 units; 260 returns a logical-work Partial retaining exact `32896`, compared
with 932 units at the base. The reduction is the removed hash/intern/node and
downstream normalization work, not an uncharged O(n) fold.

Deterministic one-calculation allocation changed as follows:

| Case | Before | After |
| --- | ---: | ---: |
| 256-term integer sum | 277,843 bytes / 10,085 blocks | 108,157 / 5,449 |
| peak live wide sum | 109,443 bytes / 1,891 blocks | 38,104 / 1,023 |
| fraction + fraction control | 13,134 / 599 | unchanged |
| `7 + -5/6` | 9,603 / 456 | 9,291 / 442 |
| `-5/6 - 7` | 9,936 / 476 | 9,374 / 454 |
| symbolic control | 54,588 / 1,860 | unchanged |
| algebraic control | 100,491 / 3,913 | 100,443 / 3,907 |
| approximate control | 172,283 / 2,737 | unchanged |

Logical-work controls remained 231, 401,216, and 400,447 for exact rational,
symbolic, and approximate calculations; the algebraic source moved from 400,234
to 400,229 by eliminating its literal-node work. Native snapshots were:

| Terms | Base | Candidate |
| ---: | ---: | ---: |
| 16 | `[64.04,71.75]` us | `[46.15,53.99]` us |
| 64 | `[201.97,230.34]` us | `[83.12,89.82]` us |
| 128 | `[331.43,397.15]` us | `[123.65,137.87]` us |
| 256 | `[629.13,710.31]` us | `[302.13,333.93]` us |

The end-to-end 256 sample moved from `[630.57,808.84]` us to
`[232.03,268.43]` us. Stage snapshots moved from `[67.81,71.57]` us parse,
`[588.82,673.30]` us evaluate, and `[13.42,17.39]` us present to
`[47.70,56.82]`, `[159.74,163.18]`, and `[24.16,30.16]` us respectively.
Only evaluation contains the changed lowering path; parse/present movement is a
host-load control and is not attributed to this change.

The final Wasm/npm artifact is
`85b810f369e8ec7f6abfbfbe5693b151a9ef99a7f67d122d5db3f5865806987b`
(823,356 bytes). A directly comparable three-iteration/one-warmup wide-add smoke
moved from 2.646 ms to 1.739 ms per iteration with unchanged 1,728-byte payload.
This cold sample verifies the boundary and is not a stable throughput guarantee.

`max_expression_nodes` counts the materialized DAG, so a literal-only `1+2` now
fits at one node instead of failing due to unreachable intermediates. This is an
intentional supported-range expansion. Input bytes and source AST node/depth
limits still run before lowering, while a nonliteral DAG still enforces the
expression-node limit. Reproduce with the existing wide allocation, logical-work,
scaling/stage, and Wasm/npm harnesses.

## Multiplicative literal lowering accumulator

At base commit `ae637b6`, a left-associated numeric product lowered and
normalized every binary multiplication independently. Each literal and growing
intermediate coefficient was retained in the DAG even though the next product
immediately absorbed it. Multiplication source lowering now flattens only
`BinaryOperator::Multiply`, parses numeric literals including unary signs from
left to right, and materializes their accumulated Rational coefficient once.
Divide, percent, power, functions, constants, and other nonliteral factors retain
their existing lowering and domain behavior. In particular, zero does not remove
an undefined or unproved-defined factor.

Every accumulated product reserves structural work for the numerator and
denominator products and conservative GCD/exact-division normalization before
mutation. Failure is latched, later literals are still parsed but retained, and
folding does not resume. For `1*2*...*128`, deterministic one-calculation values
changed as follows:

| Metric | Base | Candidate |
| --- | ---: | ---: |
| allocated | 327,178 bytes / 11,050 blocks | 86,818 / 3,212 |
| peak live | 79,411 bytes / 1,478 blocks | 18,904 / 511 |
| logical work | 38,176 units | 6,377 units |

The 6,376-unit request returns a typed logical-work Partial with the same exact
128! expression, so the range expansion is removed intermediate work rather than
an uncharged fold. With the candidate native build, Criterion measured
`wide_multiply_128` at `[745.72,1004.4]` us. Scaling snapshots were
`[116.25,133.23]` us for 16 terms, `[192.02,207.81]` for 32,
`[380.86,449.28]` for 64, and `[1091.7,1239.7]` for 128; growing BigInt
multiplication remains the dominant scaling cost.

Wasm/npm benchmark definition v19 adds the same public case. The comparable
three-iteration/one-warmup smoke moved from 14.664 ms at base artifact
`85b810f369e8ec7f6abfbfbe5693b151a9ef99a7f67d122d5db3f5865806987b`
(823,356 bytes) to 6.419 ms at candidate artifact
`4ad06055917fd804f2b3cb89a6ee02a9c8e4fd0937fef9bde5c0f70f3ae93c04`
(824,961 bytes), with the exact payload unchanged at 2,162 bytes. This cold
sample verifies the Wasm/public facade path, not stable throughput.

Controls remained at 261 logical-work units and 108,157 bytes / 5,449 blocks for
the 256-term sum, and 400,447 units for the approximate composite. Exact-rational
and approximate allocation improved slightly to 12,966 / 567 and 172,272 / 2,726
because their source products use the same general lowering path. Reproduce with
allocation/logical-work case `wide_multiply_128`, Criterion groups
`large_product`/`large_product_scaling`, and the v19 Wasm/npm case.

## Canonical Rational integer multiplication

At base commit `772e594`, canonical Rational multiplication still formed a
denominator product and ran the generic GCD/exact-division constructor when both
operands were integers. The integer/integer path now multiplies only the signed
numerators and constructs the canonical integer directly. Zero and integer one
use their canonical identity paths. Mixed integer/fraction and fraction/fraction
products remain on the generic constructor because cross-cancellation can change
both numerator and denominator.

On 2026-07-15 with `rustc 1.97.0`, deterministic one-calculation results for
`1*2*...*128` changed as follows:

| Metric | Base | Candidate |
| --- | ---: | ---: |
| allocated | 86,818 bytes / 3,212 blocks | 80,930 / 3,664 |
| peak live | 18,904 bytes / 511 blocks | 18,904 / 511 |
| logical work | 6,377 units | 1,339 units |

Allocated bytes fell 6.8% and peak live allocation was unchanged. Allocation
count rose 14.1%; this is retained as an explicit follow-up signal rather than
treated as an improvement. The removed normalization dominates elapsed time:
same-host Criterion measured `[183.60,212.70]` us versus the base
`[745.72,1004.4]` us. The 1,338-unit request returns the same typed logical-work
Partial boundary, so the supported-range gain comes from removed denominator and
normalization operations rather than uncharged multiplication.

The final Wasm/npm artifact is
`852fcfdde5d85a45d1de8e17200b412ee92f21e8cd5f30c424b14e95bb45b28b`
(825,280 bytes). The unchanged v19 public benchmark, using three iterations
after one warmup, moved from 6.419 ms to 1.379 ms per iteration. Serialized
payload stayed at 2,162 bytes. This short run verifies the public boundary and
is not a stable throughput guarantee.

Wide-add and approximate controls stayed at 108,157 bytes / 5,449 blocks and
261 work units, and 172,272 / 2,726 and 400,447 respectively. Exact-rational
allocation improved from 12,966 / 567 to 12,758 / 551 while its work stayed at
231 units. Candidate exact-symbolic and algebraic controls were 52,704 / 1,612
and 401,216 units, and 99,819 / 3,801 and 400,229 units respectively. Reproduce with the
`wide_multiply_128` allocation/logical-work cases, Criterion
`large_product/wide_multiply_128`, and the v19 Wasm/npm case.

## Allocation-free Rational integer predicates

At base commit `6e40832`, `Rational::is_integer` compared its canonical positive
denominator with a newly constructed `BigInt::one()`. Arithmetic dispatch calls
this predicate repeatedly; the Issue 87 integer product path therefore exposed
the comparison temporary as 1,016 avoidable blocks across the public calculation.
The predicate now asks the stored BigInt whether it is one, without materializing
another arbitrary-precision integer.

On 2026-07-15 with `rustc 1.97.0`, deterministic allocation changed as follows;
peak live allocation and logical work were unchanged for every case:

| Case | Base bytes / blocks | Candidate bytes / blocks | Peak / work units |
| --- | ---: | ---: | ---: |
| `wide_multiply_128` | 80,930 / 3,664 | 72,802 / 2,648 | 18,904 / 511; 1,339 |
| `wide_add_256` | 108,157 / 5,449 | 99,965 / 4,425 | 38,104 / 1,023; 261 |
| exact rational | 12,758 / 551 | 12,590 / 530 | 2,615 / 51; 231 |
| exact symbolic | 52,704 / 1,612 | 51,984 / 1,522 | 8,853 / 98; 401,216 |
| algebraic | 99,819 / 3,801 | 99,251 / 3,730 | 4,884 / 104; 400,229 |
| approximate composite | 172,272 / 2,726 | 172,264 / 2,725 | 9,886 / 57; 400,447 |

Same-host Criterion measured the wide product at `[165.64,205.12]` us, versus
`[183.60,212.70]` us at base. The final Wasm/npm artifact is
`091ea180ff8623ceba34aeabfd5cd810daeb062d396a2a8ff9e455bde7a439b9`
(825,688 bytes). The unchanged v19 three-iteration/one-warmup smoke moved from
1.379 to 1.304 ms per iteration; payload stayed at 2,162 bytes. These short
timing samples corroborate the deterministic allocation result rather than form
a throughput guarantee. Growing BigInt multiplication and the unchanged public
parse/presentation boundary remain the principal wide-product work.

## Structural canonical Rational negation

At base commit `6dff484`, `Rational::negate` sent an already canonical numerator
and positive denominator back through the general constructor. Negating the
numerator cannot change their GCD or denominator sign, so the implementation now
negates only the owned numerator and clones the canonical denominator.

With one public calculation on 2026-07-15, `-5/6 - 7` moved from 9,128 bytes /
404 blocks to 9,064 / 396. The related `7 + -5/6` case, whose negative literal
uses the same primitive, moved from 9,045 / 392 to 9,013 / 388. Both retained
the same 2,047-byte / 43-block peak. Exact-rational, symbolic, algebraic,
approximate, wide-add, and wide-product controls were unchanged from the Issue
88 candidate. Logical-work values were unchanged because this slice retains the
existing conservative arithmetic reservation.

Same-host Criterion moved from `[23.304,29.411]` us at base to
`[17.212,18.618]` us at candidate. Wasm/npm benchmark definition v20 was applied
to both artifacts for the corresponding public case; ten iterations after two
warmups measured 0.719 ms at base and 0.726 ms at candidate, so the Wasm timing
difference is treated as noise. Payload stayed at 1,805 bytes. The final artifact is
`8520a6f4bb9c2a7ba5de32d238659377f3f430e919e9ac4bffa96ada6f587fcf`
(825,518 bytes). Timing is a diagnostic snapshot; deterministic allocation is
the primary claim; no Wasm speedup is claimed.

## Structural BigInt unit comparisons

At base commit `83c4c3c`, Rational display, finite-decimal residue checks,
simple-radical classification, and polynomial candidate checks constructed zero
or one BigInts solely for comparison. Structural `is_zero`/`is_one` predicates
remove those temporaries without changing classification or work accounting.

Deterministic one-calculation allocation moved from 12,590 / 530 to 12,582 /
529 for exact rational, 99,251 / 3,730 to 99,235 / 3,728 for algebraic,
72,802 / 2,648 to 72,794 / 2,647 for `wide_multiply_128`, and 172,264 / 2,725
to 172,232 / 2,721 for the approximate composite. Peak allocation was unchanged.
The final Wasm artifact is
`48096c109f8483bf0b6e9f190f30ded5b4d5ef6f23f9166cc43f5d091754a077`
(825,509 bytes). This slice claims only the deterministic allocation reduction.

## Raw directed dyadic arctangent endpoints

At base commit `defe4a4`, the public certified `atan` path canonicalized each
alternating-series fraction with a GCD and exact division, canonicalized
`pi/2-atan(1/x)` again for reciprocal-domain arguments, and immediately converted
that Rational result back to a directed dyadic endpoint. The raw endpoint path
retains the same recurrence or hybrid binary split and selects the same
sum/adjacent fraction by parity. Unit fractions are rounded directly. Reciprocal
fractions are combined exactly as
`(pi_num*atan_den - 2*pi_den*atan_num)/(2*pi_den*atan_den)` and rounded once.
Machin pi and inverse-trigonometric internal Rational consumers remain unchanged.

On 2026-07-15 with `rustc 1.97.0`, deterministic one-calculation allocation
changed as follows:

| Case | Base bytes / blocks | Candidate bytes / blocks | Base peak | Candidate peak |
| --- | ---: | ---: | ---: | ---: |
| `atan(1/2)` | 20,774 / 760 | 19,438 / 735 | 2,167 / 37 | 2,143 / 37 |
| `atan(2)` | 45,586 / 965 | 37,010 / 889 | 4,368 / 41 | 4,320 / 45 |
| `atan(2+sin(1))` | 525,594 / 2,061 | 367,762 / 1,884 | 26,696 / 47 | 17,384 / 44 |

The target's allocated bytes fell about 30.0% and peak live bytes about 34.9%.
Same-host twenty-sample Criterion ranges moved from 9.184--9.537 ms to
0.635--0.697 ms for the non-degenerate target, from 474--522 us to 304--315 us
for `atan(2)`, and from 34.6--44.0 us to 28.0--36.4 us for `atan(1/2)`.
Logical work remained 31, 5, and 200,225 units respectively.

The focused Wasm/npm comparison used base artifact
`aa0d182eb89ea17f68776af011f0ce5669b992f7349b5073ddb08ad0e2fa5987`
(820,570 bytes) and candidate artifact
`825e038e35d3e6ea5665dbb71a0b9ba9421385c94e0f546a55d7859486aac2cb`
(821,645 bytes), both below the package budget. Three iterations after one warmup
moved `atan(2+sin(1))` from 60.40 ms to 3.49 ms per iteration. Serialized payload
size stayed 1,776 bytes; retained JS heap was 3,688 and 3,496 bytes. These small
Wasm samples establish boundary integration and corroborate the native profile,
not a stable cross-host latency guarantee.

Raw-vs-canonical oracle tests cover zero, signs, unit boundaries, reciprocal
arguments, the nonunit binary-split path, and 0/1/32/128-bit precisions. Separate
tests preserve exact input ordering before work and final dyadic ordering for
unit, mixed, and reciprocal non-degenerate intervals at coarse precision.
Transformed asin/acos allocation controls remain on their Rational atan helper
paths. Reproduce with:

```sh
for case in approximate_atan_half approximate_atan_two approximate_atan_non_degenerate
do
  CALCULATOR_ALLOCATION_ITERATIONS=1 \
    cargo run --profile bench -p calculator-core --features std \
      --example allocation_baseline -- "$case"
done
cargo bench -p calculator-core --bench representative_paths --features std \
  -- 'approximate_components/atan_' --sample-size 20
cargo run --profile bench -p calculator-core --features std \
  --example logical_work_baseline
corepack pnpm --dir packages/calculator run build:wasm
CALCULATOR_BENCH_CASE=atan_non_degenerate CALCULATOR_BENCH_ITERATIONS=3 \
  CALCULATOR_BENCH_WARMUP=1 \
  corepack pnpm --silent --dir packages/calculator run benchmark
```

## Raw directed dyadic logarithm endpoints

At base commit `1f659b0`, the public certified `log` path canonicalized every
reduced-series fraction, canonicalized and scaled a separate `ln(2)` fraction
after binary range reduction, added the canonical Rationals, and immediately
converted the result to a directed dyadic endpoint. The raw endpoint layer keeps
the recurrence or hybrid binary-split fractions unreduced. For reduced `a/b`,
selected `ln(2)` bound `c/d`, and signed binary exponent `k`, it composes
`(a*d+k*c*b)/(b*d)` exactly and performs one final directed rounding. Internal
Rational log consumers used by exponential planning remain unchanged.

On 2026-07-15 with `rustc 1.97.0`, deterministic default-request allocation was:

| Case | Base bytes / blocks | Candidate bytes / blocks | Base peak | Candidate peak |
| --- | ---: | ---: | ---: | ---: |
| `ln(2)` | 20,165 / 724 | 18,533 / 680 | 1,638 / 37 | 1,598 / 29 |
| `ln(2+sin(1))` | 278,420 / 1,708 | 204,284 / 1,549 | 16,982 / 43 | 12,798 / 44 |
| large positive log | 170,153 / 1,415 | 104,609 / 1,212 | 16,090 / 30 | 12,146 / 27 |

The non-degenerate target's bytes fell about 26.6% and peak bytes about 24.6%; the
large-log totals fell about 38.5% and peak bytes about 24.5%. Same-host
twenty-sample approximate-component midpoints moved from about 3.55 ms to 225 us
for the non-degenerate log and from 2.63 ms to 85.5 us for the large log.
Logical work remained 200,225 and 21 units.

Because unreduced composition forms `b*d`, the same comparison was repeated with
a 200-significant-digit public request. Non-degenerate allocation moved from
287,418 / 1,818 (peak 16,982 / 43) to 213,282 / 1,659 (peak 12,798 / 44), and the
large log from 178,527 / 1,525 (peak 16,090 / 30) to 112,983 / 1,322 (peak
12,146 / 27). Ten-sample public `calculate` midpoints moved from 3.99 ms to
278 us and from 2.97 ms to 121 us respectively. Thus the arbitrary signed binary
composition does not trade default savings for high-precision peak or timing
regression in the measured representative paths.

The Wasm/npm smoke used base artifact
`825e038e35d3e6ea5665dbb71a0b9ba9421385c94e0f546a55d7859486aac2cb`
(821,645 bytes) and candidate artifact
`b0e5f687e15699c4e600128f2214d6d0f7f00916e3d6713d92f0a295646d22d0`
(821,780 bytes). Three iterations after one warmup moved non-degenerate log from
20.17 ms to 1.62 ms and large log from 17.67 ms to 0.884 ms per iteration.
Payloads stayed 1,772 and 1,834 bytes; candidate retained JS heap was lower. These
small Wasm samples verify boundary integration and corroborate the native profile,
not a cross-host latency guarantee.

Raw-vs-canonical endpoint oracles cover 0/1/32/128-bit precisions, values below
and above one, positive and negative binary exponents, a large exponent, exact
and non-degenerate intervals, and arbitrary-numerator binary splitting. Public
tests preserve nonpositive domain precedence, the exact-one fast path, overflow,
input ordering, and final dyadic ordering. Reproduce default or high-precision
measurements with:

```sh
for case in approximate_log_two approximate_log_non_degenerate \
  approximate_log_large_positive
do
  CALCULATOR_ALLOCATION_ITERATIONS=1 CALCULATOR_SIGNIFICANT_DIGITS=200 \
    cargo run --profile bench -p calculator-core --features std \
      --example allocation_baseline -- "$case"
done
CALCULATOR_SIGNIFICANT_DIGITS=200 \
  cargo bench -p calculator-core --bench representative_paths --features std \
    -- 'calculate/log_(non_degenerate|large_positive)' --sample-size 10
cargo run --profile bench -p calculator-core --features std \
  --example logical_work_baseline
corepack pnpm --dir packages/calculator run build:wasm
CALCULATOR_BENCH_CASE=log_non_degenerate CALCULATOR_BENCH_ITERATIONS=3 \
  CALCULATOR_BENCH_WARMUP=1 \
  corepack pnpm --silent --dir packages/calculator run benchmark
```

## Shared half-pi periodic trigonometric scans

At base commit `f83ed66`, non-degenerate periodic `sin`, `cos`, and `tan`
recomputed the same Machin-formula π enclosure for the full-period check, scan
limit, and every half-π extremum or pole candidate. The retained path constructs
one certified half-π pair per public interval call and scales that pair by each
signed primitive index. Candidate containment, uncertain fallback, scan limits,
endpoint evaluation, and directed enclosures are unchanged.

On 2026-07-13 with `rustc 1.97.0`, deterministic one-calculation allocation
changed as follows:

| Case | Bytes | Blocks | Peak bytes / blocks |
| --- | ---: | ---: | ---: |
| `sin(100+sin(1))` | 1,823,111 / 477,423 | 25,150 / 5,028 | 9,507 / 48 → 10,211 / 52 |
| `tan(100+sin(1)/100)` | 1,908,536 / 619,224 | 25,069 / 5,805 | 18,218 / 64 → 18,922 / 68 |

This removes about 74%/68% of allocated bytes and 80%/77% of allocation blocks.
Keeping the shared half-π pair live adds 704 peak bytes and four peak blocks; the
tradeoff is recorded rather than hidden. Logical work remained exactly 400,353
and 400,517 units. Ten instrumented calculations alternated on the same host in
2.49/2.26/2.30 seconds for base and 0.92/0.86/0.80 seconds for the candidate.
Saved-baseline native Criterion samples had wide, overlapping confidence
intervals and detected no change, so they are not used as a native timing claim.

Wasm/npm benchmark definition v18 exercises the same public facade paths. With
three iterations and one warmup, base artifact
`3bd844de7f84cd4f357c7b916fb3057700ac421744a74fa85dc5828c46fc5b1e`
(818,132 bytes) measured 302.1 ms/iteration for sine and 265.2 ms/iteration for
tangent. Candidate artifact
`c21aeb12bb3ab137b449d48229892ac20831d897898e42d13f76ec4e09c6e032`
(818,346 bytes) measured 45.1 and 48.3 ms/iteration. Payloads remained 1,784 and
1,802 bytes, and both artifacts stayed below the 860,000-byte budget.

The final CI-equivalent gate rebuilt the package and example artifacts at that
same candidate SHA and 818,346-byte size, passed the package-size budget, and
completed the native, no-default, Wasm, package, example-build, external-oracle,
browser-E2E, dependency-policy, generated-contract, and documentation suites.

Reproduce with:

```sh
for case in approximate_sin_periodic_non_degenerate \
  approximate_tan_periodic_non_degenerate
do
  CALCULATOR_ALLOCATION_ITERATIONS=1 \
    cargo run --profile bench -p calculator-core --features std \
      --example allocation_baseline -- "$case"
done
cargo bench -p calculator-core --bench representative_paths --features std \
  -- approximate_components/sin_periodic_non_degenerate --sample-size 10
cargo run --profile bench -p calculator-core --features std \
  --example logical_work_baseline
corepack pnpm --dir packages/calculator run build:wasm
CALCULATOR_BENCH_CASE=sin_periodic_non_degenerate \
  CALCULATOR_BENCH_ITERATIONS=3 CALCULATOR_BENCH_WARMUP=1 \
  corepack pnpm --silent --dir packages/calculator run benchmark
```

## First-child interval folds

At base commit `bc9776e`, every n-ary certified Add and Multiply evaluation built
an exact zero or one interval and combined it with the first real child. The
retained path evaluates the first child as the accumulator and folds only the
remaining children. Empty internal lists still return their monoid identity and
actual children retain left-to-right evaluation and error precedence. This removes
one directed dyadic operation per composite node; in particular, addition no
longer aligns a nonzero first child with exponent-zero identity storage that is
discarded by normalization.

On 2026-07-14 with `rustc 1.97.0`, deterministic one-calculation allocation was:

| Case | Before | After | Peak bytes / blocks |
| --- | ---: | ---: | ---: |
| `sqrt(2)*ln(2)` | 90,818 bytes / 3,422 blocks | 82,394 / 3,152 | 3,967 / 74 unchanged |
| `exp(sqrt(2)*ln(2))` | 185,470 / 3,588 | 177,046 / 3,318 | 8,620 / 37 unchanged |
| representative `approximate` | 176,139 / 2,874 | 175,931 / 2,861 | 9,886 / 57 unchanged |

The focused product removes about 9.3% of allocated bytes and 7.9% of allocation
blocks. Logical work remained 400,447 units for the stored representative
approximate case. Tests compare the old identity-seeded and new folds exactly at
32/128/512-bit precision for mixed-sign n-ary sums and products, and directly
cover defensive empty and singleton lists.

Separate ten-sample Criterion snapshots had midpoint estimates of 245.11 µs base
and 195.56 µs candidate for the product, and 343.90 µs base and 258.96 µs
candidate for the enclosing exponential. The samples were built in separate
targets and showed host variance, so the accepted performance claim is the
deterministic allocation reduction rather than those timing ratios.

Wasm/npm benchmark definition v18 returned the unchanged 1,812-byte payload for
the representative approximate case. Five iterations after one warmup measured
3.353 ms/iteration with base artifact
`c21aeb12bb3ab137b449d48229892ac20831d897898e42d13f76ec4e09c6e032`
(818,346 bytes), and 2.445 ms/iteration with candidate artifact
`fff690b89e67b74df69242f3d4f327b26ea428835a4c852e571fe8e8a22fca6a`
(818,558 bytes). Both remain below the 860,000-byte budget; this snapshot verifies
the public Wasm/facade path and is not used as a stable timing guarantee.
The final CI-equivalent gate rebuilt the package and example at the same candidate
SHA and size, and passed native/no-default/Wasm tests, generated/protocol/no-float
checks, dependency policy, TypeScript/package/example builds, the exact oracle,
package-size budget, browser E2E, and workspace documentation.

Reproduce with:

```sh
for case in approximate_power_log_product approximate_exp_power_log_product approximate
 do
  CALCULATOR_ALLOCATION_ITERATIONS=1 \
    cargo run -p calculator-core --features std \
      --example allocation_baseline -- "$case"
done
cargo bench -p calculator-core --bench representative_paths --features std \
  -- power_log_product --sample-size 10
cargo run --profile bench -p calculator-core --features std \
  --example logical_work_baseline
corepack pnpm --dir packages/calculator run build:wasm
CALCULATOR_BENCH_CASE=approximate CALCULATOR_BENCH_ITERATIONS=5 \
  CALCULATOR_BENCH_WARMUP=1 \
  corepack pnpm --silent --dir packages/calculator run benchmark
```

## Raw unit-arcsine dyadic rounding

At base commit `7f695a3`, the positive-series `|x| <= 1/2` arcsine recurrence
canonicalized its lower and upper-tail common-denominator fractions as Rational
values, including large GCD and exact division, immediately before directed
conversion back to dyadic endpoints. The retained path exposes the raw fraction
parts and applies the same exact floor/ceil directly. Negative values reverse and
negate the positive directed bounds. Non-degenerate endpoints are rounded
independently, and their final ordering validation compares the resulting dyadics
instead of reconstructing a rational cross product. The transformed `|x|>1/2`
sqrt/atan/pi algorithm itself is unchanged.

On 2026-07-15 with `rustc 1.97.0`, deterministic one-calculation allocation was:

| Case | Before | After | Peak before / after |
| --- | ---: | ---: | ---: |
| `asin(1/3)` | 426,740 bytes / 1,031 blocks | 384,596 / 1,002 | 17,079 / 33 → 11,703 / 35 |
| `asin(sin(1)/3)` | 850,867 / 1,303 | 756,483 / 1,275 | 32,560 / 40 → 21,664 / 42 |
| transformed `asin((2+sin(1))/3)` | 1,155,212 / 4,023 | 943,036 / 3,599 | 60,757 / 85 → 40,885 / 79 |
| `acos(1/3)` control | 519,394 / 1,348 | unchanged | 23,079 / 45 unchanged |

The first two paths remove about 9.9%/11.1% of allocated bytes. Their two extra
peak blocks hold the directly rounded endpoints while peak bytes fall by 31%/33%.
The transformed path does not use the raw positive-series fraction; its reduction
comes from validating final directed dyadics instead of comparing the two large
transformed Rational bounds. Logical work remained 31, 200,216, 200,447, and 31
units respectively.

Separate ten-sample Criterion midpoint snapshots moved from 1.2193 ms to
243.36 µs for `asin(1/3)` and from 3.7042 ms to 621.78 µs for the non-degenerate
unit-series case. Wasm/npm benchmark definition v18 measured three iterations
after one warmup for `asin_third` / `asin_non_degenerate_unit`. Base artifact
`fff690b89e67b74df69242f3d4f327b26ea428835a4c852e571fe8e8a22fca6a`
(818,558 bytes) measured 8.286 / 24.804 ms per iteration; candidate artifact
`aa0d182eb89ea17f68776af011f0ce5669b992f7349b5073ddb08ad0e2fa5987`
(820,570 bytes) measured 1.102 / 2.610 ms. Payloads remained 1,772 / 1,786 bytes and
both artifacts stayed below the 860,000-byte budget.

Raw and legacy canonical-Rational routes are asserted to produce identical dyadic
endpoints for zero, ±1/7, ±1/3, ±1/2, 0/1/32/128-bit precision, and both directed
bounds. Existing recurrence tests retain term-count/tail and overflow coverage.
The final CI-equivalent gate rebuilt the package and vanilla example at that same
candidate SHA and size, and passed native/no-default/Wasm tests, generated and
protocol/no-float checks, dependency policy, TypeScript/package/example builds,
the exact oracle, package-size budget, browser E2E (including the existing arcsine
UI path), and workspace documentation.

## Primitive squared arcsine coefficients

At base commit `9d0f271`, every arcsine Taylor step materialized
`numerator_squared * (2k-1) * (2k-1)` as a temporary BigInt and then multiplied
that coefficient into the growing term numerator. The retained recurrence updates
the owned term by the input numerator square and one exact primitive `u64` square.
The checked series index bounds `2k-1` to `u32`, so its square fits exactly in
`u64`; no float, truncation, precision, or term-count change is involved. The
first omitted term is also doubled in its owned buffer after the lower bound no
longer needs the undoubled value.

On 2026-07-13 with `rustc 1.97.0`, deterministic one-calculation allocation
changed as follows:

| Case | Before | After |
| --- | ---: | ---: |
| `asin(1/3)` | 438,548 bytes / 1,260 blocks | 426,740 / 1,031 |
| `asin(sin(1)/3)` | 881,571 / 1,795 | 859,259 / 1,569 |
| transformed `asin((2+sin(1))/3)` control | 1,163,828 / 4,302 | unchanged |
| `acos(1/3)` | 531,202 / 1,577 | 519,394 / 1,348 |

Peak live allocation was unchanged. Logical work remained 31, 200,216, 200,447,
and 31 units respectively. A same-target saved-baseline Criterion comparison found
no regression: `asin(1/3)` changed from a 2.4510 ms base midpoint to 2.4257 ms,
and the non-degenerate unit-series case from 8.6344 ms to 7.8899 ms, with both
confidence intervals classified as no detected change. The transformed control
does not enter this recurrence; its measured timing movement is not attributed to
the optimization.

An initial variant multiplied the growing term by the primitive odd value twice.
It preserved exact results and produced the same allocation totals but regressed
the two target Criterion midpoints by about 22% and 20%. Squaring the bounded odd
coefficient first restores one growing-term coefficient multiplication and removes
that regression. Exact oracle tests cover unit and nonunit numerators, zero,
0/1/5/20/64/128/256-term plans, paired/directed bounds, and checked oversized
term counts against the former materialized-coefficient recurrence.

The focused one-iteration/one-warmup Wasm/npm boundary smoke completed
`asin(1/3)` with the unchanged 1,772-byte payload using artifact
`ad8b8fb362a50dce605ae1e12986afa255078f6ba394e9841d34d5d037e55711`
(818,170 bytes). This is 316 bytes smaller than the `9d0f271` artifact and below
the 860,000-byte package budget; the single elapsed sample verifies boundary
integration and is not used as a timing claim.
Benchmark definition v17 adds the primary non-degenerate unit-series case
`asin(sin(1)/3)`. A same-host base/candidate smoke used base artifact
`63f5b763653137b83e035bb75733d353626d9a3980646616803a51b36eea4f89`
(818,486 bytes) and the candidate artifact above; both returned the exact
`asin(1/3*sin(1))` with a 1,786-byte payload. Their single elapsed samples were
48.5 ms and 38.8 ms respectively, but are recorded only as boundary coverage.
The final CI-equivalent gate rebuilt both package and example artifacts at the
same candidate SHA and 818,170-byte size, passed the package-size budget, and
completed the browser E2E suite.

## Shared exact-point nth-root search

At base commit `7b9e5c0`, general rational nth-root evaluation independently ran
the large-integer floor-root search for the scaled lower endpoint and again inside
the upper ceil-root helper. The two scaled integers differ by at most one. The
retained path searches the lower floor root once, raises it to the requested index,
and selects that root or the next integer for the upper endpoint. This is the
general-index counterpart of the existing shared square-root endpoint algorithm.

On 2026-07-13 with `rustc 1.97.0`, one public algebraic representative calculation
moved from 141,835 bytes / 5,264 blocks to 100,723 / 3,942, about 29% fewer bytes
and 25% fewer blocks. Peak live allocation remained 4,884 bytes / 104 blocks and
the logical-work boundary remained 400,234 units. The index-two specialized
`sqrt(2)` path and `2^sqrt(2)` general-power control were byte-, block-, and
peak-identical.

A same-target ten-sample Criterion comparison moved the algebraic midpoint from
428.97 µs to 379.52 µs, but the confidence interval was classified as no detected
change. General power also had no detected regression. The apparent movement of
the unchanged specialized `sqrt(2)` control reflects host-wide variation and is
not attributed to this change. The accepted claim is the deterministic removal of
the duplicated general nth-root search and its allocation reduction.

Exact oracle tests compare the former independent floor/ceil route for zero,
rational and integral inputs, indices 2/3/5/17, precisions 0/1/32/64, perfect and
non-perfect powers, and negative odd roots. Domain, precision, stopping behavior,
logical work, no-float, and public protocol are unchanged.

The focused one-iteration/one-warmup Wasm/npm smoke completed the algebraic case
with the unchanged 1,792-byte payload using artifact
`3bd844de7f84cd4f357c7b916fb3057700ac421744a74fa85dc5828c46fc5b1e`
(818,132 bytes), 38 bytes smaller than the `7b9e5c0` artifact and below the
860,000-byte budget. The single elapsed sample verifies boundary integration and
is not used as a timing claim.

The final CI-equivalent gate rebuilt the package and example artifacts at that
same SHA and 818,132-byte size, passed the package-size budget, and completed the
native, Wasm, package, example-build, external-oracle, and browser-E2E suites.

## Dyadic exponential common-denominator construction

The exponential Taylor state previously updated a second growing product for its
final common denominator at every recurrence step, in addition to the `b*n`
operand required by the partial sum. The final value is `b^N*N!`; dyadic public
paths now construct it by shifting the factorial for the power-of-two denominator,
with an exact power fallback for general rationals. Same-denominator unit-range
endpoints share the final value; value-dependent numerators remain independent.

On 2026-07-13 with `rustc 1.97.0`, deterministic one-calculation allocation for
`exp(sqrt(2)*ln(2))` moved from 241,934 bytes in 3,822 blocks to 205,030 bytes in
3,764 blocks. Direct `2^sqrt(2)` moved from 207,982 bytes in 2,306 blocks to
171,078 bytes in 2,248 blocks. Peak live allocation moved down by 16 bytes in
each case. Against main `069bf6b`, ten-sample Criterion midpoints moved from
1.107 ms to 835.58 µs for the cumulative exp stage and from 992.63 µs to
691.00 µs for general power (about 25% and 30% lower respectively). The final
run also improved significantly against the immediate in-place-product control.
The representative logical-work boundaries, including the 400,447-unit approximate
composite, were unchanged. The remaining endpoint cost is the necessarily
value-dependent term and partial-sum large-integer multiplication.

## Factor-seeded trigonometric range composition

At base commit `d84b6ca`, binary angle composition started from the identity pair,
so its first set bit performed a result-preserving interval composition. Commit
`ab3aec1` preserves the original bit scan but clones the factor at the first set
bit instead of multiplying it by the identity. Exact regressions compare the former identity-seeded result for
positive and negative divisors two through four; larger-divisor composition and
tangent pole handling retain the same operations.

On 2026-07-12 with `rustc 1.97.0`, controlled measurements were:

| Case | Bytes | Blocks | Native median |
| --- | ---: | ---: | ---: |
| `sin(2)` | 23,259 / 12,883 | 881 / 518 | 36.780 / 24.736 µs |
| `cos(2)` | 23,239 / 12,863 | 887 / 524 | 31.017 / 22.250 µs |
| `tan(2)` | 25,507 / 15,131 | 913 / 550 | 32.776 / 22.574 µs |

All three affected logical-work boundaries remained 200,133 units. The
three-iteration/one-warmup Wasm/npm snapshots used base artifact
`fca976e3afaaebb715dfda4f2a0021cbf7ab9d0db70a0d2cb6abb33181a60fda`
(785,129 bytes) and implementation artifact
`3f19032672f54a3e90b261835eafde326554f9b35c5463471b71a5f141152dcd`
(785,204 bytes). Public facade payloads remained 1,766/1,772/1,766 bytes for
sin/cos/tan. The three-iteration timings moved with host-wide load, so they are
retained only as integration snapshots without a Wasm speed claim.

Reproduce with the allocation, Criterion, logical-work, and Wasm commands above,
using the `sin_two`, `cos_two`, and `tan_two` case names.

## Borrowed unit trigonometric reduction

At base commit `26e6c0f`, the paired trigonometric evaluator divided canonical
unit-range inputs by the integer one through general Rational division. Commit
`509c3b9` borrows the input directly when the computed divisor is one; larger
range-reduction divisors are unchanged. Exact regressions cover negative, zero,
fractional, and boundary-one inputs.

On 2026-07-12 with `rustc 1.97.0`, one public `tan(1)` calculation moved from
11,527 bytes / 405 blocks to 11,439 bytes / 397 blocks. Twenty-sample timing
moved with host load and did not establish a speedup, so this slice claims only
the deterministic allocation reduction. Logical work remained 200,133 units.
The Wasm/npm integration snapshot used artifact
`44ab9f128810398de343bc8ec9298370548f99ea8a5a95d0de7d29e9866d86b8`
(785,232 bytes) and retained the 1,760-byte `tan(1)` payload without a timing
claim.

## Primitive trigonometric range divisor

At base commit `68e626d`, range reduction built an integer Rational for the
positive divisor and called general Rational division. Commit `baf9e7e` multiplies
the existing denominator by the primitive `u32` and canonicalizes once. Exact
regressions compare signed rationals and divisors 1, 2, 3, and 17 with the former
division, including explicit zero rejection.

On 2026-07-12 with `rustc 1.97.0`, deterministic allocations moved from
12,883/12,863/15,131 bytes to 12,843/12,823/15,091 bytes for public
`sin(2)`/`cos(2)`/`tan(2)` calculations. Blocks decreased by two in each case.
This slice claims the structural allocation reduction only; directed bounds,
logical work, and public protocol are unchanged.

## Primitive interval Rational halving

At base commit `20518b9`, interval halving built Rational two and called general
division. Commit `fb411da` reuses the positive primitive-scalar divisor path.
Exact signed, zero, integer, and fractional regressions match the former
division. On 2026-07-12 with `rustc 1.97.0`, public `acos(1/3)` allocation moved
from 676,274 bytes / 2,303 blocks to 675,954 / 2,287, and `atan(2)` moved from
81,754 / 1,783 to 81,594 / 1,775. Logical-work boundaries remained 31 units.
This slice claims deterministic allocation reduction only; public protocol and
directed bounds are unchanged.

## Primitive Machin coefficients

At base commit `f22ca1c`, Machin's pi enclosure built Rational values for
coefficients 16 and 4 and used general multiplication. Commit `de973da` applies
the positive primitive factors to the numerator before one canonicalization.
Exact signed/zero/fraction regressions match the former multiplication. On
2026-07-12 with `rustc 1.97.0`, public `atan(2)` allocation moved from 81,594
bytes / 1,775 blocks to 81,274 / 1,759. Logical work remained 5 units. This
slice claims deterministic allocation reduction only; directed bounds and the
public protocol are unchanged.

## Inverse-trigonometric exact-point sharing

At base commit `bb32a45`, atan, asin, and acos evaluated lower and upper Rational
endpoints separately even when an exact dyadic point made them equal. Commit
`d69dfbb` computes the paired bounds once for exact points; non-degenerate
interval endpoint selection is unchanged. A regression checks atan/asin/acos at
the exact dyadic point one half against their paired Rational definitions.

On 2026-07-12 with `rustc 1.97.0`, public `atan(2)` allocation moved from 81,274
bytes / 1,759 blocks to 49,434 / 1,149, and its twenty-sample native median moved
from 956.04 µs to 764.29 µs. Logical work remained 5 units. Non-dyadic inputs
such as one third still produce non-degenerate dyadic intervals and intentionally
retain endpoint-specific evaluation.

## Shared acos endpoint pi bounds

At base commit `11197e3`, non-degenerate acos intervals rebuilt the same pi
enclosure inside each endpoint evaluation. Commit `909e3bd` shares pi while
retaining endpoint-specific asin bounds and antitone selection. Exact regressions
cover -1, zero, one third, and one with shared versus independent pi.

On 2026-07-12 with `rustc 1.97.0`, public `acos(1/3)` allocation moved from
675,954 bytes / 2,287 blocks to 655,818 / 1,877. Logical work remained 31 units.
Timing moved with host load and did not establish a speedup, so this slice claims
only deterministic allocation reduction. Directed bounds and protocol are unchanged.

## Structural inverse-trig unit checks

At base commit `73f3810`, asin/acos built Rational ±1 for every endpoint special
check. Commit `088e730` compares canonical sign and numerator/denominator
magnitudes directly. Exact regressions cover -2 through 2. On 2026-07-12 with
`rustc 1.97.0`, public `acos(1/3)` allocation moved from 655,818 bytes / 1,877
blocks to 655,306 / 1,845. This slice claims deterministic allocation reduction
only; logical work, directed bounds, and protocol are unchanged.

## Structural inverse-sine half threshold

At base commit `81508c4`, the positive asin path constructed canonical Rational
`1/2` once to select the unit series and again for the unit helper assertion.
Commit `7cff422` compares twice the canonical nonnegative numerator with the
positive denominator instead. Regression values cover zero, exact one half,
nearby fractions on both sides, and values approaching one.

On 2026-07-13 with `rustc 1.97.0`, deterministic one-calculation allocations
changed as follows:

| Case | Bytes | Blocks |
| --- | ---: | ---: |
| `asin(1/3)` | 485,884 / 485,588 | 1,339 / 1,305 |
| `acos(1/3)` | 655,306 / 655,010 | 1,845 / 1,811 |

Logical work remained 31 units for both cases. This slice claims allocation
reduction only; series selection, directed bounds, domain behavior, and public
protocol are unchanged. Reproduce with allocation cases `approximate_asin_third`
and `approximate_acos_third`, followed by `logical_work_baseline`.

## Structural arctangent unit threshold

At base commit `31682c5`, nonnegative atan constructed Rational `1` to select
between its unit series and reciprocal identity, then repeated that comparison
in the unit helper assertion. Commit `5fe32ea` reuses the existing canonical
unit predicate, which compares numerator magnitude with the positive denominator.
Regression values cover both signs, zero, the exact unit boundary, and values on
either side; the existing `atan(2)` identity regression fixes reciprocal and pi
bound direction.

On 2026-07-13 with `rustc 1.97.0`, deterministic one-calculation allocations
changed as follows:

| Case | Bytes | Blocks |
| --- | ---: | ---: |
| `atan(1/2)` | 22,078 / 22,046 | 824 / 820 |
| `atan(2)` | 49,434 / 49,402 | 1,149 / 1,145 |

Logical work remained 31 and 5 units respectively. This slice claims only the
deterministic allocation reduction; reciprocal direction, directed bounds, and
public protocol are unchanged. Reproduce with allocation cases
`approximate_atan_half` and `approximate_atan_two`, followed by
`logical_work_baseline`.

## Structural inverse-trigonometric domain units

At base commit `6511317`, the shared asin/acos interval-domain layer constructed
Rational `-1` and `1`, then performed four general comparisons for every input.
Commit `da75d7b` classifies each canonical endpoint by numerator magnitude versus
its positive denominator and uses its sign to distinguish intervals wholly
outside the domain from intervals that only overlap it. Regressions cover both
fully-outside sides, both partial-overlap sides, and the inclusive `[-1, 1]`
boundary.

On 2026-07-13 with `rustc 1.97.0`, deterministic one-calculation allocations
changed as follows:

| Case | Bytes | Blocks |
| --- | ---: | ---: |
| `asin(1/3)` | 485,588 / 485,388 | 1,305 / 1,293 |
| `acos(1/3)` | 655,010 / 654,810 | 1,811 / 1,799 |

Logical work remained 31 units for both cases. This slice claims deterministic
allocation reduction only; typed domain/unsupported classification, directed
bounds, and public protocol are unchanged. Reproduce with allocation cases
`approximate_asin_third` and `approximate_acos_third`, followed by
`logical_work_baseline`.

## Primitive exponential range reduction

At base commit `61c853b`, positive exponential range reduction converted its
positive `u32` ceiling to a Rational and used general division. Directed bounds
also cloned the input when the divisor was one. Commit `72a8578` multiplies the
existing denominator by the primitive divisor and canonicalizes once, while the
unit directed path borrows its input. The paired helper regression records
`3/2 -> 3/4`, and the scalar-division regression remains exact across signs,
denominators, and divisors.

On 2026-07-13 with `rustc 1.97.0`, deterministic one-calculation allocations
changed as follows:

| Case | Bytes | Blocks |
| --- | ---: | ---: |
| `exp(2)` | 17,897 / 17,857 | 657 / 655 |
| `2^sqrt(2)` | 226,166 / 226,038 | 2,903 / 2,899 |
| `exp(sqrt(2)*ln(2))` | 260,118 / 259,990 | 4,419 / 4,415 |

This slice claims deterministic allocation reduction only. Directed bounds,
integer powers, reciprocal direction, logical-work accounting, and public
protocol are unchanged. Reproduce with allocation cases `approximate_exp_two`,
`approximate_general_power`, and `approximate_exp_power_log_product`, followed
by `logical_work_baseline`.

## Direct logarithm Mobius transform

At base commit `14964d1`, reduced logarithm evaluation formed
`z=(x-1)/(x+1)` through Rational subtraction, addition, and division, each with
general canonicalization. Commit `d56e033` uses canonical `x=n/d` to form
`(n-d)/(n+d)` and canonicalizes only that final Rational. Exact regressions cover
the endpoints one and two, noninteger inputs, and `5/3`, whose direct parts need
additional reduction to `1/4`.

On 2026-07-13 with `rustc 1.97.0`, deterministic one-calculation allocations
changed as follows:

| Case | Bytes | Blocks |
| --- | ---: | ---: |
| `ln(2)` | 21,141 / 20,917 | 806 / 778 |
| `2^sqrt(2)` | 226,038 / 225,814 | 2,899 / 2,871 |
| `sqrt(2)*ln(2)` | 109,202 / 108,978 | 4,037 / 4,009 |
| `exp(sqrt(2)*ln(2))` | 259,990 / 259,766 | 4,415 / 4,387 |

Logical-work baseline values remained unchanged. This slice claims deterministic
allocation reduction only; range reduction, series tail, directed bounds, and
public protocol are unchanged. Reproduce with allocation cases
`approximate_log_two`, `approximate_general_power`,
`approximate_power_log_product`, and `approximate_exp_power_log_product`, followed
by `logical_work_baseline`.

## Structural logarithm range reduction

At base commit `08cf67a`, logarithm range reduction compared each positive
Rational against constructed `1` and `2` values and used general Rational
multiplication or division for every binary step. Commit `33446e0` compares the
canonical numerator with the denominator or twice the denominator, and uses the
existing primitive scaling and halving helpers. Exact regressions compare the
old and new loops for positive and negative binary exponents, multiple steps,
fractional inputs, and both range boundaries.

On 2026-07-13 with `rustc 1.97.0`, deterministic one-calculation allocations
changed as follows:

| Case | Bytes | Blocks |
| --- | ---: | ---: |
| `ln(2)` | 20,917 / 20,829 | 778 / 770 |
| `2^sqrt(2)` | 225,814 / 225,726 | 2,871 / 2,863 |
| `sqrt(2)*ln(2)` | 108,978 / 108,890 | 4,009 / 4,001 |
| `exp(sqrt(2)*ln(2))` | 259,766 / 259,678 | 4,387 / 4,379 |

This slice claims deterministic allocation reduction only. Range boundaries,
step limits, binary-exponent accounting, directed bounds, logical work, and
public protocol are unchanged. Reproduce with allocation cases
`approximate_log_two`, `approximate_general_power`,
`approximate_power_log_product`, and `approximate_exp_power_log_product`, followed
by `logical_work_baseline`.

## Directed logarithm endpoints

At base commit `b95bde6`, a non-degenerate logarithm interval evaluated paired
bounds at both endpoints, retaining only the lower result from the lower endpoint
and the upper result from the upper endpoint. Commit `fb56605` keeps one shared
series state for exact points but gives directed endpoints only the requested
canonical Rational. Negative binary exponents reverse the selected log-two bound
before signed composition, exactly as in the paired path. Regressions compare
directed and paired results at precisions 1, 64, and 128 across unit, fractional,
positive/negative binary-exponent, and multi-step inputs.

The public benchmark case `ln(2+sin(1))` exercises a genuinely non-degenerate
log interval. On 2026-07-13 with `rustc 1.97.0`, one calculation moved from
410,484 bytes / 1,889 blocks to 351,468 / 1,787. Separate 20-sample Criterion
runs at the base and implementation commits measured medians of approximately
8.40 ms and 3.75 ms, about a 55% reduction. Logical work remained 200,225 units.
Directed bounds, tail guarantees, resource accounting, and public protocol are
unchanged.

The same source is part of Wasm/npm benchmark definition v8. Three iterations
after one warmup measured 51.61 ms/iteration at the base artifact
`cf78b3950892c354cb583f01fd1613aaf043f954176460cd6bc08a864e4a4dc0`
(785,878 bytes) and 26.51 ms/iteration at implementation artifact
`4df7e49eb99a733625ad6d8a24ba8dff697e3248636419557c9465254aae916a`
(787,417 bytes). Both returned the same 1,772-byte payload. This boundary sample
supports the native direction but is not treated as a statistically powered
Wasm timing claim.

Reproduce with allocation case `approximate_log_non_degenerate`, Criterion case
`approximate_components/log_non_degenerate`, and `logical_work_baseline`.
Build Wasm and reproduce the boundary snapshot with three iterations and one
warmup via the package `benchmark` script.

## Directed arctangent endpoints

At base commit `dd9d1ab`, a non-degenerate atan interval built paired bounds at
both endpoints. For reciprocal-domain inputs, each discarded side also included
paired unit-atan and Machin pi series. Commit `7f9daef` selects sum or adjacent
from the alternating-series next-term sign before canonicalization and propagates
the requested direction through reciprocal, pi/2 subtraction, and odd negation.
Exact points retain the paired path. Regressions compare directed and paired
bounds at precisions 1, 64, and 128 across zero, unit boundaries, both signs,
and reciprocal-domain values.

The public benchmark `atan(2+sin(1))` moved from 945,954 bytes / 2,643 blocks to
714,058 / 2,249. Separate 20-sample Criterion runs measured native medians of
approximately 30.24 ms and 11.04 ms, about a 63% reduction. Logical work remained
200,225 units.

Wasm/npm benchmark definition v9 measured the same 1,776-byte payload. Three
iterations after one warmup moved from 147.57 ms/iteration at base artifact
`4df7e49eb99a733625ad6d8a24ba8dff697e3248636419557c9465254aae916a`
(787,417 bytes) to 67.40 ms/iteration at implementation artifact
`be2d91fd29c5a03542f72518807fb58c16f7e679ccff07f0b188e4d5774b3090`
(789,546 bytes). This low-sample boundary snapshot supports the native direction
but is not a statistically powered Wasm timing claim. Directed bounds, resource
accounting, and public protocol are unchanged.

Reproduce with allocation case `approximate_atan_non_degenerate`, Criterion case
`approximate_components/atan_non_degenerate`, `logical_work_baseline`, and the
package Wasm build/benchmark commands.

## Shared reciprocal arctangent pi enclosure

At base commit `76fbc8f`, both reciprocal-domain endpoints of a non-degenerate
atan interval independently ran the two Machin recurrences for their directed pi
bounds. Commit `8cee3af` builds one paired pi enclosure only when both endpoints
are outside the unit interval and shares it across them. Unit-only and mixed
unit/reciprocal intervals retain the previous directed path. Exact regressions
cover both signs, a zero-crossing reciprocal endpoint pair, and the mixed path.

On 2026-07-13 with `rustc 1.97.0`, deterministic allocation for
`atan(2+sin(1))` moved from 714,058 bytes / 2,249 blocks to 710,074 / 1,983.
Logical work remained 200,225 units. Separate 10-sample Criterion snapshots had
overlapping estimates (base 13.50--17.24 ms, implementation 14.10--15.94 ms) on
the contended shared host, so this slice makes no native timing claim. The
algorithm removes two of four input-independent Machin recurrence traversals;
endpoint-specific reciprocal atan work is unchanged.

Wasm/npm benchmark definition v11 retained the 1,776-byte payload. One-iteration
smokes moved from 151.49 ms at base artifact
`c5536723ac8fecfae0b5dc4bcc09b6acf170d81eefcaa54bb387afebfdab4ae2`
(793,359 bytes) to 88.61 ms at implementation artifact
`aed5ef521c7fd69606d8651ae0020997b2726573892c77a95dd6c46290d6995f`
(793,711 bytes). This is boundary smoke data, not a powered timing claim.
CLI stdout was byte-identical at both revisions, SHA-256
`05a06ac2362c0a7adaede5f7b766aa6cf796ecec3a67af526664a9f2cfe5e85c`.
The package check, example production build, and browser E2E passed against the
implementation artifact, confirming unchanged npm presentation and example-ui
display. Directed bounds, resource accounting, and public protocol are unchanged.

## Directed unit inverse-sine endpoints

At base commit `671dc62`, non-degenerate asin endpoints built paired positive
series bounds and discarded one side. Commit `28d1f62` constructs only the
partial sum for a requested lower bound or the existing next-term tail for an
upper bound when `|x|<=1/2`; negative inputs reverse direction before negation.
The transform region retains its paired fallback. Directed and paired regressions
cover precisions 1, 64, and 128, zero, both signs, the half boundary, and a
transform-region control.

The public case `asin(sin(1)/3)` moved from 965,275 bytes / 1,827 blocks to
881,539 / 1,794. Separate 20-sample Criterion runs measured native medians of
approximately 8.72 ms and 4.05 ms, about a 54% reduction. Logical work remained
200,216 units. This slice changes no public protocol or resource accounting.

Reproduce with allocation case `approximate_asin_non_degenerate_unit`, Criterion
case `approximate_components/asin_non_degenerate_unit`, and
`logical_work_baseline`.

## Directed transformed inverse-sine endpoints

At base commit `3a4d9dd`, transformed asin endpoints built paired square-root,
atan, and pi bounds. Commit `34460c9` uses sqrt upper / atan upper / pi lower for
a requested lower bound and the opposite endpoints for an upper bound, following
`asin(x)=pi/2-atan(sqrt(1-x^2)/x)`. Negative inputs retain odd direction reversal.
Directed and paired regressions cover precisions 1, 64, and 128, the half boundary,
both signs, values near one, and exact endpoints.

The public case `asin((2+sin(1))/3)` moved from 3,690,444 bytes / 9,274 blocks to
1,479,412 / 4,383. Separate 20-sample Criterion runs measured native medians of
approximately 110.20 ms and 43.09 ms, about a 61% reduction. Logical work remained
200,447 units.

Wasm/npm benchmark definition v10 returned the same 1,788-byte payload. A
single-iteration boundary snapshot moved from 579.65 ms at base artifact
`fa966b812590848e37f0e1b0412b83fb0b9a76a7823b57c463250a51058960ac`
(790,855 bytes) to 268.87 ms at implementation artifact
`a0925e73604fa13cc32c5953cadb8747df1c8919a00e86ae3de37c29a6b04d0e`
(791,625 bytes). This is a smoke snapshot, not a powered Wasm timing claim.
Public protocol and resource accounting are unchanged.

Reproduce with allocation/criterion case `asin_non_degenerate_transform`,
`logical_work_baseline`, and the package Wasm build/benchmark commands.

## Directed inverse-cosine endpoints

At base commit `7756f8f`, non-degenerate acos selected antitone input endpoints
correctly but built paired asin bounds inside each endpoint. Commit `11303b1`
combines the shared pi enclosure with only the opposite directed asin bound.
Minus one, zero, and plus one remain direct shared-pi special cases, avoiding
independent pi cancellation. Unit and transformed values of both signs match the
old shared paired bounds exactly.

The public case `acos((2+sin(1))/3)` moved from 4,035,718 bytes / 9,834 blocks to
1,667,222 / 4,872. Separate 20-sample Criterion runs measured native medians of
approximately 157.32 ms and 75.84 ms, about a 52% reduction. Logical work remained
200,447 units. Directed enclosure, resource accounting, and protocol are unchanged.

Reproduce with allocation/criterion case `acos_non_degenerate_transform` and
`logical_work_baseline`.

## Borrowed positive cosine inputs

At base commit `61d4a6e`, unit cosine cloned every Rational before applying even
symmetry. Commit `0bcf9ab` owns only the negated storage for negative values and
borrows nonnegative inputs. On 2026-07-13, deterministic allocations changed
from 8,149 bytes / 317 blocks to 8,133 / 315 for `cos(1)`, and from 12,823 / 522
to 12,807 / 520 for `cos(2)`. This slice claims allocation reduction only;
series bounds, range reduction, logical work, and protocol are unchanged.

## Direct asin complement squares

At base commit `fe77c2e`, transformed asin formed `x*x` and then `1-x^2` as two
canonical Rational operations. Commit `8e0f69f` forms `(d^2-n^2)/d^2` directly
from canonical `x=n/d` and canonicalizes once. Exact regressions cover zero,
both signs, the half and unit boundaries, and values near one.

On 2026-07-13, `asin((2+sin(1))/3)` moved from 1,479,412 bytes / 4,383 blocks
to 1,477,652 / 4,345, while the corresponding acos case moved from 1,667,222 /
4,872 to 1,665,462 / 4,834. This slice claims deterministic allocation reduction
only; directed bounds, logical work, resource accounting, and protocol are unchanged.

## Canonical complement-square construction

At base commit `a358b8e`, direct complement-square parts still passed through a
general GCD. Commit `f868f5b` uses `gcd(n,d)=1` to prove
`gcd(d^2-n^2,d^2)=1`, directly constructs nonzero canonical Rationals, and keeps
the ±1 result as canonical zero. Transformed asin moved from 1,477,652 bytes /
4,345 blocks to 1,477,108 / 4,333; transformed acos moved from 1,665,462 / 4,834
to 1,664,918 / 4,822. This slice claims deterministic allocation reduction only.

## Signed primitive binary-log scaling

At base commit `625998a`, log composition and large-exp residuals converted their
bounded `i64` binary exponent to a Rational before general multiplication.
Commit `d697a7b` multiplies the canonical numerator by the signed primitive and
canonicalizes once. Exact regression includes zero, both signs, and `i64::MIN`.

Each affected public calculation removed 80 bytes / 4 blocks: `ln(2)` moved from
20,829 / 770 to 20,749 / 766, non-degenerate log from 351,468 / 1,787 to
351,388 / 1,783, `exp(-10000)` from 564,880 / 2,106 to 564,800 / 2,102, and
`exp(10000)` from 536,736 / 2,023 to 536,656 / 2,019. This slice claims
deterministic allocation reduction only; directions, logical work, and protocol
are unchanged.

## Shared transformed inverse-sine pi enclosure

At base commit `f3cdf78`, each transformed asin endpoint independently ran the
two Machin arctangent recurrences needed for its directed pi bound. Commits
`e612f15` and `385efeb` build one paired pi enclosure only when both endpoints
are in the transform region and share it across them. The existing acos enclosure
now also reaches its internal directed asin. Series-only intervals do not build
pi, and a mixed series/transform interval retains its single directed pi bound.
Exact regressions cover both signs, unit and half-region values, and intervals
with one or both transformed endpoints.

On 2026-07-13 with `rustc 1.97.0`, deterministic one-calculation allocations for
`asin((2+sin(1))/3)` moved from 1,477,108 bytes / 4,333 blocks to 1,473,156 /
4,068. `acos((2+sin(1))/3)` moved from 1,664,918 / 4,822 to 1,641,118 / 4,162.
Logical work remained 200,447 units for both. Criterion was executed for both
revisions, but concurrent host load changed materially between runs, so those
absolute samples are not used as a timing claim. The algorithm removes two of
four input-independent Machin recurrence traversals per non-degenerate transform
interval; endpoint-specific sqrt and atan work is unchanged. Directed bounds,
resource accounting, and protocol are unchanged.

Wasm/npm benchmark definition v11 adds the corresponding transformed-acos case.
One-iteration boundary smokes used identical sources and presentation requests.
The base artifact was
`468c30c0b5909b72ae1e4af8d2fd2d6e751328a07002735631bbae59cd34a1e6`
(792,927 bytes); the reviewed artifact was
`c5536723ac8fecfae0b5dc4bcc09b6acf170d81eefcaa54bb387afebfdab4ae2`
(793,359 bytes). Asin retained its 1,788-byte serialized payload and acos its
1,794-byte payload. Host contention also makes these single-run Wasm times smoke
data rather than a timing claim. CLI stdout remained byte-identical: asin SHA-256
`f84326849c16faeb7851f12b51ed3c5694edd9020955a1dbc96cf19ef45d5e07`
and acos SHA-256
`d10359056958748c0e2da89393fc49277510f74234a7df85a1e8c45f12fd7901`.
The package check, example production build, and browser E2E passed against the
reviewed artifact, confirming unchanged npm presentation and example-ui display.

## Shared exact transformed-acos pi enclosure

At base commit `35956af`, an exact dyadic transformed acos point built one pi
enclosure inside asin and another for the outer `pi/2-asin(x)`. Commit `ed2bd8e`
passes the outer paired enclosure through the paired asin transform. Regressions
cover both signs, series/transform values, zero, and unit endpoints.

On 2026-07-13 with `rustc 1.97.0`, one `acos(3/4)` calculation moved from
830,525 bytes / 4,291 blocks to 810,709 / 3,897. Logical work remained 31 units.
Separate 10-sample Criterion runs moved the native midpoint estimate from
10.04 ms to 7.71 ms, about 23%.
The benchmark uses dyadic `3/4`; non-dyadic `2/3` enters the already optimized
non-degenerate enclosure path and is not evidence for this exact-point slice.
Directed bounds, resource accounting, and protocol are unchanged.

Wasm/npm benchmark definition v12 retained the 1,772-byte payload. Contended
one-iteration smokes used base artifact
`aed5ef521c7fd69606d8651ae0020997b2726573892c77a95dd6c46290d6995f`
(793,711 bytes) and implementation artifact
`3c7210041401c36c10085e905fdea68846ca6ddfc9e6d8b41e1d31fdc77a1cd1`
(793,501 bytes); their elapsed values are not used as a timing claim. Package
check, example production build, and browser E2E passed for the implementation.
CLI stdout was byte-identical, SHA-256
`721cd6720b8af23ec51372d6fb543fdd6d7ee15f1a964bbfdef080013e3dbb19`.

## Directed atan endpoints for exact transformed asin

At base commit `b04cb36`, exact transformed asin evaluated paired atan bounds for
both sqrt-derived ratios and discarded one side of each pair. Commit `514c000`
uses atan lower for the lower ratio and atan upper for the upper ratio, matching
the subtraction directions exactly. Shared-vs-independent regressions cover both
signs, series/transform values, zero, and unit endpoints.

On 2026-07-13 with `rustc 1.97.0`, `acos(3/4)` moved from 810,709 bytes / 3,897
blocks to 756,117 / 3,863. Logical work remained 31 units. Separate 10-sample
Criterion midpoint estimates moved from 8.81 ms to 7.43 ms, about 16%.

Wasm/npm definition v12 retained the 1,772-byte payload. One-iteration boundary
smokes moved from 94.87 ms at base artifact
`3c7210041401c36c10085e905fdea68846ca6ddfc9e6d8b41e1d31fdc77a1cd1`
(793,501 bytes) to 51.10 ms at implementation artifact
`6c1b637bfea87ed28ee835cc7dfbe11109b3bb62802599955f814aef6728e539`
(793,396 bytes). This is not a powered Wasm timing claim. Directed enclosures,
resource accounting, and protocol are unchanged. CLI stdout remained byte-identical,
SHA-256 `721cd6720b8af23ec51372d6fb543fdd6d7ee15f1a964bbfdef080013e3dbb19`.
Package check, example production build, and browser E2E passed for the
implementation artifact, confirming unchanged npm and example-ui presentation.

## Domain-safe trigonometric square identity

At base commit `152e3c2`, `sin(1)^2+cos(1)^2` remained a two-term symbolic
polynomial and both transcendental components were evaluated for requested
numeric outputs. The implementation recognizes complementary square factors in
canonical sparse polynomial terms and materializes exact `1`. Equal common
factors and same-sign Rational coefficient portions are handled independently of
source order, multiplication spelling, distribution, and nested identity chains.

On 2026-07-13 with `rustc 1.97.0`, one public calculation moved from 40,632
bytes / 1,347 blocks to 29,190 / 984. Separate 20-sample Criterion midpoint
estimates moved from 204.13 µs to 63.96 µs. Conservative logical work increased
from 400,483 to 400,653 units because compatible-pair discovery, cascade closure,
repeated sort/merge, and arbitrary-precision coefficient operations are now
reserved before mutation; unrelated benchmark boundaries remained unchanged.

Wasm/npm benchmark definition v14 added the same public case. One-iteration
boundary smokes moved from 1.74 ms with a 1,784-byte payload at base artifact
`d848924afe25a40f477ad509e6fe189407cdc7c387f60c5b010e7475f7b7ad23`
(794,742 bytes) to 1.72 ms with the exact-one 1,720-byte payload at implementation
artifact `3289f606a208b1ed3cdef28bf6debfee37812e20c853e05ff8f44dadc6418e36`
(805,831 bytes). This is not a powered Wasm timing claim. Tight rewrite/logical
limits retain the complete original canonical expression in a typed partial;
the expression-node cap remains a typed input limit. Domain errors are not
hidden, and no-float, scientific/enclosure, and DTO protocol contracts remain
unchanged.
Package check, example production build, and browser E2E passed; the browser
regression enters the identity through the example UI and observes exact `1`.
The calculator-cli integration regression also returns exact stdout `1` for the
same public expression.

## Shared Taylor plan for non-degenerate exponential endpoints

At base commit `85065dd`, ordinary non-degenerate exponential evaluation chose
the same factorial-based Taylor term count independently for the lower and upper
endpoint. Commit `226debb` computes this precision-only plan once and passes it
to both directed endpoint evaluations. Binary-scaled endpoints retain their
independent working-precision plans, and exact points retain their shared
recurrence state.

On 2026-07-13 with `rustc 1.97.0`, deterministic allocation for `2^sqrt(2)`
moved from 208,062 bytes / 2,310 blocks to 207,982 / 2,306. The cumulative
`exp(sqrt(2)*ln(2))` path moved from 242,014 / 3,826 to 241,934 / 3,822.
Twenty-sample general-power Criterion midpoint estimates were 703.19 µs and
618.61 µs. The corresponding cumulative `exp(sqrt(2)*ln(2))` midpoint estimates
were 777.23 µs and 797.12 µs. These inconsistent host-sensitive movements do not
establish a timing speedup for this small planning change. The approximate
logical-work boundary remained 400,447 units.

One-iteration Wasm/npm boundary smokes retained the 1,812-byte approximate
payload and moved from 7.96 ms at base artifact
`72cdee12bd04cf0a9977eb272a6b49c234f5072c73040492de1e2f82fa799fe7`
(794,661 bytes) to 7.55 ms at implementation artifact
`d848924afe25a40f477ad509e6fe189407cdc7c387f60c5b010e7475f7b7ad23`
(794,742 bytes). This is not a powered Wasm timing claim. Directed enclosure,
resource accounting, no-float policy, and public protocol are unchanged.
Package check, example production build, and browser E2E passed for the
implementation artifact, confirming unchanged npm and example-ui presentation.

## Shared integer-root search for exact sqrt bounds

At base commit `bc655eb`, an exact-point square root independently searched for
the integer floor root used by its lower bound and the integer floor root used
inside its upper ceil-root calculation. The scaled lower and upper integers
differ by at most one. Commit `934ae68` searches the lower floor root once, then
compares its exact square with the scaled upper integer to select either that
root or its successor. No floating-point estimate or precision shortcut is used.

On 2026-07-13 with `rustc 1.97.0`, deterministic one-calculation allocations
changed as follows:

| Case | Bytes | Blocks |
| --- | ---: | ---: |
| `sqrt(2)` | 59,537 / 41,953 | 1,937 / 1,388 |
| `2^sqrt(2)` | 225,646 / 208,062 | 2,859 / 2,310 |
| `sqrt(2)*ln(2)` | 108,810 / 91,226 | 3,997 / 3,448 |
| `exp(sqrt(2)*ln(2))` | 259,598 / 242,014 | 4,375 / 3,826 |

Separate 20-sample `sqrt(2)` Criterion midpoint estimates moved from 102.04 µs
to 59.58 µs. General-power timings moved with host-wide load and do not support
a timing claim. The approximate logical-work boundary remained 400,447 units.

One-iteration Wasm/npm boundary smokes retained the 1,812-byte approximate
payload and moved from 10.55 ms at base artifact
`15e043173a5dec60a4500d9bc5afd9edd0a63de5596211f2d8f3064dc050dd2a`
(794,581 bytes) to 7.54 ms at implementation artifact
`72cdee12bd04cf0a9977eb272a6b49c234f5072c73040492de1e2f82fa799fe7`
(794,661 bytes). This is not a powered Wasm timing claim. Directed enclosure,
resource accounting, no-float policy, and public protocol are unchanged.
Package check, example production build, and browser E2E passed for the
implementation artifact, confirming unchanged npm and example-ui presentation.

## Region-selected exact asin transform

At base commit `56c3721`, exact rational `1/2 < x < 1/sqrt(2)` used
`pi/2-atan(sqrt(1-x^2)/x)`. Its atan argument exceeds one, so atan performed a
second pi/2 reciprocal transform and the two independently certified pi terms
were later cancelled. Commit `b12f7bf` selects the equivalent
`atan(x/sqrt(1-x^2))` form for paired evaluation in this region using exact
integer-square comparison; commit `14d6d9b` applies the same selection to
directed endpoint evaluation. The new directed interval is contained in the
former enclosure for both signs; the upper transform region retains exact
endpoint equality. Paired and directed endpoint evaluators use the same region
selection.

On 2026-07-13 with `rustc 1.97.0`, `acos(5/8)` moved from 906,672 bytes / 4,695
blocks to 730,664 / 3,899. Logical work remained 31 units. Separate 10-sample
Criterion midpoint estimates moved from 28.72 ms to 12.37 ms, about 57%.
Resource accounting, no-float policy, and protocol are unchanged.

Wasm/npm benchmark definition v13 retained the 1,772-byte payload. One-iteration
boundary smokes moved from 304.85 ms at base artifact
`6c1b637bfea87ed28ee835cc7dfbe11109b3bb62802599955f814aef6728e539`
(793,396 bytes) to 66.35 ms at implementation artifact
`6d4bd640cf3bf1e48801f21646bfde10feca7833a87c2ed6c781eacd38a301f7`
(794,277 bytes). This is not a powered Wasm timing claim.
CLI stdout remained byte-identical, SHA-256
`79b1b4204a05f5a6f97dde50d7af06222b613a03fcc61f984c83f72c84156913`.
Package check, example production build, and browser E2E passed for the
implementation artifact, confirming unchanged npm and example-ui presentation.

## Direct unit-range trigonometric pair

At base commit `f12fc66`, the paired trigonometric evaluator initialized the
identity pair and composed it with the series pair even when the range-reduction
divisor was one. That composition constructed four interval products, one
addition, one subtraction, and two clamps without changing the result. Commit `f85a422` returns
the series pair directly for divisor one while retaining binary angle
composition for larger divisors. A regression reconstructs the former identity
composition and checks exact dyadic equality for negative, zero, fractional, and
boundary-one inputs.

On 2026-07-12 with `rustc 1.97.0`, one public `tan(1)` calculation moved from
21,903 bytes / 768 blocks to 11,527 bytes / 405 blocks. A twenty-sample Criterion
run changed the native median from 33.659 µs to 16.478 µs. The affected
logical-work boundary remained 200,133 units.

A three-iteration/one-warmup Wasm/npm snapshot used base artifact
`5b56dbe91a5857d9815cd959c70246120b6e69b51e31bfd4a7b9bcba4f91454c`
(785,045 bytes) and implementation artifact
`fca976e3afaaebb715dfda4f2a0021cbf7ab9d0db70a0d2cb6abb33181a60fda`
(785,129 bytes). The public `tan(1)` path moved from 0.389 to 0.312 ms per
iteration and retained its 1,760-byte payload. These low-sample Wasm values are
integration snapshots, not statistically powered claims.

Reproduce with:

```sh
CALCULATOR_ALLOCATION_ITERATIONS=1 \
  cargo run --profile bench -p calculator-core --features std \
    --example allocation_baseline -- approximate_tan_one
cargo bench -p calculator-core --bench representative_paths --features std \
  -- approximate_components/tan_one --sample-size 20
cargo run --profile bench -p calculator-core --features std \
  --example logical_work_baseline
corepack pnpm --dir packages/calculator run build:wasm
CALCULATOR_BENCH_ITERATIONS=3 CALCULATOR_BENCH_WARMUP=1 \
  corepack pnpm --silent --dir packages/calculator run benchmark
```

## Direct unit-range trigonometric projection

At base commit `8112e5d`, `sin_rational` and `cos_rational` always evaluated both
series and passed them through the angle-addition pair, even for `|x| <= 1`, then
discarded one component. Commit `d4c1619` constructs the requested unit-range
component directly from its directed Rational bounds. Inputs outside the unit
range and tangent retain the paired range-reduction path. An exact regression
matches direct and paired dyadic intervals for negative, zero, fractional, and
boundary-one inputs.

On 2026-07-12 with `rustc 1.97.0`, controlled public-path measurements were:

| Case | Bytes | Blocks | Native median |
| --- | ---: | ---: | ---: |
| `sin(1)` | 20,007 / 8,367 | 755 / 322 | 67.962 / 9.1042 µs |
| `cos(1)` | 19,893 / 8,149 | 749 / 317 | 60.761 / 10.251 µs |
| `tan(1)` | 21,903 / 21,903 | 768 / 768 | 30.391 / 31.704 µs |

The affected logical-work boundaries remained 200,133 units. Tangent's code and
deterministic allocation path are unchanged; repeated native timing samples
moved around the baseline by a few percent, so no tangent timing change is
claimed.

A three-iteration/one-warmup Wasm/npm comparison used base artifact
`78c594a84576e1fa9f5f416fbf6106362e33508af77a3ff481e6c386ff62065d`
(784,595 bytes) and implementation artifact
`5b56dbe91a5857d9815cd959c70246120b6e69b51e31bfd4a7b9bcba4f91454c`
(785,045 bytes). The public facade snapshots moved from 0.590 to 0.318 ms for
`sin(1)` and from 0.422 to 0.237 ms for `cos(1)`, while tangent remained on its
paired path. Payloads remained 1,766 bytes for sin/cos and 1,760 bytes for tan.
These low-sample values are integration snapshots, not statistically powered
claims.

Reproduce with:

```sh
for case in approximate_sin_one approximate_cos_one approximate_tan_one
do
  CALCULATOR_ALLOCATION_ITERATIONS=1 \
    cargo run --profile bench -p calculator-core --features std \
      --example allocation_baseline -- "$case"
done
for case in sin_one cos_one tan_one
do
  cargo bench -p calculator-core --bench representative_paths --features std \
    -- "approximate_components/$case" --sample-size 20
done
cargo run --profile bench -p calculator-core --features std \
  --example logical_work_baseline
corepack pnpm --dir packages/calculator run build:wasm
CALCULATOR_BENCH_ITERATIONS=3 CALCULATOR_BENCH_WARMUP=1 \
  corepack pnpm --silent --dir packages/calculator run benchmark
```

## Inverse-sine common-denominator recurrence

At base commit `41b5531`, the positive inverse-sine series represented every
term as a Rational and canonicalized its coefficient multiplication, division,
and partial-sum addition. Commit `5253462` carries the odd power, coefficient
products, partial sum, and common denominator as BigInts, then canonicalizes
only the lower bound and the upper bound containing the unchanged twice-next-term
tail. An exact regression compares zero through one half and multiple term-count
parities with the former Rational definition.

On 2026-07-12 with `rustc 1.97.0`, controlled public-path measurements were:

| Case | Bytes | Blocks | Native median |
| --- | ---: | ---: | ---: |
| `asin(1/3)` | 5,117,548 / 486,300 | 6,356 / 1,363 | 72.396 / 1.7049 ms |
| `acos(1/3)` | 5,307,522 / 676,274 | 7,296 / 2,303 | 67.772 / 5.6749 ms |

The affected logical-work boundary remained 31 units for both paths. A
three-iteration/one-warmup Wasm/npm comparison used base artifact
`9b77e4250dd5d88926ab17667754a31a2e25f76f80b54acfd5de9b7c6111a3d3`
(784,060 bytes) and implementation artifact
`78c594a84576e1fa9f5f416fbf6106362e33508af77a3ff481e6c386ff62065d`
(784,595 bytes). The public facade changed from 462 to 11.3 ms per `asin(1/3)`
iteration and from 491 to 35.7 ms per `acos(1/3)` iteration while retaining the
1,772- and 1,766-byte payloads. These low-sample Wasm values are integration
snapshots rather than statistically powered claims.

Reproduce with:

```sh
for case in approximate_asin_third approximate_acos_third
do
  CALCULATOR_ALLOCATION_ITERATIONS=1 \
    cargo run --profile bench -p calculator-core --features std \
      --example allocation_baseline -- "$case"
done
cargo bench -p calculator-core --bench representative_paths --features std \
  -- approximate_components/asin_third --sample-size 20
cargo bench -p calculator-core --bench representative_paths --features std \
  -- approximate_components/acos_third --sample-size 20
cargo run --profile bench -p calculator-core --features std \
  --example logical_work_baseline
corepack pnpm --dir packages/calculator run build:wasm
CALCULATOR_BENCH_ITERATIONS=3 CALCULATOR_BENCH_WARMUP=1 \
  corepack pnpm --silent --dir packages/calculator run benchmark
```

## Euler's number factorial denominator

At base commit `32a84f4`, the constant `e` enclosure constructed every `1/n!`
as a Rational and canonicalized the partial sum after every addition. This is a
separate interval path from `exp(1)`. Commit `cde8380` keeps `n!` as the shared
denominator and updates the numerator with `sum*n+1`, canonicalizing only the
lower bound and the upper bound containing the unchanged `2/(N+1)!` tail. An
exact regression compares term counts from zero through 64 with the former
Rational definition.

On 2026-07-12 with `rustc 1.97.0`, one public `e` calculation moved from 34,024
bytes / 1,242 blocks to 7,664 bytes / 259 blocks. A twenty-sample Criterion run
changed the median from 165.23 µs to 9.5083 µs. The affected logical-work
boundary remained 2 units.

A three-iteration/one-warmup Wasm/npm comparison used base artifact
`03fc845ca362f661f8c33c6ba5c7a4e99537779f8628f94bdd8c6bdf7dfdfbc0`
(784,262 bytes) and implementation artifact
`9b77e4250dd5d88926ab17667754a31a2e25f76f80b54acfd5de9b7c6111a3d3`
(784,060 bytes). The public facade validated the exact `e` output and retained
the 1,750-byte payload; its low-sample time moved from 1.33 ms to 0.375 ms per
iteration and is recorded as a boundary snapshot rather than a statistically
powered claim.

Reproduce with:

```sh
CALCULATOR_ALLOCATION_ITERATIONS=1 \
  cargo run --profile bench -p calculator-core --features std \
    --example allocation_baseline -- approximate_euler
cargo bench -p calculator-core --bench representative_paths --features std \
  -- approximate_components/euler --sample-size 20
cargo run --profile bench -p calculator-core --features std \
  --example logical_work_baseline
corepack pnpm --dir packages/calculator run build:wasm
CALCULATOR_BENCH_ITERATIONS=3 CALCULATOR_BENCH_WARMUP=1 \
  corepack pnpm --silent --dir packages/calculator run benchmark
```

## Trigonometric common-denominator recurrence

At base commit `6bbfbf6`, unit-range sine and cosine represented every Taylor
term and partial sum as canonical Rationals, repeating multiplication, division,
and GCD normalization in each iteration. Commit `5406762` keeps the current term,
signed sum, and their shared cumulative denominator as BigInts. Only the final
partial sum and its first omitted-term neighbor are canonicalized. An exact
regression compares both series with the former Rational definitions across
zero, fractions, one, and both term-count parities.

On 2026-07-12 with `rustc 1.97.0`, deterministic one-calculation allocation and
twenty-sample Criterion measurements were:

| Case | Bytes | Blocks | Native median |
| --- | ---: | ---: | ---: |
| `sin(1)` | 33,695 / 20,007 | 1,673 / 755 | 140.69 / 67.962 µs |
| `cos(1)` | 33,581 / 19,893 | 1,667 / 749 | 132.03 / 60.761 µs |

The affected logical-work boundary remained 200,133 units for both paths. A
three-iteration/one-warmup Wasm/npm boundary comparison used base artifact
`ae837ff837a7d3241d7807b2b7d8fb978835cebf4d6da148af70acc1c7eddbb7`
(784,287 bytes) and implementation artifact
`03fc845ca362f661f8c33c6ba5c7a4e99537779f8628f94bdd8c6bdf7dfdfbc0`
(784,262 bytes). The public facade validated `sin(1)` and `cos(1)` exact outputs
and retained their 1,766-byte payloads. The low-sample Wasm timings moved with
host-wide noise, so they are retained as integration snapshots without a speed
claim.

Reproduce with:

```sh
for case in approximate_sin_one approximate_cos_one
do
  CALCULATOR_ALLOCATION_ITERATIONS=1 \
    cargo run --profile bench -p calculator-core --features std \
      --example allocation_baseline -- "$case"
done
cargo bench -p calculator-core --bench representative_paths --features std \
  -- approximate_components/sin_one --sample-size 20
cargo bench -p calculator-core --bench representative_paths --features std \
  -- approximate_components/cos_one --sample-size 20
cargo run --profile bench -p calculator-core --features std \
  --example logical_work_baseline
corepack pnpm --dir packages/calculator run build:wasm
CALCULATOR_BENCH_ITERATIONS=3 CALCULATOR_BENCH_WARMUP=1 \
  corepack pnpm --silent --dir packages/calculator run benchmark
```

## Arctangent common-denominator recurrence

At base commit `7e6e53a`, unit-range atan and the two reciprocal atan series in
Machin's pi formula normalized a Rational after every power update, odd
division, and partial-sum operation. Commit `d24bb3d` instead carries the power,
odd-factor product, common denominator, and signed sum as BigInts, then
canonicalizes only the final adjacent alternating-series bounds. An exact
regression compares the new helper with the former Rational definition for
zero, several proper fractions, one, and multiple term-count parities.

On 2026-07-12 with `rustc 1.97.0`, one public calculation produced these
deterministic before/after allocation totals:

| Case | Bytes | Blocks |
| --- | ---: | ---: |
| `atan(1/2)` | 80,774 / 25,638 | 3,976 / 980 |
| `atan(2)` | 444,698 / 81,754 | 11,173 / 1,783 |

Twenty-sample Criterion runs changed `atan(1/2)` from a 609.91 µs median to
62.214 µs and `atan(2)` from 4.615 ms to 956.04 µs. The larger-input path also
uses the optimized recurrence for both pi terms, accounting for its remaining
cost above the direct unit-range case. The affected-path logical-work boundaries
remained 31 units for `atan(1/2)` and 5 units for `atan(2)` before and after the
change. The other runner boundaries remained 231, 401216, 400447, 586, 582,
400234, and 932 because this internal representation change does not alter
evaluator accounting.

A three-iteration/one-warmup Wasm/npm boundary snapshot used artifact
`ae837ff837a7d3241d7807b2b7d8fb978835cebf4d6da148af70acc1c7eddbb7`
(784,287 bytes), compared with base artifact
`0d0c3b58324ae82d92d1a6eb4177252c46385ce5fa277d977ef398c8a7c7dede`
(784,623 bytes). Across three iterations, the public facade's `atan(1/2)` path
changed from 2.77 ms to 0.596 ms per iteration and `atan(2)` from 28.2 ms to
4.75 ms. Their serialized payloads remained 1,772 and 1,762 bytes respectively,
and the benchmark validates their exact symbolic output before recording a
sample. These low-sample Wasm results are integration snapshots, not
statistically powered timing claims.

Reproduce with:

```sh
for case in approximate_atan_half approximate_atan_two
do
  CALCULATOR_ALLOCATION_ITERATIONS=1 \
    cargo run --profile bench -p calculator-core --features std \
      --example allocation_baseline -- "$case"
done
cargo bench -p calculator-core --bench representative_paths --features std \
  -- approximate_components/atan_half --sample-size 20
cargo bench -p calculator-core --bench representative_paths --features std \
  -- approximate_components/atan_two --sample-size 20
cargo run --profile bench -p calculator-core --features std \
  --example logical_work_baseline
corepack pnpm --dir packages/calculator run build:wasm
CALCULATOR_BENCH_ITERATIONS=3 CALCULATOR_BENCH_WARMUP=1 \
  corepack pnpm --silent --dir packages/calculator run benchmark
```

## Structural reciprocal for negative exponential bounds

At base commit `e8fe092`, negative direct-range exponential endpoints calculated
a positive canonical Rational bound and then passed `1 / bound` through general
Rational division and GCD normalization. Commit `72b2352` swaps the already
coprime positive numerator and denominator directly, preserving reciprocal bound
direction and rejecting nonpositive helper inputs.

On 2026-07-12 with `rustc 1.97.0`, one `exp(-2)` public calculation moved from
18,822 bytes / 708 blocks to 18,502 bytes / 696 blocks. Positive `exp(2)` remained
17,897 / 657. Binary-scaled `exp(-10000)` remained 565,504 / 2,166 because its
dyadic exponent-shift path does not use the direct Rational reciprocal. Logical
work remained unchanged. A three-iteration/one-warmup Wasm snapshot used artifact
`4ed8ae956c3415be1e2c84de675e63f60a28ec0c45acd7e7e121f14c3d58d296`
(784,630 bytes), with unchanged representative payloads; timing is retained only
as a boundary snapshot.

Reproduce with the allocation runner cases `approximate_exp_negative_two`,
`approximate_exp_negative_10000`, `approximate_exp_two`, and
`approximate_exp_positive_10000`, followed by the documented logical-work and
Wasm benchmark commands above.

## Canonical reciprocals in interval evaluation

Commit `fdd20b0` generalizes the structural reciprocal from positive exponential
bounds to signed nonzero canonical Rationals, preserving a positive denominator
by flipping both signs for negative inputs. Interval reciprocal and `atan(x>1)`
therefore avoid general division/GCD. The new `atan(2)` public allocation case
moved from 444,794 bytes / 11,185 blocks to 444,698 / 11,173. The small delta is
recorded without a timing claim; directed bounds, logical-work, and public DTOs
are unchanged.

## Common-denominator logarithm recurrence

At base commit `e42c235`, reduced logarithm evaluation represented every
`z^(2k+1)/(2k+1)` as a canonical Rational, multiplied the next power, divided by
the odd index, and canonicalized the growing sum on every iteration. Commit
`31d6973` maintains the power numerator, product of odd denominators, sum numerator,
and common denominator as BigInts, then canonicalizes only the final lower and
upper bounds. The existing geometric tail and term-count inequality are unchanged.

On 2026-07-12 with `rustc 1.97.0`, controlled base/implementation measurements
were:

| Case | Bytes | Blocks | Native median |
| --- | ---: | ---: | ---: |
| `ln(2)` | 37,133 / 21,141 | 1,674 / 806 | 295.11 / 65.70 µs |
| `2^sqrt(2)` | 242,158 / 226,166 | 3,771 / 2,903 | 970.26 / 748.22 µs |
| `sqrt(2)*ln(2)` | 125,194 / 109,202 | 4,905 / 4,037 | not sampled |
| `exp(sqrt(2)*ln(2))` | 276,110 / 260,118 | 5,287 / 4,419 | not sampled |

The paired median estimates show approximately 78% improvement for `ln(2)` and
23% for general power. Logical-work boundaries remained 231, 401216, 400447, 586, 582, 400234,
and 932. A three-iteration/one-warmup Wasm/npm boundary snapshot used artifact
`f9c6c7a6ebbe09bd65540a45cdff59aeb9d38352b25c7c5604dec804167b63bb`
(784,578 bytes), measured approximate evaluation at 6.28 ms/iteration, and kept
the 1,812-byte payload.

Reproduce with:

```sh
for case in approximate_log_two approximate_general_power \
  approximate_power_log_product approximate_exp_power_log_product
do
  CALCULATOR_ALLOCATION_ITERATIONS=1 \
    cargo run --profile bench -p calculator-core --features std \
      --example allocation_baseline -- "$case"
done
cargo bench -p calculator-core --bench representative_paths --features std \
  -- approximate_components/log_two --sample-size 20
cargo bench -p calculator-core --bench representative_paths --features std \
  -- approximate_components/general_power --sample-size 20
cargo run --profile bench -p calculator-core --features std \
  --example logical_work_baseline
corepack pnpm --dir packages/calculator run build:wasm
CALCULATOR_BENCH_ITERATIONS=3 CALCULATOR_BENCH_WARMUP=1 \
  corepack pnpm --silent --dir packages/calculator run benchmark
```

## Primitive transcendental series-bound operands

At base commit `026ce83`, the term-count and tail helpers for exponential,
logarithm, trigonometric series, Euler-number enclosure, and reciprocal arctangent converted bounded
indices and fixed factors to owned BigInts before multiplying arbitrary-precision
factorials, powers, or denominators. Commits `e13da03` and `385b018` use
num-bigint's exact primitive scalar multiplication while retaining every checked
index calculation and stopping inequality.

On 2026-07-12 with `rustc 1.97.0`, deterministic one-calculation allocations were:

| Case | Bytes | Blocks |
| --- | ---: | ---: |
| `exp(1)` | 18,213 / 17,125 | 667 / 633 |
| `ln(2)` | 39,597 / 37,133 | 1,751 / 1,674 |
| `sin(1)` | 35,999 / 33,695 | 1,745 / 1,673 |
| `2^sqrt(2)` | 246,798 / 242,158 | 3,916 / 3,771 |

Native timing moved with host-wide load and did not establish a speedup; this
slice claims only the deterministic allocation reduction. Logical-work boundaries
remained 231, 401216, 400447, 586, 582, 400234, and 932. The three-iteration,
one-warmup Wasm/npm boundary snapshot used artifact
`5c9d838ff3ecb5bcc7f38e7187d8f907c2e587baa1f57361f1f05bf9b4e54908`
(784,386 bytes), measured the approximate case at 8.31 ms/iteration, and retained
the unchanged 1,812-byte payload.

Reproduce with:

```sh
for case in approximate_exp_one approximate_log_two approximate_sin_one \
  approximate_general_power
do
  CALCULATOR_ALLOCATION_ITERATIONS=1 \
    cargo run --profile bench -p calculator-core --features std \
      --example allocation_baseline -- "$case"
done
cargo bench -p calculator-core --bench representative_paths --features std \
  -- approximate_components/exp_one --sample-size 20
cargo bench -p calculator-core --bench representative_paths --features std \
  -- approximate_components/log_two --sample-size 20
cargo bench -p calculator-core --bench representative_paths --features std \
  -- approximate_components/general_power --sample-size 20
cargo run --profile bench -p calculator-core --features std \
  --example logical_work_baseline
corepack pnpm --dir packages/calculator run build:wasm
CALCULATOR_BENCH_ITERATIONS=3 CALCULATOR_BENCH_WARMUP=1 \
  corepack pnpm --silent --dir packages/calculator run benchmark
```

## Exponential recurrence product and buffer reuse

At base commit `3180b15`, each exponential-series iteration computed
`term * value_numerator` once as the sum correction and then repeated the same
arbitrary-precision multiplication to update `term`. It also replaced the owned
sum with a newly formed multiply-add result. Commit `11ab4ce` computes the next
term once, moves it into the recurrence state after adding it, and multiplies/adds
into the owned sum buffer. The upper-tail numerator similarly adds its correction
into the owned product.

On 2026-07-12 with `rustc 1.97.0`, deterministic one-calculation allocations and
20-sample native timings changed as follows:

| Case | Bytes | Blocks | Native median |
| --- | ---: | ---: | ---: |
| `exp(1)` | 18,621 / 18,213 | 702 / 667 | 31.66 / 20.43 µs |
| `exp(2)` | 19,393 / 18,985 | 726 / 691 | not sampled |
| `2^sqrt(2)` | 285,246 / 246,798 | 3,984 / 3,916 | 1.274 ms / 700.29 µs |
| `exp(sqrt(2)*ln(2))` | 319,198 / 280,750 | 5,500 / 5,432 | not sampled |
| `exp(-10000)` | 656,720 / 613,040 | 4,464 / 4,390 | 2.485 / 1.446 ms |

Criterion reported statistically significant improvements of about 29%, 43%,
and 40% for the three sampled rows. Logical-work boundaries remained 231,
401216, 400447, 586, 582, 400234, and 932. A three-iteration/one-warmup Wasm/npm
snapshot used artifact
`5319b5c4fed787e15f7629bba2f9f6c45e7e3d1f8540f3dd8adc893fc56cbb34`
(784,475 bytes); approximate evaluation measured 6.36 ms/iteration and preserved
the 1,812-byte payload. The Wasm sample is recorded as a boundary snapshot rather
than a statistically powered before/after claim.

Reproduce with:

```sh
for case in approximate_exp_one approximate_exp_two approximate_general_power \
  approximate_exp_power_log_product approximate_exp_negative_10000
do
  CALCULATOR_ALLOCATION_ITERATIONS=1 \
    cargo run --profile bench -p calculator-core --features std \
      --example allocation_baseline -- "$case"
done
cargo bench -p calculator-core --bench representative_paths --features std \
  -- approximate_components/exp_one --sample-size 20
cargo bench -p calculator-core --bench representative_paths --features std \
  -- approximate_components/general_power --sample-size 20
cargo bench -p calculator-core --bench representative_paths --features std \
  -- approximate_components/exp_negative_10000 --sample-size 20
cargo run --profile bench -p calculator-core --features std \
  --example logical_work_baseline
corepack pnpm --dir packages/calculator run build:wasm
CALCULATOR_BENCH_ITERATIONS=3 CALCULATOR_BENCH_WARMUP=1 \
  corepack pnpm --silent --dir packages/calculator run benchmark
```

## Shared exact-point binary exponential plan

Large exact dyadic exponentials previously invoked the binary-scaled endpoint
evaluator independently for lower and upper bounds. Both calls rebuilt the same
working precision, certified `ln(2)` pair, midpoint quotient, and binary exponent.
Exact points now share that immutable plan while retaining direction-specific
residuals, Taylor bounds, dyadic rounding, and exponent application. Non-degenerate
endpoints retain independent planning because their guard precision can differ.

On 2026-07-13 with `rustc 1.97.0`, one public `exp(-10000)` calculation moved
from 531,048 bytes in 2,062 blocks to 526,296 bytes in 1,848 blocks; `exp(10000)`
moved from 503,752 bytes in 1,988 blocks to 499,032 bytes in 1,778 blocks.
Ten-sample Criterion midpoints moved from 3.271 ms to 2.889 ms for the negative
case and from 3.240 ms to 2.492 ms for the positive case (about 12% and 23% lower).
Logical-work charging remains the conservative public boundary rather than an
implementation-operation count.

The shared plan originally left one precision-only operation duplicated: each
direction independently derived the Taylor term count from the same working
precision. Storing that count in the exact-point plan removes the second
factorial search without sharing the direction-specific residual or recurrence.
Against commit `7ac4123`, one `exp(-10000)` calculation moved from 526,296 bytes
in 1,848 blocks to 526,216 bytes in 1,844 blocks; `exp(10000)` moved from 499,032
bytes in 1,778 blocks to 498,952 bytes in 1,774 blocks. A 20-sample Criterion run
measured 1.944 ms and 1.745 ms midpoints respectively, but timing is treated as a
noisy snapshot rather than attributed wholly to this small allocation change.
Logical work remains 586 and 582 units.

## Reused exponential factorial plan

Taylor term selection proves its tail bound by constructing `(N+1)!`. The
selected `N!` was then discarded and rebuilt for the recurrence denominator
`b^N*N!`. The precision-only series plan now retains `N!` alongside `N` and
passes it to exact-point, non-degenerate, and binary-scaled recurrence setup.
Endpoint-specific denominator powers, sums, terms, tails, and directed rounding
remain independent.

Against commit `28af12e`, one public `2^sqrt(2)` calculation moved from 171,078
bytes in 2,248 blocks to 171,022 bytes in 2,245 blocks. The cumulative
`exp(sqrt(2)*ln(2))` case moved from 205,030 / 3,764 to 204,974 / 3,761.
`exp(-10000)` moved from 526,216 / 1,844 to 526,160 / 1,841, and `exp(10000)`
from 498,952 / 1,774 to 498,896 / 1,771. The `exp(1)` and `exp(2)` controls were
unchanged at 17,133 / 634 and 17,865 / 656. General-power peak live allocation
increased by 24 bytes because the small factorial is retained through denominator
setup; large-exp peak values were unchanged. Logical work remains 400,447, 586,
and 582 units respectively.

A 20-sample Criterion snapshot measured 1.118 ms for direct general power,
912.52 µs for the cumulative exp stage, and 2.066 ms for `exp(-10000)`.
Criterion detected no change for direct general power or large exp; the cumulative
case improved against its cached control, but this small planning change claims
only the deterministic allocation reduction.

## In-place exponential term buffer

The common-denominator recurrence previously allocated `term*a` as a new BigInt,
moved it into the state, and dropped the preceding term buffer. Updating the owned
term in place preserves the exact recurrence while allowing ordinary-size products
to reuse capacity. Against commit `de0f7c1`, one public `exp(1)` calculation moved
from 17,133 bytes in 634 blocks to 16,861 bytes in 600 blocks. General power, its
cumulative exp stage, and `exp(±10000)` were byte-for-byte unchanged because their
growing term products still require fresh capacity. Peak live allocation and the
representative logical-work boundaries were unchanged. This slice therefore claims
deterministic allocation reduction for ordinary exact exponential evaluation only.
A 20-sample Criterion snapshot measured a 30.710 µs midpoint and reported an
improvement against its cached control; host-sensitive timing is not used to
broaden the claim beyond the deterministic allocation result.

## In-place directed exponential upper tail

An upper-only endpoint previously built fresh BigInt products for its corrected
sum and denominator even though the recurrence state was immediately discarded.
Consuming that state and updating its sum, term, and denominator buffers in place
reduced peak live allocation for one `2^sqrt(2)` calculation from 11,639 to 10,743
bytes, and for `exp(sqrt(2)*ln(2))` from 11,708 to 10,812 bytes. Total bytes and
blocks were unchanged, as were `exp(1)` and `exp(±10000)`. Paired exact bounds keep
the non-consuming tail path. Directed bounds and logical-work values are unchanged.

## Raw exponential fraction to directed dyadic rounding

The exponential Taylor recurrence already holds its final positive ratio as a
raw numerator and denominator. When that ratio is immediately rounded to the
public `ExactDyadic` interval, constructing a canonical `Rational` first performs
a GCD and exact divisions whose result does not affect floor/ceiling division.
The direct path now rounds the raw ratio for non-integral `0 < x <= 1` exact
points, non-degenerate endpoints, and binary-scaled residuals. Integer inputs
retain canonicalization because their recurrence ratio shares large factorial
factors and reducing those factors first makes the final division substantially
cheaper. Integer-power range reconstruction and negative-residual reciprocal
paths also retain canonical rationals.

Against commit `75bfc3b`, deterministic one-calculation allocation changed as
follows on 2026-07-13 with `rustc 1.97.0`:

| Case | Before | After |
| --- | ---: | ---: |
| `2^sqrt(2)` | 171,022 / 2,245 | 156,814 / 2,221 |
| `exp(sqrt(2)*ln(2))` | 204,974 / 3,761 | 190,766 / 3,737 |
| `exp(-10000)` | 526,160 / 1,841 | 512,640 / 1,814 |
| `exp(10000)` | 498,896 / 1,771 | 489,992 / 1,744 |

Peak live allocation for general power fell from 10,743 to 8,551 bytes; its
cumulative exp stage fell from 10,812 to 8,620. Large-exp peak values were
unchanged. Exact-output regressions compare the direct result with the former
canonical route at 64 and 128 bits for ordinary exact values, non-degenerate
positive/negative/mixed intervals, and binary-scaled `exp(±65)`/`exp(±10000)`.
`exp(1)` remains at 16,861 bytes / 600 blocks. An initial raw-integer variant
reduced its allocation by 440 bytes but regressed the Criterion midpoint from the
cached 30.7 µs to 53.1 µs, so that variant was rejected. The accepted path measured
32.8 µs for the unchanged `exp(1)` control, 630 µs for general power, and 1.00 ms
for `exp(-10000)` in 10-sample snapshots. The two target cases improved against
their cached controls. This slice claims the deterministic allocation reduction;
directed bounds, precision, logical-work charging, and public protocol are
unchanged.

## Dyadic exponential recurrence denominator shifts

For a dyadic Taylor input denominator `b = 2^k`, every recurrence step formerly
allocated `b*n` and used it as a general BigInt multiplier for the growing sum.
The state now detects `k` once without allocating and performs the equivalent
`sum *= n; sum <<= k`. Non-dyadic denominators retain the general product. The
same non-allocating power-of-two predicate replaces the allocating comparison in
the existing final common-denominator plan.

Against commit `7b70b4d`, deterministic one-calculation allocation changed as
follows on 2026-07-13 with `rustc 1.97.0`:

| Case | Before | After |
| --- | ---: | ---: |
| `exp(1)` | 16,861 bytes / 600 blocks | 16,581 / 565 |
| `2^sqrt(2)` | 156,814 / 2,221 | 151,926 / 2,098 |
| `exp(sqrt(2)*ln(2))` | 190,766 / 3,737 | 185,878 / 3,614 |
| `exp(-10000)` | 512,640 / 1,814 | 512,480 / 1,808 |
| `exp(10000)` | 489,992 / 1,744 | 489,816 / 1,738 |

Peak live allocation was unchanged. Multiplying by the primitive index before
shifting saved a further 232 bytes and one block in each general-power case over
shift-before-multiply, without changing the other cases. Applying the same split
to tail construction saved 128 bytes and two blocks plus 32 peak bytes for general
power, but added 96 bytes/two blocks to `exp(-10000)` and 32 bytes/one block to
`exp(10000)`; that mixed, broader variant was rejected in favor of the loop-only
change. Exact regressions compare lower and upper results with the old general
factor recurrence for integer, dyadic, and non-dyadic inputs. Directed bounds,
logical-work charging, and public protocol are unchanged. Ten-sample Criterion
midpoints moved from the preceding 32.8 µs snapshot to 23.7 µs for `exp(1)`, from
630 µs to 204 µs for general power, and from 1.00 ms to 232 µs for
`exp(-10000)`. These host-sensitive snapshots agree with the structural removal
of the large general multiplications; deterministic allocation remains the
reproducible comparison.

## Shared binary logarithm composition

Non-degenerate logarithm intervals require endpoint-specific range reduction and
directed reduced-series bounds. When both binary exponents are nonzero, the
endpoint evaluators previously rebuilt the same input-independent `ln(2)` series.
The pair now builds one certified enclosure and selects its side from each exponent
sign and requested direction. If only one exponent is nonzero, that endpoint keeps
the single directed `ln(2)` path; zero exponents skip composition.

On 2026-07-13 with `rustc 1.97.0`, `ln(2+sin(1))` moved from 351,388 bytes in
1,783 blocks to 349,612 bytes in 1,645 blocks for one public calculation. A
10-sample Criterion run moved from a 5.204 ms midpoint to 3.647 ms (about 30%
lower). Endpoint-pair regressions cover negative, zero, positive, mixed, and
different binary exponents against the former independent directed evaluator.
The remaining dominant work is endpoint-specific reduced-series arithmetic.

## Unit-numerator reduced logarithm recurrence

For the positive-series variable `z=a/b`, a unit numerator makes every unweighted
power numerator exactly one. The recurrence now selects a dedicated loop once and
adds the existing odd-denominator product directly to the scaled sum, avoiding
`term *= 1` and the temporary `term*odd_product`. This applies to every `z=1/b`,
not only the `ln(2)` value `1/3`. Nonunit and zero numerators retain the former
loop byte-for-byte; upper-tail construction remains shared and unchanged.

Against commit `b66ce00`, deterministic one-calculation allocation changed as
follows on 2026-07-13 with `rustc 1.97.0`:

| Case | Before | After |
| --- | ---: | ---: |
| `ln(2)` | 20,749 bytes / 766 blocks | 20,197 / 728 |
| `ln(2+sin(1))` | 349,612 / 1,645 | 349,060 / 1,607 |
| `2^sqrt(2)` | 151,926 / 2,098 | 151,374 / 2,060 |
| `sqrt(2)*ln(2)` | 91,226 / 3,448 | 90,674 / 3,410 |
| `exp(sqrt(2)*ln(2))` | 185,878 / 3,614 | 185,326 / 3,576 |
| `exp(-10000)` | 512,480 / 1,808 | 511,776 / 1,765 |
| `exp(10000)` | 489,816 / 1,738 | 489,112 / 1,695 |

`exp(1)` remained at 16,581 bytes / 565 blocks, and peak live allocation was
unchanged in every case. Twenty-sample reruns detected no timing change for the
`exp(1)` control or `ln(2)`; host load varied substantially during the much longer
non-degenerate log sample, so this slice claims only deterministic allocation.
Logical work was unchanged: `approximate` remained 400,447 units,
`log_non_degenerate` 200,225, `exp_negative_10000` 586, and
`exp_positive_10000` 582.
Exact tests cover `z=1/3`, `1/4`, `1/5`, `1/7`, nonunit `3/10`, zero, term counts
0/1/5/20, and the 64/128-bit term plans against the former general recurrence and
the independent Rational definition.

Two broader alternatives were rejected before this slice. Folding the odd product
into the stored term reduced non-degenerate-log allocation from 349,612 to 280,836
bytes but changed `ln(2)` from 82.9 to 104.8 µs and the non-degenerate log from
4.43 to 6.18 ms in same-load base/candidate runs because subsequent general
multiplications consumed a larger term. Explicitly consuming buffers in the upper
tail produced byte-, block-, and peak-identical measurements; a 20-sample run
measured 101.1 µs and 5.87 ms against base values 91.1 µs and 4.92 ms. Neither
variant is retained.

## Shared non-degenerate logarithm term plan

After endpoint range reduction, the reduced logarithm term count depends only on
requested precision. At base commit `7db9fd2`, a non-degenerate interval repeated
that same integer-only plan for its lower directed series, upper directed series,
and any required shared or single `ln(2)` composition. The endpoint pair now
computes it once and passes it to each value-dependent recurrence. Exact-point
paired evaluation is intentionally unchanged.

On 2026-07-13 with `rustc 1.97.0`, deterministic one-calculation allocation for
`ln(2+sin(1))` moved from 349,060 bytes / 1,607 blocks to 347,780 / 1,517;
peak live allocation remained 16,982 bytes. Controls were byte-, block-, and
peak-identical: `ln(2)` 20,197 / 728, `2^sqrt(2)` 151,374 / 2,060,
`sqrt(2)*ln(2)` 90,674 / 3,410, `exp(sqrt(2)*ln(2))` 185,326 / 3,576,
`exp(-10000)` 511,776 / 1,765, `exp(10000)` 489,112 / 1,695, and
`exp(1)` 16,581 / 565. A same-host ten-sample Criterion comparison was noisy:
the base and candidate non-degenerate-log medians were approximately 4.365 ms
and 4.300 ms with overlapping variation, so this slice claims deterministic
allocation reduction rather than timing speedup.

Logical work was unchanged: `approximate` remained 400,447 units,
`log_non_degenerate` 200,225, `exp_negative_10000` 586, and
`exp_positive_10000` 582. Exact regressions compare against the former independent
directed evaluator for negative, zero, positive, and mixed binary exponents,
unit reduced endpoints, multiple precisions, and the zero/error preflight order.
Reproduce with allocation case `approximate_log_non_degenerate`, Criterion case
`approximate_components/log_non_degenerate`, and `logical_work_baseline`.

## Canonical logarithm range scaling

Logarithm range reduction repeatedly halves values at or above two and doubles
values below one. For canonical `n/d`, parity determines the only possible factor
of two cancellation: halving uses `(n/2)/d` when `n` is even and `n/(2d)`
otherwise; doubling uses `n/(d/2)` when `d` is even and `(2n)/d` otherwise. The
range reducer now constructs those already-canonical forms directly instead of
running a general GCD and exact division at every step.

The representative native, allocation, logical-work, and Wasm/npm harnesses now
include `ln(340282366920938463463374607431768211457)`, a non-power integer just
above `2^128` that requires 128 halving steps without simplifying to an integer
multiple of `ln(2)`. On 2026-07-13 with `rustc 1.97.0`, deterministic one-call
allocation changed as follows:

| Case | Before | After |
| --- | ---: | ---: |
| large positive log | 179,401 bytes / 1,927 blocks | 170,153 / 1,415 |
| `ln(2+sin(1))` | 347,780 / 1,517 | 347,588 / 1,509 |
| `ln(2)` | 20,197 / 728 | 20,165 / 724 |
| `2^sqrt(2)` | 151,374 / 2,060 | 151,342 / 2,056 |
| `exp(-10000)` | 511,776 / 1,765 | unchanged |
| `exp(10000)` | 489,112 / 1,695 | unchanged |

Peak live allocation was unchanged. A same-host saved-baseline Criterion run was
noisy for the new large-log case (5.114 ms base midpoint, 5.608 ms candidate,
confidence intervals overlapping and `p=0.43`) and therefore supports no timing
claim for that case. The same run classified `ln(2)` and the non-degenerate log
as improved, with midpoint changes of about 41% and 22%; deterministic allocation
is the reproducible claim. Logical work remains 200,225 units for the
non-degenerate log and is 21 units for the new large-log case.
The one-iteration Wasm/npm boundary smoke used artifact
`2d4b563c30964faaf8c2566e511efddf5b6b349ebdbaf0b63d80447f26e9f3c8`
(811,498 bytes); the new case completed in 22.3 ms with a 1,834-byte payload.
This cold single sample verifies the facade and payload path and is not a timing
comparison.

Structural regressions compare both parity branches and multi-step reductions
against general Rational multiplication/division. A rejected alternative built
the nonunit series common denominator once with a final power. It reduced the
non-degenerate-log allocation from 347,780 to 314,628 bytes but a saved-baseline
run regressed the Criterion midpoint from 4.53 ms to 5.11 ms (about 12%,
`p=0.01`), so it is not retained.

Reproduce with allocation cases `approximate_log_large_positive`,
`approximate_log_non_degenerate`, `approximate_log_two`, and the existing exp
controls; Criterion cases are under `approximate_components`. Run
`logical_work_baseline` and the package benchmark to cover deterministic work and
the Wasm/npm boundary.

## Hybrid binary splitting for nonunit logarithms

For nonunit `z=a/b`, let `x=a^2`, `y=b^2`, and define the positive-series
ratios `p_k=x(2k-1)` and `q_k=y(2k+1)`. A contiguous segment stores product
`P/Q` and cumulative-product sum `T/Q`. Adjacent segments combine exactly as
`P=P_l P_r`, `Q=Q_l Q_r`, and `T=T_l Q_r + P_l T_r`. The final lower bound is
`2a(Q+T)/(bQ)`; the existing first-omitted-term upper guarantee is reconstructed
from the same product without a division or floating-point approximation.

Fully balanced one-term leaves reduced total bytes but created many small blocks
and regressed the 128-bit public-path Criterion midpoint by about 22%. The retained
hybrid builds sequential chunks of at most 32 terms with reusable owned buffers
and balances only the chunks. It is selected for nonzero, nonunit series with more
than 32 planned recurrence terms. Zero, the unit-numerator specialization, and
smaller plans retain their previous loops. Upper-tail products are deferred until
an upper bound is requested, preserving directed lower-only evaluation.

Against `12fa2b1` on 2026-07-13 with `rustc 1.97.0`, one public
`ln(2+sin(1))` calculation changed from 347,588 bytes / 1,509 blocks to
278,612 bytes / 1,721 blocks. Total bytes fell by about 19.8%; the larger block
count reflects short-lived chunk temporaries, while peak live allocation remained
16,982 bytes / 43 blocks. Allocation controls were unchanged: `ln(2)`
20,165 / 724, `2^sqrt(2)` 151,342 / 2,056, `sqrt(2)*ln(2)` 90,642 / 3,406,
`exp(sqrt(2)*ln(2))` 185,294 / 3,572, `exp(-10000)` 511,776 / 1,765,
`exp(10000)` 489,112 / 1,695, and `exp(1)` 16,581 / 565.

A same-host saved-baseline ten-sample Criterion comparison at `c8d4411` moved the
non-degenerate-log midpoint from 4.272 ms to 3.634 ms (about 14.9%); Criterion's
distribution estimate was a 19.4% reduction (`p<0.01`).
`ln(2)` had no detected change. A three-iteration/one-warmup Wasm/npm comparison
at `c8d4411`, using separate builds of that revision with the compile-time
threshold disabled/enabled moved the non-degenerate case from 45.1 to 30.6 ms per
iteration. Payload size remained
1,772 bytes and retained JS heap remained 9,568 bytes. That timing artifact was
813,416 bytes. After the directed lower-tail review correction, the final
`945bfb3` example/package build produced
`7cac7fc1f881194390f70c00097090ad4624fac84d4360bd13b093ef50effb6d`
(814,038 bytes), 2,540 bytes (about 0.31%) above the `12fa2b1` artifact's
811,498 bytes and below the 860,000-byte package budget. The final example build
and package-size gate used that same 814,038-byte artifact. Wasm timing is a small
same-host diagnostic from the stated measurement commit; native Criterion and
deterministic final-tip allocation provide the primary evidence.

Logical work remains 200,225 units for the non-degenerate log. Exact regressions
cover the old incremental recurrence at small, threshold, 64-, 128-, and 256-bit
term plans, directed and paired bounds, zero/unit dispatch, and checked oversized
term counts. Reproduce with allocation case `approximate_log_non_degenerate`,
Criterion case `approximate_components/log_non_degenerate`,
`logical_work_baseline`, and the package benchmark.

## Hybrid binary splitting for nonunit arctangent series

At base commit `19ea116`, the nonunit unit-range arctangent recurrence multiplied
every growing partial sum by the next denominator factor. The hybrid evaluator
keeps the exact signed ratio `p_k=-a^2(2k-1)`, `q_k=b^2(2k+1)` and merges segment
products and cumulative-product sums above a 32-term sequential leaf. Zero, unit
numerators, and smaller plans retain the recurrence. Directed bounds that select
the current alternating partial sum omit the adjacent-only root product.

On 2026-07-13 with `rustc 1.97.0`, deterministic one-calculation allocation
changed from 710,074 bytes / 1,983 blocks to 528,330 / 2,194 for
`atan(2+sin(1))`, from 1,473,476 / 4,073 to 1,166,372 / 4,422 for transformed
`asin`, and from 1,641,406 / 4,166 to 1,334,302 / 4,515 for transformed `acos`.
Total bytes fell by about 25.6%, 20.8%, and 18.7%; short-lived tree temporaries
increased block counts while peak live allocation was unchanged. Unit/small
controls remained 22,046 / 820 for `atan(1/2)` and 49,402 / 1,145 for `atan(2)`.

A full-balanced one-term-leaf experiment reduced the target further to 395,114
bytes / 2,666 blocks, but its Criterion midpoint was 15.780 ms, about 13.9%
slower than the selected 32-term leaf's 13.853 ms. It was rejected because the
extra product-tree temporaries traded away most of the target's timing gain.

A saved-baseline ten-sample Criterion comparison moved the target midpoint from
16.087 ms to 13.853 ms (about 13.9%). Its distribution estimate was a nonsignificant
6.3% reduction (`p=0.29`), so the claim is deterministic allocation reduction
without a detected timing regression. The unit/small controls do not use the new
dispatch; their noisy movement is not attributed to this change. Logical work
remained 200,225 units for atan and 200,447 for each transformed inverse function.

The three-iteration/one-warmup Wasm/npm runs used base artifact
`7cac7fc1f881194390f70c00097090ad4624fac84d4360bd13b093ef50effb6d`
(814,038 bytes) and candidate artifact
`00a75a0925f83f664b5c1ffe23ac15ef2dd20cd192a15ed2ebb8e328c5ba76ca`
(816,896 bytes), below the 860,000-byte budget. Broad control movement showed host
contention, so these runs establish boundary integration, payload stability, and
size rather than a Wasm speed claim. Target payloads remained 1,776 bytes for atan,
1,788 for asin, and 1,794 for acos. Exact oracle tests cover threshold-adjacent
and 64/128/256-bit plans, paired/directed parity, legacy recurrence agreement,
zero/unit/small dispatch, and checked term-count overflow.

## Unit-numerator arctangent recurrence

At base commit `5a2b612`, `atan(1/b)` still multiplied its arbitrary-precision
term numerator by one and then multiplied that one by the growing odd product on
every recurrence step. The specialized paired and directed loops use the odd
product directly as the alternating correction. Nonunit binary splitting,
sum/adjacent parity, reciprocal transformation, and shared Machin π are unchanged.

On 2026-07-13 with `rustc 1.97.0`, deterministic one-calculation allocation
changed from 22,046 bytes / 820 blocks to 20,774 / 760 for `atan(1/2)`, from
49,402 / 1,145 to 45,586 / 965 for `atan(2)`, from 528,330 / 2,194 to
525,786 / 2,074 for non-degenerate atan, and from 654,810 / 1,799 to
531,202 / 1,577 for the Machin-π-using `acos(1/3)`. Peak live allocation did not
increase. Logical work remained 31, 5, 200,225, and 31 units respectively.
The semantically unaffected direct-series `asin(1/3)` control moved from
485,388 / 1,293 to 438,548 / 1,260 in the candidate artifact, but its stack does
not enter atan or Machin π; that artifact-level code-generation movement is not
attributed to this specialization.

Same-source saved-baseline Criterion runs were dominated by host drift: the later
run also moved the unchanged nonunit atan control upward about 10%. Alternating
base/candidate Wasm processes produced overlapping ranges for `atan(1/2)` and
`asin(1/3)`, so no timing claim is made; the accepted claim is the deterministic
allocation and block reduction from removing two provably redundant operations.
The alternating timing smoke used base Wasm artifact
`021ce40894287cc85e21e86a2755fb1691d3ef40fafa73fabed27de712005618`
(816,896 bytes) and pre-review candidate
`ff91afb27c59c7f369d83d7e412e9f644382427c9c7cde85077904e91f342a39`
(818,499 bytes). After adding the checked overflow preflight, the final candidate
package/example artifact was
`63f5b763653137b83e035bb75733d353626d9a3980646616803a51b36eea4f89`
(818,486 bytes), 1,590 bytes (about 0.19%) above base and below the 860,000-byte
budget. Payloads remained 1,772 bytes. The Wasm benchmark now accepts
`CALCULATOR_BENCH_CASE` to make alternating focused boundary measurements
reproducible without changing case definitions.

## Primitive exponential recurrence indices

At base commit `5506090`, the common-denominator exponential recurrence converted
every `u32` term index to an owned BigInt before multiplying it into the dyadic
input denominator. The upper tail did the same for `N+1` and the constant two.
These operands are bounded primitives; only the growing numerator and denominator
state requires arbitrary precision. Commit `7590b24` passes the primitive values
directly to num-bigint multiplication without changing the recurrence or tail.

On 2026-07-12 with `rustc 1.97.0`, one public calculation produced these
deterministic before/after allocation totals:

| Case | Bytes | Blocks |
| --- | ---: | ---: |
| `exp(1)` | 19,773 / 18,621 | 738 / 702 |
| `exp(2)` | 20,545 / 19,393 | 762 / 726 |
| `2^sqrt(2)` | 287,486 / 285,246 | 4,054 / 3,984 |
| `exp(sqrt(2)*ln(2))` | 321,438 / 319,198 | 5,570 / 5,500 |
| `exp(-10000)` | 659,152 / 656,720 | 4,540 / 4,464 |

Twenty-sample Criterion runs detected no significant change for `exp(1)` or
`exp(-10000)` and classified general-power movement within the configured noise
threshold, so this slice claims allocation reduction rather than timing speedup.
Logical-work boundaries remained 231, 401216, 400447, 586, 582, 400234, and 932.
A three-iteration/one-warmup Wasm/npm snapshot used artifact
`73be1fb55a894b6c2b26bc8265f29256da90e179a4a9deefa461863dcfddca55`
(784,484 bytes); approximate evaluation measured 7.14 ms/iteration and retained
the unchanged 1,812-byte payload.

Reproduce with:

```sh
for case in approximate_exp_one approximate_exp_two approximate_general_power \
  approximate_exp_power_log_product approximate_exp_negative_10000
do
  CALCULATOR_ALLOCATION_ITERATIONS=1 \
    cargo run --profile bench -p calculator-core --features std \
      --example allocation_baseline -- "$case"
done
cargo bench -p calculator-core --bench representative_paths --features std \
  -- approximate_components/exp_one --sample-size 20
cargo bench -p calculator-core --bench representative_paths --features std \
  -- approximate_components/general_power --sample-size 20
cargo bench -p calculator-core --bench representative_paths --features std \
  -- approximate_components/exp_negative_10000 --sample-size 20
cargo run --profile bench -p calculator-core --features std \
  --example logical_work_baseline
corepack pnpm --dir packages/calculator run build:wasm
CALCULATOR_BENCH_ITERATIONS=3 CALCULATOR_BENCH_WARMUP=1 \
  corepack pnpm --silent --dir packages/calculator run benchmark
```
