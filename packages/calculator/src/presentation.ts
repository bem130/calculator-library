export type PresentationNodeDto =
    | {
        readonly tag: "text";
        readonly text: string;
    }
    | {
        readonly tag: "row";
        readonly children: readonly PresentationNodeDto[];
    };

export function renderPlainText(node: PresentationNodeDto): string {
    switch (node.tag) {
        case "text":
            return node.text;
        case "row":
            return node.children.map(renderPlainText).join("");
    }
}

export function renderMathMl(node: PresentationNodeDto): string {
    switch (node.tag) {
        case "text":
            return `<mi>${escapeXml(node.text)}</mi>`;
        case "row":
            return `<mrow>${node.children.map(renderMathMl).join("")}</mrow>`;
    }
}

function escapeXml(text: string): string {
    return text
        .replaceAll("&", "&amp;")
        .replaceAll("<", "&lt;")
        .replaceAll(">", "&gt;")
        .replaceAll("\"", "&quot;");
}
