import { createCalculator } from "./direct";
import type {
    ApiResult,
    CalculationOutcome,
    CalculationRequest,
} from "./generated/dto";

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

const scope = globalThis as unknown as {
    addEventListener(
        type: "message",
        listener: (event: MessageEvent<WorkerRequestMessage>) => void,
    ): void;
    postMessage(message: WorkerResponseMessage): void;
};

scope.addEventListener("message", (event) => {
    const message = event.data;
    if (message.tag !== "calculate") {
        return;
    }

    void calculateAndPost(message);
});

async function calculateAndPost(message: WorkerRequestMessage): Promise<void> {
    try {
        const calculator = await createCalculator({
            ...(message.wasmGlueUrl === undefined ? {} : { wasmGlueUrl: message.wasmGlueUrl }),
            ...(message.wasmModuleUrl === undefined ? {} : { wasmModuleUrl: message.wasmModuleUrl }),
        });
        scope.postMessage({
            tag: "result",
            id: message.id,
            result: calculator.calculate(message.source, message.request),
        });
    } catch {
        scope.postMessage({
            tag: "thrown",
            id: message.id,
        });
    }
}
