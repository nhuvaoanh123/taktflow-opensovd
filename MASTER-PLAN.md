# Eclipse OpenSOVD — Taktflow End-to-End Master Plan

**Owner:** Taktflow SOVD workstream
**Target:** Full OpenSOVD MVP running on Taktflow hardware, end of 2026
**Status:** Planning — 2026-04-14
**Team size:** 20 (Taktflow-wide, SOVD workstream draws a subset)

---

## A. What is OpenSOVD (and why it exists)

**SOVD** = Service-Oriented Vehicle Diagnostics, standardized as **ISO 17978** by ASAM.
It is the modern replacement for UDS (ISO 14229), the 40-year-old byte-level diagnostic
protocol that runs on CAN.

**The shift SOVD represents:**

| Dimension | UDS (legacy) | SOVD (modern) |
|-----------|-------------|---------------|
| Transport | CAN + ISO-TP (or DoIP) | REST/HTTP over IP |
| Data format | Binary byte frames | JSON resources |
| Addressing | Session + service IDs | URL paths |
| Security | Seed/key, minimal | HTTPS + certificates + OAuth |
| Topology | Point-to-point tester | Distributed, cloud-connected |
| Tooling | Specialized diag tools | Any HTTP client (curl, Postman, browser) |

**Why the industry is moving to SOVD:**
- Modern vehicles have IP networks internally (zonal architectures, Ethernet backbones)
- OEMs need cloud fleet diagnostics, OTA update feedback, AI/ML fault analysis
- Binary UDS does not fit structured-data needs of those workflows
- Service-oriented architectures (SOA, SDV) require service-oriented diagnostics

**What OpenSOVD provides concretely:**
- `GET /sovd/v1/components/{id}/faults` — read faults (SOVD term; UDS layer still calls them DTCs)
- `DELETE /sovd/v1/components/{id}/faults` — clear fault memory for a component
- `POST /sovd/v1/components/{id}/operations/{op_id}/executions` — trigger diagnostic operations (SOVD term; UDS layer calls them routines)
- `GET /sovd/v1/components/{id}` — read ECU metadata (HW/SW versions, config)
- REST-based software update and calibration flows

**Eclipse OpenSOVD** is the open-source reference implementation of ISO 17978 under the
Eclipse Automotive umbrella, and has been designated by **Eclipse S-CORE** (the SDV reference
OS / middleware stack) as its diagnostic layer. OpenSOVD MVP target is **end of 2026** for
S-CORE v1.0 integration.

**Legacy compatibility:** OpenSOVD includes the `Classic Diagnostic Adapter` (CDA) that
translates SOVD REST calls into UDS/DoIP for ECUs that still only speak UDS. That is why
our plan includes a CAN-to-DoIP proxy on the Raspberry Pi — it lets the CDA reach
Taktflow's physical CAN-only STM32 ECUs.

---

## B. Why Taktflow is doing this

Four layers, stacked from immediate to strategic. Later work should always trace back to
one of these.

### B.1 Technical purpose — SOVD for Taktflow

Taktflow's embedded platform speaks UDS over CAN today. Adding SOVD means every Taktflow
ECU becomes reachable via modern REST diagnostics. This is a capability gap that every
customer will hit within 2–3 years.

### B.2 Product purpose — customer value

Taktflow is a multi-customer BMS platform serving 20+ T1/T2 automotive customers. OEMs are
moving toward SOVD as the diagnostic standard. A Taktflow that speaks SOVD natively is
more valuable to those customers than one that only speaks legacy UDS — and cheaper to
integrate into their service-oriented architectures.

### B.3 Strategic purpose — Eclipse SDV credibility (shadow-ninja)

Eclipse OpenSOVD's `opensovd-core` repository is currently an **empty stub**. Whoever
lands the first real code there becomes the de facto implementer of the SOVD Server,
Gateway, DFM, and Diagnostic DB. This is the **largest missing piece in the entire Eclipse
OpenSOVD project** and the single highest-leverage spot to land contributions.

Taktflow's strategy is shadow-ninja: passive public visibility, never ping maintainers,
let the work speak. Owning `opensovd-core` scaffolding is as much credibility as a team
can buy in Eclipse SDV in under a year.

### B.4 Tactical purpose — own the code before sharing it

Build it ourselves first. No upstream PRs in the early phases. No dependency on upstream
maintainer responsiveness, no design-by-committee, no churn from review feedback on
half-built code. When we upstream, we upstream finished, tested, working systems.

---

## C. How we build it — core principles (non-negotiable)

These principles govern every task, phase, and decision in this plan. When in doubt,
check against these.

### C.1 Build first, contribute later
- No upstream PRs during Phases 0–3
- Upstream PRs begin only when we have a working end-to-end stack (Phase 4+)
- Local feature branches, local CI, local verification — nothing pushed to our forks
  unless explicitly approved

### C.2 Follow upstream, build extras on top, sync later

**The model:** we are a downstream fork that continuously tracks upstream CDA/opensovd-core.
Taktflow's code is **extras layered on top** of the upstream baseline, not a rewrite beside it.

Three rules:

**C.2a — Mirror upstream wholesale, not selectively.**
- Copy CDA's `build.yml`, feature flags, patches, deny.toml, workflows VERBATIM
- Do not cherry-pick "what we need now" — copy everything, let CI fail on unimplemented
  parts. Broken CI is our TDD-style punch list; each failure tells us what to build next
- Features we don't use yet stay in the file but gated off
- Deps we don't use yet stay declared in `[workspace.dependencies]`
- The goal: any upstream diff `git diff upstream/main -- <mirrored-files>` is ≈0

**C.2b — Taktflow extras go in clearly separated layers.**
- Our additions live in distinct crates (`sovd-client`, `sovd-dfm`, etc.) or clearly
  labeled modules, never inline-edits inside mirrored code
- Commit message convention:
  - `mirror(<area>): ...` when the change reflects upstream content
  - `feat(<crate>): ...` / `feat(sovd-taktflow): ...` when the change is our own extra
  - `sync(upstream): rebase on <upstream-sha>` for rebase commits
- Rule of thumb: if upstream could adopt our change verbatim and it would make sense in
  their codebase, it's a `feat:`; if it only makes sense on top of upstream, it's an
  `extra:` or `taktflow:` scope

**C.2c — Weekly upstream sync, never drift more than a week.**
- `upstream-sync.yml` runs Monday 09:00: `git fetch upstream` → attempt rebase → open
  internal issue if conflicts
- Architect resolves conflicts within 24h; rebase is never deferred
- After sync, re-run local verification gates (build / test / clippy / fmt)
- If upstream adds a feature we were about to build ourselves, STOP our work and adopt
  theirs — we are downstream, not parallel

**Why this model:**
- When we finally upstream, the diff is the Taktflow extras and nothing else — minimal,
  reviewable, unambiguous
- We never re-invent anything upstream already has
- We never diverge so far that upstreaming becomes impossible
- Our CI failures map 1:1 to our implementation backlog — no separate todo tracking needed

### C.3 Concrete before abstract
- Build for the first target (CVC virtual ECU in Docker), then extend
- Do not design generic abstractions until we have the concrete case working
- Do not invent roles or types not present in the upstream design document

### C.4 Safety never slips
- No changes to safety-critical code paths without updated HARA
- MISRA C:2012 clean on all new embedded code (ASIL-D lifecycle is in force)
- Every new Dcm service goes through the safety engineer before merge

### C.5 Test gates everything
- New module → unit tests before merge
- New Rust code → pedantic clippy + nightly fmt must pass
- New firmware → SIL and HIL must be green before merge to main
- No exceptions, even for "quick fixes"

