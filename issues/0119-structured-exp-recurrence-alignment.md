# Issue 119: Retain dyadic alignment across exponential recurrence additions

## Problem

On current main, one `2^sqrt(2)` calculation allocates 101,393 bytes in 692
blocks. DHAT attributes about 78 KB across the two directed exponential
endpoints to the growing Taylor numerator: each step multiplies the sum by the
bounded term index, materializes the fixed denominator left shift, and only
then adds the next numerator power. Reordering those two operations was rejected
in Issue 107, while balanced and reverse finite-sum trees were rejected in
Issue 115 because they increased blocks or peak live state.

## Requirements

- Investigate a representation that retains the dyadic denominator alignment
  across addition rather than merely reordering the same multiply and shift.
- Preserve the exact finite Taylor sum, upper-tail proof, directed lower/upper
  rounding, exact-point and non-degenerate endpoints, binary scaling, general
  non-dyadic Rational fallback, and all stopping thresholds.
- Do not raise limits, reduce precision, approximate with floats, or specialize
  a public source expression. Preserve deterministic logical-work/resource
  accounting, typed errors, no-float policy, and public protocol.
- Compare against current main for general power, ordinary/tiny/large positive
  and negative exp, and non-degenerate exp. Reject the representation if bytes,
  blocks, peak live state, timing, or Wasm size trade away a representative
  path without a justified general benefit.
- Add an independent exact recurrence oracle for all retained representations.
  Record native allocation/timing, logical work, Wasm/npm, CLI, package/example,
  and browser evidence. Complete diff, whole-system, and merge-granularity
  reviews plus repository gates before one ff-only integration into `main`.

## Resolution

Rejected after the mandatory first measurement; no runtime or test change is
retained. A bounded pair-step prototype combined two exact recurrence steps as

`S[r+2] = S[r] * n * (n+1) * b² + (a^r * a) * ((n+1) * b + a)`.

where `n = r + 1`. It therefore shifted the growing global sum once per pair and held only one
local correction, unlike Issue 115's balanced tree. An independent legacy
recurrence oracle covered term counts 0--4 and 15--17, dyadic denominators,
large odd numerators, and general-denominator fallbacks; focused tests and
clippy passed.

The current-main DHAT baseline for `2^sqrt(2)` completed in under one second at
101,393 bytes / 692 blocks (6,223 / 43 peak). The pair-step candidate did not
complete the same one-calculation DHAT command after more than 120 seconds and
was terminated. The exact correction multiplication causes catastrophic
profiler-visible allocation work before any possible reduction in global sum
shifts, so it fails the mandatory allocation/time gate by orders of magnitude.
Downstream timing, logical-work, Wasm/npm, and public-path candidate gates were
intentionally skipped. The runtime and expanded prototype oracle were restored
byte-for-byte to current main.

Do not retry bounded multi-step affine corrections, larger blocks, or the
balanced/reverse variants from Issue 115 without a multiplication primitive or
representation that avoids materializing the correction. A pending shift alone
cannot cross addition: adding `a^n` forces the aligned integer to exist.

The restored tip passed formatting, native and wasm32 clippy, no-default check
and 391 core tests, 37 native Wasm tests, 23 wasm32 tests, doc tests/build,
generated DTO regeneration, protocol snapshot, no-float, `cargo deny`, package
check, example build, external arithmetic oracle, package-size, and browser E2E.
Logical-work output remained byte-identical at SHA-256
`7342dcca027f7a801364ddc8624fba95d88617161fbfc32dec27e63ea11c4773`.
The restored optimized Wasm is the main artifact: 833,365 bytes at SHA-256
`7dce68b0b7c4c9ef1fb5b063627fc954a328b3ba0e452a71e63c21b97c48834c`.
Both pnpm audit commands reached the registry's retired endpoint and returned
HTTP 410 rather than an advisory result.
