<!--
SPDX-FileCopyrightText: 2026 The Contributors to Eclipse OpenSOVD (Taktflow fork)
SPDX-License-Identifier: Apache-2.0
-->

# Taktflow Eclipse OpenSOVD — Requirements Specification

- Document ID: TAKTFLOW-SOVD-REQ
- Revision: 1.0
- Status: Draft
- Date: 2026-04-14
- Owner: Taktflow SOVD workstream (architect)

---

## 1. Introduction and Scope

### 1.1 Purpose

This document is the formal, numbered, testable requirements specification for
the Taktflow integration of Eclipse OpenSOVD. It is the contract every
subsequent task, code change, test case, and ADR in the SOVD workstream traces
back to. It is the authoritative reference for "what must the system do" and
"against what do we verify it".

### 1.2 Scope

In scope:

- The `opensovd-core` Rust workspace (Server, Gateway, DFM, DB, interfaces,
  client, tracing, main binary) as built in the Taktflow fork.
- The Classic Diagnostic Adapter (CDA) integration — consumed as-is from
  upstream, not re-implemented. Our requirements cover its integration, not
  its internals.
- The CAN-to-DoIP proxy that lives on the Raspberry Pi gateway and bridges
  physical CAN ECUs to the SOVD stack.
- The embedded Fault Library C shim on Taktflow firmware (POSIX virtual ECUs
  and the STM32 / TMS570 physical ECUs).
- The new UDS services (0x14, 0x19, 0x31) and DoIP POSIX transport added to
  Taktflow firmware to satisfy SOVD use cases via CDA.
- The Docker Compose SIL topology, the Pi HIL topology, and the production
  deployment.

Out of scope: see §8.

### 1.3 How to use this document

- Every requirement has a stable ID. IDs are never renumbered. If a requirement
  is removed it is marked obsolete, not deleted.
- Acceptance criteria in each requirement are the primary input to test-case
  design. Unit, integration, SIL, and HIL tests trace their `Verifies:` tags to
  these IDs.
- `Source` fields cite the authoritative upstream document (ISO 17978 clause,
  upstream design.md section, upstream mvp.md item) or the Taktflow-internal
  document this requirement is derived from. No requirement is self-sourced.
- `Rationale` ties every requirement to one of the governing principles in
  MASTER-PLAN §A, §B, or §C. If a requirement has no governance link it does
  not belong here.
- Phase tags (Phase 1..Phase 6) align with MASTER-PLAN §4.

### 1.4 Revision control

This document is version-controlled in the Taktflow fork under
`H:\taktflow-opensovd\docs\REQUIREMENTS.md`. Changes happen via internal review
only during Phases 0..3 (per MASTER-PLAN §C.1). Upstream contribution of this
doc, or a derivative of it, is a Phase 6 decision.

### 1.5 Document conventions

- Keywords MUST, SHALL, SHOULD, MAY follow RFC 2119 semantics.
- "The system" means the full Taktflow OpenSOVD stack end-to-end unless a
  requirement names a specific component.
- "MVP" means the five use cases listed in upstream mvp.md §Use-cases, re-stated
  in MASTER-PLAN §2.2.
- "Component" is used in the SOVD sense (one addressable ECU view, identified by
  a `ComponentId` per `sovd-interfaces`).
- "ASIL-D code" refers to any firmware module whose safety allocation is
  ASIL-B or higher per Taktflow's safety goals; SOVD does not distinguish
  below that threshold since diagnostics is QM per upstream design.md §Safety
  Impact.

---

## 2. Stakeholders

| # | Stakeholder | Concern |
|---|-------------|---------|
| S1 | Taktflow BMS OEM/T1 customers | SOVD-reachable diagnostics for their integrations; legacy UDS paths must keep working |
| S2 | Eclipse OpenSOVD upstream maintainers | Any eventual PR from us must not diverge from upstream design |
| S3 | Eclipse S-CORE v1.0 integration team | Fault Library boundary per ADR-001 must be honored |
| S4 | Taktflow safety engineer | ISO 26262 ASIL-D lifecycle preservation; HARA coverage for any new diagnostic service |
| S5 | Taktflow embedded developers | Minimal churn to existing Dcm/Dem; clear insertion points |
| S6 | Taktflow Rust developers | Trait contracts stable; upstream sync never blocked by our own divergence |
| S7 | Taktflow test engineers | Testable acceptance criteria; SIL/HIL parity |
| S8 | Taktflow DevOps | Reproducible SIL topologies; observable Pi deployment |
| S9 | Vehicle-level testers (off-board) | ISO 17978 SOVD REST endpoints reachable over HTTPS |
| S10 | Fleet / cloud operators (future) | Scalable authn, audit, rate limits |

Every requirement below must serve at least one stakeholder; `Stakeholder:`
tags list the primary beneficiaries.

---

## 3. Functional Requirements

### 3.1 FR-1 — DTC management

#### FR-1.1 List DTCs per component
- Title: List DTCs for a named component, filtered by status mask.
- Description: The SOVD Server SHALL expose `GET /sovd/v1/components/{id}/faults`
  and return the set of DTCs currently held for component `{id}`, filtered by
  the `status-mask` query parameter interpreted as a bitwise-AND mask against
  each DTC's status byte. A mask of `0x00` disables filtering.
- Rationale: ISO 17978 §6 defines the `faults` resource; MASTER-PLAN §2.2 UC1;
  upstream mvp.md §Use-cases item 1.
- Acceptance criteria:
  1. Given a component with three DTCs of statuses `0x09`, `0x01`, `0x0A`, a
     GET with `status-mask=0x08` returns exactly the two DTCs whose status bit
     3 is set.
  2. GET with no mask returns all DTCs (order unspecified but stable within
     one call).
  3. Response is valid JSON matching the `Dtc` schema in `sovd-interfaces`.
  4. Unknown component returns HTTP 404 with a `SovdError::NotFound` body.
- Source: ISO 17978 §6 "faults"; upstream design.md §SOVD Server; upstream
  mvp.md §Use-cases 1.
- Stakeholder: S1, S9
- Priority: MVP
- Phase: 4

#### FR-1.2 Per-DTC detail
- Title: Fetch a single DTC by id.
- Description: The SOVD Server SHALL expose
  `GET /sovd/v1/components/{id}/faults/{dtc-id}` and return the full `Dtc`
  record for the named DTC. If the DTC is not currently held, the server
  SHALL return HTTP 404.
- Rationale: ISO 17978 §6; UC1 drill-down; matches
  `SovdServer::get_dtc` contract.
- Acceptance criteria:
  1. Detailed record includes status byte, occurrence count, first/last
     occurrence timestamp, severity, and any metadata fields supplied by the
     originating Fault Library or CDA path.
  2. Returned JSON round-trips through `sovd-interfaces::types::dtc::Dtc`.
  3. 404 for unknown DTC id carries body `{"error":"NotFound", ...}`.
- Source: ISO 17978 §6; upstream design.md §Diagnostic DB; opensovd-core
  `sovd-interfaces/src/traits/server.rs` `get_dtc`.
- Stakeholder: S1, S9
- Priority: MVP
- Phase: 4

