import { cp, mkdir, rm } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const exampleRoot = resolve(here, "..");
const source = resolve(exampleRoot, "../../packages/calculator/wasm");
const destination = resolve(exampleRoot, "public/wasm");

await rm(destination, { force: true, recursive: true });
await mkdir(destination, { recursive: true });
await Promise.all([
    cp(resolve(source, "calculator_wasm.js"), resolve(destination, "calculator_wasm.js")),
    cp(resolve(source, "calculator_wasm_bg.wasm"), resolve(destination, "calculator_wasm_bg.wasm")),
]);
