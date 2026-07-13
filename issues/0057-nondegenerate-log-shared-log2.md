# Issue 57: 非退化log endpointでln(2) enclosureを共有する

非退化intervalのlogはlower/upper endpointを方向付き評価するが、range reductionの
binary exponentが非zeroなら入力非依存の同一`ln(2)`級数を各endpointで再計算する。
endpoint固有のreduced argumentと方向付き級数は独立に保ち、同一精度のpaired
`ln(2)` enclosureだけを一度構築して符号・方向に応じて選択する。

- exact pointとbinary exponent zeroだけのintervalは既存経路を維持する。
- 正負または異なるbinary exponent、unit-range混在でもdirected boundを維持する。
- 旧独立endpoint評価とのexact一致を境界・正負exponentで回帰する。
- logical work、resource accounting、no-float、公開protocolを変更しない。
- non-degenerate logのallocation/timingと公開境界を測定し、全gate/review後にmainへ一度だけ統合する。
