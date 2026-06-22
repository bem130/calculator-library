import { spawn } from "node:child_process";
import { readFile, readdir } from "node:fs/promises";
import http from "node:http";
import net from "node:net";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const exampleRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

await assertPublicApiUsage();

const port = await findFreePort();
const origin = `http://127.0.0.1:${port}`;
const preview = startPreview(port);

try {
    await waitForHttp(origin);
    await runBrowserChecks(`${origin}/`, origin);
} finally {
    stopPreview(preview);
}

async function assertPublicApiUsage() {
    const srcRoot = path.join(exampleRoot, "src");
    const sources = await readSources(srcRoot);
    const mainSource = sources.get(path.join(srcRoot, "main.ts")) ?? "";

    for (const [fileName, source] of sources) {
        for (const specifier of moduleSpecifiers(source)) {
            assert(
                !/calculator_wasm|[\\/]wasm[\\/]|calculator-wasm/u.test(specifier),
                `${fileName} imports private Wasm binding ${specifier}`,
            );
        }
    }

    assert(
        mainSource.includes("from \"@bem130/exact-calculator/worker\""),
        "sample UI must use the public worker API export",
    );
    assert(
        mainSource.includes("session.dispatch("),
        "sample UI must drive button operations through session dispatch",
    );
    assert(
        mainSource.includes("renderPlainText("),
        "plain text copy must use renderPlainText",
    );
    assert(
        mainSource.includes("calculation.scientific.value.presentation"),
        "scientific output must render the DTO presentation tree",
    );
    assert(
        !mainSource.includes("formatScientificDecimal"),
        "sample UI must not reimplement scientific notation formatting",
    );
    assert(
        mainSource.includes("renderMathMl("),
        "MathML display must use renderMathMl",
    );
    assert(
        mainSource.includes("renderResultRelationPlainText("),
        "result relation text must use the public relation renderer",
    );
    assert(
        mainSource.includes("presentInput("),
        "input preview must use the public presentInput facade",
    );
}

