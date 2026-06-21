import { readFile } from "node:fs/promises";

const wasmModuleUrl = new URL(
    "../../packages/calculator/wasm/calculator_wasm.js",
    import.meta.url,
);
const wasmBinaryUrl = new URL(
    "../../packages/calculator/wasm/calculator_wasm_bg.wasm",
    import.meta.url,
);

const wasmBytes = await readFile(wasmBinaryUrl).catch((error) => {
    throw new Error(
        `failed to read ${wasmBinaryUrl.pathname}: ${error.message}; run ` +
            "`corepack pnpm --dir packages/calculator run build:wasm` first",
    );
});
const wasm = await import(wasmModuleUrl.href);
await wasm.default({ module_or_path: wasmBytes });

const roundingModes = [
    "nearestTiesToEven",
    "nearestTiesAwayFromZero",
    "towardPositiveInfinity",
    "towardNegativeInfinity",
    "towardZero",
    "awayFromZero",
];

const exactCases = [
    {
        name: "decimal addition stays exact",
        source: "0.1 + 0.2",
        value: add(decimal("0.1"), decimal("0.2")),
        scientific: [{ digits: 4, modes: ["nearestTiesToEven"] }],
    },
    {
        name: "rational addition reduces",
        source: "1 / 3 + 1 / 6",
        value: add(frac(1n, 3n), frac(1n, 6n)),
        scientific: [{ digits: 6, modes: ["nearestTiesToEven", "towardZero"] }],
    },
    {
        name: "integer power remains rational",
        source: "2^20 / 3",
        value: frac(2n ** 20n, 3n),
        scientific: [{ digits: 8, modes: ["nearestTiesToEven"] }],
    },
    {
        name: "positive decimal rounding boundary",
        source: "1.25",
        value: decimal("1.25"),
        scientific: [{ digits: 2, modes: roundingModes }],
    },
    {
        name: "negative decimal rounding boundary",
        source: "-1.25",
        value: neg(decimal("1.25")),
        scientific: [{ digits: 2, modes: roundingModes }],
    },
    {
        name: "carry changes decimal exponent",
        source: "9.995",
        value: decimal("9.995"),
        scientific: [{ digits: 3, modes: ["nearestTiesToEven"] }],
    },
    {
        name: "ties-to-even can stay on even digit",
        source: "9.985",
        value: decimal("9.985"),
        scientific: [{ digits: 3, modes: ["nearestTiesToEven"] }],
    },
    {
        name: "fraction below one changes exponent on carry",
        source: "0.9995",
        value: decimal("0.9995"),
        scientific: [{ digits: 3, modes: ["nearestTiesToEven"] }],
    },
    {
        name: "large positive exponent",
        source: "100000000000000000000000000000000000000000000 + 0.5",
        value: add(
            frac(100000000000000000000000000000000000000000000n, 1n),
            frac(1n, 2n),
        ),
        scientific: [{ digits: 6, modes: ["nearestTiesToEven", "towardPositiveInfinity"] }],
    },
    {
        name: "large negative exponent",
        source: "1 / 100000000000000000000000000000000000000000000",
        value: frac(1n, 100000000000000000000000000000000000000000000n),
        scientific: [{ digits: 5, modes: ["nearestTiesToEven", "awayFromZero"] }],
    },
    {
        name: "special angle sine",
        source: "sin(pi/6)",
        value: frac(1n, 2n),
        scientific: [{ digits: 5, modes: ["nearestTiesToEven"] }],
    },
    {
        name: "special angle cosine",
        source: "cos(pi/3)",
        value: frac(1n, 2n),
        scientific: [{ digits: 5, modes: ["nearestTiesToEven"] }],
    },
    {
        name: "special angle tangent",
        source: "tan(pi/4)",
        value: frac(1n, 1n),
        scientific: [{ digits: 3, modes: ["nearestTiesToEven"] }],
    },
    {
        name: "degree special angle sine",
        source: "sin(30)",
        angleUnit: "degree",
        value: frac(1n, 2n),
        scientific: [{ digits: 5, modes: ["nearestTiesToEven"] }],
    },
];

const errorCases = [
    {
        name: "radian tangent pole",
        source: "tan(pi/2)",
        code: "tangentPole",
    },
    {
        name: "degree tangent pole",
        source: "tan(90)",
        angleUnit: "degree",
        code: "tangentPole",
    },
];

