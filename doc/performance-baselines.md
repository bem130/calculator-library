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

## Region-selected exact asin transform

At base commit `56c3721`, exact rational `1/2 < x < 1/sqrt(2)` used
`pi/2-atan(sqrt(1-x^2)/x)`. Its atan argument exceeds one, so atan performed a
second pi/2 reciprocal transform and the two independently certified pi terms
were later cancelled. Commit `b12f7bf` selects the equivalent
`atan(x/sqrt(1-x^2))` form in this whole region using exact integer-square
comparison. The new directed interval is contained in the former enclosure for
both signs; the upper transform region retains exact endpoint equality. Paired
and directed endpoint evaluators use the same region selection.

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
