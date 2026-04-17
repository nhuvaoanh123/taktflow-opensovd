<!--
SPDX-FileCopyrightText: 2026 The Contributors to Eclipse OpenSOVD (Taktflow fork)
SPDX-License-Identifier: Apache-2.0
-->

# Taktflow Eclipse OpenSOVD -- Development Story

- Document ID: TAKTFLOW-SOVD-STORY
- Revision: 1.0
- Status: Draft
- Date: 2026-04-17
- Owner: Taktflow SOVD workstream

## 1. Purpose

This document explains the professional engineering story of the repo:

- why the use cases exist
- how use cases become formal requirements
- how the architecture realizes those requirements
- how ADRs constrain the design
- how code is split across crates and runtime components
- how tests and phase gates prove the system

It complements, but does not replace:

- [USE-CASES.md](USE-CASES.md) for the canonical capability catalog
- [REQUIREMENTS.md](REQUIREMENTS.md) for numbered contractual requirements
- [ARCHITECTURE.md](ARCHITECTURE.md) for arc42 design and runtime views
- [SYSTEM-SPECIFICATION.md](SYSTEM-SPECIFICATION.md) for the single-file summary
- [MASTER-PLAN.md](../MASTER-PLAN.md) for delivery phases, gates, and risks

## 2. The Professional Story in One Line

OpenSOVD is not being built as "some Rust services with tests." It is being
built as a professional diagnostic product line:

`stakeholder need -> use case -> requirement -> architecture -> ADR -> implementation -> verification -> phase exit -> release evidence`

That chain is the development story.

## 3. Starting Point: Why the Product Exists

The project starts from a business and engineering problem:

- legacy diagnostics are expressed as UDS services over CAN / ISO-TP / DoIP
- those flows are difficult to consume from modern tools, cloud workflows, and
  multi-ECU zonal platforms
- Taktflow wants the same underlying ECU diagnostics exposed through ASAM SOVD
  v1.1 OpenAPI / ISO 17978-3 without breaking the existing safety lifecycle

That problem statement is visible in:

- [README.md](../README.md)
- [MASTER-PLAN.md](../MASTER-PLAN.md)
- [SYSTEM-SPECIFICATION.md](SYSTEM-SPECIFICATION.md)

So the project is not "invent an API." The real problem is:

1. keep legacy UDS and existing firmware behavior intact
2. add a modern REST diagnostic surface
3. preserve safety boundaries and upstream contribution discipline

## 4. The Development Flow

### 4.1 Use cases come first

Use cases are the customer-visible stories. In this repo they are the stable
answer to "what is this stack supposed to let a tester, observer, or ECU do?"

The five MVP use cases are:

1. UC1 -- read DTCs via SOVD
2. UC2 -- report a fault via Fault API
3. UC3 -- clear DTCs
4. UC4 -- reach a UDS ECU through CDA
5. UC5 -- trigger a diagnostic routine

These are the top-level operational stories. They define the system from the
outside in.

### 4.2 Requirements formalize the use cases

Once a use case is accepted, the next step is to turn it into formal,
testable, reviewable requirements.

That is why [REQUIREMENTS.md](REQUIREMENTS.md) is organized as:

- `FR-*` for what the product must do
- `NFR-*` for timing, scale, availability, portability, and observability
- `SR-*` for safety-preservation constraints
- `SEC-*` for transport, auth, audit, and abuse resistance
- `COMP-*` for standards and process obligations

In other words:

- the use case says what the user wants
- the requirement says exactly what the system shall do and how we will know
  it is correct

### 4.3 Architecture is the realization strategy

Architecture is not a restatement of requirements. It is the technical answer
to "which runtime pieces do we need, and how do they collaborate, so that the
requirements become true?"

That is why [ARCHITECTURE.md](ARCHITECTURE.md) contains:

