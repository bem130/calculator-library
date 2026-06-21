import {
    createCalculator,
    defaultCalculationRequest,
    exactOnlyCalculationRequest,
    renderMathMl,
    renderPlainText,
    type ApiResult,
    type Calculation,
    type CalculationOutcome,
    type CalculationRequest,
    type CalculatorErrorDto,
    type DecimalRoundingMode,
    type ExactFormatPreference,
} from "@bem130/exact-calculator";
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
};

const assetBaseUrl = new URL(import.meta.env.BASE_URL, window.location.href);
const calculator = createCalculator({
    wasmGlueUrl: new URL("wasm/calculator_wasm.js", assetBaseUrl),
    wasmModuleUrl: new URL("wasm/calculator_wasm_bg.wasm", assetBaseUrl),
});
const app = document.querySelector<HTMLDivElement>("#app");

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
});
calculateButton.addEventListener("click", () => void calculate());
copyButton.addEventListener("click", () => void copyPlainText());
includeExact.addEventListener("change", () => {
    state.includeExact = includeExact.checked;
});
includeScientific.addEventListener("change", () => {
    state.includeScientific = includeScientific.checked;
});
includeEnclosure.addEventListener("change", () => {
    state.includeEnclosure = includeEnclosure.checked;
});
exactFormat.addEventListener("change", () => {
    state.exactFormat = exactFormat.value as ExactFormatPreference;
});
digits.addEventListener("change", () => {
    state.significantDigits = clamp(Number.parseInt(digits.value, 10), 1, 200);
    digits.value = String(state.significantDigits);
});
rounding.addEventListener("change", () => {
    state.roundingMode = rounding.value as DecimalRoundingMode;
});
for (const button of document.querySelectorAll<HTMLButtonElement>("[data-angle]")) {
    button.addEventListener("click", () => {
        state.angleUnit = button.dataset.angle as AngleUnit;
        syncControls();
    });
}
for (const button of keypad.querySelectorAll<HTMLButtonElement>("[data-key]")) {
    button.addEventListener("click", () => {
        insertText(button.dataset.key ?? "");
    });
}
window.addEventListener("keydown", (event) => {
    if ((event.metaKey || event.ctrlKey) && event.key === "Enter") {
        event.preventDefault();
        void calculate();
    }
});

syncControls();
void calculate();

async function calculate(): Promise<void> {
    state.busy = true;
    state.copied = false;
    renderStatus();
    const result = (await calculator).calculate(state.expression, buildRequest());
    state.result = result;
    state.busy = false;
    renderResult();
    renderStatus();
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
    } else {
        statusOutput.textContent = "";
    }
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

function insertText(text: string): void {
    const start = expressionInput.selectionStart;
    const end = expressionInput.selectionEnd;
    const before = state.expression.slice(0, start);
    const after = state.expression.slice(end);
    state.expression = `${before}${text}${after}`;
    syncControls();
    const cursor = start + text.length;
    expressionInput.focus();
    expressionInput.setSelectionRange(cursor, cursor);
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
