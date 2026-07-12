# Issue 34: inverse trig domainの±1境界を構造判定する

`asin` / `acos` のinterval domain層は全入力でRational ±1を構築し、上下端との
汎用比較を4回行う。canonical Rationalの分子絶対値と正分母を比較するunit predicateと
端点符号から、完全にdomain外・一部だけdomain外・範囲内を直接分類できる。

- 完全に±1の外側は従来どおりtyped domain errorにする。
- intervalの一部だけが外側なら従来どおりunsupported expressionにする。
- ±1境界を含む範囲内intervalと通常のexact pointを維持する。
- directed bounds、logical work、resource accounting、no-float、公開protocolを変えない。
- native allocationをbefore/after測定し、全gateとsubagent review後にmainへ一度だけ統合する。
