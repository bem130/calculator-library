import init, * as wasm from "../wasm/calculator_wasm.js";
import { readFile } from "node:fs/promises";
import { createHash } from "node:crypto";
import {
    createCalculatorFromWasmModule,
    createSessionFromWasmModule,
    defaultCalculationRequest,
    defaultInputPolicy,
} from "../src/direct.ts";

const iterations = positiveInteger(process.env.CALCULATOR_BENCH_ITERATIONS, 10);
const warmup = nonNegativeInteger(process.env.CALCULATOR_BENCH_WARMUP, 2);

const wasmBytes = await readFile(new URL("../wasm/calculator_wasm_bg.wasm", import.meta.url));
await init({ module_or_path: wasmBytes });
const calculator = createCalculatorFromWasmModule(wasm);

const cases = [
    ["exact_rational", "12345678901234567890/7 + 98765432109876543210/11", "827160492682716049260/77"],
    ["exact_symbolic", "(exp(1)+sin(1))*cos(1)-exp(1)*cos(1)", "sin(1)*cos(1)"],
    ["approximate", "sin(1)+ln(2)+2^sqrt(2)", "2^sqrt(2)+sin(1)+ln(2)"],
    ["euler", "e", "e"],
    ["exp_negative_10000", "exp(-10000)", "exp(-10000)"],
    ["exp_positive_10000", "exp(10000)", "exp(10000)"],
    ["atan_half", "atan(1/2)", "atan(1/2)"],
    ["atan_two", "atan(2)", "atan(2)"],
    ["asin_third", "asin(1/3)", "asin(1/3)"],
    ["acos_third", "acos(1/3)", "acos(1/3)"],
    ["sin_one", "sin(1)", "sin(1)"],
    ["cos_one", "cos(1)", "cos(1)"],
    ["tan_one", "tan(1)", "tan(1)"],
    ["algebraic", "((2^(1/3)-2^(1/3))+2)^(1/3)", "2^(1/3)"],
    ["wide_add_256", Array.from({ length: 256 }, (_, index) => String(index + 1)).join("+"), "32896"],
];

const results = [];
for (const [name, source, expectedExact] of cases) {
    results.push(measure(name, () => {
        const result = calculator.calculate(source, defaultCalculationRequest);
        if (result.tag !== "ok") throw new Error(`${name} failed: ${result.error.tag}.${result.error.code}`);
        const exact = result.value.calculation.exact;
        if (exact.tag !== "included" || exact.value.plainText !== expectedExact) {
            throw new Error(`${name} returned unexpected exact output`);
        }
        return result;
    }));
}
results.push(measure("session_dispatch_sequence", dispatchSessionSequence));

process.stdout.write(`${JSON.stringify({
    schemaVersion: 1,
    benchmarkDefinition: "representative-paths-v6",
    artifact: {
        wasmSha256: createHash("sha256").update(wasmBytes).digest("hex"),
        wasmBytes: wasmBytes.byteLength,
        gcExposed: typeof globalThis.gc === "function",
    },
    runtime: { node: process.version, platform: process.platform, arch: process.arch },
    iterations,
    warmup,
    cases: results,
}, null, 2)}\n`);

function dispatchSessionSequence() {
    const session = createSessionFromWasmModule(wasm, defaultInputPolicy);
    try {
        const actions = [
            { tag: "digit", value: 1 },
            { tag: "digit", value: 2 },
            { tag: "digit", value: 3 },
            { tag: "decimalPoint" },
            { tag: "digit", value: 4 },
            { tag: "digit", value: 5 },
            { tag: "percent" },
            { tag: "evaluate" },
        ];
        for (const [index, action] of actions.entries()) {
            const dispatch = session.dispatch(action);
            if (dispatch.tag === "inputError") {
                throw new Error(`session action ${index} failed: ${dispatch.error.code}`);
            }
            const expectedTag = index === actions.length - 1 ? "calculate" : "state";
            if (dispatch.tag !== expectedTag) {
                throw new Error(`session action ${index} returned ${dispatch.tag}, expected ${expectedTag}`);
            }
            if (dispatch.tag === "calculate" && dispatch.source !== "123.45%") {
                throw new Error(`session evaluate returned unexpected source ${dispatch.source}`);
            }
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
        payloadBytes: Buffer.byteLength(JSON.stringify(value), "utf8"),
    };
}

function positiveInteger(value, fallback) {
    const parsed = parseInteger(value, fallback);
    if (!Number.isSafeInteger(parsed) || parsed <= 0) throw new Error("iterations must be a positive integer");
    return parsed;
}

function nonNegativeInteger(value, fallback) {
    const parsed = parseInteger(value, fallback);
    if (!Number.isSafeInteger(parsed) || parsed < 0) throw new Error("warmup must be a non-negative integer");
    return parsed;
}

function parseInteger(value, fallback) {
    if (value === undefined) return fallback;
    if (!/^(0|[1-9][0-9]*)$/u.test(value)) throw new Error("benchmark counts must be canonical integers");
    return Number(value);
}
