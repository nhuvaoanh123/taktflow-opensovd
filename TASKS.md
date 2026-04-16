# Eclipse OpenSOVD — Taktflow Task Breakdown

**Companion to:** [MASTER-PLAN.md](MASTER-PLAN.md)
**Principle:** Build our own implementation, but mirror upstream infrastructure exactly
(Cargo workspace layout, clippy.toml, rustfmt.toml, edition 2024, MSRV 1.88.0, Axum 0.8,
Tokio 1.48, SPDX headers, cargo-deny, pre-commit via cicd-workflows) so every PR is
upstream-ready.

---

## Task Format

```
T<phase>.<stream>.<num>  — <title>
  What:   one-line description
  Out:    files created or modified
  Deps:   prerequisite task IDs
  Effort: person-hours or days
  Role:   owner role
  Status: [ ] todo  [~] doing  [x] done
```

**Streams:** S=Setup · R=Rust core · E=Embedded · G=Gateway/Pi · T=Test · D=Docs · Sf=Safety · Ops=DevOps

---

## Phase 0 — Foundation  (Apr 14 – Apr 30, ~2 weeks)

Goal: workspace, CI, governance in place. First throwaway hello-world in `opensovd-core`.

### Stream S — Setup & Governance

```
T0.S.1  — Confirm Eclipse ECA signed for all SOVD workstream members
  What:   Each member verifies ECA at accounts.eclipse.org; emails match Git config
  Out:    docs/sovd/eca-status.md (internal checklist)
  Deps:   —
  Effort: 1 h × N people
  Role:   Architect
  Status: [x] confirmed 2026-04-14

T0.S.2  — Create docs/adr/0001-taktflow-sovd-integration.md
  What:   ADR covering why we build opensovd-core, Fault Lib shim decision, DoIP proxy, phased plan
  Out:    taktflow-embedded-production/docs/adr/0001-taktflow-sovd-integration.md
  Deps:   —
  Effort: 4 h
  Role:   Architect
  Status: [ ]

T0.S.3  — Post introduction discussion on upstream opensovd/discussions
  What:   Short public post: "Taktflow planning SOVD integration, open to feedback"
  Out:    GitHub discussion thread (shadow-ninja visibility — no maintainer pings)
  Deps:   T0.S.2
  Effort: 1 h
  Role:   Architect
  Status: [ ]

T0.S.4  — Resolve 6 open questions from MASTER-PLAN §14
  What:   Make decisions (Unix socket vs shmem, SQLite vs flat file, ODX schema source, auth model, DoIP discovery, physical DoIP)
  Out:    docs/adr/0002…0007 — one micro-ADR per decision
  Deps:   T0.S.2
  Effort: 2 days (meetings + writing)
  Role:   Architect + leads
  Status: [ ]

T0.S.5  — Git branch strategy for SOVD work
  What:   Document: feature/sovd-* branches, PRs require SIL green, nightly HIL advisory, upstream PRs from dedicated upstream-pr/* branches
  Out:    docs/sovd/branching.md
  Deps:   —
  Effort: 2 h
  Role:   Architect
  Status: [ ]

T0.S.6  — Subscribe to upstream signals
  What:   Join opensovd-dev@eclipse.org; join #eclipse-opensovd Slack; add opensovd/discussions RSS to team feed; set weekly calendar slot for upstream review
  Out:    docs/sovd/upstream-watch.md with links
  Deps:   —
  Effort: 1 h
  Role:   Architect
  Status: [ ]
```

### Stream R — opensovd-core Workspace Scaffold

```
T0.R.1  — Create Cargo workspace skeleton in our opensovd-core fork
  What:   Initialize workspace Cargo.toml mirroring classic-diagnostic-adapter exactly
  Out:    opensovd-core/Cargo.toml, opensovd-core/Cargo.lock, opensovd-core/README.md
  Deps:   —
  Effort: 3 h
  Role:   Rust lead
  Status: [ ]

T0.R.2  — Copy rustfmt.toml and clippy.toml from upstream CDA
  What:   Exact-copy CDA's config (max_width=100, pedantic, too-many-lines=130)
  Out:    opensovd-core/rustfmt.toml, opensovd-core/clippy.toml, opensovd-core/.rustfmt-nightly.toml
  Deps:   T0.R.1
  Effort: 1 h
  Role:   Rust lead
  Status: [ ]

T0.R.3  — Copy deny.toml and SPDX/license setup from CDA
  What:   Apache-2.0 headers, license allowlist (Apache-2.0/BSD-3-Clause/ISC/MIT/Unicode-3.0/Zlib)
  Out:    opensovd-core/deny.toml, opensovd-core/LICENSES/Apache-2.0.txt, opensovd-core/.reuse/dep5
  Deps:   T0.R.1
  Effort: 2 h
  Role:   Rust lead
  Status: [ ]

T0.R.4  — rust-toolchain.toml pinning stable 1.88.0 + nightly 2025-07-14
  What:   Match CDA's toolchain pin exactly
  Out:    opensovd-core/rust-toolchain.toml
  Deps:   T0.R.1
  Effort: 15 min
  Role:   Rust lead
  Status: [ ]

T0.R.5  — Scaffold empty workspace crates mirroring CDA naming pattern
  What:   Create empty lib crates: sovd-interfaces, sovd-core, sovd-dfm, sovd-db, sovd-server, sovd-gateway, sovd-tracing, integration-tests. Binary crate: sovd-main
  Out:    9 crate directories, each with Cargo.toml + src/lib.rs or src/main.rs with SPDX header
  Deps:   T0.R.1, T0.R.2, T0.R.3, T0.R.4
  Effort: 4 h
  Role:   Rust lead
  Status: [ ]

T0.R.6  — Add common workspace dependencies matching CDA versions
  What:   [workspace.dependencies] block with tokio 1.48, axum 0.8, tower 0.5, serde 1.0, thiserror 2.0, tracing 0.1.41, clap 4.5, figment 0.10.19, utoipa (for OpenAPI)
  Out:    opensovd-core/Cargo.toml [workspace.dependencies]
  Deps:   T0.R.5
  Effort: 1 h
  Role:   Rust lead
  Status: [ ]

T0.R.7  — Hello-world SOVD server binary
  What:   sovd-main runs axum server, single route GET /health → 200 OK with JSON {"status":"ok"}
  Out:    opensovd-core/sovd-main/src/main.rs, opensovd-core/sovd-server/src/lib.rs
  Deps:   T0.R.5, T0.R.6
  Effort: 3 h
  Role:   Rust engineer
  Status: [ ]

T0.R.8  — First workspace unit test
  What:   Trivial test in sovd-interfaces proving `cargo test --workspace` works end-to-end
  Out:    opensovd-core/sovd-interfaces/src/lib.rs (with test)
  Deps:   T0.R.5
  Effort: 30 min
  Role:   Rust engineer
  Status: [ ]
```

