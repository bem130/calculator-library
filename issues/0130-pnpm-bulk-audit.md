# Issue 130: Restore pnpm dependency audit

## Problem

The pinned pnpm 10.14.0 sends `pnpm audit` to npm's retired full-audit
endpoint, which now returns HTTP 410 before either workspace lockfile is
audited. This blocks the repository dependency-policy gate independently of
the source change under test.

## Requirements

- Update the pinned package manager to a compatible pnpm 10 release that uses
  npm's supported bulk-advisory endpoint.
- Preserve frozen-lockfile installs, package/example builds, browser E2E and
  the existing audit severity policy.
- Regenerate only package-manager-owned lockfile metadata when required.
- Verify both workspace audits and the complete repository gates.

## Resolution

The repository now pins pnpm 11.13.0, whose audit client uses npm's supported
bulk-advisory endpoint. Both package and example audits complete with no known
vulnerabilities instead of failing with HTTP 410. The example explicitly
allows the required `esbuild` install script through pnpm 11's supply-chain
policy; all other dependency build scripts remain denied by default.

Frozen-lockfile installs require no dependency or lockfile changes. Package
checks, the Wasm-backed example build and browser E2E retain their existing
commands and behavior.
