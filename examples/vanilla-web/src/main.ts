import {
    createSession,
    defaultCalculationRequest,
    defaultInputPolicy,
    exactOnlyCalculationRequest,
    renderMathMl,
    renderPlainText,
    type ApiResult,
    type Calculation,
    type CalculationOutcome,
    type CalculationRequest,
    type CalculatorErrorDto,
    type CalculatorSession,
    type DecimalRoundingMode,
    type ExactFormatPreference,
    type InputActionDto,
    type InputPolicyDto,
    type SessionDispatchResult,
} from "@bem130/exact-calculator";
import { createWorkerCalculator } from "@bem130/exact-calculator/worker";
import "./styles.css";

type AngleUnit = CalculationRequest["semantics"]["angleUnit"];

type CalculatorState = {
    expression: string;
    angleUnit: AngleUnit;
    exactFormat: ExactFormatPreference;
    roundingMode: DecimalRoundingMode;
    significantDigits: number;
    includeExact: boolean;
    includeScientific: boolean;
    includeEnclosure: boolean;
    busy: boolean;
    result: ApiResult<CalculationOutcome>;
    copied: boolean;
    sessionSynced: boolean;
    statusMessage: string;
};

const state: CalculatorState = {
    expression: "0.1 + 0.2",
    angleUnit: "radian",
    exactFormat: "auto",
    roundingMode: "nearestTiesToEven",
    significantDigits: 50,
    includeExact: true,
    includeScientific: false,
    includeEnclosure: false,
    busy: false,
    result: {
        tag: "error",
        error: {
            tag: "unsupportedFeature",
            code: "evaluationEngine",
        },
    },
    copied: false,
    sessionSynced: false,
    statusMessage: "",
};

const assetBaseUrl = new URL(import.meta.env.BASE_URL, window.location.href);
const wasmOptions = {
    wasmGlueUrl: new URL("wasm/calculator_wasm.js", assetBaseUrl),
    wasmModuleUrl: new URL("wasm/calculator_wasm_bg.wasm", assetBaseUrl),
};
const workerCalculator = createWorkerCalculator(wasmOptions);
let activeSession: CalculatorSession | null = null;
let activeCalculation: ActiveCalculation | null = null;
let operationVersion = 0;
let keypadQueue = Promise.resolve();
const app = document.querySelector<HTMLDivElement>("#app");

type ActiveCalculation = {
    readonly operation: number;
    readonly abortController: AbortController;
};

if (app === null) {
    throw new Error("missing #app");
}

app.innerHTML = `
<main class="app-shell">
  <header class="topbar">
    <div>
      <h1>Exact Calculator</h1>
      <p>Rust core -> Wasm -> npm facade -> Vanilla TypeScript</p>
    </div>
    <nav aria-label="Project">
      <a href="https://github.com/bem130/calculator-library">GitHub</a>
    </nav>
  </header>

  <section class="workspace" aria-label="Calculator workspace">
    <section class="calculation-panel" aria-label="Calculation">
      <div class="expression-row">
        <label for="expression">Expression</label>
        <textarea id="expression" spellcheck="false" rows="3"></textarea>
      </div>
      <div class="action-row">
        <button class="primary" id="calculate" type="button">
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 12h14M13 6l6 6-6 6"/></svg>
          Calculate
        </button>
        <button id="cancel" type="button" disabled>
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M8 8h8v8H8z"/></svg>
          Cancel
        </button>
        <button id="copy" type="button">
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M8 8h10v12H8z"/><path d="M6 16H4V4h12v2"/></svg>
          Copy
        </button>
        <span id="status" role="status" aria-live="polite"></span>
      </div>

      <div class="result-stack" aria-live="polite">
        <section class="result-block exact-block" aria-label="Exact result">
          <div class="block-heading">
            <span>Exact</span>
            <span id="exact-kind"></span>
          </div>
          <output id="exact-output"></output>
          <div id="mathml-output" class="mathml-preview"></div>
        </section>
        <section class="result-block" aria-label="Scientific result">
          <div class="block-heading">
            <span>Scientific</span>
            <span id="scientific-state"></span>
          </div>
          <output id="scientific-output"></output>
        </section>
        <section class="result-block" aria-label="Certified enclosure">
          <div class="block-heading">
            <span>Certified interval</span>
            <span id="enclosure-state"></span>
          </div>
          <output id="enclosure-output"></output>
        </section>
      </div>
    </section>

    <aside class="settings-panel" aria-label="Calculation settings">
      <div class="settings-section">
        <h2>Settings</h2>
        <label class="switch-row">
          <input id="include-exact" type="checkbox" />
          <span>Exact output</span>
        </label>
        <label class="switch-row">
          <input id="include-scientific" type="checkbox" />
          <span>Scientific output</span>
        </label>
        <label class="switch-row">
          <input id="include-enclosure" type="checkbox" />
          <span>Certified interval</span>
        </label>
      </div>

      <div class="settings-section">
        <h2>Angle unit</h2>
        <div class="segmented" role="group" aria-label="Angle unit">
          <button data-angle="radian" type="button">rad</button>
          <button data-angle="degree" type="button">deg</button>
          <button data-angle="gradian" type="button">grad</button>
        </div>
      </div>

      <div class="settings-section">
        <h2>Exact format</h2>
        <select id="exact-format">
          <option value="auto">Auto</option>
          <option value="rational">Rational</option>
          <option value="finiteDecimal">Finite decimal</option>
          <option value="mixedFraction">Mixed fraction</option>
          <option value="symbolic">Symbolic</option>
        </select>
      </div>

      <div class="settings-grid">
        <label>
          <span>Digits</span>
          <input id="digits" type="number" min="1" max="200" step="1" />
        </label>
        <label>
          <span>Rounding</span>
          <select id="rounding">
            <option value="nearestTiesToEven">Ties to even</option>
            <option value="nearestTiesAwayFromZero">Ties away</option>
            <option value="towardPositiveInfinity">Toward +infinity</option>
            <option value="towardNegativeInfinity">Toward -infinity</option>
            <option value="towardZero">Toward zero</option>
            <option value="awayFromZero">Away from zero</option>
          </select>
        </label>
      </div>
    </aside>
  </section>

  <section class="keypad" aria-label="Expression keypad"></section>
</main>
`;

