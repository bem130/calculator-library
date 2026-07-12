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

## Euler constant factorial denominator

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
