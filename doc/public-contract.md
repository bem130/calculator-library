# Current Public Contract

この文書は、現行実装で利用者に公開する契約をまとめる。将来の設計目標は [`design.md`](design.md) に置き、この文書には現在の公開 surface と整合性の境界だけを書く。

この project は試作段階では後方互換性を保証しない。公開 surface は破壊的に変更してよいが、変更時には仕様、DTO、example、conformance、tutorial を同じ方針に揃え、不整合な互換層を残さない。

## 正本

現行公開契約の正本は次の生成物と型である。

* Rust core: `calculator-core` の公開型と `calculate` / `evaluate` / `parse` / `present` / `present_input` / `reduce_input` / `apply_calculation_result`。
* Wasm DTO: `crates/calculator-wasm/src/dto.rs` と生成 TypeScript `packages/calculator/src/generated/dto.ts`。
* npm facade: package root `packages/calculator/src/index.ts` と worker subpath `packages/calculator/src/worker.ts` から export される型・関数。
* protocol snapshot: `crates/xtask/snapshots/protocol-2.0.dto.ts`。

`doc/design.md` は最終設計の目標であり、現行リリースの完全実装リストではない。

## Metadata

author は `bem130`、license は `MIT` とする。license 本文は repository root の `LICENSE` を正本とする。Cargo workspace の公開 crate、npm package、Wasm 生成 package metadata はこの値と矛盾してはならない。

## Calculation API

公開 API は入力文字列と `CalculationRequest` を受け取り、`ApiResult<CalculationOutcome>` または Rust の `Result<CalculationOutcome, CalculatorError>` を返す。成功値は `Complete` または `Partial` であり、失敗値は typed error で返す。

近似値は厳密値と混同しない。小数表示を返す場合は、要求された桁が保証区間から確定した場合だけ `ScientificOutput::Included` / DTO の scientific output として返す。確定できない場合は推測値を返さない。

`Partial` は「計算が壊れた」ことを意味しない。厳密式または認識済み exact 表現を保持したまま、要求された出力の一部が未確定または未対応であることを表す。

## Input And Semantics

入力文法、演算子優先順位、implicit multiplication、unicode alias、percent parse policy、angle unit、real principal power semantics は `CalculationRequest` の DTO 値で明示する。

現行 semantics は実数領域を対象とする。負数の偶数根、非実数になる累乗、0 除算、tan の極、log の非正値、log の底 1、逆三角関数の定義域外は domain error として返す。

`log(argument, base)` と `exp(exponent, base)` は2引数形式を受ける。`ln(argument)` は底 `e` の自然対数として受ける。`log(argument)` のように底を省略した対数は受け付けない。

## Outputs

Exact output は relation、presentation tree、plain text、MathML、LaTeX、representation kind、simplification status、method tags を持つ。Scientific output と enclosure output も relation を持つ。npm facade は `renderPlainText`、`renderMathMl`、`renderLatex` と `renderResultRelationPlainText` / `renderResultRelationMathMl` / `renderResultRelationLatex` を公開し、sample UI はこの public facade だけを使用する。`presentInput` は入力式そのものの presentation tree を返し、評価結果とは混同しない。

Scientific output は significant digits と rounding mode を要求として受ける。rounding mode は DTO と Rust enum の両方で明示 variant として扱う。既定の `CalculationRequest` は 5 significant digits を要求する。`ScientificPresentation` は機械可読な `significand` / `exponentTen` に加えて、UI 表示用の presentation tree を `x.xxx × 10^n` 形式で返す。

Enclosure output は `exactDyadic` または `decimalScientific` を明示して要求する。既定の `CalculationRequest` は 5 significant digits の `decimalScientific` enclosure を要求する。`decimalScientific` enclosure は下端を下向き、上端を上向きに丸め、presentation tree は `x.xxx × 10^n` 形式で返す。

## Protocol And Release Policy

`ProtocolVersion` は現行 DTO surface を識別するための version であり、Cargo crate や npm package の semver とは別に扱う。現行 protocol snapshot は `2.0` である。

Protocol major/minor の運用は、後方互換保証ではなく、DTO surface の変更を見落とさないための識別子として扱う。

Wasm DTO と TypeScript facade は unknown `tag` / `code` を成功値として扱ってはならない。未知の protocol surface は `unsupportedProtocol` typed error へ変換する。

Rust 公開 enum は、利用者が網羅 match し得るものとして扱う。計算意味論に関わる `DomainErrorKind`、`DecimalRoundingMode`、`PowerSemantics`、公開 DTO の representation / method / error code を変更した場合は、protocol version と snapshot 更新を必要とする。

Release で公開 surface を変える場合は、生成 DTO、protocol snapshot、README、この文書、`implementation-status.md`、native/Wasm DTO conformance、browser e2e のうち影響を受けるものを同じ変更で更新する。

## Errors And Limits

公開 error は `domain`、`parse`、`inputLimit`、`computationLimit`、`unsupportedFeature`、`internalInvariant`、`unsupportedProtocol` に分類する。Wasm 境界では unknown tag/code、`null` / `undefined`、非 canonical number などを `unsupportedProtocol` または input limit として typed error に変換する。

Resource limits は公開契約であり、入力 byte 数、source AST nodes/depth、expression nodes、integer bits、cyclotomic order、algebraic degree、polynomial coefficient bits、root isolation steps、logical work units を制限する。制限超過時に近似値へ破壊的に落としてはならない。

## Session And Worker

npm facade は package root で `createSession` を、worker subpath で `createWorkerCalculator` を公開する。session dispatch は headless であり、`Evaluate` は calculate command を返し、呼び出し側が計算結果を `applyResult` で戻す。

worker cancellation は typed result を返し、壊れた部分結果を成功値として返さない。

## Consistency Gates

現行仕様との整合性を守るため、CI は生成 DTO の再生成差分、protocol snapshot、native/Wasm DTO conformance、browser e2e、package size budget、Rust/Node dependency audit、Rust doc tests、`f32` / `f64` 禁止、public enum match exhaustiveness を検査する。

公開 enum の分岐は wildcard arm で握りつぶさない。workspace lint `clippy::wildcard_enum_match_arm = "deny"` により、新しい variant 追加時に分岐更新漏れを検出する。
