# Issue 8: example-ui の表示契約と数式編集モデルを整合させる

## 背景

`examples/vanilla-web` の現在の実装と公開されている表示契約を照合し、表示上の不整合を根本から解消する。同時に、数式入力欄をキーボードだけでなく pointer、mouse、touch からも通常のテキスト編集と同じ感覚で操作できるようにする。

この修正は example-ui に限定した割り込み作業である。完了後は calculator-library の計算性能、計算効率、計算可能範囲を改善するメインループへ戻る。

## 問題

表示値、内部の式、選択範囲、スクロール位置、IME composition、画面キーパッド入力を別々の場当たり的な状態として扱うと、次の不具合が生じる。

- 実装の表示と `doc/public-contract.md` 等が定める契約が一致しない。
- クリックまたはドラッグした位置と caret/selection が一致せず、横スクロール時には誤差が増える。
- touch と互換 mouse event、または pointer と click の双方を処理して入力が重複する。
- IME composition 中の未確定文字列や selection が画面キーパッド操作で破壊される。
- DOM の selection とアプリケーション内の cursor state が競合する。
- mobile/desktop で異なる入力経路となり、片方だけ回帰する。
- 見かけ上の座標補正に依存すると font、zoom、DPR、スクロール、レイアウト変更で再発する。

## 根本方針

まず現行 UI、`doc/design.md`、`doc/public-contract.md`、`doc/implementation-status.md`、calculator package の公開 protocol を照合し、どの値が表示の source of truth かを明文化する。その上で、ブラウザ標準の編集・selection primitives を中心に入力モデルを統一する。

- expression、selection start/end/direction、composition 状態、scroll は所有者と同期境界を一意にする。
- caret hit testing はブラウザのレイアウト結果を用いる。固定幅仮定や magic number による座標補正は導入しない。
- Pointer Events を採用する場合は pointer capture と primary pointer の規則を定め、互換 mouse/click event を二重処理しない。fallback が必要なら排他的に選択する。
- IME composition 中はブラウザの未確定編集を尊重し、確定前後の selection/input 順序を壊さない。
- 画面キーパッドと物理キーボードは同じ編集 command 層を通し、文字挿入と cursor 移動の意味を一致させる。
- selection/caret を移動したときは必要な範囲だけ scroll into view し、利用者が行ったスクロールや選択を不必要にリセットしない。
- focus、button semantics、accessible name、現在位置の視認性を含むキーボードおよび支援技術の利用可能性を維持する。

## 受入条件

### 表示契約

- 現在の実装と文書化された表示契約の不一致が列挙され、表示 source of truth を含む修正がコード・テスト・必要な契約文書へ一貫して反映されている。
- exact/approximate、成功/エラー、入力中/評価後を含む既存の calculator protocol の意味を UI 独自解釈で変更しない。
- 修正は calculator の決定性、no-float 方針、資源制限、公開 protocol 互換性を弱めない。

### pointer、mouse、touch と selection

- desktop のクリックで、横スクロール、可変幅文字、zoom を含め、視覚的に選んだ文字境界へ caret を移動できる。
- desktop のドラッグで前後どちらの方向にも selection を作成・更新でき、pointer が要素境界を越えても操作が破綻しない。
- touch の tap と drag/selection 操作が利用でき、同一 gesture から編集 command が重複発火しない。
- selection の anchor/focus または direction を必要な範囲で保持し、置換入力と cursor 移動が通常のテキスト編集規則に従う。
- expression が表示幅を超える場合、caret/selection と scrollLeft の関係が保たれ、操作のたびに末尾へ強制移動しない。

### キーパッドと物理キーボード

- 画面キーパッドに左矢印と右矢印を追加し、押下後も数式入力の focus と selection を維持する。
- `ArrowLeft`、`ArrowRight`、`Home`、`End` を標準的な cursor/selection 移動として処理する。
- 少なくとも Shift による selection 拡張、既存の Backspace/Delete/入力/評価操作との相互作用を確認する。OS 固有 modifier はブラウザ標準挙動を阻害しない。
- 画面キーパッドと物理キーで同じ初期 state に同じ command を適用した結果が一致する。

### IME とアクセシビリティ

- composition start/update/end と input/keydown の順序を考慮し、未確定入力を二重挿入、消失、途中評価しない。
- pointer/touch 操作、画面キーパッド、物理キーボードのすべてで focus の行方が予測可能である。
- 新しい矢印ボタンには意味の分かる accessible name があり、キーボード操作と focus indication が維持される。

### 回帰テスト

- desktop browser E2E に click positioning、forward/backward drag selection、横スクロール、物理キー、画面矢印キー、重複発火防止の回帰を追加する。
- mobile/touch browser E2E に tap/drag、scroll、画面矢印キー、互換 event の重複防止の回帰を追加する。
- IME/composition と selection command 層は、ブラウザ自動化で安定して再現できる範囲を E2E、残りを focused unit/integration test で固定する。
- 表示契約の主要 state をテストし、見た目の snapshot だけでなく表示内容と accessible semantics を検証する。

## 検証 gate

変更を統合する前に repository が実際に提供する command を確認し、少なくとも次を成功させる。

1. example-ui の focused unit/integration tests。
2. desktop と mobile profile の browser E2E。
3. `packages/calculator` の test/typecheck/build。
4. `examples/vanilla-web` の test/typecheck/build。
5. repository 全体で要求される format/lint/test/build gate。
6. 差分 review、全体整合 review、および branch の commit/merge 粒度 review。全指摘を修正し、必要な gate を再実行する。

検証結果には実行した command、対象 browser/profile、成功結果、未検証事項があればその理由を記録する。

## 統合条件

- 専用 development branch 上に checkpoint commit を蓄積し、この Issue の途中では main へ merge/push しない。
- 統合直前に first-parent log と branch 内 commit 列を確認する。
- repository lease を取得して main の更新競合を避ける。
- main が作業開始点から分岐していなければ `git merge --ff-only` で一度だけ統合し、統合後に development branch を削除する。
- example-ui の統合完了後は、計測に基づく calculator performance/capability の次 slice を main から開始する。
