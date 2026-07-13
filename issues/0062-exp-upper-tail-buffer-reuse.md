# Issue 62: directed exp upper tailのbufferを再利用する

upper-only exp endpointはrecurrence stateを直後に破棄するが、tail補正時に部分和、
term、共通分母から新しいBigInt積を構築している。stateを消費し、所有bufferを
in-place更新して同じupper Rationalを構築する。

- lowerも必要なpaired exact boundsは借用tail計算を維持する。
- tail式、canonicalization、directed enclosure、logical workを変更しない。
- 非退化endpoint、binary scaling、通常値とlarge expを測定し全gate後に統合する。
