# Issue 122: Compose negative central-acos endpoints before dyadic rounding

## Problem

After the outer-acos raw endpoint work, `acos((-1+sin(1))/3)` remains the
largest inverse-trigonometric allocation control. Both selected endpoints are
negative and satisfy `|x|<=1/2`, but the evaluator canonicalizes the unit asin
series and then canonicalizes `pi/2-asin(x)` before immediately rounding it to
a directed dyadic.

## Requirements

- Compose shared `pi/2` and the existing raw unit-asin fraction exactly at the
  final directed dyadic boundary for uniformly negative central endpoints.
- Preserve `acos(-m)=pi/2+asin(m)`, antitone endpoint selection, direction,
  positivity, monotonicity, precision, ordering/error precedence, stopping,
  logical-work/resource accounting, determinism, no-float, and protocol.
- Retain canonical routes for mixed sign/classification, positive central,
  outer, exact, zero, and special endpoints.
- Add raw/canonical oracles, boundary and reversed-input regressions, native and
  npm benchmarks, browser coverage, allocation/timing/logical-work/Wasm evidence,
  all repository gates, and diff/whole-system/merge-granularity reviews.

## Resolution

For uniformly negative non-degenerate central endpoints, the evaluator now
uses `acos(-m)=pi/2+asin(m)`, keeps the unit-asin recurrence as raw numerator
and denominator parts, composes it exactly with the selected shared-pi bound,
and rounds once to the directed dyadic boundary. Mixed, positive, outer,
exact, zero, and special routes retain their established dispatch.

Against base `e87ece6`, `acos((-1+sin(1))/3)` moved from 1,019,757 bytes /
1,793 blocks (48,790 / 74 peak) to 825,605 / 1,713 (33,598 / 71 peak):
19.0% fewer bytes, 80 fewer blocks, and 31.1% lower peak bytes. Positive and
negative outer controls were byte-identical. A 20-sample native run measured
base at 8.43--9.29 ms and candidate at 1.44--1.87 ms.

Logical-work output stayed byte-identical at SHA-256
`7342dcca027f7a801364ddc8624fba95d88617161fbfc32dec27e63ea11c4773`.
Optimized Wasm moved from 835,889 bytes
(`8f0c0c618cd609b2b319114263dfa7558fb7cc77ecd31aa7792a7be2080bbd89`)
to 836,732 bytes
(`75c301f61fe9993392711c88ce34fc3f9c337503dd5d91647f7d80693f110ee2`),
within budget. A 100-iteration/10-warmup npm run moved from 60.929 to
4.257 ms/iteration with the same 1,788-byte payload and definition v25.