async function runBrowserChecks(url, origin) {
    const browser = await chromium.launch();
    const context = await browser.newContext();
    await context.grantPermissions(["clipboard-read", "clipboard-write"], { origin });

    let delayNextWorkerWasm = false;
    await context.route("**/*", async (route) => {
        const url = new URL(route.request().url());
        if (delayNextWorkerWasm && /^\/.*calculator_wasm(?:-[^/]+)?\.js$/u.test(url.pathname)) {
            delayNextWorkerWasm = false;
            await sleep(1200);
        }
        await route.continue();
    });

    const page = await context.newPage();
    const browserErrors = [];
    page.on("console", (message) => {
        if (message.type() === "error") {
            browserErrors.push(`console: ${message.text()}`);
        }
    });
    page.on("pageerror", (error) => {
        browserErrors.push(`pageerror: ${error.message}`);
    });

    try {
        await page.goto(url);
        await waitForText(page, "#exact-output", "= sqrt(2)");
        assert(
            await page.locator("#mathml-output math msqrt").count() > 0,
            "MathML radical was not rendered",
        );
        assert(
            await page.locator("#input-preview math msqrt").count() > 0,
            "input preview radical was not rendered",
        );
        await waitForText(page, "#scientific-state", "5 digits");
        await waitForText(page, "#scientific-output", "≈ 1.4142 × 10^0");
        await waitForText(page, "#enclosure-state", "DECIMAL SCIENTIFIC");
        await waitForText(page, "#enclosure-output", "∈ [1.4142 × 10^0, 1.4143 × 10^0]");
        assert(await page.locator("#include-scientific").count() === 0, "output toggles remain");
        await assertPhase2Outputs(page);

        await page.click("#copy");
        await waitForText(page, "#status", "Copied");
        const clipboardText = await page.evaluate(() => navigator.clipboard.readText());
        assert(clipboardText === "= 3/10", `clipboard text was ${JSON.stringify(clipboardText)}`);

        await assertIrrationalSqrtPartial(page);
        await assertPiPartial(page);
        await assertRationalPiMultiplePartial(page);
        await assertSpecialAngles(page);
        await assertSimpleRadicalAlgebra(page);
        await assertInitialExpLog(page);
        await assertArbitraryBaseLogExp(page);
        await assertRationalPowers(page);

        await page.fill("#expression", "");
        await page.click('button[data-key="7"]');
        await page.click('button[data-key="+"]');
        await page.click('button[data-key="8"]');
        await page.waitForFunction(() => {
            const input = document.querySelector("#expression");
            return input instanceof HTMLTextAreaElement && input.value.includes("8");
        });
        await page.click("#calculate");
        await waitForText(page, "#exact-output", "= 15");

        await page.fill("#expression", "");
        await page.click('button[data-key="asin("]');
        await page.click('button[data-key="sqrt(2)/2"]');
        await page.click('button[data-key=")"]');
        await page.waitForFunction(() => {
            const input = document.querySelector("#expression");
            return input instanceof HTMLTextAreaElement && input.value === "asin(sqrt(2)/2)";
        });
        await page.click("#calculate");
        await waitForText(page, "#exact-output", "= pi/4");

        await page.fill("#expression", "");
        await page.click('button[data-key="sin("]');
        await page.click('button[data-key="pi/6"]');
        await page.click('button[data-key=")"]');
        await page.click("#calculate");
        await waitForText(page, "#exact-output", "= 1/2");

        const previousExact = await textContent(page, "#exact-output");
        delayNextWorkerWasm = true;
        await page.fill("#expression", "0.1 + 0.2");
        await page.click("#calculate");
        await page.waitForSelector("#cancel:not([disabled])");
        await page.click("#cancel");
        await waitForText(page, "#status", "Canceled");
        assert(await page.locator("#cancel").isDisabled(), "cancel button stayed enabled");
        await page.waitForTimeout(1400);
        assert(
            await textContent(page, "#exact-output") === previousExact,
            "canceled worker calculation updated the exact result",
        );

        assert(browserErrors.length === 0, browserErrors.join("\n"));
    } finally {
        await browser.close();
    }
}

async function assertPhase2Outputs(page) {
    await page.fill("#expression", "0.1 + 0.2");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 3/10");

    await waitForText(page, "#scientific-state", "5 digits");
    await waitForText(
        page,
        "#scientific-output",
        "≈ 3.0000 × 10^-1",
    );
    await waitForText(page, "#enclosure-state", "DECIMAL SCIENTIFIC");

    const interval = parseDecimalScientificInterval(await textContent(page, "#enclosure-output"));
    assert(
        rationalCompareWithRational(interval.lower, 3n, 10n) <= 0,
        "certified enclosure lower bound is above 3/10",
    );
    assert(
        rationalCompareWithRational(interval.upper, 3n, 10n) >= 0,
        "certified enclosure upper bound is below 3/10",
    );
}

async function assertInitialExpLog(page) {
    await page.fill("#expression", "exp(1)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= exp(1)");
    await waitForText(page, "#scientific-state", "5 digits");
    await waitForText(page, "#enclosure-state", "DECIMAL SCIENTIFIC");

    const interval = parseDecimalScientificInterval(await textContent(page, "#enclosure-output"));
    assert(
        rationalCompareWithRational(interval.lower, 2n, 1n) > 0,
        "exp(1) lower bound is not above 2",
    );
    assert(
        rationalCompareWithRational(interval.upper, 3n, 1n) < 0,
        "exp(1) upper bound is not below 3",
    );

    await page.fill("#expression", "ln(1)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 0");

    await page.fill("#expression", "exp(ln(2))");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 2");

    await page.fill("#expression", "exp(ln(sqrt(2)))");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= sqrt(2)");
    await waitForText(page, "#exact-kind", "RADICAL");

    await page.fill("#expression", "exp(ln(sqrt(2)+sqrt(3)))");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= sqrt(2) + sqrt(3)");
    await waitForText(page, "#exact-kind", "RADICAL");

    await page.fill("#expression", "ln(exp(2))");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 2");

    await page.fill("#expression", "ln(exp(-sqrt(2)))");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= -sqrt(2)");
    await waitForText(page, "#exact-kind", "RADICAL");
}

