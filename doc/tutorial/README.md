# Tutorial: Building A Calculator UI

この tutorial は、`calculator-library` を使って独自の電卓 UI を作るための手順を説明する。対象は主に Web UI 実装者であり、npm facade から Wasm を呼び出す経路を中心に扱う。

この library は UI を持たない計算 engine ではなく、UI が必要とする次の情報を typed API として返す。

* 入力式の parse と表示用 presentation tree
* exact output
* scientific output
* certified interval
* typed error
* button 入力向けの headless session state

実装や仕様を確認するときは、まず [`doc/public-contract.md`](../public-contract.md) を読む。設計思想は [`doc/design.md`](../design.md)、現行実装の対応範囲は [`doc/implementation-status.md`](../implementation-status.md) に分かれている。

---

## 1. 全体像

Web UI から使う入口は `@bem130/exact-calculator` package である。

```ts
import {
    createCalculator,
    createSession,
    defaultCalculationRequest,
    defaultInputPolicy,
    renderMathMl,
    renderPlainText,
    type Calculation,
    type CalculationRequest,
    type InputActionDto,
} from "@bem130/exact-calculator";
import { createWorkerCalculator } from "@bem130/exact-calculator/worker";
```

使い分けは次の通り。

| API | 役割 | UI での典型用途 |
| --- | --- | --- |
| `createCalculator()` | main thread で Wasm module を読み、同期的な facade を返す | 入力プレビュー、軽い計算、テスト |
| `createWorkerCalculator()` | 計算を worker に逃がす | ユーザー操作を止めたくない本番 UI |
| `createSession()` | button 入力を headless に管理する | 電卓キー、cursor、`Ans`、memory、calculator percent |
| `renderMathMl()` | presentation tree を MathML 文字列へ変換する | exact output や入力式の美しい表示 |
| `renderPlainText()` | presentation tree を plain text へ変換する | copy、fallback 表示、ログ |

UI はこの public facade だけを呼び、`crates/calculator-wasm` の生成 glue や Rust 内部型へ直接依存しない。

---

## 2. Step 1: request を決める

計算 API は必ず `CalculationRequest` を受け取る。request には parse 設定、意味論、出力要求、resource limit が含まれる。

まずは `defaultCalculationRequest` を土台にし、UI の設定だけ上書きする。

```ts
const request: CalculationRequest = {
    ...defaultCalculationRequest,
    semantics: {
        ...defaultCalculationRequest.semantics,
        angleUnit: "radian",
    },
    exactOutput: {
        tag: "include",
        format: "auto",
    },
    scientificOutput: {
        tag: "include",
        significantDigits: 5,
        roundingMode: "nearestTiesToEven",
    },
    enclosureOutput: {
        tag: "include",
        format: "exactDyadic",
    },
};
```

重要な点は、exact / scientific / enclosure を個別に要求することである。UI で常に3つの出力欄を見せたい場合は、上のように3つとも `include` にする。

`enclosureOutput.format` は現行 API では `exactDyadic` である。`x.xxx * 10^n` のような decimal scientific 表示にしたい場合は、UI 側で exact dyadic endpoint を丸めて表示する。実装例は [`examples/vanilla-web/src/main.ts`](../../examples/vanilla-web/src/main.ts) の `formatCertifiedInterval` を参照する。

---

## 3. Step 2: 入力式をそのまま美しく表示する

入力 textarea の文字列を評価せずに数式表示したい場合は `presentInput()` を使う。

```ts
const calculator = await createCalculator();

function renderInputPreview(source: string): void {
    const result = calculator.presentInput(source, request);
    const preview = document.querySelector("#input-preview");
    if (!(preview instanceof HTMLElement)) {
        return;
    }

    if (result.tag === "ok") {
        preview.innerHTML = `<math display="block">${renderMathMl(result.value)}</math>`;
    } else {
        preview.textContent = "";
    }
}
```