const expressionInput = required<HTMLTextAreaElement>("#expression");
const calculateButton = required<HTMLButtonElement>("#calculate");
const cancelButton = required<HTMLButtonElement>("#cancel");
const copyButton = required<HTMLButtonElement>("#copy");
const statusOutput = required<HTMLElement>("#status");
const exactKind = required<HTMLElement>("#exact-kind");
const exactOutput = required<HTMLOutputElement>("#exact-output");
const mathmlOutput = required<HTMLElement>("#mathml-output");
const scientificState = required<HTMLElement>("#scientific-state");
const scientificOutput = required<HTMLOutputElement>("#scientific-output");
const enclosureState = required<HTMLElement>("#enclosure-state");
const enclosureOutput = required<HTMLOutputElement>("#enclosure-output");
const includeExact = required<HTMLInputElement>("#include-exact");
const includeScientific = required<HTMLInputElement>("#include-scientific");
const includeEnclosure = required<HTMLInputElement>("#include-enclosure");
const exactFormat = required<HTMLSelectElement>("#exact-format");
const digits = required<HTMLInputElement>("#digits");
const rounding = required<HTMLSelectElement>("#rounding");
const keypad = required<HTMLElement>(".keypad");

const keys = [
    "7",
    "8",
    "9",
    "/",
    "(",
    ")",
    "4",
    "5",
    "6",
    "*",
    "^",
    "%",
    "1",
    "2",
    "3",
    "-",
    "pi",
    "e",
    "0",
    ".",
    "+",
    "sin(",
    "cos(",
    "sqrt(",
] as const;

keypad.innerHTML = keys
    .map((key) => `<button type="button" data-key="${key}">${key}</button>`)
    .join("");

expressionInput.addEventListener("input", () => {
    state.expression = expressionInput.value;
    invalidateSession();
});
calculateButton.addEventListener("click", () => void calculateFromSession());
cancelButton.addEventListener("click", cancelFromUi);
copyButton.addEventListener("click", () => void copyPlainText());
includeExact.addEventListener("change", () => {
    state.includeExact = includeExact.checked;
    invalidateSession();
});
includeScientific.addEventListener("change", () => {
    state.includeScientific = includeScientific.checked;
    invalidateSession();
});
includeEnclosure.addEventListener("change", () => {
    state.includeEnclosure = includeEnclosure.checked;
    invalidateSession();
});
exactFormat.addEventListener("change", () => {
    state.exactFormat = exactFormat.value as ExactFormatPreference;
    invalidateSession();
});
digits.addEventListener("change", () => {
    state.significantDigits = clamp(Number.parseInt(digits.value, 10), 1, 200);
    digits.value = String(state.significantDigits);
    invalidateSession();
});
rounding.addEventListener("change", () => {
    state.roundingMode = rounding.value as DecimalRoundingMode;
    invalidateSession();
});
for (const button of document.querySelectorAll<HTMLButtonElement>("[data-angle]")) {
    button.addEventListener("click", () => {
        state.angleUnit = button.dataset.angle as AngleUnit;
        invalidateSession();
        syncControls();
    });
}
for (const button of keypad.querySelectorAll<HTMLButtonElement>("[data-key]")) {
    button.addEventListener("click", () => enqueueKeyDispatch(button.dataset.key ?? ""));
}
window.addEventListener("keydown", (event) => {
    if ((event.metaKey || event.ctrlKey) && event.key === "Enter") {
        event.preventDefault();
        void calculateFromSession();
    }
});
window.addEventListener("pagehide", shutdown);

