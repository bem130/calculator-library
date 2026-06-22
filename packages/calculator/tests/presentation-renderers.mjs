import assert from "node:assert/strict";

import {
    renderLatex,
    renderMathMl,
    renderPlainText,
    renderResultRelationLatex,
    renderResultRelationMathMl,
    renderResultRelationPlainText,
} from "../src/presentation.ts";

const logBase = {
    tag: "row",
    children: [
        {
            tag: "subscript",
            base: { tag: "text", text: "log" },
            subscript: { tag: "text", text: "2" },
        },
        { tag: "text", text: "(" },
        { tag: "text", text: "8" },
        { tag: "text", text: ")" },
    ],
};

const radicalFraction = {
    tag: "fraction",
    numerator: {
        tag: "radical",
        index: { tag: "square" },
        radicand: { tag: "text", text: "2" },
    },
    denominator: { tag: "text", text: "2" },
};

const decimalScientificInterval = {
    tag: "row",
    children: [
        { tag: "text", text: "[" },
        { tag: "text", text: "1.4142" },
        { tag: "text", text: " × " },
        {
            tag: "superscript",
            base: { tag: "text", text: "10" },
            exponent: { tag: "text", text: "0" },
        },
        { tag: "text", text: ", " },
        { tag: "text", text: "1.4143" },
        { tag: "text", text: " × " },
        {
            tag: "superscript",
            base: { tag: "text", text: "10" },
            exponent: { tag: "text", text: "0" },
        },
        { tag: "text", text: "]" },
    ],
};

const mixedFraction = {
    tag: "row",
    children: [
        { tag: "text", text: "2" },
        { tag: "text", text: " " },
        {
            tag: "fraction",
            numerator: { tag: "text", text: "1" },
            denominator: { tag: "text", text: "3" },
        },
    ],
};

assert.equal(renderPlainText(radicalFraction), "sqrt(2)/2");
assert.equal(renderMathMl(radicalFraction), "<mfrac><msqrt><mn>2</mn></msqrt><mn>2</mn></mfrac>");
assert.equal(renderLatex(radicalFraction), "\\frac{\\sqrt{2}}{2}");

assert.equal(renderPlainText(logBase), "log_2(8)");
assert.equal(renderLatex(logBase), "\\log_{2}(8)");
assert.equal(renderPlainText(mixedFraction), "2 1/3");
assert.equal(
    renderMathMl(mixedFraction),
    "<mrow><mn>2</mn><mspace width=\"0.5em\"/><mfrac><mn>1</mn><mn>3</mn></mfrac></mrow>",
);

const decimalScientificIntervalMathMl = [
    "<mrow>",
    "<mo>[</mo>",
    "<mn>1.4142</mn>",
    "<mo>×</mo>",
    "<msup><mn>10</mn><mn>0</mn></msup>",
    "<mo>,</mo>",
    "<mn>1.4143</mn>",
    "<mo>×</mo>",
    "<msup><mn>10</mn><mn>0</mn></msup>",
    "<mo>]</mo>",
    "</mrow>",
].join("");

assert.equal(
    renderMathMl(decimalScientificInterval),
    decimalScientificIntervalMathMl,
);
assert.equal(
    renderLatex(decimalScientificInterval),
    "[1.4142 \\times 10^{0}, 1.4143 \\times 10^{0}]",
);

assert.equal(renderResultRelationPlainText("exactEqual"), "=");
assert.equal(renderResultRelationPlainText("approximatelyEqual"), "≈");
assert.equal(renderResultRelationPlainText("elementOf"), "∈");
assert.equal(renderResultRelationMathMl("elementOf"), "<mo>∈</mo>");
assert.equal(renderResultRelationLatex("approximatelyEqual"), "\\approx");
