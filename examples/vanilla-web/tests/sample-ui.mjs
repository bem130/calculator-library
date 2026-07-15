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
        mainSource.includes("createWorkerCalculator("),
        "sample UI must calculate through the public worker API facade",
    );
    assert(
        mainSource.includes("<textarea id=\"expression-editor\"") && mainSource.includes("inputmode=\"none\""),
        "sample UI must use a native selection editor without requesting a touch keyboard",
    );
    assert(
        mainSource.includes("button.addEventListener(\"pointerdown\", (event) => event.preventDefault())"),
        "sample UI keys must keep focus on the in-app editor",
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
        await assertDesktopEditorInput(page);
        assert(await page.locator("#include-scientific").count() === 0, "output toggles remain");
        await assertPhase2Outputs(page);
        await assertExactFormatPreferences(page);

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
        await assertNondegenerateOuterAcos(page);
        await assertNegativeTinyExponential(page);
        await assertLargeNegativeExponential(page);
        await assertArbitraryBaseLogExp(page);
        await assertRationalPowers(page);
        await assertExtendedFunctions(page);

        await setExpression(page, "sin(1)^2+cos(1)^2");
        await page.click("#calculate");
        await waitForText(page, "#exact-output", "= 1");

        await setExpression(page, "");
        await page.click("#key-seven");
        await page.click("#key-plus");
        await page.click("#key-eight");
        await waitForEditorText(page, "7+8");
        await page.click("#calculate");
        await waitForText(page, "#exact-output", "= 15");

        await setExpression(page, "");
        await page.click("#key-shift");
        await page.click("#key-sin");
        await page.keyboard.type("1/2");
        await waitForEditorText(page, "asin(1/2)");
        await page.click("#calculate");
        await waitForText(page, "#exact-output", "= pi/6");

        await setExpression(page, "");
        await page.click("#key-sin");
        await page.click("#key-pi6");
        await waitForEditorText(page, "sin(pi/6)");
        await page.click("#calculate");
        await waitForText(page, "#exact-output", "= 1/2");

        assert(await page.locator("#cancel").isDisabled(), "cancel button stayed enabled");

        assert(browserErrors.length === 0, browserErrors.join("\n"));
    } finally {
        const mobileContext = await browser.newContext({
            viewport: { width: 390, height: 844 },
            hasTouch: true,
            isMobile: true,
        });
        try {
            const mobilePage = await mobileContext.newPage();
            const mobileErrors = [];
            mobilePage.on("console", (message) => {
                if (message.type() === "error") mobileErrors.push(`console: ${message.text()}`);
            });
            mobilePage.on("pageerror", (error) => mobileErrors.push(`pageerror: ${error.message}`));
            await mobilePage.goto(url);
            await assertTouchEditorInput(mobilePage, mobileErrors);
        } finally {
            await mobileContext.close();
        }
        await browser.close();
    }
}

