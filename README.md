# taktflow-opensovd

Taktflow's monorepo for [Eclipse OpenSOVD](https://github.com/eclipse-opensovd) --
an open-source implementation of **ISO 17978 Service-Oriented Vehicle Diagnostics (SOVD)**.

## Goal

Run a full OpenSOVD stack on Taktflow hardware by end of 2026, then upstream
the work to Eclipse. SOVD replaces raw UDS/CAN with REST/HTTP diagnostics --
every Taktflow ECU becomes reachable via `curl`.

| Dimension | UDS (legacy) | SOVD (modern) |
|-----------|-------------|---------------|
| Transport | CAN + ISO-TP / DoIP | REST/HTTP over IP |
| Data format | Binary byte frames | JSON resources |
| Security | Seed/key | HTTPS + certificates |
| Tooling | Specialized diag tools | Any HTTP client |

## Current status

**Phase 5 -- Hardware-in-the-Loop testing** (April 2026)

- **Line A** (Rust / opensovd-core): SOVD server + gateway running on Raspberry Pi,
  integration tests passing, Pi full-stack deploy complete
- **Line B** (Embedded firmware): UDS stack on STM32, FZC SingleFrame F191
  round-trip proven live on real hardware, multi-frame TX follow-up in progress

Previous phases delivered: upstream code-style alignment, workspace scaffolding,
CDA integration, DFM + diagnostic DB, OpenAPI contract tests, gateway routing.

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
                                   |
             +-----------+---------+---------+-----------+
             |           |                   |           |
       SOVD Server   Service App       Classic Diag   SOVD Client
             |      (fault reset,      Adapter (CDA)
             |       flash, ...)            |
             v                              | UDS over DoIP
       Diagnostic DB                        v
       (FlatBuffers)                   Physical ECU (CAN)
```

## Repository map

| Directory | Language | Description |
|-----------|----------|-------------|
| `opensovd-core/` | Rust | SOVD Server, Gateway, DFM, Diagnostic DB, Client -- the main codebase |
| `classic-diagnostic-adapter/` | Rust | SOVD -> UDS/DoIP bridge for legacy ECUs |
| `uds2sovd-proxy/` | Rust | UDS/DoIP -> SOVD REST proxy for legacy testers |
| `fault-lib/` | Rust | Framework-agnostic fault reporting API (S-CORE boundary) |
| `dlt-tracing-lib/` | Rust | Rust `tracing` subscriber for COVESA DLT daemon |
| `cpp-bindings/` | C++ | C++ API bindings for SOVD Server and Fault Manager |
| `odx-converter/` | Kotlin | ODX (.pdx) -> MDD binary format converter |
| `opensovd/` | Markdown | Architecture specs, ADRs, MVP roadmap, governance |
| `external/` | Mixed | Third-party references (opendbc, ASAM public docs, CI templates) |
| `docs/` | Markdown | Working plans, prompts, progress tracking |
| `scripts/` | Shell | Automation and deployment scripts |

## Relationship to upstream

This repo consolidates forks of the individual
[eclipse-opensovd](https://github.com/eclipse-opensovd) repositories into a
single monorepo for development velocity. The strategy is **build first,
contribute later** -- no upstream PRs until we have a working end-to-end stack.

## License

Individual components retain their upstream licenses (Apache-2.0).