- the context and boundary model
- the building blocks (`sovd-server`, `sovd-gateway`, `sovd-dfm`, CDA,
  CAN-to-DoIP proxy)
- the runtime views for UC1-UC5
- safety and security cross-cutting concepts
- deployment views for SIL, HIL, and production

### 4.4 ADRs stop architecture drift

A professional system also needs controlled design decisions.

ADRs answer questions like:

- why the Fault Library is a C shim on embedded and Rust on Pi
- why SQLite is used for DFM persistence
- why physical ECUs are reached through a Pi CAN-to-DoIP proxy
- why auth is both mTLS and OIDC instead of one or the other

Without ADRs, architecture becomes opinion. With ADRs, architecture becomes a
traceable decision record.

### 4.5 Implementation follows the architecture seams

Code is then organized to match the architecture:

- `sovd-interfaces` owns pure contracts and types
- `sovd-server` owns the SOVD REST surface
- `sovd-gateway` owns routing and fan-out
- `sovd-dfm` owns fault ingest, debounce, operation-cycle gating, and DTC state
- `sovd-db-sqlite` owns persistence
- `fault-sink-*` owns transport from firmware-side fault reports into DFM
- CDA owns UDS / DoIP adaptation for legacy ECUs
- the Pi proxy owns CAN ISO-TP bridging for physical CAN-only ECUs

This is the critical professional move: implementation boundaries follow the
architecture, not random convenience.

### 4.6 Verification closes the loop

The final step is proving the chain:

- unit tests prove local logic
- integration tests prove crate collaboration
- SIL proves end-to-end behavior in a controlled software topology
- HIL proves the same behavior on real hardware
- phase exit criteria prove the maturity level expected at that point in the
  program

That is what turns "design intent" into engineering evidence.

## 5. How Use Cases, Requirements, and Architecture Fit Together

The most useful way to read this repo is not by document alone, but by
following one use case through the full chain.

### 5.1 UC1 -- Read DTCs via SOVD

| Layer | Story |
|---|---|
| Use case | A tester wants `GET /sovd/v1/components/{id}/faults` and expects JSON DTCs, not raw UDS bytes. |
| Requirements | `FR-1.1` defines per-component DTC listing, `FR-1.5` defines cross-component aggregation, `NFR-1.1` defines the latency target. |
| Architecture | `sovd-gateway` routes by component id, `sovd-server` serves the resource, DFM handles native DTC state, CDA handles legacy UDS-backed ECUs. Runtime sequences are in `ARCHITECTURE.md` 6.1 and 6.4. |
| Implementation | `sovd-server` handlers, `SovdBackend` trait dispatch, DFM query path, CDA integration, optional Pi proxy path for physical ECUs. |
| Verification | Unit tests, integration tests, and `hil_sovd_01_read_faults_all.yaml`. |

This is the professional mapping:

- the use case defines the external behavior
- the requirement makes it precise
- the architecture decides who is responsible
- the code implements exactly those responsibilities
- the test proves the user story on real hardware

### 5.2 UC2 -- Report fault via Fault API

| Layer | Story |
|---|---|
| Use case | An ECU-side software component reports a fault without blocking its time-critical path. |
| Requirements | `FR-4.1`, `FR-4.2`, `FR-4.3` define the fault reporting path; `NFR-1.2` sets visibility latency; `SR-4.1` and `SR-4.2` protect the safety boundary. |
| Architecture | Faults cross exactly one boundary through the Fault Library into DFM. DFM owns debounce, operation-cycle gating, DTC lifecycle, and persistence. |
| Implementation | C shim on embedded, Rust-side sink, DFM pipeline, SQLite storage. |
| Verification | `phase3_dfm_sqlite_roundtrip.rs` plus unit coverage on debounce and lifecycle logic. |

This use case is especially important because it shows the difference between
functional and safety requirements:

- functionally, a fault must become visible through SOVD
- safety-wise, the report path must never block or destabilize ASIL-rated code

