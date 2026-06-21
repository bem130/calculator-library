import type {
    ApiResult,
    CalculationOutcome,
    CalculationRequest,
} from "./generated/dto";

export type CalculatorWasmModule = {
    readonly calculate: (
        source: string,
        request: CalculationRequest,
    ) => ApiResult<CalculationOutcome>;
};

export type CreateCalculatorOptions = {
    readonly module?: CalculatorWasmModule;
};

export interface Calculator {
    calculate(
        source: string,
        request: CalculationRequest,
    ): ApiResult<CalculationOutcome>;
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

export async function createCalculator(
    options: CreateCalculatorOptions = {},
): Promise<Calculator> {
    if (options.module !== undefined) {
        return createCalculatorFromWasmModule(options.module);
    }

    return {
        calculate: () => ({
            tag: "error",
            error: {
                tag: "unsupportedFeature",
                code: "evaluationEngine",
            },
        }),
    };
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
