# Issue 58: exact-point large expのbinary scaling計画を共有する

`exp(±10000)`のようなexact dyadic pointはlower/upperが同じ値でも、binary-scaled
endpoint evaluatorを二回呼び、同じworking precision、certified `ln(2)` enclosure、
midpoint quotient、binary exponentを独立に構築している。exact pointでは一度だけ
計画し、方向固有のresidual・Taylor bound・dyadic roundingへ共有する。

- 非退化endpointはworking precisionが異なり得るため既存の独立計画を維持する。
- 正負のresidual方向、binary exponent、正値性、単調性、保証区間を維持する。
- exponent capとshift overflowを数値recurrence前のtyped errorとして維持する。
- logical work、resource accounting、no-float、公開protocolを変更しない。
- `exp(±10000)`、通常値、上限境界を回帰・測定し、全gate/review後にmainへ一度だけ統合する。
