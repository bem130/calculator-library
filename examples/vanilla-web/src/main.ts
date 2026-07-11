import {
    createCalculator,
    defaultCalculationRequest,
    renderMathMl,
    renderPlainText,
    renderResultRelationMathMl,
    renderResultRelationPlainText,
    type ApiResult,
    type Calculation,
    type CalculationOutcome,
    type CalculationRequest,
    type CalculatorErrorDto,
    type DecimalRoundingMode,
    type ExactFormatPreference,
} from "@bem130/exact-calculator";
import { createWorkerCalculator } from "@bem130/exact-calculator/worker";
import "./styles.css";

type AngleUnit = CalculationRequest["semantics"]["angleUnit"];

type CalculatorState = {
    expression: string;
    shift: boolean;
    settingsOpen: boolean;
    angleUnit: AngleUnit;
    exactFormat: ExactFormatPreference;
    roundingMode: DecimalRoundingMode;
    significantDigits: number;
    busy: boolean;
    result: ApiResult<CalculationOutcome>;
    copied: boolean;
    statusMessage: string;
};

type ActiveCalculation = {
    readonly operation: number;
    readonly abortController: AbortController;
};

type KeyLayer = {
    readonly label: string;
    readonly insert: string;
    readonly cursorFromEnd?: number;
};

type Command =
    | { readonly tag: "insert"; readonly layer: KeyLayer }
    | { readonly tag: "move"; readonly delta: -1 | 1 }
    | { readonly tag: "backspace" }
    | { readonly tag: "delete" }
    | { readonly tag: "clear" }
    | { readonly tag: "shift" }
    | { readonly tag: "calculate" };

type KeySpec = {
    readonly id: string;
    readonly primary: KeyLayer | Command;
    readonly shift?: KeyLayer | Command;
    readonly tone?: "action" | "operator" | "equals" | "shift";
};

const state: CalculatorState = {
    expression: "sqrt(2)",
    shift: false,
    settingsOpen: false,
    angleUnit: "radian",
    exactFormat: "auto",
    roundingMode: "nearestTiesToEven",
    significantDigits: 5,
    busy: false,
    result: {
        tag: "error",
        error: {
            tag: "unsupportedFeature",
            code: "evaluationEngine",
        },
    },
    copied: false,
    statusMessage: "",
};

const workerCalculator = createWorkerCalculator();
const directCalculator = createCalculator();
const graphemeSegmenter = new Intl.Segmenter(undefined, { granularity: "grapheme" });
let activeCalculation: ActiveCalculation | null = null;
let operationVersion = 0;
let previewVersion = 0;
let editorIsComposing = false;

const app = document.querySelector<HTMLDivElement>("#app");
if (app === null) {
    throw new Error("missing #app");
}

