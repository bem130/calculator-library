import { spawnSync } from "node:child_process";
import { copyFileSync, rmSync } from "node:fs";
import { fileURLToPath } from "node:url";

const input = fileURLToPath(
  new URL("../wasm/calculator_wasm_bg.wasm", import.meta.url),
);
const output = `${input}.optimized`;
const result = spawnSync(
  "wasm-opt",
  [
    input,
    "-Oz",
    "--converge",
    "--strip-debug",
    "--strip-dwarf",
    "--strip-producers",
    "--vacuum",
    "-o",
    output,
  ],
  { stdio: "inherit" },
);

if (result.error) {
  throw result.error;
}
if (result.status !== 0) {
  process.exit(result.status ?? 1);
}

copyFileSync(output, input);
rmSync(output);
