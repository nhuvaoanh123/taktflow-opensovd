# taktflow-opensovd

Open-source **SOVD diagnostic stack** implementing the ASAM SOVD v1.1
OpenAPI (ISO 17978-3) -- from REST API to physical ECU, tested on real
automotive hardware.

Built by [Taktflow](https://github.com/Taktflow-Systems). Targeting upstream
contribution to [Eclipse OpenSOVD](https://github.com/eclipse-opensovd) and
integration with [Eclipse S-CORE](https://projects.eclipse.org/projects/automotive.score).

## Goal

Replace legacy UDS/CAN diagnostics with modern REST/HTTP for **multi-ECU
zonal architectures**. Every ECU -- regardless of role or zone -- becomes
reachable via standard HTTP tooling (`curl`, Postman, cloud fleet APIs)
instead of proprietary diagnostic hardware and binary protocols.

Taktflow's HIL bench uses a minimal 3-ECU zonal setup (CVC central + SC
safety + BCM virtual body) as the reference test integration (see ADR-0023).
The stack itself is architecture-agnostic within automotive diagnostics and
scales to arbitrary ECU counts without code change.

| Dimension | UDS (legacy) | SOVD (modern) |
|-----------|-------------|---------------|
| Transport | CAN + ISO-TP / DoIP | REST/HTTP over IP |
| Data format | Binary byte frames | JSON resources |
| Addressing | Session + service IDs | URL paths (`/sovd/v1/components/{id}/faults`) |
| Security | Seed/key | HTTPS + certificates + OAuth |
| Tooling | Specialized diagnostic tools | Any HTTP client |

## Scope

**In scope:**

- SOVD Server -- REST API implementing the ASAM SOVD v1.1 OpenAPI
  (ISO 17978-3), async Rust (Tokio + Axum)
- SOVD Gateway -- federated routing across local and remote diagnostic hosts
- Diagnostic Fault Manager (DFM) -- fault ingestion, persistence, operation-cycle gating
- Classic Diagnostic Adapter (CDA) -- SOVD-to-UDS/DoIP bridge for legacy ECUs
- Fault ingestion IPC -- Unix sockets / Windows named pipes, no_std-compatible wire format
- ODX-to-MDD converter -- diagnostic database format tooling
- Hardware-in-the-loop test bench -- 3 ECUs (CVC STM32G4 + SC TMS570 + BCM POSIX virtual), ADR-0023

**Out of scope:**

- Safety-relevant functionality (handled by S-CORE, ASIL-B). OpenSOVD is QM.
- Embedded RTOS or base software. Firmware lives in a separate repository.
- Production deployment tooling. This is the diagnostic stack, not the vehicle OS.

## Design principles

- **Rust-first.** Async (Tokio), memory-safe, `#![forbid(unsafe_code)]` where
  possible. Edition 2024, Rust 1.88+. Clippy pedantic + deny rules enforced in CI.
- **Trait boundaries, not frameworks.** `sovd-interfaces` defines all contracts
  (SovdBackend, FaultSink, SovdDb, OperationCycle) with zero I/O. Implementations
  are swappable: SQLite or S-CORE KV for persistence, Unix sockets or LoLa
  shared-memory for fault transport, Taktflow or S-CORE lifecycle for operation cycles.
- **Spec-locked API surface.** OpenAPI schema is snapshot-tested against ASAM
  SOVD v1.1 (ISO 17978-3).
  `cargo xtask openapi-dump --check` gates every PR.
- **Build first, contribute later.** No upstream PRs during early phases. When we
  upstream, we upstream finished, tested, working systems.

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
| Embedded UDS (STM32) | CVC SingleFrame F191 round-trip proven live on real hardware |
| OpenAPI contract | Snapshot-locked to ASAM SOVD v1.1 OpenAPI (ISO 17978-3), xtask regeneration |

Previous phases delivered: upstream code-style alignment (Phase 0), workspace
scaffolding + CDA integration (Phase 1-2), DFM + diagnostic DB + gateway
routing (Phase 3-4), OpenAPI contract tests, Pi full-stack deploy (Phase 5 D1).

## Testing

| Layer | What | Count |
|-------|------|-------|
| Unit + async | `#[test]` + `#[tokio::test]` across all Rust crates | 5,680 |
| Snapshot | `insta` schema snapshots (sovd-interfaces, locked to ASAM SOVD v1.1 OpenAPI / ISO 17978-3) | 36 files |
| OpenAPI contract | Schema regeneration gate (`cargo xtask openapi-dump --check`) | per PR |
| Integration | End-to-end flows: in-memory MVP, CDA+ECU-sim, DFM SQLite roundtrip, gateway routing | 25 test files |
| HIL | Live CAN captures on physical STM32 bench (vcan0 smoke, real CAN, proxy) | 3 capture logs |
| CI enforcement | clippy pedantic + deny-warnings, rustfmt, cargo-deny (license + advisory audit) | every push |

CI runs on Linux and Windows, stable (1.88.0) and nightly toolchains, with a
feature matrix covering all-features, minimal, and mbedtls-only configurations.

## Architecture

```mermaid
graph TB
    UDS["Off-board UDS Tester"]
    SOVD_CLI["Off-board SOVD Client / Cloud"]

    UDS -->|"UDS over DoIP"| PROXY["UDS2SOVD Proxy"]
    PROXY -->|"SOVD REST"| GW
    SOVD_CLI -->|"SOVD REST<br/>(ASAM v1.1 / ISO 17978-3)"| GW

    subgraph STACK ["SOVD Gateway"]
        GW["<b>sovd-gateway</b>"]
        GW --> SRV["<b>SOVD Server</b>"]
        GW --> DFM["<b>DFM</b>"]
        GW --> APP["Service App<br/>(fault reset, flash)"]
        GW -->|"HTTP fan-out"| REMOTE["Remote hosts"]
        DFM --> DB[("SovdDb<br/>SQLite")]
        DFM --> FS["FaultSink<br/>(Unix IPC)"]
        SRV --> OA["OpenAPI doc"]
    end

    FS --> SHIM["Fault shim<br/>(POSIX / STM32)"]
    SHIM --> CDA["<b>Classic Diagnostic<br/>Adapter (CDA)</b>"]
    CDA -->|"UDS over DoIP"| ECU["<b>Physical ECU</b><br/>(CAN bus)"]

    style STACK fill:#e8f4fd,stroke:#1a73e8,stroke-width:2px
    style GW fill:#bbdefb,stroke:#1565c0
    style SRV fill:#bbdefb,stroke:#1565c0
    style DFM fill:#bbdefb,stroke:#1565c0
    style DB fill:#c8e6c9,stroke:#2e7d32
    style CDA fill:#ffe0b2,stroke:#e65100
    style ECU fill:#fce4ec,stroke:#c62828
```

## Testing bench

Hardware-in-the-loop bench with physical and virtual ECUs:

```mermaid
graph LR
    subgraph DEV ["Dev Host (Windows)"]
        STL["3x ST-LINK"]
        XDS["1x XDS110"]
        GS["GS_USB (CAN)"]
    end

    subgraph PI ["Raspberry Pi (gateway host)"]
        SM["sovd-main"]
        SIM["ecu-sim"]
        PX["can-to-doip proxy"]
    end

    subgraph HW ["Physical ECUs (2)"]
        CVC["CVC STM32G474RE<br/>(central)"]
        SC["SC TMS570LC43x<br/>(safety)"]
    end

    subgraph VIRT ["Virtual ECU (1)"]
        BCM["BCM (POSIX)<br/>on Pi or Docker"]
    end

    DEV -->|"SSH"| PI
    DEV -->|"ST-LINK / XDS110<br/>(flash)"| HW
    PI -->|"can0 (500 kbps)<br/>ISO-TP frames"| HW
    PI -->|"DoIP / TCP"| VIRT

    style DEV fill:#fff3e0,stroke:#e65100,stroke-width:2px
    style PI fill:#e8f4fd,stroke:#1a73e8,stroke-width:2px
    style HW fill:#fce4ec,stroke:#c62828,stroke-width:2px
    style VIRT fill:#e8f5e9,stroke:#2e7d32,stroke-width:2px
```

| Service | Host | Role |
|---------|------|------|
| sovd-main | Pi | SOVD REST API |
| ecu-sim | Pi | Virtual ECU simulator (POSIX build of BCM) |
| can-to-doip proxy | Pi | Bridges CAN ISO-TP to DoIP for physical ECUs |

**3-ECU bench (ADR-0023):**

- **CVC** — STM32G474RE, central vehicle controller, ST-LINK flashing
- **SC** — TMS570LC43x, safety controller, XDS110 flashing (different vendor,
  proves no accidental ST-lock-in)
- **BCM** — POSIX virtual, DoIP-direct path (no proxy)

The 3-ECU set covers every architectural code path in the stack. The stack
itself is not hardcoded to this count — additional ECUs can be added via
config without code change.

**Deployment:** `deploy/pi/phase5-full-stack.sh` cross-compiles for aarch64,
rsyncs to Pi, installs systemd units, and verifies with a health check.

## Repository map

### Core (~86k LoC Rust, ~4.2k LoC Kotlin)

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
| `sovd-interfaces` | Trait + type contracts (SovdBackend, FaultSink, OperationCycle). Zero I/O. |
| `sovd-server` | Axum HTTP server, routes to backend impls, OpenAPI generation via utoipa |
| `sovd-gateway` | Federated routing across local + remote SOVD hosts, parallel fan-out |
| `sovd-dfm` | Diagnostic Fault Manager -- holds DB + fault sink + operation cycle |
| `sovd-db-sqlite` | SQLite persistence, WAL journaling, auto-migration |
| `sovd-db-score` | S-CORE key-value backend (placeholder) |
| `fault-sink-unix` | Unix socket / Windows named pipe IPC, postcard wire format |
| `fault-sink-lola` | S-CORE LoLa shared-memory transport (placeholder) |
| `opcycle-taktflow` | In-process operation cycle state machine, tokio watch fan-out |
| `opcycle-score-lifecycle` | S-CORE lifecycle subscriber (placeholder) |
| `sovd-main` | Entry point binary, wires backends from TOML config |
| `sovd-client` | HTTP client (skeleton) |
| `xtask` | `cargo xtask openapi-dump [--check]` for OpenAPI YAML regeneration |
| `integration-tests` | End-to-end HIL and contract tests |

### Planned

| Directory | Language | Description |
|-----------|----------|-------------|
| `uds2sovd-proxy/` | Rust | UDS/DoIP to SOVD REST proxy -- scaffolded, implementation pending |
| `cpp-bindings/` | C++ | C++ API bindings -- planned |

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
| `docs/SYSTEM-SPECIFICATION.md` | **Single-file consolidated spec: architecture + requirements + safety + API + state machines** |
| `docs/ARCHITECTURE.md` | arc42-format system design and deployment topology |
| `docs/REQUIREMENTS.md` | FR/NFR/SR/SEC/COMP requirements, ASPICE-traceable |
| `docs/TRADE-STUDIES.md` | 18 trade studies: every major technical decision with options, criteria, rationale |
| `docs/SAFETY-CONCEPT.md` | Safety classification, QM/ASIL boundary, Fault Library isolation |
| `docs/TEST-STRATEGY.md` | Test levels, CI pipeline, HIL gating, coverage tooling |
| `docs/CODING-STANDARDS.md` | Rust/Kotlin formatting, linting, error handling, naming, SPDX |
| `docs/DEVELOPER-GUIDE.md` | Build prerequisites, toolchain setup, run and test instructions |
| `docs/DEPLOYMENT-GUIDE.md` | SIL / HIL / production topology, configuration, rollback |
| `docs/GLOSSARY.md` | Domain terms: SOVD, UDS, DTC, DoIP, ASIL, DFM, and more |
| `docs/adr/` | 23 Architecture Decision Records (ADR-0001 through ADR-0023) |
| `.github/CONTRIBUTING.md` | How to contribute, PR process, commit conventions |
| `.github/CODE_OF_CONDUCT.md` | Eclipse Community Code of Conduct |
| `.github/CHANGELOG.md` | Release history by phase |
| `SYSTEM-SPECIFICATION.html` | Single-file visual spec (opens in any browser) |

## Relationship to upstream

This repo consolidates forks of the individual
[eclipse-opensovd](https://github.com/eclipse-opensovd) repositories into a
single monorepo. Each component tracks its upstream and can be split back out
for contribution. OpenSOVD is the designated diagnostic layer for
[Eclipse S-CORE](https://projects.eclipse.org/projects/automotive.score) v1.0
(target: end of 2026).

## License

Apache-2.0. See individual component LICENSE files.
