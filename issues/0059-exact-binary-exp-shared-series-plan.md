# Issue 59: exact-point large expのTaylor項数計画を共有する

Issue 58で`exp(±10000)`のexact dyadic pointはworking precision、certified
`ln(2)` enclosure、midpoint quotient、binary exponentを共有したが、方向固有の
endpoint評価は同じworking precisionからTaylor項数を再計算している。項数は値や
丸め方向には依存しないためbinary-scaling計画へ含め、上下で一度だけ構築する。

- residual、Taylor recurrence、reciprocal方向、dyadic roundingは方向固有に保つ。
- working precisionが異なり得る非退化endpointは独立計画を維持する。
- 正値性、単調性、保証区間、logical work、resource accountingを変更しない。
- `exp(±10000)`と近傍・通常値を回帰し、allocation/timingと公開境界を測定する。
- 全gateとsubagentの差分・全体整合・merge粒度review後にmainへ一度だけ統合する。