syncControls();
void calculateFromSession();

async function calculateFromSession(): Promise<void> {
    const operation = beginOperation();
    try {
        const session = await sessionForCurrentExpression(operation);
        if (session === null || !isCurrentOperation(operation)) {
            return;
        }
        await handleDispatchResult(session.dispatch({ tag: "evaluate" }), operation);
    } catch {
        if (isCurrentOperation(operation)) {
            await calculateDirectly(operation);
        }
    }
}

async function calculateDirectly(operation: number): Promise<void> {
    if (!isCurrentOperation(operation)) {
        return;
    }
    state.busy = true;
    state.copied = false;
    state.statusMessage = "";
    const result = await calculateWithWorker(state.expression, buildRequest(), operation);
    if (result === null) {
        return;
    }
    state.result = result;
    state.busy = false;
    renderResult();
    renderStatus();
}

async function dispatchKey(key: string): Promise<void> {
    const action = keyAction(key);
    if (action === null) {
        return;
    }
    const operation = beginOperation();
    const session = await sessionForCurrentExpression(operation);
    if (session === null || !isCurrentOperation(operation)) {
        return;
    }
    await handleDispatchResult(session.dispatch(action), operation);
    if (!isCurrentOperation(operation)) {
        return;
    }
    expressionInput.focus();
    expressionInput.setSelectionRange(expressionInput.value.length, expressionInput.value.length);
}

async function handleDispatchResult(result: SessionDispatchResult, operation: number): Promise<void> {
    if (!isCurrentOperation(operation)) {
        return;
    }
    state.expression = result.state.source;
    state.sessionSynced = true;
    syncControls();
    if (result.tag === "inputError") {
        statusOutput.textContent = formatLabel(result.error.code);
        return;
    }
    if (result.tag === "calculate") {
        state.busy = true;
        state.copied = false;
        state.statusMessage = "";
        const calculation = await calculateWithWorker(result.source, result.request, operation);
        if (calculation === null) {
            return;
        }
        state.result = calculation;
        activeSession?.applyResult(calculation);
        state.busy = false;
        renderResult();
        renderStatus();
    }
}

async function copyPlainText(): Promise<void> {
    const text = currentPlainText();
    if (text.length === 0) {
        return;
    }
    await navigator.clipboard.writeText(text);
    state.copied = true;
    renderStatus();
}

function enqueueKeyDispatch(key: string): void {
    keypadQueue = keypadQueue.then(() => dispatchKey(key)).catch((error: unknown) => {
        state.busy = false;
        statusOutput.textContent = error instanceof Error ? error.message : String(error);
    });
}

function beginOperation(): number {
    cancelActiveCalculation();
    operationVersion += 1;
    state.statusMessage = "";
    return operationVersion;
}

function invalidateSession(): void {
    disposeActiveSession();
    state.sessionSynced = false;
    state.busy = false;
    beginOperation();
    renderStatus();
}

function disposeActiveSession(): void {
    activeSession?.dispose?.();
    activeSession = null;
}

function cancelFromUi(): void {
    if (activeCalculation === null) {
        return;
    }
    cancelActiveCalculation();
    operationVersion += 1;
    state.busy = false;
    state.statusMessage = "Canceled";
    renderStatus();
}

function cancelActiveCalculation(): void {
    activeCalculation?.abortController.abort();
    activeCalculation = null;
}

function shutdown(): void {
    cancelActiveCalculation();
    disposeActiveSession();
    void workerCalculator.then((calculator) => calculator.terminate()).catch(() => undefined);
}