### Stream Ops — CI Pipelines

```
T0.Ops.1 — GitHub Actions workflow for opensovd-core PRs
  What:   Match CDA's pr-checks.yml exactly — uses eclipse-opensovd/cicd-workflows reusable action; stable 1.88.0 strict, nightly 2025-07-14 warn-only
  Out:    opensovd-core/.github/workflows/pr-checks.yml
  Deps:   T0.R.4
  Effort: 2 h
  Role:   DevOps
  Status: [ ]

T0.Ops.2 — Build workflow for opensovd-core
  What:   Mirror CDA's build.yml — cargo build --release, cargo test --workspace, cargo deny check, artifact upload
  Out:    opensovd-core/.github/workflows/build.yml
  Deps:   T0.R.4
  Effort: 2 h
  Role:   DevOps
  Status: [ ]

T0.Ops.3 — pre-commit hook config matching CDA
  What:   .pre-commit-config.yaml with pre-commit-hooks 5.0.0, yamlfmt, rustfmt-check
  Out:    opensovd-core/.pre-commit-config.yaml
  Deps:   T0.R.4
  Effort: 1 h
  Role:   DevOps
  Status: [ ]

T0.Ops.4 — Taktflow embedded CI: new job "sovd-readiness"
  What:   Add job in taktflow-embedded-production CI that runs MISRA + unit tests on firmware/bsw/services/Dcm on any PR touching Dcm files
  Out:    taktflow-embedded-production/.github/workflows/sovd-readiness.yml
  Deps:   —
  Effort: 2 h
  Role:   DevOps
  Status: [ ]

T0.Ops.5 — Weekly upstream-sync workflow
  What:   Cron workflow that rebases our forks against upstream main, opens issue if conflicts
  Out:    tools/upstream-sync.sh + .github/workflows/upstream-sync.yml (in a small ops repo or opensovd-core)
  Deps:   —
  Effort: 3 h
  Role:   DevOps
  Status: [ ]
```

### Stream E — Embedded Prep

```
T0.E.1  — Read CVC Dcm.c and Dcm.h thoroughly
  What:   Understand dispatch table, session state machine, how existing 0x22 handler is wired. Note reusable patterns for new 0x19/0x14/0x31 handlers.
  Out:    docs/sovd/notes-dcm-walkthrough.md (short, for internal use)
  Deps:   —
  Effort: 1 day
  Role:   Embedded lead
  Status: [ ]

T0.E.2  — Read Dem.c/Dem.h and Dem_EventStatus handling
  What:   Map existing DEM event table to what 0x19 needs (DTC status bits, count, timestamps)
  Out:    docs/sovd/notes-dem-walkthrough.md
  Deps:   —
  Effort: 4 h
  Role:   Embedded lead
  Status: [ ]

T0.E.3  — Inventory DIDs across all 7 ECUs
  What:   Produce a table: ECU × DID ID × name × data type × source SWC. Foundation for ODX writing.
  Out:    docs/sovd/did-inventory.md
  Deps:   —
  Effort: 1 day
  Role:   Embedded engineer
  Status: [ ]

T0.E.4  — Inventory DTCs (fault events) across all 7 ECUs
  What:   Same table for DEM events: ECU × EventId × DTC code × severity × source
  Out:    docs/sovd/dtc-inventory.md
  Deps:   —
  Effort: 4 h
  Role:   Embedded engineer
  Status: [ ]
```

### Stream G — Gateway Spike

```
T0.G.1  — Throwaway CAN-to-DoIP proxy spike
  What:   Prototype in 1 file using socket2 + socketcan crate. Minimum: accept 1 DoIP diagnostic message, forward to vcan0, read response, return. No error handling, no tests.
  Out:    gateway/can_to_doip_proxy/spike/main.rs (throwaway, committed to feature branch only)
  Deps:   —
  Effort: 1 day
  Role:   Pi engineer
  Status: [ ]

T0.G.2  — De-risk findings document
  What:   What worked, what didn't, what ISO-TP edge cases surfaced. Input to Phase 2 design.
  Out:    docs/sovd/notes-doip-spike.md
  Deps:   T0.G.1
  Effort: 2 h
  Role:   Pi engineer
  Status: [ ]
```

