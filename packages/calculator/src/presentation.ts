import type { PresentationNodeDto } from "./generated/dto";

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
            return `<mn>${escapeXml(node.text)}</mn>`;
        case "row":
            return `<mrow>${node.children.map(renderMathMl).join("")}</mrow>`;
        case "fraction":
            return `<mfrac>${renderMathMl(node.numerator)}${renderMathMl(node.denominator)}</mfrac>`;
        case "superscript":
            return `<msup>${renderMathMl(node.base)}${renderMathMl(node.exponent)}</msup>`;
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

function escapeXml(text: string): string {
    return text
        .replaceAll("&", "&amp;")
        .replaceAll("<", "&lt;")
        .replaceAll(">", "&gt;")
        .replaceAll("\"", "&quot;");
}