app.innerHTML = `
<main class="app-shell">
  <header class="topbar">
    <h1>Exact Calculator</h1>
    <div class="topbar-actions">
      <button id="settings-toggle" class="icon-button" type="button" aria-expanded="false" aria-controls="settings-popover" title="Settings">
        <span aria-hidden="true">⚙</span>
      </button>
      <a href="https://github.com/bem130/calculator-library">GitHub</a>
    </div>
    <section id="settings-popover" class="settings-popover" aria-label="Calculation settings">
      <div class="settings-section">
        <span>Angle</span>
        <div class="segmented" role="group" aria-label="Angle unit">
          <button data-angle="radian" type="button">rad</button>
          <button data-angle="degree" type="button">deg</button>
          <button data-angle="gradian" type="button">grad</button>
        </div>
      </div>
      <label>
        <span>Exact</span>
        <select id="exact-format">
          <option value="auto">Auto</option>
          <option value="rational">Rational</option>
          <option value="finiteDecimal">Finite decimal</option>
          <option value="mixedFraction">Mixed fraction</option>
          <option value="symbolic">Symbolic</option>
        </select>
      </label>
      <label>
        <span>Digits</span>
        <input id="digits" type="number" min="1" max="200" step="1" inputmode="numeric" />
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
    </section>
  </header>

  <section class="calculator-grid" aria-label="Calculator">
    <section class="editor-panel" aria-label="Expression">
      <textarea id="expression-editor" class="expression-editor" aria-label="Expression" rows="1" wrap="off" inputmode="none" enterkeyhint="done" autocomplete="off" autocapitalize="off" spellcheck="false" placeholder="0"></textarea>
      <div id="input-preview" class="input-preview" aria-label="Input preview"></div>
      <div class="action-row">
        <button class="primary" id="calculate" type="button">=</button>
        <button id="cancel" type="button" disabled>Stop</button>
        <button id="copy" type="button">Copy</button>
        <span id="status" role="status" aria-live="polite"></span>
      </div>
    </section>

    <section class="result-panel" aria-label="Results">
      <section class="result-block exact-block" aria-label="Exact output">
        <div class="block-heading">
          <span>Exact output</span>
          <span id="exact-kind"></span>
        </div>
        <output id="exact-output"></output>
        <div id="mathml-output" class="mathml-preview"></div>
      </section>
      <section class="result-block" aria-label="Scientific output">
        <div class="block-heading">
          <span>Scientific output</span>
          <span id="scientific-state"></span>
        </div>
        <output id="scientific-output"></output>
      </section>
      <section class="result-block" aria-label="Certified interval">
        <div class="block-heading">
          <span>Certified interval</span>
          <span id="enclosure-state"></span>
        </div>
        <output id="enclosure-output"></output>
      </section>
    </section>
  </section>

  <section id="keypad" class="keypad" aria-label="Expression keypad"></section>
</main>
`;

const editor = required<HTMLTextAreaElement>("#expression-editor");
const inputPreview = required<HTMLElement>("#input-preview");
const settingsToggle = required<HTMLButtonElement>("#settings-toggle");
const settingsPopover = required<HTMLElement>("#settings-popover");
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
const exactFormat = required<HTMLSelectElement>("#exact-format");
const digits = required<HTMLInputElement>("#digits");
const rounding = required<HTMLSelectElement>("#rounding");
const keypad = required<HTMLElement>("#keypad");

const keys: readonly KeySpec[] = [
    { id: "shift", primary: { tag: "shift" }, tone: "shift" },
    { id: "left", primary: { tag: "move", delta: -1 }, tone: "action" },
    { id: "right", primary: { tag: "move", delta: 1 }, tone: "action" },
    { id: "backspace", primary: { tag: "backspace" }, shift: { tag: "delete" }, tone: "action" },
    { id: "clear", primary: { tag: "clear" }, tone: "action" },
    { id: "equals", primary: { tag: "calculate" }, tone: "equals" },

    key("sin", math("sin"), "sin()", 1, math("sin<sup>-1</sup>"), "asin()", 1),
    key("cos", math("cos"), "cos()", 1, math("cos<sup>-1</sup>"), "acos()", 1),
    key("tan", math("tan"), "tan()", 1, math("tan<sup>-1</sup>"), "atan()", 1),
    key("sinh", math("sinh"), "sinh()", 1, math("sinh<sup>-1</sup>"), "asinh()", 1),
    key("cosh", math("cosh"), "cosh()", 1, math("cosh<sup>-1</sup>"), "acosh()", 1),
    key("tanh", math("tanh"), "tanh()", 1, math("tanh<sup>-1</sup>"), "atanh()", 1),

    key("ln", math("ln"), "ln()", 1, math("log<sub>b</sub>"), "log(,)", 2),
    key("exp", math("e<sup>x</sup>"), "exp()", 1, math("a<sup>x</sup>"), "exp(,)", 2),
    key("pow", math("x<sup>a</sup>"), "^()", 1, math("x<sup>2</sup>"), "^2", 0, "operator"),
    key("sqrt", math("√x"), "sqrt()", 1, math("<sup>n</sup>√x"), "root(,)", 2),
    key("abs", math("|x|"), "abs()", 1, math("⌊x⌋"), "floor()", 1),
    key("fact", math("x!"), "!", 0, math("mod"), "mod(,)", 2),

    key("seven", "7", "7"),
    key("eight", "8", "8"),
    key("nine", "9", "9"),
    key("divide", math("÷"), "/", 0, math("nPr"), "perm(,)", 2, "operator"),
    key("gcd", math("gcd"), "gcd(,)", 2, math("lcm"), "lcm(,)", 2),
    key("pi6", math("π/6"), "pi/6", 0, "30°", "30", 0),

    key("four", "4", "4"),
    key("five", "5", "5"),
    key("six", "6", "6"),
    key("multiply", math("×"), "*", 0, math("nCr"), "comb(,)", 2, "operator"),
    key("pi", math("π"), "pi", 0, math("e"), "e", 0),
    key("pi4", math("π/4"), "pi/4", 0, "45°", "45", 0),

    key("one", "1", "1"),
    key("two", "2", "2"),
    key("three", "3", "3"),
    key("minus", math("−"), "-", 0, math("%"), "%", 0, "operator"),
    key("open", "(", "(", 0, math("["), "(", 0),
    key("pi3", math("π/3"), "pi/3", 0, "60°", "60", 0),

    key("zero", "0", "0"),
    key("decimal", ".", "."),
    key("comma", ",", ","),
    key("plus", math("+"), "+", 0, math("±"), "-", 0, "operator"),
    key("close", ")", ")", 0, math("]"), ")", 0),
    key("pi2", math("π/2"), "pi/2", 0, "90°", "90", 0),
];

