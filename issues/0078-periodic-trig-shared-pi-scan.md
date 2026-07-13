# Issue 78: periodic trig scanでπ enclosureを共有する

非unit intervalの`sin`/`cos`/`tan`は全周期判定、half-π scan上限、各extremumまたは
pole候補の包含判定で同じprecisionの`pi_bounds`を繰り返し構築する。特に各scan indexの
包含判定がMachin π recurrenceを再実行するため、引数の絶対値に比例して同一の保証区間
計算が最大4096回重複する。

- public trig呼出しごとにpaired π enclosureを一度構築し、全周期判定、scan上限、
  half-π multipleのdirection付きboundsへ借用する。
- extrema/pole候補、uncertain時の保守的fallback、scan上限、endpoint evaluation、
  directed enclosure、logical-work予約を変更しない。
- 旧独立`pi_bounds`経路と正負index、extrema、pole、全周期境界でexact比較する。
- 通常値と大きい周期引数をnative allocation・Criterion・logical work・Wasm/npmで
  before/after測定し、単なる上限引上げや入力固有分岐にしない。
- no-float、決定性、停止性、resource limit、公開protocolを維持し、全gateとsubagentの
  差分・全体整合・merge粒度review後にmainへ一度だけ統合する。