### Stream Sf — Safety

```
T0.Sf.1 — Preliminary HARA delta scan
  What:   Review existing HARA; identify which hazards could be affected by 0x19 / 0x14 / 0x31. Flag high-risk ones early.
  Out:    docs/safety/deltas/hara-sovd-prelim.md
  Deps:   —
  Effort: 1 day
  Role:   Safety engineer
  Status: [ ]
```

### Stream T — Test Infrastructure

```
T0.T.1  — Design HIL scenario template for SOVD tests
  What:   YAML template matching existing test/hil/ conventions. Includes: given/when/then, expected CAN traffic, expected SOVD JSON response, tolerance windows.
  Out:    test/hil/templates/sovd-scenario.yaml + test/hil/templates/README.md
  Deps:   —
  Effort: 4 h
  Role:   Test lead
  Status: [ ]

T0.T.2  — Identify which existing HIL tests need SOVD companion scenarios
  What:   List of ~20 current HIL UDS tests that have SOVD equivalents
  Out:    docs/sovd/hil-scenario-matrix.md
  Deps:   T0.T.1
  Effort: 2 h
  Role:   Test lead
  Status: [ ]
```

### Phase 0 Exit Checklist

- [ ] T0.R.7 hello-world binary responds to `curl http://localhost:8080/health`
- [ ] `cargo test --workspace` green in opensovd-core
- [ ] PR checks workflow passes on a trivial no-op PR
- [ ] ADR-0001 committed and linked from master plan
- [ ] 6 micro-ADRs covering all open questions from MASTER-PLAN §14
- [ ] All workstream members confirmed ECA-signed
- [ ] Upstream discussion post live

---

## Phase 1 — Embedded UDS + DoIP POSIX  (May 1 – May 31, 4 weeks)

Goal: Taktflow firmware exposes full MVP UDS service set and is reachable over DoIP from
CDA (POSIX platform only — no STM32 firmware DoIP yet).

### Stream E — Dcm 0x19 ReadDTCInformation

```
T1.E.1  — Design 0x19 handler — subfunctions and data shapes
  What:   Document which ISO 14229 subfunctions we implement in MVP (0x01, 0x02, 0x0A, 0x06). Define request/response byte layout. Review with embedded lead.
  Out:    docs/sovd/design-dcm-0x19.md
  Deps:   T0.E.1, T0.E.2
  Effort: 4 h
  Role:   Embedded engineer
  Status: [ ]

T1.E.2  — Implement Dcm_ReadDtcInfo.c — subfunction 0x01 (reportNumberOfDTCByStatusMask)
  What:   Wire Dem_GetNumberOfFilteredDTC into Dcm dispatch table; MISRA-clean; follow existing 0x22 handler pattern
  Out:    firmware/bsw/services/Dcm/src/Dcm_ReadDtcInfo.c, firmware/bsw/services/Dcm/include/Dcm_ReadDtcInfo.h
  Deps:   T1.E.1
  Effort: 1 day
  Role:   Embedded engineer
  Status: [ ]

T1.E.3  — Implement 0x19 subfunction 0x02 (reportDTCByStatusMask)
  What:   Iterate DEM events, filter by status mask, format ISO 14229 response with pagination
  Out:    extends Dcm_ReadDtcInfo.c
  Deps:   T1.E.2
  Effort: 1.5 days
  Role:   Embedded engineer
  Status: [ ]

T1.E.4  — Implement 0x19 subfunction 0x0A (reportSupportedDTC)
  What:   Return full DTC list regardless of status — used by SOVD /components/{id}/dtcs
  Out:    extends Dcm_ReadDtcInfo.c
  Deps:   T1.E.2
  Effort: 4 h
  Role:   Embedded engineer
  Status: [ ]

T1.E.5  — Unit tests for 0x19 handler
  What:   Unity tests covering all 3 subfunctions, edge cases (empty DTC list, max pagination, invalid subfunction)
  Out:    test/unit/test_dcm_0x19.c
  Deps:   T1.E.4
  Effort: 1 day
  Role:   Embedded engineer
  Status: [ ]

T1.E.6  — Integrate 0x19 into Dcm service dispatch table
  What:   Add SID 0x19 to Dcm_SidTable[]; wire session/security check
  Out:    firmware/bsw/services/Dcm/src/Dcm.c (small edit), firmware/bsw/services/Dcm/cfg/Dcm_Cfg.c
  Deps:   T1.E.5
  Effort: 2 h
  Role:   Embedded engineer
  Status: [ ]
```

### Stream E — Dcm 0x14 ClearDiagnosticInformation

```
T1.E.7  — Design 0x14 handler
  What:   Define clear-by-group semantics, NvM flush behavior, response timing
  Out:    docs/sovd/design-dcm-0x14.md
  Deps:   T1.E.1
  Effort: 2 h
  Role:   Embedded engineer
  Status: [ ]

T1.E.8  — Implement Dcm_ClearDtc.c
  What:   Call Dem_ClearDTC(group); trigger NvM write; send positive response
  Out:    firmware/bsw/services/Dcm/src/Dcm_ClearDtc.c, firmware/bsw/services/Dcm/include/Dcm_ClearDtc.h
  Deps:   T1.E.7
  Effort: 1 day
  Role:   Embedded engineer
  Status: [ ]

T1.E.9  — Unit tests for 0x14
  What:   Clear all, clear by group, clear with NvM failure path
  Out:    test/unit/test_dcm_0x14.c
  Deps:   T1.E.8
  Effort: 4 h
  Role:   Embedded engineer
  Status: [ ]

T1.E.10 — Integrate 0x14 into dispatch table
  What:   Same pattern as T1.E.6
  Out:    firmware/bsw/services/Dcm/src/Dcm.c, firmware/bsw/services/Dcm/cfg/Dcm_Cfg.c
  Deps:   T1.E.9
  Effort: 1 h
  Role:   Embedded engineer
  Status: [ ]
```

