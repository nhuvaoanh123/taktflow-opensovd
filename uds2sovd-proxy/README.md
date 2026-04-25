<!--
SPDX-License-Identifier: Apache-2.0
SPDX-FileCopyrightText: 2026 The Contributors to Eclipse OpenSOVD (see CONTRIBUTORS)

See the NOTICE file(s) distributed with this work for additional
information regarding copyright ownership.

This program and the accompanying materials are made available under the
terms of the Apache License Version 2.0 which is available at
https://www.apache.org/licenses/LICENSE-2.0
-->

# UDS-to-SOVD Proxy

`uds2sovd-proxy` is the first-cut UDS-over-DoIP ingress bridge for Taktflow.
It accepts tester requests on the north face, resolves the addressed
diagnostic service from runtime `.mdd` data, invokes the matching southbound
SOVD REST call, and encodes the result back into a UDS reply.

The authoritative design scope for this crate is
[`docs/adr/ADR-0040-uds2sovd-proxy-design.md`](../docs/adr/ADR-0040-uds2sovd-proxy-design.md).

## Current scope

Supported in the first cut:

- `0x22 ReadDataByIdentifier`
- `0x31 RoutineControl` start and results subfunctions
- `0x19 ReadDTCInformation` status-mask count and list subsets
- `0x14 ClearDiagnosticInformation` for all-DTC clear

Explicitly denied in the first cut:

- `0x2E WriteDataByIdentifier`
- `0x10 DiagnosticSessionControl`
- `0x27 SecurityAccess`
- `0x29 Authentication`
- `0x31 0x02` routine stop

## Layout

- `src/config.rs`: TOML configuration model and loader
- `src/proxy.rs`: DoIP listener, request dispatch, UDS reply encoding
- `src/mdd.rs`: runtime `.mdd` loading and service resolution
- `src/sovd.rs`: southbound SOVD HTTP client
- `src/uds.rs`: UDS parser and helper encoders
- `src/tracing_setup.rs`: tracing bootstrap aligned with `sovd-tracing`

## Example config

A checked-in example lives at
[`uds2sovd-proxy.example.toml`](./uds2sovd-proxy.example.toml).

Run the proxy with:

```shell
cargo run --manifest-path uds2sovd-proxy/Cargo.toml -- --config-file uds2sovd-proxy/uds2sovd-proxy.example.toml
```

## Verification

Run the local crate checks with:

```shell
cargo fmt --manifest-path uds2sovd-proxy/Cargo.toml
cargo test --manifest-path uds2sovd-proxy/Cargo.toml
cargo check --manifest-path uds2sovd-proxy/Cargo.toml
```

## Development

Project-wide checks can still be run with:

```shell
uv run https://raw.githubusercontent.com/eclipse-opensovd/cicd-workflows/main/run_checks.py
```