async function assertArbitraryBaseLogExp(page) {
    await page.fill("#expression", "log(8,2)");
    await page.waitForSelector("#input-preview math msub");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 3");

    await page.fill("#expression", "ln(e)");
    await page.waitForSelector("#input-preview math mi");
    assert(
        await textContent(page, "#input-preview") === "ln(e)",
        "ln input preview did not render as natural log",
    );
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 1");

    await page.fill("#expression", "exp(3,2)");
    await page.waitForSelector("#input-preview math msup");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 8");

    await page.fill("#expression", "");
    await page.click('button[data-key="log("]');
    await page.click('button[data-key="8"]');
    await page.click('button[data-key=","]');
    await page.click('button[data-key="2"]');
    await page.click('button[data-key=")"]');
    await page.waitForFunction(() => {
        const input = document.querySelector("#expression");
        return input instanceof HTMLTextAreaElement && input.value === "log(8,2)";
    });
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 3");
}

async function assertRationalPowers(page) {
    await page.fill("#expression", "(-8)^(1/3)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= -2");

    await page.fill("#expression", "(-8)^(2/3)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 4");

    await page.fill("#expression", "2^(1/2)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= sqrt(2)");
    await waitForText(page, "#exact-kind", "RADICAL");
    await waitForText(page, "#scientific-state", "5 digits");
    await waitForText(page, "#enclosure-state", "DECIMAL SCIENTIFIC");

    const interval = parseDecimalScientificInterval(await textContent(page, "#enclosure-output"));
    assert(
        rationalSquareCompareWithRational(interval.lower, 2n, 1n) <= 0,
        "2^(1/2) lower bound squared is above 2",
    );
    assert(
        rationalSquareCompareWithRational(interval.upper, 2n, 1n) >= 0,
        "2^(1/2) upper bound squared is below 2",
    );

    await page.fill("#expression", "2^sqrt(2)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 2^sqrt(2)");
    await waitForText(page, "#exact-kind", "GENERAL SYMBOLIC");
    await waitForText(page, "#scientific-state", "5 digits");
    await waitForText(page, "#enclosure-state", "DECIMAL SCIENTIFIC");

    await page.fill("#expression", "sqrt(2)^sqrt(2)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= sqrt(2)^sqrt(2)");
    await waitForText(page, "#exact-kind", "GENERAL SYMBOLIC");
    await waitForText(page, "#scientific-state", "5 digits");
    await waitForText(page, "#enclosure-state", "DECIMAL SCIENTIFIC");

    await page.fill("#expression", "2^(1/3)+1");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 2^(1/3)+1");
    await waitForText(page, "#exact-kind", "REAL ALGEBRAIC");
    await waitForText(page, "#scientific-state", "PRECISION LIMIT");
    await waitForText(page, "#enclosure-state", "DECIMAL SCIENTIFIC");

    await page.fill("#expression", "1-2^(1/3)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 1-2^(1/3)");
    await waitForText(page, "#exact-kind", "REAL ALGEBRAIC");

    await page.fill("#expression", "2^(1/3)/2+1");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 2^(1/3)/2+1");
    await waitForText(page, "#exact-kind", "REAL ALGEBRAIC");

    await page.fill("#expression", "1/2^(1/3)+1");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 1/2^(1/3)+1");
    await waitForText(page, "#exact-kind", "REAL ALGEBRAIC");

    await page.fill("#expression", "sqrt(2^(1/3))");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= sqrt(2^(1/3))");
    await waitForText(page, "#exact-kind", "REAL ALGEBRAIC");

    await page.fill("#expression", "(2^(1/3))^(2/5)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= (2^(1/3))^(2/5)");
    await waitForText(page, "#exact-kind", "REAL ALGEBRAIC");

    await page.fill("#expression", "2^(1/3)-2^(1/3)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 0");
    await waitForText(page, "#exact-kind", "INTEGER");

    await page.fill("#expression", "2^(1/3)/2^(1/3)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 1");
    await waitForText(page, "#exact-kind", "INTEGER");

    await page.fill("#expression", "(2^(1/3)-2^(1/3))+2^(1/3)-2^(1/3)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 0");
    await waitForText(page, "#exact-kind", "INTEGER");

    await page.fill("#expression", "(2^(1/3)/2^(1/3))*2^(1/3)/2^(1/3)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 1");
    await waitForText(page, "#exact-kind", "INTEGER");

    await page.fill("#expression", "(2^(1/3)-2^(1/3))+2^(1/3)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= (2^(1/3)-2^(1/3))+2^(1/3)");
    await waitForText(page, "#exact-kind", "REAL ALGEBRAIC");

    await page.fill("#expression", "((2^(1/3)-2^(1/3))+2)^(1/3)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= ((2^(1/3)-2^(1/3))+2)^(1/3)");
    await waitForText(page, "#exact-kind", "REAL ALGEBRAIC");
}