### Stream E — Dcm 0x31 RoutineControl

```
T1.E.11 — Design 0x31 routine dispatch table
  What:   Table of routine IDs → function pointers. Define routine lifecycle states (start/stop/getResults). Initial routines list.
  Out:    docs/sovd/design-dcm-0x31.md
  Deps:   T1.E.1
  Effort: 4 h
  Role:   Embedded lead
  Status: [ ]

T1.E.12 — Implement Dcm_RoutineControl.c core dispatcher
  What:   Subfunctions 0x01 startRoutine, 0x02 stopRoutine, 0x03 requestRoutineResults. Dispatch via table.
  Out:    firmware/bsw/services/Dcm/src/Dcm_RoutineControl.c, firmware/bsw/services/Dcm/include/Dcm_RoutineControl.h
  Deps:   T1.E.11
  Effort: 1.5 days
  Role:   Embedded engineer
  Status: [ ]

T1.E.13 — Implement first routine: ROUTINE_ID_MOTOR_SELF_TEST
  What:   Stub routine on RZC that runs a simple motor check and returns pass/fail. Wired via callback into Swc_Motor.
  Out:    firmware/ecu/rzc/src/Rzc_Routine_MotorSelfTest.c
  Deps:   T1.E.12
  Effort: 1 day
  Role:   Embedded engineer
  Status: [ ]

T1.E.14 — Implement second routine: ROUTINE_ID_BRAKE_CHECK
  What:   Stub routine on FZC for brake pressure verification
  Out:    firmware/ecu/fzc/src/Fzc_Routine_BrakeCheck.c
  Deps:   T1.E.12
  Effort: 4 h
  Role:   Embedded engineer
  Status: [ ]

T1.E.15 — Unit tests for 0x31
  What:   Mock routine dispatch, verify lifecycle transitions, test invalid routine ID
  Out:    test/unit/test_dcm_0x31.c
  Deps:   T1.E.14
  Effort: 1 day
  Role:   Embedded engineer
  Status: [ ]

T1.E.16 — Integrate 0x31 into dispatch table
  What:   Dcm.c + Dcm_Cfg.c
  Out:    firmware/bsw/services/Dcm/src/Dcm.c, firmware/bsw/services/Dcm/cfg/Dcm_Cfg.c
  Deps:   T1.E.15
  Effort: 1 h
  Role:   Embedded engineer
  Status: [ ]
```

### Stream E — DoIP POSIX Transport

```
T1.E.17 — Design DoIp_Posix protocol boundary
  What:   Which DoIP message types we implement (routing activation, diagnostic message, vehicle ID request). Payload forwarding into Dcm_DispatchRequest.
  Out:    docs/sovd/design-doip-posix.md
  Deps:   —
  Effort: 4 h
  Role:   Embedded lead
  Status: [ ]

T1.E.18 — Implement DoIp_Posix.c — TCP listener and message framing
  What:   tokio or plain C? Plain C (POSIX sockets) to match firmware tooling. Listen on :13400, accept single client, decode DoIP header.
  Out:    firmware/platform/posix/src/DoIp_Posix.c, firmware/platform/posix/include/DoIp_Posix.h
  Deps:   T1.E.17
  Effort: 1.5 days
  Role:   Embedded engineer
  Status: [ ]

T1.E.19 — DoIP routing activation handshake
  What:   Handle 0x0005 routing activation request; respond with 0x0006 routing activation response
  Out:    extends DoIp_Posix.c
  Deps:   T1.E.18
  Effort: 4 h
  Role:   Embedded engineer
  Status: [ ]

T1.E.20 — DoIP diagnostic message forwarding
  What:   Receive 0x8001 diagnostic message, extract UDS payload, call Dcm_DispatchRequest, encode response as 0x8001, send
  Out:    extends DoIp_Posix.c
  Deps:   T1.E.19
  Effort: 1 day
  Role:   Embedded engineer
  Status: [ ]

T1.E.21 — Unit tests for DoIP POSIX
  What:   Socket-level tests with a mock Dcm dispatcher; verify framing, routing activation, diagnostic forward
  Out:    test/unit/test_doip_posix.c
  Deps:   T1.E.20
  Effort: 1 day
  Role:   Embedded engineer
  Status: [ ]

T1.E.22 — Wire DoIp_Posix into platform/posix Makefile + BCM/ICU/TCU binaries
  What:   Build flag to enable DoIP (POSIX only); start listener at ECU startup
  Out:    firmware/platform/posix/Makefile.posix, firmware/ecu/bcm/src/main.c (and icu, tcu)
  Deps:   T1.E.21
  Effort: 4 h
  Role:   Embedded engineer
  Status: [ ]
```

### Stream E — ODX Descriptions