keypad.innerHTML = keys
    .map((spec) => `<button id="key-${spec.id}" type="button" data-key="${spec.id}" data-tone="${spec.tone ?? ""}"></button>`)
    .join("");

editor.addEventListener("input", handleEditorInput);
editor.addEventListener("keydown", handleKeyboard);
editor.addEventListener("compositionstart", () => {
    editorIsComposing = true;
});
editor.addEventListener("compositionend", () => {
    editorIsComposing = false;
    commitEditorInput();
});
settingsToggle.addEventListener("click", () => {
    state.settingsOpen = !state.settingsOpen;
    syncControls();
});
document.addEventListener("pointerdown", (event) => {
    const target = event.target;
    if (!(target instanceof Node)) {
        return;
    }
    if (!state.settingsOpen || settingsPopover.contains(target) || settingsToggle.contains(target)) {
        return;
    }
    state.settingsOpen = false;
    syncControls();
});
calculateButton.addEventListener("click", () => void calculateExpression());
cancelButton.addEventListener("click", cancelFromUi);
copyButton.addEventListener("click", () => void copyPlainText());
exactFormat.addEventListener("change", () => {
    state.exactFormat = exactFormat.value as ExactFormatPreference;
    invalidatePreview();
});
digits.addEventListener("change", () => {
    state.significantDigits = clamp(Number.parseInt(digits.value, 10), 1, 200);
    digits.value = String(state.significantDigits);
    invalidatePreview();
});
rounding.addEventListener("change", () => {
    state.roundingMode = rounding.value as DecimalRoundingMode;
    invalidatePreview();
});
for (const button of document.querySelectorAll<HTMLButtonElement>("[data-angle]")) {
    button.addEventListener("click", () => {
        state.angleUnit = button.dataset.angle as AngleUnit;
        invalidatePreview();
        syncControls();
    });
}
for (const button of keypad.querySelectorAll<HTMLButtonElement>("[data-key]")) {
    button.addEventListener("pointerdown", (event) => event.preventDefault());
    button.addEventListener("click", () => {
        runKey(button.dataset.key ?? "");
        editor.focus({ preventScroll: true });
    });
}
window.addEventListener("pagehide", shutdown);

syncControls();
void calculateExpression();

function key(
    id: string,
    label: string,
    insert: string,
    cursorFromEnd = 0,
    shiftLabel?: string,
    shiftInsert?: string,
    shiftCursorFromEnd = 0,
    tone?: KeySpec["tone"],
): KeySpec {
    const spec: KeySpec = {
        id,
        primary: { label, insert, cursorFromEnd },
    };
    if (shiftLabel !== undefined && shiftInsert !== undefined) {
        return {
            ...spec,
            shift: { label: shiftLabel, insert: shiftInsert, cursorFromEnd: shiftCursorFromEnd },
            ...(tone === undefined ? {} : { tone }),
        };
    }
    return tone === undefined ? spec : { ...spec, tone };
}