async function assertPiPartial(page) {
    await page.fill("#expression", "pi");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= pi");
    await waitForText(page, "#exact-kind", "RATIONAL PI MULTIPLE");
    await waitForText(page, "#scientific-state", "5 digits");
    await waitForText(page, "#enclosure-state", "DECIMAL SCIENTIFIC");

    const interval = parseDecimalScientificInterval(await textContent(page, "#enclosure-output"));
    assert(
        rationalCompareWithRational(interval.lower, 3n, 1n) > 0,
        "pi lower bound is not above 3",
    );
    assert(
        rationalCompareWithRational(interval.upper, 22n, 7n) < 0,
        "pi upper bound is not below 22/7",
    );
}

async function assertRationalPiMultiplePartial(page) {
    await page.fill("#expression", "pi/6");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= pi/6");
    await waitForText(page, "#exact-kind", "RATIONAL PI MULTIPLE");
    await waitForText(page, "#scientific-state", "5 digits");
    await waitForText(page, "#enclosure-state", "DECIMAL SCIENTIFIC");
}

async function assertSpecialAngles(page) {
    await page.fill("#expression", "sin(pi/6)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 1/2");
    await waitForText(page, "#exact-kind", "RATIONAL");
    await waitForText(page, "#scientific-state", "5 digits");
    await waitForText(page, "#enclosure-state", "DECIMAL SCIENTIFIC");

    await page.fill("#expression", "tan(pi/2)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "domain.tangentPole");

    await page.fill("#expression", "sin(pi/4)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= sqrt(2)/2");
    await waitForText(page, "#exact-kind", "RADICAL");

    await page.fill("#expression", "sin(pi/12)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= sqrt(6)/4 - sqrt(2)/4");
    await waitForText(page, "#exact-kind", "RADICAL");

    await page.fill("#expression", "tan(pi/12)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 2 - sqrt(3)");
    await waitForText(page, "#exact-kind", "RADICAL");
    await waitForIdle(page);

    await page.fill("#expression", "tan(1)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= tan(1)");
    await waitForText(page, "#exact-kind", "GENERAL SYMBOLIC");
    await waitForText(page, "#enclosure-state", "DECIMAL SCIENTIFIC");
    await waitForIdle(page);

    await page.fill("#expression", "sin(1)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= sin(1)");
    await waitForText(page, "#exact-kind", "GENERAL SYMBOLIC");
    await waitForText(page, "#enclosure-state", "DECIMAL SCIENTIFIC");
    await waitForIdle(page);

    await page.fill("#expression", "sin(-1)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= -sin(1)");
    await waitForText(page, "#exact-kind", "GENERAL SYMBOLIC");
    await waitForText(page, "#enclosure-state", "DECIMAL SCIENTIFIC");
    await waitForIdle(page);

    await page.fill("#expression", "sin(pi+1/10)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= -sin(1/10)");
    await waitForText(page, "#exact-kind", "GENERAL SYMBOLIC");
    await waitForText(page, "#enclosure-state", "DECIMAL SCIENTIFIC");
    await waitForIdle(page);

    await page.fill("#expression", "cos(pi/2+1/10)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= -sin(1/10)");
    await waitForText(page, "#exact-kind", "GENERAL SYMBOLIC");
    await waitForText(page, "#enclosure-state", "DECIMAL SCIENTIFIC");
    await waitForIdle(page);

    await page.fill("#expression", "tan(pi/2+1/10)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= -1/tan(1/10)");
    await waitForText(page, "#exact-kind", "GENERAL SYMBOLIC");
    await waitForText(page, "#enclosure-state", "DECIMAL SCIENTIFIC");
    await waitForIdle(page);

    await page.fill("#expression", "exp(sin(-1))");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= exp(-sin(1))");
    await waitForText(page, "#exact-kind", "GENERAL SYMBOLIC");
    await waitForText(page, "#enclosure-state", "DECIMAL SCIENTIFIC");
    await waitForIdle(page);

    await page.fill("#expression", "cos(1)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= cos(1)");
    await waitForText(page, "#exact-kind", "GENERAL SYMBOLIC");
    await waitForText(page, "#enclosure-state", "DECIMAL SCIENTIFIC");
    await waitForIdle(page);

    await page.fill("#expression", "sin(2)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= sin(2)");
    await waitForText(page, "#exact-kind", "GENERAL SYMBOLIC");
    await waitForText(page, "#enclosure-state", "DECIMAL SCIENTIFIC");
    await waitForIdle(page);

    await page.fill("#expression", "tan(2)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= tan(2)");
    await waitForText(page, "#exact-kind", "GENERAL SYMBOLIC");
    await waitForText(page, "#enclosure-state", "DECIMAL SCIENTIFIC");
    await waitForIdle(page);

    await page.fill("#expression", "asin(1/2)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= pi/6");
    await waitForText(page, "#exact-kind", "RATIONAL PI MULTIPLE");

    await page.fill("#expression", "asin(sqrt(2)/2)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= pi/4");
    await waitForText(page, "#exact-kind", "RATIONAL PI MULTIPLE");

    await page.fill("#expression", "atan(sqrt(3))");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= pi/3");
    await waitForText(page, "#exact-kind", "RATIONAL PI MULTIPLE");
    await waitForIdle(page);

    await page.fill("#expression", "asin(1/3)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= asin(1/3)");
    await waitForText(page, "#exact-kind", "GENERAL SYMBOLIC");
    await waitForText(page, "#enclosure-state", "DECIMAL SCIENTIFIC");
    await waitForIdle(page);

    await page.fill("#expression", "sin(asin(1/3))");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 1/3");
    await waitForText(page, "#exact-kind", "RATIONAL");
    await waitForIdle(page);

    await page.fill("#expression", "cos(asin(sqrt(2)/3))");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= sqrt(7)/3");
    await waitForText(page, "#exact-kind", "RADICAL");
    await waitForIdle(page);

    await page.fill("#expression", "acos(1/3)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= acos(1/3)");
    await waitForText(page, "#exact-kind", "GENERAL SYMBOLIC");
    await waitForText(page, "#enclosure-state", "DECIMAL SCIENTIFIC");

    await page.click('button[data-angle="degree"]');
    await page.fill("#expression", "sin(30)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 1/2");
    await waitForText(page, "#exact-kind", "RATIONAL");

    await page.fill("#expression", "asin(1/2)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 30");
    await waitForText(page, "#exact-kind", "INTEGER");

    await page.fill("#expression", "sin(asin(1/3))");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 1/3");
    await waitForText(page, "#exact-kind", "RATIONAL");

    await page.fill("#expression", "cos(asin(sqrt(2)/3))");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= sqrt(7)/3");
    await waitForText(page, "#exact-kind", "RADICAL");

    await page.fill("#expression", "atan(sqrt(3))");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 60");
    await waitForText(page, "#exact-kind", "INTEGER");
}

async function assertSimpleRadicalAlgebra(page) {
    await page.click('button[data-angle="radian"]');

    await page.fill("#expression", "sqrt(2)*sqrt(2)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 2");
    await waitForText(page, "#exact-kind", "INTEGER");

    await page.fill("#expression", "sqrt(2)*sqrt(3)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= sqrt(6)");
    await waitForText(page, "#exact-kind", "RADICAL");

    await page.fill("#expression", "sqrt(6962)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 59sqrt(2)");
    await waitForText(page, "#exact-kind", "RADICAL");

    await page.fill("#expression", "sin(pi/6)+sqrt(2)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 1/2 + sqrt(2)");
    await waitForText(page, "#exact-kind", "RADICAL");

    await page.fill("#expression", "sqrt(8)/sqrt(2)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 2");
    await waitForText(page, "#exact-kind", "INTEGER");
}

async function assertIrrationalSqrtPartial(page) {
    await page.fill("#expression", "sqrt(2)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= sqrt(2)");
    await waitForText(page, "#exact-kind", "RADICAL");
    await waitForText(page, "#scientific-state", "5 digits");
    await waitForText(page, "#scientific-output", "≈ 1.4142 × 10^0");
    await waitForText(page, "#enclosure-state", "DECIMAL SCIENTIFIC");

    const interval = parseDecimalScientificInterval(await textContent(page, "#enclosure-output"));
    assert(
        rationalSquareCompareWithRational(interval.lower, 2n, 1n) <= 0,
        "sqrt(2) lower bound squared is above 2",
    );
    assert(
        rationalSquareCompareWithRational(interval.upper, 2n, 1n) >= 0,
        "sqrt(2) upper bound squared is below 2",
    );
}

async function readSources(directory) {
    const entries = await readdir(directory, { withFileTypes: true });
    const sources = new Map();
    for (const entry of entries) {
        const entryPath = path.join(directory, entry.name);
        if (entry.isDirectory()) {
            for (const [childPath, source] of await readSources(entryPath)) {
                sources.set(childPath, source);
            }
        } else if (entry.name.endsWith(".ts")) {
            sources.set(entryPath, await readFile(entryPath, "utf8"));
        }
    }
    return sources;
}

function moduleSpecifiers(source) {
    const specifiers = [];
    const pattern = /\bfrom\s+["']([^"']+)["']|import\s*\(\s*["']([^"']+)["']\s*\)/gu;
    for (const match of source.matchAll(pattern)) {
        specifiers.push(match[1] ?? match[2]);
    }
    return specifiers;
}

function startPreview(port) {
    const viteCli = path.join(exampleRoot, "node_modules", "vite", "bin", "vite.js");
    const child = spawn(
        process.execPath,
        [viteCli, "preview", "--host", "127.0.0.1", "--port", String(port), "--strictPort"],
        {
            cwd: exampleRoot,
            stdio: ["ignore", "pipe", "pipe"],
        },
    );

    let output = "";
    child.stdout.on("data", (chunk) => {
        output += String(chunk);
    });
    child.stderr.on("data", (chunk) => {
        output += String(chunk);
    });
    child.on("exit", (code, signal) => {
        if (code !== null && code !== 0) {
            console.error(output);
            console.error(`vite preview exited with code ${code}`);
        } else if (signal !== null && signal !== "SIGTERM") {
            console.error(output);
            console.error(`vite preview exited with signal ${signal}`);
        }
    });
    return child;
}

function stopPreview(child) {
    if (child.exitCode === null && child.signalCode === null) {
        child.kill("SIGTERM");
    }
}

async function findFreePort() {
    return await new Promise((resolve, reject) => {
        const server = net.createServer();
        server.once("error", reject);
        server.listen(0, "127.0.0.1", () => {
            const address = server.address();
            if (address === null || typeof address === "string") {
                server.close(() => reject(new Error("could not allocate a TCP port")));
                return;
            }
            const port = address.port;
            server.close(() => resolve(port));
        });
    });
}

async function waitForHttp(url) {
    const deadline = Date.now() + 15000;
    while (Date.now() < deadline) {
        if (await httpOk(url)) {
            return;
        }
        await sleep(200);
    }
    throw new Error(`server did not become ready: ${url}`);
}

async function httpOk(url) {
    return await new Promise((resolve) => {
        const request = http.get(url, (response) => {
            response.resume();
            resolve(response.statusCode !== undefined && response.statusCode < 500);
        });
        request.on("error", () => resolve(false));
        request.setTimeout(1000, () => {
            request.destroy();
            resolve(false);
        });
    });
}

async function waitForText(page, selector, expected) {
    await page.waitForFunction(
        ({ selector: currentSelector, expected: currentExpected }) =>
            document.querySelector(currentSelector)?.textContent?.trim() === currentExpected,
        { selector, expected },
        { timeout: 15000 },
    );
}

async function waitForIdle(page) {
    await page.waitForSelector("#calculate:not([disabled])", { timeout: 15000 });
}

async function textContent(page, selector) {
    return await page.locator(selector).evaluate((element) => element.textContent?.trim() ?? "");
}

function parseDecimalScientificInterval(source) {
    const match =
        /^(?:∈ )?\[(-?\d+(?:\.\d+)?) × 10\^(-?\d+), (-?\d+(?:\.\d+)?) × 10\^(-?\d+)\]$/u.exec(
            source,
        );
    assert(match !== null, `unexpected enclosure output: ${JSON.stringify(source)}`);
    return {
        lower: parseDecimalScientificRational(match[1], Number.parseInt(match[2], 10)),
        upper: parseDecimalScientificRational(match[3], Number.parseInt(match[4], 10)),
    };
}

function parseDecimalScientificRational(significand, exponentTen) {
    const negative = significand.startsWith("-");
    const unsigned = negative ? significand.slice(1) : significand;
    const [integerPart, fractionalPart = ""] = unsigned.split(".");
    const digits = `${integerPart}${fractionalPart}`;
    let numerator = BigInt(digits);
    let denominator = pow10(fractionalPart.length);
    if (exponentTen >= 0) {
        numerator *= pow10(exponentTen);
    } else {
        denominator *= pow10(-exponentTen);
    }
    if (negative) {
        numerator = -numerator;
    }
    const divisor = gcd(abs(numerator), denominator);
    return {
        numerator: numerator / divisor,
        denominator: denominator / divisor,
    };
}

function rationalCompareWithRational(value, numerator, denominator) {
    assert(value.denominator > 0n, "left denominator must be positive");
    assert(denominator > 0n, "right denominator must be positive");
    const left = value.numerator * denominator;
    const right = numerator * value.denominator;
    if (left < right) {
        return -1;
    }
    if (left > right) {
        return 1;
    }
    return 0;
}

function rationalSquareCompareWithRational(value, numerator, denominator) {
    return rationalCompareWithRational(
        {
            numerator: value.numerator * value.numerator,
            denominator: value.denominator * value.denominator,
        },
        numerator,
        denominator,
    );
}

function pow10(exponent) {
    let value = 1n;
    for (let index = 0; index < exponent; index += 1) {
        value *= 10n;
    }
    return value;
}

function gcd(left, right) {
    let a = left;
    let b = right;
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

function assert(condition, message) {
    if (!condition) {
        throw new Error(message);
    }
}

function sleep(milliseconds) {
    return new Promise((resolve) => setTimeout(resolve, milliseconds));
}