### 5.3 UC3 -- Clear DTCs

| Layer | Story |
|---|---|
| Use case | An authenticated tester clears fault memory for a component. |
| Requirements | `FR-1.3` defines the functional operation; `SEC-2.1`, `SEC-2.2`, and `SEC-3.1` add authentication, authorization, and auditability. |
| Architecture | Auth middleware sits in front of the route, Gateway resolves backend, CDA maps to UDS `0x14` where needed, audit sink records the operation. |
| Implementation | Auth module, route handler, backend clear path, audit sink, CDA clear-fault translation. |
| Verification | `hil_sovd_02_clear_faults.yaml` plus audit checks. |

This is the professional lesson: use cases are rarely only functional. They
usually pull in security and compliance requirements too.

### 5.4 UC4 -- Reach a legacy UDS ECU through SOVD

| Layer | Story |
|---|---|
| Use case | A tester addresses one SOVD endpoint and does not need to care whether the ECU is virtual DoIP or physical CAN-only hardware. |
| Requirements | `FR-5.1` virtual DoIP path, `FR-5.2` physical proxy path, `FR-5.3` CDA configuration, `FR-5.4` session mirroring. |
| Architecture | Gateway chooses CDA backend; CDA speaks DoIP; Pi proxy adapts DoIP to CAN ISO-TP for physical ECUs. |
| Implementation | CDA integration, proxy server, SocketCAN, route config, MDD and backend wiring. |
| Verification | `phase2_cda_ecusim_smoke.rs`, HIL captures, HIL fault-read scenarios. |

This use case explains why the architecture has both CDA and the Pi proxy.
Those blocks are not accidental complexity; they exist because the use case
demands topology-independence at the SOVD surface.

### 5.5 UC5 -- Trigger a diagnostic routine

| Layer | Story |
|---|---|
| Use case | A tester starts a routine such as motor self-test and polls until completion. |
| Requirements | `FR-2.1` to `FR-2.3` define start/stop/status; `SR-3.1` and `SR-3.2` define safety interlocks; `FR-5.5` requires UDS security mirroring on locked paths. |
| Architecture | SOVD route -> auth -> gateway -> CDA -> UDS `0x31`; firmware enforces runtime preconditions and returns the correct failure mode. |
| Implementation | Operation handlers, session/security checks, CDA routine translation, ECU routine handlers, async execution state. |
| Verification | `hil_sovd_03_operation_execution.yaml` and supporting unit tests. |

This is a strong example of requirements and architecture fitting the use case
in layers:

- the use case says "run a routine"
- functional requirements define the API contract
- safety requirements define when it must refuse
- architecture ensures those checks live in the correct place
- tests prove both the success and refusal paths

## 6. The Phase Story: How the Product Is Built Professionally

The repo also tells a delivery story, not just a static design story.
`MASTER-PLAN.md` breaks the work into phases so the system matures in the
correct order.

### Phase 0 -- Foundation

Purpose:

- align the team
- pin toolchains and workspace conventions
- establish the fork / upstream discipline

Professional meaning:

- before features, we control the engineering environment

### Phase 1 -- Embedded UDS + DoIP POSIX

Purpose:

- make the firmware and virtual ECUs expose the minimum UDS capabilities that
  SOVD will later rely on

Professional meaning:

- we do not design the SOVD facade before the legacy diagnostic substrate is
  viable

### Phase 2 -- CDA integration + CAN-to-DoIP proxy

Purpose:

- prove the path from SOVD-side software to both virtual and physical ECUs

Professional meaning:

- topology risk is retired early

### Phase 3 -- Fault Library + DFM

Purpose:

- make fault ingest and DTC state real, not mocked

Professional meaning:

- the central diagnostic data model is established before the full REST API is
  claimed complete

### Phase 4 -- SOVD Server + Gateway

Purpose:

- expose the ASAM SOVD MVP surface and make all 5 MVP use cases work in Docker

