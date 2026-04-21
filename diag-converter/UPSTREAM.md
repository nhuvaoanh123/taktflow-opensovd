# diag-converter — upstream tracking

This directory is a **vendored snapshot** of the third-party tool
[`bburda42dot/diag-converter`](https://github.com/bburda42dot/diag-converter),
brought into the Taktflow monolith so `xtask` can turn YAML-sourced ECU
descriptions into MDD + ODX without a network dependency or an external
build step.

## Pinned commit

Vendored at upstream HEAD `a184aef98e5c7e880e61c1e919342c9ded35950f`
as of 2026-04-20.

## Why vendored rather than cargo-git dep

- Matches the rest of the monolith (`opensovd-core/`, `fault-lib/`,
  `classic-diagnostic-adapter/`, `odx-converter/`, etc. are all vendored).
- Pre-1.0; we pin our own known-good commit rather than track `main`.
- The tool is Apache-2.0, compatible with our stance.

## Policy

- **Read-only.** Do not edit files under this directory. If a fix is
  needed, file an upstream issue and either wait for a fix or stand up
  a local patch file under `docs/upstream/patches/` (none today).
- **No `target/` directory** ships in the vendored tree; builds go into
  the usual `target/` which is already gitignored.
- **Upstream sync** happens via the normal monitoring rule — see
  [`../docs/upstream/README.md`](../docs/upstream/README.md) (the daily
  fork sync applies to `eclipse-opensovd/*` forks; this tool is
  monitored manually via its GitHub page because it is a personal
  project, not part of that org).

## Build prerequisites

- Rust 1.88+ (workspace uses edition 2024).
- `protoc` (Protocol Buffers compiler) on PATH, or the `PROTOC` env
  variable pointing at a `protoc` executable.  Any recent v27+ works;
  we have been using v28.3 from
  <https://github.com/protocolbuffers/protobuf/releases>.

## Usage from xtask

`cargo run -p xtask -- phase5-yaml-to-mdd` (see
[`opensovd-core/xtask/src/main.rs`](../opensovd-core/xtask/src/main.rs))
shells out to this tool to regenerate Phase 5 MDDs and ODXs from the
YAML sources under
[`opensovd-core/deploy/pi/cda-mdd/src/`](../opensovd-core/deploy/pi/cda-mdd/src/).

## Author and license

- Author: Bartosz Burda (42dot, Hyundai Motor Group).
- License: Apache-2.0 (see `LICENSE`).
- NOTICE: no NOTICE file shipped by upstream; this directory inherits
  the repo-root NOTICE terms for third-party code.