async function assertDesktopEditorInput(page) {
    const editor = page.locator("#expression-editor");
    await editor.fill("123456789");
    const box = await editor.boundingBox();
    assert(box !== null, "expression editor has no layout box");
    const geometry = await editor.evaluate((element) => {
        const style = getComputedStyle(element);
        const canvas = document.createElement("canvas");
        const context = canvas.getContext("2d");
        if (context === null) throw new Error("canvas context unavailable");
        context.font = style.font;
        const left = Number.parseFloat(style.paddingLeft) + Number.parseFloat(style.borderLeftWidth);
        return {
            x: [2, 6].map((index) => left + context.measureText(element.value.slice(0, index)).width),
            y: Number.parseFloat(style.paddingTop) + Number.parseFloat(style.fontSize) / 2,
        };
    });

    await page.mouse.click(box.x + geometry.x[0], box.y + geometry.y);
    const clicked = await editor.evaluate((element) => element.selectionStart);
    assert(Math.abs(clicked - 2) <= 1, `mouse click did not place the caret near boundary 2: ${clicked}`);

    await page.mouse.move(box.x + geometry.x[0], box.y + geometry.y);
    await page.mouse.down();
    await page.mouse.move(box.x + geometry.x[1], box.y + geometry.y, { steps: 6 });
    await page.mouse.up();
    const dragged = await editor.evaluate((element) => [element.selectionStart, element.selectionEnd]);
    assert(dragged[1] > dragged[0], `mouse drag did not select text: ${dragged}`);

    await page.mouse.move(box.x + geometry.x[1], box.y + geometry.y);
    await page.mouse.down();
    await page.mouse.move(box.x + geometry.x[0], box.y + geometry.y, { steps: 6 });
    await page.mouse.up();
    const reverseDragged = await editor.evaluate((element) => [
        element.selectionStart,
        element.selectionEnd,
        element.selectionDirection,
    ]);
    assert(reverseDragged[1] > reverseDragged[0], `backward mouse drag did not select text: ${reverseDragged}`);

    await page.mouse.move(box.x + geometry.x[1], box.y + geometry.y);
    await page.mouse.down();
    await page.mouse.move(box.x - 20, box.y + geometry.y, { steps: 6 });
    await page.mouse.up();
    const outsideDragged = await editor.evaluate((element) => [element.selectionStart, element.selectionEnd]);
    assert(outsideDragged[1] > outsideDragged[0], "selection stopped when the pointer left the editor");

    await page.click("#key-plus");
    const expectedReplacement = `123456789`.slice(0, outsideDragged[0]) + "+" + `123456789`.slice(outsideDragged[1]);
    assert(await editor.inputValue() === expectedReplacement, "keypad insertion did not replace the selection");
    await page.click("#key-left");
    await page.keyboard.press("Home");
    await page.keyboard.press("ArrowRight");
    await page.keyboard.press("End");
    const end = await editor.evaluate((element) => element.selectionStart);
    assert(end === (await editor.inputValue()).length, "standard keyboard cursor movement failed");

    await editor.fill("a😀e\u0301z");
    await editor.evaluate((element) => element.setSelectionRange(3, 3));
    await page.click("#key-backspace");
    assert(await editor.inputValue() === "ae\u0301z", "keypad backspace split a Unicode grapheme");
    await editor.evaluate((element) => element.setSelectionRange(1, 1));
    await page.click("#key-right");
    const graphemeEnd = await editor.evaluate((element) => element.selectionStart);
    assert(graphemeEnd === 3, `keypad arrow split a combining grapheme: ${graphemeEnd}`);

    const longExpression = "1234567890".repeat(20);
    await editor.fill(longExpression);
    await editor.press("End");
    const endScroll = await editor.evaluate((element) => element.scrollLeft);
    assert(endScroll > 0, "long expression did not scroll to its caret");
    await editor.press("Home");
    await page.click("#key-right");
    const startScroll = await editor.evaluate((element) => element.scrollLeft);
    assert(startScroll < endScroll, "keypad cursor movement did not reveal the native caret");

    await editor.focus();
    await page.keyboard.down("Shift");
    await page.keyboard.press("ArrowRight");
    await page.keyboard.up("Shift");
    const shifted = await editor.evaluate((element) => [element.selectionStart, element.selectionEnd]);
    assert(shifted[1] > shifted[0], "Shift+ArrowRight did not extend the native selection");

    await editor.fill("");
    await page.locator("#key-plus").focus();
    await page.keyboard.press("Enter");
    assert(await editor.inputValue() === "+", "focused keypad button did not keep native Enter activation");
    assert(await page.locator("#key-left").getAttribute("aria-label") === "Move cursor left", "left key lacks an accessible name");

    await editor.evaluate((element) => {
        element.value = "before";
        element.dispatchEvent(new CompositionEvent("compositionstart", { bubbles: true, data: "未" }));
        element.value = "before未";
        element.dispatchEvent(new InputEvent("input", { bubbles: true, data: "未", inputType: "insertCompositionText", isComposing: true }));
    });
    await page.locator("#key-plus").evaluate((button) => button.click());
    assert(await editor.inputValue() === "before未", "keypad mutated an active composition");
    await editor.evaluate((element) => {
        element.dispatchEvent(new CompositionEvent("compositionend", { bubbles: true, data: "未" }));
        element.dispatchEvent(new InputEvent("input", { bubbles: true, data: "未", inputType: "insertText" }));
    });
    await page.click("#key-plus");
    assert(await editor.inputValue() === "before未+", "composition was not committed exactly once");

    await page.click("#key-clear");
    await page.keyboard.type("sqrt(2)");
    await waitForEditorText(page, "sqrt(2)");
    await page.click("#calculate");
}