#### FR-1.3 Clear DTCs
- Title: Clear DTCs, all or by group.
- Description: The SOVD Server SHALL expose
  `POST /sovd/v1/components/{id}/faults/clear` with an optional `group` query
  parameter. Clearing with no `group` clears every DTC held for the component;
  with `group=<code>` clears only DTCs belonging to that group. The operation
  SHALL be idempotent.
- Rationale: MASTER-PLAN §2.2 UC3; upstream mvp.md §Use-cases 3; maps directly
  to UDS `0x14 ClearDiagnosticInformation` on CDA-backed ECUs.
- Acceptance criteria:
  1. Clearing an empty set returns HTTP 204 (no content), not an error.
  2. After clear, FR-1.1 returns an empty list for that component until new
     faults arrive.
  3. On a CDA-backed component, the clear MUST reach the ECU as UDS 0x14 with
     the mapped group code; HIL test proves the underlying NvM flush.
  4. Audit log records the operation (see SEC-3.x).
- Source: ISO 17978 §6.3; upstream mvp.md §Use-cases 3; ISO 14229 §11.3.
- Stakeholder: S1, S4, S9
- Priority: MVP
- Phase: 4 (Server), 1 (UDS 0x14 handler)

#### FR-1.4 Pagination
- Title: Paginate large DTC lists.
- Description: When a component holds more than `page-size` DTCs (default 50),
  `GET .../faults` SHALL support `page` and `page-size` query parameters.
  Response SHALL include the total count and next-page cursor.
- Rationale: NFR-1.x latency budget; HIL scenario `hil_sovd_07_large_dtc_list`.
- Acceptance criteria:
  1. Component with 120 DTCs and `page-size=50` returns 50 DTCs on page 1,
     50 on page 2, 20 on page 3, with a `total=120` field and a next-cursor
     absent on page 3.
  2. Invalid pagination params return HTTP 400 with `InvalidRequest`.
- Source: ISO 17978 §6 pagination conventions; MASTER-PLAN §5 phase 5 HIL
  scenario list.
- Stakeholder: S1, S7
- Priority: MVP
- Phase: 4

#### FR-1.5 Multi-component DTC aggregation
- Title: List DTCs across all components.
- Description: The SOVD Gateway SHALL expose `GET /sovd/v1/faults` (no
  component path parameter) and return DTCs aggregated from every registered
  backend, each row tagged with its originating `ComponentId`. Single-backend
  failures SHALL be logged and omitted, not propagated as a global error,
  unless every backend failed.
- Rationale: Master plan §2 topology shows gateway fan-out; maps to
  `SovdGateway::list_all_dtcs` in `sovd-interfaces`.
- Acceptance criteria:
  1. With 3 backends (1 virtual BCM, 2 physical CVC + SC via CDA; ADR-0023)
     each holding 1-5 DTCs, the response is the ordered concatenation tagged
     by component id.
  2. Killing one ECU mid-call still returns the other backends' data with a
     warning log entry identifying the missing backend.
  3. Killing all backends returns HTTP 502 with `BackendFailure`.
- Source: upstream design.md §SOVD Gateway; opensovd-core
  `sovd-interfaces/src/traits/gateway.rs` `list_all_dtcs`.
- Stakeholder: S1, S9
- Priority: MVP
- Phase: 4

### 3.2 FR-2 — Diagnostic routines

#### FR-2.1 Start routine
- Title: Start a named routine with raw argument bytes.
- Description: The SOVD Server SHALL expose
  `POST /sovd/v1/components/{id}/operations/{routine-id}/start` accepting a raw
  binary body (argument bytes per ODX/MDD) and, upon success, report that the
  routine has been **accepted**. Routine completion is observed via FR-2.3.
- Rationale: MASTER-PLAN §2.2 UC5; upstream mvp.md §Use-cases 5; maps to
  UDS `0x31 RoutineControl subfunction 0x01 startRoutine`.
- Acceptance criteria:
  1. A valid start returns HTTP 202 (accepted) within 100 ms.
  2. An unknown routine id returns 404.
  3. Bad argument length returns 400.
  4. Precondition failure (see SR-3.x) returns 409 with a safety-interlock
     reason code.
- Source: ISO 17978 §operations; ISO 14229 §12.5; upstream design.md §Service
  App; opensovd-core `SovdServer::start_routine`.
- Stakeholder: S1, S4, S9
- Priority: MVP
- Phase: 4 (Server), 1 (UDS 0x31 handler)

#### FR-2.2 Stop routine
- Title: Stop a running routine.
- Description: The SOVD Server SHALL expose
  `POST /sovd/v1/components/{id}/operations/{routine-id}/stop`, translated on
  CDA paths to UDS 0x31 sub-function 0x02. Stop on an already-stopped routine
  is idempotent.
- Rationale: ISO 14229 §12.5; completes the UC5 pair.
- Acceptance criteria:
  1. Stop after start returns HTTP 204; subsequent status (FR-2.3) is
     `Stopped`.
  2. Stop on never-started routine returns 204 (idempotent) with a warning
     header.
- Source: ISO 14229 §12.5.
- Stakeholder: S1, S7
- Priority: MVP
- Phase: 4, 1

#### FR-2.3 Poll routine status
- Title: Query routine state.
- Description: The SOVD Server SHALL expose
  `GET /sovd/v1/components/{id}/operations/{routine-id}/status` returning the
  current state (`Idle`, `Running`, `Completed`, `Failed`, `Stopped`) and, if
  completed, the result payload.
- Rationale: Non-blocking UX; maps to UDS 0x31 sub-function 0x03
  `requestRoutineResults`.
- Acceptance criteria:
  1. Routine never started returns `Idle` — never 404 (matches
     `SovdServer::routine_status` contract).
  2. Running routine reports `Running` until the handler completes.
  3. Completed routine reports `Completed` with an opaque result byte slice
     in the body.
- Source: ISO 14229 §12.5.3; opensovd-core `SovdServer::routine_status`.
- Stakeholder: S1, S7
- Priority: MVP
- Phase: 4, 1

#### FR-2.4 Routine registry
- Title: Routines are discoverable.
- Description: The SOVD Server SHALL expose
  `GET /sovd/v1/components/{id}/operations` returning the catalogue of known
  routine ids and their ODX-declared argument schemas.
- Rationale: Discoverability is part of ISO 17978 resource model; a tester
  should never need ECU source to enumerate routines.
- Acceptance criteria:
  1. List is derived from the MDD catalogue for the component.
  2. Entries include routine id, human-readable name, arg schema, return
     schema.
- Source: ISO 17978 §6; upstream design.md §SOVD Server.
- Stakeholder: S1, S9
- Priority: MVP
- Phase: 4

### 3.3 FR-3 — Component metadata

#### FR-3.1 List components
- Title: Enumerate known components.
- Description: The SOVD Gateway SHALL expose `GET /sovd/v1/components`
  returning every registered backend's `ComponentId` and summary metadata.
- Rationale: ISO 17978 §components root; UC1/UC4 entry point.
- Acceptance criteria:
  1. Returns 3 entries for the Taktflow MVP (CVC, SC, BCM) per ADR-0023.
  2. Summary includes `component_id`, kind, backing source (`Dfm`, `Cda`,
     `NativeSovd`), and reachability status.
