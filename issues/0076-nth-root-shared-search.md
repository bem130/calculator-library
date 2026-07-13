# Issue 76: exact-point nth rootのroot探索を共有する

`nth_root_nonnegative_rational`は同じexact Rationalからscaled lower/upperを作った後、
lowerのfloor n乗根とupperのceil n乗根を独立に探索する。ceil helperは内部でfloor探索を
再実行するため、一般indexの大整数二分探索と各候補の冪計算が重複する。

- scaled lower/upperは高々1だけ異なることを使い、lower floor rootを一度探索する。
- lower rootのindex乗がscaled upperと等しければupper rootを共有し、そうでなければ
  lower root+1をceil rootとする。zero、perfect power、non-perfect powerを含める。
- 旧独立floor/ceil経路と複数index・precision・符号付きodd rootでexact比較し、
  domain、precision、停止性、logical work、resource limit、no-floatを維持する。
- cube-rootを含むalgebraic representativeのallocation、Criterion、logical work、
  Wasm/npm境界をbefore/after測定し、sqrt/general-power control、全gate、subagentの
  差分・整合・merge粒度review後にmainへ一度だけ統合する。