### C.6 Core + peripheral mindset (from Taktflow org principles)
- Core (opensovd-core Rust workspace) is domain-agnostic
- Taktflow-specific pieces (ODX files, ECU configs, safety case, DBC) are peripheral
- This plan's output can be reused by other Taktflow customers with different ECUs

### C.7 Never hard fail (from upstream CDA house style, Rust SIG 2026-04-14)
- Backend impls log-and-continue on unexpected downstream responses; they do not drop sessions or bail on the first surprise
- Spec-boundary rejection (malformed incoming REST) stays strict
- Lock acquisitions on shared state use `try_lock_for(Duration)` with a bounded budget, not blocking locks
- No `panic!` / `unwrap()` / `expect()` in any function reachable from a live HTTP handler; CI enforces via clippy lints on the backend crates
- Degraded responses carry a `stale: true` flag in extras and an explicit `error_kind` label in structured logs
- Formal spec: ADR-0018. Rationale: upstream CDA tested aggressive error propagation in realistic environments and found it breaks; we copy the behavior, not their prose.

### C.8 Upstream house style before custom patterns
- When upstream CDA (`eclipse-opensovd/classic-diagnostic-adapter`) has a convention we don't, the default is to adopt it, not invent our own
- Specific upstream conventions we track (from Rust SIG 2026-04-14, see `~/.claude/projects/h--/memory/project_opensovd_upstream_design.md`):
  - MDD = "Marvelous Diagnostic Description" — FlatBuffers-wrapped restructured ODX, memory-mappable, ~1/3 the size of ODX
  - Generics over dynamic dispatch; only the security plugin is `dyn Trait`
  - `tokio::io::split()` on DoIP TCP streams for concurrent sender/receiver tasks
  - `doip-codec` crate for DoIP wire definitions — spike complete: **PARTIAL migration** in Phase 5 Line B. Upstream CDA pins the `theswiftfox/doip-codec` fork at rev `0dba319` + `theswiftfox/doip-definitions` rev `bdeab8c` (NOT the crates.io version). Wire format matches `DoIp_Posix.c` and our hand-rolled `proxy-doip/src/frame.rs` byte-for-byte. Action: replace `frame.rs` + `message_types.rs` with the fork, keep `server.rs` + `DoipHandler` trait + ISO-TP FC integration (from PR #9) + ADR-0010 discovery logic. See `docs/doip-codec-evaluation.md`.
  - mbedtls as the TLS backend escape hatch when OpenSSL hits a feature wall
  - `tokio-console` as the deadlock / large-task debugging tool
- When we deviate (e.g. our `Arc<dyn Trait>` + TOML runtime dispatch in ADR-0016), we document the deviation and its rationale in the relevant ADR

---

## 0. Executive Summary

We will build the Eclipse OpenSOVD stack end-to-end on top of the existing Taktflow embedded
platform. The existing firmware already has ~60% of the UDS services OpenSOVD needs. The
OpenSOVD upstream has started the Classic Diagnostic Adapter but the SOVD Server itself
(`opensovd-core`) is an empty stub. This plan takes both pieces and closes the gap — built
in our own fork first, upstreamed only when ready.

**Outcome by end of 2026:**
- All 7 Taktflow ECUs reachable via SOVD REST API
- Full DTC read/clear/trigger service flow working end-to-end
- `opensovd-core` (Server, Gateway, DFM, Diag DB) built in our fork, fully working
- Docker Compose demo equivalent to the OpenSOVD Q3 upstream milestone
- HIL test suite on Raspberry Pi bench gates every commit to our main
- Ready (but not obligated) to upstream finished components when the team decides

**Critical path:**
`Dcm 0x19/0x14 → DoIP POSIX → CDA smoke test → Fault shim → DFM → SOVD Server → Gateway → E2E demo → Hardening`

**Non-negotiables:** see §C above. Every principle applies to every phase.

---

## 1. Current State

### 1.1 Taktflow embedded baseline (what we have)

| Layer | Status |
|-------|--------|
| Hardware | 3× STM32G474RE + 1× TMS570LC43x + 3× Docker POSIX + Raspberry Pi gateway |
| BSW | AUTOSAR-like: MCAL / CanIf / PduR / Com / Dcm / Dem / E2E / CanTp / WdgM / NvM |
| Safety | ISO 26262 ASIL-D lifecycle, ASPICE L2-3, MISRA clean |
| Diagnostics (UDS) | 0x10, 0x11, 0x22, 0x27, 0x3E implemented; Dem has DTC table |
| Transport | CAN 2.0B @ 500 kbps + ISO-TP (CanTp) |
| Gateway | Raspberry Pi with CAN ↔ MQTT bridge, skeleton diagnostics server |
| CI/CD | 7 pipelines including nightly SIL + HIL |
| Codegen | DBC → ARXML → C configs (Com, Rte, CanIf, PduR, E2E) |

### 1.2 Eclipse OpenSOVD upstream (what upstream has)

| Repo | State | Usefulness to us |
|------|-------|------------------|
| `classic-diagnostic-adapter` | Active, ~MVP-ready | Reusable as-is for the SOVD→UDS bridge |
| `odx-converter` | Active | Reusable for ECU description conversion |
| `fault-lib` | Alpha | Reference for Fault API shape; we port to C |
| `dlt-tracing-lib` | Active | Reusable for observability |
| `uds2sovd-proxy` | Early | Optional — only if we need legacy tester compat |
| `cpp-bindings` | Stub | We grow this for C/C++ integration |
| `opensovd-core` | **Empty stub** | We build this from scratch |
| `opensovd` | Active (docs) | Where we upstream architecture decisions |

### 1.3 Eclipse SDV context

OpenSOVD is the designated diagnostic layer for Eclipse S-CORE (SDV reference stack). The
S-CORE v1.0 target is end of 2026. ADR-001 makes the Fault Library the organizational
boundary between the two projects.

---

## 2. Target Architecture

### 2.1 Component topology

```
                                           Off-board SOVD Tester
                                                    |
                                                    | HTTPS (ASAM v1.1 OpenAPI / ISO 17978-3)
                                                    v
+-----------------------------------------------------------------------+
|                    Raspberry Pi Gateway Host                          |
|                                                                       |
|  +-------------------+       +-----------------+    +--------------+  |
|  |   SOVD Gateway    |<----->|   SOVD Server   |<-->|     DFM      |  |
|  | (opensovd-core)   |       | (opensovd-core) |    | + SQLite DB  |  |
|  +---------+---------+       +-----------------+    +------+-------+  |
|            |                                               ^          |
|            | (routing)                                     | IPC      |
|            v                                               |          |
|  +-------------------+       +------------------+          |          |
|  |  CDA              |       | CAN-to-DoIP      |          |          |
|  | (Rust, from       |<----->| Proxy (Rust)     |          |          |
|  |  upstream)        | DoIP  | (new, our code)  |          |          |
|  +---------+---------+       +------+-----------+          |          |
|            |                        |                     |          |
+------------|------------------------|---------------------|----------+
             | DoIP / TCP             | SocketCAN / ISO-TP  | Fault IPC
             v                        v                     |
    +------------+------------+       +-----+-----+-----+   |
    |            |            |       |     |     |     |   |
    v            v            v       v     v     v     v   |
  +---+                            +---+           +---+      |
  |BCM|                            |CVC|           |SC |      |
  +---+                            +---+           +---+      |
  (POSIX+DoIP)                 (STM32, CAN)        (TMS570)
   \_virtual_/                   \______physical______/
          3-ECU bench per ADR-0023; all 3 run Fault Lib shim ---> DFM via IPC
```

**Key design decisions:**

1. **Virtual ECU (BCM) speaks DoIP directly.** It runs on POSIX, TCP is free.
   *(Original topology included ICU and TCU as additional virtual ECUs; retired per ADR-0023.)*
2. **Physical ECUs (CVC, SC) speak CAN.** A Rust `CAN-to-DoIP proxy` on the Pi bridges.
   *(Original topology included FZC and RZC as additional STM32 physical ECUs; retired per ADR-0023.)*
3. **Fault Library shim is C, not Rust**, on the embedded side. Rust is used for the POSIX/Pi
   components only. This avoids dragging the Rust toolchain into ASIL-D firmware.
4. **DFM uses SQLite for persistence.** DTC data is relational; SQLite is proven and zero-ops.
5. **All SOVD components live in one opensovd-core workspace** — single Cargo workspace.
6. **CDA is used as-is from upstream** — we contribute fixes, not fork.
7. **ODX descriptions are bespoke per ECU** — we write them ourselves, then convert to MDD.
8. **SIL first, HIL second, hardware last.** Nothing touches physical ECUs until Docker works.

### 2.2 Data flows (the 5 MVP use cases)

**UC1 — Read faults via SOVD (SOVD term; UDS layer uses DTCs):**
```
Tester GET /sovd/v1/components/{id}/faults
  → SOVD Server routes to DFM
  → DFM queries SQLite (cached faults from Fault Lib)
  → DFM also queries CDA for legacy-ECU faults
  → CDA sends UDS 0x19 ReadDTCInformation over DoIP
  → [virtual] direct to ECU  / [physical] via Pi proxy → CAN 0x7XX
  → ECU Dcm responds with DTC list
  → CDA aggregates, returns to DFM
  → DFM returns unified JSON ListOfFaults
```

**UC2 — Report fault via Fault API:**
```
Swc_Motor detects over-current
  → FaultShim_Report(FID_MOTOR_OVERCURRENT, FAULT_SEVERITY_ERROR)
  → Unix socket / shared memory write (POSIX build)
  → DFM receives, updates in-memory table + SQLite
  → Operation cycle and debounce handled server-side
```

**UC3 — Clear faults:**
```
Tester DELETE /sovd/v1/components/{id}/faults
  → SOVD Server → DFM clears SQLite + notifies CDA
  → CDA sends UDS 0x14 ClearDiagnosticInformation over DoIP to the component
  → ECU Dcm calls Dem_ClearDTC() → NvM flush
  → Response aggregated back to tester
```

**UC4 — Reach UDS ECU via CDA:**
```
Same as UC1 but targets only legacy UDS ECUs
  → Tester GET /sovd/v1/components/cvc/faults
  → SOVD Server → Gateway → CDA (not DFM)
  → CDA reads MDD for CVC → sends UDS 0x19 → returns
```

**UC5 — Trigger diagnostic service:**
```
Tester POST /sovd/v1/components/rzc/operations/motor_self_test/executions
  → SOVD Server → Gateway → CDA
  → CDA sends UDS 0x31 StartRoutine over DoIP
  → ECU Dcm dispatches to registered routine handler (Swc_Motor)
  → Routine runs, returns status
```

### 2.3 Deployment topologies

| Topology | Use case | Hosts |
|----------|----------|-------|
| **SIL** | Developer laptop, CI | Docker Compose on one Linux box |
| **HIL** | Integration test, nightly | Pi gateway + physical STM32 + TMS570 |
| **Production** | Demo / customer | Pi gateway + full vehicle ECU harness |

---

## 3. Gap Analysis (Precise)

### 3.1 Embedded firmware gaps

| Item | Current | Needed | Effort |
|------|---------|--------|--------|
| UDS 0x19 ReadDTCInformation | Stub in Dcm | Wire `Dem_GetNextFilteredDTC` into Dcm dispatcher, ISO 14229 encoding | 3 days |
| UDS 0x14 ClearDiagnosticInformation | Missing | Call `Dem_ClearDTC`, NvM flush, response | 2 days |
| UDS 0x31 RoutineControl | Missing | Routine dispatch table, register handlers per ECU | 5 days |
| DoIP POSIX transport | Missing | New `DoIp_Posix.c` — TCP listener on 13400, wraps Dcm | 3 days |
| Fault Library C shim | Missing | New `FaultShim/` module — Unix socket IPC | 4 days |
| Per-ECU ODX descriptions | None | 7 ODX files describing each ECU's UDS services | 5 days |
| HIL test scenarios for new services | None | Add to `test/hil/test_hil_uds.py` | 3 days |
| MISRA compliance | Enforced | Must stay clean on all new code | ongoing |

**Total embedded effort: ~25 person-days.**

### 3.2 OpenSOVD upstream gaps (what we build)

| Item | Repo | Effort |
|------|------|--------|
| DFM prototype (in-memory + SQLite) | `opensovd-core/sovd-dfm` | 15 days |
| SOVD Server (REST endpoints) | `opensovd-core/sovd-server` | 20 days |
| SOVD Gateway (routing logic) | `opensovd-core/sovd-gateway` | 10 days |
| SOVD interfaces crate (shared types) | `opensovd-core/sovd-interfaces` | 5 days |
| SQLite schema + sqlx integration | `opensovd-core/sovd-db` | 5 days |
| Integration test suite | `opensovd-core/integration-tests` | 10 days |
| CAN-to-DoIP proxy | `gateway/can_to_doip_proxy` | 7 days |
| Docker Compose demo topology | `opensovd` (docs) | 3 days |

**Total upstream/gateway effort: ~75 person-days.**

### 3.3 Integration gaps

| Item | Effort |
|------|--------|
| CVC ODX → MDD pipeline wired into Taktflow CI (ADR-0023; FZC/RZC retired) | 3 days |
| CDA configured for Taktflow topology | 2 days |
| SIL scenario: CDA smoke test against Docker CVC | 2 days |
| SIL scenario: full SOVD → CDA → ECU round-trip | 3 days |
| HIL scenario: SOVD via Pi gateway against physical CVC | 5 days |
| Safety case: update HARA for new UDS routine services | 5 days |
| ASPICE traceability: new work products | ongoing |

**Total integration effort: ~20 person-days.**

**Grand total: ~120 person-days = ~6 person-months of focused work.**
With parallelism across 4-5 engineers, **6-month calendar duration is achievable.**

---

## 4. Phased Implementation

Each phase has: entry criteria, deliverables, exit criteria, owner.

### Phase 0 — Foundation (Apr 14 – Apr 30, 2026)

**Goal:** Team aligned, workspace ready, first throwaway prototype.

**Entry:** Everyone has ECA signed, toolchain installed, can build CDA locally.

**Deliverables:**
- Architecture Decision Record for Taktflow-SOVD integration (this document → ADR)
- Git branch strategy documented: `feature/sovd-*` branches, PRs gated by SIL+HIL
- `opensovd-core` workspace skeleton (empty crates + CI)
- CI matrix for `opensovd-core`: `cargo test --workspace` + clippy pedantic + nightly fmt
- First SOVD architecture document PR to upstream `opensovd` repo

**Exit:** Hello-world Rust binary in `opensovd-core/sovd-server` returns `200 OK` on `/health`.

**Owner:** Architect + 1 Rust engineer.

---

### Phase 1 — Embedded UDS + DoIP POSIX (May 1 – May 31, 2026)

**Goal:** Taktflow firmware exposes full MVP UDS service set and is reachable over DoIP.

**Entry:** Phase 0 complete.

**Deliverables:**

1. **Dcm 0x19 ReadDTCInformation handler** — `firmware/bsw/services/Dcm/Dcm_ReadDtcInfo.c`
   - Subfunctions 0x01 (reportNumberOfDTCByStatusMask), 0x02 (reportDTCByStatusMask), 0x0A
   - Unit tests: `test/unit/test_dcm_0x19.c` — all subfunctions
   - HIL test: `hil_081_cvc_uds_read_dtc.yaml`

2. **Dcm 0x14 ClearDiagnosticInformation handler** — `firmware/bsw/services/Dcm/Dcm_ClearDtc.c`
   - Call `Dem_ClearDTC` by group
   - Trigger NvM async write
   - Unit tests + HIL test

3. **Dcm 0x31 RoutineControl handler** — `firmware/bsw/services/Dcm/Dcm_RoutineControl.c`
   - Routine dispatch table (add to ARXML codegen)
   - Initial routines: `ROUTINE_MOTOR_SELF_TEST`, `ROUTINE_BRAKE_CHECK`
   - Unit tests + HIL test

4. **DoIp_Posix.c** — `firmware/platform/posix/src/DoIp_Posix.c`
   - TCP listener on port 13400
   - DoIP message types: vehicle identification, routing activation, diagnostic message
   - Forwards diagnostic payloads to `Dcm_DispatchRequest()`
   - No physical transport yet — POSIX only

5. **Per-ECU ODX descriptions** — `firmware/ecu/*/odx/*.odx-d`
   - Written by hand, one file per ECU
   - Reviewed by diagnostics lead
   - Generated MDDs committed to `firmware/ecu/*/odx/*.mdd`

**Exit criteria (all must hold):**
- All new Dcm handlers pass unit tests
- MISRA clean (zero violations in CI)
- HIL suite passes with new tests added
- `odx-converter` produces valid MDDs for the 3 active ECUs (ADR-0023; CVC is the only ECU requiring an MDD under the reduced bench since SC is not yet UDS-addressable and BCM runs as a POSIX simulator)
- Docker-based CVC accepts DoIP connection on localhost:13400 and responds to UDS 0x19

**Owner:** Embedded lead + 2 embedded engineers.

**Dependencies:** Phase 0 workspace ready. Independent of Phase 2-5.

---

### Phase 2 — CDA Integration + CAN-to-DoIP Proxy (Jun 1 – Jun 30, 2026)

**Goal:** CDA reaches every Taktflow ECU (virtual directly, physical via Pi proxy).

**Entry:** Phase 1 Dcm handlers working in SIL; MDDs generated.

**Deliverables:**

1. **CDA configured for Taktflow** — `classic-diagnostic-adapter/opensovd-cda.toml`
   - MDD path points to committed Taktflow MDDs
   - DoIP discovery range scans virtual ECU containers
   - Logging wired to DLT (for later observability phase)

2. **CAN-to-DoIP proxy** — `gateway/can_to_doip_proxy/` (new Rust crate)
   - Cargo workspace in Pi gateway tree
   - Crates: `proxy-core`, `proxy-doip`, `proxy-can`, `proxy-main`
   - DoIP server on Pi listens on port 13400
   - Translates DoIP diagnostic messages → CAN ISO-TP frames via SocketCAN
   - Response path: CAN ISO-TP → DoIP
   - Unit tests + integration tests with virtual CAN (vcan0)

3. **SIL scenario: CDA smoke test** — `test/sil/scenarios/sil_sovd_cda_smoke.yaml`
   - Docker topology: `cvc` (POSIX, DoIP) + `cda` (upstream CDA)
   - Test: `curl http://cda:8080/sovd/v1/components/cvc/faults` returns valid JSON ListOfFaults
   - Runs in SIL nightly pipeline

4. **HIL scenario: CDA via Pi proxy** — `test/hil/scenarios/hil_sovd_cda_via_proxy.yaml`
   - Pi runs `can_to_doip_proxy`
   - CDA runs on developer laptop, points at Pi IP
   - Target: physical CVC on CAN bus
   - Test: full SOVD GET → UDS 0x19 → DTC response

**Exit criteria:**
- CDA smoke test green in SIL nightly
- CAN-to-DoIP proxy has ≥80% line coverage
- HIL scenario passes against physical CVC
- Any CDA bugs found during integration are captured as internal fix branches
  (patches staged locally, ready to upstream later per §8)
- Taktflow ODX example staged locally under `odx-converter/examples/` (not pushed)

**Owner:** Rust lead + 1 Rust engineer + 1 Pi/gateway engineer + 1 test engineer.

**Dependencies:** Phase 1 complete.

---

### Phase 3 — Fault Library + DFM Prototype (Jul 1 – Aug 15, 2026)

**Goal:** Embedded components report faults → DFM stores → SOVD GET /dtcs returns live state.

**Entry:** Phase 2 complete.

**Deliverables:**

1. **C fault shim embedded module** — `firmware/bsw/services/FaultShim/`
   - Header `FaultShim.h` mirrors Rust `fault-lib` Fault API signatures
   - `FaultShim_Init()`, `FaultShim_Report(fid, severity, metadata)`, `FaultShim_Shutdown()`
   - POSIX implementation: `firmware/platform/posix/src/FaultShim_Posix.c`
     — opens Unix socket to DFM, writes fault events as protobuf
   - STM32 implementation: `firmware/platform/stm32/src/FaultShim_Stm32.c`
     — buffers faults to NvM slot, flushed by gateway sync task
   - Unit tests + MISRA clean

2. **DFM prototype** — `opensovd-core/sovd-dfm/`
   - Crates in workspace
   - Receives fault events on Unix socket (POSIX IPC)
   - In-memory DTC table with operation cycle handling
   - SQLite persistence via `sqlx` (schema migrations versioned)
   - Aging / debounce handled server-side (not on embedded — keeps ECU path simple)
   - Exposes stub SOVD GET `/sovd/v1/components/{id}/faults` via axum

3. **Wiring test: Dem → FaultShim → DFM** — full chain
   - CVC Docker container reports a synthetic fault
   - FaultShim writes to socket
   - DFM receives, stores in SQLite
   - SOVD GET returns the DTC within 100 ms

4. **SQLite schema** — `opensovd-core/sovd-db/migrations/`
   - Tables: `dtcs`, `fault_events`, `operation_cycles`, `catalog_version`
   - Migration-based schema evolution via sqlx
   - Indexed on DTC code, timestamp, ECU source

5. **Internal ADR: DFM design** — written internally, not upstreamed yet
   - Design document covering DFM architecture, SQLite schema, IPC protocol
   - Committed to `docs/adr/` in our own workspace
   - Follows the ADR pattern in upstream `opensovd/docs/design/adr/` so it is directly
     upstreamable as a doc PR later (per §8.1) when we decide to contribute

**Exit criteria:**
- End-to-end fault report → SOVD visibility works in Docker
- DFM has integration tests for fault ingestion, DTC query, clear, operation cycle
- Internal DFM ADR committed and reviewed by architect + Rust lead

**Owner:** Embedded lead + Rust lead + 1 Rust engineer + 1 embedded engineer.

**Dependencies:** Phase 2 complete.

---

### Phase 4 — SOVD Server + Gateway (Aug 16 – Oct 15, 2026)

**Goal:** Full SOVD Server implementing the MVP ASAM SOVD v1.1 OpenAPI
subset (ISO 17978-3), with a routing Gateway.

**Entry:** Phase 3 complete; DFM serving DTCs.

**Deliverables:**

1. **SOVD Server** — `opensovd-core/sovd-server/`
   - Crate structure mirrors CDA (axum + tokio)
   - Endpoints (MVP, per ISO 17978-3 SOVD v1.1.0-rc1 — per-component shape,
     documented in `opensovd-core/docs/openapi-audit-2026-04-14.md`; SOVD
     terms "faults" and "operations" replace UDS "DTCs" and "routines"):
     - `GET    /sovd/v1/health` — liveness probe, reports SovdDb / FaultSink / OperationCycle state
     - `GET    /sovd/v1/components` — list components (from MDD + DFM catalog), returns `DiscoveredEntities`
     - `GET    /sovd/v1/components/{id}` — component metadata, `EntityCapabilities`
     - `GET    /sovd/v1/components/{id}/data` — data identifiers (DIDs), returns `Datas`
     - `GET    /sovd/v1/components/{id}/faults` — list faults, returns `ListOfFaults`
     - `GET    /sovd/v1/components/{id}/faults/{fault_code}` — single fault, returns `FaultDetails`
     - `DELETE /sovd/v1/components/{id}/faults` — clear all faults on component
     - `DELETE /sovd/v1/components/{id}/faults/{fault_code}` — clear single fault
     - `GET    /sovd/v1/components/{id}/operations` — list available operations, returns `OperationsList`
     - `POST   /sovd/v1/components/{id}/operations/{operation_id}/executions` — start execution, `StartExecutionResponse`
     - `GET    /sovd/v1/components/{id}/operations/{operation_id}/executions/{execution_id}` — execution status
   - OpenAPI spec committed to `sovd-server/openapi.yaml`
   - Request/response types generated from schema via `utoipa`

2. **SOVD Gateway** — `opensovd-core/sovd-gateway/`
   - Routes requests to backends:
     - DFM backend: DTCs from our own stack (Fault Lib sources)
     - CDA backend: DTCs / services from legacy UDS ECUs
     - Native SOVD backend: future — ECUs that speak SOVD directly
   - Configuration: `opensovd-gateway.toml` — route map per ECU
   - Aggregation logic: merge DTC lists from multiple backends, de-dup by code

3. **Authentication middleware** — `opensovd-core/sovd-server/src/auth.rs`
   - Concept only in MVP: accept `Authorization: Bearer <token>` header
   - Token validation deferred to Phase 5 hardening (just scaffold in MVP)
   - OAuth2/OIDC integration plan documented, not implemented yet

4. **Docker Compose demo** — internal workspace, not upstreamed yet
   - Services: cvc, fzc, rzc, sc (as POSIX builds), cda, sovd-server, sovd-gateway, dfm
   - Tester script sends the 5 MVP use cases, verifies responses
   - Committed to our own workspace; staged for upstream per §8 when team decides

5. **Upstream-ready polish (build first, contribute later per §C.1):**
   - At end of Phase 4, all crates should be in a state where opening upstream PRs would
     require zero code changes — only a decision
   - Code quality, test coverage, docstrings, and SPDX metadata audited against upstream
     CDA standards one final time
   - No actual PRs opened until the team decides (earliest: after Phase 5 HIL green)

**Exit criteria:**
- Docker Compose demo runs all 5 MVP use cases end-to-end
- SOVD Server has ≥70% line coverage
- Integration tests cover full SOVD → Gateway → CDA → ECU chain
- Every crate in our `opensovd-core` fork is technically upstream-ready (no PR opened)

**Owner:** Rust lead + 3 Rust engineers + 1 test engineer.

**Dependencies:** Phase 3 complete.

---

### Phase 5 — End-to-End Demo + HIL on Physical (Oct 16 – Nov 30, 2026)

**Goal:** Full SOVD stack running against physical Taktflow hardware, HIL-gated.

**Entry:** Phase 4 demo working in Docker.

**Deliverables:**

1. **Pi deployment topology** — Ansible playbook or Docker Compose on Pi
   - SOVD Server + Gateway + DFM + CAN-to-DoIP proxy all run on the Pi
   - Systemd units or docker-compose with restart policies
   - Log aggregation to DLT (Phase 6 wiring)

2. **HIL test suite** — `test/hil/scenarios/hil_sovd_*.yaml` (8 scenarios, SOVD per-component shape per ISO 17978-3)
   - `hil_sovd_01_read_faults_all.yaml` — GET /sovd/v1/components/{id}/faults across all 3 ECUs (CVC, SC, BCM; ADR-0023)
   - `hil_sovd_02_clear_faults.yaml` — DELETE /sovd/v1/components/{id}/faults, verify via separate read
   - `hil_sovd_03_operation_motor_test.yaml` — POST /sovd/v1/components/rzc/operations/motor_self_test/executions
   - `hil_sovd_04_fault_injection.yaml` — inject CAN bus off, observe fault propagation through DFM
   - `hil_sovd_05_components_metadata.yaml` — GET /sovd/v1/components for all ECUs
   - `hil_sovd_06_concurrent_testers.yaml` — two testers hit SOVD Server simultaneously
   - `hil_sovd_07_large_fault_list.yaml` — ECU with 50+ faults, pagination correctness
   - `hil_sovd_08_error_handling.yaml` — ECU disconnect, degraded-mode responses (per ADR-0018 "never hard fail"), stale cache flag, timeouts

   - Live stop note (2026-04-16, updated): the Phase 5 CDA catalog blocker is cleared and the local OpenSOVD adapter now also bridges downstream CDA execution server-errors into the SOVD async contract (`202` start plus terminal `failed` status) instead of leaking them as raw transport failures. On the current live bench, D2 is green again once the Windows CDA is running on the real Phase 5 config, D3 clear-faults is blocked only by the bench precondition ("inject at least one clearable fault"), and the direct local CDA RZC operation path now reaches runtime with `504 Ecu [3] offline` instead of `404`. The remaining live wall is deployment, not another code-path mystery: the Raspberry Pi service is restored from its Linux backup after a failed Windows-binary copy, but this host cannot currently produce a new Pi binary for the pinned toolchain because `nightly-2025-07-14` does not have the `aarch64-unknown-linux-gnu` target installed here, and the Pi itself has no Rust toolchain or source checkout for a native rebuild.

3. **Real STM32 flashing via ST-LINK on the Windows dev host**
   - First phase where `firmware/ecu/cvc/` is built as an ARM ELF (not POSIX) and flashed through COM3 ST-LINK per `tools/bench/hardware-map.toml`
   - Build target cross-compiles from the Windows dev host via STM32CubeCLT / ARM GCC toolchain
   - `cargo xtask flash-cvc` convenience command that resolves the ST-LINK serial from hardware-map.toml and calls `st-flash` or `STM32_Programmer_CLI`
   - Smoke test: flash CVC firmware, issue UDS 22F190 over real CAN via the Pi's GS_USB adapter, assert VIN matches `cvc_identity.toml`
   - FZC/RZC flashed similarly if their ARM builds exist; otherwise deferred to Phase 6

4. **TMS570 integration** — even without Ethernet
   - TCU (TMS570) flashed via XDS110 on COM11/COM12 per `tools/bench/hardware-map.toml`
   - TCU goes through the same CAN-to-DoIP Pi proxy path as the STM32 ECUs
   - CAN routing table in proxy points TCU's logical address to its CAN ID range
   - TI Uniflash or Code Composer CLI integration for flashing (not openocd/st-flash)

5. **doip-codec PARTIAL migration in proxy-doip** (from doip-codec evaluation spike, `docs/doip-codec-evaluation.md`)
   - Replace `gateway/can_to_doip_proxy/proxy-doip/src/frame.rs` + `message_types.rs` with `theswiftfox/doip-codec` + `theswiftfox/doip-definitions` (the forks upstream CDA pins — NOT the crates.io `samp-reston` version)
   - Keep `gateway/can_to_doip_proxy/proxy-doip/src/server.rs`, `DoipHandler` trait, and the ISO-TP FlowControl integration from PR #9 intact — those are the pieces doip-codec does not cover
   - Keep ADR-0010 DiscoveryMode ("both" — broadcast + static) logic
   - Gate: the existing phase 2 Line B proxy tests + the Phase 4 Line B multi-frame interop test must stay green; byte output on the wire must still match `DoIp_Posix.c`
   - Vendor vs git-rev pin: open question. Default is a git rev pin in Cargo.toml matching CDA's pin exactly, to keep drift trackable

6. **MDD FlatBuffers emitter in odx-gen** (from Rust SIG 2026-04-14 findings)
   - Extend `tools/odx-gen/` to emit Marvelous Diagnostic Description (MDD) alongside the current PDX output
   - MDD is ODX restructured, wrapped in FlatBuffers, memory-mappable — upstream CDA consumes this natively (~1/3 the size of ODX, ~10 ns conversion latency)
   - Add a `--emit=mdd` flag that produces `cvc.mdd`, `fzc.mdd`, etc. matching the schema upstream CDA's `cda-database` crate expects
   - Round-trip test: our emitted MDD loads into CDA's in-memory database and decodes the same DiagService bytes as our PDX path
   - Optional but recommended: vendor upstream CDA's FlatBuffers schema (`.fbs`) as the single source of truth; regenerate Rust bindings per odx-gen build

7. **Autonomous bench debugging tools**
   - Install `github.com/alexmohr/mdd-ui` on the dev host for ECU inspection (TUI that reads MDDs + verifies diagnostic messages + checks auth)
   - Add `console-subscriber` as a dev-dep on `sovd-main` so `tokio-console` can attach to live sessions during HIL runs — unblocks deadlock debugging per upstream CDA's experience

8. **Performance validation** — measure under load
   - Fault read latency: SIL vs HIL
   - Throughput: concurrent SOVD requests
   - Memory footprint: DFM + Server + Gateway on Pi
   - Targets: `/faults` read <100 ms, `GET /components/{id}/faults` P99 <500 ms, <200 MB RAM total on Pi (matches upstream CDA envelope)

9. **Capability-showcase observer dashboard** (per ADR-0024, two stages;
   decisions resolved 2026-04-17)
   - **Stage 1 — Self-hosted, mTLS, zero cloud cost (blocking Phase 5 exit)**:
     reuse `taktflow-embedded-production` cloud_connector + ws_bridge on
     the Pi with `AWS_IOT_ENDPOINT=""`. Add `fault-sink-mqtt` crate
     (JSON wire format) publishing DFM events to local Mosquitto. Add
     Prometheus + Grafana on Pi for historical view (replaces the
     Timestream path — $0 recurring cost). Add nginx for TLS termination
     + mTLS client-cert auth aligned with SEC-2.1. Build SvelteKit +
     Tailwind + shadcn-svelte dashboard at `dashboard/`, static build
     served by nginx at `https://<pi-ip>/` — all 20 OpenSOVD use cases
     live, including UC19 Prometheus-backed historical panel.
   - **Stage 2 — Optional AWS fleet uplink (not blocking Phase 5 exit)**:
     provision `DEVICE_ID=taktflow-sovd-hil-001` under the shared
     embedded-production AWS account via `scripts/aws-iot-setup.sh`,
     flip `AWS_IOT_ENDPOINT`, add `bench_id=sovd-hil` tag for data
     attribution. No Timestream. Fleet-level cross-bench aggregation
     lands here if/when multiple HIL rigs come online.
   - Exit (Stage 1): fault injected on bench visible at `https://<pi-ip>/`
     within 200 ms; 7 days of fault history in Grafana panel; nginx
     rejects requests without valid client cert.
   - Exit (Stage 2, optional): fault visible on AWS IoT Core test
     console within 2 s on `vehicle/dtc/new` topic with
     `bench_id=sovd-hil`.

**Exit criteria:**
- All 8 HIL scenarios green in nightly pipeline
- Performance targets met
- Observer dashboard (deliverable 9 Stage 1) serving all 20 use-case
  widgets on the bench LAN
- Demo video recorded for OpenSOVD community presentation

**Owner:** Test lead + 2 test engineers + 1 Rust engineer + 1 embedded engineer.

**Dependencies:** Phase 4 complete. Physical hardware lab available.

---

### Phase 6 — Hardening (Dec 1 – Dec 31, 2026)

**Goal:** Production-ready. Matches OpenSOVD Q4 milestone.

**Entry:** Phase 5 HIL green.

**Deliverables:**

1. **TLS everywhere** — rustls or openssl on all HTTP paths, **mbedtls as fallback** per upstream CDA's escape hatch
   - SOVD Server listens HTTPS only (self-signed for demo, cert-based for prod)
   - Gateway → Server uses mTLS
   - Certs provisioned via script (dev) or HSM (prod, deferred)
   - **If OpenSSL feature limits bite** (upstream CDA hit the "max frame size" extension wall and moved to `mbedtls` via bindgen — see Rust SIG 2026-04-14 notes), fall back to `mbedtls` the same way. Plain C via bindgen is easy; modern C++ bindings would be harder. Keep the mbedtls backend behind a Cargo feature flag so the default OpenSSL path stays visible.
   - TLS on DoIP specifically: most ECUs negotiate TLS for auth only and don't encrypt payloads. Follow the upstream CDA cipher-list setup pattern when preparing the socket.

2. **DLT tracing wired** — `dlt-tracing-lib` integrated
   - All Rust binaries emit DLT-compatible logs
   - DLT daemon on Pi collects + forwards to laptop/cloud
   - Correlation IDs propagate through Gateway → Server → CDA → ECU

3. **OpenTelemetry spans** — same pattern as existing CDA observability
   - Traces for every SOVD request from ingress to ECU response
   - Export to OTLP collector (Jaeger or Tempo)

4. **Rate limiting** — `tower::limit` middleware on SOVD Server
   - Per-client-IP rate limits to prevent diagnostic flooding

5. **Integrator guide** — internal `docs/integration/`, upstream-ready format
   - How to point OpenSOVD at a new ECU platform
   - MDD generation steps
   - DFM configuration
   - Deployment topology examples (SIL, HIL, embedded gateway)
   - Written in the shape of an upstream `opensovd/docs/integration/` PR so it can
     be pushed later if team approves (per §8)

6. **Safety case update** — `docs/safety/`
   - New UDS services HARA delta
   - Any new failure modes from DoIP + Fault Shim
   - Reviewed by safety engineer, approved before release

7. **Contribution decision point** — team reviews everything built
   - Architect + Rust lead + safety engineer decide: upstream now, upstream later, or not
   - Checklist from §12.2 applied
   - If go: open PRs in the priority order of §8.2

**Exit criteria:**
- All phases' exit criteria still hold
- Safety case delta approved
- Integrator guide complete (pushed upstream only if team decides per step 7)
- Contribution decision recorded in `docs/adr/phase-6-contribution-decision.md`

**Owner:** All hands — lead by architect.

**Dependencies:** Phase 5 complete.

---

## 5. Work Breakdown Structure (Summary Table)

| Phase | Calendar | Person-days | Owner role | Parallel to |
|-------|---------|-------------|------------|-------------|
| 0. Foundation | 2 weeks | 8 | Architect + 1 Rust | — |
| 1. Embedded UDS + DoIP | 4 weeks | 25 | Embedded lead + 2 | Phase 0 tail |
| 2. CDA integration + proxy | 4 weeks | 20 | Rust + Pi + test | Phase 1 tail (partial) |
| 3. Fault Lib + DFM | 6 weeks | 30 | Embedded + Rust | — |
| 4. SOVD Server + Gateway | 8 weeks | 50 | 3 Rust + 1 test | Phase 3 tail (partial) |
| 5. E2E demo + HIL physical | 6 weeks | 30 | Test lead + 2 | — |
| 6. Hardening | 4 weeks | 20 | All hands | — |
| **Total** | **~8 months** | **~180 person-days** | — | — |

---

## 6. Testing Strategy

Testing applies at every phase, every merge.

### 6.1 Test layers

| Layer | Tool | Where it runs | Blocks PR? |
|-------|------|---------------|------------|
| **Unit (C)** | Unity framework | `test/unit/` in firmware | Yes |
| **Unit (Rust)** | `cargo test --lib` | Each crate | Yes |
| **Integration (Rust)** | `cargo test --features integration-tests` | `opensovd-core/integration-tests/` | Yes |
| **SIL** | Docker Compose + pytest | `test/sil/` | Yes (nightly + PR) |
| **HIL** | Pi bench + test harness | `test/hil/` | Yes (nightly); warn (PR) |
| **Performance** | `criterion` + custom | CI nightly | No (tracked) |
| **MISRA** | cppcheck / coverity | CI | Yes |
| **Clippy pedantic** | `cargo clippy` | CI | Yes |
| **Nightly rustfmt** | `cargo +nightly fmt --check` | CI | Yes |
| **cargo-deny** | CI | CI | Yes |

### 6.2 Per-phase test gating

| Phase | New tests required |
|-------|-------------------|
| 1 | Unit: Dcm 0x19, 0x14, 0x31 | HIL: UDS read/clear/routine |
| 2 | Unit: CAN-to-DoIP proxy translators | SIL: CDA smoke test | HIL: via-proxy round trip |
| 3 | Unit: FaultShim, DFM components | Integration: fault report → SQLite → SOVD GET |
| 4 | Unit: all server crates | Integration: full SOVD → ECU chain | SIL: 5 use cases |
| 5 | HIL: 8 SOVD scenarios | Performance: latency / throughput |
| 6 | Integration: TLS, auth, rate limiting | Safety: delta HARA |

### 6.3 Safety test considerations

- New Dcm services must have FMEA entries before merge
- New routines (0x31) require HARA review — motor_self_test could affect safety
- MISRA deviations must be justified in `docs/safety/analysis/misra-deviation-register.md`

---

## 7. CI/CD Integration

### 7.1 New pipelines

Per §C.1, during Phases 0–3 we do not push to GitHub remotes. CI gates therefore apply
to **local pre-commit / pre-push** hooks and the existing Taktflow internal CI; GitHub
Actions workflows exist in the repo but only trigger once we decide to push.

| Pipeline | Trigger | What it does |
|----------|--------|--------------|
| `sovd-ci.yml` | Internal PR on `taktflow-embedded-production` SOVD branches | Unit + integration + SIL |
| `sovd-hil-nightly.yml` | Internal nightly 02:00 UTC | Full HIL SOVD suite on Pi bench |
| `opensovd-core-ci.yml` | Local pre-push hook + (later) any push to our fork | Rust lint + test + integration |
| `upstream-sync.yml` | nightly 04:00 UTC | Pull upstream → rebase our local branches → alert on conflicts |

### 7.2 Gating policy

- **Local pre-commit gate:** clippy + fmt + SPDX check
- **Local pre-push gate (later):** unit + integration + MISRA (if/when we start pushing)
- **Internal merge-to-main gate:** everything above + SIL + HIL smoke (subset of HIL suite)
- **Contribution gate (Phase 6):** all of the above + full HIL suite + performance benchmark deltas + team sign-off

### 7.3 Upstream synchronization

- Every Monday 09:00: `upstream-sync.yml` rebases our forks against upstream `main`
- Alerts fire if merge conflicts; architect resolves within 24h
- Weekly: review any upstream PRs to repos we depend on, flag breaking changes

---

## 8. Upstream Contribution Strategy

**Core principle (per §C.1): build first, contribute later.** No upstream PRs during
Phases 0–3. Nothing we build is pushed to a public fork remote until the team decides it
is ready. We own the code end to end before anyone else sees it.

### 8.1 Contribution timing — decision-driven, not calendar-driven

There is **no fixed PR schedule**. Contribution happens when all of these are true:

1. The component works end-to-end in our own SIL + HIL
2. The team agrees the code is production-quality by our standards (not just upstream's)
3. No pending design changes that would trigger churn
4. A Taktflow-internal review has signed off (architect + safety + Rust lead)

Earliest plausible first PR: **after Phase 4 is green** (full MVP working in Docker).
More likely: **after Phase 5** (HIL on physical hardware proving the stack works).

### 8.2 What upstream looks like when we do contribute

The order of PRs, when the decision is made:

| Priority | What | Target repo | Rationale |
|----------|------|-------------|-----------|
| 1 | `sovd-interfaces` trait contracts | `opensovd-core` | Lowest-risk, establishes house presence |
| 2 | `sovd-dfm` (with design doc) | `opensovd-core` | Fills a major gap, upstream has nothing |
| 3 | `sovd-server` MVP | `opensovd-core` | Central piece of the project |
| 4 | `sovd-gateway` | `opensovd-core` | Routing + multi-ECU support |
| 5 | Taktflow ODX examples | `odx-converter/examples/` | Low-risk, demonstrates real use |
| 6 | Any CDA bugs found during integration | `classic-diagnostic-adapter` | Isolated fixes |
| 7 | Docker Compose demo topology | `opensovd/examples/` | Ties the narrative together |
| 8 | Integrator guide | `opensovd/docs/integration/` | Final polish |

### 8.3 Alignment tactics (passive, shadow-ninja)

- **Read meeting minutes weekly** — `opensovd/discussions` — track upstream direction
- **Watch architecture board decisions** — Mondays 11:30 CET (read minutes, never attend)
- **Track upstream commits** via `upstream-sync.yml` cron job
- **Never ping maintainers** — no direct outreach, no DMs, no cold emails
- **Let public artifacts do the work** — when we eventually push, the quality speaks

### 8.4 What always stays internal (never upstreamed)

- Taktflow-specific DBC files and codegen pipelines
- Embedded Dcm modifications to `taktflow-embedded-production` (our firmware)
- Our ASPICE + ISO 26262 process artifacts
- Raspberry Pi deployment Ansible playbooks and systemd units
- Safety case deltas, HARA updates, FMEA tables
- Internal ADRs and knowledge-base notes (the stuff in `docs/sovd/notes-*`)

---

## 9. Risk Register

| # | Risk | Likelihood | Impact | Mitigation |
|---|------|-----------|--------|------------|
| R1 | `opensovd-core` scope creep — we build too much too fast | High | High | Hard-scope MVP to 5 use cases; defer anything not on critical path |
| R2 | Upstream maintainers reject our SOVD Server approach | Medium | Very High | Phase 3 design ADR PR first; don't write code until design aligned |
| R3 | ODX schema licensing blocks `odx-converter` work | Medium | Medium | Write community XSD covering our subset; bundle under Apache-2.0 |
| R4 | New UDS routines trigger HARA changes requiring full safety review | Medium | High | Involve safety engineer at Phase 0; HARA delta reviewed in Phase 1 |
| R5 | TMS570 Ethernet needed but still broken | Medium | Medium | CAN-to-DoIP Pi proxy already handles this; physical DoIP on TMS570 is deferred |
| R6 | Rust skills gap in embedded team | Medium | Medium | Fault shim is C; Rust is for Pi/laptop components only; Rust engineers lead `opensovd-core` |
| R7 | Docker networking edge cases break SIL nightly | Low | Medium | Use host networking for DoIP; document port reservations |
| R8 | SQLite concurrency limits hit under load | Low | Medium | Use WAL mode; benchmark in Phase 5; swap to Postgres if needed (unlikely) |
| R9 | OpenSOVD maintainers start `opensovd-core` in parallel | Medium | Medium | Watch upstream commits weekly via upstream-sync cron; if upstream starts, our work still runs internally — worst case we rebase onto their scaffolding or skip upstreaming entirely |
| R10 | MISRA violations block merge of new Dcm code | Low | High | Mirror existing Dcm patterns; run MISRA locally before push |
| R11 | Taktflow 20-person team gets pulled to other priorities | High | High | Architect must hold scope; phase gates give pause-points; each phase is independently shippable |
| R12 | ECA signing delays for new contributors | Low | Low | All hands sign ECA in Phase 0 |

---

## 10. Team Allocation (20 people)

SOVD workstream draws from the 20-person team. Estimated allocation at peak (Phase 4):

| Role | Count | Responsibilities |
|------|-------|------------------|
| Architect / upstream liaison | 1 | ADRs, design alignment, PR reviews, roadmap |
| Embedded lead | 1 | Dcm/Dem work, MISRA oversight, safety case delta |
| Embedded engineers | 2 | UDS handlers, DoIP POSIX, Fault shim, ODX |
| Rust lead | 1 | `opensovd-core` architecture, reviews |
| Rust engineers | 3 | DFM, SOVD Server, Gateway, CAN-to-DoIP proxy |
| Safety engineer | 1 (part-time) | HARA delta, FMEA updates |
| Test lead | 1 | SIL/HIL strategy, test infrastructure |
| Test engineers | 2 | SOVD test scenarios, performance |
| DevOps / CI | 1 | Pipelines, Docker topologies, nightly gating |
| Pi / gateway engineer | 1 | Deployment, CAN-to-DoIP proxy, DLT integration |
| Technical writer | 1 (part-time) | Integrator guide, ADRs, upstream docs |
| **Total peak** | **14** | — |

Remaining 6 on other Taktflow workstreams (not diluted by SOVD).

---

## 11. Timeline with Milestones

```
 2026-04     2026-05     2026-06     2026-07     2026-08     2026-09     2026-10     2026-11     2026-12
    |           |           |           |           |           |           |           |           |
 [P0]-[P1 Embedded UDS + DoIP]
                |--[P2 CDA + proxy]
                |       |--[P3 Fault Lib + DFM]------|
                |       |       |--[P4 SOVD Server + Gateway]-----------|
                |       |       |       |           |       |--[P5 E2E + HIL]
                |       |       |       |           |       |           |--[P6 Hardening]
                |       |       |       |           |       |           |           |
                M1      M2              M3                  M4                      M5
```

| Milestone | Date | Success criteria |
|-----------|------|------------------|
| **M1** Embedded UDS complete | 2026-05-31 | Dcm 0x19/0x14/0x31 pass HIL; DoIP POSIX accepts diagnostic messages |
| **M2** CDA integration green | 2026-06-30 | SOVD GET /dtcs via CDA round-trips to one Docker ECU; Pi proxy reaches physical CVC |
| **M3** DFM prototype serving DTCs | 2026-08-15 | Fault inject → DFM ingest → SOVD GET visible in <100ms |
| **M4** SOVD Server MVP in Docker | 2026-10-15 | All 5 MVP use cases pass in Docker Compose demo |
| **M5** Hardened, HIL green, upstream-ready | 2026-12-31 | Physical HIL passes; demo recorded; code ready to upstream if team decides |

---

## 12. Success Criteria

### 12.1 Technical success (end of 2026)

- [ ] All 5 OpenSOVD MVP use cases pass against Taktflow hardware in SIL and HIL
- [ ] SOVD Server, Gateway, DFM, CAN-to-DoIP proxy all running on Raspberry Pi in production mode
- [ ] DTC round-trip latency <500 ms at P99 across all 3 active ECUs (ADR-0023)
- [ ] Zero MISRA violations on new embedded code
- [ ] Zero clippy pedantic violations on new Rust code
- [ ] Full safety case delta approved by safety engineer
- [ ] Nightly SIL + HIL pipelines green for 30 consecutive days

### 12.2 Contribution readiness (not required — decision-driven per §C.1 and §8)

- [ ] All code is stylistically indistinguishable from upstream CDA (max sync principle)
- [ ] `sovd-interfaces` crate is the cleanest public-facing artifact in the workspace
- [ ] Design ADRs exist internally for every major component (ready to upstream as docs)
- [ ] No blocker prevents opening upstream PRs — the decision to upstream is purely policy

Upstream PRs themselves are **not** success criteria. We succeed by owning working code.
Upstreaming is a downstream benefit we can choose to realize at any time after M5.

### 12.3 Process success

- [ ] All new work products traceable in ASPICE
- [ ] All 5 MVP use cases have requirements → design → test traceability
- [ ] Safety case updated and reviewed
- [ ] Zero safety regressions on existing HIL suite

---

## 13. Governance

### 13.1 Decision authority

- **Architectural decisions** — Architect, documented as ADRs, reviewed weekly by Rust lead + Embedded lead
- **Scope decisions** — Architect, escalation to program lead if timeline at risk
- **Safety decisions** — Safety engineer, veto power on anything touching ASIL paths
- **Upstream alignment** — Architect, with upstream maintainer consent via design ADRs

### 13.2 Cadence

- **Daily standup** — 15 min, workstream members only
- **Weekly sync** — 45 min, SOVD workstream + architect
- **Weekly upstream review** — 30 min, architect reviews OpenSOVD discussions + PRs
- **Phase gate review** — end of each phase, all leads, go/no-go to next phase

### 13.3 Documentation obligations

- Every ADR lives in `opensovd/docs/design/adr/` (upstream) or `docs/adr/` (Taktflow internal)
- Every phase produces a retro document in `docs/retro/phase-<n>.md`
- Every HIL scenario YAML has a one-paragraph intent comment
- Every ADR is written in the shape of an upstream-ready PR (so it can be pushed later
  per §8 without rework)

---

## 14. Open Questions (need resolution before Phase 2)

| Question | Owner | Target resolution |
|----------|-------|-------------------|
| Fault IPC: Unix socket vs. shared memory? | Rust lead | Phase 0 week 2 |
| DFM persistence: SQLite vs. FlatBuffers file? | Architect | Phase 0 week 2 |
| ODX schema: ASAM download vs. community XSD? | Embedded lead | Phase 1 week 1 |
| Auth model for SOVD Server: OAuth2 / cert / both? | Architect + security lead | Phase 4 |
| DoIP discovery on Pi: broadcast vs. static config? | Pi engineer | Phase 2 week 1 |
| Physical DoIP on STM32: lwIP vs. ThreadX NetX vs. never? | Hardware lead | Phase 5 (deferred) |

---

## 15. Immediate Next Actions (this week)

1. **Architect** — create ADR template in `docs/adr/0001-taktflow-sovd-integration.md`
2. **Architect** — post design discussion on `opensovd/discussions` introducing our effort
3. **Rust lead** — scaffold `opensovd-core` workspace (empty crates, CI)
4. **Embedded lead** — read CVC Dcm code; draft 0x19 handler design
5. **Embedded engineer** — write CVC ODX description (first ECU)
6. **Pi engineer** — spike CAN-to-DoIP proxy (throwaway prototype) to de-risk Phase 2
7. **Test lead** — design HIL scenario templates for SOVD tests
8. **Safety engineer** — preliminary HARA delta review of planned new UDS services
9. **All hands** — confirm Eclipse ECA signed; run `cargo build` in all 8 cloned repos once