- Source: ISO 17978 §6; opensovd-core
  `sovd-interfaces::traits::backend::BackendKind`.
- Stakeholder: S1, S9
- Priority: MVP
- Phase: 4

#### FR-3.2 Component detail
- Title: Report HW and SW version per component.
- Description: The SOVD Server SHALL expose
  `GET /sovd/v1/components/{id}` returning a `ComponentInfo` record with
  hardware revision, software version, serial (if available), and the
  provider-source tag.
- Rationale: upstream mvp.md §Use-cases item 6 (OPTIONAL in upstream, MVP for
  us because customers expect it).
- Acceptance criteria:
  1. Returned JSON matches `sovd-interfaces::types::component::ComponentInfo`.
  2. HW revision comes from the ECU-local DID `0xF18C` (HW Serial) on CDA
     paths.
  3. SW version comes from DID `0xF195` (SW version) on CDA paths, or from
     the native Fault Library catalogue version on Fault-lib-backed paths.
- Source: ISO 17978 §6; upstream mvp.md §Use-cases 6; Taktflow
  `docs/sovd/did-inventory.md`.
- Stakeholder: S1, S9
- Priority: MVP (Taktflow promotes from OPTIONAL)
- Phase: 4

#### FR-3.3 Data identifier catalogue
- Title: List available DIDs and read a single DID.
- Description: The SOVD Server SHALL expose
  `GET /sovd/v1/components/{id}/data` to list DIDs and
  `GET /sovd/v1/components/{id}/data/{did}` to read a single DID's current
  value. CDA-backed reads translate to UDS 0x22; native reads use the MDD
  provider.
- Rationale: ISO 17978 §6 data resource; Taktflow `did-inventory.md`.
- Acceptance criteria:
  1. Reading DID `0xF190` on CDA-backed CVC returns the VIN string within
     500 ms.
  2. DID read errors return HTTP 422 `InvalidState` with a UDS NRC mapping.
- Source: ISO 17978 §6; ISO 14229 §10.4.
- Stakeholder: S1
- Priority: MVP
- Phase: 4

#### FR-3.4 Capability discovery
- Title: Advertise which SOVD operations a component supports.
- Description: The server SHALL return a capability flag set per component
  telling testers which resource kinds (`faults`, `operations`, `data`,
  `modes`) are implemented by the backing ECU. A CDA-only ECU without routine
  support MUST not advertise `operations`.
- Rationale: Prevents spurious 404s; lets tooling generate accurate UIs.
- Acceptance criteria:
  1. Capabilities derived from MDD + backend kind at registration time.
  2. Capability set is stable for the life of the backend registration.
- Source: ISO 17978 §discovery; Taktflow principle C.3 (concrete before
  abstract).
- Stakeholder: S1, S7
- Priority: MVP
- Phase: 4

### 3.4 FR-4 — Fault reporting pipeline

#### FR-4.1 Fault API call
- Title: Components can report faults via the Fault Library shim.
- Description: Platform and application code SHALL be able to call a
  framework-agnostic Fault API (`FaultShim_Report(fid, severity, metadata)`)
  to report a fault observation. The shim MUST be C on embedded, Rust on
  Pi/POSIX (MASTER-PLAN §2.1 design decision 3).
- Rationale: upstream design.md §Fault Library; ADR-001; MASTER-PLAN §B.1.
- Acceptance criteria:
  1. Fault API signature includes: FID (32-bit), time (monotonic ns),
     severity enum, optional metadata blob (<=64 bytes MVP).
  2. Call is non-blocking (see SR-4.x).
  3. On POSIX, the shim writes via Unix domain socket to the DFM.
  4. On STM32, the shim buffers into a NvM slot flushed by gateway sync.
- Source: upstream design.md §Fault Library; upstream mvp.md §Requirements 2;
  MASTER-PLAN §3.1.
- Stakeholder: S3, S5
- Priority: MVP
- Phase: 3

#### FR-4.2 DFM ingest
- Title: DFM receives faults from all Fault Library shims.
- Description: The DFM SHALL listen on a documented IPC endpoint and record
  every `FaultRecord` delivered by a Fault Library shim via the `FaultSink`
  trait contract.
- Rationale: upstream design.md §Diagnostic Fault Manager;
  `sovd-interfaces::traits::fault_sink::FaultSink`; ADR-001.
- Acceptance criteria:
  1. IPC path is configurable via `opensovd-gateway.toml`.
  2. Every delivered fault is visible in SOVD GET within 100 ms (NFR-1.x).
  3. IPC failure MUST NOT crash the DFM; it MUST retry with bounded backoff.
  4. `FaultSink::record_fault` is not idempotent — two calls with identical
     records represent two observations.
- Source: upstream design.md §DFM; `sovd-interfaces::FaultSink` docs.
- Stakeholder: S3, S7
- Priority: MVP
- Phase: 3

#### FR-4.3 Server-side debounce and operation cycle
- Title: DFM applies debounce and operation-cycle gating centrally.
- Description: The DFM SHALL implement debounce (N occurrences before DTC
  exposed) and operation-cycle suppression (faults expected during certain
  cycles are not promoted to DTCs). Embedded code SHALL NOT implement these
  policies — they live on the server side.
- Rationale: upstream design.md §DFM "Implements the operation cycle concept";
  MASTER-PLAN §3 principle: keep ECU path simple.
- Acceptance criteria:
  1. Debounce thresholds loaded from DFM config at startup.
  2. A fault crossing the threshold is visible as a DTC; below threshold is
     counted but not visible.
  3. Operation cycle start/stop API documented; suppressed faults during a
     cycle are not stored as DTCs.
- Source: upstream design.md §DFM; MASTER-PLAN §3 Phase 3.
- Stakeholder: S3, S4
- Priority: MVP
- Phase: 3

#### FR-4.4 Persistence
- Title: DFM persists DTCs through the Diagnostic DB.
- Description: The DFM SHALL persist DTC state via `sovd-db` (SQLite, `sqlx`)
  with schema migrations versioned.
- Rationale: upstream design.md §Diagnostic DB; MASTER-PLAN §2.1 decision 4;
  master plan §3 Phase 3 deliverable.
- Acceptance criteria:
  1. Tables: `dtcs`, `fault_events`, `operation_cycles`, `catalog_version`.
  2. Restart of DFM process recovers last-known DTC state.
  3. Migrations run on startup; version mismatch is a fatal error.
- Source: upstream design.md §Diagnostic DB; MASTER-PLAN §3.1 Phase 3.
- Stakeholder: S5, S7
- Priority: MVP
- Phase: 3

#### FR-4.5 Catalog version checks
- Title: DTC catalog version must match Fault Library reports.
- Description: The DFM SHALL compare the catalog version reported by the
  Fault Library shim at connect time against its own loaded catalog. Mismatch
  is a connect-time error.
- Rationale: upstream mvp.md §26Q2 "catalog version checks".
- Acceptance criteria:
  1. Mismatch logged with both versions and rejection reason.
  2. Mismatch prevents fault ingestion from that component; other components
     continue.
