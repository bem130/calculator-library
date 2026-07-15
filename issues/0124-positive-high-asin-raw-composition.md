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

Pending measurement and implementation.