async function assertTouchEditorInput(page, browserErrors) {
    const editor = page.locator("#expression-editor");
    await editor.fill("123456789");
    const box = await editor.boundingBox();
    assert(box !== null, "mobile expression editor has no layout box");
    const geometry = await editor.evaluate((element) => {
        const style = getComputedStyle(element);
        const canvas = document.createElement("canvas");
        const context = canvas.getContext("2d");
        if (context === null) throw new Error("canvas context unavailable");
        context.font = style.font;
        return {
            x: [2, 4, 7].map(
                (index) => Number.parseFloat(style.paddingLeft) + context.measureText(element.value.slice(0, index)).width,
            ),
            y: Number.parseFloat(style.paddingTop) + Number.parseFloat(style.fontSize) / 2,
        };
    });
    await page.touchscreen.tap(box.x + geometry.x[1], box.y + geometry.y);
    const cursor = await editor.evaluate((element) => element.selectionStart);
    assert(Math.abs(cursor - 4) <= 1, `touch did not place the caret near boundary 4: ${cursor}`);
    const client = await page.context().newCDPSession(page);
    await editor.evaluate((element) => element.setSelectionRange(2, 7, "forward"));
    const plusBox = await page.locator("#key-plus").boundingBox();
    assert(plusBox !== null, "mobile plus key has no layout box");
    await page.touchscreen.tap(plusBox.x + plusBox.width / 2, plusBox.y + plusBox.height / 2);
    assert(await editor.inputValue() === "12+89", "touch keypad did not replace the mobile selection exactly once");

    await editor.fill("1234567890".repeat(20));
    await client.send("Input.synthesizeScrollGesture", {
        x: box.x + box.width / 2,
        y: box.y + geometry.y,
        xDistance: -box.width,
        yDistance: 0,
        gestureSourceType: "touch",
        speed: 800,
    });
    const touchScroll = await editor.evaluate((element) => element.scrollLeft);
    assert(touchScroll > 0, "touch drag did not horizontally scroll a long expression");
    const leftBox = await page.locator("#key-left").boundingBox();
    const rightBox = await page.locator("#key-right").boundingBox();
    assert(leftBox !== null && rightBox !== null, "mobile arrow keys have no layout boxes");
    const originalCursor = await editor.evaluate((element) => element.selectionStart);
    await page.touchscreen.tap(leftBox.x + leftBox.width / 2, leftBox.y + leftBox.height / 2);
    const leftCursor = await editor.evaluate((element) => element.selectionStart);
    assert(leftCursor === originalCursor - 1, `one touch left gesture moved ${originalCursor - leftCursor} positions`);
    await page.touchscreen.tap(rightBox.x + rightBox.width / 2, rightBox.y + rightBox.height / 2);
    const rightCursor = await editor.evaluate((element) => element.selectionStart);
    assert(rightCursor === originalCursor, `one touch right gesture did not restore the caret: ${rightCursor}`);
    const beforeInsert = await editor.inputValue();
    await page.touchscreen.tap(plusBox.x + plusBox.width / 2, plusBox.y + plusBox.height / 2);
    assert(
        (await editor.inputValue()).length === beforeInsert.length + 1,
        "one touch keypad gesture did not produce exactly one edit",
    );
    assert(await editor.getAttribute("inputmode") === "none", "touch editor requested a soft keyboard");
    assert(browserErrors.length === 0, browserErrors.join("\n"));
}