```
T1.E.23 — Write ODX for CVC (first ECU, hardest — most DIDs)
  What:   Hand-write .odx-d file describing CVC's UDS services: 0x10/0x11/0x22/0x27/0x3E/0x19/0x14/0x31. DIDs from T0.E.3 inventory.
  Out:    firmware/ecu/cvc/odx/cvc.odx-d
  Deps:   T0.E.3, T0.E.4, T1.E.6, T1.E.10, T1.E.16
  Effort: 1 day
  Role:   Embedded engineer
  Status: [ ]

T1.E.24 — Write ODX for FZC, RZC, SC
  What:   Same as T1.E.23 for the 3 other physical ECUs. Use CVC as template.
  Out:    firmware/ecu/fzc/odx/fzc.odx-d, rzc.odx-d, sc.odx-d
  Deps:   T1.E.23
  Effort: 1.5 days
  Role:   Embedded engineer
  Status: [ ]

T1.E.25 — Write ODX for BCM, ICU, TCU
  What:   Same for virtual ECUs
  Out:    firmware/ecu/bcm/odx/bcm.odx-d, icu.odx-d, tcu.odx-d
  Deps:   T1.E.23
  Effort: 1 day
  Role:   Embedded engineer
  Status: [ ]

T1.E.26 — Package each ECU's ODX into a .pdx archive
  What:   Script using standard zip with right layout (index.xml + content files)
  Out:    tools/odx/pack_pdx.sh, 7 .pdx files under firmware/ecu/*/odx/
  Deps:   T1.E.25
  Effort: 4 h
  Role:   Embedded engineer
  Status: [ ]

T1.E.27 — Run odx-converter on each .pdx → produce .mdd
  What:   CI step using eclipse-opensovd/odx-converter (our fork). Output MDDs committed.
  Out:    firmware/ecu/*/odx/*.mdd (7 files)
  Deps:   T1.E.26
  Effort: 4 h (most time debugging schema issues)
  Role:   Embedded engineer
  Status: [ ]
```

### Stream T — HIL Scenarios for Phase 1

```
T1.T.1  — HIL scenario: hil_081_cvc_uds_read_dtc
  What:   Send UDS 0x19 0x02 to CVC via CAN, verify DTC list returned
  Out:    test/hil/scenarios/hil_081_cvc_uds_read_dtc.yaml
  Deps:   T1.E.6
  Effort: 4 h
  Role:   Test engineer
  Status: [ ]

T1.T.2  — HIL scenario: hil_082_cvc_uds_clear_dtc
  What:   Inject DTC via fault injection, 0x14 clear, verify gone
  Out:    test/hil/scenarios/hil_082_cvc_uds_clear_dtc.yaml
  Deps:   T1.E.10
  Effort: 4 h
  Role:   Test engineer
  Status: [ ]

T1.T.3  — HIL scenario: hil_083_rzc_uds_routine_motor_test
  What:   Start motor self test routine, poll status, verify result
  Out:    test/hil/scenarios/hil_083_rzc_uds_routine_motor_test.yaml
  Deps:   T1.E.16
  Effort: 4 h
  Role:   Test engineer
  Status: [ ]

T1.T.4  — SIL scenario: sil_doip_posix_smoke
  What:   Docker run BCM binary with DoIP; external client (netcat or small py script) sends routing activation + diagnostic message; verify response
  Out:    test/sil/scenarios/sil_doip_posix_smoke.yaml
  Deps:   T1.E.22
  Effort: 4 h
  Role:   Test engineer
  Status: [ ]
```

### Stream Sf — Safety Delta

```
T1.Sf.1 — Full HARA delta review for 0x19/0x14/0x31 services
  What:   Update HARA with new hazards: unauthorized DTC clear, unauthorized routine trigger (motor self test could affect safety if active during drive). Approve.
  Out:    docs/safety/hara-delta-sovd.md, signed off by safety engineer
  Deps:   T0.Sf.1
  Effort: 1 day
  Role:   Safety engineer
  Status: [ ]

T1.Sf.2 — FMEA entries for new code paths
  What:   Add failure modes for DoIP POSIX (socket disconnect, framing error), Dcm handlers (invalid subfunction, DEM access failure)
  Out:    docs/safety/fmea-sovd-delta.md
  Deps:   T1.E.21
  Effort: 4 h
  Role:   Safety engineer
  Status: [ ]

T1.Sf.3 — MISRA deviation register check
  What:   If any new deviations needed (e.g., for DoIP socket code), document and justify
  Out:    update to docs/safety/analysis/misra-deviation-register.md
  Deps:   T1.E.21
  Effort: 2 h
  Role:   Safety engineer
  Status: [ ]
```

### Phase 1 Exit Checklist

- [ ] All Dcm unit tests green (T1.E.5, T1.E.9, T1.E.15)
- [ ] DoIP POSIX unit tests green (T1.E.21)
- [ ] MISRA zero violations on all new code
- [ ] 7 MDD files generated and committed
- [ ] HIL scenarios T1.T.1, T1.T.2, T1.T.3 pass on physical CVC/FZC/RZC
- [ ] SIL DoIP smoke test green
- [ ] HARA delta approved
- [ ] Docker CVC container responds to DoIP routing activation on localhost:13400

---

## Phase 2 — CDA Integration + CAN-to-DoIP Proxy  (Jun 1 – Jun 30)

Goal: CDA reaches every Taktflow ECU. Virtual directly via DoIP, physical via Pi proxy.