- Source: upstream mvp.md §26Q2.
- Stakeholder: S3, S4
- Priority: MVP
- Phase: 3

### 3.5 FR-5 — Legacy UDS compatibility

#### FR-5.1 CDA reach — virtual ECUs over DoIP
- Title: POSIX virtual ECUs speak DoIP directly; CDA reaches them over TCP.
- Description: The virtual ECU (BCM as POSIX container, plus optional CVC
  POSIX build for SIL-only coverage) SHALL accept DoIP on TCP 13400 and
  respond to UDS 0x19 / 0x14 / 0x31 issued by CDA. (FZC/RZC/ICU/TCU retired
  per ADR-0023; the stack still supports any number of POSIX ECUs via
  config, but the bench runs only BCM.)
- Rationale: MASTER-PLAN §2.1 design decision 1; §3.1 Phase 1 deliverable 4.
- Acceptance criteria:
  1. `curl http://cda:8080/sovd/v1/components/cvc/faults` returns DTCs when
     CVC holds any.
  2. DoIP vehicle identification, routing activation, and diagnostic message
     message types are all accepted.
  3. Dcm dispatches 0x19 / 0x14 / 0x31 via the `Dcm_DispatchRequest()`
     insertion point documented in `docs/sovd/notes-dcm-walkthrough.md`.
- Source: MASTER-PLAN §2.1, §3.1 phase 1; upstream design.md §CDA; Taktflow
  `docs/sovd/notes-dcm-walkthrough.md`.
- Stakeholder: S1, S5, S7
- Priority: MVP
- Phase: 1, 2

#### FR-5.2 CDA reach — physical ECUs via CAN-to-DoIP proxy
- Title: Physical STM32 / TMS570 ECUs reached through Pi proxy.
- Description: The Raspberry Pi gateway SHALL run a CAN-to-DoIP proxy that
  listens on TCP 13400 and translates DoIP diagnostic messages to CAN ISO-TP
  frames on the physical bus; responses flow back the same way.
- Rationale: MASTER-PLAN §2.1 design decision 2; §3.2 CAN-to-DoIP proxy
  deliverable; required because STM32 ECUs have CAN only.
- Acceptance criteria:
  1. HIL test: SOVD GET → CDA → proxy → physical CVC → CAN 0x7XX → Dcm →
     response chain works end-to-end.
  2. Proxy unit tests have >=80% line coverage.
  3. SC (TMS570) is reachable through the same proxy path (no physical DoIP
     on TMS570 is required, per MASTER-PLAN §14 deferred item).
- Source: MASTER-PLAN §3.2 Phase 2 deliverable 2; §14 open questions (TMS570
  Ethernet deferred).
- Stakeholder: S1, S5, S7
- Priority: MVP
- Phase: 2

#### FR-5.3 CDA configuration
- Title: CDA is configured for the Taktflow topology without upstream fork.
- Description: The CDA SHALL be consumed as-is from upstream
  `classic-diagnostic-adapter`; Taktflow customizations live in an
  `opensovd-cda.toml` and in the committed MDD path, not in edits to CDA
  source.
- Rationale: MASTER-PLAN §C.2a (mirror upstream wholesale); §3.3 Phase 2.
- Acceptance criteria:
  1. Zero source-level diffs vs. upstream CDA main branch in our fork.
  2. MDDs for the active UDS-addressable ECUs are generated by
     `odx-converter` from Taktflow ODX files and committed. For the 3-ECU
     bench (ADR-0023) this means CVC only; SC is not yet UDS-addressable
     and BCM runs as a POSIX SOVD native.
  3. Any CDA bug found during integration is captured as an isolated
     upstream-ready patch, not an inline edit (per MASTER-PLAN §C.2b).
- Source: MASTER-PLAN §C.2a, §3.2.
- Stakeholder: S2, S6
- Priority: MVP
- Phase: 2

#### FR-5.4 UDS session mirroring
- Title: SOVD session maps to UDS session on CDA paths.
- Description: When a SOVD request reaches a CDA-backed ECU, CDA SHALL
  establish the correct UDS session (default/extended) before issuing the
  underlying 0x19 / 0x14 / 0x31 / 0x22.
- Rationale: ISO 14229 §9.2; existing Dcm session state machine.
- Acceptance criteria:
  1. Routine calls requiring extended session auto-elevate via 0x10 0x03
     before 0x31.
  2. Session state tracked per-ECU in CDA (upstream behavior, not our code).
- Source: ISO 14229 §9.2; Taktflow `docs/sovd/notes-dcm-walkthrough.md`.
- Stakeholder: S5
- Priority: MVP
- Phase: 2

#### FR-5.5 UDS security access mirroring
- Title: Security-locked services honor seed/key on CDA paths.
- Description: SOVD endpoints that ultimately trigger security-gated UDS
  services MUST complete UDS 0x27 seed/key before issuing the gated request.
  If the SOVD caller is not authorized per SEC-2.x, the request is denied
  before reaching CDA.
- Rationale: ISO 14229 §9.5; MASTER-PLAN §C.4 (safety never slips).
- Acceptance criteria:
  1. Unauthorized SOVD caller trying to clear DTCs or start a safety-gated
     routine receives 403, and no UDS traffic is emitted.
  2. Authorized caller's request reaches CDA, which runs 0x27 internally.
- Source: ISO 14229 §9.5; SEC-2.x.
- Stakeholder: S4
- Priority: MVP
- Phase: 4 (server auth) + 2 (CDA wiring)

### 3.6 FR-6 — Multi-ECU aggregation

#### FR-6.1 Gateway routing table
- Title: Gateway maps ComponentId to backend kind at startup.
- Description: The Gateway SHALL load a routing configuration
  (`opensovd-gateway.toml`) at startup that maps each `ComponentId` to exactly
  one backend (`Dfm`, `NativeSovd`, `Cda`, `Federated`). Duplicate registration
  is an error.
- Rationale: opensovd-core `SovdGateway::register_backend` contract;
  MASTER-PLAN §2.
- Acceptance criteria:
  1. Missing component in config returns 404 on any request for that id.
  2. Duplicate id in config fails startup with a clear diagnostic.
- Source: opensovd-core `sovd-interfaces::SovdGateway::register_backend`.
- Stakeholder: S7, S8
- Priority: MVP
- Phase: 4

#### FR-6.2 Federated hop
- Title: Gateway can forward to another Gateway via `sovd-client`.
- Description: The Gateway MAY route a `ComponentId` to a remote Gateway via
  an outbound `sovd-client` call. This is the federated-topology path.
- Rationale: opensovd-core architecture doc §SOVD Gateway backends table
  (row `Federated`).
- Acceptance criteria:
  1. Config file option `backend = "federated"` with a `base_url` field.
  2. Federated requests propagate auth tokens and correlation ids.
  3. Failure of the remote gateway returns HTTP 502 with a clear source tag.
- Source: opensovd-core ARCHITECTURE.md §SovdBackend table.
- Stakeholder: S1
- Priority: Optional
- Phase: 4 (stub) / 6 (full)

### 3.7 FR-7 — Session and security

#### FR-7.1 Session resource
- Title: SOVD session creation and teardown.
- Description: The SOVD Server SHALL expose a `sessions` resource per ISO
  17978 allowing a tester to create, query, and close a session. Session
  scope is per-client, not per-component.
