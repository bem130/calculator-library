import type { ApiResult, CalculationOutcome } from "./generated/dto";
import {
    createCalculator,
    type CalculationRequest,
    type CreateCalculatorOptions,
} from "./index";

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

export async function createWorkerCalculator(
    options: CreateCalculatorOptions = {},
): Promise<WorkerCalculator> {
    const calculator = await createCalculator(options);
    return {
        async calculate(source, request, options) {
            if (options.signal.tag === "abortSignal" && options.signal.signal.aborted) {
                return {
                    tag: "error",
                    error: {
                        tag: "unsupportedFeature",
                        code: "evaluationEngine",
                    },
                };
            }
            return calculator.calculate(source, request);
        },
        terminate() {
            return;
        },
    };
}
