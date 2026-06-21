import type { ApiResult, CalculationOutcome } from "./generated/dto";
import type { CalculationRequest } from "./direct";

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

export async function createWorkerCalculator(): Promise<WorkerCalculator> {
    return {
        async calculate(_source, _request, options) {
            if (options.signal.tag === "abortSignal" && options.signal.signal.aborted) {
                return {
                    tag: "error",
                    error: {
                        tag: "unsupportedFeature",
                        code: "evaluationEngine",
                    },
                };
            }
            return {
                tag: "error",
                error: {
                    tag: "unsupportedFeature",
                    code: "evaluationEngine",
                },
            };
        },
        terminate() {
            return;
        },
    };
}
