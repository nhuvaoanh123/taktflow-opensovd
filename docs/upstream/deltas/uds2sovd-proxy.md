# uds2sovd-proxy Delta Report - 2026-05-01

Purpose: `Q-PROD-11b` tree-shape audit for the local
[`uds2sovd-proxy/`](../../../uds2sovd-proxy/) subtree after the `PROD-20.1`
through `PROD-20.5` implementation thread.

## Upstream Baseline

| Field | Value |
|---|---|
| Upstream repo | [eclipse-opensovd/uds2sovd-proxy](https://github.com/eclipse-opensovd/uds2sovd-proxy) |
| Branch compared | `main` |
| Upstream head | `f6467e205620fb0017c0460ea87e8d99fa7b7bf0` |
| Latest upstream commit | 2026-04-23, CI workflow update |
| Local comparison scope | tracked files under `uds2sovd-proxy/` |
| Method | temporary shallow clone of upstream `main`; filename and SHA-256 content comparison against local tracked files |

## Tree-Shape Result

`uds2sovd-proxy/` is confirmed as a vendored upstream scaffold with a Taktflow
product implementation overlaid. It is not a separate name collision, but the
local code is intentionally far ahead of upstream.

| Metric | Count |
|---|---:|
| Local tracked files | 26 |
| Upstream tracked files | 17 |
| Common paths | 17 |
| Common paths with identical content | 9 |
| Common paths with changed content | 8 |
| Local-only tracked paths | 9 |
| Upstream-only tracked paths | 0 |

## Local-Only Product Paths

- `rust-toolchain.toml`
- `src/config.rs`
- `src/lib.rs`
- `src/mdd.rs`
- `src/proxy.rs`
- `src/sovd.rs`
- `src/tracing_setup.rs`
- `src/uds.rs`
- `uds2sovd-proxy.example.toml`

These are the `PROD-20` implementation surface. Upstream does not currently
contain competing source files or a design that should reopen the local scope.

## Changed Common Paths

- `.github/workflows/build.yml`
- `.github/workflows/pr-checks.yml`
- `.github/workflows/pre-commit.yaml`
- `.gitignore`
- `Cargo.lock`
- `Cargo.toml`
- `README.md`
- `src/main.rs`

The changed workflow files are expected monolith-local divergence. The changed
Rust and README paths reflect the Taktflow implementation and operator-facing
documentation, not an upstream absorption conflict.

## Upstream Design Watch

[eclipse-opensovd/opensovd#63](https://github.com/eclipse-opensovd/opensovd/pull/63)
remains open draft and was last updated on 2026-02-03. The upstream proxy repo's
latest movement is CI-only. This does not alter the `PROD-20` decision that
Taktflow must own the first working UDS-to-SOVD ingress proxy.

## Decision

Keep `PROD-20.1` through `PROD-20.5` closed. Continue watching upstream design
PR `#63`, but do not remove or defer local proxy functionality waiting for it.
