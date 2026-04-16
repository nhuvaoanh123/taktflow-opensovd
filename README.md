# taktflow-opensovd

Monorepo for [Eclipse OpenSOVD](https://github.com/eclipse-opensovd) development --
an open-source implementation of **ISO 17978 Service-Oriented Vehicle Diagnostics (SOVD)**,
the modern REST/HTTP replacement for UDS/CAN diagnostics in automotive.

Built by the [Taktflow](https://github.com/Taktflow-Systems) SOVD workstream.
Target: full SOVD stack on real hardware, upstreamed to Eclipse by end of 2026.

## Why SOVD

| Dimension | UDS (legacy) | SOVD (modern) |
|-----------|-------------|---------------|
| Transport | CAN + ISO-TP / DoIP | REST/HTTP over IP |
| Data format | Binary byte frames | JSON resources |
| Addressing | Session + service IDs | URL paths (`/sovd/v1/components/{id}/faults`) |
| Security | Seed/key | HTTPS + certificates + OAuth |
| Tooling | Specialized diagnostic tools | Any HTTP client |

Vehicles are moving to IP-based zonal architectures. OEMs need cloud fleet
diagnostics, OTA feedback, and AI/ML fault analysis. SOVD is the ASAM/ISO
standard that makes this possible, and OpenSOVD is the Eclipse reference
implementation -- designated as the diagnostic layer for
[Eclipse S-CORE](https://projects.eclipse.org/projects/automotive.score)
(the SDV reference OS / middleware stack).

## Current status

**Phase 5 -- Hardware-in-the-Loop** (April 2026)

| Component | State |
|-----------|-------|
| SOVD Server (Axum, async) | Running on Raspberry Pi, REST API live |
| SOVD Gateway | Federated host routing, parallel fan-out, TOML config |
| Diagnostic Fault Manager | SQLite persistence, operation-cycle gating, 50ms lock budget |
| Fault ingestion IPC | Unix sockets + postcard wire format (no_std-compatible) |
| Classic Diagnostic Adapter | 68k LoC Rust, DoIP + UDS session management, MDD database |
| CAN-to-DoIP proxy | Bridging physical STM32 ECUs to SOVD stack |
| Embedded UDS (STM32) | FZC SingleFrame F191 round-trip proven live on real hardware |
| OpenAPI contract | Snapshot-locked to ASAM SOVD v1.1, xtask regeneration |

Previous phases delivered: upstream code-style alignment (Phase 0), workspace
scaffolding + CDA integration (Phase 1-2), DFM + diagnostic DB + gateway
routing (Phase 3-4), OpenAPI contract tests, Pi full-stack deploy (Phase 5 D1).

## Architecture

```
                  Off-board UDS Tester
                         |
                         | UDS over DoIP
                         v
                  UDS2SOVD Proxy          Off-board SOVD Client / Cloud
                         |                         |
                         | SOVD REST               | SOVD REST (ISO 17978)
                         v                         v
                   +---------SOVD Gateway----------+
                         |                    |
              +----------+----------+    Remote hosts
              |          |          |    (HTTP fan-out)
        SOVD Server    DFM     Service App
              |       /    \    (fault reset,
              |    SovdDb  FaultSink  flash, ...)
              |   (SQLite)  (Unix IPC)
              |                |
        OpenAPI doc     Fault shim (POSIX / STM32)
                               |
                       Classic Diagnostic
                        Adapter (CDA)
                               |
                          UDS over DoIP
                               |
                        Physical ECU (CAN)
```

## Testing bench

Hardware-in-the-loop bench with physical and virtual ECUs:

```
 +------------------+          +--------------------+
 |  Dev host (Win)  |   SSH    |  Raspberry Pi      |
 |                  +--------->|  (gateway host)    |
 |  3x ST-LINK     |          |                    |
 |  1x XDS110      |          |  sovd-main         |
 |  GS_USB (CAN)   |          |  ecu-sim           |
 +--------+---------+          |  can-to-doip proxy |
          |                    +--------+-----------+
          | Serial                      | can0 (500 kbps)
          v                             v
 +--------+---------+          +--------+-----------+
 | Physical ECUs    |          | CAN bus             |
 |                  +<-------->|                     |
 | CVC  STM32G474RE |          | ISO-TP frames      |
 | FZC  STM32G474RE |          +--------------------+
 | RZC  STM32G474RE |
 | SC   TMS570LC43x |
 +-------------------+
```

| Service | Host | Role |
|---------|------|------|
| sovd-main | Pi | SOVD REST API |
| ecu-sim | Pi | Virtual ECU simulator (POSIX builds of CVC/FZC/RZC) |
| can-to-doip proxy | Pi | Bridges CAN ISO-TP to DoIP for physical ECUs |

**Physical ECUs:** 3x STM32G474RE (CVC, FZC, RZC) + 1x TMS570LC43x (SC),
all on CAN bus at 500 kbps via ISO-TP. Flashed via ST-LINK and XDS110.

**Virtual ECUs:** BCM, ICU, TCU run as POSIX builds on the Pi or in Docker.

**Deployment:** `deploy/pi/phase5-full-stack.sh` cross-compiles for aarch64,
rsyncs to Pi, installs systemd units, and verifies with a health check.

## Repository map

### Core (Taktflow-developed)

| Directory | Language | Lines | Description |
|-----------|----------|-------|-------------|
| `opensovd-core/` | Rust | ~11k | SOVD Server, Gateway, DFM, Diagnostic DB -- 16 workspace crates |
| `classic-diagnostic-adapter/` | Rust | ~68k | SOVD-to-UDS/DoIP bridge for legacy ECUs (upstream fork, 14 crates) |
| `fault-lib/` | Rust | ~600 | Framework-agnostic fault reporting API, `#![forbid(unsafe_code)]` |
| `dlt-tracing-lib/` | Rust | ~1.9k | Rust `tracing` subscriber for COVESA DLT daemon (FFI + safe wrapper) |
| `odx-converter/` | Kotlin | ~4.2k | ODX (.pdx) to MDD binary format converter with plugin API |

### opensovd-core workspace detail

| Crate | Purpose |
|-------|---------|
| `sovd-interfaces` | Trait + type contracts (SovdBackend, FaultSink, OperationCycle). Zero I/O. 53 tests. |
| `sovd-server` | Axum HTTP server, routes to backend impls, OpenAPI generation via utoipa |
| `sovd-gateway` | Federated routing across local + remote SOVD hosts, parallel fan-out |
| `sovd-dfm` | Diagnostic Fault Manager -- holds DB + fault sink + operation cycle |
| `sovd-db-sqlite` | SQLite persistence, WAL journaling, auto-migration |
| `sovd-db-score` | S-CORE key-value backend (Phase 4 placeholder) |
| `fault-sink-unix` | Unix socket / Windows named pipe IPC, postcard wire format |
| `fault-sink-lola` | S-CORE LoLa shared-memory transport (Phase 4 placeholder) |
| `opcycle-taktflow` | In-process operation cycle state machine, tokio watch fan-out |
| `opcycle-score-lifecycle` | S-CORE lifecycle subscriber (Phase 4 placeholder) |
| `sovd-main` | Entry point binary, wires backends from TOML config |
| `sovd-tracing` | Tracing subscriber setup |
| `sovd-client` | HTTP client (Phase 3 skeleton) |
| `xtask` | `cargo xtask openapi-dump [--check]` for OpenAPI YAML regeneration |
| `integration-tests` | End-to-end HIL and contract tests |

### Planned / early

| Directory | Language | Description |
|-----------|----------|-------------|
| `uds2sovd-proxy/` | Rust | UDS/DoIP to SOVD REST proxy -- Cargo.toml scaffolded, no src yet |
| `cpp-bindings/` | C++ | C++ API bindings -- planned, no code yet |

### Reference (read-only)

| Directory | Description |
|-----------|-------------|
| `opensovd/` | Upstream architecture specs, ADRs, MVP roadmap, governance |
| `external/opendbc/` | Community DBC files for CAN signal decoding |
| `external/odxtools/` | Mercedes-Benz ODX data model (Python, MIT) |
| `external/asam-public/` | Freely available ASAM/ISO/AUTOSAR specs including ISO 17978-3 OpenAPI |
| `external/cicd-workflows/` | Eclipse OpenSOVD shared GitHub Actions |

### Documentation

| Path | Description |
|------|-------------|
| `docs/ARCHITECTURE.md` | arc42-format system design and deployment topology |
| `docs/REQUIREMENTS.md` | FR/NFR/SR/SEC/COMP requirements, ASPICE-traceable |
| `docs/adr/` | 18 Architecture Decision Records (ADR-0001 through ADR-0018) |

## Design principles

- **Rust-first.** Async (Tokio), memory-safe, `#![forbid(unsafe_code)]` where possible.
  Edition 2024, Rust 1.88+. Clippy pedantic + deny rules enforced in CI.
- **Trait boundaries, not frameworks.** `sovd-interfaces` defines all contracts
  (SovdBackend, FaultSink, SovdDb, OperationCycle) with zero I/O. Implementations
  are swappable: SQLite or S-CORE KV for persistence, Unix sockets or LoLa
  shared-memory for fault transport, Taktflow or S-CORE lifecycle for operation cycles.
- **Spec-locked API surface.** OpenAPI schema is snapshot-tested against ASAM SOVD v1.1.
  `cargo xtask openapi-dump --check` gates every PR.
- **Build first, contribute later.** No upstream PRs during early phases. When we
  upstream, we upstream finished, tested, working systems.

## Relationship to upstream

This repo consolidates forks of the individual
[eclipse-opensovd](https://github.com/eclipse-opensovd) repositories into a
single monorepo. Each component tracks its upstream and can be split back out
for contribution. OpenSOVD is the designated diagnostic layer for
[Eclipse S-CORE](https://projects.eclipse.org/projects/automotive.score) v1.0
(target: end of 2026).

## License

Apache-2.0. See individual component LICENSE files.