function math(label: string): string {
    return `<span class="math-label">${label}</span>`;
}

function runKey(id: string): void {
    if (editorIsComposing) {
        return;
    }
    const spec = keys.find((candidate) => candidate.id === id);
    if (spec === undefined) {
        return;
    }
    const command = state.shift && spec.shift !== undefined ? spec.shift : spec.primary;
    runCommand("tag" in command ? command : { tag: "insert", layer: command });
    if (state.shift && spec.id !== "shift") {
        state.shift = false;
    }
    syncControls();
}

function runCommand(command: Command): void {
    if (editorIsComposing) {
        return;
    }
    switch (command.tag) {
        case "insert":
            insertText(command.layer.insert, command.layer.cursorFromEnd ?? 0);
            invalidatePreview();
            return;
        case "move":
            moveSelection(command.delta);
            return;
        case "backspace":
            deleteBackward();
            invalidatePreview();
            return;
        case "delete":
            deleteForward();
            invalidatePreview();
            return;
        case "clear":
            state.expression = "";
            editor.value = "";
            editor.setSelectionRange(0, 0);
            invalidatePreview();
            return;
        case "shift":
            state.shift = !state.shift;
            return;
        case "calculate":
            void calculateExpression();
            return;
    }
}

function handleKeyboard(event: KeyboardEvent): void {
    if (event.isComposing) {
        return;
    }
    if (event.metaKey || event.ctrlKey) {
        if (event.key === "Enter") {
            event.preventDefault();
            void calculateExpression();
        }
        return;
    }
    switch (event.key) {
        case "Enter":
            event.preventDefault();
            void calculateExpression();
            return;
        default:
            return;
    }
}

function insertText(text: string, cursorFromEnd = 0): void {
    const { start, end } = editorSelection();
    editor.setRangeText(text, start, end, "end");
    const cursor = Math.max(start, start + text.length - cursorFromEnd);
    editor.setSelectionRange(cursor, cursor);
    revealNativeSelection();
    state.expression = editor.value;
    state.copied = false;
}

function deleteBackward(): void {
    const { start, end } = editorSelection();
    if (start !== end) {
        replaceSelection("");
        return;
    }
    if (start === 0) {
        return;
    }
    const previous = previousCharBoundary(state.expression, start);
    editor.setRangeText("", previous, start, "start");
    state.expression = editor.value;
    revealNativeSelection();
}

function deleteForward(): void {
    const { start, end } = editorSelection();
    if (start !== end) {
        replaceSelection("");
        return;
    }
    if (start >= state.expression.length) {
        return;
    }
    const next = nextCharBoundary(state.expression, start);
    editor.setRangeText("", start, next, "start");
    state.expression = editor.value;
    revealNativeSelection();
}

function handleEditorInput(): void {
    if (editorIsComposing) {
        return;
    }
    commitEditorInput();
}

function commitEditorInput(): void {
    const previousExpression = state.expression;
    syncExpressionFromEditor();
    if (state.expression === previousExpression) {
        return;
    }
    invalidatePreview();
}

function syncExpressionFromEditor(): void {
    const original = editor.value;
    const selection = editorSelection(original.length);
    const expression = original.replaceAll(/[\r\n]/gu, "");
    if (original !== expression) {
        const start = offsetAfterRemovingLineBreaks(original, selection.start);
        const end = offsetAfterRemovingLineBreaks(original, selection.end);
        const scrollLeft = editor.scrollLeft;
        editor.value = expression;
        editor.setSelectionRange(start, end, selection.direction);
        editor.scrollLeft = scrollLeft;
    }
    state.expression = expression;
    state.copied = false;
}

function editorSelection(length = state.expression.length): {
    readonly start: number;
    readonly end: number;
    readonly direction: "forward" | "backward" | "none";
} {
    return {
        start: clamp(editor.selectionStart, 0, length),
        end: clamp(editor.selectionEnd, 0, length),
        direction: editor.selectionDirection,
    };
}

function replaceSelection(text: string): void {
    const { start, end } = editorSelection();
    editor.setRangeText(text, start, end, "end");
    state.expression = editor.value;
}

