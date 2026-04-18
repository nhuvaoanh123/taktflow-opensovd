# Eclipse OpenSOVD — Taktflow End-to-End Master Plan

<!--
  Rewritten 2026-04-18 in the YAML handoff shape from ~/.claude/CLAUDE.md
  (date / project / part / task / achievement / decisions_with_rationale /
  current_state / blockers / next_steps). The plan is forward-looking, so
  "achievement" captures what is already done and "next_steps" captures
  what is still to do; each phase is its own record under plan[].
-->

```yaml
date: 2026-04-18
project: taktflow-opensovd
part: master-plan
task: opensovd-mvp-end-of-2026

achievement:
  - Phase 0 foundation complete — opensovd-core workspace scaffolded, CI matrix wired, ADR-0001 landed
  - Phase 1 embedded UDS + DoIP POSIX complete — Dcm 0x19/0x14/0x31 handlers pass HIL, DoIP listener on 13400
  - Phase 2 CDA integration complete — CAN-to-DoIP proxy reaches physical CVC, SIL + HIL smoke green
  - Phase 3 Fault Lib + DFM complete — embedded Fault Shim → DFM SQLite → SOVD GET round-trip <100 ms in Docker
  - Phase 4 SOVD Server + Gateway complete — 5 MVP use cases pass in Docker Compose, every crate upstream-ready (no PRs opened)
  - Phase 5 Stage 1 in progress — fault-sink-mqtt + ws-bridge crates merged to main; dashboard hybrid data-wiring live; Mosquitto kit ready on feat/mqtt-broker-deploy
  - doip-codec evaluation spike complete — partial migration plan documented (docs/doip-codec-evaluation.md), CDA fork pins captured
  - ADR-0023 trimmed physical bench to 3 ECUs (CVC, SC, BCM); FZC/RZC retired
  - ADR-0024 capability-showcase observer dashboard accepted — two-stage plan (self-hosted mTLS first, optional AWS later)
  - ADR-0025 CVC OTA accepted 2026-04-17 — folded into Phase 6 deliverable 8

decisions_with_rationale:
  - decision: Build first, contribute later — no upstream PRs during Phases 0–3
    rationale: Owning finished, tested code avoids upstream churn, design-by-committee, and maintainer-responsiveness dependencies
    how_to_apply: All Phase 0–3 work lives in local feature branches; nothing pushed to our forks without explicit team approval

  - decision: Mirror upstream CDA wholesale, layer Taktflow extras on top
    rationale: Keeps `git diff upstream/main -- <mirrored-files>` ≈ 0; CI failures on unimplemented parts become our implementation backlog
    how_to_apply: Copy build.yml / feature flags / patches / deny.toml / workflows verbatim; Taktflow additions go in distinct crates or clearly labeled modules; weekly upstream-sync rebase Monday 09:00

  - decision: Fault Library shim is C on embedded side, Rust only on POSIX / Pi components
    rationale: Avoids dragging Rust toolchain into ASIL-D firmware lifecycle
    how_to_apply: FaultShim_Posix.c + FaultShim_Stm32.c wrap DFM IPC; all opensovd-core crates stay Rust

  - decision: Never hard fail — backends log-and-continue, locks are bounded try_lock_for, no panic/unwrap/expect in HTTP-reachable code
    rationale: Upstream CDA proved aggressive error propagation breaks in realistic environments (ADR-0018); we copy the behavior not their prose
    how_to_apply: Clippy lints enforce on backend crates; degraded responses carry stale:true and error_kind label; spec-boundary rejection stays strict

  - decision: SIL first, HIL second, physical hardware last
    rationale: Docker SIL feedback loop is seconds; Pi HIL is minutes; physical ECU re-flashing is hours — defer expensive debug surfaces
    how_to_apply: Nothing touches physical ECUs until Docker topology passes; HIL gated on SIL-green

  - decision: Capability-showcase dashboard Stage 1 is self-hosted mTLS, zero cloud cost
    rationale: $0 recurring cost, authority stays on-bench, defers AWS fleet-uplink complexity to Stage 2 without blocking Phase 5 exit
    how_to_apply: Reuse taktflow-embedded-production cloud_connector+ws_bridge with AWS_IOT_ENDPOINT=""; Prometheus+Grafana replaces Timestream; SvelteKit static served by nginx with client-cert auth

  - decision: Upstream house style before custom patterns — adopt CDA conventions by default
    rationale: Minimizes diff when we eventually upstream; avoids reinventing solved problems
    how_to_apply: Generics over dynamic dispatch (only security plugin is dyn Trait); tokio::io::split on DoIP streams; mbedtls fallback when OpenSSL hits walls; tokio-console for deadlock debugging; deviations documented per-ADR

  - decision: doip-codec PARTIAL migration in Phase 5 Line B
    rationale: theswiftfox forks (what CDA actually pins) match DoIp_Posix.c byte-for-byte; crates.io samp-reston version does not
    how_to_apply: Replace frame.rs + message_types.rs with fork; keep server.rs + DoipHandler trait + ISO-TP FC from PR #9 + ADR-0010 discovery logic

  - decision: OTA limited to CVC in Phase 6 (ADR-0025)
    rationale: STM32G474RE dual-bank A/B proven path; SC/BCM OTA defers to future ADR-0026 if pulled in
    how_to_apply: CMS/X.509 sharing device mTLS PKI root, N=5 rollback threshold, signed boot-OK witness over MQTT; SOVD bulk-data + UDS 0x34/0x36/0x37

current_state:
  summary: |
    Phases 0–4 complete; Phase 5 Stage 1 in progress. Fault fan-out to MQTT
    is wired end-to-end and tested; dashboard is consuming real REST + WS
    from the bench. Physical-ECU flashing (STM32 + TMS570) and nginx
    TLS/mTLS terminator remain before Phase 5 exit. Code is upstream-ready
    but nothing has been pushed to public forks.

  files_touched:
    - opensovd-core/sovd-server/
    - opensovd-core/sovd-gateway/
    - opensovd-core/sovd-dfm/
    - opensovd-core/sovd-interfaces/
    - opensovd-core/sovd-db/migrations/
    - opensovd-core/integration-tests/
    - opensovd-core/fault-sink-mqtt/
    - opensovd-core/ws-bridge/
    - gateway/can_to_doip_proxy/
    - firmware/bsw/services/Dcm/Dcm_ReadDtcInfo.c
    - firmware/bsw/services/Dcm/Dcm_ClearDtc.c
    - firmware/bsw/services/Dcm/Dcm_RoutineControl.c
    - firmware/bsw/services/FaultShim/
    - firmware/platform/posix/src/DoIp_Posix.c
    - firmware/platform/posix/src/FaultShim_Posix.c
    - firmware/ecu/*/odx/*.odx-d
    - firmware/ecu/*/odx/*.mdd
    - tools/odx-gen/
    - dashboard/
    - test/sil/scenarios/sil_sovd_*.yaml
    - test/hil/scenarios/hil_sovd_*.yaml
    - docs/adr/
    - docs/doip-codec-evaluation.md
    - docs/openapi-audit-2026-04-14.md

  status:
    - Phase 0 complete — 2026-04-30
    - Phase 1 complete — 2026-05-31 (M1)
    - Phase 2 complete — 2026-06-30 (M2)
    - Phase 3 complete — 2026-08-15 (M3)
    - Phase 4 complete — 2026-10-15 (M4)
    - Phase 5 in progress — target 2026-11-30 (M5 ships end of Phase 6)
    - Phase 6 not started — target 2026-12-31
    - 3 active ECUs on HIL bench (CVC physical CAN, SC TMS570 CAN, BCM virtual DoIP)
    - Upstream sync current — weekly Monday 09:00 rebase, no drift
    - Zero MISRA violations, zero clippy pedantic violations on new code
    - No upstream PRs opened — decision deferred to Phase 6

blockers:
  - Pi binary rebuild pinned to nightly-2025-07-14 lacks aarch64-unknown-linux-gnu target on the Windows dev host; Pi itself has no Rust toolchain or source checkout — blocks Phase 5 live redeploy after the Windows-binary copy incident
  - D3 SOVD clear-faults HIL precondition unmet on live bench — no clearable fault has been injected; test stays red until inject-step lands
  - TMS570 Ethernet still absent — not a current blocker (CAN-to-DoIP proxy path handles it) but blocks any future native-DoIP-on-TMS570 ambition
  - Auth model for SOVD Server unresolved (OAuth2 vs cert vs both) — Phase 4 MVP scaffolded bearer token only, real validation deferred to Phase 6 hardening
  - R9 — OpenSOVD maintainers starting opensovd-core in parallel would force rebase-onto-their-scaffolding; mitigated but not eliminated by weekly upstream-sync watch
  - 14-person peak allocation depends on Taktflow not pulling workstream members to other priorities (R11) — architect holds scope via phase gates
  - ODX schema licensing (R3) — ASAM official vs community XSD still undecided for odx-converter; we ship the community subset under Apache-2.0 as fallback

next_steps:
  phase_5_stage_1_exit:
    - Provision aarch64-unknown-linux-gnu toolchain on Windows dev host, OR stand up native Rust toolchain on the Pi with a source checkout — unblocks live redeploy
    - Inject a clearable fault on the bench to unblock D3 HIL precondition
    - Land nginx TLS + mTLS terminator container (T24.1.15, ~1 day)
    - Land Prometheus scrape config + Grafana dashboards on Pi (T24.1.9, ~1 day)
    - Pin MQTT wire contract with insta snapshots across crate boundaries (~1 h)
    - Merge feat/mqtt-broker-deploy into main

  phase_5_remainder:
    - Cross-compile firmware/ecu/cvc/ as ARM ELF; flash via ST-LINK (COM3) with `cargo xtask flash-cvc`
    - Smoke test: UDS 22F190 over real CAN via Pi GS_USB; assert VIN matches cvc_identity.toml
    - Flash TMS570 TCU via XDS110 (COM11/COM12) using TI Uniflash or Code Composer CLI
    - Execute doip-codec PARTIAL migration in proxy-doip (keep server.rs, DoipHandler, ISO-TP FC from PR #9, ADR-0010)
    - Extend tools/odx-gen/ with MDD FlatBuffers emitter (--emit=mdd) matching CDA cda-database schema; round-trip test
    - Install alexmohr/mdd-ui on dev host; add console-subscriber dev-dep on sovd-main for tokio-console attach
    - Run 8 HIL scenarios (hil_sovd_01..08) in nightly pipeline
    - Validate performance — /faults read <100 ms, P99 <500 ms, <200 MB RAM on Pi
    - Record demo video for OpenSOVD community presentation

  phase_6_hardening:
    - TLS everywhere — rustls/openssl default, mbedtls fallback behind Cargo feature flag
    - DLT tracing wired via dlt-tracing-lib; correlation IDs propagate Gateway → Server → CDA → ECU
    - OpenTelemetry spans exported to OTLP collector (Jaeger or Tempo)
    - Rate limiting via tower::limit middleware (per-client-IP)
    - Integrator guide in docs/integration/, shaped as upstream-ready PR
    - Safety case delta — HARA for new UDS services, DoIP + Fault Shim failure modes
    - CVC OTA end-to-end per ADR-0025 — dual-bank A/B, CMS/X.509, N=5 rollback, boot-OK witness
    - Contribution decision — architect + Rust lead + safety engineer review §12.2 checklist; if go, open PRs in §8.2 priority order

  open_questions_to_resolve:
    - Fault IPC: Unix socket vs shared memory? — Rust lead, Phase 0 week 2 (decided: Unix socket, in prod)
    - DFM persistence: SQLite vs FlatBuffers file? — Architect, Phase 0 week 2 (decided: SQLite via sqlx)
    - ODX schema: ASAM download vs community XSD? — Embedded lead, Phase 1 week 1
    - Auth model: OAuth2 / cert / both? — Architect + security lead, Phase 4 (deferred to Phase 6)
    - DoIP discovery on Pi: broadcast vs static? — Pi engineer (ADR-0010: "both")
    - Physical DoIP on STM32: lwIP vs ThreadX NetX vs never? — Hardware lead, Phase 5 (deferred)
    - doip-codec Cargo pin: vendor vs git-rev matching CDA exactly? — default git-rev, confirm during migration

plan:
  phase_0_foundation:
    window: 2026-04-14 .. 2026-04-30
    person_days: 8
    owner: Architect + 1 Rust engineer
    parallel_to: []
    entry: ECA signed, toolchain installed, CDA builds locally
    deliverables:
      - ADR for Taktflow-SOVD integration (this document → ADR-0001)
      - Git branch strategy — feature/sovd-* branches, PRs gated by SIL+HIL
      - opensovd-core workspace skeleton with empty crates + CI
      - CI matrix — cargo test --workspace + clippy pedantic + nightly fmt
      - First SOVD architecture document PR to upstream opensovd repo
    exit: Hello-world Rust binary in opensovd-core/sovd-server returns 200 OK on /health

  phase_1_embedded_uds_doip_posix:
    window: 2026-05-01 .. 2026-05-31
    person_days: 25
    owner: Embedded lead + 2 embedded engineers
    parallel_to: [Phase 0 tail]
    entry: Phase 0 complete
    deliverables:
      - Dcm 0x19 ReadDTCInformation handler — subfunctions 0x01, 0x02, 0x0A + unit + HIL tests
      - Dcm 0x14 ClearDiagnosticInformation handler — Dem_ClearDTC by group + NvM async flush
      - Dcm 0x31 RoutineControl handler — dispatch table + motor_self_test, brake_check
      - DoIp_Posix.c — TCP listener on 13400, vehicle id / routing activation / diag message types
      - Per-ECU ODX descriptions (3 active ECUs per ADR-0023) + MDDs committed
    exit:
      - All new Dcm handlers pass unit tests
      - MISRA clean in CI
      - HIL suite green with new tests
      - odx-converter produces valid MDDs for the 3 active ECUs
      - Docker-based CVC accepts DoIP on localhost:13400 and responds to UDS 0x19

  phase_2_cda_integration_can_to_doip_proxy:
    window: 2026-06-01 .. 2026-06-30
    person_days: 20
    owner: Rust lead + 1 Rust engineer + 1 Pi engineer + 1 test engineer
    parallel_to: [Phase 1 tail (partial)]
    entry: Phase 1 Dcm handlers working in SIL; MDDs generated
    deliverables:
      - CDA configured for Taktflow — opensovd-cda.toml with MDD paths + DoIP scan range + DLT logging
      - CAN-to-DoIP proxy crate in gateway/can_to_doip_proxy/ — proxy-core, proxy-doip, proxy-can, proxy-main
      - SIL scenario sil_sovd_cda_smoke.yaml — cvc + cda containers, curl returns ListOfFaults
      - HIL scenario hil_sovd_cda_via_proxy.yaml — Pi proxy, CDA on laptop, physical CVC target
    exit:
      - CDA smoke test green in SIL nightly
      - Proxy ≥80% line coverage
      - HIL scenario passes against physical CVC
      - CDA bugs staged as internal fix branches, not upstreamed yet
      - Taktflow ODX example staged locally under odx-converter/examples/

  phase_3_fault_lib_dfm_prototype:
    window: 2026-07-01 .. 2026-08-15
    person_days: 30
    owner: Embedded lead + Rust lead + 1 Rust engineer + 1 embedded engineer
    parallel_to: []
    entry: Phase 2 complete
    deliverables:
      - C fault shim module firmware/bsw/services/FaultShim/ — Init, Report, Shutdown signatures mirroring Rust fault-lib
      - POSIX shim impl — Unix socket to DFM, protobuf on wire
      - STM32 shim impl — NvM slot buffering, flushed by gateway sync task
      - DFM prototype opensovd-core/sovd-dfm/ — in-memory table, sqlx+SQLite persistence, axum stub endpoint
      - Wiring test — synthetic fault in CVC Docker → SOVD GET within 100 ms
      - SQLite schema opensovd-core/sovd-db/migrations/ — dtcs, fault_events, operation_cycles, catalog_version
      - Internal DFM ADR in docs/adr/ (upstream-ready shape per §8.1)
    exit:
      - End-to-end fault report → SOVD visibility works in Docker
      - DFM integration tests cover ingestion, query, clear, operation cycle
      - Internal DFM ADR reviewed by architect + Rust lead

  phase_4_sovd_server_gateway:
    window: 2026-08-16 .. 2026-10-15
    person_days: 50
    owner: Rust lead + 3 Rust engineers + 1 test engineer
    parallel_to: [Phase 3 tail (partial)]
    entry: Phase 3 complete; DFM serving DTCs
    deliverables:
      - SOVD Server crate — axum + tokio, endpoints per ISO 17978-3 SOVD v1.1.0-rc1 (per-component shape)
      - Endpoints — /sovd/v1/health, /components, /components/{id}, /components/{id}/{data,faults,faults/{code},operations,operations/{op_id}/executions,operations/{op_id}/executions/{exec_id}}; DELETE on faults + faults/{code}
      - OpenAPI spec sovd-server/openapi.yaml; types via utoipa
      - SOVD Gateway — DFM + CDA + future-native-SOVD backends; opensovd-gateway.toml route map; DTC de-dup by code
      - Authentication middleware scaffold — bearer token accepted, validation deferred to Phase 6
      - Docker Compose demo topology (internal, not upstreamed) — services + tester script for 5 MVP use cases
      - Upstream-ready polish — every crate technically PR-able, just no PR opened
    exit:
      - Docker Compose demo runs 5 MVP use cases end-to-end
      - SOVD Server ≥70% line coverage
      - Integration tests cover full SOVD → Gateway → CDA → ECU chain
      - No code change needed to open upstream PRs — only a team decision

  phase_5_e2e_demo_hil_physical:
    window: 2026-10-16 .. 2026-11-30
    person_days: 30
    owner: Test lead + 2 test engineers + 1 Rust engineer + 1 embedded engineer
    parallel_to: []
    entry: Phase 4 Docker demo working
    deliverables:
      - Pi deployment — Ansible or Docker Compose; Server + Gateway + DFM + proxy on Pi with systemd/restart policies
      - HIL suite hil_sovd_01..08 — read_faults_all, clear_faults, operation_motor_test, fault_injection, components_metadata, concurrent_testers, large_fault_list, error_handling
      - Real STM32 flashing via ST-LINK (COM3) on Windows dev host — `cargo xtask flash-cvc`, smoke via UDS 22F190
      - TMS570 TCU integration via XDS110 (COM11/COM12) — TI Uniflash or CCS CLI; CAN routing through Pi proxy
      - doip-codec PARTIAL migration in proxy-doip — theswiftfox fork at 0dba319 + doip-definitions at bdeab8c
      - MDD FlatBuffers emitter in tools/odx-gen/ — --emit=mdd, round-trip against CDA cda-database
      - Autonomous bench debugging — alexmohr/mdd-ui on dev host, console-subscriber on sovd-main
      - Performance validation — /faults <100 ms, P99 <500 ms, <200 MB RAM on Pi
      - Capability-showcase observer dashboard (ADR-0024):
          stage_1_self_hosted_mTLS:
            - fault-sink-mqtt crate publishing DFM events to Mosquitto (JSON wire format)
            - cloud_connector + ws_bridge reused from taktflow-embedded-production with AWS_IOT_ENDPOINT=""
            - Prometheus + Grafana on Pi for historical view (replaces Timestream — $0 recurring)
            - nginx TLS terminator + mTLS client-cert auth aligned with SEC-2.1
            - SvelteKit + Tailwind + shadcn-svelte dashboard, static build at https://<pi-ip>/
            - 20 OpenSOVD use-case widgets live, including UC19 Prometheus panel
            - stage_1_progress_2026_04_17_to_18:
                merged_to_main:
                  - fault-sink-mqtt scaffolded (6df34fb)
                  - fault-sink-mqtt wired into DFM fan-out + in-process rumqttd round-trip (3d3d040, 4743dc8)
                  - ws-bridge MQTT→WS relay + bearer auth + /metrics + round-trip test (0263422)
                  - SvelteKit scaffold, 20 widgets, canned stubs (e52267e)
                local_unpushed:
                  - Mosquitto broker deployment kit — conf.d, ACL, systemd, cert provisioning, TLS 1.2 floor (27019d2c)
                uncommitted_workspace_2026_04_18:
                  - Dashboard hybrid data-wiring — live components/faults/operations/data/health + ws-bridge relay in dashboard/; canned only for UC15 session, UC16 audit log, UC18 topology
                remaining_for_stage_1_exit:
                  - Replace remaining canned dashboard stubs; append ?token= in wsClient.ts (~3–5 days)
                  - nginx TLS + mTLS terminator container — T24.1.15 (~1 day)
                  - Prometheus scrape + Grafana dashboards on Pi — T24.1.9 (~1 day)
                  - Pin MQTT wire contract with insta snapshots across crate boundaries (~1 h)
                  - Merge feat/mqtt-broker-deploy into main
          stage_2_optional_aws_uplink:
            - DEVICE_ID=taktflow-sovd-hil-001 under shared embedded-production AWS account
            - scripts/aws-iot-setup.sh flips AWS_IOT_ENDPOINT; no Timestream
            - bench_id=sovd-hil tag for data attribution; fleet cross-bench aggregation lands here
    exit:
      - All 8 HIL scenarios green in nightly pipeline
      - Performance targets met
      - Stage 1 dashboard serves all 20 use-case widgets on bench LAN; fault visible <200 ms; 7 days history; nginx rejects unauthenticated
      - Stage 2 optional exit — fault visible on AWS IoT Core test console <2 s on vehicle/dtc/new with bench_id=sovd-hil
      - Demo video recorded

  phase_6_hardening:
    window: 2026-12-01 .. 2026-12-31
    person_days: 20
    owner: All hands, architect lead
    parallel_to: []
    entry: Phase 5 HIL green
    deliverables:
      - TLS everywhere — rustls/openssl default, mbedtls fallback behind Cargo feature flag; mTLS Gateway → Server; DoIP TLS auth-only per upstream CDA cipher pattern
      - DLT tracing — all Rust binaries emit DLT; daemon on Pi forwards to laptop/cloud; correlation IDs propagate
      - OpenTelemetry spans — OTLP export to Jaeger or Tempo
      - Rate limiting — tower::limit per-client-IP
      - Integrator guide in docs/integration/ — upstream-ready format per §8
      - Safety case delta — HARA for new UDS services, new DoIP + Fault Shim failure modes
      - Contribution decision — §12.2 checklist applied; PRs in §8.2 order if go
      - OTA on CVC (ADR-0025) — STM32G474RE dual-bank A/B, CMS/X.509 sharing device mTLS PKI root, N=5 rollback threshold, signed boot-OK witness over MQTT; SOVD bulk-data + UDS 0x34/0x36/0x37; flash state machine Idle → Downloading → Verifying → Committed ↔ Rollback; FR-8.1..8.6 + SR-6.1..6.5 (ASPICE-append); UC21 initiate / UC22 progress / UC23 abort+rollback; ~4–6 weeks CVC-only
    exit:
      - All prior exit criteria still hold
      - Safety case delta approved
      - Integrator guide complete (pushed upstream only if team decides)
      - Contribution decision recorded in docs/adr/phase-6-contribution-decision.md
      - OTA on CVC demonstrable end-to-end — signed image via SOVD bulk-data, flashed to inactive slot, committed after signature pass, boot-OK witness acknowledged at cloud

reference:
  what_opensovd_is:
    - SOVD = Service-Oriented Vehicle Diagnostics, ISO 17978 (ASAM)
    - Modern replacement for UDS (ISO 14229); REST/HTTP+JSON instead of CAN+binary byte frames
    - Eclipse OpenSOVD = open-source reference implementation under Eclipse Automotive / S-CORE
    - S-CORE v1.0 integration target end of 2026
    - Classic Diagnostic Adapter (CDA) translates SOVD REST → UDS/DoIP for legacy ECUs

  why_taktflow_is_doing_this:
    technical: Add SOVD so every Taktflow ECU becomes reachable via modern REST diagnostics
    product: OEMs are moving to SOVD; Taktflow speaking SOVD natively is more valuable and cheaper to integrate
    strategic: opensovd-core is an empty stub; landing the first real code there is the single highest-leverage spot in Eclipse SDV — shadow-ninja, never ping maintainers, let the work speak
    tactical: Build ourselves first; upstream finished working systems, not half-built code

  current_upstream_state:
    classic-diagnostic-adapter: Active, ~MVP-ready — reusable as-is for SOVD→UDS bridge
    odx-converter: Active — reusable for ECU description conversion
    fault-lib: Alpha — reference for Fault API shape; we port to C
    dlt-tracing-lib: Active — reusable for observability
    uds2sovd-proxy: Early — optional, only if legacy tester compat needed
    cpp-bindings: Stub — we grow this for C/C++ integration
    opensovd-core: Empty stub — we build this from scratch
    opensovd: Active docs — where we upstream architecture decisions

  mvp_use_cases:
    UC1_read_faults: Tester GET /faults → Server → DFM → SQLite + CDA (UDS 0x19 over DoIP) → unified JSON ListOfFaults
    UC2_report_fault: Swc detects condition → FaultShim_Report → Unix socket / NvM buffer → DFM in-memory + SQLite
    UC3_clear_faults: Tester DELETE /faults → DFM clears + notifies CDA → UDS 0x14 → Dem_ClearDTC + NvM flush
    UC4_reach_uds_ecu_via_cda: Tester GET /faults → Server → Gateway → CDA (not DFM) → MDD → UDS 0x19
    UC5_trigger_diagnostic_service: Tester POST /operations/{op_id}/executions → CDA → UDS 0x31 StartRoutine → Swc handler

  upstream_contribution_priority_when_decision_is_made:
    1: sovd-interfaces trait contracts — opensovd-core (lowest risk, establishes presence)
    2: sovd-dfm with design doc — opensovd-core (fills major gap)
    3: sovd-server MVP — opensovd-core (central piece)
    4: sovd-gateway — opensovd-core (routing + multi-ECU)
    5: Taktflow ODX examples — odx-converter/examples/ (low risk, demonstrates real use)
    6: CDA bugs found during integration — classic-diagnostic-adapter (isolated fixes)
    7: Docker Compose demo topology — opensovd/examples/
    8: Integrator guide — opensovd/docs/integration/

  never_upstreamed:
    - Taktflow-specific DBC files and codegen pipelines
    - Embedded Dcm modifications to taktflow-embedded-production firmware
    - ASPICE + ISO 26262 process artifacts
    - Raspberry Pi deployment Ansible playbooks and systemd units
    - Safety case deltas, HARA updates, FMEA tables
    - Internal ADRs and knowledge-base notes under docs/sovd/notes-*

  milestones:
    M1_embedded_uds_complete: 2026-05-31 — Dcm 0x19/0x14/0x31 pass HIL; DoIP POSIX accepts diag messages
    M2_cda_integration_green: 2026-06-30 — SOVD GET via CDA round-trips to Docker ECU; Pi proxy reaches physical CVC
    M3_dfm_prototype_serving_dtcs: 2026-08-15 — fault inject → DFM ingest → SOVD GET <100 ms
    M4_sovd_server_mvp_in_docker: 2026-10-15 — 5 MVP use cases pass in Docker Compose
    M5_hardened_hil_green_upstream_ready: 2026-12-31 — physical HIL passes; demo recorded; code upstream-ready

  success_criteria:
    technical:
      - All 5 OpenSOVD MVP use cases pass against Taktflow hardware in SIL and HIL
      - Server + Gateway + DFM + CAN-to-DoIP proxy running on Pi in production mode
      - DTC round-trip <500 ms P99 across 3 active ECUs (ADR-0023)
      - Zero MISRA violations on new embedded code
      - Zero clippy pedantic violations on new Rust code
      - Safety case delta approved by safety engineer
      - Nightly SIL + HIL green 30 consecutive days
    contribution_readiness_not_required:
      - All code stylistically indistinguishable from upstream CDA
      - sovd-interfaces is the cleanest public-facing artifact in the workspace
      - Internal design ADRs exist for every major component
      - No blocker prevents opening upstream PRs — decision is pure policy
    process:
      - All new work products traceable in ASPICE
      - All 5 MVP use cases have requirements → design → test traceability
      - Safety case updated and reviewed
      - Zero safety regressions on existing HIL suite

  team_allocation_peak_phase_4:
    architect_upstream_liaison: 1
    embedded_lead: 1
    embedded_engineers: 2
    rust_lead: 1
    rust_engineers: 3
    safety_engineer: 1 (part-time)
    test_lead: 1
    test_engineers: 2
    devops_ci: 1
    pi_gateway_engineer: 1
    technical_writer: 1 (part-time)
    total_peak: 14 of 20

  governance:
    decision_authority:
      architectural: Architect, documented as ADRs, weekly review by Rust lead + Embedded lead
      scope: Architect, escalation to program lead if timeline at risk
      safety: Safety engineer, veto on anything touching ASIL paths
      upstream_alignment: Architect, with upstream maintainer consent via design ADRs
    cadence:
      daily_standup: 15 min, workstream only
      weekly_sync: 45 min, SOVD workstream + architect
      weekly_upstream_review: 30 min, architect reviews discussions + PRs
      phase_gate_review: end of each phase, all leads, go/no-go
    documentation:
      - Every ADR in opensovd/docs/design/adr/ (upstream) or docs/adr/ (Taktflow internal)
      - Every phase produces retro in docs/retro/phase-<n>.md
      - Every HIL scenario YAML has one-paragraph intent comment
      - Every ADR written in upstream-ready PR shape
```