async function calculateWithWorker(
    source: string,
    request: CalculationRequest,
    operation: number,
): Promise<ApiResult<CalculationOutcome> | null> {
    if (!isCurrentOperation(operation)) {
        return null;
    }

    const abortController = new AbortController();
    activeCalculation = {
        operation,
        abortController,
    };
    state.busy = true;
    state.copied = false;
    state.statusMessage = "";
    renderStatus();

    const calculator = await workerCalculator;
    if (!isCurrentOperation(operation)) {
        clearActiveCalculation(operation);
        return null;
    }

    const result = await calculator.calculate(source, request, {
        signal: {
            tag: "abortSignal",
            signal: abortController.signal,
        },
    });
    clearActiveCalculation(operation);
    if (!isCurrentOperation(operation)) {
        return null;
    }
    return result;
}

function clearActiveCalculation(operation: number): void {
    if (activeCalculation?.operation === operation) {
        activeCalculation = null;
    }
}

function isCurrentOperation(operation: number): boolean {
    return operation === operationVersion;
}

function buildRequest(): CalculationRequest {
    return {
        ...defaultCalculationRequest,
        semantics: {
            ...defaultCalculationRequest.semantics,
            angleUnit: state.angleUnit,
        },
        exactOutput: state.includeExact
            ? {
                tag: "include",
                format: state.exactFormat,
            }
            : {
                tag: "omit",
            },
        scientificOutput: state.includeScientific
            ? {
                tag: "include",
                significantDigits: state.significantDigits,
                roundingMode: state.roundingMode,
            }
            : exactOnlyCalculationRequest.scientificOutput,
        enclosureOutput: state.includeEnclosure
            ? {
                tag: "include",
                format: "exactDyadic",
            }
            : exactOnlyCalculationRequest.enclosureOutput,
    };
}

function buildInputPolicy(): InputPolicyDto {
    return {
        ...defaultInputPolicy,
        calculationRequest: buildRequest(),
    };
}

async function sessionForCurrentExpression(operation: number): Promise<CalculatorSession | null> {
    if (activeSession !== null && state.sessionSynced) {
        return activeSession;
    }
    const expression = state.expression;
    const policy = buildInputPolicy();
    const session = await createSession(policy, wasmOptions);
    if (!isCurrentOperation(operation) || expression !== state.expression) {
        return null;
    }
    for (const action of actionsForExpression(expression)) {
        const result = session.dispatch(action);
        if (result.tag === "inputError") {
            throw new Error(result.error.code);
        }
    }
    disposeActiveSession();
    activeSession = session;
    state.sessionSynced = true;
    state.expression = session.getState().source;
    syncControls();
    return session;
}

function actionsForExpression(source: string): InputActionDto[] {
    const actions: InputActionDto[] = [];
    let cursor = 0;
    const functions = [
        ["asin(", "asin"],
        ["acos(", "acos"],
        ["atan(", "atan"],
        ["sqrt(", "sqrt"],
        ["sin(", "sin"],
        ["cos(", "cos"],
        ["tan(", "tan"],
        ["exp(", "exp"],
        ["log(", "log"],
    ] as const;

    while (cursor < source.length) {
        const ch = source[cursor];
        if (ch === undefined) {
            break;
        }
        if (/\s/u.test(ch)) {
            cursor += 1;
            continue;
        }
        const functionMatch = functions.find(([token]) => source.startsWith(token, cursor));
        if (functionMatch !== undefined) {
            actions.push({ tag: "function", value: functionMatch[1] });
            cursor += functionMatch[0].length;
            continue;
        }
        if (source.startsWith("pi", cursor)) {
            actions.push({ tag: "constant", value: "pi" });
            cursor += 2;
            continue;
        }
        if (source.startsWith("Ans", cursor)) {
            actions.push({ tag: "constant", value: "ans" });
            cursor += 3;
            continue;
        }
        const action = keyAction(ch);
        if (action === null) {
            throw new Error(`unsupported session token: ${ch}`);
        }
        actions.push(action);
        cursor += 1;
    }
    return actions;
}

function keyAction(key: string): InputActionDto | null {
    if (/^[0-9]$/u.test(key)) {
        return { tag: "digit", value: Number.parseInt(key, 10) };
    }
    switch (key) {
        case ".":
            return { tag: "decimalPoint" };
        case "+":
            return { tag: "binaryOperator", value: "add" };
        case "-":
            return { tag: "binaryOperator", value: "subtract" };
        case "*":
            return { tag: "binaryOperator", value: "multiply" };
        case "/":
            return { tag: "binaryOperator", value: "divide" };
        case "^":
            return { tag: "binaryOperator", value: "power" };
        case "%":
            return { tag: "percent" };
        case "(":
            return { tag: "openParenthesis" };
        case ")":
            return { tag: "closeParenthesis" };
        case "pi":
            return { tag: "constant", value: "pi" };
        case "e":
            return { tag: "constant", value: "e" };
        case "sin(":
            return { tag: "function", value: "sin" };
        case "cos(":
            return { tag: "function", value: "cos" };
        case "sqrt(":
            return { tag: "function", value: "sqrt" };
        default:
            return null;
    }
}

