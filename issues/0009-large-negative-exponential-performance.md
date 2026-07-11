# Issue 9: 巨大な負指数の exponential を有界な中間表現で計算する

## 背景

公開入力 `e^(-10000)` および同義な構文として提供される `exp(-10000)` が、現行実装で計算不能、資源制限到達、または実用上大幅に遅くなる問題を現行 `main` で再現し、数値アルゴリズムと表現の根本から解消する。

現行 `main` には directed exponential endpoint、Taylor recurrence、operand clone、range reduction などの改善が既に入っている。それらによって解消済みとも、過去の測定だけから未解消とも仮定せず、native core、CLI、Wasm/npm facade、example-ui の各公開経路を同一 revision で測定する。

この作業は追加の数値計算性能・対応範囲割り込みである。完了後は calculator-library の計算性能、計算効率、計算可能範囲を継続的に改善するメインループへ戻る。

## 問題

巨大な負指数を正指数の結果の単純な reciprocal として処理すると、最終結果には不要な `exp(10000)` の巨大な整数、有理数、分母または整数冪を先に構築し得る。さらに、計算自体が終わっても、非常に小さい正値を巨大な固定小数文字列へ展開すると Wasm 境界、JSON serialization、npm facade、UI presentation が支配的になり得る。

したがって、次の段階を分離して原因を特定する。

- parse、exact/symbolic classification、approximation/refinement の各段階。
- range reduction、Taylor recurrence、directed endpoint、負指数の reciprocal 境界。
- 多倍長整数・有理数・整数冪の時間、allocation、peak live memory と論理 work。
- resource limit または cancellation に到達する位置と、その accounting の妥当性。
- core result から CLI、Wasm protocol、npm facade、example-ui 表示へ渡る serialization/presentation。
- enclosure または scientific presentation が保証区間を保ったまま値の桁数に対して有界かどうか。

## 根本方針

- 符号、range reduction、reciprocal と directed rounding/enclosure の関係を追跡し、負の巨大指数で不要な正側巨大中間値を構築しない一般的なアルゴリズムにする。
- 正確な 0 への underflow は行わず、結果が正であることと要求精度を満たす保証区間を維持する。
- 負側だけの特殊分岐ではなく、正負の指数、range-reduced endpoint、refinement と presentation に一貫する表現を用いる。
- 出力サイズは値の小ささそのものではなく要求精度と公開表示契約に対応させる。巨大な固定小数文字列を作らず、必要なら既存 protocol と互換な scientific/enclosure presentation を整備する。
- 決定性、停止性、cancellation、logical-work/resource accounting、no-float 方針、公開 protocol 互換性を維持する。

## 禁止事項

- resource limit、桁数上限または反復上限を単に引き上げて通すこと。
- `-10000` または `exp(-10000)` だけを認識する特殊分岐。
- 要求精度を下げる、保証なしの近似へ切り替える、または 0 に underflow させること。
- `exp(10000)` の巨大中間有理数を従来どおり構築した後、表示だけを変更して性能問題を隠すこと。
- 非常に長い 0 列を含む固定小数文字列を生成することで表示を成立させること。
- logical work の課金を省略、後回し、または実コストから乖離させて資源制限を回避すること。
- benchmark、期待値またはテスト閾値だけを変更して実装の退行を隠すこと。

## 受入条件

### 現行 main の再現と原因分離

- `e^(-10000)` と、公開構文として有効なら `exp(-10000)` を現行 `main` で測定し、成功/失敗、結果 state、時間、allocation、peak、logical work、resource/cancellation state、出力サイズを記録する。
- native core、CLI、Wasm、npm facade、example-ui について同じ入力と精度を追跡し、計算、境界変換、serialization、presentation のどこが支配するかを分離する。
- benchmark の revision、toolchain、build profile、反復回数、実行 command を記録し、before/after を再現可能にする。

### 数値・資源契約

