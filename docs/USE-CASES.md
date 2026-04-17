<!--
SPDX-FileCopyrightText: 2026 The Contributors to Eclipse OpenSOVD (Taktflow fork)
SPDX-License-Identifier: Apache-2.0
-->

# Taktflow Eclipse OpenSOVD -- Use Case Catalog

- Document ID: TAKTFLOW-SOVD-UC
- Revision: 1.0
- Status: Draft
- Date: 2026-04-17
- Owner: Taktflow SOVD workstream

## Purpose

Canonical catalog of every user-visible capability of the Taktflow
OpenSOVD diagnostic stack. Each use case has a stable ID (UC1 through
UC20) that other documents, tests, dashboard widgets, HIL scenarios,
and ADRs reference. If a capability is not listed here, it is either
out of scope or needs to be added to this document first.

### How this catalog is organized

- **§2 Primary use cases (UC1-UC5)** -- the MVP set. Each has a full
  sequence diagram in [ARCHITECTURE.md §6](ARCHITECTURE.md#6-runtime-view).
  These are the five flows Eclipse OpenSOVD upstream MVP defined.
- **§3 Extended use cases (UC6-UC20)** -- the capability showcase set
  introduced by the Phase 5 observer dashboard work (ADR-0024).
  These exercise additional SOVD surface beyond the MVP.
- **§4 Traceability matrix** -- UC × Requirement × Test × Dashboard
  widget. One row per use case.
- **§5 Related documents** -- where each use case is specified,
  implemented, and verified.

### Convention

Every use case carries:

- **ID** -- stable, never renumbered
- **Actor** -- who initiates
- **Goal** -- one sentence
- **Main flow** -- 2-4 bullets
- **Requirements** -- FR/NFR/SR/SEC IDs from
  [REQUIREMENTS.md](REQUIREMENTS.md)
- **Verified by** -- test name (unit / integration / HIL) or "not yet"
- **Dashboard widget** -- SvelteKit component name (see ADR-0024)

---

## 1. Actors

| Actor | Description |
|-------|-------------|
| **Off-board tester** | Primary human or automated client using `curl`, Postman, or a purpose-built SOVD client. Addresses the Pi gateway over HTTPS. |
| **Fleet / cloud operator** | Future actor; post-2026 full-fleet cloud integration. Partial scope landed via ADR-0024 for HIL observer. |
| **On-board app / platform code** | Firmware software component (SWC) calling `FaultShim_Report` to report a fault observation. |
| **Observer** | Stakeholder watching the bench dashboard; authenticated via mTLS per ADR-0024. Read-only + triggers actions authorized to their identity. |
| **Taktflow firmware** | Existing AUTOSAR-like BSW. Dcm and Dem are the touchpoints. |
| **S-CORE (future)** | Consumes the Fault Library per ADR-SCORE; integration post-MVP. |

---

## 2. Primary Use Cases (MVP)

These are the five flows the Eclipse OpenSOVD MVP defined and that the
Taktflow project committed to delivering in Phase 5. Each has a full
sequence diagram in ARCHITECTURE.md §6.

### UC1 -- Read DTCs via SOVD

- **Actor**: Off-board tester
- **Goal**: List active diagnostic trouble codes for a specific ECU,
  optionally filtered by status mask.
- **Main flow**:
  1. Tester issues `GET /sovd/v1/components/{id}/faults?status-mask=0x08`.
  2. Gateway routes to the correct backend (CDA, DFM, or native).
  3. CDA path: UDS 0x19 over DoIP over CAN ISO-TP reaches the ECU.
  4. JSON DTC list returns to tester within P99 500 ms.
- **Requirements**: FR-1.1, FR-1.5, NFR-1.1
- **Verified by**: unit tests + `hil_sovd_01_read_faults_all.yaml`
- **Dashboard widget**: `UC01DtcList.svelte`
- **Sequence diagram**: [ARCHITECTURE.md §6.1](ARCHITECTURE.md#61-uc1--read-dtcs-via-sovd-req-fr-11-fr-15-ud-sovd-server)

### UC2 -- Report fault via Fault API

- **Actor**: On-board app / platform code
- **Goal**: An ECU-local software component reports a fault observation
  through a framework-agnostic, non-blocking API.
- **Main flow**:
  1. SWC calls `FaultShim_Report(fid, severity, meta)`; returns in
     bounded time regardless of DFM availability (SR-4.1).
  2. Shim serializes a `FaultRecord`; POSIX uses Unix socket, STM32
     buffers to NvM.
  3. DFM pipeline: `FaultListener` → `Debouncer` → `OperationCycle` →
     `DtcLifecycle` → SQLite.
  4. DTC visible via `GET /sovd/v1/components/{id}/faults` within 100 ms.
- **Requirements**: FR-4.1, FR-4.2, FR-4.3, NFR-1.2, SR-4.1, SR-4.2
- **Verified by**: `phase3_dfm_sqlite_roundtrip.rs`
- **Dashboard widget**: `UC11FaultPipeline.svelte` visualizes the chain
- **Sequence diagram**: [ARCHITECTURE.md §6.2](ARCHITECTURE.md#62-uc2--report-fault-via-fault-api-req-fr-41-fr-42-fr-43-ud-fault-library)

### UC3 -- Clear DTCs

- **Actor**: Off-board tester (authenticated)
- **Goal**: Clear all DTCs for an ECU, or only DTCs of a given group.
- **Main flow**:
  1. Tester `POST /sovd/v1/components/{id}/faults/clear` with optional
     `?group=<code>`.
  2. Auth middleware verifies mTLS cert + bearer token.
  3. Audit log records the operation (SEC-3.1).
  4. CDA path: UDS 0x14 ClearDiagnosticInformation reaches the ECU.
  5. ECU clears its fault memory; 204 No Content returned.
- **Requirements**: FR-1.3, SEC-2.1, SEC-2.2, SEC-3.1, COMP-2.1
- **Verified by**: `hil_sovd_02_clear_faults.yaml`
- **Dashboard widget**: `UC03ClearFaults.svelte`
- **Sequence diagram**: [ARCHITECTURE.md §6.3](ARCHITECTURE.md#63-uc3--clear-dtcs-req-fr-13-umvp-use-cases-3)

### UC4 -- Reach a UDS ECU via CDA

- **Actor**: Off-board tester
- **Goal**: Reach a legacy UDS-only ECU through SOVD, whether the ECU
  is virtual (POSIX DoIP) or physical (STM32 + CAN-to-DoIP proxy).
- **Main flow (virtual UC4a)**:
  1. Tester hits SOVD endpoint for a POSIX-backed ECU.
  2. Gateway routes to CDA backend.
  3. CDA speaks DoIP directly to the POSIX container on :13400.
- **Main flow (physical UC4b)**:
  1. Tester hits SOVD endpoint for an STM32 ECU.
  2. Gateway routes to CDA backend.
  3. CDA -> CAN-to-DoIP proxy on Pi :13401.
  4. Proxy translates DoIP to CAN ISO-TP frames at 500 kbps.
- **Requirements**: FR-5.1, FR-5.2, FR-5.3, FR-5.4
- **Verified by**: `phase2_cda_ecusim_smoke.rs`, HIL CAN captures
- **Dashboard widget**: `UC14CdaTopology.svelte`
- **Sequence diagram**: [ARCHITECTURE.md §6.4](ARCHITECTURE.md#64-uc4--reach-a-uds-ecu-via-cda-req-fr-51-fr-52)

### UC5 -- Trigger diagnostic routine

- **Actor**: Off-board tester (authenticated, may require elevated
  security level)
- **Goal**: Execute a diagnostic routine (motor self-test, brake check,
  etc.) with safety interlocks enforced in firmware.
- **Main flow**:
  1. Tester `POST /sovd/v1/components/{id}/operations/{rid}/start` with
     arg bytes.
  2. Auth + audit.
  3. CDA issues UDS 0x31 01 RoutineControl.
  4. ECU firmware validates preconditions (SR-3.1 motor stationary,
     SR-3.2 test-mode session). If not met: NRC 0x22, SOVD returns 409.
  5. If OK: routine starts; tester polls
     `GET .../executions/{execution_id}` for status.
- **Requirements**: FR-2.1, FR-2.2, FR-2.3, SR-3.1, SR-3.2
- **Verified by**: `hil_sovd_03_operation_execution.yaml`
- **Dashboard widget**: `UC06Operations.svelte`
- **Sequence diagram**: [ARCHITECTURE.md §6.5](ARCHITECTURE.md#65-uc5--trigger-diagnostic-routine-req-fr-21-fr-23)

---

## 3. Extended Use Cases (capability showcase)

Additional use cases beyond the MVP that the Phase 5 observer dashboard
(ADR-0024) exposes. These exercise SOVD surface introduced in
Requirements §3 and §4 but not part of the upstream MVP demo set.

### UC6 -- Start / stop / poll routines

(Same actor + flow as UC5. Listed here as a distinct dashboard widget
because the dashboard gives a persistent control surface for the
routine lifecycle, not just a one-shot invocation.)

- **Requirements**: FR-2.1, FR-2.2, FR-2.3
- **Verified by**: same as UC5
- **Dashboard widget**: `UC06Operations.svelte`

### UC7 -- Routine catalog discovery

- **Actor**: Off-board tester / observer
- **Goal**: Discover the set of routines supported by each ECU without
  reading ECU source.
- **Main flow**: `GET /sovd/v1/components/{id}/operations` returns the
  routine catalogue derived from MDD, including ids, names, arg
  schemas, return schemas.
- **Requirements**: FR-2.4
- **Verified by**: unit tests
- **Dashboard widget**: `UC07RoutineCatalog.svelte`

### UC8 -- List components with capability badges

- **Actor**: Off-board tester / observer
- **Goal**: Enumerate every registered ECU with kind, backing source,
  reachability, and capability flags.
- **Main flow**: `GET /sovd/v1/components` returns an ordered list with
  one entry per registered backend. Capability flags (`faults`,
  `operations`, `data`, `modes`) are derived from MDD + backend kind at
  registration time.
- **Requirements**: FR-3.1, FR-3.4
- **Verified by**: `hil_sovd_05_components_metadata.yaml`
- **Dashboard widget**: `UC08ComponentCards.svelte`

### UC9 -- Component HW / SW version

- **Actor**: Off-board tester / observer
- **Goal**: Read hardware revision, software version, and serial of a
  specific ECU.
- **Main flow**: `GET /sovd/v1/components/{id}` returns `ComponentInfo`.
  CDA path reads UDS DIDs 0xF18C (HW serial) and 0xF195 (SW version);
  native path reads the Fault Library catalogue version.
- **Requirements**: FR-3.2
- **Verified by**: `hil_sovd_05_components_metadata.yaml`
- **Dashboard widget**: `UC09HwSwVersion.svelte`

### UC10 -- Read data identifier (DID)

- **Actor**: Off-board tester / observer
- **Goal**: Read a live ECU value (VIN, battery voltage, temperature)
  by DID.
- **Main flow**: `GET /sovd/v1/components/{id}/data` lists available
  DIDs; `GET .../data/{did}` reads a single value. CDA-backed reads
  translate to UDS 0x22 ReadDataByIdentifier.
- **Requirements**: FR-3.3, COMP-2.1
- **Verified by**: unit tests
- **Dashboard widget**: `UC10LiveDidReads.svelte` (1 Hz poll)

### UC11 -- Visualize the fault pipeline

- **Actor**: Observer
- **Goal**: See the five-stage DFM pipeline animate as a fault traverses
  it: shim → FaultListener → Debouncer → OperationCycle → DtcLifecycle → SQLite.
- **Main flow**: WebSocket stream from ws_bridge carries pipeline events;
  dashboard animates the stage transitions in real time.
- **Requirements**: FR-4.1, FR-4.2, FR-4.3 (visualization only; no new
  functional requirement)
- **Verified by**: visual inspection; stream integrity via
  `fault-sink-mqtt` snapshot test
- **Dashboard widget**: `UC11FaultPipeline.svelte`

### UC12 -- Operation cycle state

- **Actor**: Observer / tester
- **Goal**: Show and optionally control the current operation cycle
  state (Idle / Running / Evaluating) per ADR-0012.
- **Main flow**: State machine visualization with current state
  highlighted. Cycle can be triggered by REST (`POST
  .../operation-cycles/start`) for tester-driven cycles or via Fault
  Shim IPC for ECU-driven cycles.
- **Requirements**: FR-4.3
- **Verified by**: unit tests on `opcycle-taktflow` crate
- **Dashboard widget**: `UC12OperationCycle.svelte`

### UC13 -- DTC lifecycle visualization

- **Actor**: Observer
- **Goal**: Show the state of each DTC on the Pending → Confirmed →
  Cleared axis, with Suppressed shown as an off-axis state.
- **Main flow**: Each DTC row in the dashboard shows its current state,
  with animated transitions when state changes arrive via WebSocket.
- **Requirements**: System spec §6.1 (DTC lifecycle)
- **Verified by**: unit tests on `DtcLifecycle`
- **Dashboard widget**: `UC13DtcLifecycle.svelte`

### UC14 -- CDA topology view

- **Actor**: Observer
- **Goal**: See the full request path from tester to ECU and which hops
  are active for a given component (tester → gateway → CDA → proxy →
  ECU, or shorter variants for virtual ECUs).
- **Main flow**: Read-only visualization; highlights the current path
  and shows health status per hop.
- **Requirements**: FR-5.1, FR-5.2, FR-6.1
- **Verified by**: manual inspection during HIL runs
- **Dashboard widget**: `UC14CdaTopology.svelte`

### UC15 -- Session management

- **Actor**: Off-board tester
- **Goal**: Create a SOVD session, query its security level, observe
  timeout, optionally elevate security level for privileged ops.
- **Main flow**:
  1. `POST /sovd/v1/sessions` creates a session id.
  2. Subsequent calls carry `X-SOVD-Session: <id>` header.
  3. Idle sessions expire after 30 s default (SEC-4.1).
  4. Elevation requires token exchange / cert / seed-key per ADR-0009.
- **Requirements**: FR-7.1, FR-7.2, SEC-2.1, SEC-2.2, SEC-4.1
- **Verified by**: unit tests
- **Dashboard widget**: `UC15Session.svelte` (countdown timer)

### UC16 -- Audit log

- **Actor**: Observer / security officer
- **Goal**: See a tamper-resistant append-only log of privileged
  operations: clear DTCs, start routine, write DID, session elevate.
- **Main flow**: ADR-0014 fan-out sink -- SQLite table + append-only
  file + DLT channel -- at-least-one success semantics. Dashboard
  streams entries as they arrive.
- **Requirements**: SEC-3.1, ADR-0014
- **Verified by**: manual inspection of audit sink
- **Dashboard widget**: `UC16AuditLog.svelte`

### UC17 -- Safety boundary indicator

- **Actor**: Observer / safety engineer
- **Goal**: See at a glance whether the safety boundary is healthy:
  Fault Library responsive, ASIL-D isolation intact, no SOVD path has
  breached the one-way flow contract.
- **Main flow**: Health-check signals from the firmware-side Fault Lib
  (via IPC heartbeats) and runtime checks on sovd-main report to the
  dashboard. Any boundary anomaly raises a red status light.
- **Requirements**: SR-1.1, SR-1.2, SR-4.1, SR-4.2
- **Verified by**: `phase3_dfm_sqlite_roundtrip.rs` + DFM crash/recovery
  integration test
- **Dashboard widget**: `UC17SafetyBoundary.svelte`

### UC18 -- Gateway routing and fan-out

- **Actor**: Observer
- **Goal**: See which backends the Gateway has registered, which are
  reachable, which are dead, which are local vs federated.
- **Main flow**: `GET /sovd/v1/components` extras carry health +
  routing information; dashboard renders the gateway's current backend
  inventory.
- **Requirements**: FR-6.1, FR-6.2, NFR-2.1, NFR-2.2
- **Verified by**: `phase4_sovd_gateway_cda_ecusim_bench.rs`
- **Dashboard widget**: `UC18GatewayRouting.svelte`

### UC19 -- Historical trends

- **Actor**: Observer / integrator
- **Goal**: Query fault occurrence rates, latency distributions, and
  routine success rates over the last 7 days.
- **Main flow**: Prometheus scrapes sovd-main, cloud_connector, and
  ws_bridge metrics; Grafana dashboard renders the history; observer
  sees the Grafana panel embedded in the main dashboard.
- **Requirements**: NFR-3.1, NFR-3.2, NFR-3.3
- **Verified by**: Grafana panel displaying live data during HIL runs
- **Dashboard widget**: `UC19Historical.svelte` (iframe to Grafana)

### UC20 -- Concurrent tester support

- **Actor**: Off-board tester (multiple)
- **Goal**: Two or more testers exercise the SOVD Server concurrently
  without request reordering or cross-contamination.
- **Main flow**: Testers issue independent request sequences against
  the same Pi sovd-main instance; dashboard shows current connected
  clients count and any cross-session contention.
- **Requirements**: NFR-1.3
- **Verified by**: `hil_sovd_06_concurrent_testers.yaml`
- **Dashboard widget**: `UC20ConcurrentTesters.svelte` (footer strip)

---

## 4. Traceability Matrix

One row per use case mapping UC ID to primary requirement, test, and
dashboard widget. Complete traceability between UC, REQ, test, and
implementation lives in `tools/traceability/` (per COMP-4.1) and is
the source of truth; this table is a convenience index.

| UC | Title | Primary REQ | Primary Test | Dashboard Widget |
|----|-------|-------------|--------------|-----------------|
| UC1 | Read DTCs | FR-1.1 | `hil_sovd_01_read_faults_all.yaml` | `UC01DtcList.svelte` |
| UC2 | Report fault | FR-4.1 | `phase3_dfm_sqlite_roundtrip.rs` | `UC11FaultPipeline.svelte` |
| UC3 | Clear DTCs | FR-1.3 | `hil_sovd_02_clear_faults.yaml` | `UC03ClearFaults.svelte` |
| UC4 | Reach UDS ECU | FR-5.1, FR-5.2 | `phase2_cda_ecusim_smoke.rs` | `UC14CdaTopology.svelte` |
| UC5 | Trigger routine | FR-2.1 | `hil_sovd_03_operation_execution.yaml` | `UC06Operations.svelte` |
| UC6 | Start/stop/poll routine | FR-2.1-2.3 | same as UC5 | `UC06Operations.svelte` |
| UC7 | Routine catalog | FR-2.4 | unit tests | `UC07RoutineCatalog.svelte` |
| UC8 | List components | FR-3.1, FR-3.4 | `hil_sovd_05_components_metadata.yaml` | `UC08ComponentCards.svelte` |
| UC9 | HW/SW version | FR-3.2 | `hil_sovd_05_components_metadata.yaml` | `UC09HwSwVersion.svelte` |
| UC10 | Read DID | FR-3.3 | unit tests | `UC10LiveDidReads.svelte` |
| UC11 | Fault pipeline viz | FR-4.x | `phase3_dfm_sqlite_roundtrip.rs` | `UC11FaultPipeline.svelte` |
| UC12 | Operation cycle state | FR-4.3 | unit tests on opcycle-taktflow | `UC12OperationCycle.svelte` |
| UC13 | DTC lifecycle viz | §6.1 system spec | unit tests on DtcLifecycle | `UC13DtcLifecycle.svelte` |
| UC14 | CDA topology | FR-5.1, FR-5.2 | manual during HIL | `UC14CdaTopology.svelte` |
| UC15 | Session management | FR-7.1, SEC-4.1 | unit tests | `UC15Session.svelte` |
| UC16 | Audit log | SEC-3.1, ADR-0014 | manual audit sink check | `UC16AuditLog.svelte` |
| UC17 | Safety boundary | SR-1.x, SR-4.x | DFM crash/recovery test | `UC17SafetyBoundary.svelte` |
| UC18 | Gateway routing | FR-6.x, NFR-2.x | `phase4_sovd_gateway_cda_ecusim_bench.rs` | `UC18GatewayRouting.svelte` |
| UC19 | Historical trends | NFR-3.x | Grafana live on HIL | `UC19Historical.svelte` |
| UC20 | Concurrent testers | NFR-1.3 | `hil_sovd_06_concurrent_testers.yaml` | `UC20ConcurrentTesters.svelte` |

---

## 5. Related Documents

| Document | Purpose for use cases |
|----------|----------------------|
| [REQUIREMENTS.md](REQUIREMENTS.md) | FR / NFR / SR / SEC / COMP definitions that each UC exercises |
| [ARCHITECTURE.md](ARCHITECTURE.md) §6 | Full sequence diagrams for UC1-UC5 |
| [SYSTEM-SPECIFICATION.md](SYSTEM-SPECIFICATION.md) §7 | Condensed visual view of UC1, UC2, UC3, UC5 |
| [adr/0024-*.md](adr/0024-reuse-embedded-production-cloud-connector.md) | Dashboard widget catalog (UC1-UC20) and tech stack |
| `opensovd/docs/design/mvp.md` | Upstream Eclipse OpenSOVD MVP use cases (source of UC1-UC5) |
| `opensovd-core/integration-tests/tests/` | Integration test implementations |
| `opensovd-core/test/hil/scenarios/` | HIL scenario YAMLs |
| `dashboard/src/lib/widgets/` | Dashboard widget implementations (one file per UC) |

---

## 6. Scope Notes

### What's in this catalog

Every user-visible capability of the Taktflow OpenSOVD stack as of Phase
5, including:

- Functional SOVD endpoints (UC1, UC3, UC6-10)
- Internal pipelines surfaced through the observer dashboard (UC2,
  UC11-13, UC17)
- Cross-cutting concerns with user-facing effects (UC14, UC16, UC18,
  UC19, UC20)
- Session + security mechanics (UC15)

### What's deliberately NOT in this catalog

- **Upstreaming to Eclipse OpenSOVD** -- a process, not a user-visible
  capability. Tracked in MASTER-PLAN §8.
- **Upstream sync workflow** (weekly rebases) -- internal process.
- **ECU flashing / software update** -- out of scope per REQ §8 O-1.
- **AI/ML fault prediction** -- out of scope per REQ §8 O-2.
- **AUTOSAR Adaptive native diagnostic** -- out of scope per REQ §8 O-3.
- **Physical DoIP on STM32 / TMS570** -- deferred per ADR-0011 + REQ §8 O-5.
- **UDS2SOVD Proxy** -- not on critical path, scaffolded only, REQ §8 O-10.
- **Full fleet cloud integration** -- partially unblocked by ADR-0024
  Stage 2, but fleet-scale management remains post-2026 (REQ §8 O-7).

### How to add a new use case

1. Pick the next free UC ID (UC21+).
2. Write the use case with the same shape as those above.
3. Add a row to the traceability matrix in §4.
4. If it's a primary MVP flow, also add a sequence diagram to
   [ARCHITECTURE.md §6](ARCHITECTURE.md#6-runtime-view).
5. Add (or update) the dashboard widget under
   `dashboard/src/lib/widgets/` and wire it into `src/routes/+page.svelte`.
6. Reference the UC ID from the requirement(s) it exercises in
   [REQUIREMENTS.md](REQUIREMENTS.md).

---

## 7. Revision History

| Rev | Date | Author | Change |
|-----|------|--------|--------|
| 1.0 | 2026-04-17 | SOVD workstream | Initial catalog. Consolidates the 5 MVP use cases from ARCHITECTURE.md §6 and the 20-use-case capability showcase set from ADR-0024 into a single canonical reference with full traceability. |
