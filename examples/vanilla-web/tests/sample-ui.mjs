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

        await page.click("#copy");
        await waitForText(page, "#status", "Copied");
        const clipboardText = await page.evaluate(() => navigator.clipboard.readText());
        assert(clipboardText === "3/10", `clipboard text was ${JSON.stringify(clipboardText)}`);

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

function assert(condition, message) {
    if (!condition) {
        throw new Error(message);
    }
}

function sleep(milliseconds) {
    return new Promise((resolve) => setTimeout(resolve, milliseconds));
}
