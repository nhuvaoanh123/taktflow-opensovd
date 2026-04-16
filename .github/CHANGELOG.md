# Changelog

All notable changes to taktflow-opensovd are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added
- Monorepo consolidation: merged 9 upstream forks into single repository.
- Project README with goal, scope, design principles, architecture, testing
  bench, and repository map.
- Professional documentation suite: developer guide, coding standards, test
  strategy, safety concept, deployment guide, glossary.
- LICENSE, CONTRIBUTING.md, CODE_OF_CONDUCT.md.

## Phase 5 -- Hardware-in-the-Loop (April 2026)

### Added
- Pi full-stack deploy script (`deploy/pi/phase5-full-stack.sh`).
- Systemd service units for sovd-main, ecu-sim, CAN-to-DoIP proxy.
- 8 HIL integration tests (fault read/clear, routine execution, CAN bus-off,
  concurrent testers, large fault list, error handling).
- Live CAN capture logs from physical STM32 bench.

### Changed
- sovd-main now selects backends at runtime from TOML configuration.

## Phase 4 -- Gateway and Real Backends (April 2026)

### Added
- SOVD Gateway with federated host routing and parallel fan-out.
- SQLite persistence backend (sovd-db-sqlite) with WAL journaling.
- S-CORE backend placeholders (sovd-db-score, fault-sink-lola,
  opcycle-score-lifecycle).
- OpenAPI contract gate (`cargo xtask openapi-dump --check`).
- Integration tests for real CDA + SQLite backend flows.
- ADR-0009 through ADR-0018.

## Phase 3 -- DFM and Fault Pipeline (April 2026)

### Added
- Diagnostic Fault Manager (sovd-dfm) with operation-cycle gating.
- Fault ingestion via Unix socket IPC (fault-sink-unix).
- Taktflow operation cycle state machine (opcycle-taktflow).
- DFM SQLite roundtrip integration tests.
- 36 OpenAPI schema snapshot files locked to ASAM SOVD v1.1.

## Phase 2 -- CDA Integration (April 2026)

### Added
- Classic Diagnostic Adapter fork (68k LoC Rust) integrated into workspace.
- CDA + ECU simulator smoke tests.
- Phase 2 HIL capture logs (vcan0 and live CAN).
- SIL deployment script (`deploy/sil/run-cda-local.sh`).

## Phase 1 -- Workspace Scaffolding (April 2026)

### Added
- opensovd-core Rust workspace with 16 crates.
- sovd-interfaces: trait contracts (SovdBackend, FaultSink, SovdDb,
  OperationCycle) with zero I/O.
- sovd-server: Axum HTTP server with OpenAPI generation via utoipa.
- InMemoryServer for MVP testing.
- CI workflows: build, PR checks, pre-commit, documentation generation.

## Phase 0 -- Upstream Alignment (April 2026)

### Added
- Forked all 8 eclipse-opensovd repositories.
- Matched upstream code style: rustfmt, clippy, cargo-deny, SPDX headers.
- rust-toolchain.toml pinned to 1.88.0.
- ADR-0001 through ADR-0008.
- Architecture document (arc42 format).
- Requirements specification (ASPICE-traceable).
