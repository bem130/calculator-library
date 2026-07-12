# Issue 51: Pages CIでWasm optimizerを再現可能に提供する

GitHub Actions run
[`29213135540`](https://github.com/bem130/calculator-library/actions/runs/29213135540)
では、`Deploy example to GitHub Pages` workflowの`Build example` stepが失敗した。
`wasm-pack`によるrelease Wasm生成は成功している一方、後続の
`packages/calculator/scripts/optimize-wasm.mjs`が`wasm-opt`を起動した時点で
`spawnSync wasm-opt ENOENT`となる。workflowは`wasm-pack`を導入しているが、
packageの必須最適化工程が要求するBinaryen CLIをrunnerへ提供していない。

- workflowとpackage buildの依存契約を一致させ、固定または検証可能な方法で
  `wasm-opt`を用意してからexample buildを実行する。
- optimizer不在を黙って無視したり最適化を省略したりせず、生成物と失敗条件を
  local/CIで一貫させる。
- Pages buildを現行runner相当のclean環境で再現し、package/example build、
  browser E2E、repository gateを通す。
- 指定runで失敗したworkflowを再実行または後続runで成功させ、Pages artifactの
  uploadとdeploy jobまで到達することを確認する。
- 修正差分、全体整合、merge粒度をreviewし、進行中の性能改善branchとは独立した
  CI修正としてmainへ統合する。

## Resolution evidence

Main commit `88b6833` installs the official Binaryen 130 archive through a shared
installer, verifies SHA-256 and the reported version, and exposes it to later
steps through `GITHUB_PATH`. The clean local flow completed frozen dependency
installation, package checks, example production build, and browser E2E.

GitHub Actions verified the external deployment boundary on 2026-07-13:

- [Pages run 29213742681](https://github.com/bem130/calculator-library/actions/runs/29213742681)
  completed the example build, Pages artifact upload, and deploy job.
- [CI run 29213742685](https://github.com/bem130/calculator-library/actions/runs/29213742685)
  completed every repository check successfully.
