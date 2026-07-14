# Issue 95: Move owned token payloads into the source AST

## Problem

The lexer owns number payloads in its token vector. During
primary-expression parsing, the parser clones the current token before advancing,
so every number literal `String` is duplicated into the source AST while the
original remains live in the token vector until parsing finishes. Wide expressions
therefore allocate and retain storage proportional to their literal payload twice.

## Requirements

- Consume the current token and move owned payloads into the source AST without
  cloning them.
- Preserve lookahead, precedence, associativity, implicit multiplication,
  function parsing, byte spans, and parse-error precedence and payloads.
- Preserve source AST node/depth limits, exact semantics, logical-work charging,
  no-float policy, and every public protocol surface.
- Cover successful number/function/parenthesized parsing and representative
  errors around the consumed token boundary.
- Record deterministic parse/public-path allocation, scaling timing, logical
  work, and Wasm/npm boundary evidence with unchanged-result controls.
- Complete focused tests, repository gates, diff and whole-system reviews, and
  merge-granularity review before one integration into `main`.

## Resolution

Primary parsing now consumes the current token and moves owned number payloads
into the source AST. Consumed vector slots retain their spans with a payload-free
sentinel, preserving lookahead and end/error locations without shifting the token
vector.

At base `e316e36`, one public 256-term wide sum allocated 81,525 bytes / 2,888
blocks and peaked at 38,104 / 1,023. The candidate allocates 80,865 / 2,632 and
peaks at 37,444 / 767, removing exactly the 660 bytes and 256 blocks occupied by
the cloned literal payloads. Logical work remains 261 units. Parse-only Criterion
ranges overlapped (45.735--47.633 us base, 40.549--53.232 us candidate,
`p=0.21`), so no timing claim is made.

The public Wasm/npm smoke retained its 1,728-byte payload and 3,528-byte JS heap
delta. Short samples were 0.924 ms/iteration at base and 1.340 ms at candidate
and are boundary evidence rather than a timing claim. The release artifact moved
from 829,226 bytes (`84e64abd994e78564aa01bd993c02cd44b9f3cfd5deb171b92c3811929b77f37`)
to 828,720 bytes (`c4e863ac13c094a1147943165843e1725900bfda009090d64bbd4075f840a91a`).
