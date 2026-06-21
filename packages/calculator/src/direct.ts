import type { ApiResult, CalculationOutcome } from "./generated/dto";

export type CalculationRequest = {
    readonly parse: ParseSettings;
    readonly semantics: SemanticSettings;
    readonly exactOutput: ExactOutputRequest;
    readonly scientificOutput: ScientificOutputRequest;
    readonly enclosureOutput: EnclosureOutputRequest;
    readonly limits: ResourceLimitRequest;
};

export type ParseSettings = {
    readonly grammar: "default";
    readonly implicitMultiplication: "enabled" | "disabled";
    readonly unicodeAliases: "mathematicalAliases" | "asciiOnly";
    readonly percent: "postfixPercent" | "rejectPercent";
};

export type SemanticSettings = {
    readonly domain: "real";
    readonly angleUnit: "radian" | "degree" | "gradian";
    readonly powerSemantics: "realPrincipal";
};

export type ExactOutputRequest =
    | {
        readonly tag: "omit";
    }
    | {
        readonly tag: "include";
        readonly format: "auto" | "rational" | "finiteDecimal" | "mixedFraction" | "symbolic";
    };

export type ScientificOutputRequest =
    | {
        readonly tag: "omit";
    }
    | {
        readonly tag: "include";
        readonly significantDigits: number;
        readonly roundingMode: DecimalRoundingMode;
    };

export type DecimalRoundingMode =
    | "nearestTiesToEven"
    | "nearestTiesAwayFromZero"
    | "towardPositiveInfinity"
    | "towardNegativeInfinity"
    | "towardZero"
    | "awayFromZero";

export type EnclosureOutputRequest =
    | {
        readonly tag: "omit";
    }
    | {
        readonly tag: "include";
        readonly format: "exactDyadic";
    };

export type ResourceLimitRequest =
    | {
        readonly tag: "default";
    }
    | {
        readonly tag: "custom";
        readonly value: ResourceLimits;
    };

export type ResourceLimits = {
    readonly maxLogicalWorkUnits: string;
};

export interface Calculator {
    calculate(
        source: string,
        request: CalculationRequest,
    ): ApiResult<CalculationOutcome>;
}

export async function createCalculator(): Promise<Calculator> {
    return {
        calculate: (_source, _request) => ({
            tag: "error",
            error: {
                tag: "unsupportedFeature",
                code: "evaluationEngine",
            },
        }),
    };
}