for (const testCase of exactCases) {
    checkExactCase(testCase);
    for (const scientific of testCase.scientific ?? []) {
        for (const mode of scientific.modes) {
            checkScientificCase(testCase, scientific.digits, mode);
        }
    }
}

for (const testCase of errorCases) {
    checkErrorCase(testCase);
}

console.log(
    `external oracle ok: ${exactCases.length} exact cases, ` +
        `${errorCases.length} error cases`,
);

function checkExactCase(testCase) {
    const result = calculate(testCase.source, {
        angleUnit: testCase.angleUnit,
        scientificOutput: { tag: "omit" },
    });
    const calculation = completeCalculation(result, testCase.name);
    assertIncludedExact(calculation, testCase);
}

function checkScientificCase(testCase, digits, roundingMode) {
    const result = calculate(testCase.source, {
        angleUnit: testCase.angleUnit,
        scientificOutput: {
            tag: "include",
            significantDigits: digits,
            roundingMode,
        },
    });
    const calculation = completeCalculation(
        result,
        `${testCase.name} (${digits}, ${roundingMode})`,
    );
    assertIncludedExact(calculation, testCase);

    if (calculation.scientific.tag !== "included") {
        fail(testCase.name, `expected included scientific output, got ${calculation.scientific.tag}`);
    }
    const actual = calculation.scientific.value;
    const expected = scientificParts(testCase.value, digits, roundingMode);
    assertEqual(
        actual.significand,
        expected.significand,
        testCase.name,
        "scientific.significand",
    );
    assertEqual(
        actual.exponentTen,
        expected.exponentTen,
        testCase.name,
        "scientific.exponentTen",
    );
    assertEqual(
        actual.requestedSignificantDigits,
        digits,
        testCase.name,
        "scientific.requestedSignificantDigits",
    );
    assertEqual(
        actual.confirmedSignificantDigits,
        digits,
        testCase.name,
        "scientific.confirmedSignificantDigits",
    );
    assertEqual(actual.roundingMode, roundingMode, testCase.name, "scientific.roundingMode");
}

function checkErrorCase(testCase) {
    const result = calculate(testCase.source, {
        angleUnit: testCase.angleUnit,
        scientificOutput: { tag: "omit" },
    });
    if (result.tag !== "error") {
        fail(testCase.name, `expected error result, got ${JSON.stringify(result)}`);
    }
    assertEqual(result.error.tag, "domain", testCase.name, "error.tag");
    assertEqual(result.error.code, testCase.code, testCase.name, "error.code");
}

function calculate(source, options) {
    return wasm.calculate(source, {
        parse: {
            grammar: "default",
            implicitMultiplication: "enabled",
            unicodeAliases: "mathematicalAliases",
            percent: "postfixPercent",
        },
        semantics: {
            domain: "real",
            angleUnit: options.angleUnit ?? "radian",
            powerSemantics: "realPrincipal",
        },
        exactOutput: {
            tag: "include",
            format: "auto",
        },
        scientificOutput: options.scientificOutput,
        enclosureOutput: {
            tag: "omit",
        },
        limits: {
            tag: "default",
        },
    });
}

function completeCalculation(result, name) {
    if (result.tag !== "ok") {
        fail(name, `expected ok result, got ${JSON.stringify(result)}`);
    }
    if (result.value.tag !== "complete") {
        fail(name, `expected complete calculation, got ${result.value.tag}`);
    }
    return result.value.calculation;
}

function assertIncludedExact(calculation, testCase) {
    if (calculation.exact.tag !== "included") {
        fail(testCase.name, `expected included exact output, got ${calculation.exact.tag}`);
    }
    const exact = calculation.exact.value;
    assertEqual(exact.plainText, plainFraction(testCase.value), testCase.name, "exact.plainText");
    assertEqual(
        exact.representation,
        testCase.value.denominator === 1n ? "integer" : "rational",
        testCase.name,
        "exact.representation",
    );
}