`presentInput()` は parse と入力制限の検査を行うが、計算結果は作らない。したがって、入力途中の preview と calculate button の責務を分けられる。

表示例:

| 入力 | preview の意味 |
| --- | --- |
| `sqrt(2)` | 平方根として表示 |
| `log(8,2)` | 底 `2` の対数として表示 |
| `ln(e)` | 自然対数として表示 |
| `exp(3,2)` | `2^3` として表示 |

底を省略した `log(x)` は受け付けない。自然対数は `ln(x)`、任意底の対数は `log(argument, base)` と入力する。

---

## 4. Step 3: 直接計算する

最小の計算 UI は `createCalculator()` と `calculate()` だけで作れる。

```ts
const calculator = await createCalculator();
const result = calculator.calculate("log(8,2)", request);

if (result.tag === "error") {
    console.error(result.error.tag, result.error);
} else {
    const calculation = result.value.calculation;
    renderCalculation(calculation);
}
```

`ApiResult<T>` は必ず `tag` で分岐する。例外で domain error を投げる設計ではない。

```ts
function renderCalculation(calculation: Calculation): void {
    if (calculation.exact.tag === "included") {
        const exact = calculation.exact.value;
        document.querySelector("#exact")!.textContent = `= ${exact.plainText}`;
        document.querySelector("#exact-math")!.innerHTML =
            `<math display="inline">${renderMathMl(exact.presentation)}</math>`;
    }

    if (calculation.scientific.tag === "included") {
        const scientific = calculation.scientific.value;
        document.querySelector("#scientific")!.textContent =
            `${scientific.significand} * 10^${scientific.exponentTen}`;
    } else if (calculation.scientific.tag === "unavailable") {
        document.querySelector("#scientific")!.textContent =
            "Requested decimal digits are not confirmed.";
    }

    if (calculation.enclosure.tag === "included") {
        const interval = calculation.enclosure.value;
        document.querySelector("#interval")!.textContent =
            `[${interval.lower.coefficient} * 2^${interval.lower.exponentTwo}, ` +
            `${interval.upper.coefficient} * 2^${interval.upper.exponentTwo}]`;
    }
}
```

`scientific` が `unavailable` でも、`enclosure` が `included` なら計算が壊れたわけではない。要求された有効数字が保証区間から一意に確定していないだけで、certified interval は別の保証付き出力として利用できる。

---

## 5. Step 4: scientific output と certified interval を分けて考える

この library は、近似値を exact value の代わりとして返さない。scientific output は「要求された有効数字が保証できた場合だけ」含まれる。

実装上の考え方は次の通り。

1. exact expression または recognized exact value を保持する。
2. 必要に応じて certified interval を作る。
3. 要求された有効数字が interval 内で確定するか確認する。
4. 確定した場合だけ scientific output を返す。

概念的には「必要な桁より十分多く計算し、丸めて要求桁を出す」処理だが、未確認の guard digit を推測して表示しない点が重要である。rational のように厳密商と剰余で丸めを証明できる値は直接 scientific output を返せる。一般の超越関数では certified interval の両端が同じ要求桁へ丸まることを確認できたときだけ返す。

そのため UI では、scientific output と certified interval を別々の欄に出すのがよい。

---

## 6. Step 5: worker で計算して UI を止めない

本番の Web UI では、計算を worker に逃がす。入力 preview は main thread の `presentInput()`、重い計算は worker の `calculate()` に分けると扱いやすい。

```ts
const calculator = await createCalculator();
const worker = await createWorkerCalculator();

async function calculateWithCancel(source: string, request: CalculationRequest): Promise<void> {
    const controller = new AbortController();
    const resultPromise = worker.calculate(source, request, {
        signal: {
            tag: "abortSignal",
            signal: controller.signal,
        },
    });

    document.querySelector("#cancel")?.addEventListener("click", () => {
        controller.abort();
    }, { once: true });

    const result = await resultPromise;
    if (result.tag === "ok") {
        renderCalculation(result.value.calculation);
    } else {
        renderError(result.error);
    }
}
```

