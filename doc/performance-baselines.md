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

The approximate composite is the dominant measured path in both environments.
Its Wasm facade time is roughly five times the native estimate on this run, so the
next profiling slice should separate interval refinement from DTO serialization
before choosing an optimization. A single retained-heap delta cannot establish a
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
