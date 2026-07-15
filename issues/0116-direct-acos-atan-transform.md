# Issue 116: Cancel nested inverse-trigonometric complements in acos

## Problem

For `|x| >= 1/sqrt(2)`, transformed asin evaluates
`asin(x)=pi/2-atan(sqrt(1-x^2)/x)` (with the corresponding odd-function
direction). Acos then evaluates `pi/2-asin(x)`. Non-degenerate acos endpoints
therefore build and subtract two directed half-pi expressions even though the
positive result is directly the atan term. The representative transformed acos
path allocates substantially more than its asin input path.

## Requirements

- For endpoints in the outer transform region, evaluate the mathematically
  equivalent direct atan form. Positive inputs use
  `acos(x)=atan(sqrt(1-x^2)/x)`; negative inputs use
  `acos(x)=pi-atan(sqrt(1-x^2)/|x|)`.
- Select sqrt, atan, and pi directions from monotonicity. Preserve certified
  containment even when direct cancellation produces a tighter interval than
  the former independently rounded complements.
- Preserve ±1, zero, the half and `1/sqrt(2)` boundaries, negative symmetry,
  non-degenerate antitonic endpoint selection, stopping, logical-work/resource
  accounting, no-float policy, and public protocol.
- Keep the central transform and unit-series regions on their existing paths.
  Do not special-case one expression or loosen precision/limits.
- Add exact-point and non-degenerate regression/oracle cases across both signs
  and the transform boundary. Measure allocation, peak, native timing, logical
  work, Wasm/npm, CLI/example/browser, and controls.
- Complete focused and repository gates, diff/consistency/merge reviews, and one
  integration into `main`.

## Resolution

Non-degenerate acos classifies each selected endpoint once. Endpoints in the
outer transform region use the direct directed atan form; when both endpoints
are positive, the caller does not construct pi at all. Negative outer endpoints
retain one shared pi and reverse the sqrt/atan direction before subtraction.
Exact-point paired evaluation and central/unit regions retain their prior path,
avoiding extra classification work outside the measured scope.

Against base `ef4b9fa`, deterministic one-call allocation for
`acos((2+sin(1))/3)` moved from 1,223,322 bytes / 2,842 blocks (60,805 / 85
peak) to 958,986 / 2,470 (58,069 / 81). Exact `acos(3/4)`, `acos(5/8)`,
transformed asin, and atan controls are unchanged. Same-host 20-sample Criterion
ranges were 11.997--13.347 ms at base and 11.657--12.354 ms for the candidate;
the ranges overlap, so no timing claim is made. Expanded logical-work output is
byte-identical with SHA-256
`7342dcca027f7a801364ddc8624fba95d88617161fbfc32dec27e63ea11c4773`.

The optimized Wasm artifact is 833,392 bytes with SHA-256
`41dc6351ef70d4773e0529bbb9b4e5fe748cb948d43f8c3f7d32d62e41f34ff2`
(base: 831,928 bytes). A ten-iteration/two-warmup npm public-path run measured
111.44 ms/iteration with a 1,794-byte payload; the short run is a boundary
validation, not a timing claim. The CLI preserved
`acos(1/3*sin(1)+2/3)`, and the browser regression confirmed a positive
certified enclosure below two radians.

Repository gates passed formatting, clippy, 390 core tests, 37 native Wasm
tests, 23 wasm32 tests, doc tests, generated DTO/protocol/no-float checks,
package/example builds, browser E2E, size budgets, and `cargo deny`. Both pnpm
audit commands reached the registry's retired endpoint and returned HTTP 410;
this is an external audit-service failure rather than an advisory result.

Review moved negative-magnitude construction inside the direct branch and
restored the unrelated one-bit exponential-plan regression. The added negative
central non-degenerate allocation control is unchanged from base at 1,019,757
bytes / 1,793 blocks (48,790 / 74 peak). A bit-length proof classifies values
strictly below one half without constructing integer squares; boundary-near
values retain the exact square comparison. Boundary tests cover
both signs at 707/1000 and 708/1000, and positive direct bounds are contained
by the former paired certified bounds at 1, 64, and 128 bits.