worker cancellation は typed error として返る。途中まで壊れた計算結果を成功値として扱わない。

---

## 7. Step 6: button UI は headless session に任せる

独自 UI で button を自由に配置したい場合でも、button が作る入力列は `createSession()` に任せられる。

```ts
const session = await createSession({
    ...defaultInputPolicy,
    calculationRequest: request,
});

function dispatch(action: InputActionDto): void {
    const result = session.dispatch(action);

    if (result.tag === "state") {
        renderSource(result.state.source);
        return;
    }

    if (result.tag === "inputError") {
        renderSource(result.state.source);
        renderInputError(result.error);
        return;
    }

    void calculateFromSession(result.source, result.request);
}
```

button から送る action の例:

```ts
dispatch({ tag: "digit", value: 8 });
dispatch({ tag: "comma" });
dispatch({ tag: "function", value: "ln" });
dispatch({ tag: "function", value: "log" });
dispatch({ tag: "binaryOperator", value: "power" });
dispatch({ tag: "evaluate" });
```

`evaluate` action はその場で計算しない。`SessionDispatchResult` の `calculate` command を返すので、UI は worker など好きな経路で計算し、完了後に `applyResult()` で session へ戻す。

```ts
async function calculateFromSession(source: string, request: CalculationRequest): Promise<void> {
    const result = await worker.calculate(source, request, {
        signal: { tag: "none" },
    });
    const state = session.applyResult(result);
    renderSource(state.source);
    renderDisplay(state.display);
}
```

この分離により、UI layout、button の見た目、keyboard shortcut、worker cancellation を自由に作りながら、入力規則と計算規則を library 側の contract に揃えられる。

---

## 8. Step 7: 対数・指数・有名角の入力を用意する

現行 parser は次の関数を受け付ける。

| 入力 | 意味 |
| --- | --- |
| `sin(x)` / `cos(x)` / `tan(x)` | 三角関数 |
| `asin(x)` / `acos(x)` / `atan(x)` | 逆三角関数 |
| `sqrt(x)` | 平方根 |
| `ln(x)` | 底 `e` の自然対数 |
| `log(argument, base)` | 任意底の対数 |
| `exp(x)` | `e^x` |
| `exp(exponent, base)` | `base^exponent` |

UI button としては、少なくとも次のようなキーを用意すると入力しやすい。

```ts
const functionKeys = [
    "sin(", "cos(", "tan(", "ln(",
    "asin(", "acos(", "atan(", "sqrt(",
    "exp(", "log(",
];

const knownValueKeys = [
    "pi/6", "pi/4", "pi/3", "pi/2",
    "sqrt(2)/2", "sqrt(3)/2",
    "log(8,2)", "ln(e)", "exp(3,2)",
];
```

`log(8,2)`、`log(2^(1/3),2)`、`ln(e)`、`exp(3,2)` は exact output でそれぞれ `3`、`1/3`、`1`、`8` になる。

---

## 9. Step 8: error と partial を UI に出す

失敗は `ApiResult` の `error` として返る。

```ts
function renderError(error: { tag: string }): void {
    switch (error.tag) {
        case "domain":
            // divisionByZero, logarithmOfNonPositive, logarithmBaseOne など
            break;
        case "parse":
            // 入力途中や構文エラー
            break;
        case "inputLimit":
        case "computationLimit":
            // resource limit
            break;
    }
}
```

一方、`CalculationOutcome` の `partial` は失敗ではない。exact expression または certified interval は有効で、要求された一部の出力だけが未確定または制限到達である。

```ts
if (result.tag === "ok" && result.value.tag === "partial") {
    console.info(result.value.reason);
    console.info(result.value.certifiedEnclosure);
    renderCalculation(result.value.calculation);
}
```

