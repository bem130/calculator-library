import init, * as wasm from "../wasm/calculator_wasm.js";
import { readFile } from "node:fs/promises";
import {
    createCalculatorFromWasmModule,
    createSessionFromWasmModule,
    defaultCalculationRequest,
    defaultInputPolicy,
} from "../src/direct.ts";

const iterations = positiveInteger(process.env.CALCULATOR_BENCH_ITERATIONS, 50);
const warmup = nonNegativeInteger(process.env.CALCULATOR_BENCH_WARMUP, 5);

await init({
    module_or_path: await readFile(new URL("../wasm/calculator_wasm_bg.wasm", import.meta.url)),
});
const calculator = createCalculatorFromWasmModule(wasm);

const cases = [
    ["exact_rational", "12345678901234567890/7 + 98765432109876543210/11"],
    ["exact_symbolic", "(exp(1)+sin(1))*cos(1)-exp(1)*cos(1)"],
    ["approximate", "sin(1)+ln(2)+2^sqrt(2)"],
    ["algebraic", "((2^(1/3)-2^(1/3))+2)^(1/3)"],
    ["wide_add_256", Array.from({ length: 256 }, (_, index) => String(index + 1)).join("+")],
];

const results = [];
for (const [name, source] of cases) {
    results.push(measure(name, () => {
        const result = calculator.calculate(source, defaultCalculationRequest);
        if (result.tag !== "ok") throw new Error(`${name} failed: ${result.error.tag}.${result.error.code}`);
        return result;
    }));
}
results.push(measure("session_dispatch_sequence", dispatchSessionSequence));

process.stdout.write(`${JSON.stringify({
    schemaVersion: 1,
    runtime: { node: process.version, platform: process.platform, arch: process.arch },
    iterations,
    warmup,
    cases: results,
}, null, 2)}\n`);

function dispatchSessionSequence() {
    const session = createSessionFromWasmModule(wasm, defaultInputPolicy);
    try {
        for (const action of [
            { tag: "digit", value: 1 },
            { tag: "digit", value: 2 },
            { tag: "digit", value: 3 },
            { tag: "decimalPoint" },
            { tag: "digit", value: 4 },
            { tag: "digit", value: 5 },
            { tag: "percent" },
            { tag: "evaluate" },
        ]) {
            session.dispatch(action);
        }
        return session.getState();
    } finally {
        session.dispose?.();
    }
}

function measure(name, operation) {
    for (let index = 0; index < warmup; index += 1) operation();
    globalThis.gc?.();
    const heapBefore = process.memoryUsage().heapUsed;
    const started = performance.now();
    let value;
    for (let index = 0; index < iterations; index += 1) value = operation();
    const elapsedMilliseconds = performance.now() - started;
    globalThis.gc?.();
    const heapAfter = process.memoryUsage().heapUsed;
    if (value === undefined) throw new Error(`${name} produced no value`);
    return {
        name,
        elapsedMilliseconds,
        nanosecondsPerIteration: elapsedMilliseconds * 1_000_000 / iterations,
        retainedHeapBytes: heapAfter - heapBefore,
    };
}

function positiveInteger(value, fallback) {
    const parsed = value === undefined ? fallback : Number.parseInt(value, 10);
    if (!Number.isSafeInteger(parsed) || parsed <= 0) throw new Error("iterations must be a positive integer");
    return parsed;
}

function nonNegativeInteger(value, fallback) {
    const parsed = value === undefined ? fallback : Number.parseInt(value, 10);
    if (!Number.isSafeInteger(parsed) || parsed < 0) throw new Error("warmup must be a non-negative integer");
    return parsed;
}