function moveSelection(delta: -1 | 1): void {
    const { start, end } = editorSelection();
    const cursor = delta < 0
        ? (start !== end ? start : previousCharBoundary(state.expression, start))
        : (start !== end ? end : nextCharBoundary(state.expression, end));
    editor.setSelectionRange(cursor, cursor);
    revealNativeSelection();
}

function revealNativeSelection(): void {
    const start = editor.selectionStart;
    editor.focus({ preventScroll: true });
    editor.setSelectionRange(start, editor.selectionEnd, editor.selectionDirection);
}

function offsetAfterRemovingLineBreaks(value: string, offset: number): number {
    return value.slice(0, offset).replaceAll(/[\r\n]/gu, "").length;
}

async function calculateExpression(): Promise<void> {
    if (editorIsComposing) {
        return;
    }
    const operation = beginOperation();
    state.busy = true;
    state.copied = false;
    state.statusMessage = "";
    renderStatus();
    const result = await calculateWithWorker(state.expression, buildRequest(), operation);
    if (result === null) {
        return;
    }
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

function beginOperation(): number {
    cancelActiveCalculation();
    operationVersion += 1;
    state.statusMessage = "";
    return operationVersion;
}

function invalidatePreview(): void {
    cancelActiveCalculation();
    operationVersion += 1;
    state.busy = false;
    renderEditor();
    renderInputPreview();
    renderStatus();
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
    activeCalculation = { operation, abortController };
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
    return isCurrentOperation(operation) ? result : null;
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
        exactOutput: {
            tag: "include",
            format: state.exactFormat,
        },
        scientificOutput: {
            tag: "include",
            significantDigits: state.significantDigits,
            roundingMode: state.roundingMode,
        },
        enclosureOutput: {
            tag: "include",
            format: {
                tag: "decimalScientific",
                significantDigits: state.significantDigits,
            },
        },
    };
}

function renderEditor(): void {
    const selection = editorSelection();
    editor.dataset.empty = String(state.expression.length === 0);
    if (editor.value !== state.expression) {
        const scrollLeft = editor.scrollLeft;
        editor.value = state.expression;
        editor.setSelectionRange(selection.start, selection.end, selection.direction);
        editor.scrollLeft = scrollLeft;
    }
}

function renderInputPreview(): void {
    const source = state.expression;
    const version = previewVersion + 1;
    previewVersion = version;
    if (source.trim().length === 0) {
        inputPreview.textContent = "";
        inputPreview.dataset.state = "empty";
        return;
    }
    inputPreview.dataset.state = "loading";
    void directCalculator
        .then((calculator) => {
            if (version !== previewVersion) {
                return;
            }
            const result = calculator.presentInput(source, buildRequest());
            if (version !== previewVersion) {
                return;
            }
            if (result.tag === "ok") {
                inputPreview.innerHTML = `<math display="block">${renderMathMl(result.value)}</math>`;
                inputPreview.dataset.state = "ready";
            } else {
                inputPreview.textContent = "";
                inputPreview.dataset.state = "error";
            }
        })
        .catch(() => {
            if (version === previewVersion) {
                inputPreview.textContent = "";
                inputPreview.dataset.state = "error";
            }
        });
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
    exactOutput.textContent =
        `${renderResultRelationPlainText(calculation.exact.value.relation)} ${calculation.exact.value.plainText}`;
    mathmlOutput.innerHTML =
        `<math display="inline">${renderResultRelationMathMl(calculation.exact.value.relation)}${renderMathMl(calculation.exact.value.presentation)}</math>`;
}

function renderScientific(calculation: Calculation): void {
    switch (calculation.scientific.tag) {
        case "omitted":
            scientificState.textContent = "omitted";
            scientificOutput.textContent = "Scientific output is disabled.";
            return;
        case "unavailable":
            scientificState.textContent = formatLabel(calculation.scientific.value.reason.tag);
            scientificOutput.textContent = "Not confirmed at the requested significant digits.";
            return;
        case "included":
            scientificState.textContent = `${calculation.scientific.value.confirmedSignificantDigits} digits`;
            scientificOutput.textContent =
                `${renderResultRelationPlainText(calculation.scientific.value.relation)} ${renderPlainText(
                    calculation.scientific.value.presentation,
                )}`;
            return;
    }
}