function renderResult(): void {
    if (state.result.tag === "error") {
        exactKind.textContent = formatLabel(state.result.error.tag);
        exactOutput.textContent = formatError(state.result.error);
        mathmlOutput.textContent = "";
        scientificState.textContent = "not available";
        scientificOutput.textContent = "Calculation did not complete.";
        enclosureState.textContent = "not available";
        enclosureOutput.textContent = "Calculation did not complete.";
        return;
    }

    const calculation = state.result.value.calculation;
    renderExact(calculation);
    renderScientific(calculation);
    renderEnclosure(calculation);
}

function renderExact(calculation: Calculation): void {
    if (calculation.exact.tag === "omitted") {
        exactKind.textContent = "omitted";
        exactOutput.textContent = "Exact output is disabled.";
        mathmlOutput.textContent = "";
        return;
    }
    exactKind.textContent = formatLabel(calculation.exact.value.representation);
    exactOutput.textContent = `= ${calculation.exact.value.plainText}`;
    mathmlOutput.innerHTML = `<math display="inline">${renderMathMl(calculation.exact.value.presentation)}</math>`;
}

function renderScientific(calculation: Calculation): void {
    switch (calculation.scientific.tag) {
        case "omitted":
            scientificState.textContent = "omitted";
            scientificOutput.textContent = "Enable scientific output to request decimal notation.";
            return;
        case "unavailable":
            scientificState.textContent = formatLabel(calculation.scientific.value.reason.tag);
            scientificOutput.textContent = "Phase 1 exact mode keeps this as unavailable.";
            return;
        case "included":
            scientificState.textContent = `${calculation.scientific.value.confirmedSignificantDigits} digits`;
            scientificOutput.textContent = `${calculation.scientific.value.significand}e${calculation.scientific.value.exponentTen}`;
            return;
    }
}

function renderEnclosure(calculation: Calculation): void {
    switch (calculation.enclosure.tag) {
        case "omitted":
            enclosureState.textContent = "omitted";
            enclosureOutput.textContent = "No certified interval was requested or produced.";
            return;
        case "included":
            enclosureState.textContent = formatLabel(calculation.enclosure.value.format);
            enclosureOutput.textContent = `[${calculation.enclosure.value.lower.coefficient} * 2^${calculation.enclosure.value.lower.exponentTwo}, ${calculation.enclosure.value.upper.coefficient} * 2^${calculation.enclosure.value.upper.exponentTwo}]`;
            return;
    }
}

function renderStatus(): void {
    if (state.busy) {
        statusOutput.textContent = "Calculating";
    } else if (state.copied) {
        statusOutput.textContent = "Copied";
    } else if (state.statusMessage.length > 0) {
        statusOutput.textContent = state.statusMessage;
    } else {
        statusOutput.textContent = "";
    }
    calculateButton.disabled = state.busy;
    cancelButton.disabled = activeCalculation === null;
    copyButton.disabled = state.busy;
}

function syncControls(): void {
    expressionInput.value = state.expression;
    includeExact.checked = state.includeExact;
    includeScientific.checked = state.includeScientific;
    includeEnclosure.checked = state.includeEnclosure;
    exactFormat.value = state.exactFormat;
    digits.value = String(state.significantDigits);
    rounding.value = state.roundingMode;
    for (const button of document.querySelectorAll<HTMLButtonElement>("[data-angle]")) {
        button.dataset.active = String(button.dataset.angle === state.angleUnit);
    }
}

function currentPlainText(): string {
    if (state.result.tag === "error") {
        return formatError(state.result.error);
    }
    const calculation = state.result.value.calculation;
    if (calculation.exact.tag === "included") {
        return renderPlainText(calculation.exact.value.presentation);
    }
    return "";
}

function formatError(error: CalculatorErrorDto): string {
    return `${error.tag}.${error.code}`;
}

function formatLabel(value: string): string {
    return value.replaceAll(/([a-z])([A-Z])/g, "$1 $2").toUpperCase();
}

function clamp(value: number, min: number, max: number): number {
    if (!Number.isFinite(value)) {
        return min;
    }
    return Math.min(Math.max(value, min), max);
}

function required<T extends Element>(selector: string): T {
    const element = document.querySelector<T>(selector);
    if (element === null) {
        throw new Error(`missing element: ${selector}`);
    }
    return element;
}
