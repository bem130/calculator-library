import type {
    ApiResult,
    CalculationOutcome,
    CalculationRequest,
} from "./generated/dto";
import type { CreateCalculatorOptions } from "./direct";

export interface WorkerCalculator {
    calculate(
        source: string,
        request: CalculationRequest,
        options: WorkerCalculationOptions,
    ): Promise<ApiResult<CalculationOutcome>>;

    terminate(): void;
}

export type WorkerCalculationOptions = {
    readonly signal: AbortSignalOption;
};

export type AbortSignalOption =
    | {
        readonly tag: "none";
    }
    | {
        readonly tag: "abortSignal";
        readonly signal: AbortSignal;
    };

export type CreateWorkerCalculatorOptions = Pick<
    CreateCalculatorOptions,
    "wasmGlueUrl" | "wasmModuleUrl"
>;

type WorkerRequestMessage = {
    readonly tag: "calculate";
    readonly id: number;
    readonly source: string;
    readonly request: CalculationRequest;
    readonly wasmGlueUrl?: string;
    readonly wasmModuleUrl?: string;
};

type WorkerResponseMessage =
    | {
        readonly tag: "result";
        readonly id: number;
        readonly result: ApiResult<CalculationOutcome>;
    }
    | {
        readonly tag: "thrown";
        readonly id: number;
    };

export async function createWorkerCalculator(
    options: CreateWorkerCalculatorOptions = {},
): Promise<WorkerCalculator> {
    const wasmGlueUrl = stringifyUrl(options.wasmGlueUrl);
    const wasmModuleUrl = stringifyUrl(options.wasmModuleUrl);
    const activeCalculations = new Set<ActiveWorkerCalculation>();
    let nextId = 1;
    let terminated = false;

    return {
        async calculate(source, request, options) {
            if (terminated) {
                return cancellationResult();
            }
            if (options.signal.tag === "abortSignal" && options.signal.signal.aborted) {
                return cancellationResult();
            }

            const id = nextId;
            nextId += 1;
            const worker = new Worker(new URL("./worker-thread.ts", import.meta.url), {
                type: "module",
            });

            return await new Promise<ApiResult<CalculationOutcome>>((resolve) => {
                let settled = false;
                const signal = options.signal.tag === "abortSignal"
                    ? options.signal.signal
                    : null;
                const activeCalculation: ActiveWorkerCalculation = {
                    cancel() {
                        finish(cancellationResult());
                    },
                };
                activeCalculations.add(activeCalculation);

                const finish = (result: ApiResult<CalculationOutcome>): void => {
                    if (settled) {
                        return;
                    }
                    settled = true;
                    worker.removeEventListener("message", handleMessage);
                    worker.removeEventListener("error", handleWorkerError);
                    signal?.removeEventListener("abort", handleAbort);
                    activeCalculations.delete(activeCalculation);
                    worker.terminate();
                    resolve(result);
                };

                const handleAbort = (): void => {
                    finish(cancellationResult());
                };

                const handleWorkerError = (event: ErrorEvent): void => {
                    event.preventDefault();
                    finish(cancellationResult());
                };

                const handleMessage = (event: MessageEvent<WorkerResponseMessage>): void => {
                    const message = event.data;
                    if (message.id !== id) {
                        return;
                    }
                    if (message.tag === "result") {
                        finish(message.result);
                    } else {
                        finish(cancellationResult());
                    }
                };

                worker.addEventListener("message", handleMessage);
                worker.addEventListener("error", handleWorkerError);
                signal?.addEventListener("abort", handleAbort, { once: true });

                const message: WorkerRequestMessage = {
                    tag: "calculate",
                    id,
                    source,
                    request,
                    ...(wasmGlueUrl === undefined ? {} : { wasmGlueUrl }),
                    ...(wasmModuleUrl === undefined ? {} : { wasmModuleUrl }),
                };
                worker.postMessage(message);
            });
        },
        terminate() {
            terminated = true;
            for (const activeCalculation of Array.from(activeCalculations)) {
                activeCalculation.cancel();
            }
            activeCalculations.clear();
        },
    };
}

type ActiveWorkerCalculation = {
    cancel(): void;
};

function cancellationResult(): ApiResult<CalculationOutcome> {
    return {
        tag: "error",
        error: {
            tag: "unsupportedFeature",
            code: "evaluationEngine",
        },
    };
}

function stringifyUrl(value: string | URL | undefined): string | undefined {
    return value === undefined ? undefined : String(value);
}