- Rationale: ISO 17978 §sessions; mirrors UDS 0x10 conceptually.
- Acceptance criteria:
  1. POST `/sovd/v1/sessions` creates a session id.
  2. Subsequent calls carry `X-SOVD-Session: <id>` header.
  3. Session expiry follows SEC-4.x.
- Source: ISO 17978 §sessions; opensovd-core
  `sovd-interfaces::types::session::Session`.
- Stakeholder: S1, S9
- Priority: MVP
- Phase: 4

#### FR-7.2 Security level
- Title: Elevated operations require elevated session security level.
- Description: Routines or clears that require elevated privilege SHALL be
  rejected unless the session holds the required `SecurityLevel`.
- Rationale: ISO 14229 security access parity; MASTER-PLAN §C.4.
- Acceptance criteria:
  1. Default security level is `Locked`.
  2. Elevation path (token exchange, cert, seed/key) is advertised via
     OpenAPI.
  3. Attempted elevated op under `Locked` returns 403.
- Source: ISO 17978 §security; opensovd-core
  `sovd-interfaces::types::session::SecurityLevel`.
- Stakeholder: S4, S9
- Priority: MVP
- Phase: 4 (scaffold), 6 (hardened)

---

## 4. Non-functional Requirements

### 4.1 NFR-1 — Performance

#### NFR-1.1 DTC read latency
- Description: GET `/sovd/v1/components/{id}/faults` MUST return in <= 500 ms
  at P99 across all 3 active ECUs (ADR-0023), measured on the production Pi
  topology under nominal load.
- Rationale: MASTER-PLAN §12.1 success criteria.
- Acceptance criteria: HIL nightly runs 500 iterations per scenario; P99 is
  computed and asserted in `hil_sovd_01_read_dtcs_all.yaml`.
- Source: MASTER-PLAN §5 Phase 5 performance targets.
- Priority: MVP
- Phase: 5

#### NFR-1.2 Fault ingest latency
- Description: The end-to-end time from `FaultShim_Report` to SOVD GET
  visibility MUST be <= 100 ms at median on the Pi topology.
- Rationale: MASTER-PLAN §3 Phase 3 exit criteria.
- Acceptance criteria: wiring test in Phase 3 asserts this with a synthetic
  fault injected at t0 and SOVD GET verified within 100 ms.
- Source: MASTER-PLAN §3 Phase 3 exit criterion 1.
- Priority: MVP
- Phase: 3

#### NFR-1.3 Concurrent testers
- Description: The SOVD Server MUST serve at least two concurrent off-board
  testers without request reordering or cross-contamination.
- Rationale: HIL scenario `hil_sovd_06_concurrent_testers`.
- Acceptance criteria: two clients each running the UC1/UC3/UC5 loop 100
  times interleave cleanly; no request observes another client's session.
- Source: MASTER-PLAN §5 Phase 5 scenario 6.
- Priority: MVP
- Phase: 5

#### NFR-1.4 Memory footprint
- Description: SOVD Server + Gateway + DFM combined RSS on the Pi MUST remain
  below 200 MB in steady state.
- Rationale: MASTER-PLAN §5 Phase 5 performance targets.
- Acceptance criteria: HIL nightly captures `/proc/<pid>/status` RSS for each
  process; sum is recorded as a metric.
- Source: MASTER-PLAN §5 Phase 5.
- Priority: MVP
- Phase: 5

### 4.2 NFR-2 — Availability

#### NFR-2.1 Degraded mode on missing backend
- Description: If a backend fails or disconnects, the Gateway MUST continue
  serving requests for other backends.
- Rationale: FR-1.5 partial-failure semantics; HIL scenario
  `hil_sovd_08_error_handling`.
- Acceptance criteria: killing one ECU container mid-HIL does not affect
  requests to other components.
- Source: MASTER-PLAN §5 scenario 8.
- Priority: MVP
- Phase: 4

#### NFR-2.2 Reconnect on backend recovery
- Description: A previously failed backend that recovers MUST be auto-reintegrated
  without process restart.
- Rationale: Keeps field deployments robust.
- Acceptance criteria: bringing the killed backend back makes subsequent
  list-all-DTCs include it within 5 s.
- Source: master plan §5 scenario 8.
- Priority: MVP
- Phase: 4

#### NFR-2.3 No-ECU startup
- Description: The SOVD Server MUST start successfully even if zero backends
  are currently reachable; requests return 503 until backends are up.
- Rationale: SIL topology bring-up order.
- Acceptance criteria: start Server before any ECU container; `/health`
  returns 200, other routes 503.
- Source: internal — Taktflow C.3 (concrete before abstract, real startup
  order).
- Priority: MVP
- Phase: 4

### 4.3 NFR-3 — Observability

#### NFR-3.1 DLT tracing
- Description: Every Rust binary in `opensovd-core` MUST emit DLT-compatible
  tracing via `sovd-tracing` wired to `dlt-tracing-lib`.
- Rationale: MASTER-PLAN §4 Phase 6 deliverable 2.
- Acceptance criteria: `dlt-viewer` on the Pi shows context ids `SOVD`,
  `DFM`, `GW`, `CDA` with structured payloads.
- Source: MASTER-PLAN §4 Phase 6; upstream `dlt-tracing-lib`.
- Priority: MVP
- Phase: 6

#### NFR-3.2 OpenTelemetry spans
- Description: Every SOVD request MUST produce an OpenTelemetry trace that
  spans ingress → Gateway → Server → backend → ECU response.
- Rationale: MASTER-PLAN §4 Phase 6 deliverable 3.
- Acceptance criteria: traces visible in the configured OTLP collector
  (Jaeger or Tempo); P99 latencies computed from spans match HIL measurements.
- Source: MASTER-PLAN §4 Phase 6.
- Priority: MVP
- Phase: 6

#### NFR-3.3 Structured logs with correlation ids
- Description: All logs MUST be JSON-formatted and include the request
  correlation id inherited from the inbound `X-Request-Id` header (or
  generated if absent).
- Rationale: Observability parity with upstream CDA.
- Acceptance criteria: grepping a correlation id across Server, Gateway, DFM,
  CDA logs returns a full request trace.
- Source: upstream CDA observability conventions (NFR-6.x drives parity).
- Priority: MVP
- Phase: 6

### 4.4 NFR-4 — Portability

#### NFR-4.1 SIL/HIL/prod parity
- Description: The same `opensovd-core` binary artifact MUST run in all three
  topologies (Docker Compose SIL, Pi HIL bench, Pi production). Only
  configuration may differ.
- Rationale: MASTER-PLAN §2.3; principle C.3.
- Acceptance criteria: CI produces one artifact; SIL + HIL both consume it;
  no `#[cfg(...)]` branches keyed on topology in source.
- Source: MASTER-PLAN §2.3.
- Priority: MVP
- Phase: 4

#### NFR-4.2 Host OS portability
- Description: The Rust workspace MUST build on Linux (x86_64 and aarch64)
  and Windows (x86_64 dev hosts).
