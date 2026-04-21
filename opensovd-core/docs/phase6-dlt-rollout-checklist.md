<!--
SPDX-License-Identifier: Apache-2.0
SPDX-FileCopyrightText: 2026 The Contributors to Eclipse OpenSOVD (see CONTRIBUTORS)

See the NOTICE file(s) distributed with this work for additional
information regarding copyright ownership.

This program and the accompanying materials are made available under the
terms of the Apache License Version 2.0 which is available at
https://www.apache.org/licenses/LICENSE-2.0
-->

# Phase 6 DLT Rollout Checklist

`P6-02` closes the one-binary DLT spike by routing every current
`opensovd-core` binary through the shared `sovd-tracing` bootstrap.

## Per-binary coverage

| Binary | Status | Shared bootstrap | DLT context | Correlation coverage | Notes |
|---|---|---|---|---|---|
| `sovd-main` | Covered | `sovd-main/src/tracing_setup.rs` -> `sovd-tracing::init` | `SOVD` | Yes - request spans derive from `X-Request-Id` / `traceparent` via `sovd_server::correlation::resolve_correlation_id()` | `TraceLayer` is active when DLT or OTLP is enabled so the request path emits span-backed events in both modes. |
| `ws-bridge` | Covered | `crates/ws-bridge/src/main.rs` -> `Config::tracing_config()` -> `sovd-tracing::init` | `WSBR` | Not applicable - no SOVD correlation middleware on this observer relay | Request spans log only the path, never the `/ws?token=...` query string. |
| `xtask` | Covered | `xtask/src/main.rs` -> `init_tracing()` -> `sovd-tracing::init` | `XTSK` | Not applicable - developer task runner, not HTTP ingress | DLT stays opt-in via `XTASK_DLT_ENABLED` and the forwarded `dlt-tracing` Cargo feature. |

## Operator notes

- Default builds keep DLT disabled; use each binary's `dlt-tracing` Cargo feature plus the documented env or config switch to enable the DLT sink.
- `sovd-main` keeps OTLP support in the shared bootstrap, so P6-03 can extend the same request spans instead of adding a second tracing stack.
- `ws-bridge` deployment assets now surface `WS_BRIDGE_DLT_ENABLED`, `WS_BRIDGE_DLT_APP_ID`, and `WS_BRIDGE_DLT_APP_DESCRIPTION` alongside `RUST_LOG`.

## Verification status on the Windows dev host

- `cargo check -p sovd-tracing -p sovd-main -p ws-bridge -p xtask` passes.
- `cargo test -p sovd-main -p ws-bridge -p xtask` passes.
- `cargo check -p sovd-main -p ws-bridge -p xtask --features dlt-tracing` is blocked on missing native DLT headers on Windows: `dlt/dlt.h` not found while building `dlt-sys`.