async function assertPhase2Outputs(page) {
    await setExpression(page, "0.1 + 0.2");
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

async function assertExactFormatPreferences(page) {
    await setExpression(page, "0.1 + 0.2");
    await selectExactFormat(page, "finiteDecimal");
    await page.click("#calculate");
    await waitForText(page, "#exact-kind", "FINITE DECIMAL");
    await waitForText(page, "#exact-output", "= 0.3");

    await setExpression(page, "1/3");
    await page.click("#calculate");
    await waitForText(page, "#exact-kind", "RATIONAL");
    await waitForText(page, "#exact-output", "= 1/3");

    await selectExactFormat(page, "mixedFraction");
    await setExpression(page, "7/3");
    await page.click("#calculate");
    await waitForText(page, "#exact-kind", "RATIONAL");
    await waitForText(page, "#exact-output", "= 2 1/3");

    await selectExactFormat(page, "auto");
    await setExpression(page, "0.1 + 0.2");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 3/10");
}

async function assertInitialExpLog(page) {
    await setExpression(page, "exp(1)");
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

    await setExpression(page, "ln(1)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 0");

    await setExpression(page, "exp(ln(2))");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 2");

    await setExpression(page, "exp(ln(sqrt(2)))");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= sqrt(2)");
    await waitForText(page, "#exact-kind", "RADICAL");

    await setExpression(page, "exp(ln(sqrt(2)+sqrt(3)))");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= sqrt(2) + sqrt(3)");
    await waitForText(page, "#exact-kind", "RADICAL");

    await setExpression(page, "ln(exp(2))");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 2");

    await setExpression(page, "ln(exp(-sqrt(2)))");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= -sqrt(2)");
    await waitForText(page, "#exact-kind", "RADICAL");
}

async function assertLargeNegativeExponential(page) {
    for (const source of ["exp(-10000)", "e^(-10000)"]) {
        await setExpression(page, source);
        await page.click("#calculate");
        await waitForText(page, "#exact-output", "= exp(-10000)");
        await waitForText(page, "#scientific-output", "≈ 1.1355 × 10^-4343");
        await waitForText(
            page,
            "#enclosure-output",
            "∈ [1.1354 × 10^-4343, 1.1355 × 10^-4343]",
        );
    }

    const cancellationState = await page.evaluate(() => {
        const calculate = document.querySelector("#calculate");
        const cancel = document.querySelector("#cancel");
        if (!(calculate instanceof HTMLButtonElement) || !(cancel instanceof HTMLButtonElement)) {
            throw new Error("calculation controls are unavailable");
        }
        calculate.click();
        const enabledWhileActive = !cancel.disabled;
        cancel.click();
        return { enabledWhileActive, disabledAfterCancel: cancel.disabled };
    });
    assert(cancellationState.enabledWhileActive, "active calculation did not enable cancellation");
    assert(cancellationState.disabledAfterCancel, "cancel button remained enabled after cancellation");
    await waitForText(page, "#status", "Canceled");
}

async function assertNondegenerateOuterAcos(page) {
    await setExpression(page, "acos(5/8)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= acos(5/8)");
    const exactMidAcosInterval = parseDecimalScientificInterval(
        await textContent(page, "#enclosure-output"),
    );
    assert(
        rationalCompareWithRational(exactMidAcosInterval.lower, 0n, 1n) > 0 &&
            rationalCompareWithRational(exactMidAcosInterval.upper, 2n, 1n) < 0,
        "exact mid acos enclosure must remain between zero and two radians",
    );

    await setExpression(page, "acos(3/8)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= acos(3/8)");
    const exactCentralAcosInterval = parseDecimalScientificInterval(
        await textContent(page, "#enclosure-output"),
    );
    assert(
        rationalCompareWithRational(exactCentralAcosInterval.lower, 0n, 1n) > 0 &&
            rationalCompareWithRational(exactCentralAcosInterval.upper, 2n, 1n) < 0,
        "exact central acos enclosure must remain between zero and two radians",
    );

    await setExpression(page, "acos(3/4)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= acos(3/4)");
    const exactAcosInterval = parseDecimalScientificInterval(
        await textContent(page, "#enclosure-output"),
    );
    assert(
        rationalCompareWithRational(exactAcosInterval.lower, 0n, 1n) > 0 &&
            rationalCompareWithRational(exactAcosInterval.upper, 2n, 1n) < 0,
        "exact outer acos enclosure must remain between zero and two radians",
    );

    await setExpression(page, "asin(3/4)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= asin(3/4)");
    const exactAsinInterval = parseDecimalScientificInterval(
        await textContent(page, "#enclosure-output"),
    );
    assert(
        rationalCompareWithRational(exactAsinInterval.lower, 0n, 1n) > 0 &&
            rationalCompareWithRational(exactAsinInterval.upper, 2n, 1n) < 0,
        "exact transformed asin enclosure must remain between zero and two radians",
    );

    await setExpression(page, "asin((1+sin(1))/3)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= asin(1/3*sin(1)+1/3)");
    const midAsinInterval = parseDecimalScientificInterval(
        await textContent(page, "#enclosure-output"),
    );
    assert(
        rationalCompareWithRational(midAsinInterval.lower, 0n, 1n) > 0 &&
            rationalCompareWithRational(midAsinInterval.upper, 2n, 1n) < 0,
        "mid-transform asin enclosure must remain between zero and two radians",
    );

    await setExpression(page, "asin((2+sin(1))/3)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= asin(1/3*sin(1)+2/3)");
    const asinInterval = parseDecimalScientificInterval(
        await textContent(page, "#enclosure-output"),
    );
    assert(
        rationalCompareWithRational(asinInterval.lower, 0n, 1n) > 0,
        "transformed asin enclosure must remain positive",
    );
    assert(
        rationalCompareWithRational(asinInterval.upper, 2n, 1n) < 0,
        "transformed asin enclosure must remain below two radians",
    );

    await setExpression(page, "asin((-2-sin(1))/3)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= -asin(2/3+1/3*sin(1))");
    const negativeAsinInterval = parseDecimalScientificInterval(
        await textContent(page, "#enclosure-output"),
    );
    assert(
        rationalCompareWithRational(negativeAsinInterval.lower, -2n, 1n) > 0,
        "negative transformed asin enclosure must remain above minus two radians",
    );
    assert(
        rationalCompareWithRational(negativeAsinInterval.upper, 0n, 1n) < 0,
        "negative transformed asin enclosure must remain negative",
    );

    await setExpression(page, "acos((2+sin(1))/3)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= acos(1/3*sin(1)+2/3)");
    await waitForText(page, "#scientific-state", "5 digits");
    await waitForText(page, "#enclosure-state", "DECIMAL SCIENTIFIC");
    const interval = parseDecimalScientificInterval(await textContent(page, "#enclosure-output"));
    assert(
        rationalCompareWithRational(interval.lower, 0n, 1n) > 0,
        "outer acos enclosure must stay positive",
    );
    assert(
        rationalCompareWithRational(interval.upper, 2n, 1n) < 0,
        "outer acos enclosure must remain below two radians",
    );

    await setExpression(page, "acos((-6+sin(1))/7)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= acos(1/7*sin(1)-6/7)");
    await waitForText(page, "#scientific-state", "5 digits");
    const negativeInterval = parseDecimalScientificInterval(
        await textContent(page, "#enclosure-output"),
    );
    assert(
        rationalCompareWithRational(negativeInterval.lower, 2n, 1n) > 0,
        "negative outer acos enclosure must remain above two radians",
    );
    assert(
        rationalCompareWithRational(negativeInterval.upper, 4n, 1n) < 0,
        "negative outer acos enclosure must remain below four radians",
    );

    await setExpression(page, "acos((-1+sin(1))/3)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= acos(1/3*sin(1)-1/3)");
    const centralInterval = parseDecimalScientificInterval(
        await textContent(page, "#enclosure-output"),
    );
    assert(
        rationalCompareWithRational(centralInterval.lower, 1n, 1n) > 0,
        "negative central acos enclosure must remain above one radian",
    );
    assert(
        rationalCompareWithRational(centralInterval.upper, 3n, 1n) < 0,
        "negative central acos enclosure must remain below three radians",
    );

    await setExpression(page, "acos((1+sin(1))/4)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= acos(1/4*sin(1)+1/4)");
    const positiveCentralInterval = parseDecimalScientificInterval(
        await textContent(page, "#enclosure-output"),
    );
    assert(
        rationalCompareWithRational(positiveCentralInterval.lower, 0n, 1n) > 0,
        "positive central acos enclosure must remain positive",
    );
    assert(
        rationalCompareWithRational(positiveCentralInterval.upper, 2n, 1n) < 0,
        "positive central acos enclosure must remain below two radians",
    );
}

async function assertNegativeTinyExponential(page) {
    await setExpression(page, "exp(-1/1267650600228229401496703205376)");
    await page.click("#calculate");
    await waitForText(
        page,
        "#exact-output",
        "= exp(-1/1267650600228229401496703205376)",
    );
    await waitForText(page, "#scientific-state", "5 digits");
    await waitForText(page, "#enclosure-state", "DECIMAL SCIENTIFIC");
    const interval = parseDecimalScientificInterval(await textContent(page, "#enclosure-output"));
    assert(
        rationalCompareWithRational(interval.lower, 0n, 1n) > 0,
        "negative tiny exponential enclosure must stay strictly positive",
    );
    assert(
        rationalCompareWithRational(interval.upper, 1n, 1n) <= 0,
        "negative tiny exponential enclosure must not exceed one",
    );
}

async function assertArbitraryBaseLogExp(page) {
    await setExpression(page, "log(8,2)");
    await page.waitForSelector("#input-preview math msub");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 3");

    for (const [source, expected] of [
        ["ln(8)/ln(2)", "= 3"],
        ["log(8,7)*log(7,3)*log(3,2)", "= 3"],
        ["log(2,10)+log(5,10)", "= 1"],
        ["ln(3)/ln(2)", "= log(3,2)"],
    ]) {
        await setExpression(page, source);
        await page.click("#calculate");
        await waitForText(page, "#exact-output", expected);
    }

    await setExpression(page, "ln(e)");
    await page.waitForSelector("#input-preview math mi");
    assert(
        await textContent(page, "#input-preview") === "ln(e)",
        "ln input preview did not render as natural log",
    );
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 1");

    await setExpression(page, "exp(3,2)");
    await page.waitForSelector("#input-preview math msup");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 8");

    await setExpression(page, "");
    await page.click("#key-shift");
    await page.click("#key-ln");
    await page.click("#key-eight");
    await page.click("#key-right");
    await page.click("#key-two");
    await waitForEditorText(page, "log(8,2)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 3");
}

async function assertExtendedFunctions(page) {
    const cases = [
        ["abs(-4)", "= 4"],
        ["floor(7/3)", "= 2"],
        ["5!", "= 120"],
        ["fact(5)", "= 120"],
        ["root(27,3)", "= 3"],
        ["perm(5,2)", "= 20"],
        ["comb(5,2)", "= 10"],
        ["mod(17,5)", "= 2"],
        ["gcd(12,18)", "= 6"],
        ["lcm(12,18)", "= 36"],
        ["sinh(0)", "= 0"],
        ["cosh(0)", "= 1"],
        ["tanh(0)", "= 0"],
        ["asinh(0)", "= 0"],
        ["acosh(1)", "= 0"],
        ["atanh(0)", "= 0"],
        ["exp(sinh(0))", "= 1"],
        ["ln(cosh(0))", "= 0"],
        ["sqrt(abs(-4))", "= 2"],
    ];

    for (const [source, expected] of cases) {
        await setExpression(page, source);
        await page.click("#calculate");
        await waitForText(page, "#exact-output", expected);
    }

    await setExpression(page, "");
    await page.click("#key-shift");
    await page.click("#key-sinh");
    await page.keyboard.type("0");
    await waitForEditorText(page, "asinh(0)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 0");
}

async function assertRationalPowers(page) {
    await setExpression(page, "(-8)^(1/3)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= -2");

    await setExpression(page, "(-8)^(2/3)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 4");

    await setExpression(page, "2^(1/2)");
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

    await setExpression(page, "2^sqrt(2)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 2^sqrt(2)");
    await waitForText(page, "#exact-kind", "GENERAL SYMBOLIC");
    await waitForText(page, "#scientific-state", "5 digits");
    await waitForText(page, "#enclosure-state", "DECIMAL SCIENTIFIC");

    await setExpression(page, "sqrt(2)^sqrt(2)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= sqrt(2)^sqrt(2)");
    await waitForText(page, "#exact-kind", "GENERAL SYMBOLIC");
    await waitForText(page, "#scientific-state", "5 digits");
    await waitForText(page, "#enclosure-state", "DECIMAL SCIENTIFIC");

    await setExpression(page, "2^(1/3)+1");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 2^(1/3)+1");
    await waitForText(page, "#exact-kind", "REAL ALGEBRAIC");
    await waitForText(page, "#scientific-state", "5 digits");
    await waitForText(page, "#enclosure-state", "DECIMAL SCIENTIFIC");

    await setExpression(page, "1-2^(1/3)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 1-2^(1/3)");
    await waitForText(page, "#exact-kind", "REAL ALGEBRAIC");

    await setExpression(page, "2^(1/3)/2+1");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 1/2*2^(1/3)+1");
    await waitForText(page, "#exact-kind", "REAL ALGEBRAIC");

    await setExpression(page, "1/2^(1/3)+1");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 1/2^(1/3)+1");
    await waitForText(page, "#exact-kind", "REAL ALGEBRAIC");

    await setExpression(page, "sqrt(2^(1/3))");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= sqrt(2^(1/3))");
    await waitForText(page, "#exact-kind", "REAL ALGEBRAIC");

    await setExpression(page, "(2^(1/3))^(2/5)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= (2^(1/3))^(2/5)");
    await waitForText(page, "#exact-kind", "REAL ALGEBRAIC");

    await setExpression(page, "2^(1/3)-2^(1/3)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 0");
    await waitForText(page, "#exact-kind", "INTEGER");

    await setExpression(page, "2^(1/3)/2^(1/3)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 1");
    await waitForText(page, "#exact-kind", "INTEGER");

    await setExpression(page, "(2^(1/3)-2^(1/3))+2^(1/3)-2^(1/3)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 0");
    await waitForText(page, "#exact-kind", "INTEGER");

    await setExpression(page, "(2^(1/3)/2^(1/3))*2^(1/3)/2^(1/3)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 1");
    await waitForText(page, "#exact-kind", "INTEGER");

    await setExpression(page, "(2^(1/3)-2^(1/3))+2^(1/3)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 2^(1/3)");
    await waitForText(page, "#exact-kind", "REAL ALGEBRAIC");

    await setExpression(page, "((2^(1/3)-2^(1/3))+2)^(1/3)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 2^(1/3)");
    await waitForText(page, "#exact-kind", "REAL ALGEBRAIC");
}

async function assertPiPartial(page) {
    await setExpression(page, "pi");
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
    await setExpression(page, "pi/6");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= pi/6");
    await waitForText(page, "#exact-kind", "RATIONAL PI MULTIPLE");
    await waitForText(page, "#scientific-state", "5 digits");
    await waitForText(page, "#enclosure-state", "DECIMAL SCIENTIFIC");
}

async function assertSpecialAngles(page) {
    await setExpression(page, "sin(pi/6)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 1/2");
    await waitForText(page, "#exact-kind", "RATIONAL");
    await waitForText(page, "#scientific-state", "5 digits");
    await waitForText(page, "#enclosure-state", "DECIMAL SCIENTIFIC");

    await setExpression(page, "tan(pi/2)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "domain.tangentPole");

    await setExpression(page, "sin(pi/4)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= sqrt(2)/2");
    await waitForText(page, "#exact-kind", "RADICAL");

    await setExpression(page, "sin(pi/12)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= sqrt(6)/4 - sqrt(2)/4");
    await waitForText(page, "#exact-kind", "RADICAL");

    await setExpression(page, "tan(pi/12)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 2 - sqrt(3)");
    await waitForText(page, "#exact-kind", "RADICAL");
    await waitForIdle(page);

    await setExpression(page, "tan(1)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= tan(1)");
    await waitForText(page, "#exact-kind", "GENERAL SYMBOLIC");
    await waitForText(page, "#enclosure-state", "DECIMAL SCIENTIFIC");
    await waitForIdle(page);

    await setExpression(page, "sin(1)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= sin(1)");
    await waitForText(page, "#exact-kind", "GENERAL SYMBOLIC");
    await waitForText(page, "#enclosure-state", "DECIMAL SCIENTIFIC");
    await waitForIdle(page);

    await setExpression(page, "sin(-1)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= -sin(1)");
    await waitForText(page, "#exact-kind", "GENERAL SYMBOLIC");
    await waitForText(page, "#enclosure-state", "DECIMAL SCIENTIFIC");
    await waitForIdle(page);

    await setExpression(page, "sin(pi+1/10)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= -sin(1/10)");
    await waitForText(page, "#exact-kind", "GENERAL SYMBOLIC");
    await waitForText(page, "#enclosure-state", "DECIMAL SCIENTIFIC");
    await waitForIdle(page);

    await setExpression(page, "cos(pi/2+1/10)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= -sin(1/10)");
    await waitForText(page, "#exact-kind", "GENERAL SYMBOLIC");
    await waitForText(page, "#enclosure-state", "DECIMAL SCIENTIFIC");
    await waitForIdle(page);

    await setExpression(page, "tan(pi/2+1/10)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= -1/tan(1/10)");
    await waitForText(page, "#exact-kind", "GENERAL SYMBOLIC");
    await waitForText(page, "#enclosure-state", "DECIMAL SCIENTIFIC");
    await waitForIdle(page);

    await setExpression(page, "exp(sin(-1))");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= exp(-sin(1))");
    await waitForText(page, "#exact-kind", "GENERAL SYMBOLIC");
    await waitForText(page, "#enclosure-state", "DECIMAL SCIENTIFIC");
    await waitForIdle(page);

    await setExpression(page, "cos(1)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= cos(1)");
    await waitForText(page, "#exact-kind", "GENERAL SYMBOLIC");
    await waitForText(page, "#enclosure-state", "DECIMAL SCIENTIFIC");
    await waitForIdle(page);

    await setExpression(page, "sin(2)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= sin(2)");
    await waitForText(page, "#exact-kind", "GENERAL SYMBOLIC");
    await waitForText(page, "#enclosure-state", "DECIMAL SCIENTIFIC");
    await waitForIdle(page);

    await setExpression(page, "tan(2)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= tan(2)");
    await waitForText(page, "#exact-kind", "GENERAL SYMBOLIC");
    await waitForText(page, "#enclosure-state", "DECIMAL SCIENTIFIC");
    await waitForIdle(page);

    await setExpression(page, "asin(1/2)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= pi/6");
    await waitForText(page, "#exact-kind", "RATIONAL PI MULTIPLE");

    await setExpression(page, "asin(sqrt(2)/2)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= pi/4");
    await waitForText(page, "#exact-kind", "RATIONAL PI MULTIPLE");

    await setExpression(page, "atan(sqrt(3))");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= pi/3");
    await waitForText(page, "#exact-kind", "RATIONAL PI MULTIPLE");
    await waitForIdle(page);

    await setExpression(page, "asin(1/3)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= asin(1/3)");
    await waitForText(page, "#exact-kind", "GENERAL SYMBOLIC");
    await waitForText(page, "#enclosure-state", "DECIMAL SCIENTIFIC");
    await waitForIdle(page);

    await setExpression(page, "sin(asin(1/3))");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 1/3");
    await waitForText(page, "#exact-kind", "RATIONAL");
    await waitForIdle(page);

    await setExpression(page, "cos(asin(sqrt(2)/3))");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= sqrt(7)/3");
    await waitForText(page, "#exact-kind", "RADICAL");
    await waitForIdle(page);

    await setExpression(page, "acos(1/3)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= acos(1/3)");
    await waitForText(page, "#exact-kind", "GENERAL SYMBOLIC");
    await waitForText(page, "#enclosure-state", "DECIMAL SCIENTIFIC");

    await setAngleUnit(page, "degree");
    await setExpression(page, "sin(30)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 1/2");
    await waitForText(page, "#exact-kind", "RATIONAL");

    await setExpression(page, "asin(1/2)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 30");
    await waitForText(page, "#exact-kind", "INTEGER");

    await setExpression(page, "sin(asin(1/3))");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 1/3");
    await waitForText(page, "#exact-kind", "RATIONAL");

    await setExpression(page, "cos(asin(sqrt(2)/3))");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= sqrt(7)/3");
    await waitForText(page, "#exact-kind", "RADICAL");

    await setExpression(page, "atan(sqrt(3))");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 60");
    await waitForText(page, "#exact-kind", "INTEGER");
}

async function assertSimpleRadicalAlgebra(page) {
    await setAngleUnit(page, "radian");

    await setExpression(page, "sqrt(2)*sqrt(2)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 2");
    await waitForText(page, "#exact-kind", "INTEGER");

    await setExpression(page, "sqrt(2)*sqrt(3)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= sqrt(6)");
    await waitForText(page, "#exact-kind", "RADICAL");

    await setExpression(page, "sqrt(6962)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 59*sqrt(2)");
    await waitForText(page, "#exact-kind", "RADICAL");

    await setExpression(page, "sin(pi/6)+sqrt(2)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 1/2 + sqrt(2)");
    await waitForText(page, "#exact-kind", "RADICAL");

    await setExpression(page, "sqrt(8)/sqrt(2)");
    await page.click("#calculate");
    await waitForText(page, "#exact-output", "= 2");
    await waitForText(page, "#exact-kind", "INTEGER");
}

async function assertIrrationalSqrtPartial(page) {
    await setExpression(page, "sqrt(2)");
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

async function setExpression(page, source) {
    const normalized = source.replaceAll(/\s+/gu, "");
    await page.click("#key-clear");
    await page.locator("#expression-editor").click();
    if (normalized.length > 0) {
        await page.keyboard.type(normalized);
    }
    await waitForEditorText(page, normalized);
}

async function waitForEditorText(page, expected) {
    await page.waitForFunction(
        (currentExpected) =>
            document.querySelector("#expression-editor")?.value === currentExpected,
        expected,
        { timeout: 15000 },
    );
}

async function openSettings(page) {
    if ((await page.locator("#settings-popover").getAttribute("data-open")) !== "true") {
        await page.click("#settings-toggle");
    }
}

async function selectExactFormat(page, value) {
    await openSettings(page);
    await page.selectOption("#exact-format", value);
}

async function setAngleUnit(page, angle) {
    await openSettings(page);
    await page.click(`button[data-angle="${angle}"]`);
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
