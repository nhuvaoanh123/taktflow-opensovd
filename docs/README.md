<!--
SPDX-FileCopyrightText: 2026 The Contributors to Eclipse OpenSOVD (Taktflow fork)
SPDX-License-Identifier: Apache-2.0
-->

# Taktflow Eclipse OpenSOVD — Documentation Index

This is the documentation index for the Taktflow fork of Eclipse OpenSOVD.
Use this file to locate the right document for the question you are asking.

All paths are relative to the repository root
`H:\taktflow-opensovd\` unless otherwise noted.

## Start here

| Doc | Purpose | Read when |
|-----|---------|-----------|
| **`docs/SYSTEM-SPECIFICATION.md`** | **Single-file consolidated spec: architecture, requirements, safety, API, state machines, test strategy — all in one document with Mermaid diagrams** | **You want the complete picture in one read** |

## Project-level docs

| Doc | Purpose | Read when |
|-----|---------|-----------|
| `MASTER-PLAN.md` | The governing end-to-end plan: goals (§A/B), principles (§C), gap analysis, phased delivery, risks, timeline | You need the big picture, a phase deadline, or the governing principle for a decision |
| `docs/DEVELOPMENT-STORY.md` | Narrative bridge from use case -> requirement -> architecture -> ADR -> implementation -> verification -> phase gate | You want the professional engineering story in one read |
| `docs/REQUIREMENTS.md` | Formal numbered requirements (FR / NFR / SR / SEC / COMP), traceable, ASPICE-compatible | You need to verify what the system must do, or design a test that traces back to a stable ID |
| `docs/ARCHITECTURE.md` | arc42-format project-level architecture description | You need the component topology, runtime views, deployment views, or a cross-cutting concept |
| `docs/integration/README.md` | Final integrator guide: authority hosts, checked-in configs, auth profile selection, deployment proofs | You need to stand up the stack without tribal knowledge |
| `docs/deploy/pilot-oem/README.md` | Pilot OEM deployment playbook with evidence slots and OEM-owned value register | You are preparing a first OEM pilot deployment |
| `docs/integration/repair-shop.md` | Workshop / repair-shop operational guide aligned to UC1..UC5 | You need the mechanic-facing flow for faults, data, and routines |
| `docs/examples/` | Happy-path walkthroughs for OTA, predictive maintenance, and repair-shop sessions | You want one realistic scenario end to end |
| `docs/traceability/matrix.md` | Requirement -> design -> implementation -> verification matrix for the current repo | You need the Phase 11 manual traceability view |
| `docs/adr/` | Architecture Decision Records (in the upstream-ready shape) | You need the rationale for a specific decision |
| `docs/architecture/score-alignment-decisions.md` | Phase 10 memo for the monolith-over-IPC-peers S-CORE alignment decision | You need the OEM rationale for keeping Config/Auth/Crypto inline |
| `docs/ecosystem/` | Phase 10 ecosystem-alignment reviews (VSS drift, ML boundary) | You need the internal compatibility and standards-drift notes behind P10 |
| `work/TASKS.md` *(gitignored)* | Task breakdown (tactical, week-level) | You need the week's work list |
| `work/WORKING-LINES.md` *(gitignored)* | Parallel working lines | You need to know which parallel effort you are on |
| `README.md` | Workspace top-level readme | Orientation |

## ADR index (live as of Rev 1.4)

| ID | Title | Status | File |
|----|-------|--------|------|
| ADR-0001 | Taktflow-SOVD integration | Accepted | `adr/0001-taktflow-sovd-integration.md` |
| ADR-0002 | Fault Library as C shim on embedded, Rust on Pi | Accepted | `adr/0002-fault-library-c-shim-embedded-rust-pi.md` |
| ADR-0003 | SQLite for DFM persistence (sqlx + WAL) | Accepted | `adr/0003-sqlite-for-dfm-persistence.md` |
| ADR-0004 | CAN-to-DoIP proxy on Raspberry Pi for physical ECUs | Accepted | `adr/0004-can-to-doip-proxy-on-raspberry-pi.md` |
| ADR-0005 | Virtual ECUs speak DoIP directly (no proxy for POSIX builds) | Accepted | `adr/0005-virtual-ecus-speak-doip-directly.md` |
| ADR-0006 | Fork + track upstream + extras-on-top model | Accepted | `adr/0006-fork-track-upstream-extras-on-top.md` |
| ADR-0007 | Build-first contribute-later (archived 2026-04-20 — upstream contribution dropped from scope) | Archived | `adr/archive/0007-build-first-contribute-later.md` |
| ADR-0008 | Community-written ODX XSD as default, ASAM as pluggable drop-in | Accepted | `adr/0008-odx-community-xsd-default.md` |
| ADR-0009 | Authentication — both OAuth2/OIDC and mTLS client certificates | Accepted | `adr/0009-auth-both-oauth2-and-cert.md` |
| ADR-0010 | DoIP discovery — both broadcast and static configuration | Accepted | `adr/0010-doip-discovery-both-broadcast-and-static.md` |
| ADR-0011 | Physical DoIP on STM32 — both lwIP and ThreadX NetX (deferred) | Accepted | `adr/0011-physical-doip-both-lwip-and-netx.md` |
| ADR-0012 | DFM operation-cycle API — both tester-driven and ECU-driven | Accepted | `adr/0012-operation-cycle-both-tester-and-ecu-driven.md` |
| ADR-0013 | Correlation ID — accept both `X-Request-Id` and `traceparent` | Accepted | `adr/0013-correlation-id-accept-both-headers.md` |
| ADR-0014 | Audit log sink — all three: SQLite + append-only file + DLT | Accepted | `adr/0014-audit-log-sink-all-three.md` |
| ADR-0015 | sovd-interfaces layering: `spec/` + `extras/` + `types/` | Accepted | `adr/0015-sovd-interfaces-spec-extras-types-layering.md` |
| ADR-0016 | Pluggable S-CORE backends behind standalone defaults | Accepted | `adr/0016-pluggable-score-backends.md` |
| ADR-0017 | FaultSink wire protocol — postcard + WireFaultRecord shadow | Accepted | `adr/0017-faultsink-wire-protocol-postcard-shadow.md` |
| ADR-0018 | Never hard fail — log-and-continue for backend impls | Accepted | `adr/0018-never-hard-fail-in-backends.md` |
| ADR-0019 | SOVD session model derived from UDS modes | Accepted | `adr/0019-sovd-session-model-from-uds.md` |
| ADR-0020 | SOVD wire errors follow the Part 3 OpenAPI envelopes | Accepted | `adr/0020-sovd-wire-errors-from-part3-openapi.md` |
| ADR-0021 | Taktflow MVP subset is a local conformance class | Accepted | `adr/0021-taktflow-mvp-subset-as-local-conformance-class.md` |
| ADR-0022 | Lock lifecycle defaults to TTL, refresh, and auto-release | Accepted | `adr/0022-lock-lifecycle-ttl-refresh-expiry.md` |
| ADR-0023 | Reduce HIL/SIL test bench from 7 ECUs to 3 ECUs (CVC + SC + BCM) | Accepted | `adr/0023-reduce-bench-to-3-ecus.md` |
| ADR-0024 | Reuse embedded-production cloud connector + SvelteKit capability-showcase dashboard | Accepted | `adr/0024-reuse-embedded-production-cloud-connector.md` |
| ADR-0025 | Pull OTA firmware update into scope (STM32/CVC first, reuse signing) | Accepted | `adr/0025-ota-firmware-update-scope.md` |
| ADR-0038 | Pluggable backend compatibility interface | Accepted | `adr/ADR-0038-pluggable-backend-compatibility-interface.md` |
| ADR-0039 | ISO 17978 conformance subset for Phase 11 | Accepted | `adr/ADR-0039-iso-17978-conformance-subset.md` |

Upstream ADRs referenced by this project:

- `opensovd/docs/design/adr/001-adr-score-interface.md` — S-CORE <-> OpenSOVD
  interface (Fault Library is the boundary). Binding on us.

## Crate-level docs (narrower scope)

| Doc | Purpose |
|-----|---------|
| `opensovd-core/ARCHITECTURE.md` | Crate-level role boundaries and trait contracts (narrower than `docs/ARCHITECTURE.md`) |
| `opensovd-core/CODESTYLE.md` | Rust style conventions (mirrors upstream CDA) |
| `opensovd-core/CONTRIBUTING.md` | Contribution rules (local, pre-upstreaming) |
| `opensovd-core/docs/sync-diff-*.md` | Upstream sync tracking (auto-generated) |
| `opensovd-core/docs/tdd-punchlist-*.md` | Punchlist mirror from failing upstream CI (if present) |

## Upstream reference (read-only mirror)

| Doc | Purpose |
|-----|---------|
| `opensovd/docs/design/design.md` | Upstream design document — role definitions are verbatim source of truth |
| `opensovd/docs/design/mvp.md` | Upstream MVP scope — use-cases and requirements seed |
| `opensovd/docs/design/adr/001-adr-score-interface.md` | ADR-SCORE |

## Reading order for new contributors

1. `MASTER-PLAN.md` §A, §B, §C (charter, why, principles)
2. `docs/REQUIREMENTS.md` §1, §3 (what must be done)
3. `docs/ARCHITECTURE.md` §1, §5, §6 (how it is done)
4. `opensovd-core/ARCHITECTURE.md` (crate internals)
5. `opensovd/docs/design/design.md` (upstream terms)

## Revision

This index is maintained alongside the other docs. When you add a new doc
under `docs/` or a new ADR, update this file in the same commit.

Rev 1.6 - 2026-04-23 - Indexed the Phase 11 integrator, repair-shop, example,
and traceability documents alongside ADR-0039.
