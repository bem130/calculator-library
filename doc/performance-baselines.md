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
corepack pnpm --dir packages/calculator run benchmark > target/wasm-baseline.json
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
boundary leak/payload proxy.

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
| wide add (256 terms) | 293,710 | 11,124 | 110,358 | 1,892 |
| session dispatch sequence | 102 | 20 | 26 | 3 |

All six cases retained zero native bytes at process exit. Use the fixed case names
shown in the table when collecting comparisons; arbitrary source strings are not
accepted by the allocation runner.

The native session row measures reducer state creation and eight actions. The Wasm
row additionally includes Wasm session construction/disposal, DTO conversion,
dispatch, and final state retrieval; the rows diagnose their respective layers
rather than identical operation scope.
