import type {
    ApiResult,
    CalculationOutcome,
    CalculationRequest,
    InputActionDto,
    InputPolicyDto,
    SessionDispatchResult,
    SessionStateDto,
} from "./generated/dto";

export type CalculatorWasmModule = {
    readonly calculate: (
        source: string,
        request: CalculationRequest,
    ) => ApiResult<CalculationOutcome>;
};

export type CalculatorSessionWasmModule = {
    readonly CalculatorSession: new (policy: InputPolicyDto) => WasmCalculatorSession;
};

export type CalculatorWasmBundle = CalculatorWasmModule & CalculatorSessionWasmModule;

export type WasmCalculatorSession = {
    dispatch(action: InputActionDto): SessionDispatchResult;
    applyResult(result: ApiResult<CalculationOutcome>): SessionStateDto;
    getState(): SessionStateDto;
};

export type CreateCalculatorOptions = {
    readonly module?: CalculatorWasmModule;
    readonly wasmGlueUrl?: string | URL;
    readonly wasmModuleUrl?: string | URL;
};

export type CreateSessionOptions = {
    readonly module?: CalculatorSessionWasmModule;
    readonly wasmGlueUrl?: string | URL;
    readonly wasmModuleUrl?: string | URL;
};

type WasmLoadOptions = {
    readonly wasmGlueUrl?: string | URL;
    readonly wasmModuleUrl?: string | URL;
};

export interface Calculator {
    calculate(
        source: string,
        request: CalculationRequest,
    ): ApiResult<CalculationOutcome>;
}

export interface CalculatorSession {
    dispatch(action: InputActionDto): SessionDispatchResult;
    applyResult(result: ApiResult<CalculationOutcome>): SessionStateDto;
    getState(): SessionStateDto;
}

export function createCalculatorFromWasmModule(
    module: CalculatorWasmModule,
): Calculator {
    return {
        calculate(source, request) {
            return module.calculate(source, request);
        },
    };
}

export function createSessionFromWasmModule(
    module: CalculatorSessionWasmModule,
    policy: InputPolicyDto,
): CalculatorSession {
    const session = new module.CalculatorSession(policy);
    return {
        dispatch(action) {
            return session.dispatch(action);
        },
        applyResult(result) {
            return session.applyResult(result);
        },
        getState() {
            return session.getState();
        },
    };
}

export async function createCalculator(
    options: CreateCalculatorOptions = {},
): Promise<Calculator> {
    if (options.module !== undefined) {
        return createCalculatorFromWasmModule(options.module);
    }

    return createCalculatorFromWasmModule(await loadDefaultWasmModule(options));
}

export async function createSession(
    policy: InputPolicyDto,
    options: CreateSessionOptions = {},
): Promise<CalculatorSession> {
    if (options.module !== undefined) {
        return createSessionFromWasmModule(options.module, policy);
    }

    return createSessionFromWasmModule(await loadDefaultWasmModule(options), policy);
}

export const exactOnlyCalculationRequest: CalculationRequest = {
    parse: {
        grammar: "default",
        implicitMultiplication: "enabled",
        unicodeAliases: "mathematicalAliases",
        percent: "postfixPercent",
    },
    semantics: {
        domain: "real",
        angleUnit: "radian",
        powerSemantics: "realPrincipal",
    },
    exactOutput: {
        tag: "include",
        format: "auto",
    },
    scientificOutput: {
        tag: "omit",
    },
    enclosureOutput: {
        tag: "omit",
    },
    limits: {
        tag: "default",
    },
};

export const defaultCalculationRequest: CalculationRequest = {
    ...exactOnlyCalculationRequest,
    scientificOutput: {
        tag: "include",
        significantDigits: 50,
        roundingMode: "nearestTiesToEven",
    },
    enclosureOutput: {
        tag: "include",
        format: "exactDyadic",
    },
};

export const defaultInputPolicy: InputPolicyDto = {
    calculationRequest: defaultCalculationRequest,
    percentPolicy: "expressionPercent",
};

type GeneratedWasmModule = CalculatorWasmModule & {
    readonly CalculatorSession: new (policy: InputPolicyDto) => WasmCalculatorSession;
    readonly default: (input?: WasmInitInput) => Promise<unknown>;
};

type WasmInitInput = {
    readonly module_or_path: string | URL | Request | Response | BufferSource | WebAssembly.Module;
};

async function loadDefaultWasmModule(
    options: WasmLoadOptions,
): Promise<CalculatorWasmBundle> {
    const moduleUrl = options.wasmGlueUrl ?? packageAssetUrl("calculator_wasm.js");
    const wasmUrl = options.wasmModuleUrl ?? packageAssetUrl("calculator_wasm_bg.wasm");
    const module = await import(
        /* @vite-ignore */ String(moduleUrl)
    ) as GeneratedWasmModule;
    await module.default({
        module_or_path: wasmUrl,
    });
    return module;
}

function packageAssetUrl(fileName: string): URL {
    return new URL(`../wasm/${fileName}`, import.meta.url);
}