- Rationale: Team dev environment includes Windows machines; Pi target is
  linux-aarch64; SIL runners are linux-x86_64.
- Acceptance criteria: CI matrix has three targets and green on all.
- Source: Taktflow internal — team env.
- Priority: MVP
- Phase: 0-4

### 4.5 NFR-5 — Scalability

#### NFR-5.1 3-ECU MVP
- Description: The system MUST handle the 3-ECU Taktflow bench (BCM virtual
  + CVC, SC physical) concurrently. The 3 ECUs are chosen per ADR-0023 for
  maximal architectural code-path coverage with zero redundancy. The stack
  itself is not hardcoded to this count — additional ECUs can be added via
  config without code change (see NFR-5.2 for scaling headroom).
- Rationale: MASTER-PLAN §1.1; ADR-0023.
- Acceptance criteria: HIL scenario `hil_sovd_01_read_faults_all` exercises
  all 3 ECUs; the server's `/sovd/v1/components` endpoint returns exactly
  those 3 entries.
- Source: MASTER-PLAN §1.1.
- Priority: MVP
- Phase: 5

#### NFR-5.2 20+-ECU headroom
- Description: The Gateway routing table and backend registry MUST support
  at least 20 registered components without linear degradation.
- Rationale: future multi-customer topologies (MASTER-PLAN §B.2).
- Acceptance criteria: synthetic load test with 20 dummy backends shows
  P99 list-all-DTCs still within 1500 ms.
- Source: MASTER-PLAN §B.2 (multi-customer).
- Priority: Optional
- Phase: 6

### 4.6 NFR-6 — Maintainability

#### NFR-6.1 Max sync with upstream CDA style
- Description: All `opensovd-core` Rust code style, idioms, and file layout
  MUST be indistinguishable from upstream CDA by the end of Phase 4.
- Rationale: MASTER-PLAN §C.2a; §12.2 contribution readiness.
- Acceptance criteria: Phase 4 audit: any file in our fork can be diffed
  against the upstream CDA style guide with zero style deltas.
- Source: MASTER-PLAN §C.2a.
- Priority: MVP
- Phase: 4

#### NFR-6.2 Rust toolchain pinning
- Description: `rust-toolchain.toml` MUST pin 1.88.0 stable for builds and
  nightly for `cargo fmt --check`; edition 2024.
- Rationale: opensovd-core `ARCHITECTURE.md` §Conventions.
- Acceptance criteria: CI fails if toolchain drifts; `cargo +nightly fmt
  --check` is a PR gate.
- Source: opensovd-core `ARCHITECTURE.md`; `rust-toolchain.toml`.
- Priority: MVP
- Phase: 0

#### NFR-6.3 Axum 0.8 baseline
- Description: HTTP layer uses `axum 0.8` matching upstream CDA; no other
  HTTP framework introduced.
- Rationale: MASTER-PLAN §C.2a.
- Acceptance criteria: `cargo tree` shows a single axum version; deny.toml
  enforces it.
- Source: MASTER-PLAN §C.2a.
- Priority: MVP
- Phase: 4

#### NFR-6.4 Clippy pedantic clean
- Description: `cargo clippy --all-targets -- -D clippy::pedantic` MUST be
  clean on every Rust crate in `opensovd-core`.
- Rationale: MASTER-PLAN §C.5.
- Acceptance criteria: CI gate.
- Source: MASTER-PLAN §C.5.
- Priority: MVP
- Phase: 0+

#### NFR-6.5 MISRA clean embedded
- Description: All new C code (Fault shim, Dcm handlers, DoIP POSIX) MUST be
  MISRA C:2012 clean; deviations MUST be justified in
  `docs/safety/analysis/misra-deviation-register.md`.
- Rationale: MASTER-PLAN §C.4; SR-2.x.
- Acceptance criteria: cppcheck / coverity CI pass.
- Source: MASTER-PLAN §C.4.
- Priority: MVP
- Phase: 1-3

---

## 5. Safety Requirements

Context: Taktflow firmware is ISO 26262 ASIL-D. OpenSOVD itself is QM (upstream
design.md §Safety Impact). The interaction surface is where risk lives, and
these SRs protect it.

#### SR-1.1 No SOVD path modifies ASIL-D code without HARA update
- Description: Any change that introduces a new SOVD path into an ASIL-B or
  higher firmware module MUST be preceded by a HARA delta signed off by the
  safety engineer.
- Rationale: MASTER-PLAN §C.4; Taktflow `docs/safety/concept/hara.md`.
- Acceptance criteria: PR template enforces safety-engineer approval before
  any file under `firmware/bsw/services/Dcm`, `Dem`, `NvM`, `WdgM`, or
  `FaultShim` can be merged.
- Source: ISO 26262-3 §7; Taktflow HARA.
- Stakeholder: S4
- Priority: MVP
- Phase: 0+

#### SR-1.2 SOVD is QM by default
- Description: No SOVD component in `opensovd-core` MUST hold any ASIL
  allocation. Any code path where SOVD could influence a safety function MUST
  go through a safety-engineer-reviewed isolation layer.
- Rationale: upstream design.md §Safety Impact (QM posture).
- Acceptance criteria: `opensovd-core` crates do not link against any
  ASIL-allocated static or dynamic library.
- Source: upstream design.md §Safety Impact.
- Stakeholder: S2, S4
- Priority: MVP
- Phase: 0+

#### SR-2.1 MISRA C:2012 clean on new embedded code
- Description: All new C source introduced by SOVD work (Dcm handlers 0x14 /
  0x19 / 0x31, DoIP POSIX, Fault shim) MUST pass MISRA C:2012 static analysis.
- Rationale: Existing ASIL-D lifecycle in force (MASTER-PLAN §1.1).
- Acceptance criteria: `tools/misra/` CI gate green on every PR touching
  `firmware/bsw/services/` or `firmware/platform/`.
- Source: MASTER-PLAN §C.4; Taktflow `GOVERNANCE-SAFETY-ASPICE.md`.
- Stakeholder: S4, S5
- Priority: MVP
- Phase: 1+

#### SR-3.1 Routine interlock — motor self-test
- Description: `ROUTINE_MOTOR_SELF_TEST` MUST only execute when the vehicle
  is stationary (speed = 0 and park brake applied). The Dcm routine handler
  MUST reject the request with NRC `0x22 ConditionsNotCorrect` otherwise.
- Rationale: ASIL-D hazard HE-001/HE-017 (unintended motion); SG-001 safe
  state SS-MOTOR-OFF.
- Acceptance criteria:
  1. Unit test: precondition-fail returns the NRC and the routine never
     reaches the SWC.
  2. HIL test: attempt during simulated motion is refused.
- Source: Taktflow `docs/safety/concept/safety-goals.md` SG-001; ISO 14229
  §12.5.
- Stakeholder: S4
- Priority: MVP
- Phase: 1

#### SR-3.2 Routine interlock — brake check
- Description: `ROUTINE_BRAKE_CHECK` MUST only execute under explicit test
  mode (session = extended + service mode bit set in a platform status DID).
- Rationale: SG-004 (loss of braking); SG-005 (unintended braking).
- Acceptance criteria: preconditions enforced in the Dcm routine handler;
  HIL test proves refusal outside test mode.
