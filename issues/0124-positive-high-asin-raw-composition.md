# Issue 124: Compose positive high-asin atan endpoints before dyadic rounding

## Problem

Positive non-degenerate asin endpoints at `x^2>=1/2` evaluate
`pi/2-atan(sqrt(1-x^2)/x)` as canonical Rationals before immediate directed
dyadic rounding. The transformed asin representative still allocates 733,736
bytes despite the raw atan endpoint infrastructure.

## Requirements

- Keep the unit atan recurrence raw through exact `pi/2-atan` composition and
  round once for positive high-transform asin endpoints.
- Preserve sqrt/atan directions, monotonicity, positivity, precision, ordering,
  exact/special/negative/mid-transform/unit fallbacks, resource accounting,
  no-float, determinism, and protocol.
- Add canonical oracle, boundary/coarse/fallback regressions, allocation,
  native/logical/Wasm/npm/CLI/browser evidence, all gates, and three reviews.

## Resolution

Positive non-degenerate high-transform endpoints now retain the oppositely
directed unit atan recurrence as raw numerator/denominator parts, compose
`pi/2-atan` exactly with the selected shared-pi bound, and round once. A coarse
sqrt ratio outside the unit range falls back to the canonical route. Exact,
special, negative, mid-transform, and unit-series routes remain unchanged.

Against base `625a1ef`, `asin((2+sin(1))/3)` moved from 733,736 bytes / 1,874
blocks (40,893 / 79 peak) to 606,168 / 1,793 (36,757 / 73 peak): 17.4% fewer
bytes and 81 fewer blocks. Native ranges moved from 24.17--27.74 ms to
1.419--1.504 ms. The existing target uses 200,442 minimum logical-work units;
base/candidate output matched at SHA-256
`c26664ee5491163d519586976c98f433a1e45c8aed35ded9effa4d0b81d50e30`.

Optimized Wasm moved from 837,564 bytes
(`6c708a2d469d1c1ad783a3b0c2c770e0c606fade0b2ca84835439fbc2c9348e4`)
to 838,728 bytes
(`68af2bc35f8da0980707e709492e413e1097f34fb59cf65892732959dc05cb4b`),
within budget. A 100-iteration/10-warmup npm run moved from 113.742 to
5.192 ms/iteration with the same 1,788-byte payload and definition v26.
