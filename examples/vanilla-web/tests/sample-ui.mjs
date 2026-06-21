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
        mainSource.includes("renderMathMl("),
        "MathML display must use renderMathMl",
    );
}

async function runBrowserChecks(url, origin) {
    const browser = await chromium.launch();
    const context = await browser.newContext();
    await context.grantPermissions(["clipboard-read", "clipboard-write"], { origin });

    let delayNextWorkerWasm = false;
    await context.route("**/wasm/calculator_wasm.js", async (route) => {
        if (delayNextWorkerWasm) {
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
        await waitForText(page, "#exact-output", "= 3/10");
        assert(
            await page.locator("#mathml-output math mfrac").count() > 0,
            "MathML fraction was not rendered",
        );
        await assertPhase2Outputs(page);

        await page.click("#copy");
        await waitForText(page, "#status", "Copied");
        const clipboardText = await page.evaluate(() => navigator.clipboard.readText());
        assert(clipboardText === "3/10", `clipboard text was ${JSON.stringify(clipboardText)}`);

        await assertIrrationalSqrtPartial(page);
        await assertPiPartial(page);
        await assertRationalPiMultiplePartial(page);
        await assertSpecialAngles(page);
        await assertInitialExpLog(page);
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
    await page.check("#include-scientific");
    await page.check("#include-enclosure");
    await page.click("#calculate");

    await waitForText(page, "#scientific-state", "50 digits");
    await waitForText(
        page,
        "#scientific-output",
        "3.0000000000000000000000000000000000000000000000000e-1",
    );
    await waitForText(page, "#enclosure-state", "EXACT DYADIC");

    const interval = parseExactDyadicInterval(await textContent(page, "#enclosure-output"));
    assert(
        dyadicCompareWithRational(interval.lower, 3n, 10n) <= 0,
        "certified enclosure lower bound is above 3/10",
    );
    assert(
        dyadicCompareWithRational(interval.upper, 3n, 10n) >= 0,
        "certified enclosure upper bound is below 3/10",
    );
}

async function assertInitialExpLog(page) {
    await page.fill("#expression", "exp(1)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= exp(1)");
    await waitForText(page, "#scientific-state", "PRECISION LIMIT");
    await waitForText(page, "#enclosure-state", "EXACT DYADIC");

    const interval = parseExactDyadicInterval(await textContent(page, "#enclosure-output"));
    assert(
        dyadicCompareWithRational(interval.lower, 2n, 1n) > 0,
        "exp(1) lower bound is not above 2",
    );
    assert(
        dyadicCompareWithRational(interval.upper, 3n, 1n) < 0,
        "exp(1) upper bound is not below 3",
    );

    await page.fill("#expression", "log(1)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 0");

    await page.fill("#expression", "exp(log(2))");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 2");

    await page.fill("#expression", "log(exp(2))");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 2");
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
    await waitForText(page, "#scientific-state", "PRECISION LIMIT");
    await waitForText(page, "#enclosure-state", "EXACT DYADIC");

    const interval = parseExactDyadicInterval(await textContent(page, "#enclosure-output"));
    assert(
        dyadicSquareCompareWithRational(interval.lower, 2n, 1n) <= 0,
        "2^(1/2) lower bound squared is above 2",
    );
    assert(
        dyadicSquareCompareWithRational(interval.upper, 2n, 1n) >= 0,
        "2^(1/2) upper bound squared is below 2",
    );
}

async function assertPiPartial(page) {
    await page.fill("#expression", "pi");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= pi");
    await waitForText(page, "#exact-kind", "RATIONAL PI MULTIPLE");
    await waitForText(page, "#scientific-state", "PRECISION LIMIT");
    await waitForText(page, "#enclosure-state", "EXACT DYADIC");

    const interval = parseExactDyadicInterval(await textContent(page, "#enclosure-output"));
    assert(
        dyadicCompareWithRational(interval.lower, 3n, 1n) > 0,
        "pi lower bound is not above 3",
    );
    assert(
        dyadicCompareWithRational(interval.upper, 22n, 7n) < 0,
        "pi upper bound is not below 22/7",
    );
}

async function assertRationalPiMultiplePartial(page) {
    await page.fill("#expression", "pi/6");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= pi/6");
    await waitForText(page, "#exact-kind", "RATIONAL PI MULTIPLE");
    await waitForText(page, "#scientific-state", "PRECISION LIMIT");
    await waitForText(page, "#enclosure-state", "EXACT DYADIC");
}

async function assertSpecialAngles(page) {
    await page.fill("#expression", "sin(pi/6)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 1/2");
    await waitForText(page, "#exact-kind", "RATIONAL");
    await waitForText(page, "#scientific-state", "50 digits");
    await waitForText(page, "#enclosure-state", "EXACT DYADIC");

    await page.fill("#expression", "tan(pi/2)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "domain.tangentPole");

    await page.fill("#expression", "sin(pi/4)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= sqrt(2)/2");
    await waitForText(page, "#exact-kind", "RADICAL");

    await page.fill("#expression", "asin(1/2)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= pi/6");
    await waitForText(page, "#exact-kind", "RATIONAL PI MULTIPLE");

    await page.click('button[data-angle="degree"]');
    await page.fill("#expression", "sin(30)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 1/2");
    await waitForText(page, "#exact-kind", "RATIONAL");

    await page.fill("#expression", "asin(1/2)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 30");
    await waitForText(page, "#exact-kind", "INTEGER");
}

async function assertIrrationalSqrtPartial(page) {
    await page.fill("#expression", "sqrt(2)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= sqrt(2)");
    await waitForText(page, "#exact-kind", "RADICAL");
    await waitForText(page, "#scientific-state", "PRECISION LIMIT");
    await waitForText(page, "#scientific-output", "Requested decimal digits are not confirmed.");
    await waitForText(page, "#enclosure-state", "EXACT DYADIC");

    const interval = parseExactDyadicInterval(await textContent(page, "#enclosure-output"));
    assert(
        dyadicSquareCompareWithRational(interval.lower, 2n, 1n) <= 0,
        "sqrt(2) lower bound squared is above 2",
    );
    assert(
        dyadicSquareCompareWithRational(interval.upper, 2n, 1n) >= 0,
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

async function textContent(page, selector) {
    return await page.locator(selector).evaluate((element) => element.textContent?.trim() ?? "");
}

function parseExactDyadicInterval(source) {
    const match = /^\[(-?\d+) \* 2\^(-?\d+), (-?\d+) \* 2\^(-?\d+)\]$/u.exec(source);
    assert(match !== null, `unexpected enclosure output: ${JSON.stringify(source)}`);
    return {
        lower: {
            coefficient: BigInt(match[1]),
            exponentTwo: Number.parseInt(match[2], 10),
        },
        upper: {
            coefficient: BigInt(match[3]),
            exponentTwo: Number.parseInt(match[4], 10),
        },
    };
}

function dyadicCompareWithRational(dyadic, numerator, denominator) {
    assert(denominator > 0n, "denominator must be positive");
    const coefficient = dyadic.coefficient;
    const exponentTwo = dyadic.exponentTwo;
    let left;
    let right;
    if (exponentTwo >= 0) {
        left = coefficient * (1n << BigInt(exponentTwo)) * denominator;
        right = numerator;
    } else {
        left = coefficient * denominator;
        right = numerator * (1n << BigInt(-exponentTwo));
    }
    if (left < right) {
        return -1;
    }
    if (left > right) {
        return 1;
    }
    return 0;
}

function dyadicSquareCompareWithRational(dyadic, numerator, denominator) {
    return dyadicCompareWithRational(
        {
            coefficient: dyadic.coefficient * dyadic.coefficient,
            exponentTwo: dyadic.exponentTwo * 2,
        },
        numerator,
        denominator,
    );
}

function assert(condition, message) {
    if (!condition) {
        throw new Error(message);
    }
}

function sleep(milliseconds) {
    return new Promise((resolve) => setTimeout(resolve, milliseconds));
}