- Source: Taktflow safety goals SG-004, SG-005.
- Stakeholder: S4
- Priority: MVP
- Phase: 1

#### SR-4.1 Fault API is non-blocking
- Description: `FaultShim_Report` MUST return to the caller in bounded time
  regardless of DFM availability. The shim MUST buffer and never block the
  calling ASIL-D code path.
- Rationale: upstream design.md §Fault Library "framework agnostic interface
  for apps or FEO activities to report faults"; ASIL-D isolation.
- Acceptance criteria:
  1. Unit test: shim returns within <10 microseconds on STM32 even when the
     IPC peer is absent.
  2. The shim provides FFI-safe signatures per upstream design.md §Security
     Impact "client lib(s) need to be developed with ... FFI guarantees".
- Source: upstream design.md §Fault Library; §Security Impact.
- Stakeholder: S3, S4
- Priority: MVP
- Phase: 3

#### SR-4.2 DFM failure does not propagate to safety functions
- Description: Crash, hang, or network loss of the DFM MUST NOT affect any
  ASIL-rated firmware function. The Fault shim on STM32 MUST buffer to NvM
  and continue.
- Rationale: upstream design.md §Safety Impact "no direct safety impact ...
  The Fault Library could also have a safety impact if faults are propagated
  and act upon by other components".
- Acceptance criteria: fault injection test: kill DFM, verify ECU Safe State
  monitor stays green, faults buffer, reconnect flushes.
- Source: upstream design.md §Safety Impact; Taktflow HARA.
- Stakeholder: S4
- Priority: MVP
- Phase: 3

#### SR-5.1 DoIP transport isolation
- Description: The DoIP POSIX transport (new in Phase 1) MUST NOT share task
  context with any safety function. A malformed DoIP packet MUST not starve
  safety tasks of CPU.
- Rationale: MASTER-PLAN §3.1 embedded gaps; preserves ASIL-D runtime
  guarantees.
- Acceptance criteria:
  1. DoIP task has a bounded stack and a rate-limiter.
  2. Watchdog supervises DoIP task separately from safety tasks.
- Source: Taktflow `docs/safety/requirements/hsi-specification.md`.
- Stakeholder: S4
- Priority: MVP
- Phase: 1

---

## 6. Security Requirements

#### SEC-1.1 TLS on all external SOVD endpoints
- Description: The SOVD Server and Gateway MUST accept only HTTPS on external
  network interfaces. Plain HTTP is permitted only on `127.0.0.1` for local
  SIL tests.
- Rationale: upstream design.md §Security Impact; ISO 17978 §security.
- Acceptance criteria: Phase 6 HIL + prod topology refuses HTTP connections
  on external adapters; `curl http://...` is rejected.
- Source: upstream design.md §Security Impact; MASTER-PLAN §4 Phase 6.
- Stakeholder: S2, S4, S9
- Priority: MVP
- Phase: 6

#### SEC-2.1 Cert-based mutual authentication
- Description: External testers MUST authenticate via X.509 client
  certificate; the server validates against a configured trust anchor.
- Rationale: upstream design.md §Security Impact.
- Acceptance criteria: unauth'd client receives TLS handshake failure; auth'd
  client reaches the API.
- Source: upstream design.md §Security Impact; ISO 17978 §security.
- Stakeholder: S4, S9
- Priority: MVP (scaffold Phase 4), full (Phase 6)
- Phase: 6

#### SEC-2.2 Token-based authorization
- Description: In addition to certs, fine-grained authorization MUST be
  expressed via bearer tokens in the `Authorization: Bearer <token>` header.
  MVP scaffolds the plumbing; Phase 6 enforces.
- Rationale: ISO 17978 §security; MASTER-PLAN §4 Phase 4 deliverable 3.
- Acceptance criteria: token validated against configurable OAuth2/OIDC
  endpoint (Phase 6); MVP accepts any well-formed token.
- Source: MASTER-PLAN §4 Phase 4 deliverable 3.
- Stakeholder: S9
- Priority: MVP (scaffold), full (Phase 6)
- Phase: 4, 6

#### SEC-3.1 Audit log for privileged operations
- Description: Every clear-DTC, start-routine, write-DID, and session-elevate
  call MUST produce an immutable audit log entry including caller identity,
  component id, timestamp, and outcome.
- Rationale: upstream design.md §Security Impact; regulatory traceability.
- Acceptance criteria: audit log is a separate append-only sink (not mingled
  with operational logs); HIL test verifies entries.
- Source: upstream design.md §Security Impact.
- Stakeholder: S4
- Priority: MVP
- Phase: 4

#### SEC-4.1 Session timeout
- Description: Idle SOVD sessions MUST expire after a configurable timeout
  (default 30 s, matching UDS S3 semantics). Expired session ids return 401.
- Rationale: ISO 14229 S3 timer parity; upstream design.md §Security Impact.
- Acceptance criteria: sleep 31 s then call → 401; rescue new session OK.
- Source: ISO 14229 §9.2; upstream design.md §Security Impact.
- Stakeholder: S4, S9
- Priority: MVP
- Phase: 4

#### SEC-5.1 Rate limiting on diagnostic endpoints
- Description: The SOVD Server MUST apply per-client-IP rate limits on all
  routes to prevent diagnostic flooding. Default: 20 rps.
- Rationale: MASTER-PLAN §4 Phase 6 deliverable 4; upstream design.md
  §Security Impact.
- Acceptance criteria: exceeding 20 rps yields 429; HIL test asserts.
- Source: MASTER-PLAN §4 Phase 6.
- Stakeholder: S4
- Priority: MVP
- Phase: 6

#### SEC-5.2 Input validation and size limits
- Description: All POST bodies (routine args, session bodies, etc.) MUST
  enforce maximum sizes (default 64 KiB) and strict schema validation.
- Rationale: hardening / defensive; standard REST practice.
- Acceptance criteria: oversized or schema-invalid bodies return 400; no
  memory growth.
- Source: upstream design.md §Security Impact.
- Stakeholder: S4
- Priority: MVP
- Phase: 4

---

## 7. Compliance Requirements

#### COMP-1.1 ASAM SOVD v1.1 OpenAPI implementation (MVP subset)
- Description: The SOVD Server and Gateway MUST implement the ASAM SOVD v1.1
  OpenAPI (ISO 17978-3) wire contract for the subset of resources specified
  in upstream mvp.md §Use-cases. Non-subset endpoints MAY return 501.
- Rationale: upstream mvp.md; MASTER-PLAN §B.1; the public Part 3 OpenAPI is
  the verifiable SOVD contract available to this project today.
- Acceptance criteria: an OpenAPI-schema-driven compatibility suite runs
  against the Server in SIL and `cargo xtask openapi-dump --check` stays
  green.
- Source: ISO 17978-3 OpenAPI template; upstream mvp.md.
- Phase: 4

#### COMP-2.1 ISO 14229 UDS compatibility via CDA
- Description: Legacy UDS services 0x10, 0x11, 0x14, 0x19, 0x22, 0x27, 0x31,
  0x3E MUST be reachable from SOVD through CDA on every Taktflow ECU (virtual
  directly, physical via Pi proxy).