```
T2.R.1  — Configure upstream CDA for Taktflow (opensovd-cda.toml)
  Out:    classic-diagnostic-adapter/config/taktflow/opensovd-cda.toml
  Deps:   T1.E.27
  Effort: 4 h
  Role:   Rust engineer

T2.G.1  — CAN-to-DoIP proxy: workspace scaffold
  Out:    gateway/can_to_doip_proxy/{Cargo.toml, proxy-core, proxy-doip, proxy-can, proxy-main}
  Deps:   T0.G.2
  Effort: 4 h
  Role:   Pi engineer

T2.G.2  — proxy-doip: DoIP server (listener, framing, message types)
  Out:    gateway/can_to_doip_proxy/proxy-doip/src/*.rs
  Deps:   T2.G.1
  Effort: 2 days
  Role:   Pi engineer

T2.G.3  — proxy-can: SocketCAN + ISO-TP client
  What:   Use socketcan crate + custom ISO-TP state machine (or isotp-rs)
  Out:    gateway/can_to_doip_proxy/proxy-can/src/*.rs
  Deps:   T2.G.1
  Effort: 2 days
  Role:   Pi engineer

T2.G.4  — proxy-core: DoIP → CAN translation logic
  What:   Routing table: DoIP logical address → CAN frame ID range. Request/response correlation.
  Out:    gateway/can_to_doip_proxy/proxy-core/src/*.rs
  Deps:   T2.G.2, T2.G.3
  Effort: 2 days
  Role:   Pi engineer

T2.G.5  — proxy-main: config, logging, graceful shutdown
  Out:    gateway/can_to_doip_proxy/proxy-main/src/main.rs
  Deps:   T2.G.4
  Effort: 4 h
  Role:   Pi engineer

T2.G.6  — Proxy unit tests + vcan integration tests
  Out:    gateway/can_to_doip_proxy/*/tests/*.rs
  Deps:   T2.G.5
  Effort: 1.5 days
  Role:   Pi engineer

T2.G.7  — Proxy Docker image + systemd unit
  Out:    gateway/can_to_doip_proxy/Dockerfile, gateway/systemd/can-to-doip-proxy.service
  Deps:   T2.G.5
  Effort: 4 h
  Role:   Pi engineer

T2.T.1  — SIL scenario: sil_sovd_cda_smoke
  What:   Docker topology cvc+cda; curl CDA; verify DTC JSON response
  Out:    test/sil/scenarios/sil_sovd_cda_smoke.yaml
  Deps:   T2.R.1
  Effort: 4 h
  Role:   Test engineer

T2.T.2  — SIL scenario: sil_sovd_cda_all_virtual_ecus
  What:   Topology cvc+fzc+rzc+bcm+icu+tcu+cda (all 6 POSIX builds); curl for each ECU
  Out:    test/sil/scenarios/sil_sovd_cda_all_virtual_ecus.yaml
  Deps:   T2.T.1
  Effort: 4 h
  Role:   Test engineer

T2.T.3  — HIL scenario: hil_sovd_cda_physical_cvc_via_proxy
  What:   CDA on laptop → Pi proxy → physical CVC on CAN bus
  Out:    test/hil/scenarios/hil_sovd_cda_physical_cvc_via_proxy.yaml
  Deps:   T2.G.7, T2.R.1
  Effort: 1 day
  Role:   Test engineer

T2.D.1  — First upstream PR: Taktflow ODX example
  What:   Contribute cvc.odx-d as an example in odx-converter/examples/
  Out:    PR on eclipse-opensovd/odx-converter
  Deps:   T1.E.23
  Effort: 2 h
  Role:   Architect

T2.D.2  — Upstream PRs: any CDA bugs found during integration
  What:   File issues + PRs for anything broken
  Out:    PRs on eclipse-opensovd/classic-diagnostic-adapter
  Deps:   T2.T.1
  Effort: varies — 1 day budget
  Role:   Rust engineer
```

**Phase 2 Exit:** all 6 virtual ECUs return DTCs via CDA in SIL; physical CVC works via Pi proxy in HIL; CDA bug fix patches staged locally (per §8, no PRs yet).

---

## Phase 3 — Fault Library + DFM Prototype  (Jul 1 – Aug 15)