function scientificParts(value, digits, roundingMode) {
    if (value.numerator === 0n) {
        return {
            significand: zeroSignificand(digits),
            exponentTen: "0",
        };
    }

    const negative = value.numerator < 0n;
    const numerator = abs(value.numerator);
    let denominator = value.denominator;
    let exponentTen = decimalExponent(numerator, denominator);
    const scale = digits - 1 - exponentTen;
    let scaledNumerator = numerator;
    if (scale >= 0) {
        scaledNumerator *= pow10(scale);
    } else {
        denominator *= pow10(-scale);
    }

    let quotient = scaledNumerator / denominator;
    const remainder = scaledNumerator % denominator;
    if (shouldRoundUp({ negative, quotient, remainder, denominator, roundingMode })) {
        quotient += 1n;
    }

    if (quotient >= pow10(digits)) {
        quotient /= 10n;
        exponentTen += 1;
    }

    return {
        significand: significand(negative, quotient, digits),
        exponentTen: String(exponentTen),
    };
}

function shouldRoundUp({ negative, quotient, remainder, denominator, roundingMode }) {
    if (remainder === 0n) {
        return false;
    }
    switch (roundingMode) {
        case "towardPositiveInfinity":
            return !negative;
        case "towardNegativeInfinity":
            return negative;
        case "towardZero":
            return false;
        case "awayFromZero":
            return true;
        case "nearestTiesAwayFromZero":
        case "nearestTiesToEven": {
            const doubled = remainder * 2n;
            if (doubled > denominator) {
                return true;
            }
            if (doubled < denominator) {
                return false;
            }
            return roundingMode === "nearestTiesAwayFromZero" || quotient % 2n !== 0n;
        }
        default:
            throw new Error(`unhandled rounding mode ${roundingMode}`);
    }
}

function decimalExponent(numerator, denominator) {
    if (numerator >= denominator) {
        let exponent = 0;
        let power = 10n;
        while (numerator >= denominator * power) {
            exponent += 1;
            power *= 10n;
        }
        return exponent;
    }

    let exponent = -1;
    let scaled = numerator * 10n;
    while (scaled < denominator) {
        exponent -= 1;
        scaled *= 10n;
    }
    return exponent;
}

function significand(negative, roundedDigits, digits) {
    const text = roundedDigits.toString().padStart(digits, "0");
    const magnitude = digits === 1 ? text : `${text[0]}.${text.slice(1)}`;
    return negative ? `-${magnitude}` : magnitude;
}

function zeroSignificand(digits) {
    return digits === 1 ? "0" : `0.${"0".repeat(digits - 1)}`;
}

function decimal(source) {
    const negative = source.startsWith("-");
    const unsigned = negative ? source.slice(1) : source;
    const [integer, fractional = ""] = unsigned.split(".");
    const denominator = pow10(fractional.length);
    const numerator = BigInt(`${integer}${fractional}` || "0");
    return normalize({
        numerator: negative ? -numerator : numerator,
        denominator,
    });
}

function frac(numerator, denominator = 1n) {
    return normalize({ numerator, denominator });
}

function add(lhs, rhs) {
    return normalize({
        numerator: lhs.numerator * rhs.denominator + rhs.numerator * lhs.denominator,
        denominator: lhs.denominator * rhs.denominator,
    });
}

function neg(value) {
    return {
        numerator: -value.numerator,
        denominator: value.denominator,
    };
}

function normalize(value) {
    if (value.denominator === 0n) {
        throw new Error("zero denominator");
    }
    let numerator = value.numerator;
    let denominator = value.denominator;
    if (denominator < 0n) {
        numerator = -numerator;
        denominator = -denominator;
    }
    const divisor = gcd(abs(numerator), denominator);
    return {
        numerator: numerator / divisor,
        denominator: denominator / divisor,
    };
}

function plainFraction(value) {
    return value.denominator === 1n
        ? value.numerator.toString()
        : `${value.numerator}/${value.denominator}`;
}

function pow10(exponent) {
    if (exponent < 0) {
        throw new Error(`negative power of ten ${exponent}`);
    }
    return 10n ** BigInt(exponent);
}

function gcd(lhs, rhs) {
    let a = lhs;
    let b = rhs;
    while (b !== 0n) {
        const remainder = a % b;
        a = b;
        b = remainder;
    }
    return a;
}

function abs(value) {
    return value < 0n ? -value : value;
}

function assertEqual(actual, expected, name, field) {
    if (actual !== expected) {
        fail(name, `${field}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`);
    }
}

function fail(name, message) {
    throw new Error(`${name}: ${message}`);
}
