# dlt-tracing-lib Subtree Delta Report - 2026-06-11

Purpose: close the `Q-PROD-11b` audit for `dlt-tracing-lib/`.

## Verdict

**Vendored snapshot** of
[eclipse-opensovd/dlt-tracing-lib](https://github.com/eclipse-opensovd/dlt-tracing-lib)
at release v0.1.2 (`e33196e`, 2026-01-09), with **nine local
SPDX-header-only patches** (three-line SPDX/copyright headers prepended
to `dlt-rs`, `dlt-sys`, `tracing-dlt`, and integration-test sources as
part of the coding-standards compliance sweep). No functional local
changes.

## Divergence

| Category | Count |
|---|---|
| Total tracked files | 41 |
| Blob-identical to v0.1.2 | 32 |
| SPDX-header-only local patches | 9 |
| Local-only / upstream-only | 0 |

Upstream `main` past v0.1.2 contains only the fork sync workflow and
monitoring notices - no functional library changes and no newer
release.

## Action

**Maintain as-is.** Syncing to upstream would regress the SPDX
compliance patches for zero functional gain. Revisit on the next
upstream release (v0.1.3+) and evaluate functional changes then; keep
under PROD-15 monthly cadence.