```
T3.R.1  — sovd-interfaces: DTC, Fault, OperationCycle, Severity types (Rust)
  Out:    opensovd-core/sovd-interfaces/src/{dtc.rs,fault.rs,operation_cycle.rs,severity.rs}
  Deps:   T0.R.5
  Effort: 1.5 days
  Role:   Rust engineer

T3.R.2  — IPC protocol design: Fault Shim ↔ DFM (protobuf or bincode?)
  What:   Decide and document. Mirror fault-lib Rust API shape. Versioned.
  Out:    opensovd-core/docs/ipc-fault-protocol.md + .proto file if protobuf
  Deps:   T3.R.1
  Effort: 1 day
  Role:   Rust lead

T3.R.3  — sovd-dfm: Unix socket server, fault ingestion
  Out:    opensovd-core/sovd-dfm/src/ingest.rs
  Deps:   T3.R.2
  Effort: 2 days
  Role:   Rust engineer

T3.R.4  — sovd-dfm: in-memory DTC table + operation cycle
  Out:    opensovd-core/sovd-dfm/src/{table.rs,operation_cycle.rs}
  Deps:   T3.R.3
  Effort: 2 days
  Role:   Rust engineer

T3.R.5  — sovd-db: SQLite schema + migrations via sqlx
  Out:    opensovd-core/sovd-db/migrations/0001_init.sql, sovd-db/src/*.rs
  Deps:   T3.R.1
  Effort: 1.5 days
  Role:   Rust engineer

T3.R.6  — sovd-dfm: SQLite persistence layer
  Out:    opensovd-core/sovd-dfm/src/persist.rs
  Deps:   T3.R.4, T3.R.5
  Effort: 1.5 days
  Role:   Rust engineer

T3.R.7  — sovd-dfm: stub REST endpoint GET /sovd/v1/components/{id}/faults
  What:   Wire into sovd-server; returns faults from DFM table (ISO 17978 shape)
  Out:    opensovd-core/sovd-server/src/routes/faults.rs
  Deps:   T3.R.6
  Effort: 1 day
  Role:   Rust engineer

T3.R.8  — Integration test: fault → ingest → SQLite → REST
  Out:    opensovd-core/integration-tests/tests/fault_to_dtc.rs
  Deps:   T3.R.7
  Effort: 1 day
  Role:   Rust engineer

T3.E.1  — FaultShim C header (mirrors Rust Fault API)
  Out:    firmware/bsw/services/FaultShim/include/FaultShim.h, FaultShim_Cfg.h
  Deps:   T3.R.2
  Effort: 4 h
  Role:   Embedded engineer

T3.E.2  — FaultShim C core (platform-agnostic)
  Out:    firmware/bsw/services/FaultShim/src/FaultShim.c
  Deps:   T3.E.1
  Effort: 1 day
  Role:   Embedded engineer

T3.E.3  — FaultShim_Posix backend (Unix socket IPC)
  Out:    firmware/platform/posix/src/FaultShim_Posix.c
  Deps:   T3.E.2
  Effort: 1 day
  Role:   Embedded engineer

T3.E.4  — FaultShim_Stm32 backend (NvM buffering, gateway sync task)
  What:   STM32 can't talk Unix sockets. Buffer faults to NvM; Pi gateway polls via CAN 0x500 broadcast.
  Out:    firmware/platform/stm32/src/FaultShim_Stm32.c
  Deps:   T3.E.2
  Effort: 1.5 days
  Role:   Embedded engineer

T3.E.5  — Wire FaultShim calls into Dem_SetEventStatus
  What:   On DTC transition, also call FaultShim_Report
  Out:    firmware/bsw/services/Dem/src/Dem.c (minimal edit)
  Deps:   T3.E.3
  Effort: 4 h
  Role:   Embedded engineer

T3.E.6  — FaultShim unit tests
  Out:    test/unit/test_faultshim.c
  Deps:   T3.E.3
  Effort: 1 day
  Role:   Embedded engineer

T3.D.1  — Upstream ADR PR: DFM design
  What:   Submit DFM architecture doc as ADR to opensovd/docs/design/adr/
  Out:    PR on eclipse-opensovd/opensovd
  Deps:   T3.R.4
  Effort: 1 day
  Role:   Architect

T3.T.1  — SIL scenario: fault injection → SOVD DTC visibility
  Out:    test/sil/scenarios/sil_sovd_fault_to_visibility.yaml
  Deps:   T3.R.8, T3.E.5
  Effort: 4 h
  Role:   Test engineer

T3.Sf.1 — FMEA update for FaultShim + DFM
  Out:    docs/safety/fmea-faultshim-delta.md
  Deps:   T3.E.6
  Effort: 4 h
  Role:   Safety engineer
```

**Phase 3 Exit:** fault injected in CVC Docker container appears in `curl /sovd/v1/components/cvc/faults` within 100ms; DFM upstream ADR accepted or in active review.

---

## Phase 4 — SOVD Server + Gateway  (Aug 16 – Oct 15)

```
T4.R.1   — OpenAPI spec for MVP SOVD endpoints (utoipa)
  Out:    opensovd-core/sovd-server/openapi.yaml
  Effort: 1 day

T4.R.2   — Route: wire /sovd/v1/components/{id}/faults (existing stub) to real SovdDb-backed backend
  Out:    opensovd-core/sovd-server/src/routes/faults.rs (extend, no new route — already mounted)
  Deps:   T3.R.7
  Effort: 1 day

T4.R.3   — Route: wire /sovd/v1/components/{id}/faults/{fault_code} to real SovdDb
  Effort: 4 h

T4.R.4   — Route: wire DELETE /sovd/v1/components/{id}/faults (and fault_code variant) to DFM + CDA
  Effort: 1 day

T4.R.5   — Route: wire POST /sovd/v1/components/{id}/operations/{op_id}/executions to real backend
  Effort: 1 day

T4.R.6   — Route: wire GET /sovd/v1/components/{id}/operations/{op_id}/executions/{exec_id} status
  Effort: 4 h

T4.R.7   — Route: GET /sovd/v1/components (from DFM catalog + MDD) — already mounted, wire real backend
  Effort: 1 day

T4.R.8   — Route: GET /sovd/v1/components/{id} — already mounted, wire real backend
  Effort: 4 h

T4.R.9   — Route: GET /sovd/v1/components/{id}/data — NEW, not yet mounted; wire to spec::data types (note utoipa Value name clash documented in openapi.rs)
  Effort: 1 day

T4.R.10  — sovd-gateway: backend trait + DFM backend impl
  Out:    opensovd-core/sovd-gateway/src/{backend.rs,dfm_backend.rs}
  Effort: 1.5 days

T4.R.11  — sovd-gateway: CDA backend (HTTP client → CDA REST)
  Out:    opensovd-core/sovd-gateway/src/cda_backend.rs
  Effort: 1.5 days

T4.R.12  — sovd-gateway: aggregation logic (multi-backend DTC merge)
  Effort: 1 day

T4.R.13  — sovd-gateway: config-driven route map
  Out:    opensovd-core/sovd-gateway/opensovd-gateway.toml example
  Effort: 1 day

T4.R.14  — Auth middleware skeleton (Bearer token validation stub)
  Out:    opensovd-core/sovd-server/src/auth.rs
  Effort: 1 day

T4.R.15  — Integration tests: 5 MVP use cases end-to-end
  Out:    opensovd-core/integration-tests/tests/mvp_use_cases/*.rs
  Deps:   T4.R.2..T4.R.14
  Effort: 2 days

T4.T.1   — SIL topology: full Docker Compose with 6 POSIX ECUs + CDA + SOVD stack
  Out:    opensovd/examples/taktflow-demo/docker-compose.yml
  Effort: 1 day

T4.T.2   — SIL test: all 5 MVP use cases in Docker
  Out:    test/sil/scenarios/sil_sovd_mvp_*.yaml (5 files)
  Effort: 1 day

T4.D.1   — Upstream PR: sovd-interfaces crate
  Out:    PR on eclipse-opensovd/opensovd-core
  Effort: 1 day

T4.D.2   — Upstream PR: sovd-dfm crate (after ADR approved)
  Effort: 1 day

T4.D.3   — Upstream PR: sovd-server scaffolding
  Effort: 1 day

T4.D.4   — Upstream PR: Docker Compose demo to opensovd/examples/
  Effort: 4 h
```