Professional meaning:

- only after the data paths and adapter paths are solid do we expose the
  customer-facing API contract

### Phase 5 -- End-to-end demo + HIL

Purpose:

- prove the same stories on real Taktflow hardware

Professional meaning:

- Docker success is necessary but not sufficient; real hardware is the actual
  integration truth

### Phase 6 -- Hardening

Purpose:

- complete production-grade transport security, observability, performance
  controls, and release readiness

Professional meaning:

- MVP behavior is separated from product hardening, which prevents false
  claims early and endless architecture churn late

## 7. Where Each Document Fits

| Document | Professional role in the story |
|---|---|
| [MASTER-PLAN.md](../MASTER-PLAN.md) | Program-level delivery story, phases, risks, gates, staffing, milestones |
| [USE-CASES.md](USE-CASES.md) | Customer-visible and observer-visible behavior catalog |
| [REQUIREMENTS.md](REQUIREMENTS.md) | Formal engineering contract and test basis |
| [ARCHITECTURE.md](ARCHITECTURE.md) | Technical realization strategy and runtime collaboration |
| [TRADE-STUDIES.md](TRADE-STUDIES.md) | Option analysis before committing to a direction |
| [docs/adr/](adr/) | Frozen design decisions with rationale |
| [TEST-STRATEGY.md](TEST-STRATEGY.md) | Verification approach and test-layer responsibilities |
| [SAFETY-CONCEPT.md](SAFETY-CONCEPT.md) | Safety boundary and QM vs ASIL discipline |
| [security-concept.md](security-concept.md) | Security control model and deferred hardening items |
| [SYSTEM-SPECIFICATION.md](SYSTEM-SPECIFICATION.md) | Single-file executive engineering reference |

This is the professional documentation stack:

- `USE-CASES` explains the user story
- `REQUIREMENTS` converts it into obligations
- `ARCHITECTURE` converts obligations into structure
- `ADRs` explain why that structure was chosen
- `TEST-STRATEGY` proves it
- `MASTER-PLAN` says when and in what maturity order it gets delivered

## 8. How to Read One Feature End-to-End

If you want to understand a feature professionally, read it in this order:

1. Find the use case in [USE-CASES.md](USE-CASES.md).
2. Open the linked requirement IDs in [REQUIREMENTS.md](REQUIREMENTS.md).
3. Open the linked runtime view in [ARCHITECTURE.md](ARCHITECTURE.md).
4. Check the relevant ADRs that constrain the design.
5. Locate the implementation crate or service named in the architecture.
6. Open the referenced integration or HIL test that verifies the flow.
7. Check the phase and exit criteria in [MASTER-PLAN.md](../MASTER-PLAN.md).

That is the intended engineering workflow for this repo.

## 9. What Makes This Story Professional

The repo tells a professional story because it does all of the following:

- starts from stakeholder-visible use cases, not internal modules
- expresses those use cases as numbered requirements
- separates functional, non-functional, safety, security, and compliance demands
- uses architecture as realization, not as marketing
- records major decisions in ADRs before burying them in code
- aligns implementation boundaries with architectural boundaries
- verifies the system at unit, integration, SIL, and HIL levels
- uses phase exits and milestones instead of declaring everything "done" at once
- preserves the safety boundary instead of treating diagnostics as a free-for-all
- keeps an upstream contribution path open by design

## 10. Bottom Line

The fit between use cases, requirements, and architecture in this repo is:

- **Use cases** define the externally meaningful stories.
- **Requirements** turn those stories into precise, testable obligations.
- **Architecture** assigns those obligations to concrete runtime elements.
- **ADRs** freeze the key choices behind that architecture.
- **Implementation** follows the architecture seams.
- **Verification** proves the use case works in the real topology.
- **Phase gates** make the maturity story explicit.

That is the full development story of OpenSOVD in professional terms.