UI 文言としては「計算に失敗」ではなく、「要求された桁は未確定」「一部出力のみ利用可能」のように分ける。

---

## 10. 仕様と実装を読む場所

利用者向けの調査は、次の順に読むと迷いにくい。

| 読みたい内容 | 参照先 |
| --- | --- |
| 現行の公開契約 | [`doc/public-contract.md`](../public-contract.md) |
| 設計方針と将来像 | [`doc/design.md`](../design.md) |
| 現在実装済みの範囲 | [`doc/implementation-status.md`](../implementation-status.md) |
| npm facade の export | [`packages/calculator/src/index.ts`](../../packages/calculator/src/index.ts) |
| direct/session の public wrapper | [`packages/calculator/src/direct.ts`](../../packages/calculator/src/direct.ts) |
| worker facade | [`packages/calculator/src/worker.ts`](../../packages/calculator/src/worker.ts) |
| MathML/plain text renderer | [`packages/calculator/src/presentation.ts`](../../packages/calculator/src/presentation.ts) |
| DTO の TypeScript 型 | [`packages/calculator/src/generated/dto.ts`](../../packages/calculator/src/generated/dto.ts) |
| Rust core API | [`crates/calculator-core/src/api.rs`](../../crates/calculator-core/src/api.rs) |
| request/result/error 型 | [`crates/calculator-core/src/types.rs`](../../crates/calculator-core/src/types.rs) |
| parser の入力文法 | [`crates/calculator-core/src/syntax.rs`](../../crates/calculator-core/src/syntax.rs) |
| lowering/evaluation semantics | [`crates/calculator-core/src/expression.rs`](../../crates/calculator-core/src/expression.rs) |
| parser/session conformance | [`crates/calculator-core/fixtures/parser_session_conformance.json`](../../crates/calculator-core/fixtures/parser_session_conformance.json) |
| Wasm DTO conformance | [`crates/calculator-wasm/fixtures/native_wasm_dto_conformance.json`](../../crates/calculator-wasm/fixtures/native_wasm_dto_conformance.json) |
| 実際の vanilla UI | [`examples/vanilla-web/src/main.ts`](../../examples/vanilla-web/src/main.ts) |

`doc/design.md` は最終設計の目標を含むため、現在利用できる API の正本としては扱わない。実装時に迷ったら `public-contract.md` と DTO 型を優先する。

---

## 11. 実装チェックリスト

独自 UI を作るときは、次を満たすと API の意図と揃いやすい。

* 入力文字列は UI state として保持する。
* 入力 preview は `presentInput()` で評価と分けて作る。
* 計算 request は UI 設定から毎回明示的に組み立てる。
* exact / scientific / certified interval の3欄を別々に描画する。
* `scientific.unavailable` を計算失敗として扱わない。
* worker calculation には cancel 経路を用意する。
* button UI は `createSession()` と `InputActionDto` を使う。
* `ApiResult`、`CalculationOutcome`、output union は必ず `tag` で分岐する。
* `renderMathMl()` の結果は `<math>` の内側へ入れる。
* 仕様確認は `public-contract.md`、実装範囲確認は `implementation-status.md`、実装詳細確認は該当 source を読む。

---

## 12. ローカルで確認する

library 全体の確認:

```sh
cargo fmt --all --check
cargo test --workspace
corepack pnpm run check
cargo xtask check-generated
cargo xtask check-protocol-snapshot
```

vanilla web example の確認:

```sh
corepack pnpm --dir examples/vanilla-web run build
corepack pnpm --dir examples/vanilla-web run test:e2e
corepack pnpm --dir examples/vanilla-web run dev
```

実装例を読みながら UI を作る場合は、まず `examples/vanilla-web/src/main.ts` の `buildRequest`、`renderInputPreview`、`renderResult`、`formatCertifiedInterval`、session dispatch 周辺を追うとよい。
