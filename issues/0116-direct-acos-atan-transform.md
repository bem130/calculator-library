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