- Rationale: MASTER-PLAN §3.1, upstream design.md §CDA.
- Acceptance criteria: HIL `hil_sovd_*` scenarios exercise each service.
- Source: ISO 14229; MASTER-PLAN §3.1.
- Phase: 1-5

#### COMP-3.1 ISO 26262 ASIL-D lifecycle preservation
- Description: SOVD integration MUST NOT degrade the existing ISO 26262 ASIL-D
  lifecycle of Taktflow firmware. Item definition, HARA, safety goals,
  FSC/TSC, HSI, and the safety case MUST be updated before release.
- Rationale: Taktflow `docs/safety/concept/*`; MASTER-PLAN §C.4.
- Acceptance criteria: Phase 6 exit criteria include safety-case delta
  approved by safety engineer.
- Source: ISO 26262; Taktflow safety docs.
- Phase: 0+

#### COMP-4.1 ASPICE L2-3 traceability
- Description: Requirements (this doc), design (ARCHITECTURE.md, ADRs), code,
  and tests MUST be linked via ASPICE-conformant traceability. Every FR/NFR
  traces to a design element and at least one test.
- Rationale: Taktflow `GOVERNANCE-SAFETY-ASPICE.md`; MASTER-PLAN §12.3.
- Acceptance criteria: `tools/traceability/` report shows 100% coverage for
  MVP requirements by the end of Phase 4.
- Source: Automotive SPICE PAM v4.0 SWE.1-6.
- Phase: 0+

#### COMP-5.1 Apache 2.0 + REUSE SPDX
- Description: Every source file in `opensovd-core` and the `fault-lib`
  embedded port MUST carry an SPDX header matching the Apache 2.0 license;
  REUSE tooling MUST pass clean.
- Rationale: upstream REUSE.toml; MASTER-PLAN §C.2a.
- Acceptance criteria: `reuse lint` CI gate green; `deny.toml` enforces
  license for crates.
- Source: Eclipse Foundation + REUSE spec; upstream `REUSE.toml`.
- Phase: 0+

#### COMP-5.2 Eclipse ECA signed
- Description: Every contributor to Taktflow OpenSOVD code MUST have a signed
  Eclipse Contributor Agreement before code lands.
- Rationale: MASTER-PLAN §15 item 9.
- Acceptance criteria: DevOps maintains an ECA register; PRs from
  non-ECA-signed authors are blocked.
- Source: Eclipse Foundation ECA policy.
- Phase: 0

---

## 8. Out of Scope

The following are explicitly out of scope for this project, even though they
may be valid future extensions:

- O-1: ECU flashing / software update (`Flash Service App` in upstream
  design.md §Out-of-scope components). Deferred indefinitely.
- O-2: AI/ML fault prediction or cloud-side analytics. Out of SOVD core.
- O-3: AUTOSAR Adaptive native diagnostic stack integration. Listed in
  upstream design.md §Open Issues — not our work.
- O-4: Upstream PRs during Phases 0-3 (MASTER-PLAN §C.1). A Phase 6 decision.
- O-5: Physical DoIP on STM32 / TMS570 (MASTER-PLAN §14 open questions —
  deferred). Physical ECUs are reached via the Pi CAN-to-DoIP proxy only.
- O-6: Automatic ODX ingestion from OEM standards. Taktflow writes ODX by
  hand for each ECU and commits the result (MASTER-PLAN §3.1).
- O-7: Off-the-shelf vehicle fleet back-office integration. Cloud
  integration is a future step, deferred to post-2026.
- O-8: Replacing existing Taktflow BSW modules (Dcm, Dem, NvM, etc.). We add
  new handlers inside existing modules; we do not rewrite them.
- O-9: S-CORE-scale safety of the Fault Library (upstream ADR-001: S-CORE
  owns ASIL-B; OpenSOVD remains QM). Taktflow does not push ASIL onto
  OpenSOVD code.
- O-10: UDS2SOVD Proxy (upstream `uds2sovd-proxy`). Not required for the MVP
  and not on the critical path. Deferred.

---

## 9. Open Questions

Open questions that directly affect requirements and must be resolved during
Phase 0 or early Phase 2. Re-stated from MASTER-PLAN §14 with requirements
context:

| ID | Question | Affects | Owner | Target |
|----|----------|---------|-------|--------|
| ~~OQ-1~~ | ~~Fault IPC transport: Unix socket vs. shared memory?~~ **Resolved by ADR-0002**: Unix socket on POSIX, NvM buffering on STM32 | FR-4.1, FR-4.2 | Rust lead | **Closed 2026-04-14** |
| ~~OQ-2~~ | ~~DFM persistence: SQLite vs. FlatBuffers file?~~ **Resolved by ADR-0003**: SQLite via sqlx, migration-based schema | FR-4.4 | Architect | **Closed 2026-04-14** |
| ~~OQ-3~~ | ~~ODX schema source~~ **Resolved by ADR-0008**: community XSD as default (ASAM is paywalled); ASAM XSD as pluggable override for members | FR-3.3, FR-5.3 | Embedded lead | **Closed 2026-04-14** |
| ~~OQ-4~~ | ~~Auth model~~ **Resolved by ADR-0009**: both OAuth2/OIDC bearer AND mTLS client certs, unified scope model | SEC-2.1, SEC-2.2 | Architect + security lead | **Closed 2026-04-14** |
| ~~OQ-5~~ | ~~DoIP discovery on Pi~~ **Resolved by ADR-0010**: both broadcast and static config, dual-mode default with drift warnings | FR-5.2 | Pi engineer | **Closed 2026-04-14** |
| ~~OQ-6~~ | ~~Physical DoIP on STM32~~ **Resolved by ADR-0011**: both lwIP and ThreadX NetX behind common C header, per-ECU build-time flag, implementation deferred post-MVP | O-5 (deferred) | Hardware lead | **Closed 2026-04-14** |
| ~~OQ-7~~ | ~~Operation-cycle API surface in DFM~~ **Resolved by ADR-0012**: both tester-driven (REST) and ECU-driven (Fault Shim IPC) feeding the same state machine | FR-4.3 | Architect | **Closed 2026-04-14** |
| ~~OQ-8~~ | ~~Correlation-id header name~~ **Resolved by ADR-0013**: accept both `X-Request-Id` and `traceparent`, synthesize one from the other when only one is present | NFR-3.3 | Rust lead | **Closed 2026-04-14** |
| ~~OQ-9~~ | ~~Audit log sink~~ **Resolved by ADR-0014**: all three (SQLite table, append-only file, DLT channel), fan-out with at-least-one success semantics | SEC-3.1 | Architect | **Closed 2026-04-14** |

Unresolved items that affect a requirement are marked in that requirement's
Acceptance criteria; the requirement cannot exit draft until resolved.

---

## 10. Revision history

| Rev | Date | Author | Change |
|-----|------|--------|--------|
| 1.0 | 2026-04-14 | SOVD workstream architect | Initial draft. Seeded from MASTER-PLAN rev 2026-04-14, upstream design.md, upstream mvp.md, ADR-001, opensovd-core ARCHITECTURE.md, Taktflow safety goals, Taktflow Dcm walkthrough. |
