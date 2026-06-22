import type { FunctionNameDto, PresentationNodeDto } from "./generated/dto";

export function renderPlainText(node: PresentationNodeDto): string {
    switch (node.tag) {
        case "text":
            return node.text;
        case "row":
            return node.children.map(renderPlainText).join("");
        case "fraction":
            return `${renderPlainText(node.numerator)}/${renderPlainText(node.denominator)}`;
        case "superscript":
            return `${renderPlainText(node.base)}^${renderPlainText(node.exponent)}`;
        case "subscript":
            return `${renderPlainText(node.base)}_${renderPlainText(node.subscript)}`;
        case "radical":
            return node.index.tag === "square"
                ? `sqrt(${renderPlainText(node.radicand)})`
                : `root(${node.index.value}, ${renderPlainText(node.radicand)})`;
        case "function":
            return `${node.name}(${renderPlainText(node.argument)})`;
        case "parenthesized":
            return `(${renderPlainText(node.value)})`;
    }
}

export function renderMathMl(node: PresentationNodeDto): string {
    switch (node.tag) {
        case "text":
            return renderTextMathMl(node.text);
        case "row":
            return `<mrow>${node.children.map(renderMathMl).join("")}</mrow>`;
        case "fraction":
            return `<mfrac>${renderMathMl(node.numerator)}${renderMathMl(node.denominator)}</mfrac>`;
        case "superscript":
            return `<msup>${renderMathMl(node.base)}${renderMathMl(node.exponent)}</msup>`;
        case "subscript":
            return `<msub>${renderMathMl(node.base)}${renderMathMl(node.subscript)}</msub>`;
        case "radical":
            if (node.index.tag === "square") {
                return `<msqrt>${renderMathMl(node.radicand)}</msqrt>`;
            }
            return `<mroot>${renderMathMl(node.radicand)}<mn>${escapeXml(node.index.value)}</mn></mroot>`;
        case "function":
            return `<mrow><mi>${escapeXml(node.name)}</mi><mo>(</mo>${renderMathMl(node.argument)}<mo>)</mo></mrow>`;
        case "parenthesized":
            return `<mrow><mo>(</mo>${renderMathMl(node.value)}<mo>)</mo></mrow>`;
    }
}

export function renderLatex(node: PresentationNodeDto): string {
    switch (node.tag) {
        case "text":
            return renderTextLatex(node.text);
        case "row":
            return node.children.map(renderLatex).join("");
        case "fraction":
            return `\\frac{${renderLatex(node.numerator)}}{${renderLatex(node.denominator)}}`;
        case "superscript":
            return `${renderLatexAtom(node.base)}^{${renderLatex(node.exponent)}}`;
        case "subscript":
            return `${renderLatexAtom(node.base)}_{${renderLatex(node.subscript)}}`;
        case "radical":
            if (node.index.tag === "square") {
                return `\\sqrt{${renderLatex(node.radicand)}}`;
            }
            return `\\sqrt[${renderTextLatex(node.index.value)}]{${renderLatex(node.radicand)}}`;
        case "function":
            if (node.name === "sqrt") {
                return `\\sqrt{${renderLatex(node.argument)}}`;
            }
            return `${latexFunctionName(node.name)}\\left(${renderLatex(node.argument)}\\right)`;
        case "parenthesized":
            return `\\left(${renderLatex(node.value)}\\right)`;
    }
}

function escapeXml(text: string): string {
    return text
        .replaceAll("&", "&amp;")
        .replaceAll("<", "&lt;")
        .replaceAll(">", "&gt;")
        .replaceAll("\"", "&quot;");
}

function renderTextMathMl(text: string): string {
    const escaped = escapeXml(text);
    if (/^[+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:e[+-]?\d+)?$/iu.test(text)) {
        return `<mn>${escaped}</mn>`;
    }
    const operatorText = text.trim();
    if (MATH_OPERATOR_TEXT.has(operatorText)) {
        return `<mo>${escapeXml(operatorText)}</mo>`;
    }
    return `<mi>${escaped}</mi>`;
}

function renderLatexAtom(node: PresentationNodeDto): string {
    switch (node.tag) {
        case "text":
        case "radical":
        case "function":
        case "parenthesized":
            return renderLatex(node);
        case "row":
        case "fraction":
        case "superscript":
        case "subscript":
            return `{${renderLatex(node)}}`;
    }
}

function renderTextLatex(text: string): string {
    if (text === "pi" || text === "π") {
        return "\\pi";
    }
    if (LATEX_OPERATOR_NAMES.has(text)) {
        return `\\${text}`;
    }
    return Array.from(text, renderLatexCharacter).join("");
}

function renderLatexCharacter(character: string): string {
    switch (character) {
        case "\\":
            return "\\backslash{}";
        case "{":
            return "\\{";
        case "}":
            return "\\}";
        case "#":
            return "\\#";
        case "$":
            return "\\$";
        case "%":
            return "\\%";
        case "&":
            return "\\&";
        case "_":
            return "\\_";
        case "^":
            return "\\^{}";
        case "~":
            return "\\~{}";
        case "×":
            return "\\times";
        case "∞":
            return "\\infty";
        case "≤":
            return "\\le";
        case "≥":
            return "\\ge";
        case "≠":
            return "\\ne";
        case "≈":
            return "\\approx";
        case "∈":
            return "\\in";
        default:
            return character;
    }
}

function latexFunctionName(name: FunctionNameDto): string {
    switch (name) {
        case "sin":
        case "cos":
        case "tan":
        case "log":
        case "ln":
        case "exp":
            return `\\${name}`;
        case "asin":
            return "\\arcsin";
        case "acos":
            return "\\arccos";
        case "atan":
            return "\\arctan";
        case "sqrt":
            return "\\sqrt";
    }
}

const MATH_OPERATOR_TEXT = new Set([
    "(",
    ")",
    "[",
    "]",
    "{",
    "}",
    ",",
    "+",
    "-",
    "*",
    "/",
    "^",
    "%",
    "=",
    "×",
]);

const LATEX_OPERATOR_NAMES = new Set(["sin", "cos", "tan", "log", "ln", "exp"]);