**Phase 4 Exit:** All 5 MVP use cases green in Docker demo; at least 1 upstream PR merged.

---

## Phase 5 — End-to-End HIL on Physical  (Oct 16 – Nov 30)

```
T5.Ops.1 — Pi deployment: Ansible playbook for SOVD stack
  Out:    gateway/ansible/sovd-deploy.yml
  Effort: 1.5 days

T5.Ops.2 — Systemd units: sovd-server, sovd-gateway, sovd-dfm, can-to-doip-proxy
  Out:    gateway/systemd/*.service
  Effort: 4 h

T5.T.1   — HIL scenario: hil_sovd_01_read_dtcs_all (all 7 ECUs)
  Effort: 4 h

T5.T.2   — HIL scenario: hil_sovd_02_clear_dtcs
  Effort: 4 h

T5.T.3   — HIL scenario: hil_sovd_03_routine_motor_test
  Effort: 4 h

T5.T.4   — HIL scenario: hil_sovd_04_fault_injection_bus_off
  Effort: 1 day

T5.T.5   — HIL scenario: hil_sovd_05_components_metadata
  Effort: 4 h

T5.T.6   — HIL scenario: hil_sovd_06_concurrent_testers
  Effort: 4 h

T5.T.7   — HIL scenario: hil_sovd_07_large_dtc_list (50+ DTCs, pagination)
  Effort: 4 h

T5.T.8   — HIL scenario: hil_sovd_08_error_handling (timeouts, disconnects)
  Effort: 4 h

T5.T.9   — Performance benchmark: DTC read latency P99
  Out:    test/perf/sovd_latency.rs, results archived per build
  Effort: 1 day

T5.T.10  — Performance benchmark: concurrent request throughput
  Effort: 1 day

T5.D.1   — Demo video recording
  Effort: 4 h
```

**Phase 5 Exit:** all 8 HIL scenarios green; P99 DTC read <500ms.

---

## Phase 6 — Hardening  (Dec 1 – Dec 31)

```
T6.R.1   — TLS: rustls on sovd-server
  Effort: 1 day

T6.R.2   — mTLS: Gateway → Server
  Effort: 1 day

T6.R.3   — Cert provisioning script (dev) + docs (prod)
  Effort: 4 h

T6.R.4   — dlt-tracing-lib integration in all Rust binaries
  Effort: 1 day

T6.R.5   — OpenTelemetry spans: end-to-end trace propagation
  Effort: 1.5 days

T6.R.6   — Rate limiting middleware (tower::limit)
  Effort: 4 h

T6.D.1   — Integrator guide: opensovd/docs/integration/
  Effort: 2 days

T6.D.2   — Upstream integrator guide PR
  Effort: 4 h

T6.D.3   — Final upstream PR push: all remaining opensovd-core code
  Effort: 2 days

T6.Sf.1  — Final safety case delta, signed and archived
  Effort: 1 day

T6.S.1   — Retro documents for all phases
  Out:    docs/retro/phase-0.md … phase-6.md
  Effort: 1 day
```

**Phase 6 Exit:** all success criteria from MASTER-PLAN §12 met; upstream PRs merged or in review; project ready for demo.

---

## Task Count Summary

| Phase | Task count | Est effort |
|-------|-----------|-----------|
| 0 | ~25 | 8 person-days |
| 1 | ~30 | 25 person-days |
| 2 | ~12 | 20 person-days |
| 3 | ~16 | 30 person-days |
| 4 | ~22 | 50 person-days |
| 5 | ~12 | 30 person-days |
| 6 | ~11 | 20 person-days |
| **Total** | **~128** | **~183 person-days** |

---

## How to Use This File

1. **Copy tasks into your tracker** (Jira / GitHub Issues / Linear). IDs stay stable.
2. **Start with Phase 0.** Everything downstream depends on it.
3. **Update status** inline: `[ ]` → `[~]` → `[x]`.
4. **Add subtasks** as needed — don't re-number, append letters: `T1.E.2a`, `T1.E.2b`.
5. **When a phase exits,** open the next phase's "medium detail" section and break tasks down further (add specific files, hours, owner).
6. **Retro after each phase** — capture surprises and feed back into remaining tasks.