- `exp(x)` の結果は有限な入力に対して正であり、負側で 0 に underflow しない。
- directed lower/upper endpoint と reciprocal の向きが正しく、返す enclosure が真値を含み、要求精度を満たす。
- 単調性を保ち、少なくとも `x1 < x2` に対する代表的な負側・0近傍・正側の enclosure が順序契約に反しない。
- range reduction と refinement が決定的に停止し、cancellation を尊重する。
- 新しい整数、有理数、pow、series、formatting の仕事を既存方針に従って logical work と resource limit に課金する。
- no-float 方針を維持し、公開 protocol の既存 consumer を破壊しない。

### 対応範囲と一般化

- `exp(-10000)` に加えて、負側の近傍、より絶対値の小さい/大きい指数、0近傍、通常値、および `exp(10000)` を回帰対象にする。
- 境界ケースには少なくとも `-10001`, `-10000`, `-9999`, 大きさの異なる巨大負指数、`-1`, `0`, `1`, 対応する巨大正指数を含め、実装上の range-reduction 境界も追加する。
- `e^x` と `exp(x)` がともに公開構文なら、意味・精度・資源 accounting・presentation が一致する。
- 巨大正指数についても不要な退行がなく、成功する入力は正しく scientific/enclosure presentation され、資源上限に達する場合は決定的で契約どおりの状態を返す。

### 公開経路と presentation

- native core と CLI が保証区間および状態を失わず、CLI が巨大な固定小数展開を不要に生成しない。
- Wasm/npm facade で serialization の時間・allocation・payload size が要求精度に対して妥当で、core の結果や resource stateを変更しない。
- example-ui が非常に小さい正値を 0 と表示せず、巨大な 0 列も生成せず、既存の exact/approximate/error/resource 表示契約と整合する。
- scientific/enclosure 表示を変更する場合は core、CLI、TypeScript types、Wasm snapshot、npm facade、example-ui と契約文書を一貫して更新する。

### 性能と回帰テスト

- native benchmark と allocation harness に巨大な正負指数を加え、logical work とともに before/after を記録する。
- focused test で正値性、単調性、reciprocal の方向、保証区間、要求精度、range-reduction 境界、停止性、cancellation、resource accounting を固定する。
- 通常の `exp(1)`、小さな負指数、既存 approximate/general-power benchmark に有意な性能退行がないことを確認する。
- CLI integration、Wasm、npm facade、example-ui の公開経路を検証し、desktop/mobile browser E2E で表示と操作の回帰を確認する。

## 検証 gate

統合前に repository が提供する実 command を確認し、少なくとも次を成功させる。

1. exponential、range reduction、directed enclosure、resource accounting の focused tests。
2. native benchmark、allocation baseline、logical-work baseline の before/after 比較。
3. native core、CLI、Wasm/npm facade の integration tests。
4. `packages/calculator` の test/typecheck/build と package validation。
5. `examples/vanilla-web` の test/typecheck/build、および desktop/mobile browser E2E。
6. repository 全体の format、lint、no-default、native/Wasm test、doc test、no-float、protocol snapshot、generated files、dependency policy、package-size gate。
7. subagent による差分 review、全体整合 review、first-parent/branch commit列を含む merge粒度 review。指摘を修正後、影響する gate を再実行する。

検証記録には revision、command、toolchain、profile、入力、精度、時間、allocation、logical work、payload/output size、各公開経路の結果 state、残る bottleneck を含める。

## 統合条件

- 専用 development branch 上に小さく意味のある checkpoint commit を蓄積し、この Issue の途中では main へ merge/push しない。
- 統合直前に main の first-parent log、作業開始点、branch 内 commit 列、origin/main との差分を確認する。
- repository lease を取得し、main が作業開始点から分岐していなければ `git merge --ff-only` で一度だけ統合する。
- 内容 commit 1個を包むだけの `--no-ff` merge commitや checkpoint ごとの merge commitを作らない。
- 全 review と gate が完了するまで統合せず、統合・push 後に作業 branch を削除して lease を解放する。
- 完了後は計測に基づく calculator performance/capability の次の独立 slice を main から開始する。