function renderEnclosure(calculation: Calculation): void {
    switch (calculation.enclosure.tag) {
        case "omitted":
            enclosureState.textContent = "omitted";
            enclosureOutput.textContent = "No certified interval was requested.";
            return;
        case "included":
            enclosureState.textContent = formatLabel("decimalScientific");
            enclosureOutput.textContent =
                `${renderResultRelationPlainText(calculation.enclosure.value.relation)} ${renderPlainText(
                    calculation.enclosure.value.presentation,
                )}`;
            return;
        case "unavailable":
            enclosureState.textContent = formatLabel(calculation.enclosure.reason.tag);
            enclosureOutput.textContent = "A certified interval is unavailable for this output.";
            return;
    }
}

function renderStatus(): void {
    if (state.busy) {
        statusOutput.textContent = "Calculating";
    } else if (state.copied) {
        statusOutput.textContent = "Copied";
    } else {
        statusOutput.textContent = state.statusMessage;
    }
    calculateButton.disabled = state.busy;
    cancelButton.disabled = activeCalculation === null;
    copyButton.disabled = state.busy;
}

function syncControls(): void {
    renderEditor();
    settingsPopover.dataset.open = String(state.settingsOpen);
    settingsToggle.setAttribute("aria-expanded", String(state.settingsOpen));
    exactFormat.value = state.exactFormat;
    digits.value = String(state.significantDigits);
    rounding.value = state.roundingMode;
    for (const button of document.querySelectorAll<HTMLButtonElement>("[data-angle]")) {
        button.dataset.active = String(button.dataset.angle === state.angleUnit);
    }
    for (const spec of keys) {
        const button = required<HTMLButtonElement>(`#key-${spec.id}`);
        const layer = state.shift && spec.shift !== undefined ? spec.shift : spec.primary;
        button.innerHTML = keyLabel(spec, layer);
        button.setAttribute("aria-label", keyAccessibleLabel(spec, layer));
        button.dataset.active = String(spec.id === "shift" && state.shift);
    }
    renderInputPreview();
    renderStatus();
}

function keyLabel(spec: KeySpec, layer: KeyLayer | Command): string {
    if ("tag" in layer) {
        switch (layer.tag) {
            case "shift":
                return "Shift";
            case "move":
                return layer.delta < 0 ? "←" : "→";
            case "backspace":
                return "⌫";
            case "delete":
                return "⌦";
            case "clear":
                return "C";
            case "calculate":
                return "=";
            case "insert":
                return layer.layer.label;
        }
    }
    if (state.shift && spec.shift !== undefined) {
        return `<span class="shift-label">${layer.label}</span>`;
    }
    return layer.label;
}

function keyAccessibleLabel(spec: KeySpec, layer: KeyLayer | Command): string {
    if ("tag" in layer) {
        switch (layer.tag) {
            case "move":
                return layer.delta < 0 ? "Move cursor left" : "Move cursor right";
            case "backspace":
                return "Backspace";
            case "delete":
                return "Delete";
            case "clear":
                return "Clear expression";
            case "shift":
                return "Shift keypad";
            case "calculate":
                return "Calculate";
            case "insert":
                return `Insert ${layer.layer.insert}`;
        }
    }
    return `Insert ${layer.insert}`;
}

function currentPlainText(): string {
    if (state.result.tag === "error") {
        return formatError(state.result.error);
    }
    const calculation = state.result.value.calculation;
    if (calculation.exact.tag === "included") {
        return `${renderResultRelationPlainText(calculation.exact.value.relation)} ${renderPlainText(
            calculation.exact.value.presentation,
        )}`;
    }
    return "";
}

function previousCharBoundary(value: string, cursor: number): number {
    let previous = 0;
    for (const segment of graphemeSegmenter.segment(value)) {
        if (segment.index >= cursor) {
            break;
        }
        previous = segment.index;
    }
    return previous;
}

function nextCharBoundary(value: string, cursor: number): number {
    for (const segment of graphemeSegmenter.segment(value)) {
        if (segment.index > cursor) {
            return segment.index;
        }
    }
    return value.length;
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
